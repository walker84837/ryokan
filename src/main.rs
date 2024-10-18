use aes_gcm::{
    aead::{Aead, KeyInit, Nonce, OsRng},
    Aes256Gcm, Key,
};
use anyhow::Result;
use argon2::{self, Argon2};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rand::RngCore;
use serde_derive::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::stdout,
    io::{self, Read, Write},
    path::PathBuf,
    process::{self, Command},
};
use tempfile::NamedTempFile;
use tui::backend::CrosstermBackend;
use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use uuid::Uuid;

// Configuration file and pin storage setup
fn config_dir() -> PathBuf {
    let mut config_path = dirs::config_dir().unwrap();
    config_path.push("note_encryption");
    if !config_path.exists() {
        fs::create_dir_all(&config_path).expect("Failed to create config directory");
    }
    config_path
}

fn pin_file_path() -> PathBuf {
    let mut path = config_dir();
    path.push("pin.txt");
    path
}

fn store_pin(pin: &str) {
    let mut file = File::create(pin_file_path()).expect("Failed to store PIN");
    file.write_all(pin.as_bytes()).expect("Failed to write PIN");
}

fn load_pin() -> Option<String> {
    let mut pin = String::new();
    if let Ok(mut file) = File::open(pin_file_path()) {
        file.read_to_string(&mut pin).ok()?;
        Some(pin)
    } else {
        None
    }
}

// Derive encryption key from 6-digit PIN using Argon2
fn derive_key_from_pin(pin: &str, salt: &[u8]) -> Result<Key<Aes256Gcm>> {
    let argon2 = Argon2::default();
    let mut key = [0u8; 32]; // Aes256 needs 32 bytes
    argon2.hash_password_into(pin.as_bytes(), salt, &mut key)?;
    Ok(*Key::<Aes256Gcm>::from_slice(&key))
}

fn encrypt_note_content(content: &[u8], pin: &str) -> Result<Vec<u8>> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt); // Generate a unique salt for each encryption

    let key = derive_key_from_pin(pin, &salt)?;
    let cipher = Aes256Gcm::new(&key);

    let mut nonce = [0u8; 12]; // Nonce must be 12 bytes for AES-GCM
    OsRng.fill_bytes(&mut nonce);

    let ciphertext = cipher
        .encrypt(Nonce::<Aes256Gcm>::from_slice(&nonce), content)
        .expect("Encryption failed!");

    Ok([salt.to_vec(), nonce.to_vec(), ciphertext].concat()) // Prepend salt and nonce
}

fn decrypt_note_content(encrypted_data: &[u8], pin: &str) -> Result<String> {
    let (salt, remainder) = encrypted_data.split_at(16);
    let (nonce, ciphertext) = remainder.split_at(12);

    let key = derive_key_from_pin(pin, salt)?;
    let cipher = Aes256Gcm::new(&key);

    let decrypted = cipher
        .decrypt(Nonce::<Aes256Gcm>::from_slice(nonce), ciphertext)
        .expect("Decryption failed!");

    Ok(String::from_utf8(decrypted).unwrap())
}

// Save note to file
fn save_note_to_file(content: &[u8], filename: &str) -> Result<()> {
    let mut file = File::create(filename)?;
    file.write_all(content)?;
    Ok(())
}

// Load encrypted file and decrypt
fn handle_decryption(filename: &str, pin: &str) {
    let mut encrypted_data = Vec::new();
    let mut file = File::open(filename).expect("Unable to open file");
    file.read_to_end(&mut encrypted_data)
        .expect("Unable to read file");

    let decrypted_content = decrypt_note_content(&encrypted_data, pin);
    println!("Decrypted content: {}", decrypted_content);
}

// Initial setup to check PIN in ~/.config/note_encryption
fn initial_setup() -> String {
    if let Some(pin) = load_pin() {
        pin
    } else {
        println!("No PIN found. Please set a 6-digit PIN:");
        let mut pin = String::new();
        io::stdin().read_line(&mut pin).expect("Failed to read PIN");
        let pin = pin.trim();
        if pin.len() != 6 {
            println!("PIN must be 6 digits.");
            process::exit(1);
        }
        store_pin(pin);
        pin.to_string()
    }
}

// Generate a UUID and assign as filename
fn generate_uuid_filename() -> String {
    let id = Uuid::new_v4().to_string();
    format!("{}.enc.txt", id)
}

// Your configuration and encryption-related functions here ...

// Function to extract the preview (first sentence of first non-blank line)
fn get_note_preview(content: &str) -> (String, String) {
    let mut lines = content.lines();
    let title = lines.next().unwrap_or_default().to_string();
    let preview = lines
        .find(|&line| !line.trim().is_empty())
        .unwrap_or("")
        .to_string();
    (title, preview)
}

