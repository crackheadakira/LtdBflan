use std::collections::HashMap;

use ttf_parser::{Face, GlyphId};

use crate::renderer::texture::TextureCache;

pub const GLYPH_ATLAS_TEXTURE_NAME: &str = "__glyph_atlas";
const PADDING: u32 = 1;

#[derive(Debug, Clone, Copy, Default)]
pub struct AtlasGlyph {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],

    /// Bitmap size in pixels, at the size it was rasterized.
    pub width: f32,
    pub height: f32,

    /// Offset from the pen position to the bitmap's left/bottom edge.
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct GlyphKey {
    font_id: usize,
    glyph_index: u16,
    size_bits: u32,
}

#[derive(Default)]
pub struct GlyphAtlas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    glyphs: HashMap<GlyphKey, AtlasGlyph>,
    cursor_x: u32,
    cursor_y: u32,
    shelf_height: u32,
    dirty: bool,

    max_used_x: u32,
    max_used_y: u32,
}

impl GlyphAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
            glyphs: HashMap::new(),
            cursor_x: PADDING,
            cursor_y: PADDING,
            shelf_height: 0,
            dirty: true,
            max_used_x: 0,
            max_used_y: 0,
        }
    }

    pub fn used_bounds(&self) -> (u32, u32) {
        (self.max_used_x, self.max_used_y)
    }

    pub fn utilization_percentage(&self) -> f32 {
        let total_pixels = (self.width * self.height) as f32;
        let used_pixels = (self.max_used_x * self.max_used_y) as f32;

        if total_pixels == 0.0 {
            0.0
        } else {
            (used_pixels / total_pixels) * 100.0
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn glyph(
        &mut self,
        font: &FontFace,
        font_id: usize,
        glyph_index: GlyphId,
        size_px: f32,
    ) -> Option<AtlasGlyph> {
        puffin::profile_function!();
        let key = GlyphKey {
            font_id,
            glyph_index: glyph_index.0,
            size_bits: size_px.to_bits(),
        };

        if let Some(existing) = self.glyphs.get(&key) {
            return Some(*existing);
        }

        let (metrics, bitmap) = font.rasterize_glyph(glyph_index, size_px);

        let entry = if metrics.width == 0 || metrics.height == 0 {
            AtlasGlyph {
                bearing_x: metrics.xmin as f32,
                bearing_y: metrics.ymin as f32,
                advance: metrics.advance_width,
                ..Default::default()
            }
        } else {
            let (x, y) = self.allocate(metrics.width as u32, metrics.height as u32)?;
            self.blit(&bitmap, metrics.width as u32, metrics.height as u32, x, y);

            AtlasGlyph {
                uv_min: [x as f32 / self.width as f32, y as f32 / self.height as f32],
                uv_max: [
                    (x + metrics.width as u32) as f32 / self.width as f32,
                    (y + metrics.height as u32) as f32 / self.height as f32,
                ],
                width: metrics.width as f32,
                height: metrics.height as f32,
                bearing_x: metrics.xmin as f32,
                bearing_y: metrics.ymin as f32,
                advance: metrics.advance_width,
            }
        };

        self.glyphs.insert(key, entry);
        Some(entry)
    }

    fn allocate(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        puffin::profile_function!();
        if self.cursor_x + w + PADDING > self.width {
            self.cursor_x = PADDING;
            self.cursor_y += self.shelf_height + PADDING;
            self.shelf_height = 0;
        }

        if self.cursor_y + h + PADDING > self.height {
            log::warn!(
                "GlyphAtlas: out of space ({}x{}) placing a {w}x{h} glyph - dropping it",
                self.width,
                self.height
            );
            return None;
        }

        let pos = (self.cursor_x, self.cursor_y);
        self.cursor_x += w + PADDING;
        self.shelf_height = self.shelf_height.max(h);

        self.max_used_x = self.max_used_x.max(pos.0 + w);
        self.max_used_y = self.max_used_y.max(pos.1 + h);

        Some(pos)
    }

    fn blit(&mut self, bitmap: &[u8], w: u32, h: u32, x: u32, y: u32) {
        puffin::profile_function!();
        for row in 0..h {
            let src_start = (row * w) as usize;
            let dst_start = ((y + row) * self.width + x) as usize;

            self.pixels[dst_start..dst_start + w as usize]
                .copy_from_slice(&bitmap[src_start..src_start + w as usize]);
        }

        self.dirty = true;
    }

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, cache: &mut TextureCache) {
        puffin::profile_function!();

        if !self.dirty {
            return;
        }

        let rgba: Vec<u8> = self
            .pixels
            .iter()
            .flat_map(|&coverage| [255, 255, 255, coverage])
            .collect();

        cache.insert_rgba(
            device,
            queue,
            GLYPH_ATLAS_TEXTURE_NAME,
            &rgba,
            self.width,
            self.height,
        );

        self.dirty = false;
    }
}

