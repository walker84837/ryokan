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
    pub fn new(config_path: &Option<PathBuf>) -> Result<Config, AppError> {
        let path = match config_path.to_owned() {
            Some(p) => p,
            None => Self::config_file_path()?,
        };

        if !path.exists() {
            fs::create_dir_all(NOTES_FOLDER).map_err(AppError::Io)?;
            return Ok(Config::default());
        }
        match std::fs::read_to_string(&path) {
            Ok(config_str) => Self::parse_config(config_str),
            Err(e) => {
                error!("Error while reading the configuration: {e}");
                warn!("Using default configuration");
                fs::create_dir_all(NOTES_FOLDER).map_err(AppError::Io)?;
                Ok(Config::default())
            }
        }
    }

    /// Parse the TOML config from a string.
    /// Returns default configuration if it fails to parse.
    fn parse_config(config_str: String) -> Result<Config, AppError> {
        match toml::from_str(&config_str) {
            Ok(config) => Ok(config),
            Err(e) => {
                error!("Error while parsing the configuration: {e}");
                warn!("Using default configuration");
                fs::create_dir_all(NOTES_FOLDER).map_err(AppError::Io)?;
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
        config_path.push("cryptnote");
        config_path.push("cryptnote.toml");
        Ok(config_path)
    }
}