// Display and handle TUI
fn run_tui(notes: Vec<(String, String)>, pin: &str) -> Result<()> {
    // Setup Crossterm terminal backend
    enable_raw_mode().unwrap();
    let mut stdout = stdout();
    execute!(stdout, crossterm::terminal::EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut selected_note = 0;

    loop {
        terminal.draw(|f| {
            let size = f.size();

            // Layout: two areas (left = note previews, right = selected note content)
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(size);

            // Left side: Titles and Previews
            let previews: Vec<Spans> = notes
                .iter()
                .enumerate()
                .map(|(i, (title, preview))| {
                    let style = if i == selected_note {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    Spans::from(vec![Span::styled(
                        format!("{}\n\t{}", title, preview),
                        style,
                    )])
                })
                .collect();

            let preview_block = Paragraph::new(previews)
                .block(Block::default().title("Notes").borders(Borders::ALL));

            f.render_widget(preview_block, chunks[0]);

            // Right side: Selected note contents
            let (title, preview) = &notes[selected_note];
            let selected_content = format!("Title: {}\n\nPreview:\n{}", title, preview);

            let content_block = Paragraph::new(selected_content)
                .block(Block::default().title("Note Content").borders(Borders::ALL));

            f.render_widget(content_block, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break, // Quit on 'q'
                KeyCode::Down => {
                    if selected_note < notes.len() - 1 {
                        selected_note += 1;
                    }
                }
                KeyCode::Up => {
                    selected_note = selected_note.saturating_sub(1);
                }
                KeyCode::Enter => {
                    // Open the selected note for editing
                    let filename = &notes[selected_note].0;
                    let temp_file = edit_and_encrypt(filename, pin)?;
                    // Replace original with new encrypted content
                    fs::rename(temp_file, filename)?;
                }
                _ => {}
            }
        }
    }

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )
    .unwrap();
    terminal.show_cursor().unwrap();

    Ok(())
}

// Open the note in the default editor and save the edited content
fn edit_and_encrypt(filename: &str, pin: &str) -> Result<PathBuf> {
    let mut encrypted_data = Vec::new();
    let mut file = File::open(filename)?;
    file.read_to_end(&mut encrypted_data)?;

    let decrypted_content = decrypt_note_content(&encrypted_data, pin);

    // Create a temp file for editing
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", decrypted_content)?;

    // Open the temp file in the default text editor
    let editor = std::env::var("EDITOR").unwrap_or("nano".to_string());
    Command::new(editor)
        .arg(temp_file.path())
        .status()
        .expect("Failed to open editor");

    // Read edited content
    let mut edited_content = String::new();
    temp_file.reopen()?.read_to_string(&mut edited_content)?;

    // Re-encrypt and save the edited content
    let encrypted_content = encrypt_note_content(edited_content.as_bytes(), pin);

    // Create a UUID filename for the new encrypted note
    let new_filename = generate_uuid_filename();
    let mut new_file = File::create(&new_filename)?;
    new_file.write_all(&encrypted_content)?;

    // Move the re-encrypted file to the notes directory
    let notes_dir = config_dir(); // Assuming this is your notes folder
    let final_path = notes_dir.join(new_filename.clone());
    fs::rename(new_filename, &final_path)?;

    // Handle decryption to display the updated note in the TUI
    handle_decryption(&final_path.to_string_lossy(), pin);

    Ok(final_path)
}

fn store_notes_dir(notes_dir: &str) {
    let mut config = load_config();
    config.notes_dir = Some(notes_dir.to_string());
    save_config(&config);
}

fn load_notes_dir() -> Option<String> {
    let config = load_config();
    config.notes_dir.clone()
}

// Initial setup for notes folder location
fn initial_notes_dir_setup() -> String {
    if let Some(notes_dir) = load_notes_dir() {
        notes_dir
    } else {
        println!("No notes folder found. Please set a folder for your notes:");
        let mut notes_dir = String::new();
        io::stdin()
            .read_line(&mut notes_dir)
            .expect("Failed to read notes folder");
        let notes_dir = notes_dir.trim();
        store_notes_dir(notes_dir);
        notes_dir.to_string()
    }
}

// Configuration structure
#[derive(Default, Serialize, Deserialize)]
struct Config {
    pin: Option<String>,
    notes_dir: Option<String>,
}

// Configuration file and pin storage setup
fn config_file_path() -> PathBuf {
    let mut config_path = dirs::config_dir().unwrap();
    config_path.push("notes-renamer.toml");
    config_path
}

fn load_config() -> Config {
    let config_path = config_file_path();
    if config_path.exists() {
        let config_str = std::fs::read_to_string(config_path).expect("Unable to read config file");
        toml::de::from_str(&config_str).unwrap_or_default()
    } else {
        Config {
            pin: None,
            notes_dir: None,
        }
    }
}

fn save_config(config: &Config) {
    let config_str = toml::to_string(config).expect("Failed to serialize config");
    std::fs::write(config_file_path(), config_str).expect("Unable to write config file");
}

fn main() -> Result<()> {
    let pin = initial_setup();
    let notes_dir = initial_notes_dir_setup();

    // Process and encrypt all new non-encrypted notes
    let mut notes = vec![];
    for entry in fs::read_dir(notes_dir.clone())? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().unwrap_or_default() == "txt" {
            let mut file = File::open(&path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;

            let (title, _preview) = get_note_preview(&content);
            notes.push((path.to_string_lossy().into_owned(), title));

            // Encrypt and save note with UUID filename
            let encrypted_content = encrypt_note_content(content.as_bytes(), &pin);
            let new_filename = generate_uuid_filename();
            save_note_to_file(&encrypted_content, &new_filename);
        }
    }

    run_tui(notes, &pin)?;

    Ok(())
}
