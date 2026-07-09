use serde::{Deserialize, Serialize};

use crate::core::{Cursor, FormatError, Writer};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Curve {
    Constant(Vec<f32>),
    Step(Vec<StepKey>),
    Hermite(Vec<HermiteKey>),
}

impl Default for Curve {
    fn default() -> Self {
        Self::Constant(Vec::new())
    }
}

impl Curve {
    pub fn parse(
        cursor: &mut Cursor,
        curve_type: u8,
        frame_count: usize,
    ) -> Result<Self, FormatError> {
        let out = match curve_type {
            0 => {
                let mut keys = Vec::with_capacity(frame_count);
                for _ in 0..frame_count {
                    keys.push(cursor.read_f32()?);
                }
                Self::Constant(keys)
            }
            1 => {
                let mut keys = Vec::with_capacity(frame_count);
                for _ in 0..frame_count {
                    keys.push(StepKey {
                        frame: cursor.read_f32()?,
                        value: cursor.read_u16()?,
                    });

                    cursor.seek_relative(2);
                }
                Self::Step(keys)
            }
            2 => {
                let mut keys = Vec::with_capacity(frame_count);
                for _ in 0..frame_count {
                    keys.push(HermiteKey {
                        frame: cursor.read_f32()?,
                        value: cursor.read_f32()?,
                        slope: cursor.read_f32()?,
                    });
                }
                Self::Hermite(keys)
            }
            _ => Self::Constant(Vec::new()),
        };

        Ok(out)
    }

    pub fn serialize(&self, writer: &mut Writer) {
        match &self {
            Self::Constant(keys) => {
                for key in keys {
                    writer.write_f32(*key);
                }
            }
            Self::Step(keys) => {
                for key in keys {
                    writer.write_f32(key.frame);
                    writer.write_u16(key.value);
                    writer.write_u16(0);
                }
            }
            Self::Hermite(keys) => {
                for key in keys {
                    writer.write_f32(key.frame);
                    writer.write_f32(key.value);
                    writer.write_f32(key.slope);
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct StepKey {
    pub frame: f32,
    pub value: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HermiteKey {
    pub frame: f32,
    pub value: f32,
    pub slope: f32,
}
