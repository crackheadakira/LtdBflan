use std::{collections::HashMap, path::Path};

use bitflags::bitflags;
use nnbfl::{
    bflyt::{
        file::{Bflyt, BflytNode, BflytSection, ControlSourceElement, GroupElement, PaneElement},
        flags::{BflytOrigin, BflytParentOrigin},
        list::{CaptureTextureList, FontList, Material, MaterialList, TextureList},
        pane::{BasePaneUsageFlags, Pane, PartsPane, PartsPaneBasicInfo, PicturePane},
    },
    core::FileReadWriteable,
    sarc::file::MagicFiles,
    ui2d::{
        types::{Vector2f, Vector3f},
        userdata::UserDataArray,
    },
};

use crate::{
    anim_state::transform_uv_srt,
    archive_browser::{ArchiveEntry, resolve_nested_package_bytes},
    decompress_if_needed, extract_all_files_recursive,
    renderer::{
        quad::Quad,
        textured_quad::{PaneQuadData, TexturedQuad},
    },
    traits::Displaying,
    ui::general::SUPPORTED_SARC_EXTENSIONS,
};

pub type UvMatrix4x3x2 = [[[f32; 2]; 3]; 4];

bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct DirtyFlags: u8 {
        /// Need to recalculate transforms
        const TRANSFORM = 0x01;

        /// Need to reupload materials to GPU
        const MATERIAL = 0x02;

        /// Need to reupload vertices to GPU
        const VERTICES = 0x04;

        /// Need to rebuild bind group
        const TEXTURE = 0x08;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Corners {
    pub top_left: Vector2f,
    pub top_right: Vector2f,
    pub bottom_left: Vector2f,
    pub bottom_right: Vector2f,
}

impl Corners {
    /// Compute the four world-space corner positions [TL, TR, BL, BR] for a pane
    /// after applying rotation around the pane's pivot point (cx, cy).
    pub fn compute(
        center: Vector2f,
        size: Vector2f,
        origin_x: &BflytOrigin,
        origin_y: &BflytOrigin,
        rotation: Vector3f,
    ) -> Self {
        let lx = match origin_x {
            BflytOrigin::Center => -size.x * 0.5,
            BflytOrigin::LeftTop => 0.0,
            BflytOrigin::RightBottom => -size.x,
        };

        let ly = match origin_y {
            BflytOrigin::Center => -size.y * 0.5,
            BflytOrigin::LeftTop => 0.0,
            BflytOrigin::RightBottom => -size.y,
        };

        let tl = Vector2f::new(lx, ly);
        let tr = Vector2f::new(lx + size.x, ly);
        let bl = Vector2f::new(lx, ly + size.y);
        let br = Vector2f::new(lx + size.x, ly + size.y);

        let transform = |p: Vector2f| -> Vector2f {
            if rotation.x == 0.0 && rotation.y == 0.0 && rotation.z == 0.0 {
                return Vector2f {
                    x: center.x + p.x,
                    y: center.y + p.y,
                };
            }

            let (px, py, pz) = (p.x, p.y, 0.0f32);

            let rx = -rotation.x.to_radians();
            let ry = -rotation.y.to_radians();
            let rz = -rotation.z.to_radians();

            let (sz, cz) = rz.sin_cos();
            let (x1, y1, z1) = (px * cz - py * sz, px * sz + py * cz, pz);

            let (sy, cy) = ry.sin_cos();
            let (x2, y2, z2) = (x1 * cy + z1 * sy, y1, -x1 * sy + z1 * cy);

            let (sx, cx) = rx.sin_cos();
            let (x3, y3, _z3) = (x2, y2 * cx - z2 * sx, y2 * sx + z2 * cx);

            Vector2f {
                x: center.x + x3,
                y: center.y + y3,
            }
        };

        Self {
            top_left: transform(tl),
            top_right: transform(tr),
            bottom_left: transform(bl),
            bottom_right: transform(br),
        }
    }

    pub fn to_array(self) -> [[f32; 2]; 4] {
        [
            [self.top_left.x, self.top_left.y],
            [self.top_right.x, self.top_right.y],
            [self.bottom_left.x, self.bottom_left.y],
            [self.bottom_right.x, self.bottom_right.y],
        ]
    }

    pub fn translate(&self, delta: Vector2f) -> Self {
        Self {
            top_left: Vector2f {
                x: self.top_left.x + delta.x,
                y: self.top_left.y + delta.y,
            },
            top_right: Vector2f {
                x: self.top_right.x + delta.x,
                y: self.top_right.y + delta.y,
            },
            bottom_left: Vector2f {
                x: self.bottom_left.x + delta.x,
                y: self.bottom_left.y + delta.y,
            },
            bottom_right: Vector2f {
                x: self.bottom_right.x + delta.x,
                y: self.bottom_right.y + delta.y,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaneNode {
    pub section: BflytSection,
    pub kind: String,
    pub label: String,
    pub depth: usize,
    pub visible: bool,
    pub parts_source: Option<String>,
    pub pane_idx: usize,

    pub world_pos: Vector2f,
    pub world_size: Vector2f,
    pub world_center: Vector2f,
    pub world_corners: Corners,
    pub parent_anchor: Vector2f,

    pub plain_quad: Quad,
    pub textured_quad: Option<TexturedQuad>,
    pub base_textured_quad: Option<TexturedQuad>,

    pub dirty: DirtyFlags,
    pub children: Vec<PaneNode>,
    pub user_data: Option<UserDataArray>,
}

impl PaneNode {
    pub fn flatten_to_bflyt_nodes(&self, out: &mut Vec<BflytNode>) {
        if self.plain_quad.is_parts_root || self.parts_source.is_some() {
            return;
        }

        let mut baked_section = self.section.clone();
        if let Some(base) = baked_section.get_base_pane_mut() {
            base.pane_name = self.label.clone();
        }

        let mut child_nodes = Vec::new();
        for child in &self.children {
            child.flatten_to_bflyt_nodes(&mut child_nodes);
        }

        out.push(BflytNode::Pane(PaneElement {
            data: baked_section,
            user_data: self.user_data.clone(),
            children: child_nodes,
        }));
    }

    pub fn iter(&self) -> impl Iterator<Item = &PaneNode> {
        PaneIter { stack: vec![self] }
    }

    pub fn mark_transform_dirty(&mut self) {
        self.dirty
            .insert(DirtyFlags::TRANSFORM | DirtyFlags::VERTICES);

        for child in &mut self.children {
            child.mark_transform_dirty();
        }
    }

    fn walk_mut<F>(&mut self, f: &mut F)
    where
        F: FnMut(&mut PaneNode),
    {
        f(self);

        for child in &mut self.children {
            child.walk_mut(f);
        }
    }

    pub fn find_descendant_by_label_mut(&mut self, label: &str) -> Option<&mut PaneNode> {
        for child in &mut self.children {
            if child.label.trim_end_matches('\0') == label {
                return Some(child);
            }

            if let Some(found) = child.find_descendant_by_label_mut(label) {
                return Some(found);
            }
        }

        None
    }

    pub fn recompute(
        &mut self,
        parent_pos: Vector2f,
        parent_size: Vector2f,
        parent_scale: Vector2f,
    ) {
        let child_scale;

        if self.dirty.contains(DirtyFlags::TRANSFORM) {
            if let Some(base) = self.section.get_base_pane() {
                let (pos, size, anchor, center) =
                    resolve_rect(base, parent_pos, parent_size, parent_scale);

                self.world_pos = pos;
                self.world_size = size;
                self.world_center = center;
                self.parent_anchor = anchor;

                let corners = Corners::compute(
                    center,
                    size,
                    &base.origin.origin_x,
                    &base.origin.origin_y,
                    base.rotation,
                );

                self.world_corners = corners;

                self.plain_quad.corners = corners.to_array();
                self.plain_quad.width = size.x;
                self.plain_quad.height = size.y;

                if let Some(tq) = &mut self.textured_quad {
                    tq.x = pos.x;
                    tq.y = pos.y;
                    tq.width = size.x;
                    tq.height = size.y;
                    tq.corners = corners.to_array();
                }

                self.dirty.remove(DirtyFlags::TRANSFORM);
                child_scale = Vector2f {
                    x: base.scale.x * parent_scale.x,
                    y: base.scale.y * parent_scale.y,
                }
            } else {
                child_scale = parent_scale;
            }
        } else {
            child_scale = self
                .section
                .get_base_pane()
                .map(|b| Vector2f {
                    x: b.scale.x * parent_scale.x,
                    y: b.scale.y * parent_scale.y,
                })
                .unwrap_or(parent_scale);
        }

        for child in &mut self.children {
            child.recompute(self.world_pos, self.world_size, child_scale);
        }
    }

    pub fn recompute_dirty_material(&mut self, material_list: &MaterialList) -> bool {
        let mut requires_reupload = false;

        if self.dirty.contains(DirtyFlags::MATERIAL)
            && let Some(quad) = &mut self.textured_quad
            && let BflytSection::PicturePane(pic) = &self.section
            && let Some(mat) = material_list.materials.get(pic.material_index as usize)
            && let Some(tq) = TexturedQuad::derive_from_material(
                pic,
                mat,
                Vector2f::new(quad.x, quad.y),
                Vector2f::new(quad.width, quad.height),
                quad.corners,
                self.visible,
                self.pane_idx,
            )
        {
            *quad = tq;

            if let Some(base_quad) = &mut self.base_textured_quad {
                *base_quad = quad.clone();
            }

            requires_reupload = true;
        };

        self.dirty.remove(DirtyFlags::MATERIAL);
        requires_reupload
    }

    pub fn compute_uvs(pic: &PicturePane, material: &Material) -> (UvMatrix4x3x2, UvMatrix4x3x2) {
        let get_uv_set = |layer: usize| -> [[f32; 2]; 4] {
            if let Some(uv_set) = pic.texture_uvs.get(layer) {
                [
                    [uv_set.top_left.x, uv_set.top_left.y],
                    [uv_set.top_right.x, uv_set.top_right.y],
                    [uv_set.bottom_left.x, uv_set.bottom_left.y],
                    [uv_set.bottom_right.x, uv_set.bottom_right.y],
                ]
            } else {
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]
            }
        };

        let uvs0_base = get_uv_set(0);
        let uvs1_base = get_uv_set(1);
        let uvs2_base = get_uv_set(2);
        let base_uvs: [[[f32; 2]; 3]; 4] =
            std::array::from_fn(|i| [uvs0_base[i], uvs1_base[i], uvs2_base[i]]);

        let mut uvs = base_uvs;

        for layer in 0..3 {
            if let Some(srt) = material.tex_srts.get(layer) {
                for v_idx in 0..4 {
                    uvs[v_idx][layer] = transform_uv_srt(srt, base_uvs[v_idx][layer]);
                }
            }
        }

        (base_uvs, uvs)
    }
}

pub struct PaneIter<'a> {
    stack: Vec<&'a PaneNode>,
}
impl<'a> Iterator for PaneIter<'a> {
    type Item = &'a PaneNode;
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        for child in node.children.iter().rev() {
            self.stack.push(child);
        }

        Some(node)
    }
}

pub struct PaneTree {
    pub roots: Vec<PaneNode>,

    pub layout_size: Vector2f,
    pub material_list: Option<MaterialList>,
    pub user_data: Option<UserDataArray>,
    pub texture_list: Option<TextureList>,
    pub font_list: Option<FontList>,
    pub capture_texture_list: Option<CaptureTextureList>,
    pub control_source: Option<ControlSourceElement>,
    pub group: GroupElement,

    pub file_name: String,
    pub discovered_bntx_buffers: Vec<Vec<u8>>,

    pub parent_map: HashMap<usize, Option<usize>>,
    pub label_map: HashMap<String, usize>,
    pub max_pane_idx: usize,
}

impl PaneTree {
    pub fn iter(&self) -> impl Iterator<Item = &PaneNode> {
        self.roots.iter().flat_map(|r| r.iter())
    }

    pub fn flatten(&self) -> Vec<&PaneNode> {
        self.iter().collect()
    }

    pub fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut PaneNode),
    {
        for root in &mut self.roots {
            root.walk_mut(&mut f);
        }
    }

    pub fn for_each_descendant_mut<F>(&mut self, target_idx: usize, mut f: F)
    where
        F: FnMut(&mut PaneNode),
    {
        if let Some(target_node) = self.find_by_idx_mut(target_idx) {
            target_node.walk_mut(&mut f);
        }
    }

    pub fn recompute_dirty(&mut self) {
        for root in &mut self.roots {
            root.recompute(
                Vector2f::default(),
                self.layout_size,
                Vector2f { x: 1.0, y: 1.0 },
            );
        }
    }

    pub fn recompute_dirty_materials(&mut self) -> bool {
        let material_list = std::mem::take(&mut self.material_list);
        let mut require_reupload = false;

        if let Some(ref list) = material_list {
            self.for_each_mut(|node| {
                require_reupload |= node.recompute_dirty_material(list);
            });
        }

        self.material_list = material_list;

        require_reupload
    }

    pub fn collect_render_quads(&self) -> Vec<PaneQuadData> {
        fn collect_recursive(node: &PaneNode, out: &mut Vec<PaneQuadData>) {
            if let Some(tq) = &node.textured_quad {
                out.push(PaneQuadData::Textured(Box::new(tq.clone())));
            } else {
                if !node.plain_quad.is_parts_root {
                    out.push(PaneQuadData::Plain(node.plain_quad.clone()));
                }
            }

            for child in &node.children {
                collect_recursive(child, out);
            }
        }

        let mut out = Vec::new();
        for root in &self.roots {
            collect_recursive(root, &mut out);
        }

        out
    }

    pub fn build_idx_map(&mut self) -> HashMap<usize, *mut PaneNode> {
        let mut map = HashMap::new();

        self.for_each_mut(|node| {
            map.insert(node.pane_idx, node as *mut PaneNode);
        });

        map
    }

    pub fn find_by_label(&self, label: &str) -> Option<&PaneNode> {
        let idx = *self.label_map.get(label)?;
        self.find_by_idx(idx)
    }

    pub fn find_by_idx(&self, target_idx: usize) -> Option<&PaneNode> {
        self.iter().find(|n| n.pane_idx == target_idx)
    }

    pub fn find_by_idx_mut(&mut self, target_idx: usize) -> Option<&mut PaneNode> {
        fn find_recursive(nodes: &mut [PaneNode], target_idx: usize) -> Option<&mut PaneNode> {
            for node in nodes {
                if node.pane_idx == target_idx {
                    return Some(node);
                }
                if let Some(found) = find_recursive(&mut node.children, target_idx) {
                    return Some(found);
                }
            }
            None
        }

        find_recursive(&mut self.roots, target_idx)
    }

    pub fn label_to_idx(&self) -> HashMap<String, usize> {
        self.label_map.clone()
    }

    pub fn descendants(&self, pane_idx: usize) -> Vec<usize> {
        fn collect_all(node: &PaneNode, out: &mut Vec<usize>) {
            for child in &node.children {
                out.push(child.pane_idx);
                collect_all(child, out);
            }
        }

        let mut out = Vec::new();
        if let Some(target_node) = self.find_by_idx(pane_idx) {
            collect_all(target_node, &mut out);
        }

        out
    }

    pub fn insert_node(&mut self, parent_idx: Option<usize>, node: PaneNode) -> usize {
        self.insert_node_at(parent_idx, usize::MAX, node)
    }

    pub fn insert_node_at(
        &mut self,
        parent_idx: Option<usize>,
        position: usize,
        node: PaneNode,
    ) -> usize {
        let idx = node.pane_idx;

        fn register_subtree(
            n: &PaneNode,
            parent: Option<usize>,
            p_map: &mut HashMap<usize, Option<usize>>,
            l_map: &mut HashMap<String, usize>,
            max_idx: &mut usize,
        ) {
            let i = n.pane_idx;
            *max_idx = (*max_idx).max(i);
            p_map.insert(i, parent);
            l_map.insert(n.label.trim_end_matches('\0').to_string(), i);

            for child in &n.children {
                register_subtree(child, Some(i), p_map, l_map, max_idx);
            }
        }

        register_subtree(
            &node,
            parent_idx,
            &mut self.parent_map,
            &mut self.label_map,
            &mut self.max_pane_idx,
        );

        match parent_idx {
            Some(pid) => {
                if let Some(parent_node) = self.find_by_idx_mut(pid) {
                    let pos = position.min(parent_node.children.len());
                    parent_node.children.insert(pos, node);
                }
            }
            None => {
                let pos = position.min(self.roots.len());
                self.roots.insert(pos, node);
            }
        }

        idx
    }

    pub fn sibling_position(&self, target_idx: usize) -> Option<(Option<usize>, usize)> {
        let parent_idx = *self.parent_map.get(&target_idx)?;
        let siblings = match parent_idx {
            Some(pid) => &self.find_by_idx(pid)?.children,
            None => &self.roots,
        };

        let position = siblings.iter().position(|n| n.pane_idx == target_idx)?;
        Some((parent_idx, position))
    }

    pub fn remove_node(&mut self, target_idx: usize) -> Option<PaneNode> {
        let parent_idx = *self.parent_map.get(&target_idx)?;

        let removed_node = match parent_idx {
            Some(pid) => {
                let parent_node = self.find_by_idx_mut(pid)?;
                let pos = parent_node
                    .children
                    .iter()
                    .position(|n| n.pane_idx == target_idx)?;

                Some(parent_node.children.remove(pos))
            }
            None => {
                let pos = self.roots.iter().position(|n| n.pane_idx == target_idx)?;

                Some(self.roots.remove(pos))
            }
        };

        if let Some(ref node) = removed_node {
            fn unregister_subtree(
                n: &PaneNode,
                p_map: &mut HashMap<usize, Option<usize>>,
                l_map: &mut HashMap<String, usize>,
            ) {
                p_map.remove(&n.pane_idx);
                l_map.remove(n.label.trim_end_matches('\0'));

                for child in &n.children {
                    unregister_subtree(child, p_map, l_map);
                }
            }

            unregister_subtree(node, &mut self.parent_map, &mut self.label_map);
        }

        removed_node
    }

    pub fn next_pane_idx(&self) -> usize {
        self.max_pane_idx + 1
    }

    pub fn duplicate_node(&mut self, target_idx: usize) -> Option<usize> {
        let parent_idx = self.parent_map.get(&target_idx).copied().flatten();
        let mut clone = self.find_by_idx(target_idx)?.clone();

        fn reindex(node: &mut PaneNode, next_idx: &mut usize) {
            node.pane_idx = *next_idx;
            *next_idx += 1;

            node.plain_quad.pane_idx = node.pane_idx;
            if let Some(tq) = &mut node.textured_quad {
                tq.pane_idx = node.pane_idx;
            }
            if let Some(tq) = &mut node.base_textured_quad {
                tq.pane_idx = node.pane_idx;
            }
            node.dirty = DirtyFlags::all();

            for child in &mut node.children {
                reindex(child, next_idx);
            }
        }

        let mut next_idx = self.next_pane_idx();
        reindex(&mut clone, &mut next_idx);
        clone.label = format!("{} copy", clone.label.trim_end_matches('\0'));

        let position = self
            .sibling_position(target_idx)
            .map(|(_, pos)| pos + 1)
            .unwrap_or(usize::MAX);

        Some(self.insert_node_at(parent_idx, position, clone))
    }

    pub fn from_bflyt(
        file: Bflyt,
        blarc_dir: Option<&Path>,
        file_name: String,
        has_bntx: bool,
        archive_entries: Option<&[ArchiveEntry]>,
    ) -> Self {
        let layout_size = Vector2f {
            x: file.layout.width,
            y: file.layout.height,
        };

        let material_list = file.material_list.clone();
        let mut blarc_cache: HashMap<String, Option<Bflyt>> = HashMap::new();
        let mut discovered_bntx_buffers: Vec<Vec<u8>> = Vec::new();

        let mut builder = Builder {
            material_list: material_list.as_ref(),
            sub_material_list: None,
            archive_entries,
            blarc_dir,
            blarc_cache: &mut blarc_cache,
            discovered: &mut discovered_bntx_buffers,
            has_bntx,
            parts_depth: 0,
            parts_source: None,
            next_pane_idx: 0,
        };

        let roots = builder.build_nodes(
            &file.nodes,
            Vector2f::default(),
            layout_size,
            Vector2f { x: 1.0, y: 1.0 },
            true,
            0,
        );

        let mut parent_map = HashMap::new();
        let mut label_map = HashMap::new();
        let mut max_pane_idx = 0;

        fn index_tree(
            node: &PaneNode,
            parent: Option<usize>,
            p_map: &mut HashMap<usize, Option<usize>>,
            l_map: &mut HashMap<String, usize>,
            max_idx: &mut usize,
        ) {
            let idx = node.pane_idx;
            *max_idx = (*max_idx).max(idx);
            p_map.insert(idx, parent);

            let clean_label = node.label.trim_end_matches('\0').to_string();
            l_map.insert(clean_label, idx);

            for child in &node.children {
                index_tree(child, Some(idx), p_map, l_map, max_idx);
            }
        }

        for root in &roots {
            index_tree(
                root,
                None,
                &mut parent_map,
                &mut label_map,
                &mut max_pane_idx,
            );
        }

        PaneTree {
            roots,
            layout_size,
            material_list,
            file_name,
            discovered_bntx_buffers,
            parent_map,
            label_map,
            max_pane_idx,
            user_data: file.user_data,
            texture_list: file.texture_list,
            font_list: file.font_list,
            capture_texture_list: file.capture_texture_list,
            group: file.root_group,
            control_source: file.control_source,
        }
    }
}

struct Builder<'a> {
    material_list: Option<&'a MaterialList>,
    sub_material_list: Option<MaterialList>,
    blarc_dir: Option<&'a Path>,
    archive_entries: Option<&'a [ArchiveEntry]>,
    blarc_cache: &'a mut HashMap<String, Option<Bflyt>>,
    discovered: &'a mut Vec<Vec<u8>>,
    has_bntx: bool,
    parts_depth: usize,
    parts_source: Option<String>,
    next_pane_idx: usize,
}

