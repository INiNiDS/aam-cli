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
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::io;
use std::path::PathBuf;
use std::time::Instant;
use tui_textarea::{Input, TextArea};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const KNOWN_COMMANDS: &[&str] = &[
    "open ", "save", "check", "format", "quit", "help", "get ", "close",
];

#[derive(Clone, Copy, PartialEq)]
enum FocusArea {
    Editor,
    Input,
}

struct FileTab<'a> {
    path: PathBuf,
    content: String,
    textarea: TextArea<'a>,
    valid: bool,
    error_count: usize,
}

impl<'a> FileTab<'a> {
    fn new(path: PathBuf, content: String) -> Self {
        let mut textarea = TextArea::default();
        for line in content.lines() {
            textarea.insert_str(line);
            textarea.insert_newline();
        }

        // Basic AAM syntax highlighting hack (keys before = sign)
        let _ = textarea.set_search_pattern("^[\\w\\.\\-]+\\s*(?==)");
        textarea.set_search_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        let valid = AAM::parse(&content).is_ok();
        let error_count = if valid {
            0
        } else {
            match AAM::parse(&content) {
                Err(errors) => errors.len(),
                _ => 0,
            }
        };

        Self {
            path,
            content,
            textarea,
            valid,
            error_count,
        }
    }
}

struct App<'a> {
    files: Vec<FileTab<'a>>,
    active_file_index: Option<usize>,
    focus: FocusArea,
    input_line: String,
    status_message: String,
    error_message: Option<String>,

    // Animation physics
    last_tick: Instant,
    scanner_pos: f64,

    running: bool,
    show_help: bool,
}

impl<'a> App<'a> {
    fn new(file_paths: Option<&Vec<PathBuf>>) -> Result<Self> {
        let mut files = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        if let Some(paths) = file_paths {
            for path in paths {
                // Check for duplicates
                if !seen_paths.insert(path.canonicalize().unwrap_or_else(|_| path.clone())) {
                    return Err(anyhow::anyhow!("File specified twice: {}", path.display()));
                }

                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {}", path.display()))?;
                files.push(FileTab::new(path.clone(), content));
            }
        }

        let active_file_index = if files.is_empty() { None } else { Some(0) };
        let files_count = files.len();

        Ok(Self {
            files,
            active_file_index,
            focus: FocusArea::Editor,
            input_line: String::new(),
            status_message: if files_count == 0 {
                "Ready | Ctrl+H for help".to_string()
            } else {
                format!("Loaded {} file(s) | Ctrl+H for help", files_count)
            },
            error_message: None,
            last_tick: Instant::now(),
            scanner_pos: 0.0,
            running: true,
            show_help: false,
        })
    }

    fn update_physics(&mut self, width: usize, height: usize) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        if width < 10 || height < 10 {
            return;
        }

        let total_perimeter = (width * 2 + height * 2).saturating_sub(4) as f64;

        let a1 = (width / 4) as f64;
        let a2 = (width / 2) as f64;
        let m = ((width * 3) / 4) as f64;

        let d1 = (self.scanner_pos - a1).abs();
        let d2 = (self.scanner_pos - a2).abs();
        let d3 = (self.scanner_pos - m).abs();
        let min_d = d1.min(d2).min(d3);

        let speed_mult = (min_d / 10.0).clamp(0.05, 3.0);
        let base_speed = 150.0;

