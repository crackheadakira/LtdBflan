use crate::{bflyt::file::BflytSection, core::SectionMagic};

pub fn section_name(section: &BflytSection) -> &'static str {
    match section {
        BflytSection::UserData(_) => "User Data",
        BflytSection::Layout(_) => "Layout",
        BflytSection::TextureList(_) => "Texture List",
        BflytSection::FontList(_) => "Font List",
        BflytSection::MaterialList(_) => "Material List",
        BflytSection::CaptureTextureList(_) => "Capture Texture List",
        BflytSection::VectorGraphicsList(_) => "Vector Graphics List",
        BflytSection::Pane(_) => "NullPane",
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

pub fn section_magic(section: &BflytSection) -> SectionMagic {
    match section {
        BflytSection::UserData(_) => SectionMagic::UserData,
        BflytSection::Layout(_) => SectionMagic::Layout,
        BflytSection::TextureList(_) => SectionMagic::TextureList,
        BflytSection::FontList(_) => SectionMagic::FontList,
        BflytSection::MaterialList(_) => SectionMagic::MaterialList,
        BflytSection::CaptureTextureList(_) => SectionMagic::CaptureTextureList,
        BflytSection::VectorGraphicsList(_) => SectionMagic::VectorGraphicsList,
        BflytSection::Pane(_) => SectionMagic::Pane,
        BflytSection::PicturePane(_) => SectionMagic::PicturePane,
        BflytSection::TextBoxPane(_) => SectionMagic::TextBoxPane,
        BflytSection::WindowPane(_) => SectionMagic::WindowPane,
        BflytSection::PartsPane(_) => SectionMagic::PartsPane,
        BflytSection::AlignmentPane(_) => SectionMagic::AlignmentPane,
        BflytSection::CapturePane(_) => SectionMagic::CapturePane,
        BflytSection::BoundingPane(_) => SectionMagic::BoundingPane,
        BflytSection::ScissorPane(_) => SectionMagic::ScissorPane,
        BflytSection::Group(_) => SectionMagic::Group,
        BflytSection::ControlSource(_) => SectionMagic::ControlSource,
        BflytSection::PaneStart => SectionMagic::PaneStart,
        BflytSection::PaneEnd => SectionMagic::PaneEnd,
        BflytSection::GroupStart => SectionMagic::GroupStart,
        BflytSection::GroupEnd => SectionMagic::GroupEnd,
        BflytSection::ShapeInfoList(_) => SectionMagic::ShapeInfo,
        BflytSection::Unknown(_, _) => SectionMagic::Invalid,
    }
}