const MAX_PARTS_DEPTH: usize = 8;

impl<'a> Builder<'a> {
    pub fn build_nodes(
        &mut self,
        nodes: &[BflytNode],
        parent_pos: Vector2f,
        parent_size: Vector2f,
        parent_scale: Vector2f,
        parent_visible: bool,
        depth: usize,
    ) -> Vec<PaneNode> {
        let mut out = Vec::new();

        for node in nodes {
            match node {
                BflytNode::Pane(pane_el) => {
                    if let Some(mut pane_node) = self.build_node(
                        &pane_el.data,
                        parent_pos,
                        parent_size,
                        parent_scale,
                        parent_visible,
                        depth,
                    ) {
                        pane_node.user_data = pane_el.user_data.clone();

                        let mut current_visible = parent_visible;
                        let mut current_scale = parent_scale;

                        if let Some(base) = pane_el.data.get_base_pane() {
                            current_visible = parent_visible && base.pane_flags.is_visible;
                            current_scale = Vector2f {
                                x: base.scale.x * parent_scale.x,
                                y: base.scale.y * parent_scale.y,
                            };
                        }

                        if !pane_el.children.is_empty() {
                            pane_node.children = self.build_nodes(
                                &pane_el.children,
                                pane_node.world_pos,
                                pane_node.world_size,
                                current_scale,
                                current_visible,
                                depth + 1,
                            );
                        }

                        out.push(pane_node);
                    }
                }

                BflytNode::Group(_) => {}
                BflytNode::ControlSource(_) => {}
                BflytNode::RootSection(_) => {}
            }
        }

        out
    }

