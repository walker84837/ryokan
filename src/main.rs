mod config;
mod note_manager;
mod pin_manager;

use crate::{config::Config, note_manager::NoteManager, pin_manager::PinManager};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::info;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
    process::Command,
};
use uuid::Uuid;

struct FileManager;

impl FileManager {
    pub fn save_note_to_file(content: &[u8], filename: &str) -> Result<()> {
        let mut file = File::create(filename)?;
        file.write_all(content)?;
        info!("Note saved to {}", filename);
        Ok(())
    }

    pub fn load_and_decrypt_note_content(filename: &str, pin: &str) -> Result<Vec<u8>> {
        let mut encrypted_data = Vec::new();
        let mut file = File::open(filename).context("Unable to open file")?;
        file.read_to_end(&mut encrypted_data)
            .context("Unable to read file")?;

        NoteManager::decrypt_note_content(&encrypted_data, pin)
            .context("Failed to decrypt note content")
    }
}

fn generate_uuid_filename() -> String {
    let id = Uuid::new_v4().to_string();
    format!("{}.enc.txt", id)
}

fn open_in_editor(filename: &str) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    Command::new(editor).arg(filename).spawn()?.wait()?;
    Ok(())
}

fn main() -> Result<()> {
    let stored_pin_hash = PinManager::load_pin_hash();
    let pin = if stored_pin_hash.is_some() {
        loop {
            let entered_pin = PinManager::ask_for_pin();
            if PinManager::verify_pin(&entered_pin) {
                break entered_pin;
            } else {
                println!("Incorrect PIN. Please try again.");
            }
        }
    } else {
        println!("No PIN found. Please set a new 6-digit PIN.");
        let new_pin = PinManager::ask_for_pin();
        PinManager::store_pin(&new_pin);
        new_pin
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let notes_dir = Config::load_notes_dir()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut notes = fs::read_dir(&notes_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".enc.txt"))
        .collect::<Vec<_>>();

    let mut list_state = ListState::default();
    let mut selected = 0;
    list_state.select(Some(selected));

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(70),
                        Constraint::Percentage(20),
                        Constraint::Min(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // Notes list
            let items: Vec<_> = notes
                .iter()
                .map(|note| {
                    let filename = note.file_name().to_string_lossy().to_string();
                    ListItem::new(filename)
                })
                .collect();
            let notes_list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Notes"))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            f.render_stateful_widget(notes_list, chunks[0], &mut list_state);

            // Note preview
            let preview = if let Some(selected) = list_state.selected() {
                if let Some(note) = notes.get(selected) {
                    let filename = note.path();
                    match FileManager::load_and_decrypt_note_content(
                        filename.to_string_lossy().as_ref(),
                        &pin,
                    ) {
                        Ok(content) => String::from_utf8_lossy(&content).to_string(),
                        Err(_) => "Error reading note.".to_string(),
                    }
                } else {
                    "No note selected.".to_string()
                }
            } else {
                "No note selected.".to_string()
            };
            let preview_paragraph = Paragraph::new(preview)
                .block(Block::default().borders(Borders::ALL).title("Preview"));
            f.render_widget(preview_paragraph, chunks[1]);

            // Keybinding tips
            let help_text = Line::from(vec![
                Span::raw("Up/Down: Navigate  "),
                Span::raw("Enter: Open/Edit  "),
                Span::raw("n: New Note  "),
                Span::raw("q: Quit"),
            ]);
            let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
            f.render_widget(help, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('n') => {
                    let notes_dir = Config::load_notes_dir().unwrap_or_else(|| ".".to_string());
                    let notes_dir_path = PathBuf::from(&notes_dir);
                    let filename = generate_uuid_filename();
                    let path = notes_dir_path.join(&filename);
                    let encrypted_content = NoteManager::encrypt_note_content(&Vec::new(), &pin)?;
                    FileManager::save_note_to_file(&encrypted_content, &path.to_string_lossy())?;
                    // Reload notes list
                    notes = fs::read_dir(notes_dir_path)?
                        .filter_map(|entry| entry.ok())
                        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".enc.txt"))
                        .collect();
                }
                KeyCode::Down => {
                    if selected < notes.len().saturating_sub(1) {
                        selected += 1;
                        list_state.select(Some(selected));
                    }
                }
                KeyCode::Up => {
                    if selected > 0 {
                        selected -= 1;
                        list_state.select(Some(selected));
                    }
                }
                KeyCode::Enter => {
                    if let Some(selected) = list_state.selected() {
                        if let Some(note) = notes.get(selected) {
                            let temp_file = format!("temp_{}.txt", Uuid::new_v4());
                            let decrypted_content = FileManager::load_and_decrypt_note_content(
                                note.path().to_string_lossy().as_ref(),
                                &pin,
                            )?;
                            FileManager::save_note_to_file(&decrypted_content, &temp_file)?;
                            open_in_editor(&temp_file)?;
                            let encrypted_content =
                                NoteManager::encrypt_note_content(&decrypted_content, &pin)?;
                            FileManager::save_note_to_file(
                                &encrypted_content,
                                note.path().to_string_lossy().as_ref(),
                            )?;
                            fs::remove_file(temp_file)?;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
