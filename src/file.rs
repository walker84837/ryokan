use crate::error::AppError;
use crate::metadata::NoteMetadata;
use crate::{args::Args, note};
use log::info;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

pub const MAGIC_BYTES: &[u8] = b"RYOKAN_ENCRYPTED";

pub fn is_encrypted_file(data: &[u8]) -> bool {
    data.starts_with(MAGIC_BYTES)
}

pub fn create_new_note(
    notes_dir: &Path,
    pin: &str,
    original_filename: &str,
    content: &[u8],
) -> Result<(), AppError> {
    let encrypted_content = note::encrypt_note_content(content, pin)?;
    let metadata = NoteMetadata::new(original_filename);

    let uuid = generate_uuid();
    let (encrypted_note_path, metadata_path) = note_paths(notes_dir, &uuid);

    // Save metadata first, then encrypted content
    metadata.save(&metadata_path)?;
    save_note_to_file(&encrypted_content, &encrypted_note_path)?;

    Ok(())
}

/// Generates a UUID for a new note
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Generate note file paths from a UUID
pub fn note_paths(notes_dir: &Path, uuid: &str) -> (PathBuf, PathBuf) {
    (
        notes_dir.join(format!("{uuid}.enc.txt")),
        notes_dir.join(format!("{uuid}.meta.toml")),
    )
}

/// Saves a note to a file in encrypted format with the given content
pub fn save_note_to_file(content: &[u8], path: impl AsRef<Path>) -> Result<(), AppError> {
    let path = path.as_ref();

    // Atomic write pattern
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Config("Invalid note path".to_string()))?;
    let mut temp_file = tempfile::NamedTempFile::new_in(parent).map_err(AppError::Io)?;

    temp_file.write_all(MAGIC_BYTES).map_err(AppError::Io)?;
    temp_file.write_all(content).map_err(AppError::Io)?;

    temp_file.persist(path).map_err(|e| AppError::Io(e.error))?;

    info!("Note saved to {}", path.display());
    Ok(())
}

/// Loads and decrypts the content of a note
pub fn load_and_decrypt_note_content(
    path: impl AsRef<Path>,
    pin: &str,
) -> Result<Vec<u8>, AppError> {
    let path = path.as_ref();
    let mut encrypted_data = Vec::new();
    let mut file = File::open(path).map_err(AppError::Io)?;
    file.read_to_end(&mut encrypted_data)
        .map_err(AppError::Io)?;

    if is_encrypted_file(&encrypted_data) {
        let content_without_magic = &encrypted_data[MAGIC_BYTES.len()..];
        note::decrypt_note_content(content_without_magic, pin)
    } else {
        // If no magic bytes, assume it's an unencrypted file for now, or malformed encrypted file
        // This will be handled more robustly in encrypt_unencrypted_files
        Err(AppError::Decryption(
            "File does not contain Ryokan magic bytes.".to_string(),
        ))
    }
}

/// Deletes both the encrypted note file and its metadata
pub fn delete_note_files(notes_dir: &Path, uuid: &str) -> Result<(), AppError> {
    let (enc_path, meta_path) = note_paths(notes_dir, uuid);

    if enc_path.exists() {
        fs::remove_file(&enc_path).map_err(AppError::Io)?;
    }
    if meta_path.exists() {
        fs::remove_file(&meta_path).map_err(AppError::Io)?;
    }
    info!("Deleted note {uuid}");
    Ok(())
}

/// Opens the file in the default text editor
pub fn open_in_editor(args: &Args, path: &Path) -> Result<(), AppError> {
    let env_editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let editor = args.editor.as_ref().unwrap_or(&env_editor);
    Command::new(editor)
        .arg(path)
        .spawn()
        .map_err(AppError::Io)?
        .wait()
        .map_err(AppError::Io)?;
    Ok(())
}