    fn build_node(
        &mut self,
        section: &BflytSection,
        parent_pos: Vector2f,
        parent_size: Vector2f,
        parent_scale: Vector2f,
        parent_visible: bool,
        depth: usize,
    ) -> Option<PaneNode> {
        let base = section.get_base_pane()?;

        let is_visible = parent_visible && base.pane_flags.is_visible;
        let (pos, size, anchor, center) = resolve_rect(base, parent_pos, parent_size, parent_scale);

        let corners = Corners::compute(
            center,
            size,
            &base.origin.origin_x,
            &base.origin.origin_y,
            base.rotation,
        );

        let label = section.pane_name();
        let kind = section.kind_name().to_string();

        let pane_idx = self.next_pane_idx;
        self.next_pane_idx += 1;

        let textured_quad = if let BflytSection::PicturePane(pic) = section {
            self.build_textured_quad(pic, pos, size, center, base.rotation, is_visible, pane_idx)
        } else {
            None
        };

        let color = if is_visible {
            section.section_color()
        } else {
            [0.0; 4]
        };

        let is_parts_root = base.pane_name == "RootPane" && self.parts_source.is_some();

        let plain_quad = Quad {
            corners: corners.to_array(),
            width: size.x,
            height: size.y,
            color,
            has_textured: matches!(section, BflytSection::PicturePane(_)),
            is_parts_root,
            pane_idx,
        };

        let mut node = PaneNode {
            section: section.clone(),
            kind,
            label,
            depth,
            visible: is_visible,
            parts_source: self.parts_source.clone(),
            pane_idx,
            world_pos: pos,
            world_size: size,
            world_center: center,
            parent_anchor: anchor,
            world_corners: corners,
            textured_quad: textured_quad.clone(),
            base_textured_quad: textured_quad,
            plain_quad,
            dirty: DirtyFlags::empty(),
            children: Vec::new(),
            user_data: None,
        };

        if let BflytSection::PartsPane(parts) = section {
            self.resolve_parts(parts, &mut node, is_visible);
        }

        Some(node)
    }

