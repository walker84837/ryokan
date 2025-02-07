use crate::pin_manager::PinManagerError;
use crate::PinManager;
use aes_gcm::{
    aead::{Aead, KeyInit, Nonce},
    Aes256Gcm, Key,
};
use rand::{self, rngs::OsRng, RngCore};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoteManagerError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(aes_gcm::Error),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(aes_gcm::Error),
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(#[from] PinManagerError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct NoteManager;

impl NoteManager {
    pub fn encrypt_note_content(content: &[u8], pin: &str) -> Result<Vec<u8>, NoteManagerError> {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);

        let key = PinManager::derive_key_from_pin(pin, &salt)?;
        let cipher = Aes256Gcm::new(&key);

        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);

        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), content)
            .map_err(NoteManagerError::EncryptionFailed)?;

        Ok([salt.as_slice(), nonce.as_slice(), &ciphertext].concat())
    }

    pub fn decrypt_note_content(
        encrypted_data: &[u8],
        pin: &str,
    ) -> Result<Vec<u8>, NoteManagerError> {
        let (salt, remainder) = encrypted_data.split_at(16);
        let (nonce, ciphertext) = remainder.split_at(12);

        let key = PinManager::derive_key_from_pin(pin, salt)?;
        let cipher = Aes256Gcm::new(&key);

        let decrypted = cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(NoteManagerError::DecryptionFailed)?;

        Ok(decrypted)
    }
}
