use crate::pin_manager::*;
use aes_gcm::{
    aead::{Aead, KeyInit, Nonce},
    Aes256Gcm,
};
use rand::rngs::OsRng;
use rand::{self, Rng, RngCore};

// Struct for handling encryption and decryption of notes
pub(crate) struct NoteManager;

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
