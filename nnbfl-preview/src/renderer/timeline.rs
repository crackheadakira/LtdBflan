use std::collections::HashSet;

use bytemuck::{Pod, Zeroable};
use nnbfl::{
    bflan::{anim_info::AnimInfo, curves::Curve},
    ui2d::types::Color4f,
};
use wgpu::util::DeviceExt;

use crate::anim_state::{AnimInstance, eval_curve};

pub const TIMELINE_RULER_HEIGHT: f32 = 22.0;
pub const TIMELINE_MARKER_RADIUS: f32 = 4.0;
pub const TIMELINE_MIN_VISIBLE_FRAMES: f32 = 4.0;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TimelineVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Default, Clone)]
pub struct TimelineGeometry {
    pub lines: Vec<TimelineVertex>,
    pub tri_vertices: Vec<TimelineVertex>,
    pub tri_indices: Vec<u32>,

    pub scissor: Option<(u32, u32, u32, u32)>,
}

impl TimelineGeometry {
    pub fn build(
        anim: &AnimInstance,
        rows: &[TimelineRow],
        current_frame: f32,
        visible_start: f32,
        visible_span: f32,
        rect_size: (f32, f32, f32, f32),
    ) -> Self {
        let mut geo = Self::default();
        let rect_x = rect_size.0;
        let rect_y = rect_size.1;
        let rect_w = rect_size.2;
        let rect_h = rect_size.3;

        if rect_w <= 1.0 || rect_h <= 1.0 {
            return geo;
        }

        geo.scissor = Some((
            rect_x.max(0.0) as u32,
            rect_y.max(0.0) as u32,
            rect_w as u32,
            rect_h as u32,
        ));

        let layout = TimelineLayout::new(
            anim,
            rows,
            visible_start,
            visible_span,
            rect_x,
            rect_y,
            rect_w,
            rect_h,
        );

        let ruler_h = TIMELINE_RULER_HEIGHT.min(rect_h);
        geo.draw_ruler(rect_x, rect_y, rect_w, ruler_h, &layout);

        for (row_idx, row) in rows.iter().enumerate() {
            let row_top =
                rect_y + TIMELINE_RULER_HEIGHT + row_idx as f32 * TimelineRow::total_height();
            if row_top >= rect_y + rect_h {
                break;
            }

            let row_bottom = (row_top + TimelineRow::HEIGHT).min(rect_y + rect_h);

            match row {
                TimelineRow::PaneHeader { .. } => {
                    push_quad(
                        &mut geo.tri_vertices,
                        &mut geo.tri_indices,
                        [rect_x, row_top],
                        [rect_x + rect_w, row_bottom],
                        Color4f {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.09,
                        }
                        .into(),
                    );
                }
                TimelineRow::Track(track) => {
                    let bg = if row_idx % 2 == 0 {
                        Color4f {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.03,
                        }
                        .into()
                    } else {
                        Color4f {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.06,
                        }
                        .into()
                    };

                    push_quad(
                        &mut geo.tri_vertices,
                        &mut geo.tri_indices,
                        [rect_x, row_top],
                        [rect_x + rect_w, row_bottom],
                        bg,
                    );

                    if let Some(curve) = anim.curve(track) {
                        geo.draw_curve(curve, row_idx, &layout);
                    }
                }
            }
        }

        let px = layout.frame_to_x(current_frame);
        if px >= rect_x - 1.0 && px <= rect_x + rect_w + 1.0 {
            push_quad(
                &mut geo.tri_vertices,
                &mut geo.tri_indices,
                [px - 1.0, rect_y],
                [px + 1.0, rect_y + rect_h],
                Color4f {
                    r: 1.0,
                    g: 0.3,
                    b: 0.35,
                    a: 0.85,
                }
                .into(),
            );
        }

        geo
    }

    fn draw_ruler(
        &mut self,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        ruler_h: f32,
        layout: &TimelineLayout,
    ) {
        let px_per_frame = rect_w / layout.visible_span;
        let tick_step = if px_per_frame >= 8.0 {
            1
        } else {
            (8.0 / px_per_frame.max(0.001)).ceil() as i32
        };

        let major_every = (tick_step * 5).max(1);

        let first_frame = (layout.visible_start / tick_step as f32).floor() as i32 * tick_step;
        let last_frame = (layout.visible_start + layout.visible_span).ceil() as i32;

        let mut frame = first_frame.max(0);
        while frame <= last_frame {
            let x = layout.frame_to_x(frame as f32);

            if x >= rect_x && x <= rect_x + rect_w {
                let is_major = frame % major_every == 0;
                let y0 = rect_y + ruler_h - if is_major { 12.0 } else { 6.0 };

                push_line(
                    &mut self.lines,
                    [x, y0],
                    [x, rect_y + ruler_h],
                    Color4f {
                        r: 0.75,
                        g: 0.75,
                        b: 0.8,
                        a: if is_major { 0.55 } else { 0.3 },
                    }
                    .into(),
                );
            }

            frame += tick_step;
        }

        push_line(
            &mut self.lines,
            [rect_x, rect_y + ruler_h],
            [rect_x + rect_w, rect_y + ruler_h],
            Color4f {
                r: 0.75,
                g: 0.75,
                b: 0.8,
                a: 0.4,
            }
            .into(),
        );
    }

