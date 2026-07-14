use nnbfl::bflyt::flags::WindowKind;
use nnbfl::bflyt::list::MaterialList;
use nnbfl::bflyt::pane::{TextureFlip, TextureUv, WindowPane};
use nnbfl::ui2d::types::{Color4u8, Vector2f};

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
    pub fn to_binary_index(self, total_frames: usize) -> Option<usize> {
        match total_frames {
            4 => match self {
                FrameKind::TopLeft => Some(0),
                FrameKind::TopRight => Some(1),
                FrameKind::BottomLeft => Some(2),
                FrameKind::BottomRight => Some(3),
                _ => None,
            },
            8 => match self {
                FrameKind::TopLeft => Some(0),
                FrameKind::TopRight => Some(1),
                FrameKind::BottomLeft => Some(2),
                FrameKind::BottomRight => Some(3),
                FrameKind::Left => Some(4),
                FrameKind::Right => Some(5),
                FrameKind::Top => Some(6),
                FrameKind::Bottom => Some(7),
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

    if !win.flag.not_draw_content {
        let inflation = Some((
            win.inflation_left as f32,
            win.inflation_right as f32,
            win.inflation_top as f32,
            win.inflation_bottom as f32,
        ));

        let local_rect = calculate_content_rect(effective_size, fs, inflation);

        content = Some(map_local_to_world(local_rect));
    }

    match win.frames.len() {
        4 => {
            let all_local_frames = calculate_frame_rects_four(effective_size, fs);

            for i in 0..4 {
                frames.push(map_local_to_world(all_local_frames[i]));
            }
        }

        8 => {
            let all_local_frames = calculate_frame_rects_around(effective_size, fs);

            for rect in all_local_frames {
                frames.push(map_local_to_world(rect));
            }
        }

        _ => {}
    }

    if win.frames.len() == 4 {}

    WindowLayoutGeometry { content, frames }
}

pub fn derive_from_window(
    win: &WindowPane,
    material_list: &MaterialList,
    corners: [[f32; 2]; 4],
    is_visible: bool,
    pane_idx: usize,
) -> Vec<TexturedQuad> {
    let mut out = Vec::new();

    let layout = calculate_window_layout(win, corners);

    if let Some(geom) = layout.content {
        if let Some(mat) = material_list
            .materials
            .get(win.content.material_index as usize)
        {
            let mut content_uvs = if win.content.picture_uvs.is_empty() {
                flipped_plain_uvs(mat.tex_maps.len(), TextureFlip::None)
            } else {
                win.content.picture_uvs.clone()
            };

            // TODO: probably make this work in shader?
            if content_uvs.is_empty() {
                content_uvs.push(TextureUv {
                    top_left: Vector2f::new(0.0, 0.0),
                    top_right: Vector2f::new(1.0, 0.0),
                    bottom_left: Vector2f::new(0.0, 1.0),
                    bottom_right: Vector2f::new(1.0, 1.0),
                });
            }

            if let Some(tq) = TexturedQuad::derive_from_material(
                MaterialPaneData {
                    base_section: &win.base,
                    top_left_vertex_color: &win.content.top_left_vertex_color,
                    top_right_vertex_color: &win.content.top_right_vertex_color,
                    bottom_left_vertex_color: &win.content.bottom_left_vertex_color,
                    bottom_right_vertex_color: &win.content.bottom_right_vertex_color,
                    material_idx: win.content.material_index,
                    texture_uvs: &content_uvs,
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
    }

    for geom in layout.frames {
        let Some(kind) = geom.frame_kind else {
            continue;
        };

        let Some(config_idx) = kind.to_binary_index(win.frames.len()) else {
            continue;
        };

        let Some(frame_data) = win.frames.get(config_idx) else {
            continue;
        };

        let Some(lt_frame_data) = win.frames.first() else {
            continue;
        };

        let base_material_idx = if win.flag.use_left_corner_material {
            lt_frame_data.material_index
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

        if win.flag.use_left_corner_material {
            if let Some(original_mat) = material_list
                .materials
                .get(frame_data.material_index as usize)
            {
                mat.tex_maps = original_mat.tex_maps.clone();
                mat.interpolation_colors = original_mat.interpolation_colors.clone();
            }
        }

        let frame_uvs = flipped_plain_uvs(mat.tex_maps.len(), frame_data.texture_flip_mode);

        let (tl, tr, bl, br) = if win.flag.use_vertex_color_for_all_window {
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

        if let Some(tq) = TexturedQuad::derive_from_material(
            MaterialPaneData {
                base_section: &win.base,
                top_left_vertex_color: tl,
                top_right_vertex_color: tr,
                bottom_left_vertex_color: bl,
                bottom_right_vertex_color: br,
                material_idx: frame_data.material_index,
                texture_uvs: &frame_uvs,
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
