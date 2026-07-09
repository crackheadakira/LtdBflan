use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};

use crate::{
    bflan::{anim_info::AnimInfoType, curves::Curve},
    core::{Cursor, FormatError, Writer},
};

#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct AnimTarget {
    pub layer: u8,
    pub target: TargetIndex,
    pub curve: Curve,
}

impl AnimTarget {
    pub fn parse(cursor: &mut Cursor, parent_magic: &AnimInfoType) -> Result<Self, FormatError> {
        let base_offset = cursor.ctx_section_start::<Self>()?;

        cursor.seek(base_offset)?;

        let layer = cursor.read_u8()?;
        let target_raw = cursor.read_u8()?;
        let curve_type = cursor.read_u8()?;
        let _reserve1 = cursor.read_u8()?;
        let frame_count = cursor.read_u16()?;
        let _reserve2 = cursor.read_u16()?;
        let key_array_offset = cursor.read_u32()?;

        let target = TargetIndex::resolve(parent_magic, target_raw);

        cursor.seek(base_offset + key_array_offset as usize)?;

        let curve = Curve::parse(cursor, curve_type, frame_count as usize)?;

        Ok(Self {
            layer,
            target,
            curve,
        })
    }

    pub fn write(&self, writer: &mut Writer) {
        let base_offset = writer.ctx_section_start::<Self>();

        writer.mark("AnimTarget");
        writer.write_u8(self.layer);
        writer.write_u8(self.target.to_raw());

        let (curve_type, frame_count) = match &self.curve {
            Curve::Constant(keys) => (0, keys.len()),
            Curve::Step(keys) => (1, keys.len()),
            Curve::Hermite(keys) => (2, keys.len()),
        };

        writer.write_u8(curve_type);
        writer.write_u8(0);
        writer.write_u16(frame_count as u16);
        writer.write_u16(0);

        let key_array_offset_pos = writer.write_placeholder_u32();

        let keys_base = writer.pos();
        writer.patch_u32(key_array_offset_pos, (keys_base - base_offset) as u32);

        self.curve.serialize(writer);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum TargetIndex {
    PerCharacterTransformCurve(PerCharacterTransformCurveTarget),
    PerCharacterTransform(PerCharacterTransformTarget),
    PaneSrt(PaneSrtTarget),
    VertexColor(VertexColorTarget),
    Visibility(VisibilityTarget),
    DropShadow(DropShadowTarget),
    MaskTexture(MaskTextureTarget),
    ProceduralShape(ProceduralShapeTarget),
    Window(WindowTarget),
    StateMachine(StateMachineTarget),
    AlphaCompare(AlphaCompareTarget),
    FontShadow(FontShadowTarget),
    IndirectSrt(IndirectSrtTarget),
    MaterialColor(MaterialColorTarget),
    TextureSrt(TextureSrtTarget),
    TexturePattern(TexturePatternTarget),
    BrickRepeat(BrickRepeatTarget),
    VectorGraphics(VectorGraphicsTarget),
    #[default]
    Invalid,
}

impl TargetIndex {
    pub fn to_raw(&self) -> u8 {
        match self {
            Self::PerCharacterTransformCurve(t) => t.clone() as u8,
            Self::PerCharacterTransform(t) => t.clone() as u8,
            Self::PaneSrt(t) => t.clone() as u8,
            Self::VertexColor(t) => t.clone() as u8,
            Self::Visibility(t) => t.clone() as u8,
            Self::DropShadow(t) => t.clone() as u8,
            Self::MaskTexture(t) => t.clone() as u8,
            Self::ProceduralShape(t) => t.clone() as u8,
            Self::Window(t) => t.clone() as u8,
            Self::StateMachine(t) => t.clone() as u8,
            Self::AlphaCompare(t) => t.clone() as u8,
            Self::FontShadow(t) => t.clone() as u8,
            Self::IndirectSrt(t) => t.clone() as u8,
            Self::MaterialColor(t) => t.clone() as u8,
            Self::TextureSrt(t) => t.clone() as u8,
            Self::TexturePattern(t) => t.clone() as u8,
            Self::BrickRepeat(t) => t.clone() as u8,
            Self::VectorGraphics(t) => t.clone() as u8,
            Self::Invalid => 255,
        }
    }

    pub fn resolve(magic: &AnimInfoType, raw: u8) -> Self {
        match magic {
            AnimInfoType::PerCharacterTransformCurveAnim => {
                Self::PerCharacterTransformCurve(raw.into())
            }
            AnimInfoType::PerCharacterTransformAnim => Self::PerCharacterTransform(raw.into()),
            AnimInfoType::PaneSrtAnim => Self::PaneSrt(raw.into()),
            AnimInfoType::VertexColorAnim => Self::VertexColor(raw.into()),
            AnimInfoType::VisibilityAnim => Self::Visibility(raw.into()),
            AnimInfoType::DropShadowAnim => Self::DropShadow(raw.into()),
            AnimInfoType::MaskTextureAnim => Self::MaskTexture(raw.into()),
            AnimInfoType::ProceduralShapeAnim => Self::ProceduralShape(raw.into()),
            AnimInfoType::WindowAnim => Self::Window(raw.into()),
            AnimInfoType::StateMachineAnim => Self::StateMachine(raw.into()),
            AnimInfoType::AlphaCompareAnim => Self::AlphaCompare(raw.into()),
            AnimInfoType::FontShadowAnim => Self::FontShadow(raw.into()),
            AnimInfoType::IndirectSrtAnim => Self::IndirectSrt(raw.into()),
            AnimInfoType::MaterialColorAnim => Self::MaterialColor(raw.into()),
            AnimInfoType::TextureSrtAnim => Self::TextureSrt(raw.into()),
            AnimInfoType::TexturePatternAnim => Self::TexturePattern(raw.into()),
            AnimInfoType::BrickRepeatAnim => Self::BrickRepeat(raw.into()),
            AnimInfoType::VectorGraphicsAnim => Self::VectorGraphics(raw.into()),
            _ => Self::Invalid,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum PaneSrtTarget {
    #[default]
    TranslateX,
    TranslateY,
    TranslateZ,
    RotateX,
    RotateY,
    RotateZ,
    ScaleX,
    ScaleY,
    SizeX,
    SizeY,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum FontShadowTarget {
    #[default]
    BlackRed,
    BlackGreen,
    BlackBlue,
    WhiteRed,
    WhiteGreen,
    WhiteBlue,
    WhiteAlpha,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum PerCharacterTransformCurveTarget {
    #[default]
    TranslateX,
    TranslateY,
    TranslateZ,
    RotateX,
    RotateY,
    RotateZ,
    LeftTopRed,
    LeftTopGreen,
    LeftTopBlue,
    LeftTopAlpha,
    LeftBottomRed,
    LeftBottomGreen,
    LeftBottomBlue,
    LeftBottomAlpha,
    ScaleX,
    ScaleY,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum PerCharacterTransformTarget {
    #[default]
    EvalTimeOffset,
    EvalTimeWidth,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum TexturePatternTarget {
    #[default]
    Image,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum AlphaCompareTarget {
    #[default]
    CompareReference,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum VisibilityTarget {
    #[default]
    Visibility,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum VertexColorTarget {
    #[default]
    LeftTopRed,
    LeftTopGreen,
    LeftTopBlue,
    LeftTopAlpha,
    RightTopRed,
    RightTopGreen,
    RightTopBlue,
    RightTopAlpha,
    LeftBottomRed,
    LeftBottomGreen,
    LeftBottomBlue,
    LeftBottomAlpha,
    RightBottomRed,
    RightBottomGreen,
    RightBottomBlue,
    RightBottomAlpha,
    PaneAlpha,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum MaterialColorTarget {
    #[default]
    BufferRed,
    BufferGreen,
    BufferBlue,
    BufferAlpha,
    Constant0Red,
    Constant0Green,
    Constant0Blue,
    Constant0Alpha,
    Color0Red,
    Color0Green,
    Color0Blue,
    Color0Alpha,
    Color1Red,
    Color1Green,
    Color1Blue,
    Color1Alpha,
    Color2Red,
    Color2Green,
    Color2Blue,
    Color2Alpha,
    Color3Red,
    Color3Green,
    Color3Blue,
    Color3Alpha,
    Color4Red,
    Color4Green,
    Color4Blue,
    Color4Alpha,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum TextureSrtTarget {
    #[default]
    TranslateU,
    TranslateV,
    Rotate,
    ScaleU,
    ScaleV,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum IndirectSrtTarget {
    #[default]
    Rotate,
    ScaleU,
    ScaleV,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum BrickRepeatTarget {
    #[default]
    CountX,
    CountY,
    OffsetX,
    OffsetY,
    LocalScaleX,
    LocalScaleY,
    LocalRotate,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum WindowTarget {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum VectorGraphicsTarget {
    #[default]
    Time,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum ProceduralShapeTarget {
    #[default]
    ExpLeftTop,
    ExpRightTop,
    ExpLeftBottom,
    ExpRightBottom,

    RadiusLeftTop,
    RadiusRightTop,
    RadiusLeftBottom,
    RadiusRightBottom,

    InnerStrokeSize,
    InnerStrokeColorRed,
    InnerStrokeColorGreen,
    InnerStrokeColorBlue,
    InnerStrokeColorAlpha,
    InnerShadowColorRed,

    InnerShadowColorGreen,
    InnerShadowColorBlue,
    InnerShadowColorAlpha,
    InnerShadowAngle,
    InnerShadowDistance,
    InnerShadowSize,

    ColorOverlayColorRed,
    ColorOverlayColorGreen,
    ColorOverlayColorBlue,
    ColorOverlayColorAlpha,

    GradationOverlayControl0,
    GradationOverlayControl1,
    GradationOverlayControl2,
    GradationOverlayControl3,

    GradationOverlayColor0Red,
    GradationOverlayColor0Green,
    GradationOverlayColor0Blue,
    GradationOverlayColor0Alpha,

    GradationOverlayColor1Red,
    GradationOverlayColor1Green,
    GradationOverlayColor1Blue,
    GradationOverlayColor1Alpha,

    GradationOverlayColor2Red,
    GradationOverlayColor2Green,
    GradationOverlayColor2Blue,
    GradationOverlayColor2Alpha,

    GradationOverlayColor3Red,
    GradationOverlayColor3Green,
    GradationOverlayColor3Blue,
    GradationOverlayColor3Alpha,
    GradationOverlayAngle,

    OuterShadowColorRed,
    OuterShadowColorGreen,
    OuterShadowColorBlue,
    OuterShadowColorAlpha,
    OuterShadowAngle,
    OuterShadowDistance,
    OuterShadowSize,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum StateMachineTarget {
    #[default]
    PostToChild,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum MaskTextureTarget {
    #[default]
    TranslateX,
    TranslateY,
    Rotate,
    ScaleX,
    ScaleY,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum DropShadowTarget {
    #[default]
    StrokeSize,
    StrokeColorRed,
    StrokeColorGreen,
    StrokeColorBlue,
    StrokeColorAlpha,

    OuterGlowColorRed,
    OuterGlowColorGreen,
    OuterGlowColorBlue,
    OuterGlowColorAlpha,
    OuterGlowSpread,
    OuterGlowSize,

    DropShadowColorRed,
    DropShadowColorGreen,
    DropShadowColorBlue,
    DropShadowColorAlpha,

    DropShadowAngle,
    DropShadowDistance,
    DropShadowSpread,
    DropShadowSize,
}
