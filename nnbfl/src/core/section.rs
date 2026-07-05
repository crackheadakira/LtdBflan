use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};

use crate::core::{Cursor, FormatError, ReadWriteable, Writer, tchar_code32};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct SectionHeader {
    pub magic: SectionMagic,
    pub section_size: u32,
}

impl ReadWriteable for SectionHeader {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            magic: cursor.read_u32()?.into(),
            section_size: cursor.read_u32()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.write_u32(self.magic.into());
        writer.write_u32(self.section_size);
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    IntoPrimitive,
    FromPrimitive,
)]
#[repr(u32)]
pub enum SectionMagic {
    #[default]
    Invalid = 0,

    UserData = tchar_code32(b"usd1"),
    Layout = tchar_code32(b"lyt1"),
    TextureList = tchar_code32(b"txl1"),
    FontList = tchar_code32(b"fnl1"),
    MaterialList = tchar_code32(b"mat1"),
    CaptureTextureList = tchar_code32(b"ctl1"),
    VectorGraphicsList = tchar_code32(b"vgl1"),

    PaneStart = tchar_code32(b"pas1"),
    PaneEnd = tchar_code32(b"pae1"),
    Pane = tchar_code32(b"pan1"),
    PicturePane = tchar_code32(b"pic1"),
    TextBoxPane = tchar_code32(b"txt1"),
    WindowPane = tchar_code32(b"wnd1"),
    PartsPane = tchar_code32(b"prt1"),
    AlignmentPane = tchar_code32(b"ali1"),
    CapturePane = tchar_code32(b"cpt1"),
    BoundingPane = tchar_code32(b"bnd1"),
    ScissorPane = tchar_code32(b"scr1"),

    GroupStart = tchar_code32(b"grs1"),
    GroupEnd = tchar_code32(b"gre1"),
    Group = tchar_code32(b"grp1"),

    ControlSource = tchar_code32(b"cnt1"),
    ShapeInfo = tchar_code32(b"spi1"),
    AnimInfo = tchar_code32(b"pai1"),
    AnimTag = tchar_code32(b"pat1"),
}
