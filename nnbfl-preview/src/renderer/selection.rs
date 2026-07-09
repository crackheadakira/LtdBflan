use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::quad::{RenderPipelineContainer, Uniforms};
use crate::pane_tree::Corners;

const EDGE_THICKNESS: f32 = 2.0;
const HANDLE_HALF: f32 = 5.0;

const OUTLINE_COLOR: [f32; 4] = [0.2, 0.7, 1.0, 1.0];
const HANDLE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const HANDLE_BORDER: [f32; 4] = [0.2, 0.7, 1.0, 1.0];

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct SelectionVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl SelectionVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Handle {
    Body,
    Rotation,

    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,

    Top,
    Bottom,
    Left,
    Right,
}

pub struct SelectionRenderer {
    pipeline: RenderPipelineContainer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    pub handle_positions: Vec<([f32; 2], Handle)>,
}

impl SelectionRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let identity = [
            [1.0f32, 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];

        let pipeline = RenderPipelineContainer::new(
            device,
            "selection",
            include_str!("../shaders/selection.wgsl"),
            &[SelectionVertex::desc()],
            surface_format,
            Uniforms::from_matrix(identity),
            wgpu::ShaderStages::VERTEX,
        );

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selection_vb"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selection_ib"),
            size: 0,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: 0,
            handle_positions: Vec::new(),
        }
    }

    pub fn update_projection(&self, queue: &wgpu::Queue, matrix: [[f32; 4]; 4]) {
        queue.write_buffer(
            &self.pipeline.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms::from_matrix(matrix)),
        );
    }

    pub fn update(&mut self, device: &wgpu::Device, corners: &Corners) {
        let tl = [corners.top_left.x, corners.top_left.y];
        let tr = [corners.top_right.x, corners.top_right.y];
        let bl = [corners.bottom_left.x, corners.bottom_left.y];
        let br = [corners.bottom_right.x, corners.bottom_right.y];

        let mt = midpoint(tl, tr);
        let mb = midpoint(bl, br);
        let ml = midpoint(tl, bl);
        let mr = midpoint(tr, br);

        let dx = mt[0] - mb[0];
        let dy = mt[1] - mb[1];
        let len = (dx * dx + dy * dy).sqrt().max(f32::EPSILON);
        let nx = dx / len;
        let ny = dy / len;

        let rotation_handle_pos = [mt[0] + nx * 25.0, mt[1] + ny * 25.0];

        let mut vertices: Vec<SelectionVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for &(a, b) in &[(tl, tr), (tr, br), (br, bl), (bl, tl)] {
            push_edge(
                &mut vertices,
                &mut indices,
                a,
                b,
                EDGE_THICKNESS,
                OUTLINE_COLOR,
            );
        }

        push_edge(
            &mut vertices,
            &mut indices,
            mt,
            rotation_handle_pos,
            1.5,
            OUTLINE_COLOR,
        );

        for &pos in &[tl, tr, bl, br] {
            push_square(&mut vertices, &mut indices, pos, HANDLE_HALF, HANDLE_COLOR);
            push_square_border(
                &mut vertices,
                &mut indices,
                pos,
                HANDLE_HALF,
                1.5,
                HANDLE_BORDER,
            );
        }

        for &pos in &[mt, mb, ml, mr] {
            push_square(
                &mut vertices,
                &mut indices,
                pos,
                HANDLE_HALF * 0.75,
                HANDLE_COLOR,
            );

            push_square_border(
                &mut vertices,
                &mut indices,
                pos,
                HANDLE_HALF * 0.75,
                1.5,
                HANDLE_BORDER,
            );
        }

        push_square(
            &mut vertices,
            &mut indices,
            rotation_handle_pos,
            HANDLE_HALF,
            HANDLE_COLOR,
        );
        push_square_border(
            &mut vertices,
            &mut indices,
            rotation_handle_pos,
            HANDLE_HALF,
            1.5,
            HANDLE_BORDER,
        );

        self.handle_positions = vec![
            (tl, Handle::TopLeft),
            (tr, Handle::TopRight),
            (bl, Handle::BottomLeft),
            (br, Handle::BottomRight),
            (mt, Handle::Top),
            (mb, Handle::Bottom),
            (ml, Handle::Left),
            (mr, Handle::Right),
            (rotation_handle_pos, Handle::Rotation),
        ];

        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("selection_vb"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("selection_ib"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.num_indices = indices.len() as u32;
    }

    pub fn clear(&mut self) {
        self.num_indices = 0;
        self.handle_positions.clear();
    }

    pub fn render<'rpass>(&'rpass self, rpass: &mut wgpu::RenderPass<'rpass>) {
        if self.num_indices == 0 {
            return;
        }

        rpass.set_pipeline(&self.pipeline.pipeline);
        rpass.set_bind_group(0, &self.pipeline.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..self.num_indices, 0, 0..1);
    }

    pub fn hit_test(&self, world_pos: [f32; 2], radius: f32) -> Option<Handle> {
        let r2 = radius * radius;
        self.handle_positions
            .iter()
            .min_by(|(a, _), (b, _)| {
                dist2(*a, world_pos)
                    .partial_cmp(&dist2(*b, world_pos))
                    .unwrap()
            })
            .filter(|(pos, _)| dist2(*pos, world_pos) <= r2)
            .map(|(_, h)| *h)
    }
}

fn midpoint(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]
}

