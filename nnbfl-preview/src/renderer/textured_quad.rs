use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use nnbfl::bflyt::flags::{TexFilter, TexWrapMode};
use nnbfl::bflyt::list::{
    CombinerTevMode, Material, MaterialTextureMap, MaterialTextureSrt, TexGenSrc,
};
use nnbfl::bflyt::pane::{Pane, TextureUv};
use nnbfl::ui2d::types::{Color4u8, Vector2f, Vector3f};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, TextureView};

use super::quad::Quad;
use super::texture::TextureCache;
use crate::pane_tree::PaneNode;
use crate::renderer::quad::Uniforms;
use crate::ui::general::PaneVisibilityFlags;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv0: [f32; 2],
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    pub tint: [f32; 4],
    pub quad_size: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x2, // uv0
        2 => Float32x2, // uv1
        3 => Float32x2, // uv2
        4 => Float32x4, // tint
        5 => Float32x2, // quad_size
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const PLAIN_UVS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
pub struct StandardMaterial {
    pub black_color: u32,
    pub white_color: u32,

    pub combine_mode: u32,
    pub combine_mode2: u32,

    pub texture_count: u32,
    pub tex_gen_mode: u32,
    pub visible: u32,
    pub packed_alpha_flags: u32,

    pub debug_stage: u32,

    pub is_plain: u32,
    pub _padding: [f32; 2],

    pub indirect_mtx0: [f32; 4],
    pub indirect_mtx1: [f32; 4],

    pub proj_mtx0: [[f32; 4]; 2],
    pub proj_mtx1: [[f32; 4]; 2],
    pub proj_mtx2: [[f32; 4]; 2],
}

impl Default for StandardMaterial {
    fn default() -> Self {
        Self {
            black_color: 0,
            white_color: u32::MAX,
            combine_mode: 0,
            combine_mode2: 0,
            texture_count: 0,
            tex_gen_mode: 0,
            visible: 1,
            packed_alpha_flags: 0,
            debug_stage: 0,
            is_plain: 0,
            _padding: [0.0; 2],
            indirect_mtx0: [0.0; 4],
            indirect_mtx1: [0.0; 4],
            proj_mtx0: [[1.0, 0.0, 0.0, 0.5], [0.0, 1.0, 0.0, 0.5]],
            proj_mtx1: [[1.0, 0.0, 0.0, 0.5], [0.0, 1.0, 0.0, 0.5]],
            proj_mtx2: [[1.0, 0.0, 0.0, 0.5], [0.0, 1.0, 0.0, 0.5]],
        }
    }
}

