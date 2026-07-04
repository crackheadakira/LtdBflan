use serde::{Deserialize, Serialize};

use crate::{
    core::{Cursor, FormatError, Writer},
    ui2d::systemdata::{LayoutData, PaneData, SystemData},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserDataArray {
    pub user_data: Vec<UserData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserData {
    pub content: UserDataContent,
    pub o_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserDataContent {
    String(String),
    S32(Vec<i32>),
    Float(Vec<f32>),
    SystemData(Vec<Vec<SystemData>>),
}

impl UserDataContent {
    pub fn type_tag(&self) -> u8 {
        match self {
            Self::String(_) => 0,
            Self::S32(_) => 1,
            Self::Float(_) => 2,
            Self::SystemData(_) => 3,
        }
    }
}

impl UserDataArray {
    pub fn parse(cursor: &mut Cursor, is_pane: bool) -> Result<Self, FormatError> {
        let user_data_count = cursor.read_u16()?;
        let _reserve0 = cursor.read_u16()?;
        let mut user_data = Vec::with_capacity(user_data_count as usize);

        for _ in 0..user_data_count {
            user_data.push(UserData::parse(cursor, is_pane)?)
        }

        Ok(Self { user_data })
    }

    pub fn serialize(&self, writer: &mut Writer) {
        writer.mark("UserData (section)");
        writer.write_u16(self.user_data.len() as u16);
        writer.write_u16(0);

        let mut slots = Vec::with_capacity(self.user_data.len());

        for data in &self.user_data {
            let entry_base = writer.pos();
            let name_ph = writer.write_placeholder_u32();
            let data_ph = writer.write_placeholder_u32();

            let count = match &data.content {
                UserDataContent::String(s) => s.len() as u16,
                UserDataContent::S32(v) => v.len() as u16,
                UserDataContent::Float(v) => v.len() as u16,
                UserDataContent::SystemData(v) => v.len() as u16,
            };

            writer.write_u16(count);
            writer.write_u8(data.content.type_tag());
            writer.write_u8(0);

            slots.push((entry_base, name_ph, data_ph));
        }

        let type_order: &[fn(&UserDataContent) -> bool] = &[
            |i| matches!(i, UserDataContent::SystemData(_)),
            |i| matches!(i, UserDataContent::Float(_) | UserDataContent::S32(_)),
        ];

        for type_check in type_order {
            for (i, data) in self.user_data.iter().enumerate() {
                if !type_check(&data.content) {
                    continue;
                }

                let (entry_base, _name_ph, data_ph) = slots[i];
                writer.patch_u32(data_ph, (writer.pos() - entry_base) as u32);

                match &data.content {
                    UserDataContent::Float(floats) => {
                        for &f in floats {
                            writer.write_f32(f)
                        }
                    }
                    UserDataContent::S32(ints) => {
                        for &s in ints {
                            writer.write_i32(s)
                        }
                    }
                    UserDataContent::SystemData(blocks) => {
                        writer.mark("SystemDataArray");
                        for block in blocks {
                            let count = block.len();

                            writer.write_u16(0);
                            writer.write_u16(count as u16);

                            let offset: u32 = if count > 1 { 0xC } else { 0x8 };
                            writer.write_u32(offset);

                            let size_ph = if count > 1 {
                                Some(writer.write_placeholder_u32())
                            } else {
                                None
                            };

                            let items_start = writer.pos();
                            for item in block {
                                match item {
                                    SystemData::Pane(pane) => pane.serialize(writer),
                                    SystemData::Layout(layout) => layout.serialize(writer),
                                }
                            }

                            if let Some(ph) = size_ph {
                                let items_written = writer.pos() - items_start;

                                // rounding up to next 8 byte boundary
                                let block_size = (items_written + 7) & !7;
                                writer.patch_u32(ph, block_size as u32);

                                let padding = block_size - items_written;
                                for _ in 0..padding {
                                    writer.write_u8(0);
                                }
                            }
                        }
                    }
                    // strings are handled afterwards
                    _ => {}
                }
            }
        }

        for (i, data) in self.user_data.iter().enumerate() {
            let (entry_base, name_ph, data_ph) = slots[i];

            match &data.content {
                UserDataContent::String(s) if !s.is_empty() => {
                    writer.patch_u32(data_ph, (writer.pos() - entry_base) as u32);
                    writer.write_fixed_string(s, s.len());
                    writer.write_u8(0);
                }
                UserDataContent::Float(v) if v.is_empty() => writer.patch_u32(data_ph, 0),
                UserDataContent::S32(v) if v.is_empty() => writer.patch_u32(data_ph, 0),
                UserDataContent::SystemData(v) if v.is_empty() => writer.patch_u32(data_ph, 0),
                UserDataContent::String(_) => writer.patch_u32(data_ph, 0),
                _ => {}
            }

            writer.patch_u32(name_ph, (writer.pos() - entry_base) as u32);
            writer.write_null_terminated_string(&data.o_name);
        }

        writer.align(4);
    }
}

impl UserData {
    pub fn parse(cursor: &mut Cursor, is_pane: bool) -> Result<Self, FormatError> {
        let base_offset = cursor.pos;

        let name_offset = cursor.read_u32()?;
        let data_array_offset = cursor.read_u32()?;
        let data_count = cursor.read_u16()?;

        let type_tag = cursor.read_u8()?;
        let _reserve0 = cursor.read_u8()?;

        let restore_point = cursor.pos;

        let content = if data_array_offset > 0 {
            cursor.seek(base_offset + data_array_offset as usize)?;

            match type_tag {
                0 => {
                    let str_data = cursor.read_string(data_count as usize)?;
                    UserDataContent::String(str_data)
                }
                1 => {
                    let mut values = Vec::with_capacity(data_count as usize);
                    for _ in 0..data_count {
                        values.push(cursor.read_i32()?);
                    }

                    UserDataContent::S32(values)
                }
                2 => {
                    let mut values = Vec::with_capacity(data_count as usize);
                    for _ in 0..data_count {
                        values.push(cursor.read_f32()?);
                    }

                    UserDataContent::Float(values)
                }
                3 => {
                    let mut blocks = Vec::with_capacity(data_count as usize);

                    for _ in 0..data_count {
                        let base_offset = cursor.pos;

                        let _reserve0 = cursor.read_u16()?;
                        let count = cursor.read_u16()?;
                        let offset = cursor.read_u32()?;

                        let post_header_point = cursor.pos;

                        cursor.seek(base_offset + offset as usize)?;

                        let mut data_array = Vec::with_capacity(count as usize);

                        for _ in 0..count {
                            let data = if is_pane {
                                SystemData::Pane(PaneData::parse(cursor)?)
                            } else {
                                SystemData::Layout(LayoutData::parse(cursor, post_header_point)?)
                            };

                            data_array.push(data);
                        }

                        blocks.push(data_array)
                    }

                    UserDataContent::SystemData(blocks)
                }
                _ => {
                    return Err(FormatError::UnknownTag {
                        enum_name: "UserData",
                        tag: type_tag as u32,
                        offset: cursor.pos,
                    });
                }
            }
        } else {
            match type_tag {
                0 => UserDataContent::String(String::new()),
                1 => UserDataContent::S32(Vec::new()),
                2 => UserDataContent::Float(Vec::new()),
                _ => UserDataContent::SystemData(Vec::new()),
            }
        };

        cursor.seek(base_offset + name_offset as usize)?;
        let o_name = cursor.read_null_terminated_string()?;

        cursor.seek(restore_point)?;

        Ok(Self { o_name, content })
    }
}