        self.scanner_pos = (self.scanner_pos + base_speed * speed_mult * dt) % total_perimeter;
    }

    fn is_file_opened(&self, path: &PathBuf) -> bool {
        self.files.iter().any(|file| file.path == *path)
    }

    fn open_file(&mut self, path_str: &str) {
        let path = PathBuf::from(path_str);

        // Check if file is already opened
        if self.is_file_opened(&path) {
            self.status_message = format!("✗ File already opened: {}", path.display());
            self.error_message = Some("This file is already open in another tab.".to_string());
            return;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.files.push(FileTab::new(path.clone(), content));
                self.active_file_index = Some(self.files.len() - 1);
                self.status_message = format!("✓ Opened {}", path.display());
                self.error_message = None;
            }
            Err(e) => {
                self.status_message = "✗ Failed to open file".to_string();
                self.error_message = Some(e.to_string());
            }
        }
    }

    fn get_active_file(&self) -> Option<&FileTab<'_>> {
        self.active_file_index.and_then(|i| self.files.get(i))
    }

    fn get_active_file_mut(&mut self) -> Option<&mut FileTab<'a>> {
        let index = self.active_file_index?;
        self.files.get_mut(index)
    }

    fn save_active_file(&mut self) {
        if let Some(file) = self.get_active_file() {
            let content = file.textarea.lines().join("\n");
            if let Err(e) = std::fs::write(&file.path, &content) {
                self.status_message = "✗ Save error".to_string();
                self.error_message = Some(e.to_string());
                return;
            }
            self.status_message = format!("✓ Saved: {}", file.path.display());
            self.error_message = None;
        }
    }

    fn check_active_file(&mut self) {
        if let Some(file) = self.get_active_file() {
            let content = file.textarea.lines().join("\n");
            match AAM::parse(&content) {
                Ok(aam) => {
                    self.status_message = format!("✓ Valid ({} key(s))", aam.keys().len());
                    self.error_message = None;
                    if let Some(active_file) = self.get_active_file_mut() {
                        active_file.valid = true;
                        active_file.error_count = 0;
                    }
                }
                Err(errors) => {
                    let count = errors.len();
                    self.status_message = format!("✗ Errors: {}", count);
                    self.error_message = Some(
                        errors
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                    if let Some(active_file) = self.get_active_file_mut() {
                        active_file.valid = false;
                        active_file.error_count = count;
                    }
                }
            }
        }
    }

    fn format_active_file(&mut self) {
        if let Some(file) = self.get_active_file() {
            let content = file.textarea.lines().join("\n");
            match AAM::parse(&content) {
                Ok(aam) => {
                    if let Ok(formatted) = aam.format(&content, &FormattingOptions::default()) {
                        if let Some(active_file) = self.get_active_file_mut() {
                            active_file.content = formatted.clone();
                            active_file.textarea =
                                FileTab::new(active_file.path.clone(), formatted).textarea;
                        }
                        self.status_message = "✓ Formatted".to_string();
                    }
                }
                Err(errors) => {
                    self.status_message = "✗ Cannot format: has errors".to_string();
                    self.error_message = Some(
                        errors
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                }
            }
        }
    }

    fn next_tab(&mut self) {
        if !self.files.is_empty() {
            self.active_file_index =
                Some((self.active_file_index.unwrap_or(0) + 1) % self.files.len());
        }
    }

    fn prev_tab(&mut self) {
        if !self.files.is_empty() {
            let current = self.active_file_index.unwrap_or(0);
            self.active_file_index = Some(if current == 0 {
                self.files.len() - 1
            } else {
                current - 1
            });
        }
    }

    fn close_active_file(&mut self) {
        if let Some(index) = self.active_file_index {
            self.files.remove(index);
            if self.files.is_empty() {
                self.active_file_index = None;
                self.status_message = "Ready | Ctrl+H for help".to_string();
            } else {
                self.active_file_index = Some(if index >= self.files.len() {
                    self.files.len() - 1
                } else {
                    index
                });
                self.status_message = format!("✓ Closed file. {} remaining", self.files.len());
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(ratatui::widgets::Clear, area); // Clear artifacts

    // Screen layout:
    // [0] Main area (Editor + Animated border)
    // [1] Status bar
    // [2] Input line
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let editor_area = main_chunks[0];

    // Draw border ONLY around editor
    render_perimeter_animated_bar(f, editor_area, app);

    // Inner area for editor text (minus 1 char on each side for border)
    let inner_editor_area = Rect {
        x: editor_area.x + 1,
        y: editor_area.y + 1,
        width: editor_area.width.saturating_sub(2),
        height: editor_area.height.saturating_sub(2),
    };

    let files_count = app.files.len();
    if files_count > 0 {
        let file_constraints: Vec<Constraint> = (0..files_count)
            .map(|_| Constraint::Ratio(1, files_count as u32))
            .collect();

        let file_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(file_constraints)
            .split(inner_editor_area);

        for (i, file) in app.files.iter().enumerate() {
            let is_active = Some(i) == app.active_file_index;
            let file_valid_status = if file.valid { "✓" } else { "✗" };
            let filename = file
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("new file");

            let title_style = if file.valid {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };

            // Inner border removed, only title at top (as tab header)
            let editor_block = Block::default().title(Span::styled(
                format!(" {} ({}) ", filename, file_valid_status),
                title_style,
            ));

            let mut ta_clone = file.textarea.clone();
            ta_clone.set_block(editor_block);

            // Dim inactive files
            if !is_active {
                ta_clone.set_style(Style::default().fg(Color::DarkGray));
            } else if app.focus == FocusArea::Editor {
                // Highlight active editor
                ta_clone.set_cursor_line_style(Style::default().bg(Color::Rgb(30, 30, 30)));
            }

            f.render_widget(&ta_clone, file_chunks[i]);
        }
    } else {
        let welcome = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Welcome to AAM CLI TUI",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Use input below: open <file.aam>",
                Style::default().fg(Color::Gray),
            )),
        ])
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().title(" AAM Editor "));
        f.render_widget(welcome, inner_editor_area);
    }

    // --- STATUS BAR ---
    let status_style = if app.status_message.starts_with("✓") {
        Style::default().fg(Color::Green)
    } else if app.status_message.starts_with("✗") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Gray)
    };

    let status_paragraph = Paragraph::new(app.status_message.as_str())
        .style(status_style)
        .alignment(ratatui::layout::Alignment::Right);
    f.render_widget(status_paragraph, main_chunks[1]);

    // --- COMMAND INPUT ---
    let is_focused = app.focus == FocusArea::Input;
    let mut hint = "";
    if !app.input_line.is_empty() {
        for cmd in KNOWN_COMMANDS {
            if cmd.starts_with(&app.input_line) {
                hint = &cmd[app.input_line.len()..];
                break;
            }
        }
    }

    let text_color = if is_focused {
        Color::White
    } else {
        Color::DarkGray
    };
    let mut input_spans = Vec::new();
    input_spans.push(Span::styled(
        if is_focused { "> " } else { "  " },
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));

    if app.input_line.is_empty() && !is_focused {
        input_spans.push(Span::styled(
            "Press Tab to enter command...",
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        input_spans.push(Span::styled(
            &app.input_line,
            Style::default().fg(text_color),
        ));
        if !hint.is_empty() {
            input_spans.push(Span::styled(
                hint,
                Style::default().fg(Color::Rgb(80, 80, 80)),
            ));
        }
    }
    f.render_widget(Paragraph::new(Line::from(input_spans)), main_chunks[2]);

    // --- ERROR POPUP ---
    if let Some(ref error) = app.error_message {
        let error_popup = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Errors (Esc to close) ")
                    .border_style(Style::default().fg(Color::Red)),
            )
            .wrap(Wrap { trim: true });

        // Centered popup window on top of everything
        let popup_area = Rect {
            x: area.width.saturating_sub(area.width / 2) / 2,
            y: area.height.saturating_sub(area.height / 2) / 2,
            width: area.width / 2,
            height: area.height / 2,
        };
        // Clear background under popup
        f.render_widget(ratatui::widgets::Clear, popup_area);
        f.render_widget(error_popup, popup_area);
    }

    if app.show_help {
        render_help_popup(f);
    }
}