impl StandardMaterial {
    pub fn plain() -> Self {
        Self {
            is_plain: 1,
            texture_count: 0,
            visible: 1,
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
pub struct DetailedCombinerMaterial {
    pub constant_colors: [[f32; 4]; 7],

    pub stage_count: u32,
    pub _pad0: [u32; 3],

    pub stage_bits: [[i32; 4]; 6],

    pub texture_count: u32,
    pub _pad1: [u32; 3],
}

impl Default for DetailedCombinerMaterial {
    fn default() -> Self {
        Self {
            constant_colors: [[0.0; 4]; 7],
            stage_count: 0,
            _pad0: [0; 3],
            stage_bits: [[0; 4]; 6],
            texture_count: 1,
            _pad1: [0; 3],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub struct WgpuSamplerSettings {
    pub address_mode_u: wgpu::AddressMode,
    pub address_mode_v: wgpu::AddressMode,
    pub min_filter: wgpu::FilterMode,
    pub mag_filter: wgpu::FilterMode,
}

impl WgpuSamplerSettings {
    pub fn from_tex_map(map: Option<&MaterialTextureMap>) -> Self {
        match map {
            Some(m) => Self {
                address_mode_u: Self::wrap_to_wgpu(&m.u_options.wrap_mode),
                address_mode_v: Self::wrap_to_wgpu(&m.v_options.wrap_mode),
                min_filter: Self::filter_to_wgpu(&m.u_options.filter),
                mag_filter: Self::filter_to_wgpu(&m.v_options.filter),
            },
            None => Self {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
            },
        }
    }

    fn wrap_to_wgpu(w: &TexWrapMode) -> wgpu::AddressMode {
        match w {
            TexWrapMode::Repeat => wgpu::AddressMode::Repeat,
            TexWrapMode::Mirror => wgpu::AddressMode::MirrorRepeat,
            TexWrapMode::Clamp => wgpu::AddressMode::ClampToEdge,
        }
    }

    fn filter_to_wgpu(f: &TexFilter) -> wgpu::FilterMode {
        match f {
            TexFilter::Linear => wgpu::FilterMode::Linear,
            TexFilter::Near => wgpu::FilterMode::Nearest,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TexturedQuad {
    pub pane_idx: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// World-space corner positions [TL, TR, BL, BR] after rotation.
    pub corners: [[f32; 2]; 4],
    pub rotation: Vector3f,

    pub uvs: [[[f32; 2]; 3]; 4],
    pub base_uvs: [[[f32; 2]; 3]; 4],
    pub tex_srts: Vec<MaterialTextureSrt>,
    pub tint: [f32; 4],
    pub corner_tints: [[f32; 4]; 4],
    pub texture_name: String,
    pub texture_name1: Option<String>,
    pub texture_name2: Option<String>,

    pub sampler_0: WgpuSamplerSettings,
    pub sampler_1: WgpuSamplerSettings,
    pub sampler_2: WgpuSamplerSettings,

    pub material_idx: u16,
    pub piece_id: usize,

    pub is_detailed: bool,
    pub standard_material: StandardMaterial,
    pub detailed_combiner_material: DetailedCombinerMaterial,

    pub proj_scales: [[f32; 2]; 3],
    pub proj_translations: [[f32; 2]; 3],

    pub indirect_rotation: f32,
    pub indirect_scale: Vector2f,
}

pub fn build_indirect_matrices(rotation_deg: f32, scale: Vector2f) -> ([f32; 4], [f32; 4]) {
    puffin::profile_function!();
    let rad = rotation_deg.to_radians();
    let cos_r = rad.cos();
    let sin_r = rad.sin();

    // TODO: fix still

    let a0x = -cos_r * scale.x;
    let a1x = sin_r * scale.y;
    let a0y = sin_r * scale.x;
    let a1y = cos_r * scale.y;

    let tx = (a0x * -0.5) + (a1x * -0.5);
    let ty = (a0y * -0.5) + (a1y * -0.5);

    ([a0x, a1x, 0.0, tx], [a0y, a1y, 0.0, ty])
}

pub fn vertex_corners_color_to_corner_tints(
    top_left_vertex_color: &Color4u8,
    top_right_vertex_color: &Color4u8,
    bottom_left_vertex_color: &Color4u8,
    bottom_right_vertex_color: &Color4u8,
) -> [[f32; 4]; 4] {
    let to_f32_rgba = |color: &Color4u8| -> [f32; 4] {
        let rgba: [f32; 4] = (*color).into();
        if rgba[3] > 0.0 {
            rgba
        } else {
            [1.0, 1.0, 1.0, 1.0]
        }
    };

    [
        to_f32_rgba(top_left_vertex_color),
        to_f32_rgba(top_right_vertex_color),
        to_f32_rgba(bottom_left_vertex_color),
        to_f32_rgba(bottom_right_vertex_color),
    ]
}

pub struct MaterialPaneData<'a> {
    pub base_section: &'a Pane,
    pub corner_tints: [[f32; 4]; 4],
    pub material_idx: u16,
    pub piece_id: usize,
    pub rotation: Vector3f,
    pub texture_uvs: &'a [TextureUv],
}

pub struct GlyphQuadOverride<'a> {
    pub texture_name: &'a str,

    /// Per-corner UVs into the atlas, ordered [TL, TR, BL, BR].
    pub uvs: [[f32; 2]; 4],
}

impl TexturedQuad {
    pub fn derive_from_material(
        pane_data: MaterialPaneData,
        mat: &Material,
        position: Vector2f,
        size: Vector2f,
        corners: [[f32; 2]; 4],
        is_visible: bool,
        pane_idx: usize,
        glyph_override: Option<GlyphQuadOverride<'_>>,
    ) -> Option<Self> {
        puffin::profile_function!();
        let tex_map = mat.tex_maps.first();
        let tex_name = glyph_override
            .as_ref()
            .map(|g| g.texture_name)
            .unwrap_or_else(|| tex_map.map(|m| m.texture_name.trim_end()).unwrap_or(""));

        if glyph_override.is_none()
            && tex_name.is_empty()
            && mat.blend_mode.is_none()
            && mat.alpha_compare.is_none()
            && mat.tev_combiners.is_empty()
            && mat.detailed_combiner.is_none()
        {
            return None;
        }

        let base_uvs = PaneNode::compute_uvs(pane_data.texture_uvs);
        let mut uvs = PaneNode::apply_srt_to_uvs(base_uvs, &mat.tex_srts);

        if let Some(glyph) = &glyph_override {
            for (idx, corner) in uvs.iter_mut().enumerate() {
                corner[0] = glyph.uvs[idx];
            }
        }

        let texture_count = if glyph_override.is_some() {
            (mat.tex_maps.len().min(3) as u32).max(1)
        } else {
            mat.tex_maps.len().min(3) as u32
        };

        let sampler_0 = WgpuSamplerSettings::from_tex_map(mat.tex_maps.first());
        let sampler_1 = WgpuSamplerSettings::from_tex_map(mat.tex_maps.get(1));
        let sampler_2 = WgpuSamplerSettings::from_tex_map(mat.tex_maps.get(2));

        let get_name = |idx: usize| {
            mat.tex_maps
                .get(idx)
                .map(|m| m.texture_name.trim_end().to_string())
                .filter(|s| !s.is_empty())
        };

        let texture_name1 = get_name(1);
        let texture_name2 = get_name(2);

        let mut tex_gen_flags = [0u32; 3];
        for (flag, coord_gen) in tex_gen_flags
            .iter_mut()
            .zip(mat.tex_coord_gens.iter().take(texture_count as usize))
        {
            let (mode, is_ortho) = match coord_gen.tex_gen_source {
                TexGenSrc::PaneBasedPerspectiveProjection
                | TexGenSrc::PaneBasedOrthogonalProjection => (1, false),
                TexGenSrc::OrthogonalProjection | TexGenSrc::PerspectiveProjection => (1, true),
                TexGenSrc::BrickRepeat => (2, false),
                _ => (0, false),
            };
            *flag = mode;
            if is_ortho {
                *flag |= 1 << 5;
            }
        }

        let mut proj_scales = [[1.0; 2]; 3];
        let mut proj_translations = [[0.0; 2]; 3];
        let mut target_layer = 0;
        for tex_gen in mat.projection_tex_gens.iter().take(texture_count as usize) {
            while target_layer < 3 && (tex_gen_flags[target_layer] & 0x3) != 1 {
                target_layer += 1;
            }
            if target_layer >= 3 {
                break;
            }

            proj_scales[target_layer] = [tex_gen.scale.x, tex_gen.scale.y];
            proj_translations[target_layer] = [tex_gen.translation.x, tex_gen.translation.y];

            if tex_gen.flags.fitting_layout_size {
                tex_gen_flags[target_layer] |= 1 << 2;
            }
            if tex_gen.flags.fitting_pane_size {
                tex_gen_flags[target_layer] |= 1 << 3;
            }
            if tex_gen.flags.adjust_projection_scale_rotate {
                tex_gen_flags[target_layer] |= 1 << 4;
            }
            target_layer += 1;
        }

        let tex_gen_mode_packed =
            tex_gen_flags[0] | (tex_gen_flags[1] << 8) | (tex_gen_flags[2] << 16);

        let color_u8 = |entry: &nnbfl::bflyt::list::MaterialColorEntry| -> [u8; 4] {
            if let Some(c) = &entry.color_u8 {
                (*c).into()
            } else if let Some(c) = &entry.color_f32 {
                (*c).into()
            } else {
                [0; 4]
            }
        };

        let black_u8 = color_u8(&mat.interpolation_colors.black_color);
        let white_u8 = color_u8(&mat.interpolation_colors.white_color);

        let black_packed = black_u8[0] as u32
            | (black_u8[1] as u32) << 8
            | (black_u8[2] as u32) << 16
            | (black_u8[3] as u32) << 24;

        let white_packed = white_u8[0] as u32
            | (white_u8[1] as u32) << 8
            | (white_u8[2] as u32) << 16
            | (white_u8[3] as u32) << 24;

        let is_detailed = mat.detailed_combiner.is_some();
        let mut detailed_combiner_material = DetailedCombinerMaterial::default();
        if let Some(dc) = &mat.detailed_combiner {
            detailed_combiner_material.stage_count = dc.entries.len().min(6) as u32;
            detailed_combiner_material.texture_count = texture_count;
            detailed_combiner_material.constant_colors[0] = dc.color1.into();
            detailed_combiner_material.constant_colors[1] = dc.color2.into();
            detailed_combiner_material.constant_colors[2] = dc.color3.into();
            detailed_combiner_material.constant_colors[3] = dc.color4.into();
            detailed_combiner_material.constant_colors[4] = dc.color5.into();

            for (idx, entry) in dc.entries.iter().enumerate().take(6) {
                let (color_flags, alpha_flags, constant_selectors, _) = entry.pack_flags();
                detailed_combiner_material.stage_bits[idx] = [
                    color_flags as i32,
                    alpha_flags as i32,
                    constant_selectors as i32,
                    1i32,
                ];
            }
        }

        let (combine_mode, combine_mode2) = if let Some(tev0) = mat.tev_combiners.first() {
            let alpha_select_1 = (tev0.alpha_mode == CombinerTevMode::Modulate) as u32;
            let alpha_select_2 = mat
                .tev_combiners
                .get(1)
                .map(|t| (t.alpha_mode == CombinerTevMode::Modulate) as u32)
                .unwrap_or(0);

            let packed_mode1 = (tev0.rgb_mode as u32) | (alpha_select_1 << 24);
            let packed_mode2 = mat
                .tev_combiners
                .get(1)
                .map(|t| t.rgb_mode as u32)
                .unwrap_or(0)
                | (alpha_select_2 << 24);

            (packed_mode1, packed_mode2)
        } else {
            (0, 0)
        };

        let (indirect_rotation, indirect_scale) = if let Some(im) = &mat.indirect_matrix {
            (im.rotation, im.scale)
        } else {
            (0.0, Vector2f::new(0.0, 0.0))
        };

        let (indirect_mtx0, indirect_mtx1) =
            build_indirect_matrices(indirect_rotation, indirect_scale);

        let (alpha_compare, ref_value) = if let Some(alpha_compare) = &mat.alpha_compare {
            (alpha_compare.compare, alpha_compare.alpha_compare_ref_value)
        } else {
            (Default::default(), 0.0)
        };

        let compare_bits = u8::from(alpha_compare) as u32 & 0x7;
        let ref_value_bits = (ref_value.clamp(0.0, 1.0) * 255.0).round() as u32 & 0xFF;

        let tex_only_bit = mat.use_texture_only as u32 & 1;
        let thresh_bit = mat.use_thresholding_alpha_interpolation as u32 & 1;

        let packed_alpha_flags =
            compare_bits | (ref_value_bits << 3) | (tex_only_bit << 11) | (thresh_bit << 12);

        let standard_material = StandardMaterial {
            black_color: black_packed,
            white_color: white_packed,
            combine_mode,
            combine_mode2,
            texture_count,
            tex_gen_mode: tex_gen_mode_packed,
            packed_alpha_flags,
            visible: is_visible as u32,
            indirect_mtx0,
            indirect_mtx1,
            ..Default::default()
        };

        Some(TexturedQuad {
            x: position.x,
            y: position.y,
            width: size.x,
            height: size.y,
            corners,
            uvs,
            base_uvs,
            tint: [1.0; 4],
            corner_tints: pane_data.corner_tints,
            texture_name: tex_name.to_string(),
            texture_name1,
            texture_name2,
            sampler_0,
            sampler_1,
            sampler_2,
            standard_material,
            detailed_combiner_material,
            is_detailed,
            pane_idx,
            tex_srts: mat.tex_srts.clone(),
            proj_scales,
            proj_translations,
            indirect_rotation,
            indirect_scale,
            material_idx: pane_data.material_idx,
            piece_id: pane_data.piece_id,
            rotation: pane_data.rotation,
        })
    }
}

#[derive(Debug)]
pub enum PaneQuadData<'a> {
    Plain(&'a Quad),
    Textured(&'a mut TexturedQuad),
}

impl<'a> PaneQuadData<'a> {
    pub fn pane_idx(&self) -> usize {
        match self {
            PaneQuadData::Plain(q) => q.pane_idx,
            PaneQuadData::Textured(t) => t.pane_idx,
        }
    }
}

fn highlight(color: [f32; 4]) -> [f32; 4] {
    [
        (color[0] + 0.4).min(1.0),
        (color[1] + 0.4).min(1.0),
        (color[2] + 0.4).min(1.0),
        0.95,
    ]
}

fn corner_vertex(data: &PaneQuadData, corner: usize, tint: [f32; 4]) -> Vertex {
    match data {
        PaneQuadData::Plain(q) => Vertex {
            position: q.corners[corner],
            uv0: PLAIN_UVS[corner],
            uv1: PLAIN_UVS[corner],
            uv2: PLAIN_UVS[corner],
            tint,
            quad_size: [q.width, q.height],
        },
        PaneQuadData::Textured(tq) => {
            let ct = tq.corner_tints[corner];
            Vertex {
                position: tq.corners[corner],
                uv0: tq.uvs[corner][0],
                uv1: tq.uvs[corner][1],
                uv2: tq.uvs[corner][2],
                tint: [
                    tint[0] * ct[0],
                    tint[1] * ct[1],
                    tint[2] * ct[2],
                    tint[3] * ct[3],
                ],
                quad_size: [tq.width, tq.height],
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum BatchKey {
    Plain,
    Textured {
        texture_name: String,
        sampler: WgpuSamplerSettings,
        material_idx: u16,
        combine_mode: u32,
        combine_mode2: u32,
        is_detailed: bool,
        detailed_combiner_hash: [i32; 6],
    },
}

struct Batch {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    mat_buffer: Option<wgpu::Buffer>,

    detailed_buffer: Option<wgpu::Buffer>,
    num_indices: u32,

    cached_material: Option<StandardMaterial>,
    cached_detailed_material: Option<DetailedCombinerMaterial>,
    cached_sampler_settings: Option<(
        WgpuSamplerSettings,
        WgpuSamplerSettings,
        WgpuSamplerSettings,
    )>,

    key: BatchKey,

    /// [`TexturedQuad::pane_idx`] & [`TexturedQuad::piece_id`]
    piece_keys: Vec<(usize, usize)>,
}

pub struct PaneRenderer {
    pipeline_standard: wgpu::RenderPipeline,
    pipeline_detailed: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bgl: BindGroupLayout,

    #[allow(dead_code)]
    placeholder_texture: wgpu::Texture,
    placeholder_view: TextureView,
    placeholder_sampler: wgpu::Sampler,

    batches: Vec<Batch>,

    quad_lookup: HashMap<(usize, usize), usize>,
}

impl PaneRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pane_quad_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/textured_quad.wgsl").into()),
        });

        let proj_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pane_proj_bgl"),
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

        let identity = [
            [1f32, 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pane_proj_buffer"),
            contents: bytemuck::bytes_of(&Uniforms::from_matrix(identity)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pane_proj_bg"),
            layout: &proj_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pane_texture_bgl"),
            entries: &[
                Self::tex_entry(0), // t_texture0
                Self::smp_entry(1), // s_sampler0
                Self::tex_entry(2), // t_texture1
                Self::smp_entry(3), // s_sampler1
                Self::tex_entry(4), // t_texture2
                Self::smp_entry(5), // s_sampler2
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pane_pipeline_layout"),
            bind_group_layouts: &[Some(&proj_bgl), Some(&texture_bgl)],
            immediate_size: 0,
        });

        let create_pipeline = |entry: &str| -> wgpu::RenderPipeline {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("pane_pipeline_{}", entry)),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let pipeline_standard = create_pipeline("fs_standard");
        let pipeline_detailed = create_pipeline("fs_detailed");

        let placeholder_texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("pane_placeholder_white"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[255u8, 255, 255, 255],
        );
        let placeholder_view =
            placeholder_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let placeholder_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline_standard,
            pipeline_detailed,
            uniform_buffer,
            uniform_bind_group,
            texture_bgl,
            placeholder_texture,
            placeholder_view,
            placeholder_sampler,
            batches: Vec::new(),
            quad_lookup: HashMap::new(),
        }
    }

    fn tex_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }
    }

    fn smp_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }
    }

    pub fn update_projection(&self, queue: &wgpu::Queue, matrix: [[f32; 4]; 4]) {
        puffin::profile_function!();
        puffin::profile_function!();
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms::from_matrix(matrix)),
        );
    }

    fn batch_key_for(data: &PaneQuadData) -> BatchKey {
        match data {
            PaneQuadData::Plain(_) => BatchKey::Plain,
            PaneQuadData::Textured(tq) => {
                let mut detailed_combiner_hash = [0i32; 6];
                if tq.is_detailed {
                    for (i, hash) in detailed_combiner_hash.iter_mut().enumerate() {
                        *hash = tq.detailed_combiner_material.stage_bits[i][0]
                            ^ tq.detailed_combiner_material.stage_bits[i][1]
                            ^ tq.detailed_combiner_material.stage_bits[i][2];
                    }
                }

                // better batch key needed, material idx defeats puprpose of batching, but for now
                // helps avoid bad collisions.
                BatchKey::Textured {
                    texture_name: tq.texture_name.clone(),
                    sampler: tq.sampler_0,
                    material_idx: tq.material_idx,
                    combine_mode: tq.standard_material.combine_mode,
                    combine_mode2: tq.standard_material.combine_mode2,
                    is_detailed: tq.is_detailed,
                    detailed_combiner_hash,
                }
            }
        }
    }

    pub fn upload_quads(
        &mut self,
        device: &wgpu::Device,
        ordered: &[PaneQuadData],
        texture_cache: &TextureCache,
        layout_size: Vector2f,
    ) {
        puffin::profile_function!();
        self.batches.clear();
        self.quad_lookup.clear();

        for data in ordered {
            let key = Self::batch_key_for(data);
            let pane_idx = data.pane_idx();
            let piece_id = match data {
                PaneQuadData::Plain(_) => usize::MAX,
                PaneQuadData::Textured(tq) => tq.piece_id,
            };
            let piece_key = (pane_idx, piece_id);

            let flags = PaneVisibilityFlags::default();
            let base_tint = match data {
                PaneQuadData::Plain(q) => flags.plain_color(q, false),
                PaneQuadData::Textured(tq) => flags.textured_tint(tq, false),
            };

            let verts: [Vertex; 4] = std::array::from_fn(|i| corner_vertex(data, i, base_tint));

            let mut match_found = false;
            if let Some(last) = self.batches.last_mut()
                && last.key == key
            {
                let base = last.vertices.len() as u32;
                last.vertices.extend_from_slice(&verts);
                last.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base + 1,
                    base + 3,
                    base + 2,
                ]);
                last.piece_keys.push(piece_key);
                match_found = true;
            }

            if !match_found {
                let cached_material = match data {
                    PaneQuadData::Plain(_) => None,
                    PaneQuadData::Textured(tq) => Some(tq.standard_material),
                };

                let cached_detailed_material = match data {
                    PaneQuadData::Plain(_) => None,
                    PaneQuadData::Textured(tq) => Some(tq.detailed_combiner_material),
                };

                let cached_sampler_settings = match data {
                    PaneQuadData::Plain(_) => None,
                    PaneQuadData::Textured(tq) => Some((tq.sampler_0, tq.sampler_1, tq.sampler_2)),
                };

                self.batches.push(Batch {
                    vertices: verts.to_vec(),
                    indices: vec![0, 1, 2, 1, 3, 2],
                    vertex_buffer: None,
                    index_buffer: None,
                    bind_group: None,
                    mat_buffer: None,
                    detailed_buffer: None,
                    num_indices: 0,
                    key,
                    cached_material,
                    cached_detailed_material,
                    cached_sampler_settings,
                    piece_keys: vec![piece_key],
                });
            }
        }

        let make_sampler = |sampler_settings: WgpuSamplerSettings| -> wgpu::Sampler {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: sampler_settings.address_mode_u,
                address_mode_v: sampler_settings.address_mode_v,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                min_filter: sampler_settings.min_filter,
                mag_filter: sampler_settings.mag_filter,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            })
        };

