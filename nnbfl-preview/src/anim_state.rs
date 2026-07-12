use std::collections::HashSet;

use nnbfl::bflyt::list::MaterialTextureSrt;
use nnbfl::ui2d::userdata::UserDataContent;
use nnbfl::{
    bflan::{
        anim_info::{AnimInfo, AnimInfoType, AnimType, PaneAnimInfo},
        curves::Curve,
        file::Bflan,
        targets::{
            IndirectSrtTarget, PaneSrtTarget, TargetIndex, TextureSrtTarget, VertexColorTarget,
        },
    },
    ui2d::types::Vector2f,
};

use crate::bflyt_view::BflytView;
use crate::pane_tree::DirtyFlags;
use crate::traits::Displaying;
use crate::ui::timeline::{PendingKeyEdit, PendingSlopeEdit, TimelineTrack};

fn eval_hermite(keys: &[nnbfl::bflan::curves::HermiteKey], frame: f32) -> f32 {
    if keys.is_empty() {
        return 0.0;
    }

    if frame <= keys[0].frame {
        return keys[0].value;
    }

    if frame >= keys[keys.len() - 1].frame {
        return keys[keys.len() - 1].value;
    }

    let idx = keys.partition_point(|k| k.frame <= frame) - 1;
    let k0 = &keys[idx];
    let k1 = &keys[idx + 1];
    let dt = k1.frame - k0.frame;
    let t = (frame - k0.frame) / dt;
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * k0.value + h10 * dt * k0.slope + h01 * k1.value + h11 * dt * k1.slope
}

pub fn eval_curve(curve: &Curve, frame: f32) -> f32 {
    match curve {
        Curve::Constant(keys) => {
            let idx = (frame as usize).min(keys.len().saturating_sub(1));
            keys.get(idx).copied().unwrap_or(0.0)
        }
        Curve::Step(keys) => {
            if keys.is_empty() {
                return 0.0;
            }
            let idx = keys.partition_point(|k| k.frame <= frame).saturating_sub(1);
            keys[idx].value as f32
        }
        Curve::Hermite(keys) => eval_hermite(keys, frame),
    }
}

pub fn eval_curve_step_u16(curve: &Curve, frame: f32) -> u16 {
    match curve {
        Curve::Step(keys) => {
            if keys.is_empty() {
                return 0;
            }
            let idx = keys.partition_point(|k| k.frame <= frame).saturating_sub(1);
            keys[idx].value
        }
        _ => eval_curve(curve, frame) as u16,
    }
}

pub struct AnimInstance {
    pub bflan: Bflan,
    pub name: String,
    pub frame: f32,
    pub playing: bool,
    pub autoplay: bool,
    pub next_anim: Option<String>,
}

impl AnimInstance {
    pub fn new(bflan: Bflan) -> Self {
        let next_anim = find_next_anim(&bflan);
        let name = bflan.anim_tag.o_name.clone();

        Self {
            bflan,
            name,
            frame: 0.0,
            playing: false,
            autoplay: false,
            next_anim,
        }
    }

    pub fn frame_count(&self) -> f32 {
        self.bflan.anim_info.frame_count as f32
    }

    pub fn is_looping(&self) -> bool {
        self.bflan.anim_info.is_looping
    }

    pub fn toggle_looping(&mut self) {
        self.bflan.anim_info.is_looping = !self.bflan.anim_info.is_looping
    }

    pub fn curve(&self, track: &TimelineTrack) -> Option<&Curve> {
        let content = self.bflan.anim_info.contents.get(track.content_idx)?;
        let info = content.infos.get(track.info_idx)?;
        let AnimInfo::Standard { targets, .. } = info else {
            return None;
        };

        Some(&targets.get(track.target_idx)?.curve)
    }

    pub fn curve_mut(&mut self, track: &TimelineTrack) -> Option<&mut Curve> {
        let content = self.bflan.anim_info.contents.get_mut(track.content_idx)?;
        let info = content.infos.get_mut(track.info_idx)?;
        let AnimInfo::Standard { targets, .. } = info else {
            return None;
        };

        Some(&mut targets.get_mut(track.target_idx)?.curve)
    }

