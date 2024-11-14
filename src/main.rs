use aes_gcm::{
    aead::{Aead, KeyInit, Nonce},
    Aes256Gcm, Key,
};
use anyhow::{Context, Result};
use argon2::{password_hash::Salt, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use log::info;
use rand::rngs::OsRng;
use rand::{self, Rng, RngCore};
use serde_derive::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
    process::{self, Command},
};
use uuid::Uuid;

// Config struct for handling configuration loading and saving
#[derive(Default, Serialize, Deserialize)]
struct Config {
    pin_hash: Option<String>,
    notes_dir: Option<String>,
}

impl Config {
    fn load() -> Self {
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

    fn save(&self) {
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

// Struct for managing PIN-related operations
struct PinManager;

impl PinManager {
    pub fn ask_for_pin() -> String {
        println!("Please enter your 6-digit PIN:");
        let mut pin = String::new();
        io::stdin().read_line(&mut pin).expect("Failed to read PIN");
        let pin = pin.trim().to_string();
        if pin.len() != 6 {
            println!("PIN must be 6 digits.");
            process::exit(1);
        }
        pin
    }

    pub fn load_pin_hash() -> Option<String> {
        let config = Config::load();
        config.pin_hash
    }

    pub fn store_pin(pin: &str) {
        let mut config = Config::load();
        let salt = rand::thread_rng().gen::<[u8; 16]>();
        let argon2 = Argon2::default();
        let salt_str = base64::encode(&salt);
        let salt = Salt::from_b64(&salt_str).unwrap();
        let hash = argon2
            .hash_password(pin.as_bytes(), *&salt)
            .map(|p| p.to_string())
            .unwrap();
        config.pin_hash = Some(hash);
        config.save();
    }

    pub fn verify_pin(pin: &str) -> bool {
        if let Some(stored_hash) = Self::load_pin_hash() {
            let argon2 = Argon2::default();
            let stored_hash = PasswordHash::new(&stored_hash).unwrap();
            match argon2.verify_password(pin.as_bytes(), &stored_hash) {
                Ok(_) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub fn derive_key_from_pin(pin: &str, salt: &[u8]) -> Result<Key<Aes256Gcm>> {
        let argon2 = Argon2::default();
        let mut key = [0u8; 32];

        argon2
            .hash_password_into(pin.as_bytes(), salt, &mut key)
            .map_err(|e| anyhow::anyhow!("Argon2 error: {}", e))?;

        Ok(*Key::<Aes256Gcm>::from_slice(&key))
    }
}

// Struct for handling encryption and decryption of notes
struct NoteManager;

impl NoteManager {
    pub fn encrypt_note_content(content: &[u8], pin: &str) -> Result<Vec<u8>> {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);

        let key = PinManager::derive_key_from_pin(pin, &salt)?;
        let cipher = Aes256Gcm::new(&key);

        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);

        let ciphertext = cipher
            .encrypt(Nonce::<Aes256Gcm>::from_slice(&nonce), content)
            .expect("Encryption failed!");

        Ok([salt.to_vec(), nonce.to_vec(), ciphertext].concat())
    }

    pub fn decrypt_note_content(encrypted_data: &[u8], pin: &str) -> Result<Vec<u8>> {
        let (salt, remainder) = encrypted_data.split_at(16);
        let (nonce, ciphertext) = remainder.split_at(12);

        let key = PinManager::derive_key_from_pin(pin, salt)?;
        let cipher = Aes256Gcm::new(&key);

        let decrypted = cipher
            .decrypt(Nonce::<Aes256Gcm>::from_slice(nonce), ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        Ok(decrypted)
    }
}

// Struct for handling notes file operations
struct FileManager;

impl FileManager {
    pub fn save_note_to_file(content: &[u8], filename: &str) -> Result<()> {
        let mut file = File::create(filename)?;
        file.write_all(content)?;
        info!("Note saved to {}", filename);
        Ok(())
    }

    pub fn load_and_decrypt_note_content(filename: &str, pin: &str) -> Result<Vec<u8>> {
        let mut encrypted_data = Vec::new();
        let mut file = File::open(filename).context("Unable to open file")?;
        file.read_to_end(&mut encrypted_data)
            .context("Unable to read file")?;

        NoteManager::decrypt_note_content(&encrypted_data, pin)
    }
}

// Function to get the UUID as filename
fn generate_uuid_filename() -> String {
    let id = Uuid::new_v4().to_string();
    format!("{}.enc.txt", id)
}

// Function to open the note in the default editor
fn open_in_editor(filename: &str) -> Result<()> {
    let editor = if let Ok(editor) = std::env::var("EDITOR") {
        editor
    } else {
        "nano".to_string()
    };

    Command::new(editor).arg(filename).spawn()?.wait()?;
    Ok(())
}

fn main() {
    let pin = PinManager::ask_for_pin();
    PinManager::store_pin(&pin);

    let encrypted_note_content =
        NoteManager::encrypt_note_content(b"My Secret Note", &pin).expect("Encryption failed");
    let filename = generate_uuid_filename();
    FileManager::save_note_to_file(&encrypted_note_content, &filename)
        .expect("Failed to save note");

    // Open the note in the default editor
    open_in_editor(&filename).expect("Failed to open editor");
}
