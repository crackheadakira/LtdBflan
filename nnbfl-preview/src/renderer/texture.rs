use std::collections::HashMap;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tomolib::formats::bntx::{
    Bntx,
    image::{ChannelResolve, decode_texture_rgba_with},
};
use wgpu::util::DeviceExt;

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