// 💥 PERIMETER ANIMATION MAGIC
fn render_perimeter_animated_bar(f: &mut Frame, area: Rect, app: &App) {
    let width = area.width as usize;
    let height = area.height as usize;
    if width < 10 || height < 10 {
        return;
    }

    let total_perimeter = width * 2 + height * 2 - 4;
    let head = app.scanner_pos as usize % total_perimeter.max(1);
    let tail_len = 25;

    let get_snake_color = |pos: usize| -> Option<Color> {
        let dist = if head >= pos {
            head - pos
        } else {
            total_perimeter - pos + head
        };

        if dist < tail_len {
            if dist <= 2 {
                Some(Color::Cyan)
            } else if dist <= 6 {
                Some(Color::LightCyan)
            } else if dist <= 15 {
                Some(Color::Blue)
            } else {
                Some(Color::DarkGray)
            }
        } else {
            None
        }
    };

    let a1_pos = width / 4;
    let a2_pos = width / 2;
    let m_pos = (width * 3) / 4;

    let bg_color = Color::Rgb(15, 15, 15);

    let mut top_spans = Vec::new();
    for i in 0..width {
        let pos = i;
        let mut char_str = if i == 0 {
            "╭"
        } else if i == width - 1 {
            "╮"
        } else {
            "─"
        };
        let mut modifier = Modifier::empty();

        if pos == a1_pos || pos == a2_pos {
            char_str = "A";
            modifier = Modifier::BOLD;
        } else if pos == m_pos {
            char_str = "M";
            modifier = Modifier::BOLD;
        }

        let color = get_snake_color(pos).unwrap_or(bg_color);
        if get_snake_color(pos).is_some() && char_str == "─" {
            char_str = "━";
        }
        top_spans.push(Span::styled(
            char_str,
            Style::default().fg(color).add_modifier(modifier),
        ));
    }
    f.render_widget(
        Paragraph::new(Line::from(top_spans)),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );

    for i in 0..(height.saturating_sub(2)) {
        let pos = width + i;
        let mut char_str = "│";
        let modifier = Modifier::empty();

        let color = get_snake_color(pos).unwrap_or(bg_color);
        if get_snake_color(pos).is_some() && char_str == "│" {
            char_str = "┃";
        }
        f.render_widget(
            Paragraph::new(Span::styled(
                char_str,
                Style::default().fg(color).add_modifier(modifier),
            )),
            Rect {
                x: area.x + area.width - 1,
                y: area.y + 1 + i as u16,
                width: 1,
                height: 1,
            },
        );
    }

    let mut bottom_spans = Vec::new();
    let brand_text = format!(" INiNiDS v{} ", VERSION);
    let brand_len = brand_text.chars().count();
    let brand_start = (width.saturating_sub(brand_len)) / 2;

    for i in 0..width {
        let pos = width + height.saturating_sub(2) + (width - 1 - i);
        let mut char_str = if i == 0 {
            "╯"
        } else if i == width - 1 {
            "╰"
        } else {
            "─"
        };
        let modifier = Modifier::empty();

        let color = get_snake_color(pos).unwrap_or(bg_color);
        if get_snake_color(pos).is_some() && char_str == "─" {
            char_str = "━";
        }

        if i >= brand_start && i < brand_start + brand_len {
            if i == brand_start {
                bottom_spans.push(Span::styled(
                    &brand_text,
                    Style::default()
                        .fg(Color::Rgb(100, 100, 120))
                        .add_modifier(Modifier::BOLD),
                ));
            }
        } else {
            bottom_spans.push(Span::styled(
                char_str,
                Style::default().fg(color).add_modifier(modifier),
            ));
        }
    }
    f.render_widget(
        Paragraph::new(Line::from(bottom_spans)),
        Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: area.width,
            height: 1,
        },
    );

    for i in 0..(height.saturating_sub(2)) {
        let pos = total_perimeter - 1 - i;
        let mut char_str = "│";
        let modifier = Modifier::empty();

        let color = get_snake_color(pos).unwrap_or(bg_color);
        if get_snake_color(pos).is_some() && char_str == "│" {
            char_str = "┃";
        }
        f.render_widget(
            Paragraph::new(Span::styled(
                char_str,
                Style::default().fg(color).add_modifier(modifier),
            )),
            Rect {
                x: area.x,
                y: area.y + 1 + (height - 3 - i) as u16,
                width: 1,
                height: 1,
            },
        );
    }
}

