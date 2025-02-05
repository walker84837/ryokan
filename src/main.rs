mod config;
mod note_manager;
mod pin_manager;

use crate::{note_manager::NoteManager, pin_manager::PinManager};
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
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
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
    let pin = PinManager::ask_for_pin();
    PinManager::store_pin(&pin);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let notes = fs::read_dir(".")?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".enc.txt"))
        .collect::<Vec<_>>();

    let mut selected = 0;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(20)].as_ref())
                .split(f.size());

            // Notes list
            let items: Vec<_> = notes
                .iter()
                .map(|note| {
                    let filename = note.file_name().to_string_lossy();
                    ListItem::new(filename.to_string())
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
            f.render_stateful_widget(
                notes_list,
                chunks[0],
                &mut ratatui::widgets::ListState::default().select(Some(selected)),
            );

            // Note preview
            let preview = if let Some(note) = notes.get(selected) {
                let filename = note.path();
                if let Ok(content) = FileManager::load_and_decrypt_note_content(
                    filename.to_string_lossy().as_ref(),
                    &pin,
                ) {
                    String::from_utf8_lossy(&content).to_string()
                } else {
                    "Error reading note.".to_string()
                }
            } else {
                "No note selected.".to_string()
            };
            let preview_paragraph = Paragraph::new(preview)
                .block(Block::default().borders(Borders::ALL).title("Preview"));
            f.render_widget(preview_paragraph, chunks[1]);

            // Keybinding tips
            let help_text = Spans::from(vec![
                Span::raw("Up/Down: Navigate  "),
                Span::raw("Enter: Open in Editor  "),
                Span::raw("q: Quit"),
            ]);
            let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
            f.render_widget(help, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down => {
                    if selected < notes.len().saturating_sub(1) {
                        selected += 1;
                    }
                }
                KeyCode::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(note) = notes.get(selected) {
                        open_in_editor(note.path().to_string_lossy().as_ref())?;
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
