use crate::{
    bflan::{anim_info::PaneAnimInfo, anim_tag::ResBflanPaneAnimTag, constants::*},
    bflyt::constants::*,
    core::{
        Cursor, FormatError, ReadWriteable, SectionHeader, VersionFormat, Writer, tchar_code32,
    },
    ui2d::userdata::ResUi2dUserDataSection,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Bflan {
    pub endianness: u16,
    pub version: VersionFormat,
    pub anim_tag: ResBflanPaneAnimTag,
    pub anim_info: PaneAnimInfo,
    pub user_data: Option<ResUi2dUserDataSection>,
}

impl ReadWriteable for Bflan {
    const EXTENSION: &'static str = "bflan";

    fn parse(file: &[u8]) -> Result<Self, FormatError> {
        let mut cursor = Cursor {
            data: file,
            pos: 0,
            ..Default::default()
        };

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
        let version = VersionFormat::parse(&mut cursor)?;
        cursor.version = version;
        let _file_size = cursor.read_u32()?;
        let section_count = cursor.read_u32()?;

        if (header_size as usize) > file.len() {
            return Err(FormatError::InvalidHeaderSize {
                specified_size: header_size as usize,
                actual_size: file.len(),
            });
        }

        let mut anim_tag = None;
        let mut anim_info = None;
        let mut user_data = None;

        for _ in 0..section_count {
            let section = BflanSections::parse(&mut cursor)?;

            match section {
                BflanSections::PaneAnimTag(t) => anim_tag = Some(t),
                BflanSections::PaneAnimInfo(i) => anim_info = Some(i),
                BflanSections::UserData(usd) => user_data = Some(usd),
                _ => {}
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

    fn write(&self) -> Writer {
        let mut writer = Writer::new();
        writer.version = self.version;

        writer.mark("File header");
        writer.write_bytes(b"FLAN");
        writer.write_u16(self.endianness);
        let header_size = writer.write_placeholder_u16();
        self.version.serialize(&mut writer);

        let file_size_pos = writer.write_placeholder_u32();
        let mut section_count = 2;
        section_count += self.user_data.is_some() as u32;
        writer.write_u32(section_count);

        writer.patch_u16(header_size, writer.pos() as u16);

        BflanSectionsRef::PaneAnimTag(&self.anim_tag).serialize(&mut writer);
        BflanSectionsRef::PaneAnimInfo(&self.anim_info).serialize(&mut writer);

        // TODO: is user data here, or earlier?
        if let Some(user_data) = &self.user_data {
            BflanSectionsRef::UserData(user_data).serialize(&mut writer);
        }

        let total_size = writer.pos() as u32;
        writer.patch_u32(file_size_pos, total_size);

        writer
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum BflanSections {
    UserData(ResUi2dUserDataSection),
    PaneAnimTag(ResBflanPaneAnimTag),
    PaneAnimInfo(PaneAnimInfo),
    Unknown(SectionHeader, Vec<u8>),
}

impl BflanSections {
    pub fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.pos;

        let header = SectionHeader::parse(cursor)?;
        let section = match header.magic {
            MAGIC_USERDATA => Self::UserData(ResUi2dUserDataSection::parse(cursor, false)?),
            MAGIC_ANIMTAG => Self::PaneAnimTag(ResBflanPaneAnimTag::parse(cursor, section_start)?),
            MAGIC_ANIMINFO => Self::PaneAnimInfo(PaneAnimInfo::parse(cursor, section_start)?),
            _ => {
                let remaining_payload = (header.size - 8) as usize;
                let data = cursor.read_bytes(remaining_payload)?.to_vec();
                Self::Unknown(header, data)
            }
        };

        cursor.seek(section_start + header.size as usize)?;

        Ok(section)
    }
}

enum BflanSectionsRef<'a> {
    UserData(&'a ResUi2dUserDataSection),
    PaneAnimTag(&'a ResBflanPaneAnimTag),
    PaneAnimInfo(&'a PaneAnimInfo),
}

impl<'a> BflanSectionsRef<'a> {
    pub fn serialize(&self, writer: &mut Writer) {
        let section_start = writer.pos();

        writer.mark("Section (header)");
        match self {
            Self::UserData(_) => writer.write_u32(MAGIC_USERDATA),
            Self::PaneAnimTag(_) => writer.write_u32(MAGIC_ANIMTAG),
            Self::PaneAnimInfo(_) => writer.write_u32(MAGIC_ANIMINFO),
        }

        let size_pos = writer.write_placeholder_u32();

        writer.mark("Section (data)");
        match self {
            Self::UserData(data) => data.serialize(writer),
            Self::PaneAnimTag(tag) => tag.serialize(writer, section_start),
            Self::PaneAnimInfo(info) => info.serialize(writer, section_start),
        }

        writer.align(4);

        let size = (writer.pos() - section_start) as u32;
        writer.patch_u32(size_pos, size);
    }
}
