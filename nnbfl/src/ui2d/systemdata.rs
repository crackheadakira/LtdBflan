use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};

use crate::{
    bflyt::flags::{DropShadowFlags, TexOptions},
    core::{BitPackable as _, Cursor, FormatError, ReadWriteable, Writer},
    ui2d::types::{Color4f, VertexPos},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
/// System data configurations for either an entire layout or an individual pane.
pub enum SystemData {
    /// Global configuration belonging to the entire layout.
    Layout(LayoutData),

    /// Visual modifiers
    Pane(PaneData),
}

impl Default for SystemData {
    fn default() -> Self {
        Self::Layout(Default::default())
    }
}

#[derive(Debug, FromPrimitive, IntoPrimitive, Default)]
#[repr(u32)]
/// The possible types of [`LayoutData`].
pub enum LayoutDataType {
    /// The animation tag name to be found in the bflan for this layout.
    AnimTagName = 0,

    #[default]
    /// An invalid layout data type.
    Invalid = 1,
}

#[derive(Debug, FromPrimitive, IntoPrimitive, Default)]
#[repr(u32)]
/// The possible types of [`PaneData`].
pub enum PaneDataType {
    /// Maps to vertex layout scale data.
    VertexPos0 = 0,

    /// Maps to secondary vertex layout scale data.
    VertexPos1 = 1,

    /// Maps to layout alignment options and margins.
    Alignment = 2,

    /// Maps to masking texture configurations.
    MaskTexture = 3,

    /// Maps to pane drop shadow and glow styling parameters.
    DropShadow = 4,

    /// Maps to procedurally generated geometry & vector shape properties.
    ProceduralShape = 6,

    #[default]
    /// An invalid pane data type.
    Invalid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// A container for specialized pane data properties.
pub enum PaneData {
    /// The primary coordinate space transformations of a vertex.
    VertexPos0(VertexPos),

    /// The secondary coordinate space transformations of a vertex.
    VertexPos1(VertexPos),

    /// Properties for procedurally generated UI geometry.
    ProceduralShape(ProceduralShape),

    /// Layout alignment and margin settings.
    Alignment(Alignment),

    /// Drop shadow and styling properties.
    DropShadow(DropShadow),

    /// Masking texture configuration properties.
    MaskTexture(MaskTexture),
}

impl Default for PaneData {
    fn default() -> Self {
        Self::VertexPos0(Default::default())
    }
}

impl ReadWriteable for PaneData {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let offset = cursor.pos;
        let data_type: PaneDataType = cursor.read_u32()?.into();

        let res = match data_type {
            PaneDataType::VertexPos0 => Self::VertexPos0(VertexPos::parse(cursor)?),
            PaneDataType::VertexPos1 => Self::VertexPos1(VertexPos::parse(cursor)?),
            PaneDataType::MaskTexture => Self::MaskTexture(MaskTexture::parse(cursor)?),
            PaneDataType::DropShadow => Self::DropShadow(DropShadow::parse(cursor)?),
            PaneDataType::Alignment => Self::Alignment(Alignment::parse(cursor)?),
            PaneDataType::ProceduralShape => Self::ProceduralShape(ProceduralShape::parse(cursor)?),
            PaneDataType::Invalid => {
                return Err(FormatError::UnknownTag {
                    enum_name: "PaneDataType",
                    tag: data_type.into(),
                    offset,
                });
            }
        };

        Ok(res)
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("PaneDataType");

        let type_id: u32 = match self {
            Self::VertexPos0(_) => 0,
            Self::VertexPos1(_) => 1,
            Self::Alignment(_) => 2,
            Self::MaskTexture(_) => 3,
            Self::DropShadow(_) => 4,
            Self::ProceduralShape(_) => 6,
        };

        writer.write_u32(type_id);

        match self {
            Self::VertexPos0(v) | Self::VertexPos1(v) => v.write(writer),
            Self::Alignment(a) => a.write(writer),
            Self::MaskTexture(m) => m.write(writer),
            Self::DropShadow(d) => d.write(writer),
            Self::ProceduralShape(p) => p.write(writer),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// A container for the layout-specific data.
pub enum LayoutData {
    /// A list containing the animation tag names.
    AnimTagName(Vec<String>),

    /// An invalid layout data.
    Invalid,
}

impl Default for LayoutData {
    fn default() -> Self {
        Self::AnimTagName(Vec::new())
    }
}

impl ReadWriteable for LayoutData {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.ctx_section_start::<Self>()?;

        let data_type: LayoutDataType = cursor.read_u32()?.into();

        let res = match data_type {
            LayoutDataType::AnimTagName => {
                let string_count = cursor.read_u32()?;
                let mut strings = Vec::with_capacity(string_count as usize);

                for _ in 0..string_count {
                    let string_offset = cursor.read_u32()?;
                    let restore_point = cursor.pos;

                    cursor.seek(section_start + string_offset as usize)?;
                    let string = cursor.read_null_terminated_string()?;

                    cursor.seek(restore_point)?;

                    strings.push(string);
                }

                Self::AnimTagName(strings)
            }
            LayoutDataType::Invalid => Self::Invalid,
        };

        Ok(res)
    }

    fn write(&self, writer: &mut Writer) {
        match self {
            Self::AnimTagName(strings) => {
                let base_offset = writer.pos();

                writer.write_u32(LayoutDataType::AnimTagName as u32);
                writer.write_u32(strings.len() as u32);

                let mut offset_positions = Vec::with_capacity(strings.len());
                for _ in strings {
                    offset_positions.push(writer.write_placeholder_u32());
                }

                let string_pool_start = writer.pos();
                for (i, string) in strings.iter().enumerate() {
                    let relative_offset = (writer.pos() - base_offset) as u32;
                    writer.patch_u32(offset_positions[i], relative_offset);
                    writer.write_null_terminated_string(string);
                }

                let bytes_written = writer.pos() - string_pool_start;

                const ALIGNMENT: usize = 64;
                let padding_needed = (ALIGNMENT - (bytes_written % ALIGNMENT)) % ALIGNMENT;
                for _ in 0..padding_needed {
                    writer.write_u8(0);
                }
            }

            Self::Invalid => {
                writer.write_u32(0xFFFFFFFF);
                writer.write_u32(0);
            }
        }

        writer.align(4);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
/// Configuration properties for pane alignment.
pub struct Alignment {
    /// Packed options for the pane alignment.
    pub options: u32,

    /// The margin applied to every direction.
    pub margin: f32,
}

impl ReadWriteable for Alignment {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            options: cursor.read_u32()?,
            margin: cursor.read_f32()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("System Data Alignment");
        writer.write_u32(self.options);
        writer.write_f32(self.margin);
    }
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive, Default,
)]
#[repr(u8)]
/// The possible blend modes for [`DropShadow`].
pub enum DropShadowBlendMode {
    #[default]
    /// Normal blend mode.
    Normal = 0,

    /// Multiply blend mode.
    Multiply = 1,

    /// Addition blend mode.
    Addition = 2,

    /// Subtraction blend mode.
    Subtraction = 3,

    /// Normal blend mode using maximum alpha.
    NormalMaxAlpha = 4,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
/// Styling and transformation properties for a pane's stroke, outer glow, and drop shadow.
pub struct DropShadow {
    /// The 0-based index of the shadow texture within [`TextureList`](crate::bflyt::list::TextureList).
    pub texture_id: u16,

    /// Texture coordinate wrap and filtering options along the horizontal (U) axis.
    pub u_options: TexOptions,

    /// Texture coordinate wrap and filtering options along the vertical (V) axis.
    pub v_options: TexOptions,

    /// Configuration and state flags for the shadow effects.
    pub flags: DropShadowFlags,

    /// The maximum size boundary allowed for the shadow effect.
    pub max_size: u8,

    /// The blend mode used for rendering the stroke layer.
    pub stroke_blend_mode: DropShadowBlendMode,

    /// The blend mode used for rendering the outer glow layer.
    pub outer_glow_blend_mode: DropShadowBlendMode,

    /// The blend mode used for rendering the drop shadow layer.
    pub drop_shadow_blend_mode: DropShadowBlendMode,

    /// The width of the outline stroke.
    pub stroke_size: f32,

    /// The color of the outline stroke.
    pub stroke_color: Color4f,

    /// The color of the outer glow layer.
    pub outer_glow_color: Color4f,

    /// The blur spread factor of the outer glow layer.
    pub outer_glow_spread: f32,

    /// The total size radius of the outer glow layer.
    pub outer_glow_size: f32,

    /// The color of the drop shadow layer.
    pub drop_shadow_color: Color4f,

    /// The angle in degrees indicating the shadow's projection direction.
    pub drop_shadow_angle: f32,

    /// The offset distance from the source pane to project the shadow.
    pub drop_shadow_distance: f32,

    /// The blur spread factor of the drop shadow layer.
    pub drop_shadow_spread: f32,

    /// The total size radius of the drop shadow layer.
    pub drop_shadow_size: f32,
}

impl ReadWriteable for DropShadow {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let texture_id = cursor.read_u16()?;
        let u_options = TexOptions::decode(cursor.read_u8()?);
        let v_options = TexOptions::decode(cursor.read_u8()?);
        let flags = DropShadowFlags::decode(cursor.read_u8()?);

        cursor.read_u8()?;
        cursor.read_u8()?;
        cursor.read_u8()?;

        let max_size = cursor.read_u8()?;
        let stroke_blend_mode = cursor.read_u8()?.into();
        let outer_glow_blend_mode = cursor.read_u8()?.into();
        let drop_shadow_blend_mode = cursor.read_u8()?.into();

        cursor.read_u32()?;
        cursor.read_u32()?;
        cursor.read_u32()?;
        cursor.read_u32()?;

        let stroke_size = cursor.read_f32()?;
        let stroke_color = Color4f::parse(cursor)?;

        let outer_glow_color = Color4f::parse(cursor)?;
        let outer_glow_spread = cursor.read_f32()?;
        let outer_glow_size = cursor.read_f32()?;

        let drop_shadow_color = Color4f::parse(cursor)?;
        let drop_shadow_angle = cursor.read_f32()?;
        let drop_shadow_distance = cursor.read_f32()?;
        let drop_shadow_spread = cursor.read_f32()?;
        let drop_shadow_size = cursor.read_f32()?;

        cursor.read_u32()?;
        cursor.read_u32()?;
        cursor.read_u32()?;
        cursor.read_u32()?;

        Ok(Self {
            texture_id,
            u_options,
            v_options,
            flags,
            max_size,
            stroke_blend_mode,
            outer_glow_blend_mode,
            drop_shadow_blend_mode,
            stroke_size,
            stroke_color,
            outer_glow_color,
            outer_glow_spread,
            outer_glow_size,
            drop_shadow_color,
            drop_shadow_angle,
            drop_shadow_distance,
            drop_shadow_spread,
            drop_shadow_size,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("Drop Shadow");
        writer.write_u16(self.texture_id);
        writer.write_u8(self.u_options.encode());
        writer.write_u8(self.v_options.encode());
        writer.write_u8(self.flags.encode());

        writer.write_u8(0);
        writer.write_u8(0);
        writer.write_u8(0);

        writer.write_u8(self.max_size);
        writer.write_u8(self.stroke_blend_mode as u8);
        writer.write_u8(self.outer_glow_blend_mode as u8);
        writer.write_u8(self.drop_shadow_blend_mode as u8);

        writer.write_u64(0);
        writer.write_u64(0);

        writer.write_f32(self.stroke_size);
        self.stroke_color.write(writer);

        self.outer_glow_color.write(writer);
        writer.write_f32(self.outer_glow_spread);
        writer.write_f32(self.outer_glow_size);

        self.drop_shadow_color.write(writer);
        writer.write_f32(self.drop_shadow_angle);
        writer.write_f32(self.drop_shadow_distance);
        writer.write_f32(self.drop_shadow_spread);
        writer.write_f32(self.drop_shadow_size);

        writer.write_u64(0);
        writer.write_u64(0);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
/// Configuration properties for a layout masking texture and its coordinate space transformations.
pub struct MaskTexture {
    /// Packed feature and state flags for the masking behavior.
    pub flags: u8,

    /// The 0-based index of the mask texture within [`TextureList`](crate::bflyt::list::TextureList).
    pub texture_id: u16,

    /// Texture coordinate wrap and filtering options along the horizontal (U) axis.
    pub u_options: u8,

    /// Texture coordinate wrap and filtering options along the vertical (V) axis.
    pub v_options: u8,

    /// Extended texture mapping and rendering flags.
    pub tex_ext_flags: u32,

    /// The 0-based index of the capture texture within [`TextureList`](crate::bflyt::list::TextureList).
    pub capture_texture_id: u16,

    /// Capture texture coordinate options along the horizontal (U) axis.
    pub capture_u_options: u8,

    /// Capture texture coordinate options along the vertical (V) axis.
    pub capture_v_options: u8,

    /// If a capture mask should be used on the mask texture.
    pub is_use_capture_mask: bool,

    /// The 2D spatial translation offset `[x, y]` applied to the mask coordinates.
    pub translation: [f32; 2],

    /// The Z-rotation of the mask texture.
    pub rotation: f32,

    /// The 2D scale factors `[x, y]` applied to the mask coordinates.
    pub scale: [f32; 2],
}

impl ReadWriteable for MaskTexture {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let flags = cursor.read_u8()?;
        let _reserve0 = [cursor.read_u8()?, cursor.read_u8()?, cursor.read_u8()?];
        let texture_id = cursor.read_u16()?;
        let u_options = cursor.read_u8()?;
        let v_options = cursor.read_u8()?;
        let tex_ext_flags = cursor.read_u32()?;
        let capture_texture_id = cursor.read_u16()?;
        let capture_u_options = cursor.read_u8()?;
        let capture_v_options = cursor.read_u8()?;
        let is_use_capture_mask = cursor.read_u8()? != 0;
        let _reserve1 = [cursor.read_u8()?, cursor.read_u8()?, cursor.read_u8()?];
        let translation = [cursor.read_f32()?, cursor.read_f32()?];
        let rotation = cursor.read_f32()?;
        let scale = [cursor.read_f32()?, cursor.read_f32()?];

        Ok(Self {
            flags,
            texture_id,
            u_options,
            v_options,
            tex_ext_flags,
            capture_texture_id,
            capture_u_options,
            capture_v_options,
            is_use_capture_mask,
            translation,
            rotation,
            scale,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("Mask Texture");

        writer.write_u8(self.flags);

        writer.write_u8(0);
        writer.write_u8(0);
        writer.write_u8(0);

        writer.write_u16(self.texture_id);
        writer.write_u8(self.u_options);
        writer.write_u8(self.v_options);
        writer.write_u32(self.tex_ext_flags);
        writer.write_u16(self.capture_texture_id);
        writer.write_u8(self.capture_u_options);
        writer.write_u8(self.capture_v_options);
        writer.write_u8(self.is_use_capture_mask.into());

        writer.write_u8(0);
        writer.write_u8(0);
        writer.write_u8(0);

        for &f in &self.translation {
            writer.write_f32(f);
        }

        writer.write_f32(self.rotation);
        for &f in &self.scale {
            writer.write_f32(f);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
/// Properties for procedurally generated UI geometry.
pub struct ProceduralShape {
    /// Packed feature and state flags for the procedural shape behavior.
    pub options: u8,

    /// Packed feature and state flags for the inner-shape color.
    pub color0_options: u8,

    /// Packed feature and state flags for the inner-shape shadow color.
    pub inner_shadow_options: u8,

    /// Packed feature and state flags for the inner-shape base comp?.
    pub inner_shadow_base_comp: u8,

    /// Packed feature and state flags for the solid color overlay.
    pub color_overlay_options: u8,

    /// Packed feature and state flags for the gradient (gradation) overlay.
    pub gradation_overlay_options: u8,

    /// Enum for the procedural shape drop shadow blend mode.
    pub drop_shadow_blend_mode: u8,

    /// Enum for the procedural shape drop shadow base component.
    pub drop_shadow_base_comp: u8,

    /// The inner corner rounding factors going: `[TL, TR, BL, BR]`.
    pub rounded_corner0: [f32; 4],

    /// The outer corner rounding factors going: `[TL, TR, BL, BR]`.
    pub rounded_corner1: [f32; 4],

    /// The width the inner stroke should be drawn at.
    pub inner_stroke_size: f32,

    /// The primary color of the shape geometry.
    pub color0: Color4f,

    /// The color of the inner shadow effect.
    pub inner_shadow_color: Color4f,

    /// The 3D spatial transformation `[x, y, z]` applied to the inner shadow.
    pub inner_shadow_transform: [f32; 3],

    /// The color applied as a solid overlay over the base shape.
    pub color_overlay: Color4f,

    /// The interpolation weights or stop positions `[w0, w1, w2, w3]` for the gradient overlay.
    pub gradation_weights: [f32; 4],

    /// The array of color values matching each stop in the gradient overlay.
    pub gradation_color_array: [Color4f; 4],

    /// The rotation angle of the gradient overlay.
    pub gradation_rotation: f32,

    /// The color of the external procedural drop shadow.
    pub drop_shadow_color: Color4f,

    /// The 3D spatial transformation `[x, y, z]` applied to the procedural shape drop shadow.
    pub drop_shadow_transform: [f32; 3],
}

impl ReadWriteable for ProceduralShape {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let options = cursor.read_u8()?;
        let color0_options = cursor.read_u8()?;
        let inner_shadow_options = cursor.read_u8()?;
        let inner_shadow_base_comp = cursor.read_u8()?;
        let color_overlay_options = cursor.read_u8()?;
        let gradation_overlay_options = cursor.read_u8()?;
        let drop_shadow_blend_mode = cursor.read_u8()?;
        let drop_shadow_base_comp = cursor.read_u8()?;

        cursor.read_u32()?;
        cursor.read_u32()?;
        cursor.read_u32()?;
        cursor.read_u32()?;

        let rounded_corner0 = [
            cursor.read_f32()?,
            cursor.read_f32()?,
            cursor.read_f32()?,
            cursor.read_f32()?,
        ];

        let rounded_corner1 = [
            cursor.read_f32()?,
            cursor.read_f32()?,
            cursor.read_f32()?,
            cursor.read_f32()?,
        ];

        let inner_stroke_size = cursor.read_f32()?;

        let color0 = Color4f::parse(cursor)?;
        let inner_shadow_color = Color4f::parse(cursor)?;
        let inner_shadow_transform = [cursor.read_f32()?, cursor.read_f32()?, cursor.read_f32()?];
        let color_overlay = Color4f::parse(cursor)?;

        let gradation_weights = [
            cursor.read_f32()?,
            cursor.read_f32()?,
            cursor.read_f32()?,
            cursor.read_f32()?,
        ];

        let gradation_color_array = [
            Color4f::parse(cursor)?,
            Color4f::parse(cursor)?,
            Color4f::parse(cursor)?,
            Color4f::parse(cursor)?,
        ];

        let gradation_rotation = cursor.read_f32()?;

        let drop_shadow_color = Color4f::parse(cursor)?;
        let drop_shadow_transform = [cursor.read_f32()?, cursor.read_f32()?, cursor.read_f32()?];

        cursor.read_u32()?;
        cursor.read_u32()?;
        cursor.read_u32()?;
        cursor.read_u32()?;

        Ok(Self {
            options,
            color0_options,
            inner_shadow_options,
            inner_shadow_base_comp,
            color_overlay_options,
            gradation_overlay_options,
            drop_shadow_blend_mode,
            drop_shadow_base_comp,
            rounded_corner0,
            rounded_corner1,
            inner_stroke_size,
            color0,
            inner_shadow_color,
            inner_shadow_transform,
            color_overlay,
            gradation_weights,
            gradation_color_array,
            gradation_rotation,
            drop_shadow_color,
            drop_shadow_transform,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("Procedural Shape");
        writer.write_u8(self.options);
        writer.write_u8(self.color0_options);
        writer.write_u8(self.inner_shadow_options);
        writer.write_u8(self.inner_shadow_base_comp);
        writer.write_u8(self.color_overlay_options);
        writer.write_u8(self.gradation_overlay_options);
        writer.write_u8(self.drop_shadow_blend_mode);
        writer.write_u8(self.drop_shadow_base_comp);

        writer.write_u64(0);
        writer.write_u64(0);

        for &f in &self.rounded_corner0 {
            writer.write_f32(f);
        }

        for &f in &self.rounded_corner1 {
            writer.write_f32(f);
        }

        writer.write_f32(self.inner_stroke_size);

        self.color0.write(writer);
        self.inner_shadow_color.write(writer);

        for &f in &self.inner_shadow_transform {
            writer.write_f32(f);
        }

        self.color_overlay.write(writer);

        for &f in &self.gradation_weights {
            writer.write_f32(f);
        }

        for c in &self.gradation_color_array {
            c.write(writer);
        }

        writer.write_f32(self.gradation_rotation);
        self.drop_shadow_color.write(writer);

        for &f in &self.drop_shadow_transform {
            writer.write_f32(f);
        }

        writer.write_u64(0);
        writer.write_u64(0);
    }
}
