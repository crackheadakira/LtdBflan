use serde::{Deserialize, Serialize};

use crate::{
    bflyt::{
        constants::*,
        list::{
            CaptureTextureList, ControlSource, FontList, Group, Layout, MaterialList,
            ShapeInfoList, TextureList, VectorGraphicsList,
        },
        pane::{AlignmentPane, Pane, PartsPane, PicturePane, TextBoxPane, WindowPane},
    },
    core::{
        Cursor, FormatError, ReadWriteable, SectionHeader, VersionFormat, Writer, tchar_code32,
    },
    ui2d::userdata::UserDataArray,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BflytSection {
    UserData(UserDataArray),
    Layout(Layout),
    TextureList(TextureList),
    FontList(FontList),
    MaterialList(MaterialList),
    CaptureTextureList(CaptureTextureList),
    VectorGraphicsList(VectorGraphicsList),
    ShapeInfoList(ShapeInfoList),
    Pane(Pane),
    PicturePane(PicturePane),
    TextBoxPane(TextBoxPane),
    WindowPane(WindowPane),
    PartsPane(PartsPane),
    AlignmentPane(AlignmentPane),
    CapturePane(Pane),
    BoundingPane(Pane),
    ScissorPane(Pane),
    Group(Group),
    ControlSource(ControlSource),
    PaneStart,
    PaneEnd,
    GroupStart,
    GroupEnd,

    Unknown(SectionHeader, Vec<u8>),
}

impl BflytSection {
    pub fn parse(
        cursor: &mut Cursor,
        last_was_pane: &mut bool,
        is_embed: bool,
    ) -> Result<Self, FormatError> {
        let section_start = cursor.pos;
        let magic = cursor.read_u32()?;
        let section_size = cursor.read_u32()?;
        let end = section_start + section_size as usize;

        let section = match magic {
            MAGIC_USERDATA => {
                let s = UserDataArray::parse(cursor, *last_was_pane)?;
                BflytSection::UserData(s)
            }
            MAGIC_LAYOUT => {
                let s = Layout::parse(cursor)?;
                if !is_embed {
                    *last_was_pane = false;
                }
                BflytSection::Layout(s)
            }
            MAGIC_TEXTURELIST => {
                let s = TextureList::parse(cursor)?;
                BflytSection::TextureList(s)
            }
            MAGIC_FONTLIST => {
                let s = FontList::parse(cursor)?;
                BflytSection::FontList(s)
            }
            MAGIC_MATERIALLIST => {
                let s = MaterialList::parse(cursor, section_start)?;
                BflytSection::MaterialList(s)
            }
            MAGIC_CAPTURETEXTURELIST => {
                let s = CaptureTextureList::parse(cursor, section_start)?;
                BflytSection::CaptureTextureList(s)
            }
            MAGIC_VECTORGRAPHICSLIST => {
                let s = VectorGraphicsList::parse(cursor, section_start)?;
                BflytSection::VectorGraphicsList(s)
            }
            MAGIC_PANESTART => BflytSection::PaneStart,
            MAGIC_PANEEND => BflytSection::PaneEnd,
            MAGIC_GROUPSTART => BflytSection::GroupStart,
            MAGIC_GROUPEND => BflytSection::GroupEnd,
            MAGIC_PANE => {
                let s = Pane::parse(cursor)?;
                if !is_embed {
                    *last_was_pane = true;
                }
                BflytSection::Pane(s)
            }
            MAGIC_PICTUREPANE => {
                let s = PicturePane::parse(cursor)?;
                BflytSection::PicturePane(s)
            }
            MAGIC_TEXTBOXPANE => {
                let s = TextBoxPane::parse(cursor, section_start)?;
                BflytSection::TextBoxPane(s)
            }
            MAGIC_WINDOWPANE => {
                let s = WindowPane::parse(cursor)?;
                BflytSection::WindowPane(s)
            }
            MAGIC_PARTSPANE => {
                let s = PartsPane::parse(cursor, last_was_pane)?;
                BflytSection::PartsPane(s)
            }
            MAGIC_ALIGNMENTPANE => {
                let s = AlignmentPane::parse(cursor)?;
                BflytSection::AlignmentPane(s)
            }
            MAGIC_CAPTUREPANE => {
                let s = Pane::parse(cursor)?;
                BflytSection::CapturePane(s)
            }
            MAGIC_BOUNDINGPANE => {
                let s = Pane::parse(cursor)?;
                BflytSection::BoundingPane(s)
            }
            MAGIC_SCISSORPANE => {
                let s = Pane::parse(cursor)?;
                BflytSection::ScissorPane(s)
            }
            MAGIC_GROUP => {
                let s = Group::parse(cursor)?;
                BflytSection::Group(s)
            }
            MAGIC_CONTROLSOURCE => {
                let s = ControlSource::parse(cursor)?;
                BflytSection::ControlSource(s)
            }
            MAGIC_SHAPEINFO => {
                let s = ShapeInfoList::parse(cursor)?;
                BflytSection::ShapeInfoList(s)
            }
            _ => {
                println!("Got unknown pane w/ magic: {magic}");

                let data_size = (section_size as usize).saturating_sub(8);
                let data = cursor
                    .read_bytes(data_size.min(end.saturating_sub(cursor.pos)))?
                    .to_vec();

                BflytSection::Unknown(
                    SectionHeader {
                        magic,
                        size: section_size,
                    },
                    data,
                )
            }
        };

        cursor.seek(end)?;

        Ok(section)
    }

    pub fn serialize(&self, writer: &mut Writer) {
        let section_start = writer.pos();
        let magic = section_magic(self);

        writer.write_u32(magic);
        let size_pos = writer.write_placeholder_u32();

        writer.mark(&format!("BflytSection {}", section_name(self)));

        match self {
            Self::UserData(s) => s.serialize(writer),
            Self::Layout(s) => s.serialize(writer),
            Self::TextureList(s) => s.serialize(writer),
            Self::FontList(s) => s.serialize(writer),
            Self::MaterialList(s) => s.serialize(writer, section_start),
            Self::CaptureTextureList(s) => s.serialize(writer, section_start),
            Self::VectorGraphicsList(s) => s.serialize(writer, section_start),
            Self::Pane(s) | Self::BoundingPane(s) | Self::ScissorPane(s) => s.serialize(writer),
            Self::PicturePane(s) => s.serialize(writer),
            Self::TextBoxPane(s) => s.serialize(writer, section_start),
            Self::WindowPane(s) => s.serialize(writer),
            Self::PartsPane(s) => s.serialize(writer, section_start),
            Self::AlignmentPane(s) => s.serialize(writer),
            Self::CapturePane(s) => s.serialize(writer),
            Self::Group(s) => s.serialize(writer),
            Self::ControlSource(s) => s.serialize(writer, section_start),
            Self::Unknown(_, data) => writer.write_bytes(data),
            Self::ShapeInfoList(s) => s.serialize(writer, section_start),
            Self::PaneStart | Self::PaneEnd | Self::GroupStart | Self::GroupEnd => {}
        }

        writer.align(4);

        let size = (writer.pos() - section_start) as u32;
        writer.patch_u32(size_pos, size);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bflyt {
    pub endianness: u16,
    pub version: VersionFormat,

    pub layout: Layout,
    pub user_data: Option<UserDataArray>,
    pub texture_list: Option<TextureList>,
    pub font_list: Option<FontList>,
    pub material_list: Option<MaterialList>,
    pub capture_texture_list: Option<CaptureTextureList>,

    pub nodes: Vec<BflytNode>,
}

impl ReadWriteable for Bflyt {
    const EXTENSION: &'static str = "bflyt";

    fn parse(file: &[u8]) -> Result<Self, FormatError> {
        let mut cursor = Cursor {
            data: file,
            pos: 0,
            ..Default::default()
        };

        let magic = cursor.read_u32()?;
        if magic != tchar_code32(b"FLYT") {
            return Err(FormatError::InvalidMagic {
                expected: "FLYT",
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

        cursor.seek(header_size as usize)?;

        let mut layout = None;
        let mut user_data = None;
        let mut texture_list = None;
        let mut font_list = None;
        let mut material_list = None;
        let mut capture_texture_list = None;

        let mut tree_stack = vec![Vec::new()];

        let mut has_entered_hierarchy = false;
        let mut last_was_pane = false;
        for i in 0..section_count {
            let section =
                BflytSection::parse(&mut cursor, &mut last_was_pane, false).map_err(|e| {
                    FormatError::SectionCountMismatch {
                        expected: section_count,
                        actual: i,
                        source: Box::new(e),
                    }
                })?;

            match section {
                BflytSection::Layout(l) => {
                    layout = Some(l);
                }

                BflytSection::TextureList(t) => {
                    texture_list = Some(t);
                }

                BflytSection::FontList(f) => {
                    font_list = Some(f);
                }

                BflytSection::MaterialList(m) => {
                    material_list = Some(m);
                }

                BflytSection::CaptureTextureList(c) => {
                    capture_texture_list = Some(c);
                }

                BflytSection::UserData(usd) if !has_entered_hierarchy && user_data.is_none() => {
                    user_data = Some(usd);
                }

                BflytSection::PaneStart => {
                    has_entered_hierarchy = true;
                    tree_stack.push(Vec::new());
                }

                BflytSection::PaneEnd => {
                    has_entered_hierarchy = true;
                    if let Some(children) = tree_stack.pop()
                        && let Some(current_layer) = tree_stack.last_mut()
                    {
                        current_layer.push(BflytNode::Panes(children));
                    }
                }
                BflytSection::GroupStart => {
                    has_entered_hierarchy = true;
                    tree_stack.push(Vec::new());
                }
                BflytSection::GroupEnd => {
                    has_entered_hierarchy = true;
                    if let Some(children) = tree_stack.pop()
                        && let Some(current_layer) = tree_stack.last_mut()
                    {
                        current_layer.push(BflytNode::Groups(children));
                    }
                }
                s => {
                    has_entered_hierarchy = true;
                    if let Some(current_layer) = tree_stack.last_mut() {
                        match s {
                            // If it's a UserData section inside the hierarchy, try to bind it to the preceding pane
                            BflytSection::UserData(usd) => {
                                let attached_to_pane = if let Some(BflytNode::Section(prev_sec)) =
                                    current_layer.last()
                                {
                                    matches!(
                                        prev_sec,
                                        BflytSection::Pane(_)
                                            | BflytSection::PicturePane(_)
                                            | BflytSection::TextBoxPane(_)
                                            | BflytSection::WindowPane(_)
                                            | BflytSection::PartsPane(_)
                                            | BflytSection::AlignmentPane(_)
                                            | BflytSection::CapturePane(_)
                                            | BflytSection::BoundingPane(_)
                                            | BflytSection::ScissorPane(_)
                                    )
                                } else {
                                    false
                                };

                                if attached_to_pane {
                                    if let Some(BflytNode::Section(pane_sec)) = current_layer.pop()
                                    {
                                        current_layer.push(BflytNode::PaneWithUserData {
                                            pane: pane_sec,
                                            user_data: usd,
                                        });
                                    }
                                } else {
                                    // Fallback if there's somehow a floating UserData section inside the tree
                                    current_layer
                                        .push(BflytNode::Section(BflytSection::UserData(usd)));
                                }
                            }
                            // All other normal layout sections just get pushed straight into the tree layer
                            other => {
                                current_layer.push(BflytNode::Section(other));
                            }
                        }
                    }
                }
            }
        }

        let nodes = tree_stack.pop().unwrap_or_default();

        if layout.is_none() {
            return Err(FormatError::MissingLayout);
        }

        let mut bflyt = Self {
            endianness,
            version,
            layout: layout.unwrap(),
            user_data,
            texture_list,
            font_list,
            material_list,
            capture_texture_list,
            nodes,
        };

        bflyt.resolve_names();
        Ok(bflyt)
    }

    fn write(&self) -> Writer {
        let mut this = self.clone();
        this.rebuild_indices();

        let mut writer = Writer::new();
        writer.version = this.version;

        writer.mark("File header");
        writer.write_bytes(b"FLYT");
        writer.write_u16(this.endianness);
        let header_size = writer.write_placeholder_u16();
        this.version.serialize(&mut writer);

        let file_size_pos = writer.write_placeholder_u32();
        let mut total_sections = this.nodes.iter().map(|n| n.section_count()).sum();
        total_sections += 1;
        total_sections += this.user_data.is_some() as u32;
        total_sections += this.texture_list.is_some() as u32;
        total_sections += this.font_list.is_some() as u32;
        total_sections += this.material_list.is_some() as u32;
        total_sections += this.capture_texture_list.is_some() as u32;

        writer.write_u32(total_sections);
        writer.patch_u16(header_size, writer.pos() as u16);

        BflytSection::Layout(this.layout.clone()).serialize(&mut writer);

        if let Some(usd) = &this.user_data {
            BflytSection::UserData(usd.clone()).serialize(&mut writer);
        }

        if let Some(t) = &this.texture_list {
            BflytSection::TextureList(t.clone()).serialize(&mut writer);
        }

        if let Some(f) = &this.font_list {
            BflytSection::FontList(f.clone()).serialize(&mut writer);
        }

        if let Some(m) = &this.material_list {
            BflytSection::MaterialList(m.clone()).serialize(&mut writer);
        }

        if let Some(ctl) = &this.capture_texture_list {
            BflytSection::CaptureTextureList(ctl.clone()).serialize(&mut writer);
        }

        for node in &this.nodes {
            node.serialize(&mut writer);
        }

        let total = writer.pos() as u32;
        writer.patch_u32(file_size_pos, total);

        writer
    }
}

impl Bflyt {
    fn resolve_names(&mut self) {
        let Some(t_list) = &self.texture_list else {
            return;
        };

        let textures = &t_list.textures;

        if let Some(ml) = &mut self.material_list {
            for mat in &mut ml.materials {
                for tm in &mut mat.tex_maps {
                    if let Some(name) = textures.get(tm.texture_index as usize) {
                        tm.texture_name = name.to_string();
                    }
                }
            }
        }
    }

    fn rebuild_indices(&mut self) {
        let Some(t_list) = &self.texture_list else {
            return;
        };

        let textures = &t_list.textures;

        if let Some(ml) = &mut self.material_list {
            for mat in &mut ml.materials {
                for tm in &mut mat.tex_maps {
                    tm.texture_index = textures
                        .iter()
                        .position(|t| t == &tm.texture_name)
                        .unwrap_or(0) as u16;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BflytNode {
    Section(BflytSection),
    PaneWithUserData {
        pane: BflytSection,
        user_data: UserDataArray,
    },
    Panes(Vec<BflytNode>),
    Groups(Vec<BflytNode>),
}

impl BflytNode {
    pub fn serialize(&self, writer: &mut Writer) {
        match self {
            Self::Section(section) => section.serialize(writer),

            Self::PaneWithUserData { pane, user_data } => {
                pane.serialize(writer);
                BflytSection::UserData(user_data.clone()).serialize(writer);
            }

            Self::Panes(children) => {
                BflytSection::PaneStart.serialize(writer);

                for child in children {
                    child.serialize(writer);
                }

                BflytSection::PaneEnd.serialize(writer);
            }

            Self::Groups(children) => {
                BflytSection::GroupStart.serialize(writer);

                for child in children {
                    child.serialize(writer);
                }

                BflytSection::GroupEnd.serialize(writer);
            }
        }
    }

    pub fn section_count(&self) -> u32 {
        match self {
            Self::Section(_) => 1,
            Self::PaneWithUserData { .. } => 2,
            Self::Panes(children) | Self::Groups(children) => {
                2 + children.iter().map(|c| c.section_count()).sum::<u32>()
            }
        }
    }
}

fn section_magic(section: &BflytSection) -> u32 {
    match section {
        BflytSection::UserData(_) => MAGIC_USERDATA,
        BflytSection::Layout(_) => MAGIC_LAYOUT,
        BflytSection::TextureList(_) => MAGIC_TEXTURELIST,
        BflytSection::FontList(_) => MAGIC_FONTLIST,
        BflytSection::MaterialList(_) => MAGIC_MATERIALLIST,
        BflytSection::CaptureTextureList(_) => MAGIC_CAPTURETEXTURELIST,
        BflytSection::VectorGraphicsList(_) => MAGIC_VECTORGRAPHICSLIST,
        BflytSection::Pane(_) => MAGIC_PANE,
        BflytSection::PicturePane(_) => MAGIC_PICTUREPANE,
        BflytSection::TextBoxPane(_) => MAGIC_TEXTBOXPANE,
        BflytSection::WindowPane(_) => MAGIC_WINDOWPANE,
        BflytSection::PartsPane(_) => MAGIC_PARTSPANE,
        BflytSection::AlignmentPane(_) => MAGIC_ALIGNMENTPANE,
        BflytSection::CapturePane(_) => MAGIC_CAPTUREPANE,
        BflytSection::BoundingPane(_) => MAGIC_BOUNDINGPANE,
        BflytSection::ScissorPane(_) => MAGIC_SCISSORPANE,
        BflytSection::Group(_) => MAGIC_GROUP,
        BflytSection::ControlSource(_) => MAGIC_CONTROLSOURCE,
        BflytSection::PaneStart => MAGIC_PANESTART,
        BflytSection::PaneEnd => MAGIC_PANEEND,
        BflytSection::GroupStart => MAGIC_GROUPSTART,
        BflytSection::GroupEnd => MAGIC_GROUPEND,
        BflytSection::ShapeInfoList(_) => MAGIC_SHAPEINFO,
        BflytSection::Unknown(h, _) => h.magic,
    }
}
