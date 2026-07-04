use crate::{bflyt::file::BflytSection, core::tchar_code32};

pub const MAGIC_USERDATA: u32 = tchar_code32(b"usd1");
pub const MAGIC_LAYOUT: u32 = tchar_code32(b"lyt1");
pub const MAGIC_TEXTURELIST: u32 = tchar_code32(b"txl1");
pub const MAGIC_FONTLIST: u32 = tchar_code32(b"fnl1");
pub const MAGIC_MATERIALLIST: u32 = tchar_code32(b"mat1");
pub const MAGIC_CAPTURETEXTURELIST: u32 = tchar_code32(b"ctl1");
pub const MAGIC_VECTORGRAPHICSLIST: u32 = tchar_code32(b"vgl1");
pub const MAGIC_PANESTART: u32 = tchar_code32(b"pas1");
pub const MAGIC_PANEEND: u32 = tchar_code32(b"pae1");
pub const MAGIC_PANE: u32 = tchar_code32(b"pan1");
pub const MAGIC_PICTUREPANE: u32 = tchar_code32(b"pic1");
pub const MAGIC_TEXTBOXPANE: u32 = tchar_code32(b"txt1");
pub const MAGIC_WINDOWPANE: u32 = tchar_code32(b"wnd1");
pub const MAGIC_PARTSPANE: u32 = tchar_code32(b"prt1");
pub const MAGIC_ALIGNMENTPANE: u32 = tchar_code32(b"ali1");
pub const MAGIC_CAPTUREPANE: u32 = tchar_code32(b"cpt1");
pub const MAGIC_BOUNDINGPANE: u32 = tchar_code32(b"bnd1");
pub const MAGIC_SCISSORPANE: u32 = tchar_code32(b"scr1");
pub const MAGIC_GROUPSTART: u32 = tchar_code32(b"grs1");
pub const MAGIC_GROUPEND: u32 = tchar_code32(b"gre1");
pub const MAGIC_GROUP: u32 = tchar_code32(b"grp1");
pub const MAGIC_CONTROLSOURCE: u32 = tchar_code32(b"cnt1");
pub const MAGIC_SHAPEINFO: u32 = tchar_code32(b"spi1");

pub fn section_name(section: &BflytSection) -> &'static str {
    match section {
        BflytSection::UserData(_) => "User Data",
        BflytSection::Layout(_) => "Layout",
        BflytSection::TextureList(_) => "Texture List",
        BflytSection::FontList(_) => "Font List",
        BflytSection::MaterialList(_) => "Material List",
        BflytSection::CaptureTextureList(_) => "Capture Texture List",
        BflytSection::VectorGraphicsList(_) => "Vector Graphics List",
        BflytSection::Pane(_) => "Pane",
        BflytSection::PicturePane(_) => "Picture Pane",
        BflytSection::TextBoxPane(_) => "Text Box Pane",
        BflytSection::WindowPane(_) => "Window Pane",
        BflytSection::PartsPane(_) => "Parts Pane",
        BflytSection::AlignmentPane(_) => "Alignment Pane",
        BflytSection::CapturePane(_) => "Capture Pane",
        BflytSection::BoundingPane(_) => "Bounding Pane",
        BflytSection::ScissorPane(_) => "Scissor Pane",
        BflytSection::Group(_) => "Group",
        BflytSection::ControlSource(_) => "Control Source",
        BflytSection::PaneStart => "Pane Start",
        BflytSection::PaneEnd => "Pane End",
        BflytSection::GroupStart => "Group Start",
        BflytSection::GroupEnd => "Group End",
        BflytSection::ShapeInfoList(_) => "Shape Info List",
        BflytSection::Unknown(_, _) => "Unknown",
    }
}
