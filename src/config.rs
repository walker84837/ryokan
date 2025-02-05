use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

// Config struct for handling configuration loading and saving
#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub pin_hash: Option<String>,
    pub notes_dir: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        let config_path = Self::config_file_path();
        if config_path.exists() {
            let config_str =
                std::fs::read_to_string(config_path).expect("Unable to read config file");
            toml::de::from_str(&config_str).unwrap_or_default()
        } else {
            Self {
                pin_hash: None,
                notes_dir: None,
            }
        }
    }

    pub fn save(&self) {
        let config_str = toml::to_string(self).expect("Failed to serialize config");
        std::fs::write(Self::config_file_path(), config_str).expect("Unable to write config file");
    }

    pub fn load_notes_dir() -> Option<String> {
        let config = Config::load();
        config.notes_dir
    }

    fn config_file_path() -> PathBuf {
        let mut config_path = dirs::config_dir().unwrap();
        config_path.push("notes-renamer.toml");
        config_path
    }
}
