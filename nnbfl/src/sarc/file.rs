use serde::{Deserialize, Serialize};

use crate::core::{Cursor, FileReadWriteable, FormatError, ReadWriteable, Writer, tchar_code32};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Sarc {
    pub endianness: u16,
    pub version_number: u16,
    pub padding: u16,
    pub hash_multiplier: u32,
    pub data_alignment: u32,
    pub files: Vec<SarcFile>,
}

impl FileReadWriteable for Sarc {
    const INPUT_EXTENSION: &'static str = "blarc";
}

impl ReadWriteable for Sarc {
    fn parse(cursor: &mut Cursor) -> Result<Self, FormatError> {
        let magic = cursor.read_u32()?;
        if magic != tchar_code32(b"SARC") {
            return Err(FormatError::InvalidMagic {
                expected: "SARC",
                found: magic,
                offset: 0,
            });
        }

        let _header_size = cursor.read_u16()?;
        let endianness = cursor.read_u16()?;
        let _file_size = cursor.read_u32()?;
        let data_start_offset = cursor.read_u32()?;
        let version_number = cursor.read_u16()?;
        let padding = cursor.read_u16()?;

        let data_alignment = if data_start_offset % 8192 == 0 {
            8192
        } else if data_start_offset % 4096 == 0 {
            4096
        } else if data_start_offset % 2048 == 0 {
            2048
        } else if data_start_offset % 512 == 0 {
            512
        } else {
            16
        };

        let sfat_magic = cursor.read_u32()?;
        if sfat_magic != tchar_code32(b"SFAT") {
            return Err(FormatError::InvalidMagic {
                expected: "SFAT",
                found: sfat_magic,
                offset: cursor.pos - 4,
            });
        }

        let _sfat_header_size = cursor.read_u16()?;
        let num_files = cursor.read_u16()?;
        let hash_multiplier = cursor.read_u32()?;

        let mut fat_entries = Vec::with_capacity(num_files as usize);
        for _ in 0..num_files {
            let name_hash = cursor.read_u32()?;
            let attributes = cursor.read_u32()?;
            let data_start = cursor.read_u32()?;
            let data_end = cursor.read_u32()?;

            let word_offset = attributes & 0x00FFFFFF;

            fat_entries.push(FatEntry {
                name_hash,
                name_table_offset: word_offset * 4,
                data_start,
                data_end,
            });
        }

        let sfnt_magic = cursor.read_u32()?;
        if sfnt_magic != tchar_code32(b"SFNT") {
            return Err(FormatError::InvalidMagic {
                expected: "SFNT",
                found: sfnt_magic,
                offset: cursor.pos - 4,
            });
        }

        let _sfnt_header_size = cursor.read_u16()?;
        let _sfnt_padding = cursor.read_u16()?;

        let sfnt_pool_start = cursor.pos;
        let mut files = Vec::with_capacity(num_files as usize);

        for entry in fat_entries {
            let data_offset = (data_start_offset + entry.data_start) as usize;
            let data_length = (entry.data_end - entry.data_start) as usize;
            let file_bytes = cursor.data[data_offset..(data_offset + data_length)].to_vec();

            let name = if entry.name_table_offset > 0 || cursor.data[sfnt_pool_start] != 0 {
                let str_start = sfnt_pool_start + entry.name_table_offset as usize;
                let mut str_bytes = Vec::new();
                let mut current_pos = str_start;

                while current_pos < cursor.data.len() && cursor.data[current_pos] != 0 {
                    str_bytes.push(cursor.data[current_pos]);
                    current_pos += 1;
                }

                String::from_utf8(str_bytes).ok()
            } else {
                None
            };

            files.push(SarcFile {
                name,
                hash: entry.name_hash,
                data: file_bytes,
            });
        }

        Ok(Self {
            endianness,
            version_number,
            padding,
            hash_multiplier,
            data_alignment,
            files,
        })
    }

