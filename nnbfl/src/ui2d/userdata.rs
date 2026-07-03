use serde::{Deserialize, Serialize};

use crate::{
    core::{Cursor, FormatError, Writer},
    ui2d::{systemdata::ResUi2dSystemDataArray, types::Ui2dUserDataType},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResUi2dUserDataSection {
    pub user_data: Vec<ResUi2dUserData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResUi2dUserData {
    pub data_type: Ui2dUserDataType,
    pub data_array: Vec<ResUi2dUserDataInner>,
    pub o_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResUi2dUserDataInner {
    Float(f32),
    S32(i32),
    String(String),
    SystemData(ResUi2dSystemDataArray),
}

impl ResUi2dUserDataSection {
    pub fn parse(cursor: &mut Cursor, is_pane: bool) -> Result<Self, FormatError> {
        let user_data_count = cursor.read_u16()?;
        let _reserve0 = cursor.read_u16()?;
        let mut user_data = Vec::new();

        for _ in 0..user_data_count {
            user_data.push(ResUi2dUserData::parse(cursor, is_pane)?)
        }

        Ok(Self { user_data })
    }

    pub fn serialize(&self, writer: &mut Writer) {
        writer.mark("UserData (section)");
        writer.write_u16(self.user_data.len() as u16);
        writer.write_u16(0);

        let mut slots: Vec<(usize, usize, usize)> = Vec::new();

        for data in &self.user_data {
            let entry_base = writer.pos();
            let name_ph = writer.write_placeholder_u32();
            let data_ph = writer.write_placeholder_u32();

            let data_type_val = match data.data_type {
                Ui2dUserDataType::String => 0,
                Ui2dUserDataType::S32 => 1,
                Ui2dUserDataType::Float => 2,
                Ui2dUserDataType::SystemData => 3,
                Ui2dUserDataType::Invalid => 4,
            };

            if data_type_val == 0 {
                if let ResUi2dUserDataInner::String(str) = &data.data_array[0] {
                    writer.write_u16(str.len() as u16)
                }
            } else {
                writer.write_u16(data.data_array.len() as u16);
            }

            writer.write_u8(data_type_val);
            writer.write_u8(0);

            slots.push((entry_base, name_ph, data_ph));
        }

        let type_order: &[fn(&ResUi2dUserDataInner) -> bool] = &[
            |i| matches!(i, ResUi2dUserDataInner::SystemData(_)),
            |i| {
                matches!(
                    i,
                    ResUi2dUserDataInner::Float(_) | ResUi2dUserDataInner::S32(_)
                )
            },
        ];

        for type_check in type_order {
            for (i, data) in self.user_data.iter().enumerate() {
                if data.data_array.is_empty() {
                    continue;
                }
                if !data.data_array.iter().any(type_check) {
                    continue;
                }

                let (entry_base, _name_ph, data_ph) = slots[i];
                writer.patch_u32(data_ph, (writer.pos() - entry_base) as u32);

                for item in &data.data_array {
                    match item {
                        ResUi2dUserDataInner::Float(f) => writer.write_f32(*f),
                        ResUi2dUserDataInner::S32(s) => writer.write_i32(*s),
                        ResUi2dUserDataInner::SystemData(sys) => sys.serialize(writer),
                        // strings are handled afterwards
                        _ => {}
                    }
                }
            }
        }

        for (i, data) in self.user_data.iter().enumerate() {
            if data.data_array.is_empty() {
                let (_entry_base, _name_ph, data_ph) = slots[i];
                writer.patch_u32(data_ph, 0);
            }
        }

        for (i, data) in self.user_data.iter().enumerate() {
            let (entry_base, name_ph, data_ph) = slots[i];

            if data.data_type == Ui2dUserDataType::String {
                if !data.data_array.is_empty() {
                    writer.patch_u32(data_ph, (writer.pos() - entry_base) as u32);
                    for item in &data.data_array {
                        if let ResUi2dUserDataInner::String(s) = item {
                            writer.write_fixed_string(s, s.len());
                            writer.write_u8(0);
                        }
                    }
                } else {
                    writer.patch_u32(data_ph, 0);
                }
            } else if data.data_array.is_empty() {
                writer.patch_u32(data_ph, 0);
            }

            writer.patch_u32(name_ph, (writer.pos() - entry_base) as u32);
            writer.write_null_terminated_string(&data.o_name);
        }

        writer.align(4);
    }
}

impl ResUi2dUserData {
    pub fn parse(cursor: &mut Cursor, is_pane: bool) -> Result<Self, FormatError> {
        let base_offset = cursor.pos;

        let name_offset = cursor.read_u32()?;
        let data_array_offset = cursor.read_u32()?;
        let data_count = cursor.read_u16()?;

        let data_type = cursor.read_u8()?.into();
        let _reserve0 = cursor.read_u8()?;
        let mut data = Self {
            data_type,
            data_array: Vec::new(),
            o_name: String::new(),
        };

        let restore_point = cursor.pos;

        if data_array_offset > 0 {
            cursor.seek(base_offset + data_array_offset as usize)?;

            match data.data_type {
                Ui2dUserDataType::Float => {
                    for _ in 0..data_count {
                        data.data_array
                            .push(ResUi2dUserDataInner::Float(cursor.read_f32()?));
                    }
                }
                Ui2dUserDataType::S32 => {
                    for _ in 0..data_count {
                        data.data_array
                            .push(ResUi2dUserDataInner::S32(cursor.read_i32()?));
                    }
                }
                Ui2dUserDataType::String => {
                    let str_data = cursor.read_string(data_count as usize)?;
                    data.data_array.push(ResUi2dUserDataInner::String(str_data));
                }
                Ui2dUserDataType::SystemData => {
                    for _ in 0..data_count {
                        let sys_data = ResUi2dSystemDataArray::parse(cursor, is_pane)?;
                        data.data_array
                            .push(ResUi2dUserDataInner::SystemData(sys_data));
                    }
                }
                _ => {}
            }
        }

        cursor.seek(base_offset + name_offset as usize)?;
        data.o_name = cursor.read_null_terminated_string()?;

        cursor.seek(restore_point)?;

        Ok(data)
    }
}
