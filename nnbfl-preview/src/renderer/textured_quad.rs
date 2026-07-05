use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use nnbfl::bflyt::flags::{TexFilter, TexWrapMode};
use nnbfl::bflyt::list::{Material, MaterialTextureMap, MaterialTextureSrt, TexGenSrc};
use nnbfl::bflyt::pane::PicturePane;
use nnbfl::ui2d::types::Vector2f;
use wgpu::util::DeviceExt;

use super::quad::Quad;
use super::texture::TextureCache;
use crate::pane_tree::PaneNode;
use crate::renderer::quad::Uniforms;
use crate::ui::PaneVisibilityFlags;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv0: [f32; 2],
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    pub tint: [f32; 4],
    pub tex_aspects: [f32; 3],
    pub quad_size: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x2, // uv0
        2 => Float32x2, // uv1
        3 => Float32x2, // uv2
        4 => Float32x4, // tint
        5 => Float32x3, // tex_aspects
        6 => Float32x2, // quad_size
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const PLAIN_UVS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct StandardMaterial {
    pub interpolate_width: [f32; 4],
    pub interpolate_offset: [f32; 4],
    pub combine_mode: u32,
    pub combine_mode2: u32,
    pub texture_count: u32,
    pub alpha_select: u32,
    pub tex_gen_mode: u32,
    pub visible: u32,
    pub use_texture_only: u32,
    pub use_thresholding_alpha_interpolation: u32,

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
            interpolate_width: [1.0, 1.0, 1.0, 1.0],
            interpolate_offset: [0.0, 0.0, 0.0, 0.0],
            combine_mode: 0,
            combine_mode2: 0,
            texture_count: 1,
            alpha_select: 0,
            tex_gen_mode: 0,
            visible: 1,
            use_texture_only: 0,
            use_thresholding_alpha_interpolation: 0,
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
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
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

    pub is_detailed: bool,
    pub standard_material: StandardMaterial,
    pub detailed_combiner_material: DetailedCombinerMaterial,

    pub proj_scales: [[f32; 2]; 3],
    pub proj_translations: [[f32; 2]; 3],
}

impl TexturedQuad {
    pub fn derive_from_material(
        pic: &PicturePane,
        mat: &Material,
        position: Vector2f,
        size: Vector2f,
        corners: [[f32; 2]; 4],
        is_visible: bool,
        pane_idx: usize,
    ) -> Option<Self> {
        let tex_map = mat.tex_maps.first()?;
        let tex_name = tex_map.texture_name.trim_end();

        if tex_name.is_empty() {
            return None;
        }

        let tl: [f32; 4] = pic.top_left_vertex_color.into();
        let tint = if is_visible {
            if tl[3] > 0.0 { tl } else { [1.0; 4] }
        } else {
            [0.0; 4]
        };

        let (base_uvs, uvs) = PaneNode::compute_uvs(pic, mat);

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

        let texture_count = mat.tex_maps.len().min(3) as u32;

        let mut tex_gen_flags = [0u32; 3];
        for (flag, coord_gen) in tex_gen_flags
            .iter_mut()
            .zip(mat.tex_coord_gens.iter().take(texture_count as usize))
        {
            let (mode, is_ortho) = match coord_gen.tex_gen_source {
                TexGenSrc::PaneBasedProjection | TexGenSrc::PaneBasedPerspectiveProjection => {
                    (1, false)
                }
                TexGenSrc::OrthogonalProjection | TexGenSrc::PerspectiveProjection => (1, true),
                TexGenSrc::BrickRepeat => (2, false),
                _ => (0, false),
            };
            *flag = mode;
            if is_ortho {
                *flag |= 1 << 5;
            }
        }

        let mut proj_scales = [[1.0f32; 2]; 3];
        let mut proj_translations = [[0.0f32; 2]; 3];
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

        let color_f32 = |entry: &nnbfl::bflyt::list::MaterialColorEntry| -> [f32; 4] {
            if let Some(c) = &entry.color_u8 {
                (*c).into()
            } else if let Some(c) = &entry.color_f32 {
                (*c).into()
            } else {
                [0.0; 4]
            }
        };

        let black_color = mat
            .colors
            .first()
            .map(color_f32)
            .unwrap_or([0.0, 0.0, 0.0, 0.0]);
        let white_color = mat
            .colors
            .get(1)
            .map(color_f32)
            .unwrap_or([1.0, 1.0, 1.0, 1.0]);
        let interpolate_width = [
            white_color[0] - black_color[0],
            white_color[1] - black_color[1],
            white_color[2] - black_color[2],
            white_color[3] - black_color[3],
        ];

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
            (
                tev0.rgb_mode as u32,
                mat.tev_combiners
                    .get(1)
                    .map(|t| t.rgb_mode as u32)
                    .unwrap_or(0),
            )
        } else {
            (0, 0)
        };

