mod cursor;
mod error;
mod section;
mod tests;
mod writer;

pub use cursor::Cursor;
pub use error::{FormatError, NnbflError};
pub use section::SectionHeader;
pub use writer::{Placeholder16, Placeholder32, Writer};

pub const fn tchar_code32(b: &[u8; 4]) -> u32 {
    (b[0] as u32) | ((b[1] as u32) << 8) | ((b[2] as u32) << 16) | ((b[3] as u32) << 24)
}

pub trait ReadWriteable: Sized + serde::Serialize + serde::de::DeserializeOwned {
    const EXTENSION: &'static str;

    fn parse(file: &[u8]) -> Result<Self, FormatError>;
    fn write(&self) -> writer::Writer;
}

#[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, Copy)]
pub struct VersionFormat {
    pub major: u8,
    pub minor: u8,
    pub micro: u16,
}

impl VersionFormat {
    pub fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(VersionFormat {
            micro: cursor.read_u16()?,
            minor: cursor.read_u8()?,
            major: cursor.read_u8()?,
        })
    }

    pub fn serialize(&self, writer: &mut Writer) {
        writer.write_u16(self.micro);
        writer.write_u8(self.minor);
        writer.write_u8(self.major);
    }
}
