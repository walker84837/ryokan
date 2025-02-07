use crate::{args::Args, note_manager::NoteManager};
use anyhow::{Context, Result};
use log::info;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

/// Struct for managing file operations, like saving and loading notes
pub struct FileManager;

impl FileManager {
    /// Saves a note to a file in encrypted format with the given content
    pub fn save_note_to_file(content: &[u8], filename: &str) -> Result<()> {
        let mut file = File::create(filename)?;
        file.write_all(content)?;
        info!("Note saved to {}", filename);
        Ok(())
    }

    /// Loads and decrypts the content of a note
    pub fn load_and_decrypt_note_content(filename: &str, pin: &str) -> Result<Vec<u8>> {
        let mut encrypted_data = Vec::new();
        let mut file =
            File::open(filename).with_context(|| format!("Unable to open file: {}", filename))?; // Include filename in error
        file.read_to_end(&mut encrypted_data)
            .context("Unable to read file")?;

        NoteManager::decrypt_note_content(&encrypted_data, pin)
            .context("Failed to decrypt note content")
    }

    /// Generates a UUID filename for a new note in the format of "UUID.enc.txt"
    pub fn generate_uuid_filename() -> String {
        let id = Uuid::new_v4().to_string();
        format!("{}.enc.txt", id)
    }

    /// Opens the file in the default text editor
    pub fn open_in_editor(args: &Args, filename: PathBuf) -> Result<()> {
        let env_editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        let editor = args.editor.as_ref().unwrap_or(&env_editor);
        Command::new(editor).arg(filename).spawn()?.wait()?;
        Ok(())
    }
}