    pub fn get_active_group_pane_indices(&self, view: &BflytView) -> HashSet<usize> {
        let mut active_indices = HashSet::new();

        if self.bflan.anim_tag.groups.is_empty() {
            return active_indices;
        }

        let label_to_idx = view.tree.label_to_idx();

        for group_name in &self.bflan.anim_tag.groups {
            if let Some(layout_group) = view
                .tree
                .group
                .children
                .iter()
                .find(|g| g.group_name == *group_name)
            {
                for child_name in &layout_group.child_names {
                    let clean_name = child_name.trim_end_matches('\0');

                    if let Some(&pane_idx) = label_to_idx.get(clean_name) {
                        active_indices.insert(pane_idx);
                    }
                }
            }
        }

        active_indices
    }

    pub fn resolve_group_visibility(&self, view: &BflytView, hidden_panes: &mut HashSet<usize>) {
        hidden_panes.clear();

        if self.bflan.anim_tag.groups.is_empty() {
            return;
        }

        let explicit_active_indices = self.get_active_group_pane_indices(view);
        let label_to_idx = view.tree.label_to_idx();

        let mut fully_allowed_indices = explicit_active_indices.clone();
        for &pane_idx in &explicit_active_indices {
            fully_allowed_indices.extend(view.tree.descendants(pane_idx));
        }

        let active_groups: HashSet<&str> = self
            .bflan
            .anim_tag
            .groups
            .iter()
            .map(|s| s.as_str())
            .collect();

        for group in &view.tree.group.children {
            if active_groups.contains(group.group_name.as_str()) {
                continue;
            }

            for child_name in &group.child_names {
                let clean_name = child_name.trim_end_matches('\0');
                if let Some(&pane_idx) = label_to_idx.get(clean_name)
                    && !fully_allowed_indices.contains(&pane_idx)
                {
                    hidden_panes.insert(pane_idx);
                }
            }
        }
    }
}

fn find_next_anim(bflan: &Bflan) -> Option<String> {
    if let Some(ud) = &bflan.anim_tag.user_data {
        for entry in &ud.user_data {
            if entry.o_name == "CommandPlayEnd_Play"
                && let UserDataContent::String(value) = &entry.content
            {
                return Some(value.clone());
            }
        }
    }

    None
}

pub struct AnimPlayer {
    pub anims: Vec<AnimInstance>,
    pub active: Option<usize>,
    pub limit_to_group: bool,
}

impl AnimPlayer {
    pub fn new() -> Self {
        Self {
            anims: Vec::new(),
            active: None,
            limit_to_group: false,
        }
    }

    pub fn load(&mut self, bflan: Bflan) {
        self.anims.push(AnimInstance::new(bflan));
    }

    pub fn play(
        &mut self,
        anim_idx: Option<usize>,
        view: Option<&BflytView>,
        hidden_panes: &mut HashSet<usize>,
    ) {
        if let Some(idx) = anim_idx {
            if let Some(prev) = self.active
                && prev < self.anims.len()
            {
                self.anims[prev].playing = false;
            }

            self.anims[idx].frame = 0.0;
            self.anims[idx].playing = true;
            self.anims[idx].autoplay = true;
            self.active = Some(idx);

            if self.limit_to_group
                && let Some(view) = view
            {
                self.anims[idx].resolve_group_visibility(view, hidden_panes);
            } else {
                hidden_panes.clear();
            }
        }
    }

    pub fn tick(&mut self, dt: f32, fps: f32) -> Option<String> {
        let idx = self.active?;
        let anim = &mut self.anims[idx];
        if !anim.playing {
            return None;
        }

        anim.frame += dt * fps;
        let frame_count = anim.frame_count();
        if anim.frame >= frame_count {
            if anim.is_looping() {
                anim.frame %= frame_count;
            } else {
                anim.frame = frame_count;
                anim.playing = false;
                return anim.next_anim.clone();
            }
        }
        None
    }

    pub fn apply(&self, view: &mut BflytView) {
        let Some(idx) = self.active else { return };
        let anim = &self.anims[idx];

        apply_anim(&anim.bflan.anim_info, anim.frame, view);
    }

    pub fn is_playing(&self) -> bool {
        self.active.map(|i| self.anims[i].playing).unwrap_or(false)
    }

