use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};

use crate::{
    bflan::anim_info::AnimInfo,
    bflyt::{
        file::BflytSection,
        flags::{BflytPosition, PaneFlags, PaneFlagsEx, TextPaneFlags, WindowFlags},
    },
    core::{BitPackable, Cursor, FormatError, Placeholder32, ReadWriteable, Writer},
    ui2d::types::{Color4u8, Vector2f, Vector3f},
};

pub const PANE_NAME_LEN: usize = 0x18;
pub const USER_NAME_LEN: usize = 0x08;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Pane {
    pub pane_flags: PaneFlags,
    pub position: BflytPosition,
    pub alpha: u8,
    pub flag_ex: PaneFlagsEx,
    pub pane_name: String,
    pub user_name: String,
    pub translation: Vector3f,
    pub rotation: Vector3f,
    pub scale: Vector2f,
    pub size: Vector2f,
}

impl ReadWriteable for Pane {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            pane_flags: PaneFlags::decode(cursor.read_u8()?),
            position: BflytPosition::decode(cursor.read_u8()?),
            alpha: cursor.read_u8()?,
            flag_ex: PaneFlagsEx::decode(cursor.read_u8()?),
            pane_name: cursor.read_fixed_string(PANE_NAME_LEN)?,
            user_name: cursor.read_fixed_string(USER_NAME_LEN)?,
            translation: Vector3f::parse(cursor)?,
            rotation: Vector3f::parse(cursor)?,
            scale: Vector2f::parse(cursor)?,
            size: Vector2f::parse(cursor)?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("Pane (generic)");

        writer.write_u8(self.pane_flags.encode());
        writer.write_u8(self.position.encode());
        writer.write_u8(self.alpha);
        writer.write_u8(self.flag_ex.encode());
        writer.write_fixed_string(&self.pane_name, PANE_NAME_LEN);
        writer.write_fixed_string(&self.user_name, USER_NAME_LEN);
        self.translation.write(writer);
        self.rotation.write(writer);
        self.scale.write(writer);
        self.size.write(writer);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextureUv {
    pub top_left: Vector2f,
    pub top_right: Vector2f,
    pub bottom_left: Vector2f,
    pub bottom_right: Vector2f,
}

impl ReadWriteable for TextureUv {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            top_left: Vector2f::parse(cursor)?,
            top_right: Vector2f::parse(cursor)?,
            bottom_left: Vector2f::parse(cursor)?,
            bottom_right: Vector2f::parse(cursor)?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("TextureUv");

        self.top_left.write(writer);
        self.top_right.write(writer);
        self.bottom_left.write(writer);
        self.bottom_right.write(writer);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PicturePane {
    pub base: Pane,
    pub top_left_vertex_color: Color4u8,
    pub top_right_vertex_color: Color4u8,
    pub bottom_left_vertex_color: Color4u8,
    pub bottom_right_vertex_color: Color4u8,
    pub material_index: u16,
    pub is_shape: bool,
    pub texture_uvs: Vec<TextureUv>,
    pub shape_info_array_index: Option<u32>,
}

impl ReadWriteable for PicturePane {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let base = Pane::parse(cursor)?;
        let top_left_vertex_color = Color4u8::parse(cursor)?;
        let top_right_vertex_color = Color4u8::parse(cursor)?;
        let bottom_left_vertex_color = Color4u8::parse(cursor)?;
        let bottom_right_vertex_color = Color4u8::parse(cursor)?;
        let material_index = cursor.read_u16()?;
        let texture_count = cursor.read_u8()?;
        let is_shape = cursor.read_u8()? != 0;

        let mut texture_uvs = Vec::with_capacity(texture_count as usize);
        for _ in 0..texture_count {
            texture_uvs.push(TextureUv::parse(cursor)?);
        }

        let shape_info_array_index = if is_shape {
            Some(cursor.read_u32()?)
        } else {
            None
        };

        Ok(Self {
            base,
            top_left_vertex_color,
            top_right_vertex_color,
            bottom_left_vertex_color,
            bottom_right_vertex_color,
            material_index,
            is_shape,
            texture_uvs,
            shape_info_array_index,
        })
    }

