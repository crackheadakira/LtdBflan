use nnbfl::core::FormatError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),

    #[error("No BFLYT file found in data payload.")]
    NoBflytFound,

    #[error("Failed to parse BFLYT layout: {0:?}")]
    BflytParse(FormatError),
}
