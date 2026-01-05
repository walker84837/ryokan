use crate::config::Config;
use crate::error::AppError;
use aes_gcm::Key;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version};
use log::{info, warn};
use std::io::{self, Write};

const MAX_PIN_LENGTH: usize = 6;

pub fn ask_for_pin() -> Result<String, AppError> {
    print!("Please enter your 6-digit PIN: ");
    // Make sure prompt is displayed before reading
    io::stdout().flush().map_err(AppError::Io)?;
    let pin = rpassword::read_password().map_err(AppError::Io)?;

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

// Create a secure Argon2 instance with strong parameters.
// Params chosen for local app with 6-digit PIN:
// - 64 MiB memory: High memory cost to resist GPU/ASIC attacks
// - t=10 iterations: Time cost for additional computational difficulty
// - p=1 parallelism: Sequential to minimize side-channel attacks on PIN verification
// These params balance security for low-entropy PINs against usability on typical hardware.
fn create_argon2<'a>() -> Result<Argon2<'a>, AppError> {
    let params = Params::new(65536, 10, 1, None)
        .map_err(|e| AppError::PinHash(format!("Failed to create Argon2 params: {}", e)))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

pub fn store_pin(config: &mut Config, pin: &str) -> Result<(), AppError> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let argon2 = create_argon2()?;
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
            let argon2 = create_argon2()?;
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
    let argon2 = create_argon2()?;
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(pin.as_bytes(), salt, &mut key)
        .map_err(|e| AppError::PinHash(format!("Key derivation failed: {}", e)))?;
    Ok(*Key::<aes_gcm::Aes256Gcm>::from_slice(&key))
}

pub fn handle_pin_setup_and_verification(config: &mut Config) -> Result<String, AppError> {
    let stored_pin_hash = load_pin_hash(config)?;
    let pin = if let Some(hash) = stored_pin_hash
        && !hash.is_empty()
    {
        loop {
            let entered_pin = ask_for_pin()?;
            if verify_pin(config, &entered_pin)? {
                break entered_pin;
            }
            eprintln!("Incorrect PIN. Please try again.");
        }
    } else {
        eprintln!("No PIN found. Please set a new 6-digit PIN.");
        let new_pin = ask_for_pin()?;
        store_pin(config, &new_pin)?;
        new_pin
    };
    Ok(pin)
}

#[cfg(test)]
mod pin_test;
