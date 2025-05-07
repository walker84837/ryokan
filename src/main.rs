mod args;
mod config;
mod file_manager;
mod note_database;
mod note_manager;
mod pin_manager;

use crate::{
    args::Args, config::Config, file_manager::FileManager, note_manager::NoteManager,
    pin_manager::PinManager,
};
use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::LevelFilter;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{fs, io, path::PathBuf};
use uuid::Uuid;

fn main() -> Result<()> {
    let args = Args::parse();

    let mut config = Config::new(&args.config_file);

    let filter_level = match args.verbose_level {
        0 => LevelFilter::Off,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Warn,
    };

    initialize_logger(filter_level);

    let stored_pin_hash = PinManager::load_pin_hash(&config);
    let pin = if stored_pin_hash.is_some() && !stored_pin_hash.unwrap().is_empty() {
        loop {
            let entered_pin = PinManager::ask_for_pin();
            if PinManager::verify_pin(&config, &entered_pin) {
                break entered_pin;
            } else {
                println!("Incorrect PIN. Please try again.");
            }
        }
    } else {
        println!("No PIN found. Please set a new 6-digit PIN.");
        let new_pin = PinManager::ask_for_pin();
        PinManager::store_pin(&mut config, &new_pin);
        new_pin
    };
    let mut note_database = note_database::NoteDatabase::new(&args.config_file);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let notes_dir = &config.notes_dir;
    let mut notes = fs::read_dir(&notes_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".enc.txt"))
        .collect::<Vec<_>>();

    let mut list_state = ListState::default();
    let mut selected = 0;
    list_state.select(Some(selected));
    loop {
        draw_terminal(&mut terminal, &mut notes, &mut list_state, &pin)?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('n') => {
                    let notes_dir_path = PathBuf::from(notes_dir.clone());
                    let filename = FileManager::generate_uuid_filename();
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
                            FileManager::save_note_to_file(&decrypted_content, &temp_file)
                                .context("Error saving note to temporary file")?;

                            let note_path = note.path().to_string_lossy().into_owned();

                            // Restore terminal to normal mode and main screen before launching editor
                            control_tui(&mut terminal, false)?;

                            FileManager::open_in_editor(&args, temp_file.clone().into())
                                .context("Error opening note in editor")?;

                            // Re-enable raw mode and alternate screen after editor exits
                            control_tui(&mut terminal, true)?;
                            draw_terminal(&mut terminal, &mut notes, &mut list_state, &pin)?;

                            let encrypted_content =
                                NoteManager::encrypt_note_content(&decrypted_content, &pin)?;

                            FileManager::save_note_to_file(&encrypted_content, &note_path)?;
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

fn control_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    show_cursor: bool,
) -> Result<()> {
    if show_cursor {
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        terminal.show_cursor()?;
    } else {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.hide_cursor()?;
    }
    Ok(())
}

/// Draws the terminal UI
fn draw_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    notes: &mut [fs::DirEntry],
    list_state: &mut ListState,
    pin: &str,
) -> Result<()> {
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
        f.render_stateful_widget(notes_list, chunks[0], list_state);

        // TODO: this should be simplified as soon as Rust Edition 2024 is stabilized
        let preview = if let Some(selected) = list_state.selected() {
            if let Some(note) = notes.get(selected) {
                let filename = note.path();
                match FileManager::load_and_decrypt_note_content(
                    filename.to_string_lossy().as_ref(),
                    pin,
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
        let preview_paragraph =
            Paragraph::new(preview).block(Block::default().borders(Borders::ALL).title("Preview"));
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

    Ok(())
}

fn initialize_logger(filter_level: LevelFilter) {
    env_logger::builder().filter_level(filter_level).init();
}
