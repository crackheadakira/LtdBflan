use nnbfl::bflyt::{file::BflytSection, pane::Pane};

pub trait Displaying {
    fn section_color(&self) -> [f32; 4];
    fn kind_name(&self) -> &'static str;
    fn get_base_pane(&self) -> Option<&Pane>;
    fn get_base_pane_mut(&mut self) -> Option<&mut Pane>;
    fn pane_name(&self) -> String;
}

impl Displaying for BflytSection {
    fn section_color(&self) -> [f32; 4] {
        match self {
            BflytSection::Pane(_) => [0.55, 0.76, 0.98, 0.55],
            BflytSection::PicturePane(_) => [0.40, 0.85, 0.55, 0.55],
            BflytSection::TextBoxPane(_) => [0.98, 0.82, 0.35, 0.55],
            BflytSection::WindowPane(_) => [0.85, 0.45, 0.85, 0.55],
            BflytSection::PartsPane(_) => [0.98, 0.55, 0.35, 0.55],
            BflytSection::AlignmentPane(_) => [0.35, 0.90, 0.90, 0.55],
            BflytSection::CapturePane(_) => [0.90, 0.35, 0.50, 0.55],
            BflytSection::BoundingPane(_) => [0.75, 0.75, 0.75, 0.30],
            BflytSection::ScissorPane(_) => [0.95, 0.95, 0.35, 0.40],
            _ => [0.60, 0.60, 0.60, 0.30],
        }
    }

    fn kind_name(&self) -> &'static str {
        match self {
            BflytSection::UserData(_) => "UserData",
            BflytSection::Layout(_) => "Layout",
            BflytSection::TextureList(_) => "TextureList",
            BflytSection::FontList(_) => "FontList",
            BflytSection::MaterialList(_) => "MaterialList",
            BflytSection::CaptureTextureList(_) => "CaptureTextureList",
            BflytSection::VectorGraphicsList(_) => "VectorGraphicsList",
            BflytSection::Pane(_) => "NullPane",
            BflytSection::PicturePane(_) => "PicturePane",
            BflytSection::TextBoxPane(_) => "TextBoxPane",
            BflytSection::WindowPane(_) => "WindowPane",
            BflytSection::PartsPane(_) => "PartsPane",
            BflytSection::AlignmentPane(_) => "AlignmentPane",
            BflytSection::CapturePane(_) => "CapturePane",
            BflytSection::BoundingPane(_) => "BoundingPane",
            BflytSection::ScissorPane(_) => "ScissorPane",
            BflytSection::Group(_) => "Group",
            BflytSection::ControlSource(_) => "ControlSource",
            BflytSection::PaneStart => "PaneStart",
            BflytSection::PaneEnd => "PaneEnd",
            BflytSection::GroupStart => "GroupStart",
            BflytSection::GroupEnd => "GroupEnd",
            BflytSection::Unknown(_, _) => "Unknown",
            BflytSection::ShapeInfoList(_) => "ShapeInfoList",
        }
    }

    fn get_base_pane(&self) -> Option<&Pane> {
        match self {
            BflytSection::Pane(p)
            | BflytSection::BoundingPane(p)
            | BflytSection::ScissorPane(p) => Some(p),
            BflytSection::PicturePane(p) => Some(&p.base),
            BflytSection::TextBoxPane(p) => Some(&p.base),
            BflytSection::WindowPane(p) => Some(&p.base),
            BflytSection::PartsPane(p) => Some(&p.base),
            BflytSection::AlignmentPane(p) => Some(&p.base),
            BflytSection::CapturePane(p) => Some(p),
            _ => None,
        }
    }

    fn get_base_pane_mut(&mut self) -> Option<&mut Pane> {
        match self {
            BflytSection::Pane(p)
            | BflytSection::BoundingPane(p)
            | BflytSection::ScissorPane(p) => Some(p),
            BflytSection::PicturePane(p) => Some(&mut p.base),
            BflytSection::TextBoxPane(p) => Some(&mut p.base),
            BflytSection::WindowPane(p) => Some(&mut p.base),
            BflytSection::PartsPane(p) => Some(&mut p.base),
            BflytSection::AlignmentPane(p) => Some(&mut p.base),
            BflytSection::CapturePane(p) => Some(p),
            _ => None,
        }
    }

    fn pane_name(&self) -> String {
        if let Some(p) = self.get_base_pane() {
            let name = p.pane_name.trim_end_matches('\0');
            if !name.is_empty() {
                return name.to_string();
            }
        }
        self.kind_name().to_string()
    }
}
