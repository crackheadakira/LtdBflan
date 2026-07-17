use bytemuck::{Pod, Zeroable};
use wgpu::{SurfaceConfiguration, util::DeviceExt};

use crate::camera::Camera;

#[derive(Clone, Debug)]
pub struct Quad {
    pub pane_idx: usize,
    pub width: f32,
    pub height: f32,
    pub corners: [[f32; 2]; 4],
    pub color: [f32; 4],

    /// True when this quad is the RootPane from a PartsPane.
    pub is_parts_root: bool,

    /// True when this pane also owns a [`super::textured_quad::TexturedQuad`].
    pub has_textured: bool,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Uniforms {
    pub proj: [[f32; 4]; 4],
}

impl Uniforms {
    pub fn from_matrix(proj: [[f32; 4]; 4]) -> Self {
        Self { proj }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridUniforms {
    pub proj: [[f32; 4]; 4],
    pub resolution: [f32; 2],
    pub zoom: f32,
    pub _padding: f32,
}

pub struct GridRenderer {
    pub grid_pipeline: RenderPipelineContainer,
}

impl GridRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let matrix = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];

        let grid_container = RenderPipelineContainer::new(
            device,
            "grid",
            include_str!("../shaders/grid.wgsl"),
            &[],
            surface_format,
            GridUniforms {
                proj: matrix,
                resolution: [1920.0, 1080.0],
                zoom: 1.0,
                _padding: 0.0,
            },
            wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
        );

        Self {
            grid_pipeline: grid_container,
        }
    }

    pub fn update_projection(
        &self,
        queue: &wgpu::Queue,
        camera: &Camera,
        config: &SurfaceConfiguration,
    ) {
        puffin::profile_function!();
        let width = config.width as f32;
        let height = config.height as f32;
        let matrix = camera.build_matrix(width, height);

        let grid_uniforms = GridUniforms {
            proj: matrix,
            resolution: [width, height],
            zoom: camera.zoom,
            _padding: 0.0,
        };

        queue.write_buffer(
            &self.grid_pipeline.uniform_buffer,
            0,
            bytemuck::bytes_of(&grid_uniforms),
        );
    }

    pub fn render_grid<'rpass>(&'rpass self, rpass: &mut wgpu::RenderPass<'rpass>) {
        rpass.set_pipeline(&self.grid_pipeline.pipeline);
        rpass.set_bind_group(0, &self.grid_pipeline.bind_group, &[]);
        rpass.draw(0..6, 0..1);
    }
}

pub struct RenderPipelineContainer {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl RenderPipelineContainer {
    pub fn new<U: Pod + Zeroable>(
        device: &wgpu::Device,
        label: &str,
        shader_src: &str,
        vertex_buffers: &[wgpu::VertexBufferLayout],
        surface_format: wgpu::TextureFormat,
        initial_uniforms: U,
        visibility: wgpu::ShaderStages,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{}_ub", label)),
            contents: bytemuck::bytes_of(&initial_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{}_bgl", label)),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{}_bg", label)),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_layout", label)),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: vertex_buffers,
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
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }
}
