// SPDX-FileCopyrightText: 2026 Nikita Goncharov
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod editor;
pub mod plugins;
pub mod ui;

use crate::tui::editor::FileTab;
use crate::tui::plugins::PluginManager;
use aam_rs::aam::AAM;
use aam_rs::pipeline::FormattingOptions;
use anyhow::{Context, Result};
use ratatui::crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Instant;
use tui_textarea::Input;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const KNOWN_COMMANDS: &[&str] = &[
    "open ", "save", "check", "format", "quit", "help", "get ", "close", "mode",
];

// Path autocompletion for the open command
pub fn get_path_completions(partial_path: &str) -> Vec<String> {
    let mut completions = Vec::new();

    // If path is empty, show files in the current directory
    let (dir_path, name_prefix) = if partial_path.is_empty() {
        (".".to_string(), "".to_string())
    } else if partial_path.ends_with('/') {
        (partial_path.to_string(), "".to_string())
    } else {
        let path = PathBuf::from(partial_path);
        match (path.parent(), path.file_name()) {
            (Some(parent), Some(name)) => {
                let parent_str = if parent.as_os_str().is_empty() {
                    ".".to_string()
                } else {
                    parent.display().to_string()
                };
                let name_str = name.to_string_lossy().to_string();
                (parent_str, name_str)
            }
            _ => (".".to_string(), partial_path.to_string()),
        }
    };

    if let Ok(entries) = fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy().to_string();

                // Filter by prefix
                if file_name_str.starts_with(&name_prefix) {
                    let full_path = entry.path();
                    let display_path = if metadata.is_dir() {
                        format!("{}/", full_path.display())
                    } else {
                        full_path.display().to_string()
                    };
                    completions.push(display_path);
                }
            }
        }
    }

    completions.sort();
    completions
}

#[derive(Clone, Copy, PartialEq)]
pub enum FocusArea {
    Editor,
    Input,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    Tabbed,
    Split,
}

#[allow(clippy::struct_excessive_bools)]
pub struct App<'a> {
    pub files: Vec<FileTab<'a>>,
    pub active_file_index: Option<usize>,
    pub focus: FocusArea,
    pub input_line: String,
    pub status_message: String,
    pub error_message: Option<String>,
    pub view_mode: ViewMode,
    pub show_diagnostics: bool,
    pub show_animations: bool,

    // Animation physics
    pub last_tick: Instant,
    pub scanner_pos: f64,

    pub running: bool,
    pub show_help: bool,

    pub plugin_manager: PluginManager,
}

impl<'a> App<'a> {
    fn load_initial_files(file_paths: Option<&Vec<PathBuf>>) -> Result<Vec<FileTab<'a>>> {
        let mut files = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        if let Some(paths) = file_paths {
            for path in paths {
                if !seen_paths.insert(path.canonicalize().unwrap_or_else(|_| path.clone())) {
                    return Err(anyhow::anyhow!("File specified twice: {}", path.display()));
                }

                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {}", path.display()))?;
                files.push(FileTab::new(path.clone(), content));
            }
        }