    fn load_bflyt_from_archive_index(&self, layout_name: &str) -> Option<Vec<MagicFiles>> {
        let entries = self.archive_entries?;
        let target = layout_name.to_lowercase();

        let entry = entries.iter().find(|e| e.layout_key() == target)?;
        let bytes = std::fs::read(&entry.path).ok()?;
        let package_bytes = resolve_nested_package_bytes(bytes, &entry.nested_path)?;

        let mut all_files = Vec::new();
        extract_all_files_recursive(package_bytes, &mut all_files);

        let has_bflyt = all_files.iter().any(|f| matches!(f, MagicFiles::Bflyt(_)));
        if !has_bflyt {
            return None;
        }

        Some(all_files)
    }

    fn resolve_parts(
        &mut self,
        parts: &PartsPane,
        parent_node: &mut PaneNode,
        parent_visible: bool,
    ) {
        if self.parts_depth >= MAX_PARTS_DEPTH {
            return;
        }

        let Some(blarc_dir) = self.blarc_dir else {
            return;
        };

        let layout_name = parts.o_layout_name.trim_end_matches('\0');
        if layout_name.is_empty() {
            return;
        }

        if !self.blarc_cache.contains_key(layout_name) {
            let assets = load_bflyt_from_blarc_dir(blarc_dir, layout_name)
                .or_else(|| self.load_bflyt_from_archive_index(layout_name));

            if let Some(assets) = assets {
                let bflyt_res = assets.iter().find_map(|f| {
                    if let MagicFiles::Bflyt(bytes) = f {
                        Bflyt::parse_file(bytes).ok()
                    } else {
                        None
                    }
                });

                if let Some(sub_bflyt) = bflyt_res {
                    for asset in assets {
                        if let MagicFiles::Bntx(bntx_data) = asset {
                            self.discovered.push(bntx_data);
                        }
                    }

                    self.blarc_cache
                        .insert(layout_name.to_string(), Some(sub_bflyt));
                }
            }
        }

        let Some(Some(sub_bflyt)) = self.blarc_cache.get(layout_name) else {
            log::warn!("PartsPane: could not load '{layout_name}'");
            return;
        };

        let scale = Vector2f {
            x: parts.base.scale.x * parts.magnify_x,
            y: parts.base.scale.y * parts.magnify_y,
        };

        let sub_size = Vector2f {
            x: sub_bflyt.layout.width,
            y: sub_bflyt.layout.height,
        };

        let sub_parent_pos = Vector2f {
            x: -sub_size.x * 0.5,
            y: -sub_size.y * 0.5,
        };

        let old_source = self.parts_source.clone();
        self.parts_source = Some(layout_name.to_string());
        self.parts_depth += 1;

        let sub_nodes = sub_bflyt.nodes.clone();
        self.sub_material_list = sub_bflyt.material_list.clone();

        let mut sub_children = self.build_nodes(
            &sub_nodes,
            sub_parent_pos + parent_node.world_center,
            sub_size * scale,
            scale,
            parent_visible,
            parent_node.depth + 1,
        );

        self.sub_material_list = None;

        for prop in &parts.properties {
            let prop_name = prop.property_name.trim_end_matches('\0');
            if prop_name.is_empty() {
                continue;
            }

            if let Some(info) = &prop.o_basic_info {
                fn find_mut<'a>(nodes: &'a mut [PaneNode], name: &str) -> Option<&'a mut PaneNode> {
                    for node in nodes.iter_mut() {
                        if node.label.trim_end_matches('\0') == name {
                            return Some(node);
                        }
                        if let Some(found) = find_mut(&mut node.children, name) {
                            return Some(found);
                        }
                    }
                    None
                }

                if let Some(target) = find_mut(&mut sub_children, prop_name) {
                    apply_basic_info_override(target, &prop.basic_usage_flag, info);
                } else {
                    log::warn!(
                        "PartsPane '{layout_name}': override target '{prop_name}' not found"
                    );
                }
            }

