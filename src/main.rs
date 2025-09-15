#![forbid(unsafe_code)]
#![warn(clippy::unwrap_used)]

mod args;
mod config;
mod file;
mod note;
mod note_database;
mod pin;

use crate::{args::Args, config::Config, note_database::NoteDatabase};
use anyhow::Result;
use clap::Parser;
mod tui;

use log::LevelFilter;
use log::info;
use std::{fs, path::Path};

fn main() -> Result<()> {
    let args = Args::parse();

    let mut config = Config::new(&args.config_file);

    let filter_level = match args.verbose_level {
        0 => LevelFilter::Off,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    env_logger::builder().filter_level(filter_level).init();

    let stored_pin_hash = pin::load_pin_hash(&config);
    let pin = if stored_pin_hash.is_some() && !stored_pin_hash.unwrap().is_empty() {
        loop {
            let entered_pin = pin::ask_for_pin().unwrap();
            if pin::verify_pin(&config, &entered_pin) {
                break entered_pin;
            }
            eprintln!("Incorrect PIN. Please try again.");
        }
    } else {
        eprintln!("No PIN found. Please set a new 6-digit PIN.");
        let new_pin = pin::ask_for_pin().unwrap();
        pin::store_pin(&mut config, &new_pin);
        new_pin
    };

    match args.command {
        Some(args::Subcommands::EncryptUnencrypted) => {
            encrypt_unencrypted_files(&config.notes_dir, &pin)?;
            return Ok(());
        }
        None => {}
    }

    let note_database = NoteDatabase::from_config(args.config_file.clone())
        .expect("FIXME: handle this in some way");

    let mut context = tui::RyokanContext::new(config, &pin, note_database, args);
    context.run_tui()?;

    Ok(())
}

fn encrypt_unencrypted_files(notes_dir: impl AsRef<Path>, pin: &str) -> Result<()> {
    info!(
        "Scanning for unencrypted files in {}...",
        notes_dir.as_ref().display()
    );

    let mut unencrypted_files = Vec::new();

    for entry in fs::read_dir(&notes_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && !path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .ends_with(".enc.txt")
        {
            // try to decrypt to check if it's an encrypted file that just doesn't have the .enc.txt extension
            match file::load_and_decrypt_note_content(path.to_string_lossy().as_ref(), pin) {
                Ok(_) => {
                    // rename it: it's an encrypted file, but without the correct extension
                    let new_path = path.with_extension("enc.txt");
                    info!(
                        "Renaming encrypted file: {} -> {}",
                        path.display(),
                        new_path.display()
                    );
                    fs::rename(&path, &new_path)?;
                }
                Err(_) => {
                    // it's truly unencrypted or corrupted, so add to list
                    unencrypted_files.push(path);
                }
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
            let new_filename = file::generate_uuid_filename();
            let new_path = notes_dir.as_ref().join(new_filename);
            file::save_note_to_file(&encrypted_content, &new_path.to_string_lossy())?;
            fs::remove_file(&file_path)?;
            info!(
                "Encrypted {} to {}",
                file_path.display(),
                new_path.display()
            );
        }
        info!("Encryption complete.");
        return Ok(());
    }

    info!("No unencrypted files found.");
    Ok(())
}
