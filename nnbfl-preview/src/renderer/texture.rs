use std::collections::HashMap;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tomolib::formats::bntx::{
    Bntx,
    image::{ChannelResolve, decode_texture_rgba_with},
};
use wgpu::util::DeviceExt;

pub struct TexturePreviewData {
    pub pipeline: TexturePreviewPipeline,
    pub bind_groups: HashMap<String, wgpu::BindGroup>,
}

pub struct TexturePreviewPipeline {
    pipeline: wgpu::RenderPipeline,
    pub sampler: wgpu::Sampler,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl TexturePreviewPipeline {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("texture_preview_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/preview.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_preview_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("texture_preview_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("texture_preview_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            sampler,
            bind_group_layout,
        }
    }
}

pub struct PreviewCallback {
    pub texture_name: String,
}

impl egui_wgpu::CallbackTrait for PreviewCallback {
    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(preview_data) = resources.get::<TexturePreviewData>() {
            if let Some(bind_group) = preview_data.bind_groups.get(&self.texture_name) {
                let rect = info.viewport;
                let ppp = info.pixels_per_point;

                let x = rect.min.x * ppp;
                let y = rect.min.y * ppp;
                let width = rect.width() * ppp;
                let height = rect.height() * ppp;

                render_pass.set_viewport(x, y, width, height, 0.0, 1.0);

                render_pass.set_pipeline(&preview_data.pipeline.pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);

                render_pass.draw(0..3, 0..1);
            }
        }
    }
}

pub struct GpuTexture {
    pub _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

pub struct TextureCache {
    pub textures: HashMap<String, GpuTexture>,
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub fn load_from_bntx(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bntx: &Bntx) {
        let all_exist = !bntx.textures.is_empty()
            && bntx
                .textures
                .iter()
                .all(|tex| self.textures.contains_key(&tex.name));

        if all_exist {
            log::debug!(
                "TextureCache: skipping BNTX payload processing. All {} textures are already loaded.",
                bntx.textures.len()
            );

            return;
        }

        log::info!(
            "TextureCache: processing {} textures...",
            bntx.textures.len()
        );

        let decoded_textures: Vec<_> = bntx
            .textures
            .par_iter()
            .filter(|tex| !self.textures.contains_key(&tex.name))
            .map(|tex| {
                let res = decode_texture_rgba_with(tex, 0, ChannelResolve::Resolved);
                (&tex.name, res)
            })
            .collect();

        for (name, result) in decoded_textures {
            match result {
                Ok(rgba) => {
                    let gpu_tex =
                        upload_rgba(device, queue, &rgba.data, rgba.width, rgba.height, name);

                    self.textures.insert(name.clone(), gpu_tex);
                }
                Err(e) => {
                    log::warn!("TextureCache: failed to decode '{name}': {e}");
                }
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&GpuTexture> {
        self.textures.get(name)
    }

    pub fn clear(&mut self) {
        for (_, gpu_tex) in self.textures.drain() {
            gpu_tex._texture.destroy();
        }
    }
}

fn upload_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[u8],
    width: u32,
    height: u32,
    label: &str,
) -> GpuTexture {
    let _texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
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
        data,
    );

    let view = _texture.create_view(&wgpu::TextureViewDescriptor::default());

    GpuTexture {
        _texture,
        view,
        width,
        height,
    }
}
