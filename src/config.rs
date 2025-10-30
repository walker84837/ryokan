use crate::error::AppError;
use log::{error, warn};
use serde_derive::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

const NOTES_FOLDER: &str = "notes";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub pin_hash: String,
    pub notes_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pin_hash: String::new(),
            notes_dir: NOTES_FOLDER.to_string(),
        }
    }
}

impl Config {
    pub fn new(config_path_param: &Option<PathBuf>) -> Result<Config, AppError> {
        let config_file_path = match config_path_param.to_owned() {
            Some(p) => p,
            None => Self::config_file_path()?,
        };

        // Ensure the parent directory for the config file exists
        std::fs::create_dir_all(
            config_file_path
                .parent()
                .ok_or_else(|| AppError::Config("Invalid config file path.".to_string()))?,
        )
        .map_err(AppError::Io)?;

        let mut config = if !config_file_path.exists() {
            let default_config = Config::default();
            std::fs::write(
                &config_file_path,
                toml::to_string(&default_config).map_err(AppError::TomlSerialize)?,
            )?;
            default_config
        } else {
            match std::fs::read_to_string(&config_file_path) {
                Ok(config_str) => Self::parse_config(config_str)?,
                Err(e) => {
                    error!("Error while reading the configuration: {e}");
                    warn!("Using default configuration");
                    Config::default()
                }
            }
        };

        // Ensure notes_dir is an absolute path
        if !PathBuf::from(&config.notes_dir).is_absolute() {
            let mut absolute_notes_dir = config_file_path.clone();
            absolute_notes_dir.pop(); // Remove config file name
            absolute_notes_dir.push(&config.notes_dir);
            config.notes_dir = absolute_notes_dir.to_string_lossy().to_string();
        }

        // Create the notes directory if it doesn't exist
        fs::create_dir_all(&config.notes_dir).map_err(AppError::Io)?;

        Ok(config)
    }

    /// Parse the TOML config from a string.
    /// Returns default configuration if it fails to parse.
    fn parse_config(config_str: String) -> Result<Config, AppError> {
        match toml::from_str(&config_str) {
            Ok(config) => Ok(config),
            Err(e) => {
                error!("Error while parsing the configuration: {e}");
                warn!("Using default configuration");
                Ok(Config::default())
            }
        }
    }

    /// Save the config to a file
    pub fn save(&self) -> Result<(), AppError> {
        let config_str = toml::to_string(self).map_err(AppError::TomlSerialize)?;
        let config_path = Self::config_file_path()?;

        std::fs::create_dir_all(
            config_path
                .parent()
                .ok_or_else(|| AppError::Config("Invalid config path.".to_string()))?,
        )
        .map_err(AppError::Io)?;

        std::fs::write(config_path, config_str).map_err(AppError::Io)?;
        Ok(())
    }

    fn config_file_path() -> Result<PathBuf, AppError> {
        let mut config_path = dirs::config_dir()
            .ok_or_else(|| AppError::Config("Could not determine config directory.".to_string()))?;
        config_path.push("ryokan"); // Changed from cryptnote to ryokan
        config_path.push("ryokan.toml"); // Changed from cryptnote.toml to ryokan.toml
        Ok(config_path)
    }
}
