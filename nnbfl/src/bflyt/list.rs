use std::cell::Cell;

use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};

use crate::{
    bflyt::flags::{TexFilter, TexWrapMode},
    core::{BitPackable, Cursor, FormatError, Placeholder32, ReadWriteable, Writer},
    ui2d::types::{Color4f, Color4u8, Vector2f},
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Layout {
    pub is_centered: bool,
    pub width: f32,
    pub height: f32,
    pub parts_width: f32,
    pub parts_height: f32,
    pub name: String,
}

impl ReadWriteable for Layout {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let is_centered = cursor.read_u8()? != 0;
        let _reserve0 = cursor.read_u8()?;
        let _reserve1 = cursor.read_u16()?;

        Ok(Self {
            is_centered,
            width: cursor.read_f32()?,
            height: cursor.read_f32()?,
            parts_width: cursor.read_f32()?,
            parts_height: cursor.read_f32()?,
            name: cursor.read_null_terminated_string()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.write_u8(self.is_centered.into());
        writer.write_u8(0);
        writer.write_u16(0);
        writer.write_f32(self.width);
        writer.write_f32(self.height);
        writer.write_f32(self.parts_width);
        writer.write_f32(self.parts_height);
        writer.write_null_terminated_string(&self.name);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// A list of textures that the bflyt uses.
pub struct TextureList {
    /// An array containing the names of the textures that can be found within the bntx in the same SARC.
    pub textures: Vec<String>,
}

impl ReadWriteable for TextureList {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let texture_count = cursor.read_u16()?;
        let _reserve0 = cursor.read_u16()?;

        let offsets_start = cursor.pos;
        let mut offsets = Vec::with_capacity(texture_count as usize);
        for _ in 0..texture_count {
            offsets.push(cursor.read_u32()?);
        }

        let mut textures = Vec::with_capacity(offsets.len());
        for offset in offsets {
            cursor.seek(offsets_start + offset as usize)?;
            textures.push(cursor.read_null_terminated_string()?);
        }

        Ok(Self { textures })
    }

    fn write(&self, writer: &mut Writer) {
        writer.write_u16(self.textures.len() as u16);
        writer.write_u16(0);

        let offsets_start = writer.pos();
        let mut offset_placeholders = Vec::with_capacity(self.textures.len());
        for _ in &self.textures {
            offset_placeholders.push(writer.write_placeholder_u32());
        }

        for (i, name) in self.textures.iter().enumerate() {
            let offset = writer.pos() - offsets_start;
            writer.patch_u32(offset_placeholders[i], offset as u32);
            writer.write_null_terminated_string(name);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FontList {
    pub fonts: Vec<String>,
}

impl ReadWriteable for FontList {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let font_count = cursor.read_u16()?;
        let _reserve0 = cursor.read_u16()?;

        let offsets_start = cursor.pos;
        let mut offsets = Vec::with_capacity(font_count as usize);
        for _ in 0..font_count {
            offsets.push(cursor.read_u32()?);
        }

        let mut fonts = Vec::with_capacity(offsets.len());
        for offset in offsets {
            cursor.seek(offsets_start + offset as usize)?;
            fonts.push(cursor.read_null_terminated_string()?);
        }

        Ok(Self { fonts })
    }

    fn write(&self, writer: &mut Writer) {
        writer.write_u16(self.fonts.len() as u16);
        writer.write_u16(0);

        let offsets_start = writer.pos();
        let mut offset_placeholders = Vec::with_capacity(self.fonts.len());
        for _ in &self.fonts {
            offset_placeholders.push(writer.write_placeholder_u32());
        }

        for (i, name) in self.fonts.iter().enumerate() {
            let offset = writer.pos() - offsets_start;
            writer.patch_u32(offset_placeholders[i], offset as u32);
            writer.write_null_terminated_string(name);
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct MaterialTextureOptions {
    pub wrap_mode: TexWrapMode,
    pub filter: TexFilter,
}

impl BitPackable<u8> for MaterialTextureOptions {
    fn decode(raw: u8) -> Self {
        Self {
            wrap_mode: (raw & 0x3).into(),
            filter: ((raw >> 2) & 0x3).into(),
        }
    }

    fn encode(&self) -> u8 {
        (self.wrap_mode as u8 & 0x3) | ((self.filter as u8 & 0x3) << 2)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialTextureExtension {
    pub is_capture_texture: bool,
    pub is_vector_texture: bool,
}

impl BitPackable<u32> for MaterialTextureExtension {
    fn decode(raw: u32) -> Self {
        Self {
            is_capture_texture: (raw & 0x1) != 0,
            is_vector_texture: ((raw >> 1) & 0x1) != 0,
        }
    }

    fn encode(&self) -> u32 {
        (self.is_capture_texture as u32 & 0x1) | ((self.is_vector_texture as u32 & 0x1) << 1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialTextureMap {
    #[serde(skip)]
    pub texture_index: Cell<u16>,
    pub texture_name: String,
    pub u_options: MaterialTextureOptions,
    pub v_options: MaterialTextureOptions,
}

impl ReadWriteable for MaterialTextureMap {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            texture_index: Cell::new(c.read_u16()?),
            texture_name: String::new(),
            u_options: MaterialTextureOptions::decode(c.read_u8()?),
            v_options: MaterialTextureOptions::decode(c.read_u8()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u16(self.texture_index.get());
        w.write_u8(self.u_options.encode());
        w.write_u8(self.v_options.encode());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialTextureSrt {
    pub translate_u: f32,
    pub translate_v: f32,
    pub rotate: f32,
    pub scale_u: f32,
    pub scale_v: f32,
}

impl ReadWriteable for MaterialTextureSrt {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            translate_u: c.read_f32()?,
            translate_v: c.read_f32()?,
            rotate: c.read_f32()?,
            scale_u: c.read_f32()?,
            scale_v: c.read_f32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_f32(self.translate_u);
        w.write_f32(self.translate_v);
        w.write_f32(self.rotate);
        w.write_f32(self.scale_u);
        w.write_f32(self.scale_v);
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum TexGenSrc {
    #[default]
    Tex0,
    Tex1,
    Tex2,
    OrthogonalProjection,
    PaneBasedOrthogonalProjection,
    PerspectiveProjection,
    PaneBasedPerspectiveProjection,
    BrickRepeat,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialTexCoordGen {
    pub tex_gen_source: TexGenSrc,
}

impl ReadWriteable for MaterialTexCoordGen {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        let _reserve0 = c.read_u8()?;
        let tex_gen_source = c.read_u8()?.into();
        let _reserve1 = c.read_u16()?;
        let _reserve2 = c.read_u32()?;
        let _reserve3 = c.read_u64()?;

        Ok(Self { tex_gen_source })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(0);
        w.write_u8(self.tex_gen_source.into());
        w.write_u16(0);
        w.write_u32(0);
        w.write_u32(0);
        w.write_u32(0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialTevCombiner {
    pub rgb_mode: CombinerTevMode,
    pub alpha_mode: CombinerTevMode,
}

impl ReadWriteable for MaterialTevCombiner {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        let rgb_mode = c.read_u8()?.into();
        let alpha_mode = c.read_u8()?.into();
        c.read_u16()?;

        Ok(Self {
            rgb_mode,
            alpha_mode,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.rgb_mode.into());
        w.write_u8(self.alpha_mode.into());
        w.write_u16(0);
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum AlphaCompare {
    Never,
    Less,
    LessThanEqual,
    Equal,
    NeverEqual,
    GreaterThanEqual,
    Greater,
    #[default]
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialAlphaCompare {
    pub compare: AlphaCompare,
    pub alpha_compare_ref_value: f32,
}

impl ReadWriteable for MaterialAlphaCompare {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        let compare = c.read_u8()?.into();
        let _reserve0 = c.read_u8()?;
        let _reserve1 = c.read_u16()?;
        let alpha_compare_ref_value = c.read_f32()?;
        Ok(Self {
            compare,
            alpha_compare_ref_value,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.compare.into());
        w.write_u8(0);
        w.write_u16(0);
        w.write_f32(self.alpha_compare_ref_value);
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum BlendFactor {
    #[default]
    V0,
    V1_0,
    DstColor,
    InvDstColor,
    SrcAlpha,
    InvSrcAlpha,
    DstAlpha,
    InvDstAlpha,
    SrcColor,
    InvSrcColor,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum LogicOp {
    #[default]
    Invalid,

    NoOp,
    Clear,
    Set,
    Copy,
    InvCopy,
    Inv,
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Equiv,
    RevAnd,
    InvAnd,
    RevOr,
    InvOr,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum BlendOp {
    #[default]
    Invalid,

    Add,
    Subtract,
    ReverseSubtract,
    SelectMin,
    SelectMax,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MaterialBlendMode {
    #[default]
    None,
    Blend {
        blend_op: BlendOp,
        function_source: BlendFactor,
        function_destination: BlendFactor,
    },
    Logic {
        logic_op: LogicOp,
    },
}

impl ReadWriteable for MaterialBlendMode {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let blend_op_raw = cursor.read_u8()?;
        let src_factor_raw = cursor.read_u8()?;
        let dst_factor_raw = cursor.read_u8()?;
        let logic_op_raw = cursor.read_u8()?;

        let out = if logic_op_raw != 0 && blend_op_raw == 0 {
            Self::Logic {
                logic_op: logic_op_raw.into(),
            }
        } else if blend_op_raw != 0 {
            Self::Blend {
                blend_op: blend_op_raw.into(),
                function_source: src_factor_raw.into(),
                function_destination: dst_factor_raw.into(),
            }
        } else {
            Self::None
        };

        Ok(out)
    }

    fn write(&self, w: &mut Writer) {
        match self {
            Self::None => {
                w.write_bytes(&[0, 0, 0, 0]);
            }
            Self::Blend {
                blend_op,
                function_source,
                function_destination,
            } => {
                w.write_u8((*blend_op).into());
                w.write_u8((*function_source).into());
                w.write_u8((*function_destination).into());
                w.write_u8(0);
            }
            Self::Logic { logic_op } => {
                w.write_bytes(&[0, 0, 0]);
                w.write_u8((*logic_op).into());
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialIndirectMatrix {
    pub rotation: f32,
    pub scale: Vector2f,
}

impl ReadWriteable for MaterialIndirectMatrix {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            rotation: c.read_f32()?,
            scale: Vector2f::parse(c)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_f32(self.rotation);
        self.scale.write(w);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct MaterialProjectionTexGenFlags {
    pub fitting_layout_size: bool,
    pub fitting_pane_size: bool,
    pub adjust_projection_scale_rotate: bool,
}

impl BitPackable<u32> for MaterialProjectionTexGenFlags {
    fn decode(raw: u32) -> Self {
        Self {
            fitting_layout_size: (raw & 0x1) != 0,
            fitting_pane_size: ((raw >> 1) & 0x1) != 0,
            adjust_projection_scale_rotate: ((raw >> 2) & 0x1) != 0,
        }
    }

    fn encode(&self) -> u32 {
        (self.fitting_layout_size as u32)
            | ((self.fitting_pane_size as u32) << 1)
            | ((self.adjust_projection_scale_rotate as u32) << 2)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialProjectionTexGen {
    pub translation: Vector2f,
    pub scale: Vector2f,
    pub flags: MaterialProjectionTexGenFlags,
}

impl ReadWriteable for MaterialProjectionTexGen {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        let s = Self {
            translation: Vector2f::parse(c)?,
            scale: Vector2f::parse(c)?,
            flags: MaterialProjectionTexGenFlags::decode(c.read_u32()?),
        };

        Ok(s)
    }

    fn write(&self, w: &mut Writer) {
        self.translation.write(w);
        self.scale.write(w);

        w.write_u32(self.flags.encode());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialFontShadowColor {
    pub black_color: Color4u8,
    pub white_color: Color4u8,
}

impl ReadWriteable for MaterialFontShadowColor {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            black_color: Color4u8::parse(c)?,
            white_color: Color4u8::parse(c)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.black_color.write(w);
        self.white_color.write(w);
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    FromPrimitive,
    IntoPrimitive,
    Hash,
    Default,
)]
#[repr(u8)]
pub enum TevSource {
    Primary = 0,
    Texture0 = 3,
    Texture1 = 4,
    Texture2 = 5,
    Texture3 = 6,
    Register = 13,
    #[default]
    Constant = 14,
    Previous = 15,
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum TevScale {
    #[default]
    V1,
    V2,
    V4,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    FromPrimitive,
    IntoPrimitive,
    Hash,
    Default,
)]
#[repr(u8)]
pub enum CombinerTevMode {
    #[default]
    Replace,
    Modulate,
    Add,
    AddSigned,
    Interpolate,
    Subtract,
    AddMultiply,
    MultiplyAdd,
    Overlay,
    Lighten,
    Darken,
    Indirect,
    BlendIndirect,
    EachIndirect,
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, IntoPrimitive, FromPrimitive, Default,
)]
#[repr(u8)]
pub enum TevColorOp {
    #[default]
    Rgb,
    InvRgb,
    Alpha,
    InvAlpha,
    Rrr,
    InvRrr,
    Ggg = 8,
    InvGgg = 9,
    Bbb = 12,
    InvBbb = 13,
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, IntoPrimitive, FromPrimitive, Default,
)]
#[repr(u8)]
pub enum TevAlphaOp {
    #[default]
    Alpha,
    InvAlpha,
    R,
    InvR,
    G,
    InvG,
    B,
    InvB,
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, IntoPrimitive, FromPrimitive, Default,
)]
#[repr(u8)]
pub enum TevKonstSel {
    BlackColor,
    #[default]
    WhiteColor,
    K0,
    K1,
    K2,
    K3,
    K4,
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, IntoPrimitive, FromPrimitive, Default,
)]
#[repr(u8)]
pub enum DetailedCombinerStageMode {
    #[default]
    Replace,
    Modulate,
    Add,
    AddSigned,
    Interpolate,
    Subtract,
    AddMult = 8,
    MultiplicateAdd,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DetailedCombinerColorStageConfig {
    pub sources: [TevSource; 3],
    pub operands: [TevColorOp; 3],
    pub mode: DetailedCombinerStageMode,
    pub scale: TevScale,
    pub copy_reg: bool,
    pub konst_sel: TevKonstSel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DetailedCombinerAlphaStageConfig {
    pub sources: [TevSource; 3],
    pub operands: [TevAlphaOp; 3],
    pub mode: DetailedCombinerStageMode,
    pub scale: TevScale,
    pub copy_reg: bool,
    pub konst_sel: TevKonstSel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialDetailedCombinerEntry {
    pub color_config: DetailedCombinerColorStageConfig,
    pub alpha_config: DetailedCombinerAlphaStageConfig,
}

impl ReadWriteable for MaterialDetailedCombinerEntry {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        let color_flags = c.read_u32()?;
        let alpha_flags = c.read_u32()?;
        let constant_selectors = c.read_u32()?;
        let _source_counts = c.read_u32()?;

        let color_config = DetailedCombinerColorStageConfig {
            sources: [
                ((color_flags & 0xF) as u8).into(),
                (((color_flags >> 4) & 0xF) as u8).into(),
                (((color_flags >> 8) & 0xF) as u8).into(),
            ],
            operands: [
                (((color_flags >> 12) & 0xF) as u8).into(),
                (((color_flags >> 16) & 0xF) as u8).into(),
                (((color_flags >> 20) & 0xF) as u8).into(),
            ],
            mode: (((color_flags >> 24) & 0xF) as u8).into(),
            scale: (((color_flags >> 28) & 0x3) as u8).into(),
            copy_reg: ((color_flags >> 30) & 0x1) as u8 != 0,
            konst_sel: ((constant_selectors & 0xF) as u8).into(),
        };

        let alpha_config = DetailedCombinerAlphaStageConfig {
            sources: [
                ((alpha_flags & 0xF) as u8).into(),
                (((alpha_flags >> 4) & 0xF) as u8).into(),
                (((alpha_flags >> 8) & 0xF) as u8).into(),
            ],
            operands: [
                (((alpha_flags >> 12) & 0xF) as u8).into(),
                (((alpha_flags >> 16) & 0xF) as u8).into(),
                (((alpha_flags >> 20) & 0xF) as u8).into(),
            ],
            mode: (((alpha_flags >> 24) & 0xF) as u8).into(),
            scale: (((alpha_flags >> 28) & 0x3) as u8).into(),
            copy_reg: ((alpha_flags >> 30) & 0x1) as u8 != 0,
            konst_sel: (((constant_selectors >> 4) & 0xF) as u8).into(),
        };

        Ok(Self {
            color_config,
            alpha_config,
        })
    }

    fn write(&self, w: &mut Writer) {
        let (color_flags, alpha_flags, constant_selectors, source_counts) = self.pack_flags();

        w.write_u32(color_flags);
        w.write_u32(alpha_flags);
        w.write_u32(constant_selectors);
        w.write_u32(source_counts);
    }
}

impl MaterialDetailedCombinerEntry {
    fn get_source_count(mode: DetailedCombinerStageMode) -> u32 {
        match mode {
            DetailedCombinerStageMode::Replace => 1,
            DetailedCombinerStageMode::Modulate
            | DetailedCombinerStageMode::Add
            | DetailedCombinerStageMode::AddSigned
            | DetailedCombinerStageMode::Subtract => 2,
            DetailedCombinerStageMode::Interpolate
            | DetailedCombinerStageMode::AddMult
            | DetailedCombinerStageMode::MultiplicateAdd => 3,
        }
    }

    pub fn pack_flags(&self) -> (u32, u32, u32, u32) {
        let mut color_flags = 0u32;
        color_flags |= self.color_config.sources[0] as u32 & 0xF;
        color_flags |= (self.color_config.sources[1] as u32 & 0xF) << 4;
        color_flags |= (self.color_config.sources[2] as u32 & 0xF) << 8;
        color_flags |= (self.color_config.operands[0] as u32 & 0xF) << 12;
        color_flags |= (self.color_config.operands[1] as u32 & 0xF) << 16;
        color_flags |= (self.color_config.operands[2] as u32 & 0xF) << 20;
        color_flags |= (self.color_config.mode as u32 & 0xF) << 24;
        color_flags |= (self.color_config.scale as u32 & 0x3) << 28;
        color_flags |= (self.color_config.copy_reg as u32 & 0x1) << 30;

        let mut alpha_flags = 0u32;
        alpha_flags |= self.alpha_config.sources[0] as u32 & 0xF;
        alpha_flags |= (self.alpha_config.sources[1] as u32 & 0xF) << 4;
        alpha_flags |= (self.alpha_config.sources[2] as u32 & 0xF) << 8;
        alpha_flags |= (self.alpha_config.operands[0] as u32 & 0xF) << 12;
        alpha_flags |= (self.alpha_config.operands[1] as u32 & 0xF) << 16;
        alpha_flags |= (self.alpha_config.operands[2] as u32 & 0xF) << 20;
        alpha_flags |= (self.alpha_config.mode as u32 & 0xF) << 24;
        alpha_flags |= (self.alpha_config.scale as u32 & 0x3) << 28;
        alpha_flags |= (self.alpha_config.copy_reg as u32 & 0x1) << 30;

        let mut constant_selectors = 0u32;
        constant_selectors |= self.color_config.konst_sel as u32 & 0xF;
        constant_selectors |= (self.alpha_config.konst_sel as u32 & 0xF) << 4;

        let mut source_counts = 0u32;
        source_counts |= Self::get_source_count(self.color_config.mode) & 0xF;
        source_counts |= (Self::get_source_count(self.alpha_config.mode) & 0xF) << 4;

        (color_flags, alpha_flags, constant_selectors, source_counts)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialDetailedCombiner {
    pub value: i32,

    pub color1: Color4u8,
    pub color2: Color4u8,
    pub color3: Color4u8,
    pub color4: Color4u8,
    pub color5: Color4u8,
    pub stage_flags: u32,

    pub entries: Vec<MaterialDetailedCombinerEntry>,
}

impl MaterialDetailedCombiner {
    pub fn parse(c: &mut Cursor, count: u8) -> Result<Self, FormatError> {
        let mut combiner = Self {
            value: c.read_i32()?,
            color1: Color4u8::parse(c)?,
            color2: Color4u8::parse(c)?,
            color3: Color4u8::parse(c)?,
            color4: Color4u8::parse(c)?,
            color5: Color4u8::parse(c)?,
            stage_flags: c.read_u32()?,
            entries: Vec::with_capacity(count as usize),
        };

        for _ in 0..count {
            let entry = MaterialDetailedCombinerEntry::parse(c)?;
            combiner.entries.push(entry);
        }

        Ok(combiner)
    }

    pub fn serialize(&self, w: &mut Writer) {
        w.write_i32(self.value);
        self.color1.write(w);
        self.color2.write(w);
        self.color3.write(w);
        self.color4.write(w);
        self.color5.write(w);
        w.write_u32(self.stage_flags);

        for entry in &self.entries {
            entry.write(w);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialUserCombiner {
    pub name: String,
    pub reserve: [u32; 5],
}

impl ReadWriteable for MaterialUserCombiner {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        let name = c.read_fixed_string(0x60)?;
        let mut reserve = [0u32; 5];

        for val in &mut reserve {
            *val = c.read_u32()?;
        }

        Ok(Self { name, reserve })
    }

    fn write(&self, w: &mut Writer) {
        w.write_fixed_string(&self.name, 0x60);

        for val in &self.reserve {
            w.write_u32(*val);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialVectorTextureInfo {
    pub time: f32,
    pub color: Color4u8,
}

impl ReadWriteable for MaterialVectorTextureInfo {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        let time = c.read_f32()?;
        let color = Color4u8::parse(c)?;
        let _reserve0 = c.read_u64();

        Ok(Self { time, color })
    }

    fn write(&self, w: &mut Writer) {
        w.write_f32(self.time);
        self.color.write(w);
        w.write_u64(0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialBrickRepeatShaderInfo {
    pub data: Vec<u8>,
}

impl ReadWriteable for MaterialBrickRepeatShaderInfo {
    fn parse(c: &mut Cursor) -> Result<Self, FormatError> {
        let data = c.read_bytes(0x58)?.to_vec();

        Ok(Self { data })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bytes(&self.data);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct MaterialInfo {
    pub tex_map_count: u8,
    pub tex_srt_count: u8,
    pub tex_coord_gen_count: u8,
    pub tev_combiner_count: u8,
    pub has_alpha_compare: bool,
    pub has_color_blend_mode: bool,
    pub use_texture_only: bool,
    pub has_separate_blend_mode: bool,
    pub has_indirect_matrix: bool,
    pub projection_tex_gen_count: u8,
    pub has_font_shadow_parameter: bool,
    pub use_thresholding_alpha_interpolation: bool,
    pub use_detailed_combiner: bool,
    pub has_user_combiner: bool,
    pub has_texture_extensions: u8,
    pub vector_texture_info_count: u8,
    pub brick_repeat_shader_info_count: u8,
}

impl BitPackable<u32> for MaterialInfo {
    fn decode(raw: u32) -> Self {
        Self {
            tex_map_count: (raw & 0x3) as u8,
            tex_srt_count: ((raw >> 2) & 0x3) as u8,
            tex_coord_gen_count: ((raw >> 4) & 0x3) as u8,
            tev_combiner_count: ((raw >> 6) & 0x7) as u8,
            has_alpha_compare: ((raw >> 9) & 0x1) != 0,
            has_color_blend_mode: ((raw >> 10) & 0x1) != 0,
            use_texture_only: ((raw >> 11) & 0x1) != 0,
            has_separate_blend_mode: ((raw >> 12) & 0x1) != 0,
            has_indirect_matrix: ((raw >> 14) & 0x1) != 0,
            projection_tex_gen_count: ((raw >> 15) & 0x3) as u8,
            has_font_shadow_parameter: ((raw >> 17) & 0x1) != 0,
            use_thresholding_alpha_interpolation: ((raw >> 18) & 0x1) != 0,
            use_detailed_combiner: ((raw >> 19) & 0x1) != 0,
            has_user_combiner: ((raw >> 20) & 0x1) != 0,
            has_texture_extensions: ((raw >> 21) & 0x1) as u8,
            vector_texture_info_count: ((raw >> 22) & 0x3) as u8,
            brick_repeat_shader_info_count: ((raw >> 24) & 0x3) as u8,
        }
    }

    fn encode(&self) -> u32 {
        ((self.tex_map_count & 0x3) as u32)
            | (((self.tex_srt_count & 0x3) as u32) << 2)
            | (((self.tex_coord_gen_count & 0x3) as u32) << 4)
            | (((self.tev_combiner_count & 0x7) as u32) << 6)
            | ((self.has_alpha_compare as u32) << 9)
            | ((self.has_color_blend_mode as u32) << 10)
            | ((self.use_texture_only as u32) << 11)
            | ((self.has_separate_blend_mode as u32) << 12)
            | ((self.has_indirect_matrix as u32) << 14)
            | (((self.projection_tex_gen_count & 0x3) as u32) << 15)
            | ((self.has_font_shadow_parameter as u32) << 17)
            | ((self.use_thresholding_alpha_interpolation as u32) << 18)
            | ((self.use_detailed_combiner as u32) << 19)
            | ((self.has_user_combiner as u32) << 20)
            | (((self.has_texture_extensions & 0x1) as u32) << 21)
            | (((self.vector_texture_info_count & 0x3) as u32) << 22)
            | (((self.brick_repeat_shader_info_count & 0x3) as u32) << 24)
    }
}

pub const MATERIAL_NAME_LEN: usize = 0x1c;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialColorEntry {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub color_u8: Option<Color4u8>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub color_f32: Option<Color4f>,
}

impl Default for MaterialColorEntry {
    fn default() -> Self {
        Self {
            color_u8: Some(Color4u8::default()),
            color_f32: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialColorInterpolation {
    pub black_color: MaterialColorEntry,
    pub white_color: MaterialColorEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Material {
    pub material_name: String,

    pub interpolation_colors: MaterialColorInterpolation,

    pub tex_maps: Vec<MaterialTextureMap>,
    pub tex_extensions: Vec<MaterialTextureExtension>,
    pub tex_srts: Vec<MaterialTextureSrt>,
    pub tex_coord_gens: Vec<MaterialTexCoordGen>,
    pub tev_combiners: Vec<MaterialTevCombiner>,
    pub alpha_compare: Option<MaterialAlphaCompare>,

    pub blend_mode: Option<MaterialBlendMode>,

    pub blend_mode_alpha: Option<MaterialBlendMode>,

    pub indirect_matrix: Option<MaterialIndirectMatrix>,
    pub projection_tex_gens: Vec<MaterialProjectionTexGen>,
    pub font_shadow_color: Option<MaterialFontShadowColor>,

    pub detailed_combiner: Option<MaterialDetailedCombiner>,

    pub user_combiner: Option<MaterialUserCombiner>,
    pub vector_texture_infos: Vec<MaterialVectorTextureInfo>,
    pub brick_repeat_shader_infos: Vec<MaterialBrickRepeatShaderInfo>,

    pub use_texture_only: bool,
    pub use_thresholding_alpha_interpolation: bool,
}

impl ReadWriteable for Material {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let mat_base = cursor.ctx_section_start::<Self>()?;

        cursor.seek(mat_base)?;
        let material_name = cursor.read_fixed_string(MATERIAL_NAME_LEN)?;
        let material_info = MaterialInfo::decode(cursor.read_u32()?);

        let color_types_byte = cursor.read_u8()?;
        let color_count = cursor.read_u8()?;

        let color_data_base = mat_base + 0x20;
        let mut color_offset_bytes = Vec::with_capacity(color_count as usize);
        for _ in 0..color_count {
            color_offset_bytes.push(cursor.read_u8()?);
        }

        let mut read_color = |index: usize| -> Result<MaterialColorEntry, FormatError> {
            let offset = color_offset_bytes[index];
            let is_float = ((color_types_byte >> index) & 1) != 0;

            let saved = cursor.pos;
            cursor.seek(color_data_base + offset as usize)?;

            let entry = if is_float {
                MaterialColorEntry {
                    color_u8: None,
                    color_f32: Some(Color4f::parse(cursor)?),
                }
            } else {
                MaterialColorEntry {
                    color_u8: Some(Color4u8::parse(cursor)?),
                    color_f32: None,
                }
            };

            cursor.seek(saved)?;
            Ok(entry)
        };

        let interpolation_colors = MaterialColorInterpolation {
            black_color: read_color(0)?,
            white_color: read_color(1)?,
        };

        let color_section_size = {
            let mut max_end = 2 + color_count as usize;
            for (i, &offset) in color_offset_bytes.iter().enumerate() {
                let is_float = ((color_types_byte >> i) & 1) != 0;
                let end = offset as usize + if is_float { 16 } else { 4 };
                if end > max_end {
                    max_end = end;
                }
            }
            max_end
        };

        let after_color = mat_base + 0x20 + color_section_size;

        cursor.seek(after_color)?;

        let mut tex_maps = Vec::with_capacity(material_info.tex_map_count as usize);
        for _ in 0..material_info.tex_map_count {
            tex_maps.push(MaterialTextureMap::parse(cursor)?);
        }

        let mut tex_extensions = Vec::with_capacity(material_info.tex_map_count as usize);
        if material_info.has_texture_extensions != 0 {
            for _ in 0..material_info.tex_map_count {
                tex_extensions.push(MaterialTextureExtension::decode(cursor.read_u32()?));
            }
        }

        let mut tex_srts = Vec::with_capacity(material_info.tex_srt_count as usize);
        for _ in 0..material_info.tex_srt_count {
            tex_srts.push(MaterialTextureSrt::parse(cursor)?);
        }

        let mut tex_coord_gens = Vec::with_capacity(material_info.tex_coord_gen_count as usize);
        for _ in 0..material_info.tex_coord_gen_count {
            tex_coord_gens.push(MaterialTexCoordGen::parse(cursor)?);
        }

        let mut tev_combiners = Vec::with_capacity(material_info.tev_combiner_count as usize);
        for _ in 0..material_info.tev_combiner_count {
            tev_combiners.push(MaterialTevCombiner::parse(cursor)?);
        }

        let alpha_compare = if material_info.has_alpha_compare {
            Some(MaterialAlphaCompare::parse(cursor)?)
        } else {
            None
        };

        let blend_mode = if material_info.has_color_blend_mode {
            Some(MaterialBlendMode::parse(cursor)?)
        } else {
            None
        };

        let blend_mode_alpha = if material_info.has_separate_blend_mode {
            Some(MaterialBlendMode::parse(cursor)?)
        } else {
            None
        };

        let indirect_matrix = if material_info.has_indirect_matrix {
            Some(MaterialIndirectMatrix::parse(cursor)?)
        } else {
            None
        };

        let detailed_combiner = if material_info.use_detailed_combiner {
            Some(MaterialDetailedCombiner::parse(
                cursor,
                material_info.tev_combiner_count,
            )?)
        } else {
            None
        };

        let mut projection_tex_gens =
            Vec::with_capacity(material_info.projection_tex_gen_count as usize);
        for _ in 0..material_info.projection_tex_gen_count {
            projection_tex_gens.push(MaterialProjectionTexGen::parse(cursor)?);
        }

        let font_shadow_color = if material_info.has_font_shadow_parameter {
            Some(MaterialFontShadowColor::parse(cursor)?)
        } else {
            None
        };

        let user_combiner = if material_info.has_user_combiner {
            Some(MaterialUserCombiner::parse(cursor)?)
        } else {
            None
        };

        let mut vector_texture_infos =
            Vec::with_capacity(material_info.vector_texture_info_count as usize);
        for _ in 0..material_info.vector_texture_info_count {
            vector_texture_infos.push(MaterialVectorTextureInfo::parse(cursor)?);
        }

        let mut brick_repeat_shader_infos =
            Vec::with_capacity(material_info.brick_repeat_shader_info_count as usize);
        for _ in 0..material_info.brick_repeat_shader_info_count {
            brick_repeat_shader_infos.push(MaterialBrickRepeatShaderInfo::parse(cursor)?);
        }

        Ok(Self {
            material_name,
            use_texture_only: material_info.use_texture_only,
            use_thresholding_alpha_interpolation: material_info
                .use_thresholding_alpha_interpolation,
            interpolation_colors,
            tex_maps,
            tex_extensions,
            tex_srts,
            tex_coord_gens,
            tev_combiners,
            alpha_compare,
            blend_mode,
            blend_mode_alpha,
            indirect_matrix,
            projection_tex_gens,
            font_shadow_color,
            detailed_combiner,
            user_combiner,
            vector_texture_infos,
            brick_repeat_shader_infos,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.write_fixed_string(&self.material_name, MATERIAL_NAME_LEN);

        let material_info = MaterialInfo {
            tex_map_count: self.tex_maps.len() as u8,
            tex_srt_count: self.tex_srts.len() as u8,
            tex_coord_gen_count: self.tex_coord_gens.len() as u8,
            tev_combiner_count: self.tev_combiners.len() as u8,
            has_alpha_compare: self.alpha_compare.is_some(),
            has_color_blend_mode: self.blend_mode.is_some(),
            has_separate_blend_mode: self.blend_mode_alpha.is_some(),
            has_indirect_matrix: self.indirect_matrix.is_some(),
            projection_tex_gen_count: self.projection_tex_gens.len() as u8,
            has_font_shadow_parameter: self.font_shadow_color.is_some(),
            use_detailed_combiner: self.detailed_combiner.is_some(),
            has_user_combiner: self.user_combiner.is_some(),
            has_texture_extensions: !self.tex_extensions.is_empty() as u8,
            vector_texture_info_count: self.vector_texture_infos.len() as u8,
            brick_repeat_shader_info_count: self.brick_repeat_shader_infos.len() as u8,
            use_texture_only: self.use_texture_only,
            use_thresholding_alpha_interpolation: self.use_thresholding_alpha_interpolation,
        };

        writer.write_u32(material_info.encode());

        let entries = [
            &self.interpolation_colors.black_color,
            &self.interpolation_colors.white_color,
        ];

        let mut color_types_byte: u8 = 0;
        for (i, entry) in entries.iter().enumerate() {
            if entry.color_f32.is_some() {
                color_types_byte |= 1 << i;
            }
        }

        writer.write_u8(color_types_byte);
        writer.write_u8(entries.len() as u8);

        let n = entries.len();
        let mut cumulative_offset = (2 + n) as u8;
        for entry in &entries {
            writer.write_u8(cumulative_offset);
            cumulative_offset += if entry.color_u8.is_some() { 4 } else { 16 };
        }

        for entry in &entries {
            if let Some(c) = &entry.color_u8 {
                c.write(writer);
            } else if let Some(c) = &entry.color_f32 {
                c.write(writer);
            }
        }

        for tm in &self.tex_maps {
            tm.write(writer);
        }

        for ext in &self.tex_extensions {
            writer.write_u32(ext.encode());
        }

        for ts in &self.tex_srts {
            ts.write(writer);
        }

        for tg in &self.tex_coord_gens {
            tg.write(writer);
        }

        for tc in &self.tev_combiners {
            tc.write(writer);
        }

        if let Some(alpha_compare) = &self.alpha_compare {
            alpha_compare.write(writer);
        }

        if let Some(blend_mode) = &self.blend_mode {
            blend_mode.write(writer);
        }

        if let Some(blend_mode) = &self.blend_mode_alpha {
            blend_mode.write(writer);
        }

        if let Some(indirect_matrix) = &self.indirect_matrix {
            indirect_matrix.write(writer);
        }

        if let Some(detailed_combiner) = &self.detailed_combiner {
            detailed_combiner.serialize(writer);
        }

        for pg in &self.projection_tex_gens {
            pg.write(writer);
        }

        if let Some(font_shadow_color) = &self.font_shadow_color {
            font_shadow_color.write(writer);
        }

        if let Some(user_combiner) = &self.user_combiner {
            user_combiner.write(writer);
        }

        for vi in &self.vector_texture_infos {
            vi.write(writer);
        }

        for br in &self.brick_repeat_shader_infos {
            br.write(writer);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialList {
    pub materials: Vec<Material>,
}

impl ReadWriteable for MaterialList {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let mat_list_base = cursor.ctx_section_start::<Self>()?;

        let material_count = cursor.read_u16()?;
        let _reserve0 = cursor.read_u16()?;

        let mut offsets = Vec::with_capacity(material_count as usize);
        for _ in 0..material_count {
            offsets.push(cursor.read_u32()?);
        }

        let saved = cursor.pos;
        let mut materials = Vec::with_capacity(offsets.len());

        for offset in offsets {
            let mat_base = mat_list_base + offset as usize;
            cursor.section_start = Some(mat_base);

            materials.push(Material::parse(cursor)?);

            cursor.section_start = None;
        }

        cursor.seek(saved)?;

        Ok(Self { materials })
    }

    fn write(&self, writer: &mut Writer) {
        let mat_list_base = writer.ctx_section_start::<Self>();
        writer.write_u16(self.materials.len() as u16);
        writer.write_u16(0);

        let mut offset_placeholders = Vec::with_capacity(self.materials.len());
        for _ in &self.materials {
            offset_placeholders.push(writer.write_placeholder_u32());
        }

        for (i, material) in self.materials.iter().enumerate() {
            writer.align(4);
            let offset = writer.pos() - mat_list_base;
            writer.patch_u32(offset_placeholders[i], offset as u32);
            material.write(writer);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaptureTextureFilter {
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaptureTexture {
    pub texture_name: String,
    pub pane_name: String,
    pub clear_color: Color4f,
    pub format_id: i16,
    pub framebuffer_capture_enabled: bool,
    pub capture_only_first_frame: bool,
    pub filters: Vec<CaptureTextureFilter>,
}

impl CaptureTexture {
    pub fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let ctl_base = cursor.ctx_section_start::<Self>()?;
        let texture_name_offset = cursor.read_u32()?;
        let pane_name_offset = cursor.read_u32()?;

        cursor.read_u64()?;

        let clear_color = Color4f::parse(cursor)?;

        let format_id = cursor.read_i16()?;
        let framebuffer_capture_enabled = cursor.read_u8()? != 0;
        let capture_only_first_frame = cursor.read_u8()? != 0;
        let filter_count = cursor.read_u16()?;

        cursor.read_u16()?;

        let mut filters = Vec::with_capacity(filter_count as usize);
        for _ in 0..filter_count {
            cursor.read_u32()?;
            let scale = cursor.read_f32()?;

            filters.push(CaptureTextureFilter { scale });
        }

        let saved = cursor.pos;
        cursor.seek(ctl_base + texture_name_offset as usize)?;
        let texture_name = cursor.read_null_terminated_string()?;
        cursor.seek(ctl_base + pane_name_offset as usize)?;
        let pane_name = cursor.read_null_terminated_string()?;
        cursor.seek(saved)?;

        Ok(Self {
            texture_name,
            pane_name,
            clear_color,
            format_id,
            framebuffer_capture_enabled,
            capture_only_first_frame,
            filters,
        })
    }

    pub fn serialize(
        &self,
        writer: &mut Writer,
        tex_placeholder: &mut Placeholder32,
        pane_placeholder: &mut Placeholder32,
    ) {
        *tex_placeholder = writer.write_placeholder_u32();
        *pane_placeholder = writer.write_placeholder_u32();

        writer.write_u64(0);

        self.clear_color.write(writer);

        writer.write_i16(self.format_id);
        writer.write_u8(self.framebuffer_capture_enabled as u8);
        writer.write_u8(self.capture_only_first_frame as u8);
        writer.write_u16(self.filters.len() as u16);

        writer.write_u16(0);

        for filter in &self.filters {
            writer.write_u32(0);
            writer.write_f32(filter.scale);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaptureTextureList {
    pub infos: Vec<CaptureTexture>,
}

impl ReadWriteable for CaptureTextureList {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let count = cursor.read_u32()?;

        let mut infos = Vec::with_capacity(count as usize);
        for _ in 0..count {
            infos.push(CaptureTexture::parse(cursor)?);
        }

        Ok(Self { infos })
    }

    fn write(&self, writer: &mut Writer) {
        let ctl_base = writer.ctx_section_start::<Self>();
        writer.write_u32(self.infos.len() as u32);

        let mut tex_placeholders = Vec::with_capacity(self.infos.len());
        let mut pane_placeholders = Vec::with_capacity(self.infos.len());

        for info in &self.infos {
            let mut tex_ph = Placeholder32(0);
            let mut pane_ph = Placeholder32(0);

            info.serialize(writer, &mut tex_ph, &mut pane_ph);

            tex_placeholders.push(tex_ph);
            pane_placeholders.push(pane_ph);
        }

        for (i, info) in self.infos.iter().enumerate() {
            let tex_off = writer.pos() - ctl_base;
            writer.patch_u32(tex_placeholders[i], tex_off as u32);
            writer.write_null_terminated_string(&info.texture_name);

            let pane_off = writer.pos() - ctl_base;
            writer.patch_u32(pane_placeholders[i], pane_off as u32);
            writer.write_null_terminated_string(&info.pane_name);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorGraphics {
    pub reserve1: u32,
    pub reserve2: u32,
    pub reserve3: u32,
    pub bnvg_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorGraphicsList {
    pub infos: Vec<VectorGraphics>,
}

impl ReadWriteable for VectorGraphicsList {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let vgl_base = cursor.ctx_section_start::<Self>()?;
        let count = cursor.read_u32()?;

        let mut offsets = Vec::with_capacity(count as usize);
        for _ in 0..count {
            offsets.push(cursor.read_u32()?);
        }

        let saved = cursor.pos;
        let mut infos = Vec::with_capacity(offsets.len());
        for offset in offsets {
            cursor.seek(vgl_base + offset as usize)?;
            let reserve1 = cursor.read_u32()?;
            let reserve2 = cursor.read_u32()?;
            let reserve3 = cursor.read_u32()?;
            let bnvg_name = cursor.read_null_terminated_string()?;
            infos.push(VectorGraphics {
                reserve1,
                reserve2,
                reserve3,
                bnvg_name,
            });
        }

        cursor.seek(saved)?;

        Ok(Self { infos })
    }

    fn write(&self, writer: &mut Writer) {
        let vgl_base = writer.ctx_section_start::<Self>();
        writer.write_u32(self.infos.len() as u32);

        let mut offset_placeholders = Vec::with_capacity(self.infos.len());
        for _ in &self.infos {
            offset_placeholders.push(writer.write_placeholder_u32());
        }

        for (i, info) in self.infos.iter().enumerate() {
            let offset = writer.pos() - vgl_base;
            writer.patch_u32(offset_placeholders[i], offset as u32);
            writer.write_u32(info.reserve1);
            writer.write_u32(info.reserve2);
            writer.write_u32(info.reserve3);
            writer.write_null_terminated_string(&info.bnvg_name);
        }
    }
}

pub const GROUP_NAME_LEN: usize = 0x21;
pub const GROUP_PANE_NAME_LEN: usize = 0x18;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Group {
    pub group_name: String,
    pub child_names: Vec<String>,
}

impl ReadWriteable for Group {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let group_name = cursor.read_fixed_string(GROUP_NAME_LEN)?;
        let _reserve0 = cursor.read_u8()?;
        let child_count = cursor.read_u16()?;
        let mut child_names = Vec::with_capacity(child_count as usize);

        for _ in 0..child_count {
            child_names.push(cursor.read_fixed_string(GROUP_PANE_NAME_LEN)?);
        }

        Ok(Self {
            group_name,
            child_names,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.write_fixed_string(&self.group_name, GROUP_NAME_LEN);
        writer.write_u8(0);
        writer.write_u16(self.child_names.len() as u16);

        for name in &self.child_names {
            writer.write_fixed_string(name, GROUP_PANE_NAME_LEN);
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlSource {
    pub control_name: String,
    pub reserve0_name: String,

    pub pane_bindings: Vec<String>,
    pub core_anims: Vec<String>,

    pub pane_names: Vec<String>,
    pub anim_names: Vec<String>,
}

impl ReadWriteable for ControlSource {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.ctx_section_start::<Self>()?;

        let reserve0_offset = cursor.read_u32()? as usize;
        let name_array_offset = cursor.read_u32()? as usize;
        let pane_count = cursor.read_u16()? as usize;
        let anim_count = cursor.read_u16()? as usize;
        let pane_name_offset_arr = cursor.read_u32()? as usize;
        let anim_name_offset_arr = cursor.read_u32()? as usize;

        let control_name = cursor.read_null_terminated_string()?;

        cursor.seek(section_start + reserve0_offset)?;
        let reserve0_name = cursor.read_null_terminated_string()?;

        let na_base = section_start + name_array_offset;
        cursor.seek(na_base)?;

        let mut pane_bindings = Vec::with_capacity(pane_count);
        for _ in 0..pane_count {
            pane_bindings.push(cursor.read_fixed_string(GROUP_PANE_NAME_LEN)?);
        }

        let core_table_base = na_base + (pane_count * GROUP_PANE_NAME_LEN);
        cursor.seek(core_table_base)?;

        let mut core_offsets = Vec::with_capacity(anim_count);
        for _ in 0..anim_count {
            core_offsets.push(cursor.read_u32()? as usize);
        }

        let mut core_anims = Vec::with_capacity(anim_count);
        for offset in core_offsets {
            cursor.seek(core_table_base + offset)?;
            core_anims.push(cursor.read_null_terminated_string()?);
        }

        let pane_table_base = section_start + pane_name_offset_arr;
        cursor.seek(pane_table_base)?;

        let mut pane_offsets = Vec::with_capacity(pane_count);
        for _ in 0..pane_count {
            pane_offsets.push(cursor.read_u32()? as usize);
        }

        let mut pane_names = Vec::with_capacity(pane_count);
        for offset in pane_offsets {
            cursor.seek(pane_table_base + offset)?;
            pane_names.push(cursor.read_null_terminated_string()?);
        }

        let anim_table_base = section_start + anim_name_offset_arr;
        cursor.seek(anim_table_base)?;

        let mut anim_offsets = Vec::with_capacity(anim_count);
        for _ in 0..anim_count {
            anim_offsets.push(cursor.read_u32()? as usize);
        }

        let mut anim_names = Vec::with_capacity(anim_count);
        for offset in anim_offsets {
            cursor.seek(anim_table_base + offset)?;
            anim_names.push(cursor.read_null_terminated_string()?);
        }

        Ok(Self {
            control_name,
            reserve0_name,
            pane_bindings,
            core_anims,
            pane_names,
            anim_names,
        })
    }

    fn write(&self, writer: &mut Writer) {
        let section_start = writer.ctx_section_start::<Self>();

        let pane_count = self.pane_names.len();
        let anim_count = self.anim_names.len();

        let reserve0_offset_pos = writer.write_placeholder_u32();
        let name_array_offset_pos = writer.write_placeholder_u32();
        writer.write_u16(pane_count as u16);
        writer.write_u16(anim_count as u16);
        let pane_name_offset_arr_pos = writer.write_placeholder_u32();
        let anim_name_offset_arr_pos = writer.write_placeholder_u32();

        writer.write_null_terminated_string(&self.control_name);
        writer.align(4);

        let reserve0_off = writer.pos() - section_start;
        writer.patch_u32(reserve0_offset_pos, reserve0_off as u32);
        writer.write_null_terminated_string(&self.reserve0_name);
        writer.align(4);

        let name_array_off = writer.pos() - section_start;
        writer.patch_u32(name_array_offset_pos, name_array_off as u32);

        for binding in &self.pane_bindings {
            writer.write_fixed_string(binding, GROUP_PANE_NAME_LEN);
        }

        let core_table_base = writer.pos();
        let mut core_phs = Vec::with_capacity(anim_count);
        for _ in 0..anim_count {
            core_phs.push(writer.write_placeholder_u32());
        }

        for (i, name) in self.core_anims.iter().enumerate() {
            let off = writer.pos() - core_table_base;
            writer.patch_u32(core_phs[i], off as u32);
            writer.write_null_terminated_string(name);
        }
        writer.align(4);

        let pane_table_base = writer.pos();
        let pane_name_off = pane_table_base - section_start;
        writer.patch_u32(pane_name_offset_arr_pos, pane_name_off as u32);

        let mut pna_phs = Vec::with_capacity(pane_count);
        for _ in 0..pane_count {
            pna_phs.push(writer.write_placeholder_u32());
        }

        for (i, name) in self.pane_names.iter().enumerate() {
            let off = writer.pos() - pane_table_base;
            writer.patch_u32(pna_phs[i], off as u32);
            writer.write_null_terminated_string(name);
        }

        writer.align(4);

        let anim_table_base = writer.pos();
        let anim_name_off = anim_table_base - section_start;
        writer.patch_u32(anim_name_offset_arr_pos, anim_name_off as u32);

        let mut ana_phs = Vec::with_capacity(anim_count);
        for _ in 0..anim_count {
            ana_phs.push(writer.write_placeholder_u32());
        }

        for (i, name) in self.anim_names.iter().enumerate() {
            let off = writer.pos() - anim_table_base;
            writer.patch_u32(ana_phs[i], off as u32);
            writer.write_null_terminated_string(name);
        }

        writer.align(4);
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoPrimitive, FromPrimitive, Default,
)]
#[repr(u16)]
pub enum ShapeInfoType {
    #[default]
    NormalQuad,
    PrimitiveRoundRect,
    PrimitiveCircle,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ShapeParam {
    #[default]
    NormalQuad,
    PrimitiveRoundRect {
        radius: u32,
        slice: u32,
    },
    PrimitiveCircle {
        slice: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShapeInfoList {
    pub shapes: Vec<ShapeParam>,
}

impl ReadWriteable for ShapeInfoList {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let count = cursor.read_u16()? as usize;
        let _padding = cursor.read_u16()?;
        let _data_offset = cursor.read_u32()? as usize;

        let mut shapes = Vec::with_capacity(count);

        for _ in 0..count {
            let type_raw = cursor.read_u16()?;
            let _shape_padding = cursor.read_u16()?;

            let shape_type = match type_raw {
                0 => ShapeInfoType::NormalQuad,
                1 => ShapeInfoType::PrimitiveRoundRect,
                2 => ShapeInfoType::PrimitiveCircle,
                _ => {
                    return Err(FormatError::UnknownTag {
                        enum_name: "ShapeInfoType",
                        tag: type_raw as u32,
                        offset: cursor.pos,
                    });
                }
            };

            let param = match shape_type {
                ShapeInfoType::NormalQuad => ShapeParam::NormalQuad,
                ShapeInfoType::PrimitiveRoundRect => {
                    let radius = cursor.read_u32()?;
                    let slice = cursor.read_u32()?;
                    ShapeParam::PrimitiveRoundRect { radius, slice }
                }
                ShapeInfoType::PrimitiveCircle => {
                    let slice = cursor.read_u32()?;
                    ShapeParam::PrimitiveCircle { slice }
                }
            };

            shapes.push(param);
        }

        Ok(Self { shapes })
    }

    fn write(&self, writer: &mut Writer) {
        let section_start = writer.ctx_section_start::<Self>();
        let count = self.shapes.len();

        writer.write_u16(count as u16);
        writer.write_u16(0);
        let data_offset_pos = writer.write_placeholder_u32();

        let current_offset = writer.pos() - section_start;
        writer.patch_u32(data_offset_pos, current_offset as u32);

        for shape in &self.shapes {
            match shape {
                ShapeParam::NormalQuad => {
                    writer.write_u16(ShapeInfoType::NormalQuad as u16);
                    writer.write_u16(0);
                }
                ShapeParam::PrimitiveRoundRect { radius, slice } => {
                    writer.write_u16(ShapeInfoType::PrimitiveRoundRect as u16);
                    writer.write_u16(0);
                    writer.write_u32(*radius);
                    writer.write_u32(*slice);
                }
                ShapeParam::PrimitiveCircle { slice } => {
                    writer.write_u16(ShapeInfoType::PrimitiveCircle as u16);
                    writer.write_u16(0);
                    writer.write_u32(*slice);
                }
            }
        }
    }
}