            let Some(override_section) = &prop.o_section else {
                continue;
            };

            fn apply_override(
                nodes: &mut [PaneNode],
                prop_name: &str,
                override_section: &BflytSection,
                builder: &Builder,
            ) {
                for node in nodes.iter_mut() {
                    if node.label.trim_end_matches('\0') == prop_name {
                        if let BflytSection::PicturePane(pic) = override_section {
                            let tq = builder.build_textured_quad(
                                pic,
                                node.world_pos,
                                node.world_size,
                                node.world_center,
                                node.section
                                    .get_base_pane()
                                    .map(|b| b.rotation)
                                    .unwrap_or(Vector3f::default()),
                                node.visible,
                                node.pane_idx,
                            );

                            node.textured_quad = tq.clone();
                            node.base_textured_quad = tq;
                        }
                        return;
                    }

                    apply_override(&mut node.children, prop_name, override_section, builder);
                }
            }

            apply_override(&mut sub_children, prop_name, override_section, self);
        }

        parent_node.children.extend(sub_children);

        self.parts_depth -= 1;
        self.parts_source = old_source;
    }

    fn build_textured_quad(
        &self,
        pic: &PicturePane,
        position: Vector2f,
        size: Vector2f,
        center: Vector2f,
        rotation: Vector3f,
        is_visible: bool,
        pane_idx: usize,
    ) -> Option<TexturedQuad> {
        if !self.has_bntx {
            return None;
        }

        let material_list = self.sub_material_list.as_ref().or(self.material_list)?;
        let mat = material_list.materials.get(pic.material_index as usize)?;

        let corners = Corners::compute(
            center,
            size,
            &pic.base.origin.origin_x,
            &pic.base.origin.origin_y,
            rotation,
        );

        TexturedQuad::derive_from_material(
            pic,
            mat,
            position,
            size,
            corners.to_array(),
            is_visible,
            pane_idx,
        )
    }
}

