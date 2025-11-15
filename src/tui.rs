use crate::error::AppError;
use crate::metadata::NoteMetadata;
use crate::{args::Args, config::Config, file, note};
use chrono::Utc;
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

pub struct Note {
    pub uuid: String,
    pub encrypted_file_path: PathBuf,
    pub metadata: NoteMetadata,
}

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
    EditSelectedNote,
    ScrollUp,
    ScrollDown,
}

pub struct App {
    config: Config,
    pin: String,
    args: Args,
    notes: Vec<Note>,
    list_state: ListState,
    selected_note_index: usize,
    note_preview_content: String,
    running_state: RunningState,
}

impl App {
    pub fn new(config: Config, pin: String, args: Args) -> Result<Self, AppError> {
        let mut app = Self {
            config,
            pin,
            args,
            notes: Vec::new(),
            list_state: ListState::default(),
            selected_note_index: 0,
            note_preview_content: String::new(),
            running_state: RunningState::Running,
        };
        app.reload_notes()?;

        if !app.notes.is_empty() {
            app.list_state.select(Some(app.selected_note_index));
        }

        app.note_preview_content =
            Self::load_preview_content(&app.notes, app.selected_note_index, &app.pin)?;

        Ok(app)
    }

    fn load_preview_content(notes: &[Note], index: usize, pin: &str) -> Result<String, AppError> {
        if let Some(note) = notes.get(index) {
            match file::load_and_decrypt_note_content(
                note.encrypted_file_path.to_string_lossy().as_ref(),
                pin,
            ) {
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
        Ok(())
    }

    fn handle_event(&self) -> Result<Message, AppError> {
        event::poll(Duration::from_millis(250))
            .map_err(AppError::Io)?
            .then(|| event::read().map_err(AppError::Io))
            .transpose() // turns Option<Result<_,_>> into Result<Option<_>, _>
            .map(|opt_event| match opt_event {
                Some(Event::Key(key)) => match key.code {
                    KeyCode::Char('q') => Message::Quit,
                    KeyCode::Char('n') => Message::NewNote,
                    KeyCode::Down => Message::ScrollDown,
                    KeyCode::Up => Message::ScrollUp,
                    KeyCode::Enter => Message::EditSelectedNote,
                    _ => Message::Tick,
                },
                _ => Message::Tick,
            })
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
            Message::Tick => { /* No action on tick for now */ }
        }
        Ok(())
    }

    fn handle_new_note(&mut self) -> Result<(), AppError> {
        let notes_dir_path = PathBuf::from(self.config.notes_dir.clone());
        let uuid = file::generate_uuid();
        let encrypted_note_path = notes_dir_path.join(format!("{}.enc.txt", uuid));
        let metadata_path = notes_dir_path.join(format!("{}.meta.toml", uuid));

        let encrypted_content = note::encrypt_note_content(&Vec::new(), &self.pin)?;
        file::save_note_to_file(&encrypted_content, &encrypted_note_path.to_string_lossy())?;

        let metadata = NoteMetadata::new("New Note".to_string());
        metadata.save(&metadata_path)?;

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
            let notes_dir_path = PathBuf::from(self.config.notes_dir.clone());
            let temp_file = format!("temp_{}.txt", Uuid::new_v4());
            let note_path = note.encrypted_file_path.to_string_lossy().into_owned();
            let decrypted_content = file::load_and_decrypt_note_content(&note_path, &self.pin)?;
            file::save_note_to_file(&decrypted_content, &temp_file)?;

            terminal_mode_guard!(terminal, {
                file::open_in_editor(&self.args, PathBuf::from(&temp_file))?;
            });

            let updated_content = fs::read(&temp_file).map_err(AppError::Io)?;
            let encrypted_content = note::encrypt_note_content(&updated_content, &self.pin)?;

            file::save_note_to_file(&encrypted_content, &note_path)?;
            fs::remove_file(temp_file).map_err(AppError::Io)?;

            // Update metadata
            let mut metadata = note.metadata.clone(); // Clone to modify
            metadata.updated_at = Utc::now();
            let metadata_path = notes_dir_path.join(format!("{}.meta.toml", note.uuid));
            metadata.save(&metadata_path)?;

            self.note_preview_content =
                Self::load_preview_content(&self.notes, self.selected_note_index, &self.pin)?;
        }
        Ok(())
    }

    fn reload_notes(&mut self) -> Result<(), AppError> {
        use std::collections::HashMap;
        let notes_dir = PathBuf::from(self.config.notes_dir.clone());
        let mut files_by_uuid: HashMap<String, (Option<PathBuf>, Option<PathBuf>)> = HashMap::new();

        for entry in fs::read_dir(&notes_dir).map_err(AppError::Io)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    let parts: Vec<&str> = file_name.split('.').collect();
                    if parts.len() == 3 {
                        let uuid = parts[0].to_string();
                        let extension = parts[1];
                        match extension {
                            "enc" => {
                                files_by_uuid.entry(uuid).or_default().0 = Some(path);
                            }
                            "meta" => {
                                files_by_uuid.entry(uuid).or_default().1 = Some(path);
                            }
                            _ => { /* ignore other files */ }
                        }
                    }
                }
            }
        }

        let mut loaded_notes = Vec::new();
        for (uuid, (enc_path_opt, meta_path_opt)) in files_by_uuid {
            if let (Some(encrypted_file_path), Some(metadata_path)) = (enc_path_opt, meta_path_opt)
            {
                match NoteMetadata::load(&metadata_path) {
                    Ok(metadata) => {
                        loaded_notes.push(Note {
                            uuid,
                            encrypted_file_path,
                            metadata,
                        });
                    }
                    Err(e) => {
                        // Log error or handle corrupted metadata file
                        eprintln!("Error loading metadata for {}: {}", uuid, e);
                    }
                }
            }
        }

        // Sort notes by updated_at, newest first
        loaded_notes.sort_by(|a, b| b.metadata.updated_at.cmp(&a.metadata.updated_at));

        self.notes = loaded_notes;
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
            .map(|note| ListItem::new(note.metadata.original_filename.clone()))
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
