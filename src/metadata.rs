use crate::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoteMetadata {
    pub original_filename: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl NoteMetadata {
    pub fn new<S: Into<String>>(original_filename: S) -> Self {
        let now = Utc::now();
        Self {
            original_filename: original_filename.into(),
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        let toml_string = toml::to_string(&self).map_err(AppError::TomlSerialize)?;
        std::fs::write(path, toml_string).map_err(AppError::Io)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, AppError> {
        let toml_string = std::fs::read_to_string(path).map_err(AppError::Io)?;
        let metadata: NoteMetadata =
            toml::from_str(&toml_string).map_err(AppError::TomlDeserialize)?;
        Ok(metadata)
    }
}