    pub fn apply_key_edit(&mut self, edit: &PendingKeyEdit) {
        let Some(idx) = self.active else { return };
        let Some(anim) = self.anims.get_mut(idx) else {
            return;
        };

        let Some(curve) = anim.curve_mut(&edit.track) else {
            return;
        };

        match curve {
            Curve::Constant(keys) => {
                if let Some(v) = keys.get_mut(edit.key_idx) {
                    *v = edit.value;
                }
            }

            Curve::Step(keys) => {
                let min_frame = edit
                    .key_idx
                    .checked_sub(1)
                    .and_then(|i| keys.get(i))
                    .map(|k| k.frame + 0.01)
                    .unwrap_or(0.0);

                let max_frame = keys
                    .get(edit.key_idx + 1)
                    .map(|k| k.frame - 0.01)
                    .unwrap_or(f32::MAX)
                    .max(min_frame);

                if let Some(k) = keys.get_mut(edit.key_idx) {
                    k.frame = edit.frame.clamp(min_frame, max_frame);
                    k.value = edit.value.round().clamp(0.0, u16::MAX as f32) as u16;
                }
            }

            Curve::Hermite(keys) => {
                let min_frame = edit
                    .key_idx
                    .checked_sub(1)
                    .and_then(|i| keys.get(i))
                    .map(|k| k.frame + 0.01)
                    .unwrap_or(0.0);

                let max_frame = keys
                    .get(edit.key_idx + 1)
                    .map(|k| k.frame - 0.01)
                    .unwrap_or(f32::MAX)
                    .max(min_frame);

                if let Some(k) = keys.get_mut(edit.key_idx) {
                    k.frame = edit.frame.clamp(min_frame, max_frame);
                    k.value = edit.value;
                }
            }
        }
    }

    pub fn apply_slope_edit(&mut self, edit: &PendingSlopeEdit) {
        let Some(idx) = self.active else { return };
        let Some(anim) = self.anims.get_mut(idx) else {
            return;
        };

        let Some(curve) = anim.curve_mut(&edit.track) else {
            return;
        };

        if let Curve::Hermite(keys) = curve
            && let Some(k) = keys.get_mut(edit.key_idx)
        {
            k.slope = edit.slope;
        }
    }
}

#[inline]
pub fn transform_uv_srt(srt: &MaterialTextureSrt, uv: [f32; 2]) -> [f32; 2] {
    let rad = srt.rotate.to_radians();
    let cos_r = rad.cos();
    let sin_r = rad.sin();

    let centered_u = uv[0] - 0.5;
    let centered_v = uv[1] - 0.5;

    let scaled_u = centered_u * srt.scale_u;
    let scaled_v = centered_v * srt.scale_v;

    let rotated_u = scaled_u * cos_r - scaled_v * sin_r;
    let rotated_v = scaled_u * sin_r + scaled_v * cos_r;

    [
        rotated_u + srt.translate_u + 0.5,
        rotated_v + srt.translate_v + 0.5,
    ]
}

fn apply_tex_srts(tq: &mut crate::renderer::textured_quad::TexturedQuad) {
    for (i, srt) in tq.tex_srts.iter().enumerate() {
        for v_idx in 0..4 {
            let base_uv = tq.base_uvs[v_idx][i];
            tq.uvs[v_idx][i] = transform_uv_srt(srt, base_uv);
        }
    }
}

fn cascade_visibility(view: &mut BflytView, pane_idx: usize, visible: bool) {
    view.tree.for_each_descendant_mut(pane_idx, |node| {
        node.visible = visible;

        node.plain_quad.color = if visible {
            node.section.section_color()
        } else {
            [0.0; 4]
        };

        if let Some(tq) = &mut node.textured_quad {
            tq.standard_material.visible = visible as u32;
        }

        node.dirty.insert(DirtyFlags::VERTICES);
    });
}

