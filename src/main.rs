#![forbid(unsafe_code)]
#![warn(clippy::unwrap_used)]

mod args;
mod config;
mod error;
mod file;
mod metadata;
mod note;
mod pin;
mod tui;

use crate::{args::Args, config::Config, error::AppError};
use clap::Parser;
use log::LevelFilter;
use log::info;
use std::{fs, path::Path};

fn main() -> Result<(), AppError> {
    let args = Args::parse();

    let mut config = Config::new(args.config_file.as_ref())?;

    let filter_level = match args.verbose_level {
        0 => LevelFilter::Off,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    env_logger::builder().filter_level(filter_level).init();

    let pin = pin::handle_pin_setup_and_verification(&mut config)?;

    if let Some(args::Subcommands::EncryptUnencrypted) = args.command {
        encrypt_unencrypted_files(config.notes_dir_path(), &pin)?;
        return Ok(());
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
            if file::is_encrypted_file(&file_content) {
                // It's an encrypted file
                let is_encrypted = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.ends_with(".enc.txt"));
                if !is_encrypted {
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

            let original_filename = file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            file::create_new_note(notes_dir.as_ref(), pin, &original_filename, &content)?;

            fs::remove_file(&file_path)?;
            info!("Encrypted {}", file_path.display());
        }
        info!("Encryption complete.");
        return Ok(());
    }

    info!("No unencrypted files found.");
    Ok(())
}
