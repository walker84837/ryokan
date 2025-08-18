mod args;
mod config;
mod file;
mod note;
mod note_database;
mod pin;

use crate::{args::Args, config::Config};
use anyhow::Result;
use clap::Parser;
mod tui;

use log::LevelFilter;
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

    initialize_logger(filter_level);

    let stored_pin_hash = pin::load_pin_hash(&config);
    let pin = if stored_pin_hash.is_some() && !stored_pin_hash.unwrap().is_empty() {
        loop {
            let entered_pin = pin::ask_for_pin().unwrap();
            if pin::verify_pin(&config, &entered_pin) {
                break entered_pin;
            } else {
                eprintln!("Incorrect PIN. Please try again.");
            }
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

    let mut note_database = note_database::NoteDatabase::new(&args.config_file);

    tui::run_tui(&mut config, &pin, &mut note_database, &args)?;

    Ok(())
}

fn initialize_logger(filter_level: LevelFilter) {
    env_logger::builder().filter_level(filter_level).init();
}

fn encrypt_unencrypted_files(notes_dir: impl AsRef<Path>, pin: &str) -> Result<()> {
    println!(
        "Scanning for unencrypted files in {:?}...",
        notes_dir.as_ref()
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
                    println!("Renaming encrypted file: {:?} -> {:?}", path, new_path);
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
        println!(
            "Found {} unencrypted files. Encrypting...",
            unencrypted_files.len()
        );
        for file_path in unencrypted_files {
            println!("Encrypting {:?}...", file_path);
            let content = fs::read(&file_path)?;
            let encrypted_content = note::encrypt_note_content(&content, pin)?;
            let new_filename = file::generate_uuid_filename();
            let new_path = notes_dir.as_ref().join(new_filename);
            file::save_note_to_file(&encrypted_content, &new_path.to_string_lossy())?;
            fs::remove_file(&file_path)?;
            println!("Encrypted {:?} to {:?}", file_path, new_path);
        }
        println!("Encryption complete.");
        return Ok(());
    }

    println!("No unencrypted files found.");
    Ok(())
}