fn apply_anim(pai: &PaneAnimInfo, frame: f32, view: &mut BflytView) {
    let pane_by_name = view.tree.label_to_idx();

    for content in &pai.contents {
        let name = content.name.trim_end_matches('\0');
        let Some(&pane_idx) = pane_by_name.get(name) else {
            continue;
        };

        if !matches!(
            content.anim_type,
            AnimType::Pane | AnimType::PaneExt | AnimType::Material
        ) {
            continue;
        }

        for info in &content.infos {
            let AnimInfo::Standard { anim_type, targets } = info else {
                continue;
            };

            match anim_type {
                AnimInfoType::PaneSrtAnim => {
                    let (base_translation, base_size, base_rotation, base_scale) = {
                        let Some(node) = view.tree.find_by_idx(pane_idx) else {
                            continue;
                        };
                        let base = node.section.get_base_pane();
                        (
                            base.map(|b| b.translation).unwrap_or_default(),
                            base.map(|b| b.size).unwrap_or(node.world_size),
                            base.map(|b| b.rotation).unwrap_or_default(),
                            base.map(|b| b.scale).unwrap_or(Vector2f::new(1.0, 1.0)),
                        )
                    };

                    let mut new_translation = base_translation;
                    let mut new_scale = base_scale;
                    let mut new_size = base_size;
                    let mut new_rotation = base_rotation;

                    for t in targets {
                        let v = eval_curve(&t.curve, frame);
                        match &t.target {
                            TargetIndex::PaneSrt(PaneSrtTarget::TranslateX) => {
                                new_translation.x = v
                            }
                            TargetIndex::PaneSrt(PaneSrtTarget::TranslateY) => {
                                new_translation.y = v
                            }
                            TargetIndex::PaneSrt(PaneSrtTarget::TranslateZ) => {
                                new_translation.z = v
                            }
                            TargetIndex::PaneSrt(PaneSrtTarget::ScaleX) => new_scale.x = v,
                            TargetIndex::PaneSrt(PaneSrtTarget::ScaleY) => new_scale.y = v,
                            TargetIndex::PaneSrt(PaneSrtTarget::SizeX) => new_size.x = v,
                            TargetIndex::PaneSrt(PaneSrtTarget::SizeY) => new_size.y = v,
                            TargetIndex::PaneSrt(PaneSrtTarget::RotateX) => new_rotation.x = v,
                            TargetIndex::PaneSrt(PaneSrtTarget::RotateY) => new_rotation.y = v,
                            TargetIndex::PaneSrt(PaneSrtTarget::RotateZ) => new_rotation.z = v,
                            _ => {}
                        }
                    }

                    if let Some(node) = view.tree.find_by_idx_mut(pane_idx)
                        && let Some(base) = node.section.get_base_pane_mut()
                    {
                        base.translation = new_translation;
                        base.scale = new_scale;
                        base.size = new_size;
                        base.rotation = new_rotation;
                        node.mark_transform_dirty();
                    }
                }

                AnimInfoType::VisibilityAnim => {
                    for t in targets {
                        let visible = eval_curve_step_u16(&t.curve, frame) != 0;
                        cascade_visibility(view, pane_idx, visible);
                    }
                }

                AnimInfoType::TextureSrtAnim => {
                    let Some(node) = view.tree.find_by_idx_mut(pane_idx) else {
                        continue;
                    };
                    let Some(tq) = &mut node.textured_quad else {
                        continue;
                    };

                    for t in targets {
                        let v = eval_curve(&t.curve, frame);
                        let layer = t.layer as usize;
                        if layer >= tq.tex_srts.len() {
                            continue;
                        }
                        match &t.target {
                            TargetIndex::TextureSrt(TextureSrtTarget::TranslateU) => {
                                tq.tex_srts[layer].translate_u = v
                            }
                            TargetIndex::TextureSrt(TextureSrtTarget::TranslateV) => {
                                tq.tex_srts[layer].translate_v = v
                            }
                            TargetIndex::TextureSrt(TextureSrtTarget::Rotate) => {
                                tq.tex_srts[layer].rotate = v
                            }
                            TargetIndex::TextureSrt(TextureSrtTarget::ScaleU) => {
                                tq.tex_srts[layer].scale_u = v
                            }
                            TargetIndex::TextureSrt(TextureSrtTarget::ScaleV) => {
                                tq.tex_srts[layer].scale_v = v
                            }
                            _ => {}
                        }
                    }
                    apply_tex_srts(tq);
                }

                AnimInfoType::IndirectSrtAnim => {
                    let Some(node) = view.tree.find_by_idx_mut(pane_idx) else {
                        continue;
                    };
                    let Some(tq) = &mut node.textured_quad else {
                        continue;
                    };

                    for t in targets {
                        let v = eval_curve(&t.curve, frame);
                        match &t.target {
                            TargetIndex::IndirectSrt(IndirectSrtTarget::Rotate) => {
                                tq.indirect_rotation = v
                            }
                            TargetIndex::IndirectSrt(IndirectSrtTarget::ScaleU) => {
                                tq.indirect_scale.x = v
                            }
                            TargetIndex::IndirectSrt(IndirectSrtTarget::ScaleV) => {
                                tq.indirect_scale.y = v
                            }
                            _ => {}
                        }
                    }

                    let (m0, m1) = crate::renderer::textured_quad::build_indirect_matrices(
                        tq.indirect_rotation,
                        tq.indirect_scale,
                    );
                    tq.standard_material.indirect_mtx0 = m0;
                    tq.standard_material.indirect_mtx1 = m1;
                }

                AnimInfoType::TexturePatternAnim => {
                    let Some(node) = view.tree.find_by_idx_mut(pane_idx) else {
                        continue;
                    };
                    let Some(tq) = &mut node.textured_quad else {
                        continue;
                    };

                    for t in targets {
                        let file_idx = eval_curve_step_u16(&t.curve, frame) as usize;
                        let Some(tex_name) = pai.textures.get(file_idx).cloned() else {
                            continue;
                        };
                        match t.layer {
                            0 => tq.texture_name = tex_name,
                            1 => tq.texture_name1 = Some(tex_name),
                            2 => tq.texture_name2 = Some(tex_name),
                            _ => {}
                        }
                    }
                }

                AnimInfoType::MaterialColorAnim => {
                    let Some(node) = view.tree.find_by_idx_mut(pane_idx) else {
                        continue;
                    };
                    let Some(tq) = &mut node.textured_quad else {
                        continue;
                    };

                    for t in targets {
                        let v = eval_curve(&t.curve, frame) / 255.0;
                        if let TargetIndex::MaterialColor(c) = &t.target {
                            use nnbfl::bflan::targets::MaterialColorTarget::*;
                            match c {
                                BufferRed => tq.standard_material.interpolate_offset[0] = v,
                                BufferGreen => tq.standard_material.interpolate_offset[1] = v,
                                BufferBlue => tq.standard_material.interpolate_offset[2] = v,
                                BufferAlpha => tq.standard_material.interpolate_offset[3] = v,
                                Constant0Red | Color0Red | Color1Red | Color2Red | Color3Red
                                | Color4Red => tq.standard_material.interpolate_width[0] = v,
                                Constant0Green | Color0Green | Color1Green | Color2Green
                                | Color3Green | Color4Green => {
                                    tq.standard_material.interpolate_width[1] = v
                                }
                                Constant0Blue | Color0Blue | Color1Blue | Color2Blue
                                | Color3Blue | Color4Blue => {
                                    tq.standard_material.interpolate_width[2] = v
                                }
                                Constant0Alpha | Color0Alpha | Color1Alpha | Color2Alpha
                                | Color3Alpha | Color4Alpha => {
                                    tq.standard_material.interpolate_width[3] = v
                                }
                            }
                        }
                    }
                }

                AnimInfoType::VertexColorAnim => {
                    for t in targets {
                        let v = eval_curve(&t.curve, frame) / 255.0;

                        match &t.target {
                            TargetIndex::VertexColor(VertexColorTarget::PaneAlpha) => {
                                let mut apply_to = vec![pane_idx];
                                apply_to.extend(view.tree.descendants(pane_idx));

                                for idx in apply_to {
                                    if let Some(node) = view.tree.find_by_idx_mut(idx)
                                        && let Some(tq) = &mut node.textured_quad
                                    {
                                        tq.tint[3] = v;
                                        for c in tq.corner_tints.iter_mut() {
                                            c[3] = v;
                                        }
                                    }
                                }
                            }
                            _ => {
                                if let Some(node) = view.tree.find_by_idx_mut(pane_idx)
                                    && let Some(tq) = &mut node.textured_quad
                                {
                                    match &t.target {
                                        TargetIndex::VertexColor(VertexColorTarget::LeftTopRed) => {
                                            tq.corner_tints[0][0] = v
                                        }
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::LeftTopGreen,
                                        ) => tq.corner_tints[0][1] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::LeftTopBlue,
                                        ) => tq.corner_tints[0][2] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::LeftTopAlpha,
                                        ) => tq.corner_tints[0][3] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::RightTopRed,
                                        ) => tq.corner_tints[1][0] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::RightTopGreen,
                                        ) => tq.corner_tints[1][1] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::RightTopBlue,
                                        ) => tq.corner_tints[1][2] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::RightTopAlpha,
                                        ) => tq.corner_tints[1][3] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::LeftBottomRed,
                                        ) => tq.corner_tints[2][0] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::LeftBottomGreen,
                                        ) => tq.corner_tints[2][1] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::LeftBottomBlue,
                                        ) => tq.corner_tints[2][2] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::LeftBottomAlpha,
                                        ) => tq.corner_tints[2][3] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::RightBottomRed,
                                        ) => tq.corner_tints[3][0] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::RightBottomGreen,
                                        ) => tq.corner_tints[3][1] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::RightBottomBlue,
                                        ) => tq.corner_tints[3][2] = v,
                                        TargetIndex::VertexColor(
                                            VertexColorTarget::RightBottomAlpha,
                                        ) => tq.corner_tints[3][3] = v,
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }

                _ => {}
            }
        }
    }
}