        for batch in &mut self.batches {
            if batch.vertices.is_empty() {
                continue;
            }

            batch.vertex_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("pane_vb"),
                    contents: bytemuck::cast_slice(&batch.vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                },
            ));
            batch.index_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("pane_ib"),
                    contents: bytemuck::cast_slice(&batch.indices),
                    usage: wgpu::BufferUsages::INDEX,
                },
            ));
            batch.num_indices = batch.indices.len() as u32;

            match &batch.key {
                BatchKey::Plain => {
                    let mat = StandardMaterial::plain();
                    let mat_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("pane_plain_mat_buf"),
                        contents: bytemuck::bytes_of(&mat),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });
                    let detailed_buf =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("pane_plain_detailed_buf"),
                            contents: bytemuck::bytes_of(&DetailedCombinerMaterial::default()),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });

                    batch.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("pane_plain_bg"),
                        layout: &self.texture_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(
                                    &self.placeholder_view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&self.placeholder_sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(
                                    &self.placeholder_view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::Sampler(&self.placeholder_sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: wgpu::BindingResource::TextureView(
                                    &self.placeholder_view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 5,
                                resource: wgpu::BindingResource::Sampler(&self.placeholder_sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 6,
                                resource: mat_buf.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 7,
                                resource: detailed_buf.as_entire_binding(),
                            },
                        ],
                    }));

                    batch.mat_buffer = Some(mat_buf);
                    batch.detailed_buffer = Some(detailed_buf);
                }
                BatchKey::Textured { texture_name, .. } => {
                    let Some(&(first_pane_idx, first_piece_id)) = batch.piece_keys.first() else {
                        continue;
                    };

                    let Some(PaneQuadData::Textured(rep_quad)) = ordered.iter().find(|d| {
                        if let PaneQuadData::Textured(tq) = d {
                            tq.pane_idx == first_pane_idx && tq.piece_id == first_piece_id
                        } else {
                            false
                        }
                    }) else {
                        continue;
                    };

                    let gpu_tex0 = texture_cache.get(texture_name);
                    if gpu_tex0.is_none() && !texture_name.is_empty() {
                        log::warn!(
                            "PaneRenderer: texture '{texture_name}' not found, falling back to placeholder."
                        );
                    }

                    let view0 = gpu_tex0.map(|t| &t.view).unwrap_or(&self.placeholder_view);

                    let mut final_mat = rep_quad.standard_material;
                    final_mat.proj_mtx0 =
                        Self::calculate_projection_matrix(rep_quad, texture_cache, layout_size, 0);
                    final_mat.proj_mtx1 =
                        Self::calculate_projection_matrix(rep_quad, texture_cache, layout_size, 1);
                    final_mat.proj_mtx2 =
                        Self::calculate_projection_matrix(rep_quad, texture_cache, layout_size, 2);

                    let mat_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("pane_standard_mat_buf"),
                        contents: bytemuck::bytes_of(&final_mat),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

                    let detailed_buf =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("pane_detailed_mat_buf"),
                            contents: bytemuck::bytes_of(&rep_quad.detailed_combiner_material),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });

                    let view1 = rep_quad
                        .texture_name1
                        .as_ref()
                        .and_then(|n| texture_cache.get(n))
                        .map(|t| &t.view)
                        .unwrap_or(view0);

                    let view2 = rep_quad
                        .texture_name2
                        .as_ref()
                        .and_then(|n| texture_cache.get(n))
                        .map(|t| &t.view)
                        .unwrap_or(view0);

                    let sampler0 = make_sampler(rep_quad.sampler_0);
                    let sampler1 = make_sampler(rep_quad.sampler_1);
                    let sampler2 = make_sampler(rep_quad.sampler_2);

                    batch.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("pane_textured_bg"),
                        layout: &self.texture_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(view0),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sampler0),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(view1),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::Sampler(&sampler1),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: wgpu::BindingResource::TextureView(view2),
                            },
                            wgpu::BindGroupEntry {
                                binding: 5,
                                resource: wgpu::BindingResource::Sampler(&sampler2),
                            },
                            wgpu::BindGroupEntry {
                                binding: 6,
                                resource: mat_buf.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 7,
                                resource: detailed_buf.as_entire_binding(),
                            },
                        ],
                    }));

                    batch.mat_buffer = Some(mat_buf);
                    batch.detailed_buffer = Some(detailed_buf);
                }
            }
        }

        self.quad_lookup = ordered
            .iter()
            .enumerate()
            .map(|(idx, d)| {
                let piece_id = match d {
                    PaneQuadData::Plain(_) => usize::MAX,
                    PaneQuadData::Textured(tq) => tq.piece_id,
                };
                ((d.pane_idx(), piece_id), idx)
            })
            .collect();
    }

    fn update_selection(
        batch: &mut Batch,
        quad_lookup: &HashMap<(usize, usize), usize>,
        ordered: &[PaneQuadData],
        selected_idx: Option<usize>,
        hidden_panes: &HashSet<usize>,
        flags: PaneVisibilityFlags,
    ) -> bool {
        let mut dirty = false;

        for (batch_quad_idx, &(pane_idx, piece_id)) in batch.piece_keys.iter().enumerate() {
            let base = batch_quad_idx * 4;
            if base + 3 >= batch.vertices.len() {
                break;
            }

            let lookup_key = (pane_idx, if piece_id == 0 { usize::MAX } else { piece_id });

            let Some(data) = quad_lookup
                .get(&lookup_key)
                .copied()
                .or_else(|| quad_lookup.get(&(pane_idx, piece_id)).copied())
                .and_then(|idx| ordered.get(idx))
            else {
                continue;
            };

            let hidden = hidden_panes.contains(&pane_idx);
            let selected = Some(pane_idx) == selected_idx;

            let tint = match data {
                PaneQuadData::Plain(q) => {
                    let base_tint = flags.plain_color(q, hidden);

                    if selected && !hidden {
                        highlight(base_tint)
                    } else {
                        base_tint
                    }
                }
                PaneQuadData::Textured(tq) => flags.textured_tint(tq, hidden),
            };

            for v_offset in 0..4 {
                let corner_scale = match data {
                    PaneQuadData::Plain(_) => [1.0, 1.0, 1.0, 1.0],
                    PaneQuadData::Textured(tq) => tq.corner_tints[v_offset],
                };

                let target_tint = [
                    tint[0] * corner_scale[0],
                    tint[1] * corner_scale[1],
                    tint[2] * corner_scale[2],
                    tint[3] * corner_scale[3],
                ];

                let vertex = &mut batch.vertices[base + v_offset];

                if vertex.tint != target_tint {
                    vertex.tint = target_tint;
                    dirty = true;
                }
            }
        }

        dirty
    }

    fn update_texture_pattern(
        batch: &mut Batch,
        device: &wgpu::Device,
        ordered: &[PaneQuadData],
        texture_cache: &TextureCache,
        quad_lookup: &HashMap<(usize, usize), usize>,
        placeholder_view: &TextureView,
        texture_bgl: &BindGroupLayout,
    ) {
        let BatchKey::Textured {
            texture_name,
            sampler,
            ..
        } = &mut batch.key
        else {
            return;
        };

        let Some(&piece_key) = batch.piece_keys.first() else {
            return;
        };

        let Some(data_idx) = quad_lookup.get(&piece_key) else {
            return;
        };

        let Some(PaneQuadData::Textured(tq)) = ordered.get(*data_idx) else {
            return;
        };

        let tex0_name = &tq.texture_name;
        let current_samplers = (*sampler, tq.sampler_1, tq.sampler_2);

        if texture_name == tex0_name && batch.cached_sampler_settings == Some(current_samplers) {
            return;
        }

        let gpu_tex0 = texture_cache.get(tex0_name);
        let view0 = gpu_tex0.map(|t| &t.view).unwrap_or(placeholder_view);

        let view1 = tq
            .texture_name1
            .as_deref()
            .and_then(|n| texture_cache.get(n))
            .map(|t| &t.view)
            .unwrap_or(view0);

        let view2 = tq
            .texture_name2
            .as_deref()
            .and_then(|n| texture_cache.get(n))
            .map(|t| &t.view)
            .unwrap_or(view0);

        let make_sampler = |sampler_settings: WgpuSamplerSettings| -> wgpu::Sampler {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: sampler_settings.address_mode_u,
                address_mode_v: sampler_settings.address_mode_v,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                min_filter: sampler_settings.min_filter,
                mag_filter: sampler_settings.mag_filter,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            })
        };

        let sampler0 = make_sampler(*sampler);
        let sampler1 = make_sampler(tq.sampler_1);
        let sampler2 = make_sampler(tq.sampler_2);

        let Some(mat_buf) = &batch.mat_buffer else {
            return;
        };

        let detailed_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pane_detailed_mat_buf_pat"),
            contents: bytemuck::bytes_of(&tq.detailed_combiner_material),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        batch.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pane_bg_pattern"),
            layout: texture_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view0),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler0),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(view1),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler1),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(view2),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&sampler2),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: mat_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: detailed_buf.as_entire_binding(),
                },
            ],
        }));

        batch.detailed_buffer = Some(detailed_buf);
        batch.cached_sampler_settings = Some(current_samplers);
        batch.cached_detailed_material = None;

        *texture_name = tex0_name.clone();
    }

    fn recompute_proj_mtx(
        tq: &mut TexturedQuad,
        texture_cache: &TextureCache,
        layout_size: Vector2f,
    ) {
        let mode0 = tq.standard_material.tex_gen_mode & 0x3;
        let mode1 = (tq.standard_material.tex_gen_mode >> 8) & 0x3;
        let mode2 = (tq.standard_material.tex_gen_mode >> 16) & 0x3;
        if mode0 != 1 && mode1 != 1 && mode2 != 1 {
            return;
        }

        tq.standard_material.proj_mtx0 =
            Self::calculate_projection_matrix(tq, texture_cache, layout_size, 0);
        tq.standard_material.proj_mtx1 =
            Self::calculate_projection_matrix(tq, texture_cache, layout_size, 1);
        tq.standard_material.proj_mtx2 =
            Self::calculate_projection_matrix(tq, texture_cache, layout_size, 2);
    }

    fn calculate_projection_matrix(
        quad: &TexturedQuad,
        texture_cache: &TextureCache,
        layout_size: Vector2f,
        layer_idx: usize,
    ) -> [[f32; 4]; 2] {
        let pane_cx = quad.x + quad.width * 0.5;
        let pane_cy = quad.y + quad.height * 0.5;

        let shift = layer_idx * 8;
        let packed = quad.standard_material.tex_gen_mode >> shift;
        let mode = packed & 0x3;

        if mode != 1 {
            return [[1.0, 0.0, 0.0, 0.5], [0.0, 1.0, 0.0, 0.5]];
        }

        let fitting_layout_size = (packed & (1 << 2)) != 0;
        let fitting_pane_size = (packed & (1 << 3)) != 0;
        let adjust_sr = (packed & (1 << 4)) != 0;
        let orthogonal = (packed & (1 << 5)) != 0;

        let (base_w, base_h) = if fitting_layout_size {
            (layout_size.x, layout_size.y)
        } else if fitting_pane_size {
            (quad.width, quad.height)
        } else {
            let (tex_w, tex_h) =
                Self::get_texture_size(quad, texture_cache, layer_idx).unwrap_or((0, 0));

            (tex_w as f32, tex_h as f32)
        };

        let (cx, cy) = if orthogonal {
            (layout_size.x * 0.5, layout_size.y * 0.5)
        } else {
            (pane_cx, pane_cy)
        };

        let srt_tu = quad
            .tex_srts
            .get(layer_idx)
            .map(|s| s.translate_u)
            .unwrap_or(0.0);

        let srt_tv = quad
            .tex_srts
            .get(layer_idx)
            .map(|s| s.translate_v)
            .unwrap_or(0.0);

        let proj_scale_x = quad.proj_scales[layer_idx][0];
        let proj_scale_y = quad.proj_scales[layer_idx][1];

        let proj_translate_x = quad.proj_translations[layer_idx][0];
        let proj_translate_y = quad.proj_translations[layer_idx][1];

        let m_s = 1.0 / (base_w * proj_scale_x);
        let m_t = 1.0 / (base_h * proj_scale_y);

        if adjust_sr {
            let srt = quad.tex_srts.get(layer_idx);
            let srt_scale_x = srt.map(|s| s.scale_u).unwrap_or(1.0);
            let srt_scale_y = srt.map(|s| s.scale_v).unwrap_or(1.0);
            let srt_rotate = srt.map(|s| s.rotate).unwrap_or(0.0);

            let rad = (srt_rotate + quad.rotation.z).to_radians();
            let (sin_r, cos_r) = rad.sin_cos();

            let base_scale_s = 1.0 / (base_w * proj_scale_x);
            let base_scale_t = 1.0 / (base_h * proj_scale_y);

            let s_factor = base_scale_s * srt_scale_x;
            let t_factor = base_scale_t * srt_scale_y;

            let m00 = s_factor * cos_r;
            let m01 = s_factor * sin_r;
            let m10 = -t_factor * sin_r;
            let m11 = t_factor * cos_r;

            let total_x = proj_translate_x + cx;
            let total_y = proj_translate_y + cy;

            let trans_s = 0.5 - (total_x * m00 + total_y * m01) + srt_tu;
            let trans_t = 0.5 - (total_x * m10 + total_y * m11) + srt_tv;

            [[m00, m01, 0.0, trans_s], [m10, m11, 0.0, trans_t]]
        } else {
            let trans_s = 0.5 - (proj_translate_x + cx) * m_s + srt_tu;
            let trans_t = 0.5 - (proj_translate_y + cy) * m_t + srt_tv;

            [[m_s, 0.0, 0.0, trans_s], [0.0, m_t, 0.0, trans_t]]
        }
    }

    fn get_texture_size(
        quad: &TexturedQuad,
        texture_cache: &TextureCache,
        layer_idx: usize,
    ) -> Option<(u32, u32)> {
        let tex_name = match layer_idx {
            1 => quad.texture_name1.as_deref(),
            2 => quad.texture_name2.as_deref(),
            _ => Some(quad.texture_name.as_str()),
        };

        tex_name
            .and_then(|name| texture_cache.get(name))
            .map(|t| (t.width, t.height))
    }

    pub fn update_visuals(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ordered: &mut [PaneQuadData],
        selected_idx: Option<usize>,
        hidden_panes: &HashSet<usize>,
        flags: PaneVisibilityFlags,
        texture_cache: &TextureCache,
        layout_size: Vector2f,
    ) {
        puffin::profile_function!();

        {
            puffin::profile_scope!("first_pass");
            ordered.par_iter_mut().for_each(|quad| {
                let PaneQuadData::Textured(tq) = quad else {
                    return;
                };

                tq.standard_material.debug_stage = flags.active_debug_stage;

                Self::recompute_proj_mtx(tq, texture_cache, layout_size);
            });
        }

        let dirty_batches: Vec<&mut Batch> = {
            puffin::profile_scope!("second_pass");

            self.batches
                .par_iter_mut()
                .filter_map(|batch| {
                    let mut dirty = false;

                    dirty |=
                        Self::update_anim(&self.quad_lookup, batch, ordered, hidden_panes, flags);

                    Self::update_texture_pattern(
                        batch,
                        device,
                        ordered,
                        texture_cache,
                        &self.quad_lookup,
                        &self.placeholder_view,
                        &self.texture_bgl,
                    );

                    dirty |= Self::update_selection(
                        batch,
                        &self.quad_lookup,
                        ordered,
                        selected_idx,
                        hidden_panes,
                        flags,
                    );

                    Self::flush_mat_buffers(batch, &self.quad_lookup, queue, ordered, hidden_panes);

                    if dirty { Some(batch) } else { None }
                })
                .collect()
        };

        puffin::profile_scope!("upload_buffers");
        for batch in dirty_batches {
            if let Some(vb) = &batch.vertex_buffer {
                queue.write_buffer(vb, 0, bytemuck::cast_slice(&batch.vertices));
            }
        }
    }

    fn update_anim(
        quad_lookup: &HashMap<(usize, usize), usize>,
        batch: &mut Batch,
        ordered: &[PaneQuadData],
        hidden_panes: &HashSet<usize>,
        flags: PaneVisibilityFlags,
    ) -> bool {
        let mut dirty = false;
        let has_hidden = !hidden_panes.is_empty();

        for (batch_quad_idx, &piece_key) in batch.piece_keys.iter().enumerate() {
            let base = batch_quad_idx * 4;

            let Some(data) = quad_lookup
                .get(&piece_key)
                .and_then(|&idx| ordered.get(idx))
            else {
                continue;
            };

            let hidden = has_hidden && hidden_panes.contains(&data.pane_idx());
            let tint = match data {
                PaneQuadData::Plain(q) => flags.plain_color(q, hidden),
                PaneQuadData::Textured(tq) => flags.textured_tint(tq, hidden),
            };

            for v_offset in 0..4 {
                let new_vertex = corner_vertex(data, v_offset, tint);
                let current_vertex = &mut batch.vertices[base + v_offset];

                if current_vertex != &new_vertex {
                    *current_vertex = new_vertex;
                    dirty = true;
                }
            }
        }

        dirty
    }

    fn flush_mat_buffers(
        batch: &mut Batch,
        quad_lookup: &HashMap<(usize, usize), usize>,
        queue: &wgpu::Queue,
        ordered: &[PaneQuadData],
        hidden_panes: &HashSet<usize>,
    ) {
        if !matches!(batch.key, BatchKey::Textured { .. }) {
            return;
        }

        let Some(&first_key) = batch.piece_keys.first() else {
            return;
        };

        let Some(data_idx) = quad_lookup.get(&first_key) else {
            return;
        };

        let Some(PaneQuadData::Textured(tq)) = ordered.get(*data_idx) else {
            return;
        };

        if let Some(mb) = &batch.mat_buffer {
            let mut mat = tq.standard_material;
            if hidden_panes.contains(&first_key.0) && mat.visible != 0 {
                mat.visible = 0;
            }

            let needs_update = match &batch.cached_material {
                Some(cached) => cached != &mat,
                None => true,
            };

            if needs_update {
                puffin::profile_scope!("gpu_write_standard_material");
                queue.write_buffer(mb, 0, bytemuck::bytes_of(&mat));
                batch.cached_material = Some(mat);
            }
        }

        if let Some(db) = &batch.detailed_buffer {
            let detailed_mat = &tq.detailed_combiner_material;

            let detailed_needs_update = match &batch.cached_detailed_material {
                Some(cached) => cached != detailed_mat,
                None => true,
            };

            if detailed_needs_update {
                puffin::profile_scope!("gpu_write_detailed_material");
                queue.write_buffer(db, 0, bytemuck::bytes_of(detailed_mat));
                batch.cached_detailed_material = Some(*detailed_mat);
            }
        }
    }

    pub fn render<'rpass>(&'rpass self, rpass: &mut wgpu::RenderPass<'rpass>) {
        puffin::profile_function!();
        rpass.set_bind_group(0, &self.uniform_bind_group, &[]);

        for batch in &self.batches {
            if batch.num_indices == 0 {
                continue;
            }

            let (Some(vb), Some(ib), Some(bg)) =
                (&batch.vertex_buffer, &batch.index_buffer, &batch.bind_group)
            else {
                continue;
            };

            let use_detailed = matches!(
                &batch.key,
                BatchKey::Textured {
                    is_detailed: true,
                    ..
                }
            );

            if use_detailed {
                rpass.set_pipeline(&self.pipeline_detailed);
            } else {
                rpass.set_pipeline(&self.pipeline_standard);
            }

            rpass.set_bind_group(1, bg, &[]);
            rpass.set_vertex_buffer(0, vb.slice(..));
            rpass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..batch.num_indices, 0, 0..1);
        }
    }
}
