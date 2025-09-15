use log::{error, warn};
use serde_derive::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use thiserror::Error;

const NOTES_FOLDER: &str = "notes";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub pin_hash: String,
    pub notes_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        fs::create_dir_all(NOTES_FOLDER).unwrap();
        Self {
            pin_hash: String::new(),
            notes_dir: NOTES_FOLDER.to_string(),
        }
    }
}

#[derive(Error, Debug)]
pub enum FileError {
    #[error("Failed to write file: {0}")]
    WriteError(String),
    #[error("Failed to serialize config: {0}")]
    SerializeError(String),
}

impl Config {
    pub fn new(config_path: &Option<PathBuf>) -> Config {
        let path = match config_path.to_owned() {
            Some(p) => p,
            None => Self::config_file_path(),
        };

        if !path.exists() {
            return Config::default();
        }
        match std::fs::read_to_string(path) {
            Ok(config_str) => Self::parse_config(config_str),
            Err(e) => {
                error!("Error while reading the configuration: {e}");
                warn!("Using default configuration");
                Config::default()
            }
        }
    }

    /// Parse the TOML config from a string.
    /// Returns default configuration if it fails to parse.
    fn parse_config(config_str: String) -> Config {
        match toml::from_str(&config_str) {
            Ok(config) => config,
            Err(e) => {
                error!("Error while parsing the configuration: {e}");
                warn!("Using default configuration");
                Config::default()
            }
        }
    }

    /// Save the config to a file
    pub fn save(&self) -> Result<(), FileError> {
        let config_str =
            toml::to_string(self).map_err(|e| FileError::SerializeError(e.to_string()))?;
        let config_path = Self::config_file_path();

        std::fs::create_dir_all(config_path.parent().unwrap())
            .map_err(|e| FileError::WriteError(e.to_string()))?;

        std::fs::write(config_path, config_str)
            .map_err(|e| FileError::WriteError(e.to_string()))?;
        Ok(())
    }

    fn config_file_path() -> PathBuf {
        let mut config_path = dirs::config_dir().unwrap();
        config_path.push("cryptnote");
        config_path.push("cryptnote.toml");
        config_path
    }
}