        Ok(files)
    }

    fn initial_status_message(files_count: usize) -> String {
        if files_count == 0 {
            "Ready | Ctrl+H Help".to_string()
        } else {
            format!("Loaded {files_count} file(s) | Ctrl+H Help")
        }
    }

    fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Tabbed => ViewMode::Split,
            ViewMode::Split => ViewMode::Tabbed,
        };
    }

    fn handle_undo_redo(&mut self, is_undo: bool) {
        if self.focus != FocusArea::Editor {
            return;
        }

        if let Some(file) = self.get_active_file_mut() {
            if is_undo {
                file.textarea.undo();
            } else {
                file.textarea.redo();
            }
            file.check_validity();
        }
    }

    fn execute_input_command(&mut self, command: &str, args: &[&str]) {
        let handled = self
            .plugin_manager
            .handle_command(command, args)
            .unwrap_or(false);
        if handled {
            return;
        }

        match command {
            "open" | "o" => {
                if let Some(path) = args.first() {
                    self.open_file(path);
                } else {
                    self.status_message = "Usage: open <file>".to_string();
                }
            }
            "check" | "c" => self.check_active_file(),
            "format" | "f" => self.format_active_file(),
            "save" | "w" => self.save_active_file(),
            "help" | "h" => self.show_help = true,
            "quit" | "q" => self.running = false,
            "close" => self.close_active_file(),
            "mode" => {
                self.toggle_view_mode();
                self.status_message = "✓ Mode changed".to_string();
            }
            _ => self.status_message = "Unknown command".to_string(),
        }
    }

    fn handle_input_submit(&mut self) {
        match self.focus {
            FocusArea::Input => {
                if self.input_line.is_empty() {
                    return;
                }

                let cmd = self.input_line.trim().to_string();
                let mut parts = cmd.split_whitespace();
                let command = parts.next().unwrap_or("");
                let args: Vec<&str> = parts.collect();
                self.execute_input_command(command, &args);
                self.input_line.clear();
            }
            FocusArea::Editor => {
                if let Some(file) = self.get_active_file_mut() {
                    file.textarea.insert_newline();
                    file.check_validity();
                }
            }
        }
    }

    fn handle_regular_input(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        match self.focus {
            FocusArea::Editor => {
                if let Some(file) = self.get_active_file_mut() {
                    file.textarea.input(Input::from(key));
                    file.check_validity();
                }
            }
            FocusArea::Input => match key.code {
                KeyCode::Char(c) => self.input_line.push(c),
                KeyCode::Backspace => {
                    self.input_line.pop();
                }
                KeyCode::Delete => self.input_line.clear(),
                _ => {}
            },
        }
    }

    fn handle_input_autocomplete(&mut self) {
        let parts: Vec<&str> = self.input_line.split_whitespace().collect();

        // Autocompletion for the "open" command
        if parts.len() == 1 && parts[0] == "open" {
            if let Some(first_completion) = get_path_completions("").first() {
                self.input_line = format!("open {}", first_completion);
            }
        } else if parts.len() >= 2 && (parts[0] == "open" || parts[0] == "o") {
            // Get the already typed path
            let input_after_open = self
                .input_line
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ");
            if let Some(first_completion) = get_path_completions(&input_after_open).first() {
                self.input_line = format!("open {}", first_completion);
            }
        }
    }

    fn handle_modal_keys(&mut self, key: ratatui::crossterm::event::KeyEvent) -> bool {
        if !(self.show_help || self.error_message.is_some()) {
            return false;
        }

        if key.code == KeyCode::Esc || key.code == KeyCode::Enter || key.code == KeyCode::Char('q')
        {
            self.show_help = false;
            self.error_message = None;
        }
        true
    }

    fn handle_key_press(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        if self.handle_modal_keys(key) {
            return;
        }

        match key.code {
            KeyCode::F(2) => self.toggle_view_mode(),
            KeyCode::F(3) => self.show_animations = !self.show_animations,
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_active_file();
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.check_active_file();
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.format_active_file();
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.close_active_file();
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.show_diagnostics = !self.show_diagnostics;
            }
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_view_mode();
            }
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_undo_redo(true);
            }
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_undo_redo(false);
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => self.next_tab(),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => self.prev_tab(),
            KeyCode::PageDown => self.next_tab(),
            KeyCode::PageUp => self.prev_tab(),
            KeyCode::Tab => {
                // If in input area, try autocompletion
                if self.focus == FocusArea::Input && self.input_line.starts_with("open") {
                    self.handle_input_autocomplete();
                } else {
                    self.focus = match self.focus {
                        FocusArea::Editor => FocusArea::Input,
                        FocusArea::Input => FocusArea::Editor,
                    };
                }
            }
            KeyCode::Esc => {
                self.show_diagnostics = false;
            }
            KeyCode::Enter => self.handle_input_submit(),
            _ => self.handle_regular_input(key),
        }
    }

    /// Creates a new App instance.
    ///
    /// # Errors
    /// Returns an error if file reading fails.
    pub fn new(file_paths: Option<&Vec<PathBuf>>) -> Result<Self> {
        let files = Self::load_initial_files(file_paths)?;

        let active_file_index = if files.is_empty() { None } else { Some(0) };
        let files_count = files.len();

        Ok(Self {
            files,
            active_file_index,
            focus: FocusArea::Editor,
            input_line: String::new(),
            status_message: Self::initial_status_message(files_count),
            error_message: None,
            view_mode: ViewMode::Split,
            show_diagnostics: false,
            show_animations: true,
            last_tick: Instant::now(),
            scanner_pos: 0.0,
            running: true,
            show_help: false,
            plugin_manager: PluginManager::new(),
        })
    }

    pub fn update_physics(&mut self, width: usize, height: usize) {
        if !self.show_animations {
            return;
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        if width < 10 || height < 10 {
            return;
        }

        #[allow(clippy::cast_precision_loss)]
        let total_perimeter = (width * 2 + height * 2).saturating_sub(4) as f64;
        let base_speed = 250.0;

        #[allow(clippy::cast_precision_loss)]
        let a1_pos = (width / 4) as f64;
        #[allow(clippy::cast_precision_loss)]
        let a2_pos = (width / 2) as f64;
        #[allow(clippy::cast_precision_loss)]
        let m_pos = ((width * 3) / 4) as f64;

        let slow_radius = 30.0;
        let slow_factor = 0.01;

        let dist_to_a1 = (self.scanner_pos - a1_pos).abs();
        let dist_to_a2 = (self.scanner_pos - a2_pos).abs();
        let dist_to_m = (self.scanner_pos - m_pos).abs();

        let dist_to_a1_cyclic = dist_to_a1.min(total_perimeter - dist_to_a1);
        let dist_to_a2_cyclic = dist_to_a2.min(total_perimeter - dist_to_a2);
        let dist_to_m_cyclic = dist_to_m.min(total_perimeter - dist_to_m);

        let mut speed_mult = 1.0;

        if dist_to_a1_cyclic < slow_radius {
            let t = 1.0 - dist_to_a1_cyclic / slow_radius;
            speed_mult *= 1.0 - (1.0 - slow_factor) * t;
        }
        if dist_to_a2_cyclic < slow_radius {
            let t = 1.0 - dist_to_a2_cyclic / slow_radius;
            speed_mult *= 1.0 - (1.0 - slow_factor) * t;
        }
        if dist_to_m_cyclic < slow_radius {
            let t = 1.0 - dist_to_m_cyclic / slow_radius;
            speed_mult *= 1.0 - (1.0 - slow_factor) * t;
        }

        self.scanner_pos = (self.scanner_pos + base_speed * speed_mult * dt) % total_perimeter;
    }

    #[must_use]
    pub fn is_file_opened(&self, path: &PathBuf) -> bool {
        self.files.iter().any(|file| file.path == *path)
    }

    pub fn open_file(&mut self, path_str: &str) {
        let path = PathBuf::from(path_str);
        if self.is_file_opened(&path) {
            self.status_message = format!("✗ File already opened: {}", path.display());
            return;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.files.push(FileTab::new(path.clone(), content));
                self.active_file_index = Some(self.files.len() - 1);
                self.status_message = format!("✓ Opened {}", path.display());
            }
            Err(e) => {
                self.status_message = "✗ Failed to open file".to_string();
                self.error_message = Some(e.to_string());
            }
        }
    }

    #[must_use]
    pub fn get_active_file(&self) -> Option<&FileTab<'a>> {
        self.active_file_index.and_then(|i| self.files.get(i))
    }

    pub fn get_active_file_mut(&mut self) -> Option<&mut FileTab<'a>> {
        let index = self.active_file_index?;
        self.files.get_mut(index)
    }

    pub fn save_active_file(&mut self) {
        let Some(index) = self.active_file_index else {
            return;
        };

        let content = self.files[index].textarea.lines().join("\n");
        let path = self.files[index].path.clone();
        if let Err(e) = std::fs::write(&path, &content) {
            self.status_message = "✗ Save error".to_string();
            self.error_message = Some(e.to_string());
            return;
        }
        self.files[index].check_validity();

        let is_valid = self.files[index].valid;
        let error_count = self.files[index].error_count;
        let path_display = path.display().to_string();

        if is_valid {
            self.show_diagnostics = false;
            self.status_message = format!("✓ Saved: {path_display}");
        } else {
            self.show_diagnostics = true;
            self.status_message = format!("✓ Saved, but has {error_count} errors");
            if let Some(line) = self.files[index].file_errors.first().map(|e| e.line) {
                #[allow(clippy::cast_possible_truncation)]
                self.files[index]
                    .textarea
                    .move_cursor(tui_textarea::CursorMove::Jump(
                        (line.saturating_sub(1)) as u16,
                        0,
                    ));
            }
        }
    }

    pub fn check_active_file(&mut self) {
        let Some(index) = self.active_file_index else {
            return;
        };

        self.files[index].check_validity();
        let is_valid = self.files[index].valid;
        let error_count = self.files[index].error_count;

        if is_valid {
            self.status_message = "✓ Valid".to_string();
            self.error_message = None;
            self.show_diagnostics = false;
        } else {
            self.status_message = format!("✗ Errors: {error_count}");
            self.show_diagnostics = true;
            if let Some(line) = self.files[index].file_errors.first().map(|e| e.line) {
                #[allow(clippy::cast_possible_truncation)]
                self.files[index]
                    .textarea
                    .move_cursor(tui_textarea::CursorMove::Jump(
                        (line.saturating_sub(1)) as u16,
                        0,
                    ));
            }
        }
    }

    pub fn format_active_file(&mut self) {
        if let Some(file) = self.get_active_file_mut() {
            let content = file.textarea.lines().join("\n");
            match AAM::parse(&content) {
                Ok(aam) => {
                    if let Ok(formatted) = aam.format(&content, &FormattingOptions::default()) {
                        file.textarea = FileTab::new(file.path.clone(), formatted).textarea;
                        file.check_validity();
                        self.status_message = "✓ Formatted".to_string();
                    }
                }
                Err(_errors) => {
                    self.status_message = "✗ Cannot format: has errors".to_string();
                    self.show_diagnostics = true;
                }
            }
        }
    }

    pub fn next_tab(&mut self) {
        if !self.files.is_empty() {
            self.active_file_index =
                Some((self.active_file_index.unwrap_or(0) + 1) % self.files.len());
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.files.is_empty() {
            let current = self.active_file_index.unwrap_or(0);
            self.active_file_index = Some(if current == 0 {
                self.files.len() - 1
            } else {
                current - 1
            });
        }
    }

    pub fn close_active_file(&mut self) {
        if let Some(index) = self.active_file_index {
            self.files.remove(index);
            if self.files.is_empty() {
                self.active_file_index = None;
                self.status_message = "Ready | Ctrl+H for help".to_string();
            } else {
                self.active_file_index = Some(index.min(self.files.len() - 1));
                self.status_message = format!("✓ Closed file. {} remaining", self.files.len());
            }
        }
    }
}

/// Runs the TUI application.
///
/// # Errors
/// Returns an error if the TUI fails to start or encounters an unrecoverable error.
pub fn run_tui(file_paths: Option<&Vec<PathBuf>>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(file_paths)?;

    terminal.clear()?;

    while app.running {
        let size = terminal.size()?;
        app.update_physics(size.width as usize, size.height as usize);

        terminal.draw(|f| ui::ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(16))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            app.handle_key_press(key);
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
