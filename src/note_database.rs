use crate::error::AppError;
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

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
    pub fn from_config<P: AsRef<Path>>(config_path: Option<P>) -> Result<Self, AppError> {
        let db_path = Self::get_database_path_from_config(config_path)?;

        let map = if db_path.exists() {
            let data = fs::read_to_string(&db_path).map_err(AppError::Io)?;
            serde_json::from_str(&data).map_err(AppError::SerdeJson)?
        } else {
            HashMap::new()
        };
        Ok(Self {
            map,
            database_path: db_path,
        })
    }

    /// Given the config's path, it gets its parent folder and appends the database's name to it.
    fn get_database_path_from_config<P: AsRef<Path>>(
        config_path: Option<P>,
    ) -> Result<PathBuf, AppError> {
        let mut path = match config_path {
            Some(p) => p.as_ref().to_path_buf(),
            None => Self::default_config_dir()?,
        };
        path.pop(); // remove config file name
        path.push("note_database.json");
        Ok(path)
    }

    fn default_config_dir() -> Result<PathBuf, AppError> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| AppError::Config("Could not determine config directory.".to_string()))?;
        path.push("cryptnote");
        Ok(path)
    }

    pub fn save(&self) -> Result<(), AppError> {
        let data = serde_json::to_string(&self.map).map_err(AppError::SerdeJson)?;

        fs::create_dir_all(self.database_path.parent().ok_or_else(|| {
            AppError::Io(io::Error::new(
                io::ErrorKind::Other,
                "Invalid database path.",
            ))
        })?)
        .map_err(AppError::Io)?;
        fs::write(&self.database_path, data).map_err(AppError::Io)?;
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