fn resolve_rect(
    pane: &Pane,
    parent_pos: Vector2f,
    parent_size: Vector2f,
    parent_scale: Vector2f,
) -> (Vector2f, Vector2f, Vector2f, Vector2f) {
    let anchor_x = match pane.origin.parent_origin_x {
        BflytParentOrigin::None => parent_pos.x + parent_size.x * 0.5,
        BflytParentOrigin::LeftTop => parent_pos.x,
        BflytParentOrigin::RightBottom => parent_pos.x + parent_size.x,
    };

    let anchor_y = match pane.origin.parent_origin_y {
        BflytParentOrigin::None => parent_pos.y + parent_size.y * 0.5,
        BflytParentOrigin::LeftTop => parent_pos.y,
        BflytParentOrigin::RightBottom => parent_pos.y + parent_size.y,
    };

    let cx = anchor_x + pane.translation.x * parent_scale.x;
    let cy = anchor_y - pane.translation.y * parent_scale.y;

    let w = pane.size.x * pane.scale.x;
    let h = pane.size.y * pane.scale.y;

    let tl_x = match pane.origin.origin_x {
        BflytOrigin::Center => cx - w * 0.5,
        BflytOrigin::LeftTop => cx,
        BflytOrigin::RightBottom => cx - w,
    };

    let tl_y = match pane.origin.origin_y {
        BflytOrigin::Center => cy - h * 0.5,
        BflytOrigin::LeftTop => cy,
        BflytOrigin::RightBottom => cy - h,
    };

    (
        Vector2f { x: tl_x, y: tl_y },
        Vector2f {
            x: w.abs().max(1.0),
            y: h.abs().max(1.0),
        },
        Vector2f {
            x: anchor_x,
            y: anchor_y,
        },
        Vector2f { x: cx, y: cy },
    )
}

