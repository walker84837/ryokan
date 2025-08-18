use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoteDatabaseError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize, Default)]
pub struct NoteDatabase {
    /// Maps UUID to original filename
    map: HashMap<String, String>,
}

impl NoteDatabase {
    pub fn new(config_path: &Option<PathBuf>) -> Self {
        let path = Self::database_path(config_path);
        let map = if path.exists() {
            let data = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self { map }
    }

    fn database_path(config_path: &Option<PathBuf>) -> PathBuf {
        let mut path = match config_path {
            Some(p) => p.clone(),
            None => Self::default_config_dir(),
        };
        path.pop(); // Remove config file name
        path.push("note_database.json");
        path
    }

    fn default_config_dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap();
        path.push("cryptnote");
        path
    }

    pub fn save(&self, config_path: &Option<PathBuf>) -> Result<(), NoteDatabaseError> {
        let path = Self::database_path(config_path);
        let data = serde_json::to_string(&self.map)?;
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn insert(&mut self, uuid: String, original_filename: String) {
        self.map.insert(uuid, original_filename);
    }

    pub fn get(&self, uuid: &str) -> Option<&String> {
        self.map.get(uuid)
    }
}
