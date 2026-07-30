use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};

use crate::core::{
    BitPackable, Cursor, Endianness, FileReadWriteable, FormatError, Placeholder32, ReadWriteable,
    VersionFormat, Writer, tchar_code32,
};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Bfcpx {
    pub endianness: Endianness,
    pub version: VersionFormat,
    pub description: BfcpxDescription,
}

impl FileReadWriteable for Bfcpx {
    const INPUT_EXTENSION: &'static str = "bfcpx";
}

impl ReadWriteable for Bfcpx {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let magic = cursor.read_u32()?;
        if magic != tchar_code32(b"FCPX") {
            return Err(FormatError::InvalidMagic {
                expected: "FCPX",
                found: magic,
                offset: 0,
            });
        }

        let endianness = Endianness::from_u16(cursor.read_u16()?)?;
        let header_size = cursor.read_u16()?;
        let version = VersionFormat::parse(cursor)?;
        cursor.version = version;
        let _file_size = cursor.read_u32()?;
        let _section_count = cursor.read_u32()?;

        let section_start = cursor.pos;

        let root_offset = cursor.read_u32()?;

        if (header_size as usize) > cursor.data.len() {
            return Err(FormatError::InvalidHeaderSize {
                specified_size: header_size as usize,
                actual_size: cursor.data.len(),
            });
        }

        cursor.seek(section_start + root_offset as usize)?;

        let description = BfcpxDescription::parse(cursor)?;

        Ok(Self {
            endianness,
            version,
            description,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.version = self.version;

        writer.mark("File header");
        writer.write_bytes(b"FCPX");
        writer.write_u16(self.endianness.to_u16());
        let header_size = writer.write_placeholder_u16();
        self.version.write(writer);

        let file_size_pos = writer.write_placeholder_u32();
        let section_count = match self.description {
            BfcpxDescription::Multi(_) => 2,
            _ => 1,
        };

        writer.write_u32(section_count);

        writer.patch_u16(header_size, writer.pos() as u16);

        writer.write_u32(4);

        self.description.write(writer);

        let total_size = writer.pos() as u32;
        writer.patch_u32(file_size_pos, total_size);
    }
}

#[derive(Debug, Default, IntoPrimitive, FromPrimitive)]
#[repr(u32)]
pub enum BfcpxDescriptionType {
    #[default]
    Res,
    Scalable,
    Multi,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BfcpxDescription {
    Res(DescriptionRes),
    Scalable(DescriptionScalable),
    Multi(DescriptionMulti),
}

impl ReadWriteable for BfcpxDescription {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.pos;

        cursor.section_start = Some(section_start);

        let description_type: BfcpxDescriptionType = cursor.read_u32()?.into();

        let out = match description_type {
            BfcpxDescriptionType::Multi => Self::Multi(DescriptionMulti::parse(cursor)?),
            BfcpxDescriptionType::Scalable => Self::Scalable(DescriptionScalable::parse(cursor)?),
            BfcpxDescriptionType::Res => Self::Res(DescriptionRes::parse(cursor)?),
        };

        cursor.section_start = None;

        Ok(out)
    }

