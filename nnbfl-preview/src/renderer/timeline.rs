use bytemuck::{Pod, Zeroable};
use nnbfl::bflan::{anim_info::AnimInfo, curves::Curve};
use wgpu::util::DeviceExt;

use crate::anim_state::{AnimInstance, eval_curve};

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

const TIMELINE_RULER_HEIGHT: f32 = 22.0;
const TIMELINE_ROW_HEIGHT: f32 = 26.0;
const TIMELINE_ROW_GAP: f32 = 2.0;
const TIMELINE_MARKER_RADIUS: f32 = 4.0;

pub struct TimelineTrack {
    pub label: String,
    pub curve: Curve,
}

pub fn timeline_track_row_height() -> f32 {
    TIMELINE_ROW_HEIGHT + TIMELINE_ROW_GAP
}

pub fn collect_tracks_for_pane(anim: &AnimInstance, pane_label: &str) -> Vec<TimelineTrack> {
    let target_name = pane_label.trim_end_matches('\0');
    let mut out = Vec::new();

    for content in &anim.bflan.anim_info.contents {
        if content.name.trim_end_matches('\0') != target_name {
            continue;
        }

        for info in &content.infos {
            if let AnimInfo::Standard { targets, .. } = info {
                for target in targets {
                    out.push(TimelineTrack {
                        label: format!("{:?}", target.target),
                        curve: target.curve.clone(),
                    });
                }
            }
        }
    }

    out
}

pub fn build_timeline_geometry(
    frame_count: f32,
    current_frame: f32,
    tracks: &[TimelineTrack],
    rect_x: f32,
    rect_y: f32,
    rect_w: f32,
    rect_h: f32,
) -> TimelineGeometry {
    let mut geo = TimelineGeometry::default();

    if rect_w <= 1.0 || rect_h <= 1.0 || frame_count <= 0.0 {
        return geo;
    }

    geo.scissor = Some((
        rect_x.max(0.0) as u32,
        rect_y.max(0.0) as u32,
        rect_w as u32,
        rect_h as u32,
    ));

    let frame_to_x = |frame: f32| -> f32 { rect_x + (frame / frame_count) * rect_w };

    let ruler_h = TIMELINE_RULER_HEIGHT.min(rect_h);
    draw_ruler(
        &mut geo,
        frame_count,
        rect_x,
        rect_y,
        rect_w,
        ruler_h,
        &frame_to_x,
    );

    let rows_top = rect_y + ruler_h;
    for (row_idx, track) in tracks.iter().enumerate() {
        let row_top = rows_top + row_idx as f32 * timeline_track_row_height();
        if row_top >= rect_y + rect_h {
            break;
        }
        let row_bottom = (row_top + TIMELINE_ROW_HEIGHT).min(rect_y + rect_h);

        let bg = if row_idx % 2 == 0 {
            rgba(1.0, 1.0, 1.0, 0.03)
        } else {
            rgba(1.0, 1.0, 1.0, 0.06)
        };
        push_quad(
            &mut geo.tri_vertices,
            &mut geo.tri_indices,
            [rect_x, row_top],
            [rect_x + rect_w, row_bottom],
            bg,
        );

        let (min_v, max_v) = curve_value_range(&track.curve);
        let value_to_y = |value: f32| -> f32 {
            if (max_v - min_v).abs() < f32::EPSILON {
                row_top + TIMELINE_ROW_HEIGHT * 0.5
            } else {
                let t = (value - min_v) / (max_v - min_v);
                (row_bottom - t * TIMELINE_ROW_HEIGHT).clamp(row_top, row_bottom)
            }
        };

        draw_curve(
            &mut geo,
            &track.curve,
            frame_count,
            &frame_to_x,
            &value_to_y,
        );
    }

    let px = frame_to_x(current_frame.clamp(0.0, frame_count));
    push_quad(
        &mut geo.tri_vertices,
        &mut geo.tri_indices,
        [px - 1.0, rect_y],
        [px + 1.0, rect_y + rect_h],
        rgba(1.0, 0.3, 0.35, 0.85),
    );

    geo
}

fn draw_ruler(
    geo: &mut TimelineGeometry,
    frame_count: f32,
    rect_x: f32,
    rect_y: f32,
    rect_w: f32,
    ruler_h: f32,
    frame_to_x: &impl Fn(f32) -> f32,
) {
    let px_per_frame = rect_w / frame_count.max(1.0);
    let tick_step = if px_per_frame >= 8.0 {
        1
    } else {
        (8.0 / px_per_frame.max(0.001)).ceil() as i32
    };
    let major_every = (tick_step * 5).max(1);

    let mut frame = 0i32;
    while (frame as f32) <= frame_count {
        let x = frame_to_x(frame as f32);
        let is_major = frame % major_every == 0;
        let y0 = rect_y + ruler_h - if is_major { 12.0 } else { 6.0 };

        push_line(
            &mut geo.lines,
            [x, y0],
            [x, rect_y + ruler_h],
            rgba(0.75, 0.75, 0.8, if is_major { 0.55 } else { 0.3 }),
        );

        frame += tick_step;
    }

    push_line(
        &mut geo.lines,
        [rect_x, rect_y + ruler_h],
        [rect_x + rect_w, rect_y + ruler_h],
        rgba(0.75, 0.75, 0.8, 0.4),
    );
}

fn draw_curve(
    geo: &mut TimelineGeometry,
    curve: &Curve,
    frame_count: f32,
    frame_to_x: &impl Fn(f32) -> f32,
    value_to_y: &impl Fn(f32) -> f32,
) {
    let line_color = rgba(0.4, 0.85, 0.95, 1.0);
    let marker_color = rgba(1.0, 0.75, 0.25, 1.0);

    let samples = ((frame_count.max(1.0) * 2.0) as usize).clamp(2, 800);
    let mut prev: Option<[f32; 2]> = None;

    for i in 0..=samples {
        let frame = frame_count * (i as f32 / samples as f32);
        let value = eval_curve(curve, frame);
        let point = [frame_to_x(frame), value_to_y(value)];

        if let Some(p) = prev {
            push_line(&mut geo.lines, p, point, line_color);
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
        let center = [frame_to_x(frame), value_to_y(value)];
        push_diamond(
            &mut geo.tri_vertices,
            &mut geo.tri_indices,
            center,
            TIMELINE_MARKER_RADIUS,
            marker_color,
        );
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

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r, g, b, a]
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
