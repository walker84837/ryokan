use crate::pin_manager::PinManagerError;
use crate::PinManager;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm,
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

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, content)
            .map_err(NoteManagerError::EncryptionFailed)?;

        Ok([salt.as_slice(), nonce.as_slice(), &ciphertext].concat())
    }

    pub fn decrypt_note_content(
        encrypted_data: &[u8],
        pin: &str,
    ) -> Result<Vec<u8>, NoteManagerError> {
        let (salt, remainder) = encrypted_data.split_at(16);
        let (nonce_slice, ciphertext) = remainder.split_at(12);

        let key = PinManager::derive_key_from_pin(pin, salt)?;
        let cipher = Aes256Gcm::new(&key);

        let nonce = aes_gcm::Nonce::from_slice(nonce_slice);

        let decrypted = cipher
            .decrypt(nonce, ciphertext)
            .map_err(NoteManagerError::DecryptionFailed)?;

        Ok(decrypted)
    }
}