    fn write(&self, writer: &mut Writer) {
        let section_start = writer.pos();
        writer.section_start = Some(section_start);

        let description_type = match self {
            Self::Res(_) => BfcpxDescriptionType::Res,
            Self::Scalable(_) => BfcpxDescriptionType::Scalable,
            Self::Multi(_) => BfcpxDescriptionType::Multi,
        };

        writer.write_u32(description_type.into());

        match self {
            Self::Res(r) => r.write(writer),
            Self::Scalable(s) => s.write(writer),
            Self::Multi(m) => m.write(writer),
        }

        writer.section_start = None;
    }
}

impl Default for BfcpxDescription {
    fn default() -> Self {
        Self::Scalable(Default::default())
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DescriptionMulti {
    pub child: Box<BfcpxDescription>,
    pub next: Box<BfcpxDescription>,
}

impl ReadWriteable for DescriptionMulti {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.ctx_section_start::<DescriptionMulti>()?;

        let child_offset = cursor.read_u32()?;
        let next_offset = cursor.read_u32()?;

        cursor.seek(section_start + child_offset as usize)?;

        let child = Box::new(BfcpxDescription::parse(cursor)?);

        cursor.seek(section_start + next_offset as usize)?;
        let next = Box::new(BfcpxDescription::parse(cursor)?);

        Ok(Self { child, next })
    }

    fn write(&self, writer: &mut Writer) {
        let section_start = writer.ctx_section_start::<DescriptionMulti>();

        let child_offset = writer.write_placeholder_u32();
        let next_offset = writer.write_placeholder_u32();

        writer.patch_u32(child_offset, (writer.pos() - section_start) as u32);
        self.child.write(writer);

        writer.patch_u32(next_offset, (writer.pos() - section_start) as u32);
        self.next.write(writer);
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DescriptionScalable {
    pub width: f32,
    pub line_feed_delta: u32,
    pub replace_char: char,
    pub font_array: Vec<BfcpxFont>,
}

impl ReadWriteable for DescriptionScalable {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.ctx_section_start::<DescriptionScalable>()?;

        let width = cursor.read_f32()?;
        let line_feed_delta = cursor.read_u32()?;

        let font_count = cursor.read_u32()?;
        let base_offset = cursor.read_u32()?;
        let replace_u16 = cursor.read_u16()?;
        let replace_char =
            char::from_u32(replace_u16 as u32).unwrap_or(char::REPLACEMENT_CHARACTER);

        let mut font_array = Vec::with_capacity(font_count as usize);

        let restore = cursor.pos;
        cursor.seek(section_start + base_offset as usize)?;

        for _ in 0..font_count {
            font_array.push(BfcpxFont::parse(cursor)?);
        }

        cursor.seek(restore)?;

        Ok(Self {
            width,
            line_feed_delta,
            replace_char,
            font_array,
        })
    }

    fn write(&self, writer: &mut Writer) {
        let section_start = writer.ctx_section_start::<DescriptionMulti>();

        writer.write_f32(self.width);
        writer.write_u32(self.line_feed_delta);
        writer.write_u32(self.font_array.len() as u32);

        let base_offset = writer.write_placeholder_u32();
        writer.write_u16(self.replace_char as u16);

        writer.align(14);

        writer.patch_u32(base_offset, (writer.pos() - section_start) as u32);

        let mut patch_list = Vec::with_capacity(self.font_array.len());
        for font in &self.font_array {
            let patches = font.write_header(writer);
            patch_list.push(patches);
        }

        for (font, patches) in self.font_array.iter().zip(patch_list.iter()) {
            let range_rel_offset = (writer.pos() - section_start) as u32;
            writer.patch_u32(patches.1, range_rel_offset);

            for range in &font.char_ranges {
                range.write(writer);
                writer.align(4);
            }
        }

        for (font, patches) in self.font_array.iter().zip(patch_list.iter()) {
            let name_rel_offset = (writer.pos() - section_start) as u32;
            writer.patch_u32(patches.0, name_rel_offset);

            writer.write_null_terminated_string(&font.font_name);
            writer.align(4);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DescriptionRes {
    pub font_name: String,
    pub char_ranges: Vec<CharRange>,
}

impl ReadWriteable for DescriptionRes {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let section_start = cursor.ctx_section_start::<DescriptionRes>()?;

        let name_offset = cursor.read_u32()?;
        let range_count = cursor.read_u32()?;
        let range_offset = cursor.read_u32()?;

        cursor.seek(section_start + name_offset as usize)?;
        let font_name = cursor.read_null_terminated_string()?;

        cursor.seek(section_start + range_offset as usize)?;
        let mut char_ranges = Vec::with_capacity(range_count as usize);

        for _ in 0..range_count {
            char_ranges.push(CharRange::parse(cursor)?)
        }

        Ok(Self {
            font_name,
            char_ranges,
        })
    }

    fn write(&self, writer: &mut Writer) {
        let section_start = writer.ctx_section_start::<DescriptionMulti>();

        let name_offset = writer.write_placeholder_u32();
        writer.write_u32(self.char_ranges.len() as u32);
        let range_offset = writer.write_placeholder_u32();

        writer.patch_u32(range_offset, (writer.pos() - section_start) as u32);
        for range in &self.char_ranges {
            range.write(writer);
        }

        writer.patch_u32(name_offset, (writer.pos() - section_start) as u32);
        writer.write_null_terminated_string(&self.font_name);
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CharRange {
    pub min: u32,
    pub max: u32,
}

impl ReadWriteable for CharRange {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            min: cursor.read_u32()?,
            max: cursor.read_u32()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.write_u32(self.min);
        writer.write_u32(self.max);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct BfcpxFontFlags {
    pub is_ignore_proportional_alternate_width: bool,
    pub is_override_bearing_x: bool,
    pub is_override_char_width: bool,
}

impl BitPackable<u8> for BfcpxFontFlags {
    fn decode(raw: u8) -> Self {
        Self {
            is_ignore_proportional_alternate_width: (raw & 0x01) != 0,
            is_override_bearing_x: ((raw >> 1) & 0x01) != 0,
            is_override_char_width: ((raw >> 2) & 0x01) != 0,
        }
    }

    fn encode(&self) -> u8 {
        let mut raw = 0u8;

        if self.is_ignore_proportional_alternate_width {
            raw |= 0b001;
        }

        if self.is_override_bearing_x {
            raw |= 0b010;
        }

        if self.is_override_char_width {
            raw |= 0b100;
        }

        raw
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BfcpxFont {
    pub bold_weight: f32,
    pub width_scale: f32,
    pub font_face_id: u32,
    pub outline_width: u8,
    pub flags: BfcpxFontFlags,
    pub baseline_offset: u16,
    pub height_scale: f32,
    pub bearing_x: u16,
    pub override_char_width: u16,
    pub ascent: u16,
    pub descent: u16,
    pub font_name: String,
    pub char_ranges: Vec<CharRange>,
}
impl BfcpxFont {
    pub fn write_header(&self, writer: &mut Writer) -> (Placeholder32, Placeholder32) {
        writer.write_f32(self.bold_weight);
        writer.write_f32(self.width_scale);
        writer.write_u32(self.font_face_id);

        let name_offset_pos = writer.write_placeholder_u32();

        writer.write_u8(self.outline_width);
        writer.write_u8(self.flags.encode());
        writer.write_u16(self.baseline_offset);
        writer.write_f32(self.height_scale);
        writer.write_u16(self.bearing_x);
        writer.write_u16(self.override_char_width);
        writer.write_u16(self.ascent);
        writer.write_u16(self.descent);

        writer.write_u32(self.char_ranges.len() as u32);

        let range_array_offset_pos = writer.write_placeholder_u32();

        (name_offset_pos, range_array_offset_pos)
    }

    pub fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let base_offset = cursor.ctx_section_start::<BfcpxFont>()?;

        let bold_weight = cursor.read_f32()?;
        let width_scale = cursor.read_f32()?;
        let font_face_id = cursor.read_u32()?;

        let name_offset = cursor.read_u32()?;
        let outline_width = cursor.read_u8()?;
        let flags = BfcpxFontFlags::decode(cursor.read_u8()?);
        let baseline_offset = cursor.read_u16()?;
        let height_scale = cursor.read_f32()?;
        let bearing_x = cursor.read_u16()?;
        let override_char_width = cursor.read_u16()?;
        let ascent = cursor.read_u16()?;
        let descent = cursor.read_u16()?;

        let range_count = cursor.read_u32()?;
        let range_array_offset = cursor.read_u32()?;

        let restore = cursor.pos;

        let mut char_ranges = Vec::with_capacity(range_count as usize);
        cursor.seek(base_offset + range_array_offset as usize)?;

        for _ in 0..range_count {
            char_ranges.push(CharRange::parse(cursor)?)
        }

        cursor.seek(base_offset + name_offset as usize)?;
        let font_name = cursor.read_null_terminated_string()?;

        cursor.seek(restore)?;

        Ok(Self {
            bold_weight,
            width_scale,
            font_face_id,
            outline_width,
            flags,
            baseline_offset,
            height_scale,
            bearing_x,
            override_char_width,
            ascent,
            descent,
            font_name,
            char_ranges,
        })
    }
}
