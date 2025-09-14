use crate::{args::Args, config::Config, file, note, note_database::NoteDatabase};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{fs, io, path::PathBuf};
use uuid::Uuid;

/// Temporarily exits the alternate screen mode, executes a block of code
/// and then re-enters the alternate screen mode.
macro_rules! terminal_mode_guard {
    ($terminal:expr, $action:block) => {
        execute!($terminal.backend_mut(), LeaveAlternateScreen)?;
        $terminal.hide_cursor()?;

        // run the action to run in this guard
        $action

        // re-enable cursor for TUI control
        execute!($terminal.backend_mut(), EnterAlternateScreen)?;
        $terminal.show_cursor()?;
    };
}

/// Runs the terminal UI
pub fn run_tui(
    config: &mut Config,
    pin: &str,
    note_database: &mut NoteDatabase,
    args: &Args,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let notes_dir = &config.notes_dir;
    let mut notes = fs::read_dir(notes_dir)?
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".enc.txt"))
        .collect::<Vec<_>>();

    let mut list_state = ListState::default();
    let mut selected = 0;
    list_state.select(Some(selected));
    loop {
        draw_terminal(&mut terminal, &mut notes, &mut list_state, pin)?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('n') => {
                    let notes_dir_path = PathBuf::from(notes_dir.clone());
                    let filename = file::generate_uuid_filename();
                    let path = notes_dir_path.join(&filename);
                    let encrypted_content = note::encrypt_note_content(&Vec::new(), pin)?;
                    file::save_note_to_file(&encrypted_content, &path.to_string_lossy())?;

                    // Extract UUID from filename and insert into note_database
                    let uuid = filename.split('.').next().unwrap_or("").to_string();
                    note_database.insert(uuid, filename.clone());

                    notes = fs::read_dir(notes_dir_path)?
                        .filter_map(std::result::Result::ok)
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
                    if let Some(selected) = list_state.selected()
                        && let Some(note) = notes.get(selected)
                    {
                        let temp_file = format!("temp_{}.txt", Uuid::new_v4());
                        let note_path = note.path().to_string_lossy().into_owned();
                        let decrypted_content =
                            file::load_and_decrypt_note_content(&note_path, pin)?;
                        file::save_note_to_file(&decrypted_content, &temp_file)
                            .context("Error saving note to temporary file")?;

                        terminal_mode_guard!(&mut terminal, {
                            file::open_in_editor(args, PathBuf::from(&temp_file))?;
                        });

                        draw_terminal(&mut terminal, &mut notes, &mut list_state, pin)?;

                        let updated_content = fs::read(&temp_file)?;
                        let encrypted_content = note::encrypt_note_content(&updated_content, pin)?;

                        file::save_note_to_file(&encrypted_content, &note_path)?;
                        fs::remove_file(temp_file)?;
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    note_database.save()?;
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

        let preview = if let Some(selected) = list_state.selected()
            && let Some(note) = notes.get(selected)
        {
            let filename = note.path();
            match file::load_and_decrypt_note_content(filename.to_string_lossy().as_ref(), pin) {
                Ok(content) => String::from_utf8_lossy(&content).to_string(),
                Err(_) => "Error reading note.".to_string(),
            }
        } else {
            "No note selected.".to_string()
        };

        let preview_paragraph =
            Paragraph::new(preview).block(Block::default().borders(Borders::ALL).title("Preview"));
        f.render_widget(preview_paragraph, chunks[1]);

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
