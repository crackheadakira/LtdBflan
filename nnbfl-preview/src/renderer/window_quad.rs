use nnbfl::bflyt::flags::WindowKind;
use nnbfl::bflyt::list::MaterialList;
use nnbfl::bflyt::pane::{TextureFlip, TextureUv, WindowPane};
use nnbfl::ui2d::types::{Color4u8, Vector2f, Vector3f};
use tomolib::formats::bntx::Bntx;

use crate::renderer::textured_quad::{MaterialPaneData, TexturedQuad};

pub fn apply_texture_flip(texture_flip: TextureFlip, uvs: [[f32; 2]; 4]) -> [[f32; 2]; 4] {
    let [tl, tr, bl, br] = uvs;
    match texture_flip {
        TextureFlip::None => uvs,
        TextureFlip::FlipU => [tr, tl, br, bl],
        TextureFlip::FlipV => [bl, br, tl, tr],
        TextureFlip::Rotate180 => [br, bl, tr, tl],
        _ => uvs,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrameKind {
    Left,
    TopLeft,
    BottomLeft,
    Right,
    TopRight,
    BottomRight,
    Top,
    Bottom,
}

impl FrameKind {
    pub fn to_binary_index(self, total_frames: usize) -> Option<(usize, Option<TextureFlip>)> {
        match total_frames {
            1 => Some((0, None)),
            2 => match self {
                FrameKind::Left => Some((0, None)),
                FrameKind::Right => Some((1, None)),
                _ => None,
            },
            4 => match self {
                FrameKind::TopLeft => Some((0, None)),
                FrameKind::TopRight => Some((1, None)),
                FrameKind::BottomLeft => Some((2, None)),
                FrameKind::BottomRight => Some((3, None)),
                _ => None,
            },
            8 => match self {
                FrameKind::TopLeft => Some((0, None)),
                FrameKind::TopRight => Some((1, None)),
                FrameKind::BottomLeft => Some((2, None)),
                FrameKind::BottomRight => Some((3, None)),
                FrameKind::Left => Some((4, None)),
                FrameKind::Right => Some((5, None)),
                FrameKind::Top => Some((6, None)),
                FrameKind::Bottom => Some((7, None)),
            },
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrameRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub frame_kind: Option<FrameKind>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct FrameSizeF {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

pub fn resolve_frame_size(win: &WindowPane, pane_size: Vector2f) -> (FrameSizeF, Vector2f) {
    let fs_left = win.frame_size_left as f32;
    let fs_right = win.frame_size_right as f32;
    let fs_top = win.frame_size_top as f32;
    let fs_bottom = win.frame_size_bottom as f32;

    match win.flag.window_kind {
        WindowKind::Around => (
            FrameSizeF {
                left: fs_left,
                right: fs_right,
                top: fs_top,
                bottom: fs_bottom,
            },
            pane_size,
        ),
        WindowKind::Horizontal => (
            FrameSizeF {
                left: fs_left,
                right: fs_right,
                top: 0.0,
                bottom: 0.0,
            },
            Vector2f::new(pane_size.x, fs_top),
        ),
        WindowKind::HorizontalNoContent => {
            let effective_size = Vector2f::new(pane_size.x, fs_top);

            (
                FrameSizeF {
                    left: pane_size.x - fs_right,
                    right: fs_right,
                    top: 0.0,
                    bottom: 0.0,
                },
                effective_size,
            )
        }
    }
}

pub fn calculate_content_rect(
    size: Vector2f,
    frame_size: FrameSizeF,
    inflation: Option<(f32, f32, f32, f32)>,
) -> FrameRect {
    let mut x = frame_size.left;
    let mut y = frame_size.top;
    let mut w = size.x - frame_size.left - frame_size.right;
    let mut h = size.y - frame_size.top - frame_size.bottom;

    if let Some((infl_l, infl_r, infl_t, infl_b)) = inflation {
        x -= infl_l;
        y -= infl_t;

        w += infl_l + infl_r;
        h += infl_t + infl_b;
    }

    FrameRect {
        x,
        y,
        width: w,
        height: h,
        frame_kind: None,
    }
}

pub fn calculate_frame_rects_four(size: Vector2f, fs: FrameSizeF) -> [FrameRect; 4] {
    let x0 = 0.0;
    let y0 = 0.0;

    let span_w = size.x - fs.left - fs.right;
    let span_h = size.y - fs.top - fs.bottom;

    let tl_w = fs.left + span_w;
    let tl_h = fs.top;

    let tr_w = fs.right;
    let tr_h = fs.top + span_h;
    let tr_x = size.x - tr_w;

    let br_w = fs.right + span_w;
    let br_h = fs.bottom;
    let br_x = size.x - br_w;
    let br_y = size.y - br_h;

    let bl_w = fs.left;
    let bl_h = fs.bottom + span_h;
    let bl_y = size.y - bl_h;

    [
        FrameRect {
            x: x0,
            y: y0,
            width: tl_w,
            height: tl_h,
            frame_kind: Some(FrameKind::TopLeft),
        },
        FrameRect {
            x: tr_x,
            y: y0,
            width: tr_w,
            height: tr_h,
            frame_kind: Some(FrameKind::TopRight),
        },
        FrameRect {
            x: x0,
            y: bl_y,
            width: bl_w,
            height: bl_h,
            frame_kind: Some(FrameKind::BottomLeft),
        },
        FrameRect {
            x: br_x,
            y: br_y,
            width: br_w,
            height: br_h,
            frame_kind: Some(FrameKind::BottomRight),
        },
    ]
}

pub fn calculate_frame_rects_around(size: Vector2f, fs: FrameSizeF) -> [FrameRect; 8] {
    let x0 = 0.0;
    let x1 = fs.left;
    let x2 = size.x - fs.right;

    let y0 = 0.0;
    let y1 = fs.top;
    let y2 = size.y - fs.bottom;

    let w0 = fs.left;
    let w1 = size.x - fs.left - fs.right;
    let w2 = fs.right;

    let h0 = fs.top;
    let h1 = size.y - fs.top - fs.bottom;
    let h2 = fs.bottom;

    [
        FrameRect {
            x: x0,
            y: y0,
            width: w0,
            height: h0,
            frame_kind: Some(FrameKind::TopLeft),
        },
        FrameRect {
            x: x2,
            y: y0,
            width: w2,
            height: h0,
            frame_kind: Some(FrameKind::TopRight),
        },
        FrameRect {
            x: x0,
            y: y2,
            width: w0,
            height: h2,
            frame_kind: Some(FrameKind::BottomLeft),
        },
        FrameRect {
            x: x2,
            y: y2,
            width: w2,
            height: h2,
            frame_kind: Some(FrameKind::BottomRight),
        },
        FrameRect {
            x: x0,
            y: y1,
            width: w0,
            height: h1,
            frame_kind: Some(FrameKind::Left),
        },
        FrameRect {
            x: x2,
            y: y1,
            width: w2,
            height: h1,
            frame_kind: Some(FrameKind::Right),
        },
        FrameRect {
            x: x1,
            y: y0,
            width: w1,
            height: h0,
            frame_kind: Some(FrameKind::Top),
        },
        FrameRect {
            x: x1,
            y: y2,
            width: w1,
            height: h2,
            frame_kind: Some(FrameKind::Bottom),
        },
    ]
}

pub fn calculate_frame_rects_horizontal(size: Vector2f, fs: FrameSizeF) -> [FrameRect; 2] {
    [
        FrameRect {
            x: 0.0,
            y: 0.0,
            width: fs.left,
            height: size.y,
            frame_kind: Some(FrameKind::Left),
        },
        FrameRect {
            x: size.x - fs.right,
            y: 0.0,
            width: fs.right,
            height: size.y,
            frame_kind: Some(FrameKind::Right),
        },
    ]
}

const PLAIN_UV_CORNERS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

fn flipped_plain_uvs(texture_count: usize, flip: TextureFlip) -> Vec<TextureUv> {
    let [tl, tr, bl, br] = apply_texture_flip(flip, PLAIN_UV_CORNERS);
    (0..texture_count)
        .map(|_| TextureUv {
            top_left: Vector2f::new(tl[0], tl[1]),
            top_right: Vector2f::new(tr[0], tr[1]),
            bottom_left: Vector2f::new(bl[0], bl[1]),
            bottom_right: Vector2f::new(br[0], br[1]),
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct WindowPieceGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub corners: [[f32; 2]; 4],
    pub frame_kind: Option<FrameKind>,
}

impl WindowPieceGeometry {
    pub fn flip_horizontal(&mut self) {
        self.corners.swap(0, 1);
        self.corners.swap(2, 3);
    }

    pub fn flip_vertical(&mut self) {
        self.corners.swap(0, 2);
        self.corners.swap(1, 3);
    }
}

#[derive(Clone, Debug)]
pub struct WindowLayoutGeometry {
    pub content: Option<WindowPieceGeometry>,
    pub frames: Vec<WindowPieceGeometry>,
}

pub fn calculate_window_layout(win: &WindowPane, corners: [[f32; 2]; 4]) -> WindowLayoutGeometry {
    let mut content = None;
    let mut frames = Vec::new();

    let unscaled_size = win.base.size;

    let (fs, effective_size) = resolve_frame_size(win, unscaled_size);

    if effective_size.x <= 0.0 || effective_size.y <= 0.0 {
        return WindowLayoutGeometry { content, frames };
    }

    let map_local_to_world = |rect: FrameRect| -> WindowPieceGeometry {
        let u0 = rect.x / effective_size.x;
        let u1 = (rect.x + rect.width) / effective_size.x;
        let v0 = rect.y / effective_size.y;
        let v1 = (rect.y + rect.height) / effective_size.y;

        let piece_corners = [
            interpolate_corner(corners, u0, v0),
            interpolate_corner(corners, u1, v0),
            interpolate_corner(corners, u0, v1),
            interpolate_corner(corners, u1, v1),
        ];

        let x = piece_corners[0][0];
        let y = piece_corners[0][1];
        let width = piece_corners[1][0] - piece_corners[0][0];
        let height = piece_corners[2][1] - piece_corners[0][1];

        WindowPieceGeometry {
            x,
            y,
            width,
            height,
            corners: piece_corners,
            frame_kind: rect.frame_kind,
        }
    };

    if !win.flag.not_draw_content && win.flag.window_kind != WindowKind::HorizontalNoContent {
        let inflation = Some((
            win.inflation_left as f32,
            win.inflation_right as f32,
            win.inflation_top as f32,
            win.inflation_bottom as f32,
        ));

        let local_rect = calculate_content_rect(effective_size, fs, inflation);

        content = Some(map_local_to_world(local_rect));
    }

    match win.flag.window_kind {
        WindowKind::Around => match win.frames.len() {
            4 => {
                let all_local_frames = calculate_frame_rects_four(effective_size, fs);

                for rect in all_local_frames {
                    frames.push(map_local_to_world(rect));
                }
            }

            8 => {
                let all_local_frames = calculate_frame_rects_around(effective_size, fs);

                for rect in all_local_frames {
                    frames.push(map_local_to_world(rect));
                }
            }

            1 => {
                let all_local_frames = calculate_frame_rects_four(effective_size, fs);

                for rect in all_local_frames {
                    let mut world_frame = map_local_to_world(rect);

                    if let Some(kind) = rect.frame_kind {
                        match kind {
                            FrameKind::TopRight => {
                                world_frame.flip_horizontal();
                            }
                            FrameKind::BottomLeft => {
                                world_frame.flip_vertical();
                            }
                            FrameKind::BottomRight => {
                                world_frame.flip_horizontal();
                                world_frame.flip_vertical();
                            }
                            _ => {}
                        }
                    }

                    frames.push(world_frame);
                }
            }

            _ => {}
        },

        WindowKind::Horizontal | WindowKind::HorizontalNoContent => {
            let all_local_frames = calculate_frame_rects_horizontal(effective_size, fs);

            match win.frames.len() {
                1 => {
                    for rect in all_local_frames {
                        let mut world_frame = map_local_to_world(rect);

                        if let Some(kind) = rect.frame_kind
                            && kind == FrameKind::Right
                        {
                            world_frame.flip_horizontal();
                        }

                        frames.push(world_frame);
                    }
                }

                2 => {
                    for rect in all_local_frames {
                        frames.push(map_local_to_world(rect));
                    }
                }

                _ => {}
            }
        }
    }

    WindowLayoutGeometry { content, frames }
}

pub fn derive_from_window(
    win: &WindowPane,
    material_list: &MaterialList,
    corners: [[f32; 2]; 4],
    is_visible: bool,
    pane_idx: usize,
    bntxs: &[Bntx],
) -> Vec<TexturedQuad> {
    let mut out = Vec::new();

    let layout = calculate_window_layout(win, corners);

    if let Some(geom) = layout.content
        && let Some(mat) = material_list
            .materials
            .get(win.content.material_index as usize)
    {
        let mut content_uvs = if win.content.picture_uvs.is_empty() {
            flipped_plain_uvs(mat.tex_maps.len(), TextureFlip::None)
        } else {
            win.content.picture_uvs.clone()
        };

        if content_uvs.is_empty() {
            content_uvs.push(TextureUv {
                top_left: Vector2f::new(0.0, 0.0),
                top_right: Vector2f::new(1.0, 0.0),
                bottom_left: Vector2f::new(0.0, 1.0),
                bottom_right: Vector2f::new(1.0, 1.0),
            });
        }

        let content_colors = (
            &win.content.top_left_vertex_color,
            &win.content.top_right_vertex_color,
            &win.content.bottom_left_vertex_color,
            &win.content.bottom_right_vertex_color,
        );

        let corner_tints = interpolate_slice_corners(geom.corners, corners, content_colors);

        if let Some(tq) = TexturedQuad::derive_from_material(
            MaterialPaneData {
                base_section: &win.base,
                corner_tints,
                material_idx: win.content.material_index,
                piece_id: 0,
                texture_uvs: &content_uvs,
                rotation: Vector3f::default(),
            },
            mat,
            Vector2f::new(geom.x, geom.y),
            Vector2f::new(geom.width, geom.height),
            geom.corners,
            is_visible,
            pane_idx,
        ) {
            out.push(tq);
        }
    }

    for (frame_idx, geom) in layout.frames.into_iter().enumerate() {
        let Some(kind) = geom.frame_kind else {
            continue;
        };

        let Some((config_idx, flip_override)) = kind.to_binary_index(win.frames.len()) else {
            continue;
        };

        let Some(frame_data) = win.frames.get(config_idx) else {
            continue;
        };

        let base_material_idx = if win.flag.use_left_corner_material {
            if let Some(lt_frame_data) = win.frames.first() {
                lt_frame_data.material_index
            } else {
                continue;
            }
        } else {
            frame_data.material_index
        };

        let Some(mut mat) = material_list
            .materials
            .get(base_material_idx as usize)
            .cloned()
        else {
            continue;
        };

        if win.flag.use_left_corner_material
            && let Some(original_mat) = material_list
                .materials
                .get(frame_data.material_index as usize)
        {
            mat.tex_maps = original_mat.tex_maps.clone();
        }

        let (tex_w, tex_h) = if let Some(tex_name) = mat.tex_maps.first().map(|m| &m.texture_name) {
            bntxs
                .iter()
                .flat_map(|b| &b.textures)
                .find(|t| t.name == *tex_name)
                .map(|t| (t.info.width as f32, t.info.height as f32))
                .unwrap_or((1.0, 1.0))
        } else {
            (1.0, 1.0)
        };

        let effective_flip = flip_override.unwrap_or(frame_data.texture_flip_mode);

        let frame_uvs = calculate_scaled_frame_uvs(
            geom.width,
            geom.height,
            tex_w,
            tex_h,
            kind,
            win.flag.window_kind,
            effective_flip,
            mat.tex_maps.len(),
        );

        let frame_colors = if win.flag.use_vertex_color_for_all_window {
            (
                &win.content.top_left_vertex_color,
                &win.content.top_right_vertex_color,
                &win.content.bottom_left_vertex_color,
                &win.content.bottom_right_vertex_color,
            )
        } else {
            static EMPTY: Color4u8 = Color4u8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            };
            (&EMPTY, &EMPTY, &EMPTY, &EMPTY)
        };

        let corner_tints = interpolate_slice_corners(geom.corners, corners, frame_colors);

        if let Some(tq) = TexturedQuad::derive_from_material(
            MaterialPaneData {
                base_section: &win.base,
                corner_tints,
                material_idx: base_material_idx,
                piece_id: frame_idx + 1,
                texture_uvs: &frame_uvs,
                rotation: Vector3f::default(),
            },
            &mat,
            Vector2f::new(geom.x, geom.y),
            Vector2f::new(geom.width, geom.height),
            geom.corners,
            is_visible,
            pane_idx,
        ) {
            out.push(tq);
        }
    }

    out
}

fn normalize_vertex(pt: [f32; 2], window_corners: [[f32; 2]; 4]) -> [f32; 2] {
    let tl = window_corners[0];
    let br = window_corners[3];

    let width = br[0] - tl[0];
    let height = br[1] - tl[1];

    let u = if width > 0.0 {
        (pt[0] - tl[0]) / width
    } else {
        0.0
    };

    let v = if height > 0.0 {
        (pt[1] - tl[1]) / height
    } else {
        0.0
    };

    [u, v]
}

pub fn interpolate_slice_corners(
    geom_corners: [[f32; 2]; 4],
    window_corners: [[f32; 2]; 4],
    global_colors: (&Color4u8, &Color4u8, &Color4u8, &Color4u8),
) -> [[f32; 4]; 4] {
    let to_f32_rgba = |color: &Color4u8| -> [f32; 4] {
        let rgba: [f32; 4] = (*color).into();
        if rgba[3] > 0.0 {
            rgba
        } else {
            [1.0, 1.0, 1.0, 1.0]
        }
    };

    let tl = to_f32_rgba(global_colors.0);
    let tr = to_f32_rgba(global_colors.1);
    let bl = to_f32_rgba(global_colors.2);
    let br = to_f32_rgba(global_colors.3);

    let lerp_color = |u: f32, v: f32| -> [f32; 4] {
        let mut final_color = [0.0; 4];
        for i in 0..4 {
            let top = tl[i] * (1.0 - u) + tr[i] * u;
            let bottom = bl[i] * (1.0 - u) + br[i] * u;
            final_color[i] = top * (1.0 - v) + bottom * v;
        }
        final_color
    };

    let uv_tl = normalize_vertex(geom_corners[0], window_corners);
    let uv_tr = normalize_vertex(geom_corners[1], window_corners);
    let uv_bl = normalize_vertex(geom_corners[2], window_corners);
    let uv_br = normalize_vertex(geom_corners[3], window_corners);

    [
        lerp_color(uv_tl[0], uv_tl[1]),
        lerp_color(uv_tr[0], uv_tr[1]),
        lerp_color(uv_bl[0], uv_bl[1]),
        lerp_color(uv_br[0], uv_br[1]),
    ]
}

pub fn calculate_scaled_frame_uvs(
    geom_width: f32,
    geom_height: f32,
    tex_w: f32,
    tex_h: f32,
    kind: FrameKind,
    window_kind: WindowKind,
    flip_mode: TextureFlip,
    texture_maps_count: usize,
) -> Vec<TextureUv> {
    let mut frame_uvs = flipped_plain_uvs(texture_maps_count, TextureFlip::None);

    let scale_factor = (geom_width / tex_w).min(geom_height / tex_h);

    let u_scale = (geom_width / scale_factor) / tex_w;
    let v_scale = (geom_height / scale_factor) / tex_h;

    if window_kind == WindowKind::Around {
        for uv_set in &mut frame_uvs {
            let (anchor_u, anchor_v) = match kind {
                FrameKind::TopLeft => (0.0, 0.0),
                FrameKind::Top => (0.5, 0.0),
                FrameKind::TopRight => (1.0, 0.0),
                FrameKind::Left => (0.0, 0.5),
                FrameKind::Right => (1.0, 0.5),
                FrameKind::BottomLeft => (0.0, 1.0),
                FrameKind::Bottom => (0.5, 1.0),
                FrameKind::BottomRight => (1.0, 1.0),
            };

            apply_anchored_scale(uv_set, u_scale, v_scale, anchor_u, anchor_v);
        }
    } else if window_kind == WindowKind::Horizontal
        || window_kind == WindowKind::HorizontalNoContent
    {
        let is_stretchy_piece =
            window_kind == WindowKind::HorizontalNoContent && kind == FrameKind::Left;

        let effective_u_scale = if is_stretchy_piece { u_scale } else { 1.0 };
        let effective_v_scale = 1.0;

        let (anchor_u, anchor_v) = match kind {
            FrameKind::Left => (0.0, 0.5),
            FrameKind::Right => (1.0, 0.5),
            _ => (0.5, 0.5),
        };

        for uv_set in &mut frame_uvs {
            apply_anchored_scale(
                uv_set,
                effective_u_scale,
                effective_v_scale,
                anchor_u,
                anchor_v,
            );
        }
    }

    // TODO: didn't seem to fix 1-frame issue for around being anchored to wrong positions.
    for uv_set in &mut frame_uvs {
        let corners = [
            [uv_set.top_left.x, uv_set.top_left.y],
            [uv_set.top_right.x, uv_set.top_right.y],
            [uv_set.bottom_left.x, uv_set.bottom_left.y],
            [uv_set.bottom_right.x, uv_set.bottom_right.y],
        ];

        let flipped = apply_texture_flip(flip_mode, corners);

        uv_set.top_left = Vector2f::new(flipped[0][0], flipped[0][1]);
        uv_set.top_right = Vector2f::new(flipped[1][0], flipped[1][1]);
        uv_set.bottom_left = Vector2f::new(flipped[2][0], flipped[2][1]);
        uv_set.bottom_right = Vector2f::new(flipped[3][0], flipped[3][1]);
    }

    frame_uvs
}

fn apply_anchored_scale(
    uv_set: &mut TextureUv,
    u_scale: f32,
    v_scale: f32,
    anchor_u: f32,
    anchor_v: f32,
) {
    let scale_coord =
        |val: f32, scale: f32, anchor: f32| -> f32 { anchor + (val - anchor) * scale };

    uv_set.top_left.x = scale_coord(uv_set.top_left.x, u_scale, anchor_u);
    uv_set.top_left.y = scale_coord(uv_set.top_left.y, v_scale, anchor_v);

    uv_set.top_right.x = scale_coord(uv_set.top_right.x, u_scale, anchor_u);
    uv_set.top_right.y = scale_coord(uv_set.top_right.y, v_scale, anchor_v);

    uv_set.bottom_left.x = scale_coord(uv_set.bottom_left.x, u_scale, anchor_u);
    uv_set.bottom_left.y = scale_coord(uv_set.bottom_left.y, v_scale, anchor_v);

    uv_set.bottom_right.x = scale_coord(uv_set.bottom_right.x, u_scale, anchor_u);
    uv_set.bottom_right.y = scale_coord(uv_set.bottom_right.y, v_scale, anchor_v);
}

fn interpolate_corner(corners: [[f32; 2]; 4], u: f32, v: f32) -> [f32; 2] {
    let [tl, tr, bl, br] = corners;

    let tx = tl[0] + u * (tr[0] - tl[0]);
    let ty = tl[1] + u * (tr[1] - tl[1]);

    let bx = bl[0] + u * (br[0] - bl[0]);
    let by = bl[1] + u * (br[1] - bl[1]);

    let x = tx + v * (bx - tx);
    let y = ty + v * (by - ty);

    [x, y]
}