pub struct FontFace {
    face_index: u32,
    fontdue_font: fontdue::Font,
    pub units_per_em: f32,
    pub ascender: f32,
    pub descender: f32,
    pub line_gap: f32,
}

impl FontFace {
    pub fn load(data: Vec<u8>, face_index: u32) -> Option<Self> {
        puffin::profile_function!();

        let face = Face::parse(&data, face_index).ok()?;
        let units_per_em = face.units_per_em() as f32;
        let ascender = face.ascender() as f32;
        let descender = face.descender() as f32;
        let line_gap = face.line_gap() as f32;

        let fontdue_font = fontdue::Font::from_bytes(
            data.clone(),
            fontdue::FontSettings {
                collection_index: face_index,

                ..Default::default()
            },
        )
        .map_err(|e| log::error!("fontdue failed to load font: {e}"))
        .ok()?;

        Some(Self {
            face_index,
            fontdue_font,
            units_per_em,
            ascender,
            descender,
            line_gap,
        })
    }

    pub fn rasterize_glyph(&self, glyph_id: GlyphId, size_px: f32) -> (fontdue::Metrics, Vec<u8>) {
        puffin::profile_function!();
        self.fontdue_font.rasterize_indexed(glyph_id.0, size_px)
    }

    pub fn glyph_id_for_char(&self, ch: char) -> Option<GlyphId> {
        let idx = self.fontdue_font.lookup_glyph_index(ch);

        if idx == 0 { None } else { Some(GlyphId(idx)) }
    }

    pub fn scale_factor(&self, size_px: f32) -> f32 {
        size_px / self.units_per_em
    }

    pub fn advance_width(&self, glyph_id: GlyphId, size_px: f32) -> f32 {
        puffin::profile_function!();

        let metrics = self.fontdue_font.metrics_indexed(glyph_id.0, size_px);
        metrics.advance_width
    }

    pub fn kerning(&self, left: GlyphId, right: GlyphId, size_px: f32) -> f32 {
        puffin::profile_function!();

        self.fontdue_font
            .horizontal_kern_indexed(left.0, right.0, size_px)
            .unwrap_or(0.0)
    }
}

#[derive(Default)]
pub struct FontCache {
    faces: Vec<FontFace>,
    name_to_id: HashMap<String, usize>,
}

impl FontCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_load(&mut self, font_name: &str) -> Option<(usize, &FontFace)> {
        puffin::profile_function!();

        if !self.name_to_id.contains_key(font_name) {
            let bytes = Self::load_placeholder_bytes(font_name)?;
            let face = FontFace::load(bytes, 0)?;

            let id = self.faces.len();
            self.faces.push(face);
            self.name_to_id.insert(font_name.to_string(), id);
        }

        let id = self.name_to_id[font_name];
        Some((id, &self.faces[id]))
    }

    fn load_placeholder_bytes(font_name: &str) -> Option<Vec<u8>> {
        puffin::profile_function!();
        crate::chinese_font::load_chinese_font()
            .map(|data| data.font.into_owned())
            .map_err(|e| {
                log::warn!("FontCache: failed to load placeholder font for '{font_name}': {e}")
            })
            .ok()
    }
}

pub struct GlyphData {
    pub fonts: FontCache,
    pub atlas: GlyphAtlas,
}

impl GlyphData {
    pub fn new() -> Self {
        Self {
            fonts: FontCache::new(),
            atlas: GlyphAtlas::new(1024, 1024),
        }
    }
}
