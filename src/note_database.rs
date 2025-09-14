use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum NoteDatabaseError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Invalid database state: {0}")]
    DatabaseError(&'static str),
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct NoteDatabase {
    /// Maps UUID to original filename
    map: HashMap<String, String>,
    database_path: PathBuf,
}

#[allow(dead_code)]
impl NoteDatabase {
    /// Given a config path, it initializes the database in the same folder as the config file.
    ///
    /// # Errors
    ///
    /// Fails when the reading the database file, or when it doesn't parse correctly.
    #[must_use]
    pub fn from_config<P: AsRef<Path>>(config_path: Option<P>) -> Option<Self> {
        let config_path = config_path.as_ref().map(|p| p.as_ref().to_path_buf());
        let db_path = Self::get_database_path_from_config(config_path);

        let map = if db_path.exists() {
            let data = fs::read_to_string(&db_path).ok()?;
            serde_json::from_str(&data).ok()?
        } else {
            HashMap::new()
        };
        Some(Self {
            map,
            database_path: db_path,
        })
    }

    /// Given the config's path, it gets its parent folder and appends the database's name to it.
    fn get_database_path_from_config<P: AsRef<Path>>(config_path: Option<P>) -> PathBuf {
        let mut path = match config_path {
            Some(p) => p.as_ref().to_path_buf(),
            None => Self::default_config_dir(),
        };
        path.pop(); // remove config file name
        path.push("note_database.json");
        path
    }

    fn default_config_dir() -> PathBuf {
        let mut path = dirs::config_dir().expect("FIXME: come up with decent error message");
        path.push("cryptnote");
        path
    }

    pub fn save(&self) -> Result<(), NoteDatabaseError> {
        let data = serde_json::to_string(&self.map)?;
        fs::create_dir_all(self.database_path.parent().unwrap())?;
        fs::write(&self.database_path, data)?;
        Ok(())
    }

    pub fn insert<S>(&mut self, uuid: S, original_filename: S)
    where
        S: Into<String>,
    {
        self.map.insert(uuid.into(), original_filename.into());
    }

    pub fn get(&self, uuid: impl AsRef<str>) -> Option<&String> {
        self.map.get(uuid.as_ref())
    }
}
