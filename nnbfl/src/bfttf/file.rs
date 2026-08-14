use std::fs;
use std::path::Path;

use crate::core::{
    Cursor, FileConverter, FileReadWriteable, FormatError, NnbflError, ReadWriteable, Writer,
};

#[derive(Debug, Clone, Copy)]
pub enum BinaryFontPlatform {
    Nx,
    Cafe,
    Windows,
}

impl BinaryFontPlatform {
    pub fn key(self) -> u32 {
        match self {
            Self::Nx => 0x49621806,
            Self::Cafe => 0x8CF2DCD9,
            Self::Windows => 0xA6018502,
        }
    }

    pub fn magic(self) -> u32 {
        match self {
            Self::Nx => 0x36F81A1E,
            Self::Cafe => 0xF368DEC1,
            Self::Windows => 0xD99B871A,
        }
    }

    pub fn from_magic(magic: u32) -> Result<Self, FormatError> {
        match magic {
            0x36F81A1E => Ok(Self::Nx),
            0xF368DEC1 => Ok(Self::Cafe),
            0xD99B871A => Ok(Self::Windows),
            _ => Err(FormatError::MissingLayout),
        }
    }
}

struct BinaryFont;

impl BinaryFont {
    pub(crate) fn decrypt(data: &[u8]) -> Result<(Vec<u8>, BinaryFontPlatform), FormatError> {
        if data.len() < 8 {
            return Err(FormatError::MissingLayout);
        }

        let magic = u32::from_be_bytes(data[0..4].try_into().unwrap());
        let platform = BinaryFontPlatform::from_magic(magic)?;
        let key = platform.key();

        let length = u32::from_be_bytes(data[4..8].try_into().unwrap()) ^ key;

        if data.len() < (length as usize + 8) {
            return Err(FormatError::MissingLayout);
        }

        let mut out = Vec::with_capacity(length as usize);

        for chunk in data[8..8 + length as usize].chunks_exact(4) {
            let value = u32::from_be_bytes(chunk.try_into().unwrap()) ^ key;
            out.extend_from_slice(&value.to_be_bytes());
        }

        Ok((out, platform))
    }

    pub(crate) fn encrypt(data: &[u8], platform: BinaryFontPlatform) -> Vec<u8> {
        let key = platform.key();

        let mut out = Vec::with_capacity(data.len() + 8);

        out.extend_from_slice(&platform.magic().to_be_bytes());
        out.extend_from_slice(&((data.len() as u32) ^ key).to_be_bytes());

        for chunk in data.chunks_exact(4) {
            let value = u32::from_be_bytes(chunk.try_into().unwrap()) ^ key;
            out.extend_from_slice(&value.to_be_bytes());
        }

        out
    }
}

#[derive(Debug)]
pub struct BinaryFontFile {
    pub data: Vec<u8>,
    pub platform: BinaryFontPlatform,
}

impl ReadWriteable for BinaryFontFile {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let (data, platform) = BinaryFont::decrypt(cursor.data)?;

        Ok(Self { data, platform })
    }

    fn write(&self, writer: &mut Writer) {
        let encrypted = BinaryFont::encrypt(&self.data, self.platform);
        writer.write_bytes(&encrypted);
    }
}

#[derive(Debug)]
pub struct Bfttf {
    pub font: BinaryFontFile,
}

impl FileReadWriteable for Bfttf {
    const INPUT_EXTENSION: &'static str = "bfttf";
}

impl ReadWriteable for Bfttf {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            font: BinaryFontFile::parse(cursor)?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        self.font.write(writer);
    }
}

impl FileConverter for Bfttf {
    const OUTPUT_EXTENSION: &'static str = "ttf";

    fn extract(&self, output: &Path) -> Result<(), NnbflError> {
        fs::write(output, &self.font.data).map_err(|e| NnbflError::Io {
            path: output.to_path_buf(),
            source: e,
        })
    }

    fn pack(data: &[u8]) -> Result<Self, NnbflError> {
        Ok(Self {
            font: BinaryFontFile {
                data: data.to_vec(),
                platform: BinaryFontPlatform::Windows,
            },
        })
    }
}

#[derive(Debug)]
pub struct Bfotf {
    pub font: BinaryFontFile,
}

impl FileReadWriteable for Bfotf {
    const INPUT_EXTENSION: &'static str = "bfotf";
}

impl ReadWriteable for Bfotf {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        Ok(Self {
            font: BinaryFontFile::parse(cursor)?,
        })
    }

    fn write(&self, writer: &mut Writer) {
        self.font.write(writer);
    }
}

impl FileConverter for Bfotf {
    const OUTPUT_EXTENSION: &'static str = "otf";

    fn extract(&self, output: &Path) -> Result<(), NnbflError> {
        fs::write(output, &self.font.data).map_err(|e| NnbflError::Io {
            path: output.to_path_buf(),
            source: e,
        })
    }

    fn pack(data: &[u8]) -> Result<Self, NnbflError> {
        Ok(Self {
            font: BinaryFontFile {
                data: data.to_vec(),
                platform: BinaryFontPlatform::Windows,
            },
        })
    }
}