        let (indirect_mtx0, indirect_mtx1) = if let Some(im) = &mat.indirect_matrix {
            let rad = im.rotation.to_radians();
            (
                [rad.cos() * im.scale.x, -rad.sin() * im.scale.x, 0.0, 0.0],
                [rad.sin() * im.scale.y, rad.cos() * im.scale.y, 0.0, 0.0],
            )
        } else {
            ([0.0f32; 4], [0.0f32; 4])
        };

        let standard_material = StandardMaterial {
            interpolate_width,
            interpolate_offset: black_color,
            combine_mode,
            combine_mode2,
            texture_count,
            alpha_select: 0,
            tex_gen_mode: tex_gen_mode_packed,
            use_texture_only: mat.use_texture_only as u32,
            use_thresholding_alpha_interpolation: mat.use_thresholding_alpha_interpolation as u32,
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
            tint,
            corner_tints: [tint; 4],
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
        })
    }
}

#[derive(Clone, Debug)]
pub enum PaneQuadData {
    Plain(Quad),
    Textured(Box<TexturedQuad>),
}

impl PaneQuadData {
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

fn corner_vertex(
    data: &PaneQuadData,
    corner: usize,
    tex_aspects: [f32; 3],
    tint: [f32; 4],
) -> Vertex {
    match data {
        PaneQuadData::Plain(q) => Vertex {
            position: q.corners[corner],
            uv0: PLAIN_UVS[corner],
            uv1: PLAIN_UVS[corner],
            uv2: PLAIN_UVS[corner],
            tint,
            tex_aspects,
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
                tex_aspects,
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
    #[allow(dead_code)]
    detailed_buffer: Option<wgpu::Buffer>,
    num_indices: u32,

    key: BatchKey,
    pane_indices: Vec<usize>,
}

pub struct PaneRenderer {
    pipeline_standard: wgpu::RenderPipeline,
    pipeline_detailed: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bgl: wgpu::BindGroupLayout,

    #[allow(dead_code)]
    placeholder_texture: wgpu::Texture,
    placeholder_view: wgpu::TextureView,
    placeholder_sampler: wgpu::Sampler,

    batches: Vec<Batch>,
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

                BatchKey::Textured {
                    texture_name: tq.texture_name.clone(),
                    sampler: tq.sampler_0,
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
        layout_w: f32,
        layout_h: f32,
    ) {
        self.batches.clear();

        for data in ordered {
            let key = Self::batch_key_for(data);
            let pane_idx = data.pane_idx();

            let tex_aspects = match data {
                PaneQuadData::Plain(_) => [1.0, 1.0, 1.0],
                PaneQuadData::Textured(tq) => [
                    Self::get_layer_aspect(tq, texture_cache, layout_w, layout_h, 0),
                    Self::get_layer_aspect(tq, texture_cache, layout_w, layout_h, 1),
                    Self::get_layer_aspect(tq, texture_cache, layout_w, layout_h, 2),
                ],
            };

            let flags = PaneVisibilityFlags::default();
            let base_tint = match data {
                PaneQuadData::Plain(q) => flags.plain_color(q, false),
                PaneQuadData::Textured(tq) => flags.textured_tint(tq, false),
            };

            let verts: [Vertex; 4] =
                std::array::from_fn(|i| corner_vertex(data, i, tex_aspects, base_tint));

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
                last.pane_indices.push(pane_idx);
                match_found = true;
            }

            if !match_found {
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
                    pane_indices: vec![pane_idx],
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
                            usage: wgpu::BufferUsages::UNIFORM,
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
                    let Some(gpu_tex0) = texture_cache.get(texture_name) else {
                        log::warn!(
                            "PaneRenderer: texture '{texture_name}' not found, skipping batch"
                        );
                        continue;
                    };

                    let Some(&first_pane_idx) = batch.pane_indices.first() else {
                        continue;
                    };
                    let Some(PaneQuadData::Textured(rep_quad)) =
                        ordered.iter().find(|d| d.pane_idx() == first_pane_idx)
                    else {
                        continue;
                    };

                    let mut final_mat = rep_quad.standard_material;
                    final_mat.proj_mtx0 = Self::calculate_projection_matrix(
                        rep_quad,
                        texture_cache,
                        layout_w,
                        layout_h,
                        0,
                    );
                    final_mat.proj_mtx1 = Self::calculate_projection_matrix(
                        rep_quad,
                        texture_cache,
                        layout_w,
                        layout_h,
                        1,
                    );
                    final_mat.proj_mtx2 = Self::calculate_projection_matrix(
                        rep_quad,
                        texture_cache,
                        layout_w,
                        layout_h,
                        2,
                    );

                    let mat_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("pane_standard_mat_buf"),
                        contents: bytemuck::bytes_of(&final_mat),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

                    let detailed_buf =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("pane_detailed_mat_buf"),
                            contents: bytemuck::bytes_of(&rep_quad.detailed_combiner_material),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });

                    let gpu_tex1 = rep_quad
                        .texture_name1
                        .as_ref()
                        .and_then(|n| texture_cache.get(n))
                        .unwrap_or(gpu_tex0);

                    let gpu_tex2 = rep_quad
                        .texture_name2
                        .as_ref()
                        .and_then(|n| texture_cache.get(n))
                        .unwrap_or(gpu_tex0);

                    let sampler0 = make_sampler(rep_quad.sampler_0);
                    let sampler1 = make_sampler(rep_quad.sampler_1);
                    let sampler2 = make_sampler(rep_quad.sampler_2);

                    batch.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("pane_textured_bg"),
                        layout: &self.texture_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&gpu_tex0.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sampler0),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(&gpu_tex1.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::Sampler(&sampler1),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: wgpu::BindingResource::TextureView(&gpu_tex2.view),
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
    }

    pub fn update_selection(
        &mut self,
        queue: &wgpu::Queue,
        ordered: &mut [PaneQuadData],
        selected_idx: Option<usize>,
        hidden_panes: &HashSet<usize>,
        flags: PaneVisibilityFlags,
        active_debug_stage: u32,
    ) {
        for data in ordered.iter_mut() {
            if let PaneQuadData::Textured(tq) = data {
                tq.standard_material.debug_stage = active_debug_stage;
            }
        }

        for batch in &mut self.batches {
            let mut dirty = false;

            for (batch_quad_idx, &pane_idx) in batch.pane_indices.iter().enumerate() {
                let base = batch_quad_idx * 4;
                if base + 3 >= batch.vertices.len() {
                    break;
                }
                let Some(data) = ordered.iter().find(|d| d.pane_idx() == pane_idx) else {
                    continue;
                };

                let hidden = hidden_panes.contains(&pane_idx);
                let selected = Some(pane_idx) == selected_idx;

                let base_tint = match data {
                    PaneQuadData::Plain(q) => flags.plain_color(q, hidden),
                    PaneQuadData::Textured(tq) => flags.textured_tint(tq, hidden),
                };
                let tint = if selected && !hidden {
                    highlight(base_tint)
                } else {
                    base_tint
                };

                for v_offset in 0..4 {
                    let corner_scale = match data {
                        PaneQuadData::Plain(_) => [1.0, 1.0, 1.0, 1.0],
                        PaneQuadData::Textured(tq) => tq.corner_tints[v_offset],
                    };

                    batch.vertices[base + v_offset].tint = [
                        tint[0] * corner_scale[0],
                        tint[1] * corner_scale[1],
                        tint[2] * corner_scale[2],
                        tint[3] * corner_scale[3],
                    ];
                }

                dirty = true;
            }

            if dirty && let Some(vb) = &batch.vertex_buffer {
                queue.write_buffer(vb, 0, bytemuck::cast_slice(&batch.vertices));
            }
        }
    }

    pub fn update_texture_pattern(
        &mut self,
        device: &wgpu::Device,
        ordered: &[PaneQuadData],
        texture_cache: &TextureCache,
    ) {
        for batch in &mut self.batches {
            let BatchKey::Textured {
                texture_name,
                sampler,
                ..
            } = &mut batch.key
            else {
                continue;
            };

            let Some(&pane_idx) = batch.pane_indices.first() else {
                continue;
            };
            let Some(PaneQuadData::Textured(tq)) =
                ordered.iter().find(|d| d.pane_idx() == pane_idx)
            else {
                continue;
            };

            let tex0_name = &tq.texture_name;
            let tex1_name = tq.texture_name1.as_deref().unwrap_or(tex0_name);
            let tex2_name = tq.texture_name2.as_deref().unwrap_or(tex0_name);

            if texture_name == tex0_name {
                continue;
            }

            let Some(gpu_tex0) = texture_cache.get(tex0_name) else {
                continue;
            };
            let gpu_tex1 = texture_cache.get(tex1_name).unwrap_or(gpu_tex0);
            let gpu_tex2 = texture_cache.get(tex2_name).unwrap_or(gpu_tex0);

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
                continue;
            };

            let detailed_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pane_detailed_mat_buf_pat"),
                contents: bytemuck::bytes_of(&tq.detailed_combiner_material),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            batch.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("pane_bg_pattern"),
                layout: &self.texture_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&gpu_tex0.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler0),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&gpu_tex1.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&sampler1),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&gpu_tex2.view),
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
            *texture_name = tex0_name.clone();
        }
    }

    pub fn recompute_proj_mtx(
        &mut self,
        ordered: &mut [PaneQuadData],
        texture_cache: &TextureCache,
        layout_w: f32,
        layout_h: f32,
    ) {
        let mut textured_lookup: HashMap<usize, &mut TexturedQuad> = ordered
            .iter_mut()
            .filter_map(|d| match d {
                PaneQuadData::Textured(tq) => Some((tq.pane_idx, &mut **tq)),
                _ => None,
            })
            .collect();

        for batch in &self.batches {
            if !matches!(batch.key, BatchKey::Textured { .. }) {
                continue;
            }

            let Some(&first_pane) = batch.pane_indices.first() else {
                continue;
            };

            let Some(tq) = textured_lookup.get_mut(&first_pane) else {
                continue;
            };

            let mode0 = tq.standard_material.tex_gen_mode & 0x3;
            let mode1 = (tq.standard_material.tex_gen_mode >> 8) & 0x3;
            let mode2 = (tq.standard_material.tex_gen_mode >> 16) & 0x3;
            if mode0 != 1 && mode1 != 1 && mode2 != 1 {
                continue;
            }

            tq.standard_material.proj_mtx0 =
                Self::calculate_projection_matrix(tq, texture_cache, layout_w, layout_h, 0);
            tq.standard_material.proj_mtx1 =
                Self::calculate_projection_matrix(tq, texture_cache, layout_w, layout_h, 1);
            tq.standard_material.proj_mtx2 =
                Self::calculate_projection_matrix(tq, texture_cache, layout_w, layout_h, 2);
        }
    }

    fn calculate_projection_matrix(
        quad: &TexturedQuad,
        texture_cache: &TextureCache,
        layout_w: f32,
        layout_h: f32,
        layer_idx: usize,
    ) -> [[f32; 4]; 2] {
        let tex_aspect_ratio =
            Self::get_layer_aspect(quad, texture_cache, layout_w, layout_h, layer_idx);

        let pane_cx = quad.x + quad.width * 0.5;
        let pane_cy = quad.y + quad.height * 0.5;

        let shift = layer_idx * 8;
        let packed = quad.standard_material.tex_gen_mode >> shift;
        let mode = packed & 0x3;

        if mode != 1 {
            return [[1.0, 0.0, 0.0, 0.5], [0.0, 1.0, 0.0, 0.5]];
        }

        let fitting_layout_size = (packed & (1 << 2)) != 0;
        let _fitting_pane_size = (packed & (1 << 3)) != 0;
        let adjust_sr = (packed & (1 << 4)) != 0;
        let orthogonal = (packed & (1 << 5)) != 0;

        let (base_w, base_h, cx, cy) = if orthogonal {
            (layout_w, layout_h, layout_w * 0.5, layout_h * 0.5)
        } else if fitting_layout_size {
            (layout_w, layout_h, pane_cx, pane_cy)
        } else {
            (quad.width, quad.height, pane_cx, pane_cy)
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

        if adjust_sr {
            let sx = quad.proj_scales[layer_idx][0];
            let sy = quad.proj_scales[layer_idx][1];

            let tx = quad.proj_translations[layer_idx][0];
            let ty = quad.proj_translations[layer_idx][1];

            let (input_w, input_h) = if orthogonal {
                (layout_w, layout_h)
            } else {
                (quad.width, quad.height)
            };

            let reciprocal_width = 1.0 / input_w;
            let reciprocal_height = 1.0 / input_h;

            let mut scale_s = 0.5 / sx;
            let mut scale_t = 0.5 / sy;

            let mut trans_s = 0.5 - (tx / sx / base_w) + srt_tu;
            let mut trans_t = 0.5 - (ty / sy / base_h) + srt_tv;

            if tex_aspect_ratio > 1.0 {
                scale_t *= tex_aspect_ratio;
                trans_t = trans_t * tex_aspect_ratio + (0.5 - 0.5 * tex_aspect_ratio);
            } else {
                let inv_ratio = 1.0 / tex_aspect_ratio;
                scale_s *= inv_ratio;
                trans_s = trans_s * inv_ratio + (0.5 - 0.5 * inv_ratio);
            }

            [
                [2.0 * reciprocal_width * scale_s, 0.0, 0.0, trans_s],
                [0.0, 2.0 * reciprocal_height * scale_t, 0.0, trans_t],
            ]
        } else {
            let mut m_s = 1.0 / base_w;
            let mut m_t = 1.0 / base_h;

            let (scale_s, scale_t) = if tex_aspect_ratio > 1.0 {
                (1.0, tex_aspect_ratio)
            } else {
                (1.0 / tex_aspect_ratio, 1.0)
            };

            m_s *= scale_s;
            m_t *= scale_t;

            let trans_s = 0.5 - (cx * m_s) + srt_tu;
            let trans_t = 0.5 - (cy * m_t) + srt_tv;

            [[m_s, 0.0, 0.0, trans_s], [0.0, m_t, 0.0, trans_t]]
        }
    }

    fn get_layer_aspect(
        quad: &TexturedQuad,
        texture_cache: &TextureCache,
        layout_w: f32,
        layout_h: f32,
        layer_idx: usize,
    ) -> f32 {
        let shift = layer_idx * 8;
        let packed = quad.standard_material.tex_gen_mode >> shift;
        let orthogonal = (packed & (1 << 5)) != 0;

        let base_aspect = if orthogonal {
            if layout_h > 0.0 {
                layout_w / layout_h
            } else {
                1.0
            }
        } else if quad.height > 0.0 {
            quad.width / quad.height
        } else {
            1.0
        };

        let tex_name = match layer_idx {
            1 => quad.texture_name1.as_deref(),
            2 => quad.texture_name2.as_deref(),
            _ => Some(quad.texture_name.as_str()),
        };

        tex_name
            .and_then(|name| texture_cache.get(name))
            .map(|t| (t.width as f32 / t.height as f32) / base_aspect)
            .unwrap_or(1.0)
    }

    pub fn update_anim(
        &mut self,
        queue: &wgpu::Queue,
        ordered: &[PaneQuadData],
        hidden_panes: &HashSet<usize>,
        flags: PaneVisibilityFlags,
    ) {
        let quad_lookup: HashMap<usize, &PaneQuadData> =
            ordered.iter().map(|d| (d.pane_idx(), d)).collect();

        for batch in &mut self.batches {
            let mut dirty = false;

            for (batch_quad_idx, &pane_idx) in batch.pane_indices.iter().enumerate() {
                let base = batch_quad_idx * 4;
                if base + 3 >= batch.vertices.len() {
                    break;
                }

                let Some(data) = quad_lookup.get(&pane_idx) else {
                    continue;
                };

                let hidden = hidden_panes.contains(&pane_idx);
                let tint = match data {
                    PaneQuadData::Plain(q) => flags.plain_color(q, hidden),
                    PaneQuadData::Textured(tq) => flags.textured_tint(tq, hidden),
                };

                let tex_aspects = batch.vertices[base].tex_aspects;

                for v_offset in 0..4 {
                    batch.vertices[base + v_offset] =
                        corner_vertex(data, v_offset, tex_aspects, tint);
                }

                dirty = true;
            }

            if dirty && let Some(vb) = &batch.vertex_buffer {
                queue.write_buffer(vb, 0, bytemuck::cast_slice(&batch.vertices));
            }
        }
    }

    pub fn flush_mat_buffers(
        &self,
        queue: &wgpu::Queue,
        ordered: &[PaneQuadData],
        hidden_panes: &HashSet<usize>,
    ) {
        let textured_lookup: HashMap<usize, &TexturedQuad> = ordered
            .iter()
            .filter_map(|d| match d {
                PaneQuadData::Textured(tq) => Some((tq.pane_idx, &**tq)),
                _ => None,
            })
            .collect();

        for batch in &self.batches {
            if !matches!(batch.key, BatchKey::Textured { .. }) {
                continue;
            }

            let Some(&first_pane_idx) = batch.pane_indices.first() else {
                continue;
            };

            let Some(tq) = textured_lookup.get(&first_pane_idx) else {
                continue;
            };

            let Some(mb) = &batch.mat_buffer else {
                continue;
            };

            let mut mat = tq.standard_material;
            if hidden_panes.contains(&first_pane_idx) {
                mat.visible = 0;
            }

            queue.write_buffer(mb, 0, bytemuck::bytes_of(&mat));
        }
    }

    pub fn render<'rpass>(&'rpass self, rpass: &mut wgpu::RenderPass<'rpass>) {
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
