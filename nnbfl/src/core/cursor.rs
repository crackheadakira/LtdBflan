use crate::core::{FormatError, VersionFormat};

#[derive(Default)]
pub struct Cursor<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub version: VersionFormat,
}

impl<'a> Cursor<'a> {
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], FormatError> {
        let end = self.pos + len;

        if end > self.data.len() {
            return Err(FormatError::UnexpectedEof {
                offset: self.pos,
                requested_bytes: len,
            });
        }

        let slice = &self.data[self.pos..end];
        self.pos = end;

        Ok(slice)
    }

    pub fn peek_bytes(&self, len: usize) -> Result<&'a [u8], FormatError> {
        let end = self.pos + len;

        if end > self.data.len() {
            return Err(FormatError::UnexpectedEof {
                offset: self.pos,
                requested_bytes: len,
            });
        }

        Ok(&self.data[self.pos..end])
    }

    pub fn read_u8(&mut self) -> Result<u8, FormatError> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0])
    }

    pub fn read_u16(&mut self) -> Result<u16, FormatError> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn read_u32(&mut self) -> Result<u32, FormatError> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_u64(&mut self) -> Result<u64, FormatError> {
        let b = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn peek_u32(&self) -> Result<u32, FormatError> {
        let b = self.peek_bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_i16(&mut self) -> Result<i16, FormatError> {
        let b = self.read_bytes(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    pub fn read_i32(&mut self) -> Result<i32, FormatError> {
        let b = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_f32(&mut self) -> Result<f32, FormatError> {
        let b = self.read_bytes(4)?;
        Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_string(&mut self, len: usize) -> Result<String, FormatError> {
        let bytes = self.read_bytes(len)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    pub fn read_fixed_string(&mut self, len: usize) -> Result<String, FormatError> {
        let remaining_bytes = self.data.len().saturating_sub(self.pos);

        if remaining_bytes == 0 && len > 0 {
            return Err(FormatError::UnexpectedEof {
                offset: self.pos,
                requested_bytes: len,
            });
        }

        let actual_len = len.min(remaining_bytes);
        let bytes = self.read_bytes(actual_len)?;
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(actual_len);

        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }

    pub fn read_null_terminated_string(&mut self) -> Result<String, FormatError> {
        let start = self.pos;
        let mut end = start;

        while end < self.data.len() && self.data[end] != 0 {
            end += 1;
        }
        if end >= self.data.len() {
            return Err(FormatError::MalformedSection {
                section_type: "StringPool".to_string(),
                offset: start,
                reason: "Unterminated string literal reached EOF".to_string(),
            });
        }

        let bytes = &self.data[start..end];
        self.pos = end + 1;

        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    pub fn seek(&mut self, pos: usize) -> Result<(), FormatError> {
        if pos > self.data.len() {
            return Err(FormatError::UnexpectedEof {
                offset: self.data.len(),
                requested_bytes: pos - self.data.len(),
            });
        }

        self.pos = pos;
        Ok(())
    }

    pub fn seek_relative(&mut self, bytes: usize) {
        self.pos += bytes;
    }
}