fn dist2(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

fn push_edge(
    verts: &mut Vec<SelectionVertex>,
    idxs: &mut Vec<u32>,
    a: [f32; 2],
    b: [f32; 2],
    thickness: f32,
    color: [f32; 4],
) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = (dx * dx + dy * dy).sqrt().max(f32::EPSILON);

    let nx = -dy / len * thickness * 0.5;
    let ny = dx / len * thickness * 0.5;

    let base = verts.len() as u32;
    for &[px, py] in &[
        [a[0] - nx, a[1] - ny],
        [b[0] - nx, b[1] - ny],
        [a[0] + nx, a[1] + ny],
        [b[0] + nx, b[1] + ny],
    ] {
        verts.push(SelectionVertex {
            position: [px, py],
            color,
        });
    }
    idxs.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
}

fn push_square(
    verts: &mut Vec<SelectionVertex>,
    idxs: &mut Vec<u32>,
    pos: [f32; 2],
    half: f32,
    color: [f32; 4],
) {
    let base = verts.len() as u32;
    let [cx, cy] = pos;
    for &[px, py] in &[
        [cx - half, cy - half],
        [cx + half, cy - half],
        [cx - half, cy + half],
        [cx + half, cy + half],
    ] {
        verts.push(SelectionVertex {
            position: [px, py],
            color,
        });
    }

    idxs.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
}

fn push_square_border(
    verts: &mut Vec<SelectionVertex>,
    idxs: &mut Vec<u32>,
    pos: [f32; 2],
    half: f32,
    thickness: f32,
    color: [f32; 4],
) {
    let [cx, cy] = pos;
    let tl = [cx - half, cy - half];
    let tr = [cx + half, cy - half];
    let bl = [cx - half, cy + half];
    let br = [cx + half, cy + half];

    for &(a, b) in &[(tl, tr), (tr, br), (br, bl), (bl, tl)] {
        push_edge(verts, idxs, a, b, thickness, color);
    }
}

pub fn point_in_quad(p: [f32; 2], corners: &Corners) -> bool {
    let pts = [
        [corners.top_left.x, corners.top_left.y],
        [corners.top_right.x, corners.top_right.y],
        [corners.bottom_right.x, corners.bottom_right.y],
        [corners.bottom_left.x, corners.bottom_left.y],
    ];

    let cross = |a: [f32; 2], b: [f32; 2], p: [f32; 2]| -> f32 {
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
    };

    let mut sign = 0.0f32;
    for i in 0..4 {
        let a = pts[i];
        let b = pts[(i + 1) % 4];
        let c = cross(a, b, p);
        if c != 0.0 {
            if sign == 0.0 {
                sign = c.signum();
            } else if c.signum() != sign {
                return false;
            }
        }
    }
    true
}

pub fn point_hits_textured_quad(
    p: [f32; 2],
    tq: &crate::renderer::textured_quad::TexturedQuad,
) -> bool {
    use wgpu::AddressMode;

    let clamp_u = tq.sampler_0.address_mode_u == AddressMode::ClampToEdge;
    let clamp_v = tq.sampler_0.address_mode_v == AddressMode::ClampToEdge;

    if !clamp_u && !clamp_v {
        return true;
    }

    let c = tq.corners;
    let uv = |tri: [[f32; 2]; 3], uv_tri: [[f32; 2]; 3]| -> Option<[f32; 2]> {
        let (a, b, c_) = (tri[0], tri[1], tri[2]);
        let v0 = [b[0] - a[0], b[1] - a[1]];
        let v1 = [c_[0] - a[0], c_[1] - a[1]];
        let v2 = [p[0] - a[0], p[1] - a[1]];
        let d00 = v0[0] * v0[0] + v0[1] * v0[1];
        let d01 = v0[0] * v1[0] + v0[1] * v1[1];
        let d11 = v1[0] * v1[0] + v1[1] * v1[1];
        let d20 = v2[0] * v0[0] + v2[1] * v0[1];
        let d21 = v2[0] * v1[0] + v2[1] * v1[1];
        let denom = d00 * d11 - d01 * d01;
        if denom.abs() < 1e-8 {
            return None;
        }
        let s = (d11 * d20 - d01 * d21) / denom;
        let t = (d00 * d21 - d01 * d20) / denom;
        if s < -0.001 || t < -0.001 || s + t > 1.001 {
            return None;
        }

        let u =
            uv_tri[0][0] + s * (uv_tri[1][0] - uv_tri[0][0]) + t * (uv_tri[2][0] - uv_tri[0][0]);
        let v =
            uv_tri[0][1] + s * (uv_tri[1][1] - uv_tri[0][1]) + t * (uv_tri[2][1] - uv_tri[0][1]);
        Some([u, v])
    };

    let uv_tl = tq.uvs[0][0];
    let uv_tr = tq.uvs[1][0];
    let uv_bl = tq.uvs[2][0];
    let uv_br = tq.uvs[3][0];

    let hit_uv = uv([c[0], c[1], c[2]], [uv_tl, uv_tr, uv_bl])
        .or_else(|| uv([c[1], c[3], c[2]], [uv_tr, uv_br, uv_bl]));

    let Some([u, v]) = hit_uv else { return true };

    let u_ok = !clamp_u || (0.0..=1.0).contains(&u);
    let v_ok = !clamp_v || (0.0..=1.0).contains(&v);

    u_ok && v_ok
}
