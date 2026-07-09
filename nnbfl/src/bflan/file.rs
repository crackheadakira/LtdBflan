use crate::{
    bflan::{anim_info::PaneAnimInfo, anim_tag::PaneAnimTag},
    core::{
        Cursor, FileReadWriteable, FormatError, ReadWriteable, SectionHeader, SectionMagic,
        VersionFormat, Writer, tchar_code32,
    },
    ui2d::userdata::UserDataArray,
};

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Bflan {
    pub endianness: u16,
    pub version: VersionFormat,
    pub anim_tag: PaneAnimTag,
    pub anim_info: PaneAnimInfo,
    pub user_data: Option<UserDataArray>,
}

impl FileReadWriteable for Bflan {
    const INPUT_EXTENSION: &'static str = "bflan";
}

impl ReadWriteable for Bflan {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let magic = cursor.read_u32()?;
        if magic != tchar_code32(b"FLAN") {
            return Err(FormatError::InvalidMagic {
                expected: "FLAN",
                found: magic,
                offset: 0,
            });
        }

        let endianness = cursor.read_u16()?;
        let header_size = cursor.read_u16()?;
        let version = VersionFormat::parse(cursor)?;
        cursor.version = version;
        let _file_size = cursor.read_u32()?;
        let section_count = cursor.read_u32()?;

        if (header_size as usize) > cursor.data.len() {
            return Err(FormatError::InvalidHeaderSize {
                specified_size: header_size as usize,
                actual_size: cursor.data.len(),
            });
        }

        let mut anim_tag = None;
        let mut anim_info = None;
        let mut user_data = None;

        for _ in 0..section_count {
            let section = BflanSections::parse(cursor)?;

            match section {
                BflanSections::PaneAnimTag(t) => anim_tag = Some(t),
                BflanSections::PaneAnimInfo(i) => anim_info = Some(i),
                BflanSections::UserData(usd) => user_data = Some(usd),
                BflanSections::Unknown(..) => {}
            }
        }

        if anim_tag.is_none() {
            return Err(FormatError::MissingLayout);
        }

        if anim_info.is_none() {
            return Err(FormatError::MissingLayout);
        }

        Ok(Self {
            anim_tag: anim_tag.unwrap(),
            anim_info: anim_info.unwrap(),
            user_data,
            endianness,
            version,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.version = self.version;

        writer.mark("File header");
        writer.write_bytes(b"FLAN");
        writer.write_u16(self.endianness);
        let header_size = writer.write_placeholder_u16();
        self.version.write(writer);

        let file_size_pos = writer.write_placeholder_u32();
        let mut section_count = 2;
        section_count += self.user_data.is_some() as u32;

        writer.write_u32(section_count);

        writer.patch_u16(header_size, writer.pos() as u16);

        BflanSectionsRef::PaneAnimTag(&self.anim_tag).serialize(writer);
        BflanSectionsRef::PaneAnimInfo(&self.anim_info).serialize(writer);

        // TODO: is user data here, or earlier?
        if let Some(user_data) = &self.user_data {
            BflanSectionsRef::UserData(user_data).serialize(writer);
        }

        let total_size = writer.pos() as u32;
        writer.patch_u32(file_size_pos, total_size);
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum BflanSections {
    UserData(UserDataArray),
    PaneAnimTag(PaneAnimTag),
    PaneAnimInfo(PaneAnimInfo),
    Unknown(SectionHeader, Vec<u8>),
}

impl BflanSections {
    pub fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.pos;
        cursor.section_start = Some(section_start);

        let header = SectionHeader::parse(cursor)?;
        let section = match header.magic {
            SectionMagic::UserData => {
                cursor.last_was_pane = false;
                Self::UserData(UserDataArray::parse(cursor)?)
            }
            SectionMagic::AnimTag => Self::PaneAnimTag(PaneAnimTag::parse(cursor)?),
            SectionMagic::AnimInfo => Self::PaneAnimInfo(PaneAnimInfo::parse(cursor)?),
            _ => {
                let remaining_payload = (header.section_size - 8) as usize;
                let data = cursor.read_bytes(remaining_payload)?.to_vec();
                Self::Unknown(header, data)
            }
        };

        cursor.section_start = None;

        cursor.seek(section_start + header.section_size as usize)?;

        Ok(section)
    }
}

enum BflanSectionsRef<'a> {
    UserData(&'a UserDataArray),
    PaneAnimTag(&'a PaneAnimTag),
    PaneAnimInfo(&'a PaneAnimInfo),
}

impl BflanSectionsRef<'_> {
    pub(crate) fn serialize(&self, writer: &mut Writer) {
        let section_start = writer.pos();
        writer.section_start = Some(section_start);

        writer.mark("Section (header)");
        match self {
            Self::UserData(_) => writer.write_u32(SectionMagic::UserData.into()),
            Self::PaneAnimTag(_) => writer.write_u32(SectionMagic::AnimTag.into()),
            Self::PaneAnimInfo(_) => writer.write_u32(SectionMagic::AnimInfo.into()),
        }

        let size_pos = writer.write_placeholder_u32();

        writer.mark("Section (data)");
        match self {
            Self::UserData(data) => data.write(writer),
            Self::PaneAnimTag(tag) => tag.write(writer),
            Self::PaneAnimInfo(info) => info.write(writer),
        }

        writer.align(4);

        writer.section_start = None;

        let size = (writer.pos() - section_start) as u32;
        writer.patch_u32(size_pos, size);
    }
}
