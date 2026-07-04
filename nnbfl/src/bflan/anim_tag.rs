use serde::{Deserialize, Serialize};

use crate::{
    bflyt::constants::MAGIC_USERDATA,
    core::{Cursor, FormatError, Writer},
    ui2d::userdata::UserDataArray,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaneAnimTag {
    pub tag_order: u16,
    pub start_frame: u16,
    pub end_frame: u16,
    pub is_descending_bind: bool,

    pub o_name: String,
    pub groups: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub user_data: Option<UserDataArray>,
}

impl PaneAnimTag {
    pub fn parse(cursor: &mut Cursor, section_start: usize) -> Result<Self, FormatError> {
        let tag_order = cursor.read_u16()?;
        let group_count = cursor.read_u16()?;
        let name_offset = cursor.read_u32()?;
        let group_array_offset = cursor.read_u32()?;
        let user_data_section_offset = cursor.read_u32()?;
        let start_frame = cursor.read_u16()?;
        let end_frame = cursor.read_u16()?;
        let is_descending_bind = cursor.read_u8()? != 0;
        let _reserve0 = cursor.read_u8()?;
        let _reserve1 = cursor.read_u16()?;

        let mut o_name = String::new();
        let mut groups = Vec::with_capacity(group_count as usize);
        let mut user_data = None;

        if name_offset > 0 {
            cursor.seek(section_start + name_offset as usize)?;
            o_name = cursor.read_null_terminated_string()?;
        }

        if group_count > 0 && group_array_offset > 0 {
            cursor.seek(section_start + group_array_offset as usize)?;
            for _ in 0..group_count {
                let group_name = cursor.read_fixed_string(0x21)?;
                let _flag = cursor.read_u8()?;
                let _reserve0 = cursor.read_u16()?;
                groups.push(group_name);
            }
        }

        if user_data_section_offset > 0 {
            cursor.seek(section_start + user_data_section_offset as usize)?;

            let embed_magic = cursor.read_u32()?;
            let _embed_size = cursor.read_u32()?;

            if embed_magic == MAGIC_USERDATA {
                user_data = Some(UserDataArray::parse(cursor, false)?);
            }
        }

        Ok(Self {
            tag_order,
            start_frame,
            end_frame,
            is_descending_bind,
            o_name,
            groups,
            user_data,
        })
    }

    pub fn serialize(&self, writer: &mut Writer, section_start: usize) {
        writer.mark("PaneAnimTag");
        writer.write_u16(self.tag_order);
        writer.write_u16(self.groups.len() as u16);

        let name_offset_pos = writer.write_placeholder_u32();
        let group_offset_pos = writer.write_placeholder_u32();
        let user_data_offset_pos = writer.write_placeholder_u32();

        writer.write_u16(self.start_frame);
        writer.write_u16(self.end_frame);
        writer.write_u8(self.is_descending_bind.into());
        writer.write_u8(0);
        writer.write_u16(0);

        writer.patch_u32(name_offset_pos, (writer.pos() - section_start) as u32);
        writer.write_null_terminated_string(&self.o_name);
        writer.align(4);

        writer.patch_u32(group_offset_pos, (writer.pos() - section_start) as u32);
        for group in &self.groups {
            writer.write_fixed_string(&group, 0x21);
            writer.write_u8(0);
            writer.write_u16(0);
        }

        if let Some(user_data) = &self.user_data {
            writer.align(4);
            writer.patch_u32(user_data_offset_pos, (writer.pos() - section_start) as u32);

            let embed_start = writer.pos();
            writer.write_u32(MAGIC_USERDATA);
            let embed_size_pos = writer.write_placeholder_u32();

            user_data.serialize(writer);

            let embed_size = (writer.pos() - embed_start) as u32;
            writer.patch_u32(embed_size_pos, embed_size);
        }
    }
}