    fn write(&self, writer: &mut Writer) {
        let mut sorted_files = self.files.clone();
        sorted_files.sort_by_key(|file| file.hash);

        let num_files = sorted_files.len() as u16;

        let mut name_table_bytes = Vec::new();
        let mut name_offsets = Vec::with_capacity(sorted_files.len());

        for file in &sorted_files {
            if let Some(name_str) = &file.name {
                let current_offset = name_table_bytes.len() as u32;
                name_offsets.push(Some(current_offset));

                name_table_bytes.extend_from_slice(name_str.as_bytes());
                name_table_bytes.push(0);

                let len = name_table_bytes.len();
                name_table_bytes.resize(len.next_multiple_of(4), 0);
            } else {
                name_offsets.push(None);
            }
        }

        let sfat_size = 0x0C + (num_files as u32 * 0x10);
        let names_size = 0x08 + name_table_bytes.len() as u32;
        let absolute_data_start =
            (0x14 + sfat_size + names_size).next_multiple_of(self.data_alignment);

        let mut relative_cursor: u32 = 0;
        let mut file_layouts = Vec::with_capacity(sorted_files.len());

        for file in &self.files {
            let alignment = if let Some(name) = &file.name {
                if name.ends_with(".bntx") || name.ends_with(".bnsh") || name.ends_with(".baglmf") {
                    self.data_alignment
                } else if name.ends_with(".szs") {
                    64
                } else {
                    16
                }
            } else {
                16
            };

            relative_cursor = relative_cursor.next_multiple_of(alignment);
            let start = relative_cursor;
            let end = start + file.data.len() as u32;

            file_layouts.push((start, end));
            relative_cursor = end;
        }

        writer.mark("SARC Header");
        writer.write_u32(tchar_code32(b"SARC"));
        writer.write_u16(0x14);
        writer.write_u16(self.endianness);

        let total_size_patch = writer.write_placeholder_u32();
        writer.write_u32(absolute_data_start);
        writer.write_u16(self.version_number);
        writer.write_u16(self.padding);

        writer.mark("SFAT Header");
        writer.write_u32(tchar_code32(b"SFAT"));
        writer.write_u16(0x0C);
        writer.write_u16(num_files);
        writer.write_u32(self.hash_multiplier);

        writer.mark("SFAT Entries");
        for (i, file) in sorted_files.iter().enumerate() {
            let (start, end) = file_layouts[i];

            let attrs = if let Some(pool_offset) = name_offsets[i] {
                let aa = 1_u32;
                let bbbbbb = pool_offset / 4;
                (aa << 24) | (bbbbbb & 0x00FFFFFF)
            } else {
                0_u32
            };

            writer.write_u32(file.hash);
            writer.write_u32(attrs);
            writer.write_u32(start);
            writer.write_u32(end);
        }

        writer.mark("SFNT Header");
        writer.write_u32(tchar_code32(b"SFNT"));
        writer.write_u16(0x08);
        writer.write_u16(0);
        writer.write_bytes(&name_table_bytes);

        writer.mark("Data Section");
        for (i, file) in sorted_files.iter().enumerate() {
            let (start, _) = file_layouts[i];
            let target_absolute_pos = absolute_data_start + start;

            let current_pos = writer.pos() as u32;
            if target_absolute_pos > current_pos {
                writer.write_bytes(&vec![0u8; (target_absolute_pos - current_pos) as usize]);
            }

            writer.write_bytes(&file.data);
        }

        let actual_total_size = writer.pos() as u32;
        writer.patch_u32(total_size_patch, actual_total_size);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SarcFile {
    pub name: Option<String>,
    pub hash: u32,
    pub data: Vec<u8>,
}

pub enum MagicFiles {
    Bflyt(Vec<u8>),
    Bflan(Vec<u8>),
    Bntx(Vec<u8>),
    Msbp(Vec<u8>),
    Msbt(Vec<u8>),
    Sarc(Vec<u8>),
    Yaz0(Vec<u8>),
    Zstd(Vec<u8>),
    Unknown(Vec<u8>),
}

impl SarcFile {
    pub fn match_by_magic(self) -> MagicFiles {
        if self.data.len() < 4 {
            return MagicFiles::Unknown(self.data);
        }

        if self.data.len() >= 8 {
            match &self.data[0..8] {
                b"MsgPrjBn" => return MagicFiles::Msbp(self.data),
                b"MsgStdBn" => return MagicFiles::Msbt(self.data),
                _ => {}
            }
        }

        match &self.data[0..4] {
            b"FLYT" => MagicFiles::Bflyt(self.data),
            b"FLAN" => MagicFiles::Bflan(self.data),
            b"BNTX" => MagicFiles::Bntx(self.data),
            b"SARC" => MagicFiles::Sarc(self.data),
            b"Yaz0" => MagicFiles::Yaz0(self.data),
            [0x28, 0xB5, 0x2F, 0xFD] => MagicFiles::Zstd(self.data),
            _ => MagicFiles::Unknown(self.data),
        }
    }
}

struct FatEntry {
    name_hash: u32,
    name_table_offset: u32,
    data_start: u32,
    data_end: u32,
}
