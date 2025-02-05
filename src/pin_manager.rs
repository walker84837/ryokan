use crate::config::Config;
use aes_gcm::{Aes256Gcm, Key};
use argon2::{password_hash::Salt, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::Rng;
use std::{io, process};

// Struct for managing PIN-related operations
pub(crate) struct PinManager;

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
        let config = Config::new();
        config.pin_hash
    }

    pub fn store_pin(pin: &str) {
        let mut config = Config::new();
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