    fn draw_curve(&mut self, curve: &Curve, row: usize, layout: &TimelineLayout) {
        let line_color = Color4f {
            r: 0.4,
            g: 0.85,
            b: 0.95,
            a: 1.0,
        }
        .into();
        let marker_color = Color4f {
            r: 1.0,
            g: 0.75,
            b: 0.25,
            a: 1.0,
        }
        .into();

        let samples = ((layout.rect_w * 0.5) as usize).clamp(2, 2000);
        let mut prev: Option<[f32; 2]> = None;

        for i in 0..=samples {
            let frame = layout.visible_start + layout.visible_span * (i as f32 / samples as f32);
            let value = eval_curve(curve, frame);
            let point = [layout.frame_to_x(frame), layout.value_to_y(row, value)];

            if let Some(p) = prev {
                push_line(&mut self.lines, p, point, line_color);
            }

            prev = Some(point);
        }

        let key_points: Vec<(f32, f32)> = match curve {
            Curve::Constant(keys) => keys
                .iter()
                .enumerate()
                .map(|(i, v)| (i as f32, *v))
                .collect(),
            Curve::Step(keys) => keys.iter().map(|k| (k.frame, k.value as f32)).collect(),
            Curve::Hermite(keys) => keys.iter().map(|k| (k.frame, k.value)).collect(),
        };

        for (frame, value) in key_points {
            let center = [layout.frame_to_x(frame), layout.value_to_y(row, value)];

            push_diamond(
                &mut self.tri_vertices,
                &mut self.tri_indices,
                center,
                TIMELINE_MARKER_RADIUS,
                marker_color,
            );
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    proj: [[f32; 4]; 4],
}

pub struct TimelineRenderer {
    line_pipeline: wgpu::RenderPipeline,
    tri_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,

    line_vertex_buffer: Option<wgpu::Buffer>,
    line_vertex_count: u32,

    tri_vertex_buffer: Option<wgpu::Buffer>,
    tri_index_buffer: Option<wgpu::Buffer>,
    tri_index_count: u32,

    scissor: Option<(u32, u32, u32, u32)>,
}

impl TimelineRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("timeline_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/timeline.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("timeline_ub"),
            contents: bytemuck::bytes_of(&Uniforms {
                proj: identity_matrix(),
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("timeline_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("timeline_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("timeline_layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TimelineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x2,
                1 => Float32x4,
            ],
        };

        let make_pipeline = |label: &str, topology: wgpu::PrimitiveTopology| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[vertex_layout.clone()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let line_pipeline = make_pipeline("timeline_lines", wgpu::PrimitiveTopology::LineList);
        let tri_pipeline = make_pipeline("timeline_tris", wgpu::PrimitiveTopology::TriangleList);

        Self {
            line_pipeline,
            tri_pipeline,
            uniform_buffer,
            bind_group,
            line_vertex_buffer: None,
            line_vertex_count: 0,
            tri_vertex_buffer: None,
            tri_index_buffer: None,
            tri_index_count: 0,
            scissor: None,
        }
    }

    pub fn update_projection(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let proj = [
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, -2.0 / height, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms { proj }),
        );
    }

    pub fn upload(&mut self, device: &wgpu::Device, geometry: &TimelineGeometry) {
        self.scissor = geometry.scissor;

        if geometry.lines.is_empty() {
            self.line_vertex_buffer = None;
            self.line_vertex_count = 0;
        } else {
            self.line_vertex_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("timeline_line_vb"),
                    contents: bytemuck::cast_slice(&geometry.lines),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));

            self.line_vertex_count = geometry.lines.len() as u32;
        }

        if geometry.tri_vertices.is_empty() || geometry.tri_indices.is_empty() {
            self.tri_vertex_buffer = None;
            self.tri_index_buffer = None;
            self.tri_index_count = 0;
        } else {
            self.tri_vertex_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("timeline_tri_vb"),
                    contents: bytemuck::cast_slice(&geometry.tri_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));

            self.tri_index_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("timeline_tri_ib"),
                    contents: bytemuck::cast_slice(&geometry.tri_indices),
                    usage: wgpu::BufferUsages::INDEX,
                },
            ));

            self.tri_index_count = geometry.tri_indices.len() as u32;
        }
    }

    pub fn render<'rpass>(&self, rpass: &mut wgpu::RenderPass<'rpass>) {
        let Some((sx, sy, sw, sh)) = self.scissor else {
            return;
        };

        if sw == 0 || sh == 0 {
            return;
        }

        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_scissor_rect(sx, sy, sw, sh);

        if let (Some(vb), Some(ib)) = (&self.tri_vertex_buffer, &self.tri_index_buffer) {
            rpass.set_pipeline(&self.tri_pipeline);
            rpass.set_vertex_buffer(0, vb.slice(..));
            rpass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..self.tri_index_count, 0, 0..1);
        }

        if let Some(vb) = &self.line_vertex_buffer {
            rpass.set_pipeline(&self.line_pipeline);
            rpass.set_vertex_buffer(0, vb.slice(..));
            rpass.draw(0..self.line_vertex_count, 0..1);
        }
    }
}

