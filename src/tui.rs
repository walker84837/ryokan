use crate::error::AppError;
use crate::{args::Args, config::Config, file, note, note_database::NoteDatabase};
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
use std::{fs, io, path::PathBuf, time::Duration};
use uuid::Uuid;

/// Temporarily exits the alternate screen mode, executes a block of code
/// and then re-enters the alternate screen mode.
macro_rules! terminal_mode_guard {
    ($terminal:expr, $action:block) => {
        execute!($terminal.backend_mut(), LeaveAlternateScreen).map_err(AppError::Io)?;
        $terminal.hide_cursor().map_err(|e| AppError::Tui(e.to_string()))?;

        // run the action to run in this guard
        $action

        // re-enable cursor for TUI control
        execute!($terminal.backend_mut(), EnterAlternateScreen).map_err(AppError::Io)?;
        $terminal.show_cursor().map_err(|e| AppError::Tui(e.to_string()))?;
    };
}

#[derive(Debug, PartialEq, Eq)]
enum RunningState {
    Running,
    Quit,
}

#[derive(Debug)]
enum Message {
    Tick,
    Quit,
    NewNote,
    SelectNote(usize),
    EditSelectedNote,
    ScrollUp,
    ScrollDown,
    Error(AppError),
}

pub struct App {
    config: Config,
    pin: String,
    database: NoteDatabase,
    args: Args,
    notes: Vec<fs::DirEntry>,
    list_state: ListState,
    selected_note_index: usize,
    note_preview_content: String,
    running_state: RunningState,
}

