use crate::error::AppError;
use crate::pin;
use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, AeadCore, KeyInit},
};
use rand::{self, RngCore, rngs::OsRng};

pub fn encrypt_note_content(content: &[u8], pin: &str) -> Result<Vec<u8>, AppError> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);

    let key = pin::derive_key_from_pin(pin, &salt)?;
    let cipher = Aes256Gcm::new(&key);

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, content)
        .map_err(|e| AppError::Encryption(format!("Encryption failed: {}", e)))?;

    Ok([salt.as_slice(), nonce.as_slice(), &ciphertext].concat())
}

pub fn decrypt_note_content(encrypted_data: &[u8], pin: &str) -> Result<Vec<u8>, AppError> {
    let (salt, remainder) = encrypted_data.split_at(16);
    let (nonce_slice, ciphertext) = remainder.split_at(12);

    let key = pin::derive_key_from_pin(pin, salt)?;
    let cipher = Aes256Gcm::new(&key);

    let nonce = aes_gcm::Nonce::from_slice(nonce_slice);

    let decrypted = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AppError::Decryption(format!("Decryption failed: {}", e)))?;

    Ok(decrypted)
}