fn identity_matrix() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub struct TimelineLayout {
    pub rect_x: f32,
    pub rect_y: f32,
    pub rect_w: f32,
    pub rect_h: f32,

    pub visible_start: f32,
    pub visible_span: f32,

    pub rows: Vec<(f32, f32, bool, f32, f32)>,
}

impl TimelineLayout {
    pub fn new(
        anim: &AnimInstance,
        rows: &[TimelineRow],
        visible_start: f32,
        visible_span: f32,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        rect_h: f32,
    ) -> Self {
        let rows_top = rect_y + TIMELINE_RULER_HEIGHT;

        let row_geoms = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let top = rows_top + i as f32 * TimelineRow::total_height();
                let bottom = (top + TimelineRow::HEIGHT).min(rect_y + rect_h);

                match row {
                    TimelineRow::PaneHeader { .. } => (top, bottom, false, 0.0, 1.0),
                    TimelineRow::Track(track) => {
                        let (min_v, max_v) = anim
                            .curve(track)
                            .map(curve_value_range)
                            .unwrap_or((0.0, 1.0));
                        (top, bottom, true, min_v, max_v)
                    }
                }
            })
            .collect();

        Self {
            rect_x,
            rect_y,
            rect_w,
            rect_h,
            visible_start,
            visible_span: visible_span.max(0.001),
            rows: row_geoms,
        }
    }

    pub fn frame_to_x(&self, frame: f32) -> f32 {
        self.rect_x + ((frame - self.visible_start) / self.visible_span) * self.rect_w
    }

    pub fn x_to_frame(&self, x: f32) -> f32 {
        self.visible_start + (x - self.rect_x) / self.rect_w * self.visible_span
    }

    pub fn value_to_y(&self, row: usize, value: f32) -> f32 {
        let Some(&(top, bottom, _, min_v, max_v)) = self.rows.get(row) else {
            return self.rect_y;
        };

        if (max_v - min_v).abs() < f32::EPSILON {
            top + TimelineRow::HEIGHT * 0.5
        } else {
            let t = (value - min_v) / (max_v - min_v);
            (bottom - t * TimelineRow::HEIGHT).clamp(top, bottom)
        }
    }

    pub fn y_to_value(&self, row: usize, y: f32) -> f32 {
        let Some(&(top, bottom, _, min_v, max_v)) = self.rows.get(row) else {
            return 0.0;
        };

        let t = ((bottom - y) / (bottom - top).max(1.0)).clamp(0.0, 1.0);
        min_v + t * (max_v - min_v)
    }

    pub fn track_row_at(&self, y: f32) -> Option<usize> {
        self.rows
            .iter()
            .position(|&(top, bottom, is_track, _, _)| is_track && y >= top && y <= bottom)
    }
}

#[derive(Clone, Debug)]
pub struct PendingKeyEdit {
    pub track: TimelineTrack,
    pub key_idx: usize,
    pub frame: f32,
    pub value: f32,
}

#[derive(Clone, Debug)]
pub struct TimelineDrag {
    pub track: TimelineTrack,
    pub key_idx: usize,
    pub row: usize,
}

impl TimelineDrag {
    const HIT_RADIUS: f32 = 9.0;

    pub fn find_nearest_key(
        anim: &AnimInstance,
        rows: &[TimelineRow],
        layout: &TimelineLayout,
        pointer_px: [f32; 2],
    ) -> Option<Self> {
        let row = layout.track_row_at(pointer_px[1])?;

        let (key_idx, row, track) = {
            let TimelineRow::Track(track) = rows.get(row)? else {
                return None;
            };

            let row = layout.track_row_at(pointer_px[1])?;

            let curve = anim.curve(track)?;

            let key_points: Vec<(f32, f32)> = match curve {
                Curve::Constant(keys) => keys
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (i as f32, *v))
                    .collect(),
                Curve::Step(keys) => keys.iter().map(|k| (k.frame, k.value as f32)).collect(),
                Curve::Hermite(keys) => keys.iter().map(|k| (k.frame, k.value)).collect(),
            };

            let mut best: Option<(usize, f32)> = None;
            let hit_radius_sq = Self::HIT_RADIUS * Self::HIT_RADIUS;

            for (i, &(frame, value)) in key_points.iter().enumerate() {
                let x = layout.frame_to_x(frame);
                let y = layout.value_to_y(row, value);
                let dist2 = (x - pointer_px[0]).powi(2) + (y - pointer_px[1]).powi(2);

                if dist2 <= hit_radius_sq && best.is_none_or(|(_, best_dist2)| dist2 < best_dist2) {
                    best = Some((i, dist2));
                }
            }

            best.map(|(key_idx, _)| (key_idx, row, track))
        }?;

