use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NnbflError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Format error: {0}")]
    Format(#[from] FormatError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] SerializationError),

    #[error("The specified path does not exist: {0}")]
    MissingPath(PathBuf),

    #[error("Command execution failed for batch operation")]
    BatchFailure,
}

#[derive(Error, Debug)]
pub enum FormatError {
    #[error(
        "Invalid file signature: expected '{expected}', found 0x{found:08X} at offset 0x{offset:X}"
    )]
    InvalidMagic {
        expected: &'static str,
        found: u32,
        offset: usize,
    },

    #[error(
        "Unexpected End-of-File (EOF) at offset 0x{offset:X} (attempted to read {requested_bytes} bytes)"
    )]
    UnexpectedEof {
        offset: usize,
        requested_bytes: usize,
    },

    #[error(
        "Header size field specifies 0x{specified_size:X} bytes, but the file buffer is only 0x{actual_size:X} bytes"
    )]
    InvalidHeaderSize {
        specified_size: usize,
        actual_size: usize,
    },

    #[error(
        "Section count mismatch: header expects {expected} sections, but file stream cut off at section {actual}. Inner error: {source}"
    )]
    SectionCountMismatch {
        expected: u32,
        actual: u32,
        #[source]
        source: Box<FormatError>,
    },

    #[error("Malformed section block '{section_type}' at offset 0x{offset:X}: {reason}")]
    MalformedSection {
        section_type: String,
        offset: usize,
        reason: String,
    },

    #[error(
        "File is missing a layout section which is mandatory, this means your file is either incomplete or corrupted."
    )]
    MissingLayout,

    #[error("Missing parsing context '{expected}' at byte offset {offset}")]
    MissingContext {
        expected: &'static str,
        offset: usize,
    },

    #[error(
        "Unknown or unsupported enum tag 0x{tag:08X} for type '{enum_name}' at offset 0x{offset:X}"
    )]
    UnknownTag {
        enum_name: &'static str,
        tag: u32,
        offset: usize,
    },

    #[error("Invalid layout hierarchy: {0}")]
    InvalidHierarchyChange(&'static str),
}

#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("JSON Syntax error at line {line}, column {column}: {message}")]
    JsonSyntax {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Semantic mismatch in JSON structure: {reason}")]
    SemanticMismatch { reason: String },
}

impl From<serde_json::Error> for SerializationError {
    fn from(err: serde_json::Error) -> Self {
        if err.is_data() {
            SerializationError::SemanticMismatch {
                reason: err.to_string(),
            }
        } else {
            SerializationError::JsonSyntax {
                line: err.line(),
                column: err.column(),
                message: err.to_string(),
            }
        }
    }
}
