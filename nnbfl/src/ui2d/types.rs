use serde::{Deserialize, Serialize};

use crate::core::{Cursor, FormatError, ReadWriteable, Writer};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct Color4f {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ReadWriteable for Color4f {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            r: cursor.read_f32()?,
            g: cursor.read_f32()?,
            b: cursor.read_f32()?,
            a: cursor.read_f32()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("Color4f");

        writer.write_f32(self.r);
        writer.write_f32(self.g);
        writer.write_f32(self.b);
        writer.write_f32(self.a);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct Color4u8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ReadWriteable for Color4u8 {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            r: cursor.read_u8()?,
            g: cursor.read_u8()?,
            b: cursor.read_u8()?,
            a: cursor.read_u8()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.write_u8(self.r);
        writer.write_u8(self.g);
        writer.write_u8(self.b);
        writer.write_u8(self.a);
    }
}

impl From<Color4u8> for Color4f {
    fn from(color: Color4u8) -> Self {
        Self {
            r: color.r as f32 / 255.0,
            g: color.g as f32 / 255.0,
            b: color.b as f32 / 255.0,
            a: color.a as f32 / 255.0,
        }
    }
}

impl From<Color4f> for Color4u8 {
    fn from(color: Color4f) -> Self {
        let to_u8 = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        Self {
            r: to_u8(color.r),
            g: to_u8(color.g),
            b: to_u8(color.b),
            a: to_u8(color.a),
        }
    }
}

impl From<[f32; 4]> for Color4f {
    fn from(arr: [f32; 4]) -> Self {
        Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        }
    }
}

impl From<Color4f> for [f32; 4] {
    fn from(color: Color4f) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

impl From<[u8; 4]> for Color4u8 {
    fn from(arr: [u8; 4]) -> Self {
        Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        }
    }
}

impl From<Color4u8> for [u8; 4] {
    fn from(color: Color4u8) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

impl From<Color4u8> for [f32; 4] {
    #[inline]
    fn from(color: Color4u8) -> Self {
        [
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            color.a as f32 / 255.0,
        ]
    }
}

impl From<[f32; 4]> for Color4u8 {
    #[inline]
    fn from(arr: [f32; 4]) -> Self {
        let to_u8 = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        Self {
            r: to_u8(arr[0]),
            g: to_u8(arr[1]),
            b: to_u8(arr[2]),
            a: to_u8(arr[3]),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct Vector2f {
    pub x: f32,
    pub y: f32,
}

impl Vector2f {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl ReadWriteable for Vector2f {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            x: cursor.read_f32()?,
            y: cursor.read_f32()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("Vector2f");

        writer.write_f32(self.x);
        writer.write_f32(self.y);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl ReadWriteable for Vector3f {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            x: cursor.read_f32()?,
            y: cursor.read_f32()?,
            z: cursor.read_f32()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("Vector3f");

        writer.write_f32(self.x);
        writer.write_f32(self.y);
        writer.write_f32(self.z);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct VertexPos {
    pub size_scale_width: f32,
    pub size_scale_height: f32,
    pub position_x_scale: f32,
    pub position_y_scale: f32,
}

impl ReadWriteable for VertexPos {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            size_scale_width: cursor.read_f32()?,
            size_scale_height: cursor.read_f32()?,
            position_x_scale: cursor.read_f32()?,
            position_y_scale: cursor.read_f32()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.mark("VertexPos");
        writer.write_f32(self.size_scale_width);
        writer.write_f32(self.size_scale_height);
        writer.write_f32(self.position_x_scale);
        writer.write_f32(self.position_y_scale);
    }
}

impl std::ops::Add for Vector2f {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::AddAssign for Vector2f {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::Sub for Vector2f {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Mul for Vector2f {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl std::ops::Mul<f32> for Vector2f {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl From<[f32; 2]> for Vector2f {
    fn from(arr: [f32; 2]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
        }
    }
}

impl From<Vector2f> for [f32; 2] {
    fn from(vec: Vector2f) -> [f32; 2] {
        [vec.x, vec.y]
    }
}

impl std::ops::Add for Vector3f {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl std::ops::AddAssign for Vector3f {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl std::ops::Sub for Vector3f {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl std::ops::Mul for Vector3f {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl std::ops::Mul<f32> for Vector3f {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl From<[f32; 3]> for Vector3f {
    fn from(arr: [f32; 3]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }
}

impl From<Vector3f> for [f32; 3] {
    fn from(vec: Vector3f) -> [f32; 3] {
        [vec.x, vec.y, vec.z]
    }
}
