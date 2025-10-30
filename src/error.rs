use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Decryption error: {0}")]
    Decryption(String),
    #[error("PIN error: {0}")]
    Pin(String),
    #[error("PIN hash error: {0}")]
    PinHash(String),
    #[error("TUI error: {0}")]
    Tui(String),

    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("TOML deserialize error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Metadata error: {0}")]
    Metadata(String),
}

impl From<Box<dyn std::error::Error>> for AppError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        AppError::Metadata(err.to_string())
    }
}