        Some(Self {
            track: track.clone(),
            key_idx,
            row,
        })
    }
}
#[derive(Clone, Debug)]
pub struct TimelineTrack {
    pub label: String,
    pub content_idx: usize,
    pub info_idx: usize,
    pub target_idx: usize,
}

#[derive(Clone, Debug)]
pub enum TimelineRow {
    PaneHeader { content_idx: usize, label: String },
    Track(TimelineTrack),
}

impl TimelineRow {
    pub const HEIGHT: f32 = 26.0;
    pub const GAP: f32 = 8.0;

    pub fn build(anim: &AnimInstance, expanded: &HashSet<usize>) -> Vec<Self> {
        let mut out = Vec::new();

        for (content_idx, content) in anim.bflan.anim_info.contents.iter().enumerate() {
            let target_count: usize = content
                .infos
                .iter()
                .map(|info| match info {
                    AnimInfo::Standard { targets, .. } => targets.len(),
                    _ => 0,
                })
                .sum();

            if target_count == 0 {
                continue;
            }

            out.push(Self::PaneHeader {
                content_idx,
                label: content.name.trim_end_matches('\0').to_string(),
            });

            if !expanded.contains(&content_idx) {
                continue;
            }

            for (info_idx, info) in content.infos.iter().enumerate() {
                if let AnimInfo::Standard { targets, .. } = info {
                    for (target_idx, target) in targets.iter().enumerate() {
                        out.push(Self::Track(TimelineTrack {
                            label: format!("{:?}", target.target),
                            content_idx,
                            info_idx,
                            target_idx,
                        }));
                    }
                }
            }
        }

        out
    }

    pub const fn total_height() -> f32 {
        Self::HEIGHT + Self::GAP
    }
}

fn curve_value_range(curve: &Curve) -> (f32, f32) {
    let values: Vec<f32> = match curve {
        Curve::Constant(keys) => keys.clone(),
        Curve::Step(keys) => keys.iter().map(|k| k.value as f32).collect(),
        Curve::Hermite(keys) => keys.iter().map(|k| k.value).collect(),
    };

    let Some(&first) = values.first() else {
        return (0.0, 1.0);
    };

    let (min_v, max_v) = values
        .iter()
        .fold((first, first), |(lo, hi), &v| (lo.min(v), hi.max(v)));

    if (max_v - min_v).abs() < f32::EPSILON {
        (min_v - 1.0, max_v + 1.0)
    } else {
        let pad = (max_v - min_v) * 0.1;
        (min_v - pad, max_v + pad)
    }
}

fn push_line(lines: &mut Vec<TimelineVertex>, a: [f32; 2], b: [f32; 2], color: [f32; 4]) {
    lines.push(TimelineVertex { position: a, color });
    lines.push(TimelineVertex { position: b, color });
}

fn push_quad(
    verts: &mut Vec<TimelineVertex>,
    indices: &mut Vec<u32>,
    top_left: [f32; 2],
    bottom_right: [f32; 2],
    color: [f32; 4],
) {
    let base = verts.len() as u32;
    verts.push(TimelineVertex {
        position: [top_left[0], top_left[1]],
        color,
    });

    verts.push(TimelineVertex {
        position: [bottom_right[0], top_left[1]],
        color,
    });

    verts.push(TimelineVertex {
        position: [bottom_right[0], bottom_right[1]],
        color,
    });

    verts.push(TimelineVertex {
        position: [top_left[0], bottom_right[1]],
        color,
    });

    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn push_diamond(
    verts: &mut Vec<TimelineVertex>,
    indices: &mut Vec<u32>,
    center: [f32; 2],
    r: f32,
    color: [f32; 4],
) {
    let base = verts.len() as u32;
    verts.push(TimelineVertex {
        position: [center[0], center[1] - r],
        color,
    });

    verts.push(TimelineVertex {
        position: [center[0] + r, center[1]],
        color,
    });

    verts.push(TimelineVertex {
        position: [center[0], center[1] + r],
        color,
    });

    verts.push(TimelineVertex {
        position: [center[0] - r, center[1]],
        color,
    });

    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}