impl App {
    pub fn new(
        config: Config,
        pin: String,
        note_database: NoteDatabase,
        args: Args,
    ) -> Result<Self, AppError> {
        let notes_dir = &config.notes_dir;
        let notes = fs::read_dir(notes_dir)
            .map_err(AppError::Io)?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".enc.txt"))
            .collect::<Vec<_>>();

        let mut list_state = ListState::default();
        let selected_note_index = 0;
        if !notes.is_empty() {
            list_state.select(Some(selected_note_index));
        }

        let note_preview_content = Self::load_preview_content(&notes, selected_note_index, &pin)?;

        Ok(Self {
            config,
            pin,
            database: note_database,
            args,
            notes,
            list_state,
            selected_note_index,
            note_preview_content,
            running_state: RunningState::Running,
        })
    }

    fn load_preview_content(
        notes: &[fs::DirEntry],
        index: usize,
        pin: &str,
    ) -> Result<String, AppError> {
        if let Some(note) = notes.get(index) {
            let filename = note.path();
            match file::load_and_decrypt_note_content(filename.to_string_lossy().as_ref(), pin) {
                Ok(content) => Ok(String::from_utf8_lossy(&content).to_string()),
                Err(e) => Ok(format!("Error reading note: {}", e)),
            }
        } else {
            Ok("No note selected.".to_string())
        }
    }

    pub fn run(&mut self) -> Result<(), AppError> {
        enable_raw_mode().map_err(AppError::Io)?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(AppError::Io)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|e| AppError::Tui(e.to_string()))?;

        while self.running_state == RunningState::Running {
            terminal
                .draw(|f| self.view(f))
                .map_err(|e| AppError::Tui(e.to_string()))?;

            let message = self.handle_event()?;
            self.update(message, &mut terminal)?;
        }

        disable_raw_mode().map_err(AppError::Io)?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(AppError::Io)?;
        self.database.save()?;
        Ok(())
    }

    fn handle_event(&self) -> Result<Message, AppError> {
        if event::poll(Duration::from_millis(250)).map_err(AppError::Io)? {
            if let Event::Key(key) = event::read().map_err(AppError::Io)? {
                match key.code {
                    KeyCode::Char('q') => Ok(Message::Quit),
                    KeyCode::Char('n') => Ok(Message::NewNote),
                    KeyCode::Down => Ok(Message::ScrollDown),
                    KeyCode::Up => Ok(Message::ScrollUp),
                    KeyCode::Enter => Ok(Message::EditSelectedNote),
                    _ => Ok(Message::Tick),
                }
            } else {
                Ok(Message::Tick)
            }
        } else {
            Ok(Message::Tick)
        }
    }

    fn update(
        &mut self,
        message: Message,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), AppError> {
        match message {
            Message::Quit => {
                self.running_state = RunningState::Quit;
            }
            Message::NewNote => self.handle_new_note()?,
            Message::ScrollDown => self.handle_scroll_down()?,
            Message::ScrollUp => self.handle_scroll_up()?,
            Message::EditSelectedNote => self.handle_edit_selected_note(terminal)?,
            Message::Error(e) => {
                self.note_preview_content = format!("Error: {}", e);
            }
            Message::Tick => { /* No action on tick for now */ }
            _ => {}
        }
        Ok(())
    }

    fn handle_new_note(&mut self) -> Result<(), AppError> {
        let notes_dir_path = PathBuf::from(self.config.notes_dir.clone());
        let filename = file::generate_uuid_filename();
        let path = notes_dir_path.join(&filename);
        let encrypted_content = note::encrypt_note_content(&Vec::new(), &self.pin)?;
        file::save_note_to_file(&encrypted_content, &path.to_string_lossy())?;

        let uuid = filename.split('.').next().unwrap_or("").to_string();
        self.database.insert(uuid, filename.clone());

        self.reload_notes()?;
        self.note_preview_content =
            Self::load_preview_content(&self.notes, self.selected_note_index, &self.pin)?;
        Ok(())
    }

    fn handle_scroll_down(&mut self) -> Result<(), AppError> {
        if self.selected_note_index < self.notes.len().saturating_sub(1) {
            self.selected_note_index += 1;
            self.list_state.select(Some(self.selected_note_index));
            self.note_preview_content =
                Self::load_preview_content(&self.notes, self.selected_note_index, &self.pin)?;
        }
        Ok(())
    }

    fn handle_scroll_up(&mut self) -> Result<(), AppError> {
        if self.selected_note_index > 0 {
            self.selected_note_index -= 1;
            self.list_state.select(Some(self.selected_note_index));
            self.note_preview_content =
                Self::load_preview_content(&self.notes, self.selected_note_index, &self.pin)?;
        }
        Ok(())
    }

    fn handle_edit_selected_note(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), AppError> {
        if let Some(note) = self.notes.get(self.selected_note_index) {
            let temp_file = format!("temp_{}.txt", Uuid::new_v4());
            let note_path = note.path().to_string_lossy().into_owned();
            let decrypted_content = file::load_and_decrypt_note_content(&note_path, &self.pin)?;
            file::save_note_to_file(&decrypted_content, &temp_file)?;

            terminal_mode_guard!(terminal, {
                file::open_in_editor(&self.args, PathBuf::from(&temp_file))?;
            });

            let updated_content = fs::read(&temp_file).map_err(AppError::Io)?;
            let encrypted_content = note::encrypt_note_content(&updated_content, &self.pin)?;

            file::save_note_to_file(&encrypted_content, &note_path)?;
            fs::remove_file(temp_file).map_err(AppError::Io)?;

            self.note_preview_content =
                Self::load_preview_content(&self.notes, self.selected_note_index, &self.pin)?;
        }
        Ok(())
    }

    fn reload_notes(&mut self) -> Result<(), AppError> {
        let notes_dir = &self.config.notes_dir;
        self.notes = fs::read_dir(notes_dir)
            .map_err(AppError::Io)?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".enc.txt"))
            .collect::<Vec<_>>();
        Ok(())
    }

    fn view(&mut self, f: &mut ratatui::Frame) {
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

        let items: Vec<_> = self
            .notes
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
        f.render_stateful_widget(notes_list, chunks[0], &mut self.list_state);

        let preview_paragraph = Paragraph::new(self.note_preview_content.clone())
            .block(Block::default().borders(Borders::ALL).title("Preview"));
        f.render_widget(preview_paragraph, chunks[1]);

        let help_text = Line::from(vec![
            Span::raw("Up/Down: Navigate  "),
            Span::raw("Enter: Open/Edit  "),
            Span::raw("n: New Note  "),
            Span::raw("q: Quit"),
        ]);
        let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[2]);
    }
}
