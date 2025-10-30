use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
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

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string(&self)?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let toml_string = std::fs::read_to_string(path)?;
        let metadata: NoteMetadata = toml::from_str(&toml_string)?;
        Ok(metadata)
    }
}
