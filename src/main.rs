#![forbid(unsafe_code)]
#![warn(clippy::unwrap_used)]

mod args;
mod config;
mod file;
mod metadata;
mod note;
mod pin;

use crate::{args::Args, config::Config};
use clap::Parser;
mod error;
mod tui;

use crate::error::AppError;
use crate::file::MAGIC_BYTES;
use crate::metadata::NoteMetadata;
use log::LevelFilter;
use log::info;
use std::{fs, path::Path};

fn main() -> Result<(), AppError> {
    let args = Args::parse();

    let mut config = Config::new(&args.config_file)?;
    fs::create_dir_all(&config.notes_dir).map_err(AppError::Io)?;

    let filter_level = match args.verbose_level {
        0 => LevelFilter::Off,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    env_logger::builder().filter_level(filter_level).init();

    let pin = pin::handle_pin_setup_and_verification(&mut config)?;

    match args.command {
        Some(args::Subcommands::EncryptUnencrypted) => {
            encrypt_unencrypted_files(&config.notes_dir, &pin)?;
            return Ok(());
        }
        None => {}
    }

    let mut app = tui::App::new(config, pin, args)?;
    app.run()?;

    Ok(())
}

fn encrypt_unencrypted_files(notes_dir: impl AsRef<Path>, pin: &str) -> Result<(), AppError> {
    info!(
        "Scanning for unencrypted files in {}...",
        notes_dir.as_ref().display()
    );

    let mut unencrypted_files = Vec::new();

    for entry in fs::read_dir(&notes_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let file_content = fs::read(&path)?;
            if file_content.starts_with(MAGIC_BYTES) {
                // It's an encrypted file
                if !path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .ends_with(".enc.txt")
                {
                    // Rename it: it's an encrypted file, but without the correct extension
                    let new_path = path.with_extension("enc.txt");
                    info!(
                        "Renaming encrypted file: {} -> {}",
                        path.display(),
                        new_path.display()
                    );
                    fs::rename(&path, &new_path)?;
                }
            } else {
                // It's truly unencrypted, so add to list
                unencrypted_files.push(path);
            }
        }
    }

    if !unencrypted_files.is_empty() {
        info!(
            "Found {} unencrypted files. Encrypting...",
            unencrypted_files.len()
        );
        for file_path in unencrypted_files {
            info!("Encrypting {}...", file_path.display());
            let content = fs::read(&file_path)?;
            let encrypted_content = note::encrypt_note_content(&content, pin)?;

            let original_filename = file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let metadata = NoteMetadata::new(original_filename);

            let uuid = file::generate_uuid();
            let encrypted_note_path = notes_dir.as_ref().join(format!("{}.enc.txt", uuid));
            let metadata_path = notes_dir.as_ref().join(format!("{}.meta.toml", uuid));

            file::save_note_to_file(&encrypted_content, &encrypted_note_path.to_string_lossy())?;
            metadata.save(&metadata_path)?;

            fs::remove_file(&file_path)?;
            info!(
                "Encrypted {} to {}",
                file_path.display(),
                encrypted_note_path.display()
            );
        }
        info!("Encryption complete.");
        return Ok(());
    }

    info!("No unencrypted files found.");
    Ok(())
}
