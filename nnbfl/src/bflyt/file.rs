use serde::{Deserialize, Serialize};

use crate::{
    bflyt::{
        constants::{section_magic, section_name},
        list::{
            CaptureTextureList, ControlSource, FontList, Group, Layout, MaterialList,
            ShapeInfoList, TextureList, VectorGraphicsList,
        },
        pane::{AlignmentPane, Pane, PartsPane, PicturePane, TextBoxPane, WindowPane},
    },
    core::{
        Cursor, FileReadWriteable, FormatError, ReadWriteable, SectionHeader, SectionMagic,
        VersionFormat, Writer, tchar_code32,
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

impl Default for BflytSection {
    fn default() -> Self {
        Self::Layout(Default::default())
    }
}

impl ReadWriteable for BflytSection {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.pos;
        let header = SectionHeader::parse(cursor)?;
        let end = section_start + header.section_size as usize;
        cursor.section_start = Some(section_start);

        let section = match header.magic {
            SectionMagic::UserData => {
                let s = UserDataArray::parse(cursor)?;
                BflytSection::UserData(s)
            }
            SectionMagic::Layout => {
                let s = Layout::parse(cursor)?;

                if !cursor.is_embed {
                    cursor.last_was_pane = false;
                }

                BflytSection::Layout(s)
            }
            SectionMagic::TextureList => {
                let s = TextureList::parse(cursor)?;
                BflytSection::TextureList(s)
            }
            SectionMagic::FontList => {
                let s = FontList::parse(cursor)?;
                BflytSection::FontList(s)
            }
            SectionMagic::MaterialList => {
                let s = MaterialList::parse(cursor)?;
                BflytSection::MaterialList(s)
            }
            SectionMagic::CaptureTextureList => {
                let s = CaptureTextureList::parse(cursor)?;
                BflytSection::CaptureTextureList(s)
            }
            SectionMagic::VectorGraphicsList => {
                let s = VectorGraphicsList::parse(cursor)?;
                BflytSection::VectorGraphicsList(s)
            }
            SectionMagic::PaneStart => BflytSection::PaneStart,
            SectionMagic::PaneEnd => BflytSection::PaneEnd,
            SectionMagic::GroupStart => BflytSection::GroupStart,
            SectionMagic::GroupEnd => BflytSection::GroupEnd,
            SectionMagic::Pane => {
                let s = Pane::parse(cursor)?;

                if !cursor.is_embed {
                    cursor.last_was_pane = true;
                }

                BflytSection::Pane(s)
            }
            SectionMagic::PicturePane => {
                let s = PicturePane::parse(cursor)?;
                BflytSection::PicturePane(s)
            }
            SectionMagic::TextBoxPane => {
                let s = TextBoxPane::parse(cursor)?;
                BflytSection::TextBoxPane(s)
            }
            SectionMagic::WindowPane => {
                let s = WindowPane::parse(cursor)?;
                BflytSection::WindowPane(s)
            }
            SectionMagic::PartsPane => {
                let s = PartsPane::parse(cursor)?;
                BflytSection::PartsPane(s)
            }
            SectionMagic::AlignmentPane => {
                let s = AlignmentPane::parse(cursor)?;
                BflytSection::AlignmentPane(s)
            }
            SectionMagic::CapturePane => {
                let s = Pane::parse(cursor)?;
                BflytSection::CapturePane(s)
            }
            SectionMagic::BoundingPane => {
                let s = Pane::parse(cursor)?;
                BflytSection::BoundingPane(s)
            }
            SectionMagic::ScissorPane => {
                let s = Pane::parse(cursor)?;
                BflytSection::ScissorPane(s)
            }
            SectionMagic::Group => {
                let s = Group::parse(cursor)?;
                BflytSection::Group(s)
            }
            SectionMagic::ControlSource => {
                let s = ControlSource::parse(cursor)?;
                BflytSection::ControlSource(s)
            }
            SectionMagic::ShapeInfo => {
                let s = ShapeInfoList::parse(cursor)?;
                BflytSection::ShapeInfoList(s)
            }
            _ => {
                println!("Got unknown pane w/ magic: {:?}", header.magic);

                let data_size = (header.section_size as usize).saturating_sub(8);
                let data = cursor
                    .read_bytes(data_size.min(end.saturating_sub(cursor.pos)))?
                    .to_vec();

                BflytSection::Unknown(header, data)
            }
        };

        cursor.seek(end)?;
        cursor.is_embed = false;
        cursor.section_start = None;

        Ok(section)
    }

    fn write(&self, writer: &mut Writer) {
        let section_start = writer.pos();
        let magic = section_magic(self);
        writer.section_start = Some(section_start);

        writer.write_u32(magic.into());
        let size_pos = writer.write_placeholder_u32();

        writer.mark(&format!("BflytSection {}", section_name(self)));

        match self {
            Self::UserData(s) => s.write(writer),
            Self::Layout(s) => s.write(writer),
            Self::TextureList(s) => s.write(writer),
            Self::FontList(s) => s.write(writer),
            Self::MaterialList(s) => s.write(writer),
            Self::CaptureTextureList(s) => s.write(writer),
            Self::VectorGraphicsList(s) => s.write(writer),
            Self::Pane(s) | Self::BoundingPane(s) | Self::ScissorPane(s) => s.write(writer),
            Self::PicturePane(s) => s.write(writer),
            Self::TextBoxPane(s) => s.write(writer),
            Self::WindowPane(s) => s.write(writer),
            Self::PartsPane(s) => s.write(writer),
            Self::AlignmentPane(s) => s.write(writer),
            Self::CapturePane(s) => s.write(writer),
            Self::Group(s) => s.write(writer),
            Self::ControlSource(s) => s.write(writer),
            Self::ShapeInfoList(s) => s.write(writer),
            Self::Unknown(_, data) => writer.write_bytes(data),
            Self::PaneStart | Self::PaneEnd | Self::GroupStart | Self::GroupEnd => {}
        }

        writer.align(4);
        writer.section_start = None;

        let size = (writer.pos() - section_start) as u32;
        writer.patch_u32(size_pos, size);
    }
}

impl BflytSection {
    pub fn write_raw_block<F>(writer: &mut Writer, magic: SectionMagic, write_body: F)
    where
        F: FnOnce(&mut Writer),
    {
        let section_start = writer.pos();
        writer.section_start = Some(section_start);

        writer.write_u32(magic as u32);
        let size_pos = writer.write_placeholder_u32();

        write_body(writer);

        writer.align(4);
        writer.section_start = None;

        let size = (writer.pos() - section_start) as u32;
        writer.patch_u32(size_pos, size);
    }

    fn is_pane_type(&self) -> bool {
        matches!(
            self,
            Self::Pane(_)
                | Self::PicturePane(_)
                | Self::TextBoxPane(_)
                | Self::WindowPane(_)
                | Self::PartsPane(_)
                | Self::AlignmentPane(_)
                | Self::CapturePane(_)
                | Self::BoundingPane(_)
                | Self::ScissorPane(_)
                | Self::PaneStart
                | Self::PaneEnd
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub root_group: GroupElement,
    pub control_source: Option<ControlSourceElement>,
}

enum StackFrame {
    Root(Vec<BflytNode>),
    Pane(Box<PaneElement>),
    Group(GroupElement),
}

impl FileReadWriteable for Bflyt {
    const INPUT_EXTENSION: &'static str = "bflyt";
}

impl ReadWriteable for Bflyt {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
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

        cursor.seek(header_size as usize)?;

        let mut layout = None;
        let mut user_data = None;
        let mut texture_list = None;
        let mut font_list = None;
        let mut material_list = None;
        let mut capture_texture_list = None;

        let mut tree_stack = vec![StackFrame::Root(Vec::new())];

        let mut has_entered_hierarchy = false;
        for i in 0..section_count {
            let section =
                BflytSection::parse(cursor).map_err(|e| FormatError::SectionCountMismatch {
                    expected: section_count,
                    actual: i,
                    source: Box::new(e),
                })?;

            match section {
                BflytSection::Layout(l) => layout = Some(l),
                BflytSection::TextureList(t) => texture_list = Some(t),
                BflytSection::FontList(f) => font_list = Some(f),
                BflytSection::MaterialList(m) => material_list = Some(m),
                BflytSection::CaptureTextureList(c) => capture_texture_list = Some(c),

                BflytSection::UserData(usd) if !has_entered_hierarchy && user_data.is_none() => {
                    user_data = Some(usd);
                }

                BflytSection::PaneStart => {
                    has_entered_hierarchy = true;

                    let pane_el = match tree_stack.last_mut() {
                        Some(StackFrame::Root(layer)) => layer.pop(),
                        Some(StackFrame::Pane(boxed_pane)) => boxed_pane.children.pop(),
                        _ => None,
                    };

                    if let Some(BflytNode::Pane(pane_el)) = pane_el {
                        tree_stack.push(StackFrame::Pane(Box::new(pane_el)));
                        continue;
                    }

                    return Err(FormatError::InvalidHierarchyChange(
                        "PaneStart encountered without a preceding Pane section",
                    ));
                }

                BflytSection::PaneEnd => {
                    has_entered_hierarchy = true;

                    if let Some(StackFrame::Pane(finished_pane)) = tree_stack.pop() {
                        match tree_stack.last_mut() {
                            Some(StackFrame::Root(layer)) => {
                                layer.push(BflytNode::Pane(*finished_pane))
                            }
                            Some(StackFrame::Pane(parent)) => {
                                parent.children.push(BflytNode::Pane(*finished_pane))
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        return Err(FormatError::InvalidHierarchyChange(
                            "Mismatched PaneEnd encountered",
                        ));
                    }
                }

                BflytSection::GroupStart => {
                    has_entered_hierarchy = true;

                    let group_el = match tree_stack.last_mut() {
                        Some(StackFrame::Root(layer)) => layer.pop(),
                        Some(StackFrame::Pane(pane_el)) => pane_el.children.pop(),
                        _ => None,
                    };

                    if let Some(BflytNode::Group(group_el)) = group_el {
                        tree_stack.push(StackFrame::Group(group_el));
                        continue;
                    }

                    return Err(FormatError::InvalidHierarchyChange(
                        "GroupStart encountered without a preceding Group section",
                    ));
                }

                BflytSection::GroupEnd => {
                    has_entered_hierarchy = true;
                    if let Some(StackFrame::Group(finished_group)) = tree_stack.pop() {
                        match tree_stack.last_mut() {
                            Some(StackFrame::Root(layer)) => {
                                layer.push(BflytNode::Group(finished_group));
                            }
                            Some(StackFrame::Pane(parent)) => {
                                parent.children.push(BflytNode::Group(finished_group));
                            }
                            _ => unreachable!("GroupEnd cannot occur inside another Group frame"),
                        }
                        continue;
                    } else {
                        return Err(FormatError::InvalidHierarchyChange(
                            "Mismatched GroupEnd encountered",
                        ));
                    }
                }

                s => {
                    has_entered_hierarchy = true;

                    if let Some(StackFrame::Group(root_group)) = tree_stack.last_mut() {
                        if let BflytSection::Group(sub_group) = s {
                            root_group.children.push(sub_group);
                        }
                        continue;
                    }

                    let current_children = match tree_stack.last_mut() {
                        Some(StackFrame::Root(layer)) => layer,
                        Some(StackFrame::Pane(p)) => &mut p.children,
                        _ => unreachable!(),
                    };

                    match s {
                        BflytSection::UserData(usd) => {
                            if let Some(BflytNode::Pane(pane_el)) = current_children.last_mut() {
                                pane_el.user_data = Some(usd);
                            } else if let Some(BflytNode::ControlSource(cs_el)) =
                                current_children.last_mut()
                            {
                                cs_el.user_data = Some(usd);
                            } else {
                                current_children
                                    .push(BflytNode::RootSection(BflytSection::UserData(usd)));
                            }
                        }
                        BflytSection::Group(g_data) => {
                            current_children.push(BflytNode::Group(GroupElement {
                                data: g_data,
                                children: Vec::new(),
                            }));
                        }
                        BflytSection::ControlSource(cs_data) => {
                            current_children.push(BflytNode::ControlSource(ControlSourceElement {
                                data: cs_data,
                                user_data: None,
                            }));
                        }

                        other if other.is_pane_type() => {
                            current_children.push(BflytNode::Pane(PaneElement {
                                data: other,
                                user_data: None,
                                children: Vec::new(),
                            }));
                        }
                        other => {
                            current_children.push(BflytNode::RootSection(other));
                        }
                    }
                }
            }
        }

        let mut nodes = match tree_stack.pop() {
            Some(StackFrame::Root(layer)) => layer,
            _ => {
                return Err(FormatError::InvalidHierarchyChange(
                    "Unclosed hierarchy elements remaining at EOF",
                ));
            }
        };

        let root_group = match nodes.iter().position(|n| matches!(n, BflytNode::Group(_))) {
            Some(pos) => {
                if let BflytNode::Group(g) = nodes.remove(pos) {
                    g
                } else {
                    unreachable!()
                }
            }
            None => {
                return Err(FormatError::InvalidHierarchyChange(
                    "Missing expected root group section",
                ));
            }
        };

        let mut control_source = None;
        if let Some(pos) = nodes
            .iter()
            .position(|n| matches!(n, BflytNode::ControlSource(_)))
            && let BflytNode::ControlSource(c) = nodes.remove(pos)
        {
            control_source = Some(c);
        }

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
            root_group,
            control_source,
        };

        bflyt.resolve_names();
        Ok(bflyt)
    }

    fn write(&self, writer: &mut Writer) {
        self.rebuild_indices();
        writer.version = self.version;

        writer.mark("File header");
        writer.write_bytes(b"FLYT");
        writer.write_u16(self.endianness);
        let header_size = writer.write_placeholder_u16();
        self.version.write(writer);

        let file_size_pos = writer.write_placeholder_u32();
        let mut total_sections = self.nodes.iter().map(|n| n.section_count()).sum();
        total_sections += 1;
        total_sections += self.user_data.is_some() as u32;
        total_sections += self.texture_list.is_some() as u32;
        total_sections += self.font_list.is_some() as u32;
        total_sections += self.material_list.is_some() as u32;
        total_sections += self.capture_texture_list.is_some() as u32;
        total_sections += self.root_group.section_count();

        if let Some(c) = &self.control_source {
            total_sections += c.section_count();
        }

        writer.write_u32(total_sections);
        writer.patch_u16(header_size, writer.pos() as u16);

        BflytSection::write_raw_block(writer, SectionMagic::Layout, |w| self.layout.write(w));

        if let Some(usd) = &self.user_data {
            BflytSection::write_raw_block(writer, SectionMagic::UserData, |w| usd.write(w));
        }

        if let Some(t) = &self.texture_list {
            BflytSection::write_raw_block(writer, SectionMagic::TextureList, |w| t.write(w));
        }

        if let Some(f) = &self.font_list {
            BflytSection::write_raw_block(writer, SectionMagic::FontList, |w| f.write(w));
        }

        if let Some(m) = &self.material_list {
            BflytSection::write_raw_block(writer, SectionMagic::MaterialList, |w| m.write(w));
        }

        if let Some(ctl) = &self.capture_texture_list {
            BflytSection::write_raw_block(writer, SectionMagic::CaptureTextureList, |w| {
                ctl.write(w)
            });
        }

        for node in &self.nodes {
            node.serialize(writer);
        }

        self.root_group.serialize(writer);

        if let Some(c) = &self.control_source {
            c.serialize(writer);
        }

        let total = writer.pos() as u32;
        writer.patch_u32(file_size_pos, total);
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
                    if let Some(name) = textures.get(tm.texture_index.get() as usize) {
                        tm.texture_name = name.to_string();
                    }
                }
            }
        }
    }

    fn rebuild_indices(&self) {
        let Some(t_list) = &self.texture_list else {
            return;
        };
        let textures = &t_list.textures;

        if let Some(ml) = &self.material_list {
            for mat in &ml.materials {
                for tm in &mat.tex_maps {
                    let idx = textures
                        .iter()
                        .position(|t| t == &tm.texture_name)
                        .unwrap_or(0) as u16;

                    tm.texture_index.set(idx);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BflytNode {
    RootSection(BflytSection),
    Pane(PaneElement),
    Group(GroupElement),
    ControlSource(ControlSourceElement),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneElement {
    pub data: BflytSection,
    pub user_data: Option<UserDataArray>,
    pub children: Vec<BflytNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GroupElement {
    pub data: Group,
    pub children: Vec<Group>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSourceElement {
    pub data: ControlSource,
    pub user_data: Option<UserDataArray>,
}

impl ControlSourceElement {
    pub fn serialize(&self, writer: &mut Writer) {
        BflytSection::ControlSource(self.data.clone()).write(writer);
        if let Some(usd) = &self.user_data {
            BflytSection::UserData(usd.clone()).write(writer);
        }
    }

    pub fn section_count(&self) -> u32 {
        1 + self.user_data.is_some() as u32
    }
}

impl GroupElement {
    pub fn serialize(&self, writer: &mut Writer) {
        BflytSection::Group(self.data.clone()).write(writer);

        if !self.children.is_empty() {
            BflytSection::GroupStart.write(writer);
            for child in &self.children {
                BflytSection::Group(child.clone()).write(writer);
            }
            BflytSection::GroupEnd.write(writer);
        }
    }

    pub fn section_count(&self) -> u32 {
        let mut count = 1;

        if !self.children.is_empty() {
            count += 2;
            count += self.children.len() as u32;
        }

        count
    }
}

impl BflytNode {
    pub fn serialize(&self, writer: &mut Writer) {
        match self {
            Self::RootSection(section) => section.write(writer),

            Self::Pane(pane) => {
                pane.data.write(writer);
                if let Some(usd) = &pane.user_data {
                    BflytSection::UserData(usd.clone()).write(writer);
                }

                if !pane.children.is_empty() {
                    BflytSection::PaneStart.write(writer);

                    for child in &pane.children {
                        child.serialize(writer);
                    }

                    BflytSection::PaneEnd.write(writer);
                }
            }

            Self::Group(group) => {
                group.serialize(writer);
            }

            Self::ControlSource(cs) => {
                cs.serialize(writer);
            }
        }
    }

    pub fn section_count(&self) -> u32 {
        match self {
            Self::RootSection(_) => 1,

            Self::Pane(pane) => {
                let mut count = 1;

                if pane.user_data.is_some() {
                    count += 1;
                }

                if !pane.children.is_empty() {
                    count += 2;
                    count += pane.children.iter().map(|c| c.section_count()).sum::<u32>();
                }

                count
            }

            Self::Group(group) => group.section_count(),
            Self::ControlSource(cs) => cs.section_count(),
        }
    }
}
