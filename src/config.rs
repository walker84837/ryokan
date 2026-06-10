use crate::error::AppError;
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::prelude::*,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const NOTES_FOLDER: &str = "notes";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub pin_hash: String,
    pub notes_dir: String,
    #[serde(skip)]
    pub config_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pin_hash: String::new(),
            notes_dir: NOTES_FOLDER.to_string(),
            config_path: PathBuf::new(),
        }
    }
}

impl Config {
    pub fn new(config_path_param: Option<&PathBuf>) -> Result<Config, AppError> {
        let config_file_path = match config_path_param {
            Some(p) => p.clone(),
            None => Self::default_config_file_path()?,
        };

        // Make sure the parent directory for the config file exists
        Self::ensure_parent_dir(&config_file_path)?;

        let mut config = if config_file_path.exists() {
            match fs::read_to_string(&config_file_path) {
                Ok(config_str) => Self::parse_config(&config_str)?,
                Err(e) => {
                    error!("Error while reading the configuration: {e}");
                    return Err(AppError::Io(e));
                }
            }
        } else {
            let default_config = Config {
                config_path: config_file_path.clone(),
                ..Default::default()
            };
            default_config.save()?;
            default_config
        };

        config.config_path = config_file_path;

        if !Path::new(&config.notes_dir).is_absolute()
            && let Some(parent) = config.config_path.parent()
        {
            config.notes_dir = parent.join(&config.notes_dir).to_string_lossy().to_string();
        }

        // Create the notes directory if it doesn't exist
        fs::create_dir_all(&config.notes_dir).map_err(AppError::Io)?;

        Ok(config)
    }

    pub fn notes_dir_path(&self) -> &Path {
        Path::new(&self.notes_dir)
    }

    fn ensure_parent_dir(path: &Path) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(AppError::Io)?;
        }
        Ok(())
    }

    /// Parse the TOML config from a string.
    /// Returns default configuration if it fails to parse.
    fn parse_config(config_str: &str) -> Result<Config, AppError> {
        toml::from_str(config_str).map_err(|e| {
            error!("Error while parsing the configuration: {e}");
            AppError::TomlDeserialize(e)
        })
    }

    /// Save the config to a file
    pub fn save(&self) -> Result<(), AppError> {
        let config_str = toml::to_string(self).map_err(AppError::TomlSerialize)?;
        let config_path = &self.config_path;

        Self::ensure_parent_dir(config_path)?;

        let parent = config_path
            .parent()
            .ok_or_else(|| AppError::Config("Invalid config path".to_string()))?;
        let mut temp_file = tempfile::NamedTempFile::new_in(parent).map_err(AppError::Io)?;

        #[cfg(unix)]
        {
            fs::set_permissions(temp_file.path(), fs::Permissions::from_mode(0o600))
                .map_err(AppError::Io)?;
        }

        temp_file
            .write_all(config_str.as_bytes())
            .map_err(AppError::Io)?;
        temp_file
            .persist(config_path)
            .map_err(|e| AppError::Io(e.error))?;

        Ok(())
    }

    fn default_config_file_path() -> Result<PathBuf, AppError> {
        let mut config_path = dirs::config_dir()
            .ok_or_else(|| AppError::Config("Could not determine config directory.".to_string()))?;
        config_path.push("ryokan");
        config_path.push("ryokan.toml");
        Ok(config_path)
    }
}
