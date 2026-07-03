use nnbfl::bflyt::flags::BflytOrigin;
use nnbfl::bflyt::list::MaterialTextureSrt;
use nnbfl::ui2d::userdata::ResUi2dUserDataInner;
use nnbfl::{
    bflan::{
        anim_info::{AnimContent, AnimInfo, AnimInfoType, AnimType, PaneAnimInfo},
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
}

fn find_next_anim(bflan: &Bflan) -> Option<String> {
    if let Some(ud) = &bflan.anim_tag.user_data {
        for entry in &ud.user_data {
            if entry.o_name == "CommandPlayEnd_Play"
                && let Some(ResUi2dUserDataInner::String(value)) = entry.data_array.first()
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
}

impl AnimPlayer {
    pub fn new() -> Self {
        Self {
            anims: Vec::new(),
            active: None,
        }
    }

    pub fn load(&mut self, bflan: Bflan) {
        self.anims.push(AnimInstance::new(bflan));
    }

    pub fn play(&mut self, name: &str) {
        if let Some(idx) = self.anims.iter().position(|a| a.name == name) {
            if let Some(prev) = self.active
                && prev < self.anims.len()
            {
                self.anims[prev].playing = false;
            }

            self.anims[idx].frame = 0.0;
            self.anims[idx].playing = true;
            self.anims[idx].autoplay = true;
            self.active = Some(idx);
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

fn node_screen_pos(node: &crate::pane_tree::PaneNode, trans_x: f32, trans_y: f32) -> (f32, f32) {
    let base = node.section.get_base_pane();
    let (ox, oy, w, h) = base
        .map(|b| {
            (
                b.origin.origin_x,
                b.origin.origin_y,
                b.size.x * b.scale.x,
                b.size.y * b.scale.y,
            )
        })
        .unwrap_or((
            BflytOrigin::Center,
            BflytOrigin::Center,
            node.world_size.x,
            node.world_size.y,
        ));

    let cx = node.parent_anchor.x + trans_x;
    let cy = node.parent_anchor.y - trans_y;

    let tl_x = match ox {
        BflytOrigin::Center => cx - w * 0.5,
        BflytOrigin::LeftTop => cx,
        BflytOrigin::RightBottom => cx - w,
    };
    let tl_y = match oy {
        BflytOrigin::Center => cy - h * 0.5,
        BflytOrigin::LeftTop => cy,
        BflytOrigin::RightBottom => cy - h,
    };

    (tl_x, tl_y)
}

fn cascade_translate(view: &mut BflytView, pane_idx: usize, new_trans_x: f32, new_trans_y: f32) {
    let idx_map = view.tree.build_idx_map();
    let Some(&node_ptr) = idx_map.get(&pane_idx) else {
        return;
    };
    let base_node = unsafe { &*node_ptr };

    let Some(base) = base_node.section.get_base_pane() else {
        return;
    };

    let (base_tx, base_ty) = (base.translation.x, base.translation.y);

    let (new_x, new_y) = node_screen_pos(base_node, new_trans_x, new_trans_y);
    let (base_x, base_y) = node_screen_pos(base_node, base_tx, base_ty);
    let dx = new_x - base_x;
    let dy = new_y - base_y;

    let mut affected = vec![pane_idx];
    affected.extend(view.tree.descendants(pane_idx));

    for idx in affected {
        let Some(&node_ptr) = idx_map.get(&idx) else {
            continue;
        };
        let node = unsafe { &mut *node_ptr };

        node.world_pos.x += dx;
        node.world_pos.y += dy;

        for corner in &mut node.plain_quad.corners {
            corner[0] += dx;
            corner[1] += dy;
        }

        if let Some(tq) = &mut node.textured_quad {
            tq.x = node.world_pos.x;
            tq.y = node.world_pos.y;

            for corner in &mut tq.corners {
                corner[0] += dx;
                corner[1] += dy;
            }
        }

        node.world_corners.translate(Vector2f::new(dx, dy));

        node.dirty.insert(DirtyFlags::VERTICES);
    }
}

fn cascade_visibility(view: &mut BflytView, pane_idx: usize, visible: bool) {
    let idx_map = view.tree.build_idx_map();
    let mut affected = vec![pane_idx];
    affected.extend(view.tree.descendants(pane_idx));

    for idx in affected {
        let Some(&node_ptr) = idx_map.get(&idx) else {
            continue;
        };
        let node = unsafe { &mut *node_ptr };
        node.visible = visible;

        node.plain_quad.color = if visible {
            node.section.section_color()
        } else {
            [0.0; 4]
        };

        if let Some(tq) = &mut node.textured_quad {
            tq.standard_material.visible = visible as u32;
        }
        node.dirty
            .insert(DirtyFlags::MATERIAL | DirtyFlags::VERTICES);
    }
}

fn apply_anim(pai: &PaneAnimInfo, frame: f32, view: &mut BflytView) {
    let pane_by_name = view.tree.label_to_idx();

    for content in &pai.contents {
        let name = content.name.trim_end_matches('\0');
        let Some(&pane_idx) = pane_by_name.get(name) else {
            continue;
        };

        match content.anim_type {
            AnimType::Pane | AnimType::PaneExt => {
                apply_pane_content(content, frame, pane_idx, view);
            }
            AnimType::Material => {
                apply_material_content(content, frame, pane_idx, view, pai);
            }
            _ => {}
        }
    }
}

fn apply_pane_content(content: &AnimContent, frame: f32, pane_idx: usize, view: &mut BflytView) {
    let idx_map = view.tree.build_idx_map();

    for info in &content.infos {
        let AnimInfo::Standard { anim_type, targets } = info else {
            continue;
        };

        match anim_type {
            AnimInfoType::PaneSrtAnim => {
                let (base_tx, base_ty, base_w, base_h) = {
                    let Some(&ptr) = idx_map.get(&pane_idx) else {
                        continue;
                    };
                    let node = unsafe { &*ptr };

                    let (tx, ty) = node
                        .section
                        .get_base_pane()
                        .map(|base| (base.translation.x, base.translation.y))
                        .unwrap_or((0.0, 0.0));

                    let base = node.section.get_base_pane();
                    let w = base
                        .map(|b| b.size.x * b.scale.x)
                        .unwrap_or(node.world_size.x);

                    let h = base
                        .map(|b| b.size.y * b.scale.y)
                        .unwrap_or(node.world_size.y);

                    (tx, ty, w, h)
                };

                let mut new_tx = base_tx;
                let mut new_ty = base_ty;
                let mut scale_x = 1.0f32;
                let mut scale_y = 1.0f32;
                let mut new_w = base_w;
                let mut new_h = base_h;

                for t in targets {
                    let v = eval_curve(&t.curve, frame);
                    match &t.target {
                        TargetIndex::PaneSrt(PaneSrtTarget::TranslateX) => new_tx = v,
                        TargetIndex::PaneSrt(PaneSrtTarget::TranslateY) => new_ty = v,
                        TargetIndex::PaneSrt(PaneSrtTarget::ScaleX) => scale_x = v,
                        TargetIndex::PaneSrt(PaneSrtTarget::ScaleY) => scale_y = v,
                        TargetIndex::PaneSrt(PaneSrtTarget::SizeX) => new_w = v,
                        TargetIndex::PaneSrt(PaneSrtTarget::SizeY) => new_h = v,
                        _ => {}
                    }
                }

                cascade_translate(view, pane_idx, new_tx, new_ty);

                let final_w = new_w * scale_x;
                let final_h = new_h * scale_y;
                if ((final_w - base_w).abs() > f32::EPSILON
                    || (final_h - base_h).abs() > f32::EPSILON)
                    && let Some(&ptr) = idx_map.get(&pane_idx)
                {
                    let node = unsafe { &mut *ptr };
                    node.world_size.x = final_w;
                    node.world_size.y = final_h;
                    node.plain_quad.width = final_w;
                    node.plain_quad.height = final_h;
                    if let Some(tq) = &mut node.textured_quad {
                        tq.width = final_w;
                        tq.height = final_h;
                    }
                    node.dirty.insert(DirtyFlags::VERTICES);
                }
            }

            AnimInfoType::VisibilityAnim => {
                for t in targets {
                    let visible = eval_curve_step_u16(&t.curve, frame) != 0;
                    cascade_visibility(view, pane_idx, visible);
                }
            }

            AnimInfoType::VertexColorAnim => {
                let has_own_tq = idx_map
                    .get(&pane_idx)
                    .map(|&ptr| unsafe { (*ptr).textured_quad.is_some() })
                    .unwrap_or(false);

                let apply_to: Vec<usize> = if has_own_tq {
                    vec![pane_idx]
                } else {
                    let mut v = vec![pane_idx];
                    v.extend(view.tree.descendants(pane_idx));
                    v
                };

                for t in targets {
                    let v = eval_curve(&t.curve, frame) / 255.0;
                    for &idx in &apply_to {
                        let Some(&ptr) = idx_map.get(&idx) else {
                            continue;
                        };
                        let node = unsafe { &mut *ptr };
                        let Some(tq) = &mut node.textured_quad else {
                            continue;
                        };

                        match &t.target {
                            TargetIndex::VertexColor(VertexColorTarget::PaneAlpha) => {
                                tq.tint[3] = v;
                                for c in tq.corner_tints.iter_mut() {
                                    c[3] = v;
                                }
                            }
                            TargetIndex::VertexColor(VertexColorTarget::LeftTopRed) => {
                                tq.corner_tints[0][0] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::LeftTopGreen) => {
                                tq.corner_tints[0][1] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::LeftTopBlue) => {
                                tq.corner_tints[0][2] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::LeftTopAlpha) => {
                                tq.corner_tints[0][3] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::RightTopRed) => {
                                tq.corner_tints[1][0] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::RightTopGreen) => {
                                tq.corner_tints[1][1] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::RightTopBlue) => {
                                tq.corner_tints[1][2] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::RightTopAlpha) => {
                                tq.corner_tints[1][3] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::LeftBottomRed) => {
                                tq.corner_tints[2][0] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::LeftBottomGreen) => {
                                tq.corner_tints[2][1] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::LeftBottomBlue) => {
                                tq.corner_tints[2][2] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::LeftBottomAlpha) => {
                                tq.corner_tints[2][3] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::RightBottomRed) => {
                                tq.corner_tints[3][0] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::RightBottomGreen) => {
                                tq.corner_tints[3][1] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::RightBottomBlue) => {
                                tq.corner_tints[3][2] = v
                            }
                            TargetIndex::VertexColor(VertexColorTarget::RightBottomAlpha) => {
                                tq.corner_tints[3][3] = v
                            }
                            _ => {}
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

fn apply_material_content(
    content: &AnimContent,
    frame: f32,
    pane_idx: usize,
    view: &mut BflytView,
    pai: &PaneAnimInfo,
) {
    let idx_map = view.tree.build_idx_map();

    for info in &content.infos {
        let AnimInfo::Standard { anim_type, targets } = info else {
            continue;
        };

        let tq = {
            let Some(&ptr) = idx_map.get(&pane_idx) else {
                continue;
            };
            let node = unsafe { &mut *ptr };
            let Some(tq) = &mut node.textured_quad else {
                continue;
            };
            tq as *mut _
        };
        let tq: &mut crate::renderer::textured_quad::TexturedQuad = unsafe { &mut *tq };

        match anim_type {
            AnimInfoType::TextureSrtAnim => {
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
                for t in targets {
                    let v = eval_curve(&t.curve, frame);
                    match &t.target {
                        TargetIndex::IndirectSrt(IndirectSrtTarget::Rotate) => {
                            let rad = v.to_radians();
                            tq.standard_material.indirect_mtx0[1] = -rad.sin();
                            tq.standard_material.indirect_mtx1[0] = rad.sin();
                        }
                        TargetIndex::IndirectSrt(IndirectSrtTarget::ScaleU) => {
                            tq.standard_material.indirect_mtx0[0] = v;
                        }
                        TargetIndex::IndirectSrt(IndirectSrtTarget::ScaleV) => {
                            tq.standard_material.indirect_mtx1[1] = v;
                        }
                        _ => {}
                    }
                }
            }

            AnimInfoType::TexturePatternAnim => {
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
                            Constant0Blue | Color0Blue | Color1Blue | Color2Blue | Color3Blue
                            | Color4Blue => tq.standard_material.interpolate_width[2] = v,
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
                    let v = eval_curve(&t.curve, frame);
                    match &t.target {
                        TargetIndex::VertexColor(VertexColorTarget::LeftTopRed) => {
                            tq.tint[0] = v / 255.0
                        }
                        TargetIndex::VertexColor(VertexColorTarget::LeftTopGreen) => {
                            tq.tint[1] = v / 255.0
                        }
                        TargetIndex::VertexColor(VertexColorTarget::LeftTopBlue) => {
                            tq.tint[2] = v / 255.0
                        }
                        TargetIndex::VertexColor(VertexColorTarget::LeftTopAlpha) => {
                            tq.tint[3] = v / 255.0
                        }
                        TargetIndex::VertexColor(VertexColorTarget::PaneAlpha) => {
                            tq.tint[3] = v / 255.0
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }
}
