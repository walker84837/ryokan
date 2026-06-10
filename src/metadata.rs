use crate::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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

        let parent = path
            .parent()
            .ok_or_else(|| AppError::Config("Invalid metadata path".to_string()))?;

        let mut temp_file = tempfile::NamedTempFile::new_in(parent).map_err(AppError::Io)?;

        temp_file
            .write_all(toml_string.as_bytes())
            .map_err(AppError::Io)?;
        temp_file.persist(path).map_err(|e| AppError::Io(e.error))?;

        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, AppError> {
        let toml_string = fs::read_to_string(path).map_err(AppError::Io)?;
        let metadata: NoteMetadata =
            toml::from_str(&toml_string).map_err(AppError::TomlDeserialize)?;
        Ok(metadata)
    }
}