fn render_help_popup(f: &mut Frame) {
    let help_text = vec![
        Line::from(Span::styled(
            "Hotkeys & Commands:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Commands (in input bar):"),
        Line::from("  open <file> - Open file side-by-side"),
        Line::from("  save / check / format / quit"),
        Line::from(""),
        Line::from("Hotkeys:"),
        Line::from("Ctrl+T - Check   | Ctrl+S - Save"),
        Line::from("Ctrl+F - Format  | Ctrl+Q - Quit"),
        Line::from("Ctrl+W - Close   | Tab - Switch focus"),
        Line::from("Ctrl+Right/Left (or PgUp/PgDn) - Switch tabs"),
    ];

    let help_popup = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    let area = f.area();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 4,
        width: area.width / 2,
        height: 13,
    };
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(help_popup, popup_area);
}

pub fn run_tui(file_paths: Option<&Vec<PathBuf>>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(file_paths)?;

    terminal.clear()?;

    // 60 FPS Game Loop
    while app.running {
        let size = terminal.size()?;
        // Update physics (pass width for perimeter)
        app.update_physics(size.width as usize, size.height as usize);

        terminal.draw(|f| ui(f, &app))?;

        // Listen for events 16ms (60 fps)
        if event::poll(std::time::Duration::from_millis(16))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Intercept input when popup is open
            if app.show_help || app.error_message.is_some() {
                if key.code == KeyCode::Esc
                    || key.code == KeyCode::Enter
                    || key.code == KeyCode::Char('q')
                {
                    app.show_help = false;
                    app.error_message = None;
                }
                continue;
            }

            match key.code {
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.save_active_file();
                }
                KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.check_active_file();
                }
                KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.format_active_file();
                }
                KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.running = false;
                }
                KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.show_help = !app.show_help;
                }
                KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.close_active_file();
                }
                KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.next_tab();
                }
                KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.prev_tab();
                }
                KeyCode::PageDown => {
                    app.next_tab();
                }
                KeyCode::PageUp => {
                    app.prev_tab();
                }
                KeyCode::Tab => {
                    app.focus = match app.focus {
                        FocusArea::Editor => FocusArea::Input,
                        FocusArea::Input => FocusArea::Editor,
                    };
                }
                KeyCode::Esc => {
                    app.show_help = false;
                }

                // Separate behavior for Enter based on focus
                KeyCode::Enter => {
                    match app.focus {
                        FocusArea::Input => {
                            if !app.input_line.is_empty() {
                                let cmd = app.input_line.trim().to_string();
                                let mut parts = cmd.split_whitespace();

                                match parts.next() {
                                    Some("open") | Some("o") => {
                                        if let Some(path) = parts.next() {
                                            app.open_file(path);
                                        } else {
                                            app.status_message = "Usage: open <file>".to_string();
                                        }
                                    }
                                    Some("check") | Some("c") => {
                                        app.check_active_file();
                                    }
                                    Some("format") | Some("f") => {
                                        app.format_active_file();
                                    }
                                    Some("save") | Some("w") => {
                                        app.save_active_file();
                                    }
                                    Some("help") | Some("h") => {
                                        app.show_help = true;
                                    }
                                    Some("quit") | Some("q") => {
                                        app.running = false;
                                    }
                                    Some("close") => {
                                        app.close_active_file();
                                    }
                                    Some("get") | Some("g") => {
                                        if let Some(key) = parts.next() {
                                            if let Some(file) = app.get_active_file() {
                                                let content = file.textarea.lines().join("\n");
                                                match AAM::parse(&content) {
                                                    Ok(aam) => {
                                                        if let Some(val) = aam.get(key) {
                                                            app.status_message =
                                                                format!("{} = {}", key, val);
                                                        } else {
                                                            app.status_message = format!(
                                                                "✗ Key '{}' not found",
                                                                key
                                                            );
                                                        }
                                                    }
                                                    Err(_) => {
                                                        app.status_message =
                                                            "✗ Cannot get: file has errors"
                                                                .to_string();
                                                    }
                                                }
                                            }
                                        } else {
                                            app.status_message = "Usage: get <key>".to_string();
                                        }
                                    }
                                    _ => {
                                        app.status_message = "Unknown command".to_string();
                                    }
                                }
                                app.input_line.clear();
                            }
                        }
                        FocusArea::Editor => {
                            // If in editor, just insert a newline
                            if let Some(file) = app.get_active_file_mut() {
                                file.textarea.insert_newline();
                            }
                        }
                    }
                }
                _ => match app.focus {
                    FocusArea::Editor => {
                        if let Some(file) = app.get_active_file_mut() {
                            file.textarea.input(Input::from(key));
                        }
                    }
                    FocusArea::Input => match key.code {
                        KeyCode::Char(c) => {
                            app.input_line.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input_line.pop();
                        }
                        KeyCode::Delete => {
                            app.input_line.clear();
                        }
                        _ => {}
                    },
                },
            }
        }
    } // One less closing brace due to collapsible_if

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