fn load_bflyt_from_blarc_dir(blarc_dir: &Path, layout_name: &str) -> Option<Vec<MagicFiles>> {
    let entry_path = std::fs::read_dir(blarc_dir).ok()?.find_map(|e| {
        let e = e.ok()?;
        let path = e.path();
        let fname = path.file_name()?.to_string_lossy().to_lowercase();

        if !fname.starts_with(&layout_name.to_lowercase()) {
            return None;
        }

        let is_valid_sarc = SUPPORTED_SARC_EXTENSIONS
            .iter()
            .any(|ext| fname.ends_with(&format!(".{}", ext.to_lowercase())));

        if is_valid_sarc { Some(path) } else { None }
    })?;

    let mut bytes = std::fs::read(&entry_path).ok()?;
    let filename = entry_path.file_name()?.to_string_lossy();

    bytes = decompress_if_needed(bytes, &filename);

    let mut all_files = Vec::new();
    extract_all_files_recursive(bytes, &mut all_files);

    let has_bflyt = all_files.iter().any(|f| matches!(f, MagicFiles::Bflyt(_)));
    if !has_bflyt {
        return None;
    }

    Some(all_files)
}

pub fn apply_basic_info_override(
    node: &mut PaneNode,
    flags: &BasePaneUsageFlags,
    info: &PartsPaneBasicInfo,
) {
    if flags.is_visible_set {
        node.visible = flags.is_visible_true;
    }

    if let Some(base) = node.section.get_base_pane_mut() {
        if flags.is_visible_set {
            base.pane_flags.is_visible = flags.is_visible_true;
        }

        if flags.has_translate {
            base.translation.x = info.translation_x;
            base.translation.y = info.translation_y;
            base.translation.z = info.translation_z;
        }

        if flags.has_size {
            base.size.x = info.size_x;
            base.size.y = info.size_y;
        }

        if flags.has_scale {
            base.scale.x = info.scale_x;
            base.scale.y = info.scale_y;
        }

        if flags.has_rotate {
            base.rotation.x = info.rotation_x;
            base.rotation.y = info.rotation_y;
            base.rotation.z = info.rotation_z;
        }

        if flags.has_alpha {
            base.alpha = info.pane_alpha;
        }
    }

    node.mark_transform_dirty();
}
