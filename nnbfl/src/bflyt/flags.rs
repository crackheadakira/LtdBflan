use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};

use crate::core::BitPackable;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, IntoPrimitive, FromPrimitive, Default)]
#[repr(u8)]
pub enum BflytOrigin {
    #[default]
    Center,
    LeftTop,
    RightBottom,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, IntoPrimitive, FromPrimitive, Default)]
#[repr(u8)]
pub enum BflytParentOrigin {
    #[default]
    None,
    LeftTop,
    RightBottom,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct BflytOrigins {
    pub origin_x: BflytOrigin,
    pub origin_y: BflytOrigin,
    pub parent_origin_x: BflytParentOrigin,
    pub parent_origin_y: BflytParentOrigin,
}

impl BitPackable<u8> for BflytOrigins {
    fn decode(raw: u8) -> Self {
        Self {
            origin_x: (raw & 0x03).into(),
            origin_y: ((raw >> 2) & 0x03).into(),
            parent_origin_x: ((raw >> 4) & 0x03).into(),
            parent_origin_y: (raw >> 6).into(),
        }
    }

    fn encode(&self) -> u8 {
        let mut raw = 0u8;

        raw |= (self.origin_x as u8) & 0x03;
        raw |= ((self.origin_y as u8) & 0x03) << 2;
        raw |= ((self.parent_origin_x as u8) & 0x03) << 4;
        raw |= ((self.parent_origin_y as u8) & 0x03) << 6;

        raw
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct PaneFlags {
    pub is_visible: bool,
    pub influenced_alpha: bool,
    pub location_adjust: bool,
    pub user_allocated: bool,
    pub is_global_matrix_dirty: bool,
    pub is_srt_matrix_user: bool,
    pub is_global_matrix_user: bool,
    pub is_constant_buffer_ready: bool,
}

impl BitPackable<u8> for PaneFlags {
    fn decode(raw: u8) -> Self {
        Self {
            is_visible: (raw & 0x01) != 0,
            influenced_alpha: ((raw >> 1) & 0x01) != 0,
            location_adjust: ((raw >> 2) & 0x01) != 0,
            user_allocated: ((raw >> 3) & 0x01) != 0,
            is_global_matrix_dirty: ((raw >> 4) & 0x01) != 0,
            is_srt_matrix_user: ((raw >> 5) & 0x01) != 0,
            is_global_matrix_user: ((raw >> 6) & 0x01) != 0,
            is_constant_buffer_ready: ((raw >> 7) & 0x01) != 0,
        }
    }

    fn encode(&self) -> u8 {
        let mut raw = 0u8;

        if self.is_visible {
            raw |= 0b1;
        }

        if self.influenced_alpha {
            raw |= 0b10;
        }

        if self.location_adjust {
            raw |= 0b100;
        }

        if self.user_allocated {
            raw |= 0b1000;
        }

        if self.is_global_matrix_dirty {
            raw |= 0b10000;
        }

        if self.is_srt_matrix_user {
            raw |= 0b100000;
        }

        if self.is_global_matrix_user {
            raw |= 0b1000000;
        }

        if self.is_constant_buffer_ready {
            raw |= 0b10000000;
        }

        raw
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct PaneFlagsEx {
    pub is_no_scale_by_parts: bool,
    pub is_scale_size_by_parts_root: bool,
    pub is_ext_user_data: bool,
    pub reserve0: u8,
}

impl BitPackable<u8> for PaneFlagsEx {
    fn decode(raw: u8) -> Self {
        Self {
            is_no_scale_by_parts: (raw & 0x01) != 0,
            is_scale_size_by_parts_root: ((raw >> 1) & 0x01) != 0,
            is_ext_user_data: ((raw >> 2) & 0x01) != 0,
            reserve0: raw >> 3,
        }
    }

    fn encode(&self) -> u8 {
        let mut raw = 0u8;

        if self.is_no_scale_by_parts {
            raw |= 0x01;
        }

        if self.is_scale_size_by_parts_root {
            raw |= 0x02;
        }

        if self.is_ext_user_data {
            raw |= 0x04;
        }

        raw |= (self.reserve0 << 3) & 0xF8;
        raw
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct TextPaneFlags {
    pub is_enable_shadow: bool,
    pub is_limit_glyph_count_to_length: bool,
    pub is_invisible_border: bool,
    pub is_double_border: bool,
    pub is_per_character_transform: bool,
    pub is_enable_center_ceiling: bool,
    pub is_line_transform: bool,
    pub is_default_tag_processor: bool,
    pub is_per_character_transform_split_by_char_width: bool,
    pub is_mix_shadow_alpha: bool,
    pub is_reverse: bool,
    pub is_per_character_transform_origin_to_center: bool,
    pub is_keeping_font_scale: bool,
    pub is_per_character_transform_fix_space: bool,
    pub is_per_character_transform_split_insert_space: bool,
}

impl BitPackable<u16> for TextPaneFlags {
    fn decode(raw: u16) -> Self {
        Self {
            is_enable_shadow: (raw & (1 << 0)) != 0,
            is_limit_glyph_count_to_length: (raw & (1 << 1)) != 0,
            is_invisible_border: (raw & (1 << 2)) != 0,
            is_double_border: (raw & (1 << 3)) != 0,
            is_per_character_transform: (raw & (1 << 4)) != 0,
            is_enable_center_ceiling: (raw & (1 << 5)) != 0,
            is_line_transform: (raw & (1 << 6)) != 0,
            is_default_tag_processor: (raw & (1 << 7)) != 0,
            is_per_character_transform_split_by_char_width: (raw & (1 << 8)) != 0,
            is_mix_shadow_alpha: (raw & (1 << 9)) != 0,
            is_reverse: (raw & (1 << 10)) != 0,
            is_per_character_transform_origin_to_center: (raw & (1 << 11)) != 0,
            is_keeping_font_scale: (raw & (1 << 12)) != 0,
            is_per_character_transform_fix_space: (raw & (1 << 13)) != 0,
            is_per_character_transform_split_insert_space: (raw & (1 << 14)) != 0,
        }
    }

    fn encode(&self) -> u16 {
        let mut raw = 0u16;

        if self.is_enable_shadow {
            raw |= 1 << 0;
        }

        if self.is_limit_glyph_count_to_length {
            raw |= 1 << 1;
        }

        if self.is_invisible_border {
            raw |= 1 << 2;
        }

        if self.is_double_border {
            raw |= 1 << 3;
        }

        if self.is_per_character_transform {
            raw |= 1 << 4;
        }

        if self.is_enable_center_ceiling {
            raw |= 1 << 5;
        }

        if self.is_line_transform {
            raw |= 1 << 6;
        }

        if self.is_default_tag_processor {
            raw |= 1 << 7;
        }

        if self.is_per_character_transform_split_by_char_width {
            raw |= 1 << 8;
        }

        if self.is_mix_shadow_alpha {
            raw |= 1 << 9;
        }

        if self.is_reverse {
            raw |= 1 << 10;
        }

        if self.is_per_character_transform_origin_to_center {
            raw |= 1 << 11;
        }

        if self.is_keeping_font_scale {
            raw |= 1 << 12;
        }

        if self.is_per_character_transform_fix_space {
            raw |= 1 << 13;
        }

        if self.is_per_character_transform_split_insert_space {
            raw |= 1 << 14;
        }

        raw
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum WindowKind {
    #[default]
    Around,
    Horizontal,
    HorizontalNoContent,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct WindowFlags {
    pub use_layout_material: bool,
    pub use_vertex_color_for_all_window: bool,
    pub window_kind: WindowKind,
    pub not_draw_content: bool,
}

impl BitPackable<u8> for WindowFlags {
    fn decode(raw: u8) -> Self {
        let kind_bits = (raw >> 2) & 0x03;
        Self {
            use_layout_material: (raw & 0x01) != 0,
            use_vertex_color_for_all_window: ((raw >> 1) & 0x01) != 0,
            window_kind: match kind_bits {
                0 => WindowKind::Around,
                1 => WindowKind::Horizontal,
                2 => WindowKind::HorizontalNoContent,
                _ => WindowKind::Around,
            },
            not_draw_content: ((raw >> 4) & 0x01) != 0,
        }
    }

    fn encode(&self) -> u8 {
        let mut raw = 0u8;
        if self.use_layout_material {
            raw |= 1 << 0;
        }
        if self.use_vertex_color_for_all_window {
            raw |= 1 << 1;
        }
        raw |= ((self.window_kind as u8) & 0x03) << 2;
        if self.not_draw_content {
            raw |= 1 << 4;
        }
        raw
    }
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum TexWrapMode {
    #[default]
    Clamp,
    Repeat,
    Mirror,
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive, Default,
)]
#[repr(u8)]
pub enum TexResourceType {
    #[default]
    LocalFile,
    LocalCaptured,
    OverrideCaptured,
    LocalVectorGraphicsFile,
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive, Default,
)]
#[repr(u8)]
pub enum TexFilter {
    #[default]
    Near,
    Linear,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub struct TexOptions {
    pub wrap_mode: TexWrapMode,
    pub filter_mode: TexFilter,
}

impl BitPackable<u8> for TexOptions {
    fn decode(raw: u8) -> Self {
        Self {
            wrap_mode: (raw & 0x03).into(),
            filter_mode: (raw >> 2).into(),
        }
    }

    fn encode(&self) -> u8 {
        (self.wrap_mode as u8 & 0x03) | ((self.filter_mode as u8 & 0x3F) << 2)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub struct DropShadowFlags {
    pub is_stroke_enabled: bool,
    pub is_outer_glow_enabled: bool,
    pub is_drop_shadow_enabled: bool,
    pub is_knockout: bool,
    pub is_only_effect: bool,
    pub is_static_rendering: bool,
    pub is_degamma_enabled: bool,
}

impl BitPackable<u8> for DropShadowFlags {
    fn decode(raw: u8) -> Self {
        Self {
            is_stroke_enabled: (raw & 0x01) != 0,
            is_outer_glow_enabled: ((raw >> 1) & 0x01) != 0,
            is_drop_shadow_enabled: ((raw >> 2) & 0x01) != 0,
            is_knockout: ((raw >> 3) & 0x01) != 0,
            is_only_effect: ((raw >> 4) & 0x01) != 0,
            is_static_rendering: ((raw >> 5) & 0x01) != 0,
            is_degamma_enabled: ((raw >> 6) & 0x01) != 0,
        }
    }

    fn encode(&self) -> u8 {
        let mut raw = 0u8;
        if self.is_stroke_enabled {
            raw |= 1 << 0;
        }

        if self.is_outer_glow_enabled {
            raw |= 1 << 1;
        }

        if self.is_drop_shadow_enabled {
            raw |= 1 << 2;
        }

        if self.is_knockout {
            raw |= 1 << 3;
        }

        if self.is_only_effect {
            raw |= 1 << 4;
        }

        if self.is_static_rendering {
            raw |= 1 << 5;
        }

        if self.is_degamma_enabled {
            raw |= 1 << 6;
        }
        raw
    }
}
