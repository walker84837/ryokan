use crate::config::Config;
use aes_gcm::Key;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use log::{info, warn};
use std::{io, process};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PinManagerError {
    #[error("Key derivation failed: {0}")]
    KeyDerivationError(argon2::password_hash::Error),
}

pub struct PinManager;

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

    pub fn load_pin_hash(config: &Config) -> Option<String> {
        let pin_hash = &config.pin_hash;
        match pin_hash.is_empty() {
            true => None,
            false => Some(pin_hash.to_string()),
        }
    }

    pub fn store_pin(config: &mut Config, pin: &str) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(pin.as_bytes(), &salt)
            .unwrap()
            .to_string();
        config.pin_hash = hash;
        info!("Saving configuration file to config path");
        config.save().unwrap();
    }

    pub fn verify_pin(config: &Config, pin: &str) -> bool {
        match Self::load_pin_hash(config) {
            Some(stored_hash) => {
                let argon2 = Argon2::default();
                if stored_hash.is_empty() {
                    warn!("Stored hash seems to be empty, assuming PIN can't be verified.");
                    return false;
                }
                let parsed_hash = argon2::PasswordHash::new(&stored_hash).unwrap();
                info!("Verifying PIN");
                argon2.verify_password(pin.as_bytes(), &parsed_hash).is_ok()
            }
            None => false,
        }
    }

    pub fn derive_key_from_pin(
        pin: &str,
        salt: &[u8],
    ) -> Result<Key<aes_gcm::Aes256Gcm>, PinManagerError> {
        let argon2 = Argon2::default();
        let mut key = [0u8; 32];
        argon2
            .hash_password_into(pin.as_bytes(), salt, &mut key)
            .map_err(|e| PinManagerError::KeyDerivationError(e.into()))?;
        Ok(*Key::<aes_gcm::Aes256Gcm>::from_slice(&key))
    }
}