    fn write(&self, writer: &mut Writer) {
        self.base.write(writer);
        writer.mark("PicturePane");

        self.top_left_vertex_color.write(writer);
        self.top_right_vertex_color.write(writer);
        self.bottom_left_vertex_color.write(writer);
        self.bottom_right_vertex_color.write(writer);
        writer.write_u16(self.material_index);
        writer.write_u8(self.texture_uvs.len() as u8);
        writer.write_u8(self.is_shape.into());

        for uv in &self.texture_uvs {
            uv.write(writer);
        }

        if let Some(sa) = self.shape_info_array_index {
            writer.write_u32(sa);
        }
    }
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum VerticalPosition {
    #[default]
    Top,
    Center,
    Bottom,
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum HorizontalPosition {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum PerCharacterTransformLoopType {
    #[default]
    OneTime,
    Loop,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerCharacterTransform {
    pub eval_time_offset: f32,
    pub eval_time_width: f32,
    pub loop_type: PerCharacterTransformLoopType,
    pub origin_v: VerticalPosition,
    pub has_anim_info: u8,
    pub origin_v_offset: f32,
    pub fix_space_width: f32,
    pub horizontal_position: HorizontalPosition,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub anim_info: Option<AnimInfo>,
}

impl ReadWriteable for PerCharacterTransform {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let eval_time_offset = cursor.read_f32()?;
        let eval_time_width = cursor.read_f32()?;
        let loop_type = cursor.read_u8()?.into();
        let origin_v = cursor.read_u8()?.into();
        let has_anim_info = cursor.read_u8()?;
        let _reserve0 = cursor.read_u8()?;

        let mut origin_v_offset = 0.0;
        let mut fix_space_width = 0.0;
        let mut horizontal_position = HorizontalPosition::Left;

        if cursor.version.major == 9 {
            origin_v_offset = cursor.read_f32()?;
            fix_space_width = cursor.read_f32()?;
            horizontal_position = cursor.read_u8()?.into();

            cursor.seek_relative(11);
        }

        let mut transform = Self {
            eval_time_offset,
            eval_time_width,
            loop_type,
            origin_v,
            has_anim_info,
            origin_v_offset,
            fix_space_width,
            horizontal_position,
            anim_info: None,
        };

        if transform.has_anim_info != 0 {
            cursor.section_start = Some(cursor.pos);
            transform.anim_info = Some(AnimInfo::parse(cursor)?);
            cursor.section_start = None;
        }

        Ok(transform)
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("PerCharacterTransform");

        writer.write_f32(self.eval_time_offset);
        writer.write_f32(self.eval_time_width);
        writer.write_u8(self.loop_type.into());
        writer.write_u8(self.origin_v.into());
        writer.write_u8(self.has_anim_info);
        writer.write_u8(0);

        if writer.version.major == 9 {
            writer.write_f32(self.origin_v_offset);
            writer.write_f32(self.fix_space_width);
            writer.write_u8(self.horizontal_position.into());

            for _ in 0..11 {
                writer.write_u8(0);
            }
        }

        if let Some(anim_info) = &self.anim_info {
            writer.section_start = Some(writer.pos());
            anim_info.write(writer);
            writer.section_start = None;
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, FromPrimitive, IntoPrimitive, Default)]
#[repr(u8)]
pub enum TextAlignment {
    Synchronous,
    Left,
    Center,
    #[default]
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextBoxPane {
    pub base: Pane,
    pub text_buffer_size: u16,
    pub text_length: u16,
    pub material_index: u16,
    pub font_index: u16,
    pub text_origin: u8,
    pub text_alignment: TextAlignment,
    pub text_flags: TextPaneFlags,
    pub italic_tilt: f32,
    pub font_top_color: Color4u8,
    pub font_bottom_color: Color4u8,

    /// When [`TextPaneFlags::is_keeping_font_scale`] is true it stores  the font scale instead.
    pub font_size: Vector2f,
    pub character_space: f32,
    pub line_space: f32,
    pub shadow_translation: Vector2f,
    pub shadow_size: Vector2f,
    pub shadow_top_color: Color4u8,
    pub shadow_bottom_color: Color4u8,
    pub shadow_italic_tilt: f32,
    pub line_transform_offset: u32,
    pub per_character_transform_offset: u32,
    pub text: Option<String>,
    pub label: Option<String>,
    pub per_character_transform: Option<PerCharacterTransform>,
}

impl ReadWriteable for TextBoxPane {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.ctx_section_start::<Self>()?;
        let base = Pane::parse(cursor)?;
        let txt1_base = section_start + 8;

        let text_buffer_size = cursor.read_u16()?;
        let text_length = cursor.read_u16()?;
        let material_index = cursor.read_u16()?;
        let font_index = cursor.read_u16()?;
        let text_origin = cursor.read_u8()?;
        let text_alignment = cursor.read_u8()?.into();
        let text_flags = TextPaneFlags::decode(cursor.read_u16()?);
        let italic_tilt = cursor.read_f32()?;
        let text_offset = cursor.read_u32()?;
        let font_top_color = Color4u8::parse(cursor)?;
        let font_bottom_color = Color4u8::parse(cursor)?;
        let font_size = Vector2f::parse(cursor)?;
        let character_space = cursor.read_f32()?;
        let line_space = cursor.read_f32()?;
        let label_offset = cursor.read_u32()?;
        let shadow_translation = Vector2f::parse(cursor)?;
        let shadow_size = Vector2f::parse(cursor)?;
        let shadow_top_color = Color4u8::parse(cursor)?;
        let shadow_bottom_color = Color4u8::parse(cursor)?;
        let shadow_italic_tilt = cursor.read_f32()?;
        let line_transform_offset = cursor.read_u32()?;
        let per_character_transform_offset = cursor.read_u32()?;

        let text = if text_offset != 0 {
            let addr = txt1_base + text_offset as usize - 8;
            let saved = cursor.pos;
            cursor.seek(addr)?;

            let char_count = (text_length / 2) as usize;
            let mut utf16_chars = Vec::with_capacity(char_count);

            for _ in 0..char_count {
                utf16_chars.push(cursor.read_u16()?);
            }

            cursor.seek(saved)?;

            String::from_utf16(&utf16_chars).ok()
        } else {
            None
        };

        let label = if label_offset != 0 {
            let addr = txt1_base + label_offset as usize - 8;
            let saved = cursor.pos;
            cursor.seek(addr)?;
            let s = cursor.read_null_terminated_string()?;
            cursor.seek(saved)?;

            Some(s)
        } else {
            None
        };

        let per_character_transform =
            if text_flags.is_per_character_transform && per_character_transform_offset != 0 {
                let addr = txt1_base + per_character_transform_offset as usize - 8;
                cursor.seek(addr)?;

                Some(PerCharacterTransform::parse(cursor)?)
            } else {
                None
            };

        Ok(Self {
            base,
            text_buffer_size,
            text_length,
            material_index,
            font_index,
            text_origin,
            text_alignment,
            text_flags,
            italic_tilt,
            font_top_color,
            font_bottom_color,
            font_size,
            character_space,
            line_space,
            shadow_translation,
            shadow_size,
            shadow_top_color,
            shadow_bottom_color,
            shadow_italic_tilt,
            line_transform_offset,
            per_character_transform_offset,
            text,
            label,
            per_character_transform,
        })
    }

    fn write(&self, writer: &mut Writer) {
        let section_start = writer.ctx_section_start::<Self>();
        let txt1_base = section_start + 8;
        self.base.write(writer);
        writer.mark("TextBoxPane");

        writer.write_u16(self.text_buffer_size);
        writer.write_u16(self.text_length);
        writer.write_u16(self.material_index);
        writer.write_u16(self.font_index);
        writer.write_u8(self.text_origin);
        writer.write_u8(self.text_alignment.into());
        writer.write_u16(self.text_flags.encode());
        writer.write_f32(self.italic_tilt);

        let text_offset_pos = writer.write_placeholder_u32();
        self.font_top_color.write(writer);
        self.font_bottom_color.write(writer);
        self.font_size.write(writer);
        writer.write_f32(self.character_space);
        writer.write_f32(self.line_space);
        let label_offset_pos = writer.write_placeholder_u32();
        self.shadow_translation.write(writer);
        self.shadow_size.write(writer);
        self.shadow_top_color.write(writer);
        self.shadow_bottom_color.write(writer);
        writer.write_f32(self.shadow_italic_tilt);
        writer.write_u32(self.line_transform_offset);
        let per_char_offset_pos = writer.write_placeholder_u32();

        if let Some(text) = &self.text {
            let offset = writer.pos() - txt1_base + 8;
            writer.patch_u32(text_offset_pos, offset as u32);

            for ch in text.encode_utf16() {
                writer.write_u16(ch);
            }
        } else {
            writer.patch_u32(text_offset_pos, 0);
        }

        if let Some(label) = &self.label {
            writer.align(4);
            let offset = writer.pos() - txt1_base + 8;
            writer.patch_u32(label_offset_pos, offset as u32);
            writer.write_null_terminated_string(label);
        } else {
            writer.patch_u32(label_offset_pos, 0);
        }

        if let Some(transform) = &self.per_character_transform {
            writer.align(4);
            let offset = writer.pos() - txt1_base + 8;

            writer.patch_u32(per_char_offset_pos, offset as u32);
            transform.write(writer);
        } else {
            writer.patch_u32(per_char_offset_pos, 0);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowContent {
    pub top_left_vertex_color: Color4u8,
    pub top_right_vertex_color: Color4u8,
    pub bottom_left_vertex_color: Color4u8,
    pub bottom_right_vertex_color: Color4u8,
    pub material_index: u16,
    pub uv_coordinate_count: u8,
    pub picture_uvs: Vec<TextureUv>,
}

impl ReadWriteable for WindowContent {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let top_left_vertex_color = Color4u8::parse(cursor)?;
        let top_right_vertex_color = Color4u8::parse(cursor)?;
        let bottom_left_vertex_color = Color4u8::parse(cursor)?;
        let bottom_right_vertex_color = Color4u8::parse(cursor)?;
        let material_index = cursor.read_u16()?;
        let uv_coordinate_count = cursor.read_u8()?;
        let _reserve0 = cursor.read_u8()?;
        let mut picture_uvs = Vec::with_capacity(uv_coordinate_count as usize);

        for _ in 0..uv_coordinate_count {
            picture_uvs.push(TextureUv::parse(cursor)?);
        }

        Ok(Self {
            top_left_vertex_color,
            top_right_vertex_color,
            bottom_left_vertex_color,
            bottom_right_vertex_color,
            material_index,
            uv_coordinate_count,
            picture_uvs,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("WindowContent");

        self.top_left_vertex_color.write(writer);
        self.top_right_vertex_color.write(writer);
        self.bottom_left_vertex_color.write(writer);
        self.bottom_right_vertex_color.write(writer);
        writer.write_u16(self.material_index);
        writer.write_u8(self.picture_uvs.len() as u8);
        writer.write_u8(0);

        for uv in &self.picture_uvs {
            uv.write(writer);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowFrame {
    pub material_index: u16,
    pub texture_flip_mode: u8,
}

impl ReadWriteable for WindowFrame {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let out = Self {
            material_index: cursor.read_u16()?,
            texture_flip_mode: cursor.read_u8()?,
        };

        let _reserve0 = cursor.read_u8()?;

        Ok(out)
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("WindowFrame");
        writer.write_u16(self.material_index);
        writer.write_u8(self.texture_flip_mode);
        writer.write_u8(0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowPane {
    pub base: Pane,
    pub inflation_left: i16,
    pub inflation_right: i16,
    pub inflation_top: i16,
    pub inflation_bottom: i16,
    pub frame_size_left: i16,
    pub frame_size_right: i16,
    pub frame_size_top: i16,
    pub frame_size_bottom: i16,
    pub frame_count: u8,
    pub flag: WindowFlags,
    pub content: WindowContent,
    pub frames: Vec<WindowFrame>,
}

impl ReadWriteable for WindowPane {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let wnd_base = cursor.pos - 8;
        let base = Pane::parse(cursor)?;

        let inflation_left = cursor.read_i16()?;
        let inflation_right = cursor.read_i16()?;
        let inflation_top = cursor.read_i16()?;
        let inflation_bottom = cursor.read_i16()?;
        let frame_size_left = cursor.read_i16()?;
        let frame_size_right = cursor.read_i16()?;
        let frame_size_top = cursor.read_i16()?;
        let frame_size_bottom = cursor.read_i16()?;

        let frame_count = cursor.read_u8()?;
        let flag = WindowFlags::decode(cursor.read_u8()?);
        let _reserve0 = cursor.read_u16()?;
        let content_offset = cursor.read_u32()?;
        let frame_offset_array_offset = cursor.read_u32()?;

        let restore_point = cursor.pos;
        cursor.seek(wnd_base + content_offset as usize)?;
        let content = WindowContent::parse(cursor)?;

        cursor.seek(wnd_base + frame_offset_array_offset as usize)?;
        let mut frame_offsets = Vec::with_capacity(frame_count as usize);
        for _ in 0..frame_count {
            frame_offsets.push(cursor.read_u32()?);
        }

        let mut frames = Vec::with_capacity(frame_offsets.len());
        for offset in frame_offsets {
            cursor.seek(wnd_base + offset as usize)?;
            frames.push(WindowFrame::parse(cursor)?);
        }

        cursor.seek(restore_point)?;

        Ok(Self {
            base,
            inflation_left,
            inflation_right,
            inflation_top,
            inflation_bottom,
            frame_size_left,
            frame_size_right,
            frame_size_top,
            frame_size_bottom,
            frame_count,
            flag,
            content,
            frames,
        })
    }

    fn write(&self, writer: &mut Writer) {
        let wnd_base = writer.pos() - 8;
        self.base.write(writer);
        writer.mark("WindowPane");

        writer.write_u16(self.inflation_left as u16);
        writer.write_u16(self.inflation_right as u16);
        writer.write_u16(self.inflation_top as u16);
        writer.write_u16(self.inflation_bottom as u16);
        writer.write_u16(self.frame_size_left as u16);
        writer.write_u16(self.frame_size_right as u16);
        writer.write_u16(self.frame_size_top as u16);
        writer.write_u16(self.frame_size_bottom as u16);
        writer.write_u8(self.frames.len() as u8);
        writer.write_u8(self.flag.encode());
        writer.write_u16(0);

        let content_offset_pos = writer.write_placeholder_u32();
        let frame_offsets_pos = writer.write_placeholder_u32();

        let content_off = writer.pos() - wnd_base;
        writer.patch_u32(content_offset_pos, content_off as u32);
        self.content.write(writer);
        writer.align(4);

        let frame_table_off = writer.pos() - wnd_base;
        writer.patch_u32(frame_offsets_pos, frame_table_off as u32);

        let mut frame_off_placeholders = Vec::with_capacity(self.frames.len());
        for _ in &self.frames {
            frame_off_placeholders.push(writer.write_placeholder_u32());
        }

        for (i, frame) in self.frames.iter().enumerate() {
            let frame_off = writer.pos() - wnd_base;
            writer.patch_u32(frame_off_placeholders[i], frame_off as u32);
            frame.write(writer);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartsPaneBasicInfo {
    pub user_name: String,
    pub translation_x: f32,
    pub translation_y: f32,
    pub translation_z: f32,
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub rotation_z: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub size_x: f32,
    pub size_y: f32,
    pub pane_alpha: u8,
}

impl ReadWriteable for PartsPaneBasicInfo {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let out = Self {
            user_name: cursor.read_fixed_string(USER_NAME_LEN)?,
            translation_x: cursor.read_f32()?,
            translation_y: cursor.read_f32()?,
            translation_z: cursor.read_f32()?,
            rotation_x: cursor.read_f32()?,
            rotation_y: cursor.read_f32()?,
            rotation_z: cursor.read_f32()?,
            scale_x: cursor.read_f32()?,
            scale_y: cursor.read_f32()?,
            size_x: cursor.read_f32()?,
            size_y: cursor.read_f32()?,
            pane_alpha: cursor.read_u8()?,
        };
        let _reserve0 = cursor.read_u16()?;

        Ok(out)
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("PaneBasicInfo");
        writer.write_fixed_string(&self.user_name, USER_NAME_LEN);
        writer.write_f32(self.translation_x);
        writer.write_f32(self.translation_y);
        writer.write_f32(self.translation_z);
        writer.write_f32(self.rotation_x);
        writer.write_f32(self.rotation_y);
        writer.write_f32(self.rotation_z);
        writer.write_f32(self.scale_x);
        writer.write_f32(self.scale_y);
        writer.write_f32(self.size_x);
        writer.write_f32(self.size_y);
        writer.write_u8(self.pane_alpha);
        writer.write_u16(0);

        writer.align(4);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TextBoxUsageFlags {
    pub use_text: bool,
    pub use_position: bool,
    pub use_shadow: bool,
}

impl BitPackable<u8> for TextBoxUsageFlags {
    fn decode(flags: u8) -> Self {
        Self {
            use_text: (flags & 0x01) != 0,
            use_position: (flags & 0x02) != 0,
            use_shadow: (flags & 0x04) != 0,
        }
    }

    fn encode(&self) -> u8 {
        let mut flags = 0u8;
        if self.use_text {
            flags |= 0x01;
        }

        if self.use_position {
            flags |= 0x02;
        }

        if self.use_shadow {
            flags |= 0x04;
        }

        flags
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BasePaneUsageFlags {
    pub is_visible_set: bool,
    pub is_visible_true: bool,
    pub has_user_data: bool,
    pub has_translate: bool,
    pub has_size: bool,
    pub has_scale: bool,
    pub has_rotate: bool,
    pub has_alpha: bool,
}

impl BitPackable<u8> for BasePaneUsageFlags {
    fn decode(flags: u8) -> Self {
        Self {
            is_visible_set: (flags & 0x01) != 0,
            is_visible_true: (flags & 0x02) != 0,
            has_user_data: (flags & 0x04) != 0,
            has_translate: (flags & 0x08) != 0,
            has_size: (flags & 0x10) != 0,
            has_scale: (flags & 0x20) != 0,
            has_rotate: (flags & 0x40) != 0,
            has_alpha: (flags & 0x80) != 0,
        }
    }

    fn encode(&self) -> u8 {
        let mut flags = 0u8;
        if self.is_visible_set {
            flags |= 0x01;
        }

        if self.is_visible_true {
            flags |= 0x02;
        }

        if self.has_user_data {
            flags |= 0x04;
        }

        if self.has_translate {
            flags |= 0x08;
        }

        if self.has_size {
            flags |= 0x10;
        }

        if self.has_scale {
            flags |= 0x20;
        }

        if self.has_rotate {
            flags |= 0x40;
        }

        if self.has_alpha {
            flags |= 0x80;
        }

        flags
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct MaterialUsageFlags {
    pub use_color_blend: bool,
    pub use_texture: bool,
    pub use_picture_procedural_shape: bool,
}

impl BitPackable<u8> for MaterialUsageFlags {
    fn decode(flags: u8) -> Self {
        Self {
            use_color_blend: (flags & 0x01) != 0,
            use_texture: (flags & 0x02) != 0,
            use_picture_procedural_shape: (flags & 0x08) != 0,
        }
    }

    fn encode(&self) -> u8 {
        let mut flags = 0u8;
        if self.use_color_blend {
            flags |= 0x01;
        }

        if self.use_texture {
            flags |= 0x02;
        }

        if self.use_picture_procedural_shape {
            flags |= 0x08;
        }

        flags
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartsProperty {
    pub property_name: String,
    pub usage_flag: TextBoxUsageFlags,
    pub basic_usage_flag: BasePaneUsageFlags,
    pub material_usage_flag: MaterialUsageFlags,
    pub user_data_type: u8,
    pub is_sentinel_override: bool,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub o_section: Option<BflytSection>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub o_user_data: Option<BflytSection>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub o_basic_info: Option<PartsPaneBasicInfo>,
}

impl PartsProperty {
    pub fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let last_parts_pane = cursor.ctx_section_start::<Self>()?;

        let mut property = Self {
            property_name: cursor.read_fixed_string(PANE_NAME_LEN)?,
            usage_flag: TextBoxUsageFlags::decode(cursor.read_u8()?),
            basic_usage_flag: BasePaneUsageFlags::decode(cursor.read_u8()?),
            material_usage_flag: MaterialUsageFlags::decode(cursor.read_u8()?),
            user_data_type: cursor.read_u8()?,
            o_section: None,
            o_user_data: None,
            o_basic_info: None,
            is_sentinel_override: false,
        };

        let pane_offset = cursor.read_u32()?;
        let user_data_offset = cursor.read_u32()?;
        let pane_basic_info_offset = cursor.read_u32()?;
        property.is_sentinel_override = user_data_offset == 1;

        let restore_point = cursor.pos;

        if pane_offset > 0 {
            cursor.seek(last_parts_pane + pane_offset as usize)?;

            cursor.is_embed = true;
            let pane = BflytSection::parse(cursor)?;

            property.o_section = Some(pane);
            cursor.seek(restore_point)?;
        }

        if user_data_offset > 1 {
            cursor.seek(last_parts_pane + user_data_offset as usize)?;

            cursor.is_embed = true;
            let user_data = BflytSection::parse(cursor)?;

            property.o_user_data = Some(user_data);
            cursor.seek(restore_point)?;
        }

        if pane_basic_info_offset > 0 {
            cursor.seek(last_parts_pane + pane_basic_info_offset as usize)?;
            let basic_info = PartsPaneBasicInfo::parse(cursor)?;

            property.o_basic_info = Some(basic_info);
            cursor.seek(restore_point)?;
        }

        Ok(property)
    }

    pub fn serialize_header(
        &self,
        writer: &mut Writer,
    ) -> (Placeholder32, Placeholder32, Placeholder32) {
        writer.write_fixed_string(&self.property_name, PANE_NAME_LEN);
        writer.write_u8(self.usage_flag.encode());
        writer.write_u8(self.basic_usage_flag.encode());
        writer.write_u8(self.material_usage_flag.encode());
        writer.write_u8(self.user_data_type);

        let pane_pos = writer.write_placeholder_u32();
        let user_data_pos = writer.write_placeholder_u32();
        let basic_info_pos = writer.write_placeholder_u32();

        (pane_pos, user_data_pos, basic_info_pos)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartsPane {
    pub base: Pane,
    pub magnify_x: f32,
    pub magnify_y: f32,
    pub properties: Vec<PartsProperty>,
    pub o_layout_name: String,
}

impl ReadWriteable for PartsPane {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let base_offset = cursor.pos;
        let base = Pane::parse(cursor)?;

        let property_count = cursor.read_u32()?;
        let magnify_x = cursor.read_f32()?;
        let magnify_y = cursor.read_f32()?;

        let props_start = cursor.pos;

        let mut properties = Vec::with_capacity(property_count as usize);

        for _ in 0..property_count {
            cursor.section_start = Some(base_offset - 8);
            let property = PartsProperty::parse(cursor)?;
            cursor.section_start = None;
            properties.push(property);
        }

        let o_layout_name = cursor.read_null_terminated_string()?;
        cursor.seek(props_start + 0x28 * property_count as usize + o_layout_name.len() + 1)?;

        Ok(Self {
            base,
            magnify_x,
            magnify_y,
            properties,
            o_layout_name,
        })
    }

    fn write(&self, writer: &mut Writer) {
        let section_start = writer.ctx_section_start::<Self>();
        self.base.write(writer);
        writer.mark("PartsPane");

        writer.write_u32(self.properties.len() as u32);
        writer.write_f32(self.magnify_x);
        writer.write_f32(self.magnify_y);

        let mut patch_offsets = Vec::with_capacity(self.properties.len());
        for prop in &self.properties {
            let (p, u, b) = prop.serialize_header(writer);
            patch_offsets.push((p, u, b, prop));
        }

        writer.write_null_terminated_string(&self.o_layout_name);
        writer.align(4);

        for (p_pos, u_pos, b_pos, prop) in patch_offsets {
            if let Some(section) = &prop.o_section {
                let start = writer.pos();

                section.write(writer);
                writer.patch_u32(p_pos, (start - section_start) as u32);
            }

            if let Some(data) = &prop.o_user_data {
                let start = writer.pos();
                data.write(writer);
                writer.patch_u32(u_pos, (start - section_start) as u32);
            } else if prop.is_sentinel_override {
                writer.patch_u32(u_pos, 1);
            } else {
                writer.patch_u32(u_pos, 0);
            }

            if let Some(info) = &prop.o_basic_info {
                let start = writer.pos();
                info.write(writer);
                writer.patch_u32(b_pos, (start - section_start) as u32);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlignmentPane {
    pub base: Pane,
    pub direction: u32,
    pub default_margin: f32,
    pub is_align_last_pane: bool,
    pub is_vertical_alignment: bool,
}

impl ReadWriteable for AlignmentPane {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let base = Pane::parse(cursor)?;

        let out = Self {
            base,
            direction: cursor.read_u32()?,
            default_margin: cursor.read_f32()?,
            is_align_last_pane: cursor.read_u8()? != 0,
            is_vertical_alignment: cursor.read_u8()? != 0,
        };

        let _reserve0 = cursor.read_u16()?;

        Ok(out)
    }

    fn write(&self, writer: &mut Writer) {
        self.base.write(writer);
        writer.mark("AlignmentPane");

        writer.write_u32(self.direction);
        writer.write_f32(self.default_margin);
        writer.write_u8(self.is_align_last_pane.into());
        writer.write_u8(self.is_vertical_alignment.into());
        writer.write_u16(0);
    }
}
