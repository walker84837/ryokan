use crate::config::Config;
use crate::error::AppError;
use aes_gcm::Key;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use log::{info, warn};
use std::io;

const MAX_PIN_LENGTH: usize = 6;

pub fn ask_for_pin() -> Result<String, AppError> {
    println!("Please enter your 6-digit PIN:");
    let mut pin = String::new();
    io::stdin().read_line(&mut pin).map_err(AppError::Io)?;

    let trimmed_pin = pin.trim().to_string();
    if trimmed_pin.len() != MAX_PIN_LENGTH {
        return Err(AppError::Pin("PIN must be 6 digits.".to_string()));
    }
    Ok(trimmed_pin)
}

pub fn load_pin_hash(config: &Config) -> Result<Option<String>, AppError> {
    let pin_hash = &config.pin_hash;
    if pin_hash.is_empty() {
        Ok(None)
    } else {
        Ok(Some(pin_hash.to_string()))
    }
}

pub fn store_pin(config: &mut Config, pin: &str) -> Result<(), AppError> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(pin.as_bytes(), &salt)
        .map_err(|e| AppError::PinHash(format!("Failed to hash PIN: {}", e)))?
        .to_string();
    config.pin_hash = hash;
    info!("Saving configuration file to config path");
    config
        .save()
        .map_err(|e| AppError::Config(format!("Failed to save config: {}", e)))?;
    Ok(())
}

pub fn verify_pin(config: &Config, pin: &str) -> Result<bool, AppError> {
    match load_pin_hash(config)? {
        Some(stored_hash) => {
            let argon2 = Argon2::default();
            if stored_hash.is_empty() {
                warn!("Stored hash seems to be empty, assuming PIN can't be verified.");
                return Ok(false);
            }
            let parsed_hash = argon2::PasswordHash::new(&stored_hash).map_err(|e| {
                AppError::PinHash(format!("Failed to parse stored PIN hash: {}", e))
            })?;
            info!("Verifying PIN");
            Ok(argon2.verify_password(pin.as_bytes(), &parsed_hash).is_ok())
        }
        None => Ok(false),
    }
}

pub fn derive_key_from_pin(pin: &str, salt: &[u8]) -> Result<Key<aes_gcm::Aes256Gcm>, AppError> {
    let argon2 = Argon2::default();
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(pin.as_bytes(), salt, &mut key)
        .map_err(|e| AppError::PinHash(format!("Key derivation failed: {}", e)))?;
    Ok(*Key::<aes_gcm::Aes256Gcm>::from_slice(&key))
}
