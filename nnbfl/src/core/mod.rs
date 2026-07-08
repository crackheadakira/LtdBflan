mod cursor;
mod error;
mod section;
mod tests;
mod writer;

pub use cursor::Cursor;
pub use error::{FormatError, NnbflError};
pub use section::{SectionHeader, SectionMagic};
pub use writer::{Placeholder16, Placeholder32, Writer};

pub const fn tchar_code32(b: &[u8; 4]) -> u32 {
    (b[0] as u32) | ((b[1] as u32) << 8) | ((b[2] as u32) << 16) | ((b[3] as u32) << 24)
}

pub trait BitPackable<T> {
    fn decode(raw: T) -> Self;
    fn encode(&self) -> T;
}

pub trait ReadWriteable: Sized {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError>;
    fn write(&self, writer: &mut Writer);
}

pub trait FileReadWriteable: ReadWriteable {
    const INPUT_EXTENSION: &'static str;

    fn parse_file(file: &[u8]) -> Result<Self, FormatError> {
        let mut cursor = Cursor {
            data: file,
            pos: 0,
            ..Default::default()
        };

        Self::parse(&mut cursor)
    }

    fn write_file(&self) -> Writer {
        let mut writer = Writer::new();
        self.write(&mut writer);

        writer
    }
}

pub trait FileConverter: FileReadWriteable {
    const OUTPUT_EXTENSION: &'static str;

    fn extract(&self, output: &std::path::Path) -> Result<(), NnbflError>;
    fn pack(data: &[u8]) -> Result<Self, NnbflError>;
}

pub trait JsonFileConverter:
    FileConverter + serde::Serialize + serde::de::DeserializeOwned
{
}

impl<T> FileConverter for T
where
    T: FileReadWriteable + serde::Serialize + serde::de::DeserializeOwned,
{
    const OUTPUT_EXTENSION: &'static str = "json";

    fn extract(&self, output: &std::path::Path) -> Result<(), NnbflError> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| NnbflError::Serialization(e.into()))?;

        std::fs::write(output, json).map_err(|e| NnbflError::Io {
            path: output.to_path_buf(),
            source: e,
        })
    }

    fn pack(data: &[u8]) -> Result<Self, NnbflError> {
        serde_json::from_slice(data).map_err(|e| NnbflError::Serialization(e.into()))
    }
}

impl<T> JsonFileConverter for T where
    T: FileReadWriteable + serde::Serialize + serde::de::DeserializeOwned
{
}

#[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, Copy)]
pub struct VersionFormat {
    pub major: u8,
    pub minor: u8,
    pub micro: u16,
}

impl ReadWriteable for VersionFormat {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(VersionFormat {
            micro: cursor.read_u16()?,
            minor: cursor.read_u8()?,
            major: cursor.read_u8()?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        writer.write_u16(self.micro);
        writer.write_u8(self.minor);
        writer.write_u8(self.major);
    }
}
