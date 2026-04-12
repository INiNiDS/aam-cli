// SPDX-FileCopyrightText: 2026 Nikita Goncharov
// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

use crate::tui::{App, FocusArea, KNOWN_COMMANDS, VERSION, ViewMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
};

fn build_editor_block(file: &crate::tui::editor::FileTab) -> String {
    let file_valid_status = if file.valid { "✓" } else { "✗" };
    let filename = file
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("new file");
    format!(" {filename} ({file_valid_status}) ")
}

// Функция для удаления ANSI escape кодов
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{001b}' {
            // Это начало ANSI escape кода
            chars.next(); // пропускаем '['
            // Пропускаем всё до первой буквы
            while let Some(&next_ch) = chars.peek() {
                chars.next();
                if next_ch.is_alphabetic() {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn render_split_view(f: &mut Frame, app: &mut App, area: Rect) {
    let files_count = app.files.len();
    let file_constraints: Vec<Constraint> = (0..files_count)
        .map(|_| Constraint::Ratio(1, files_count as u32))
        .collect();

    let file_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(file_constraints)
        .split(area);

    for (i, file) in app.files.iter_mut().enumerate() {
        let is_active = Some(i) == app.active_file_index;
        let title_style = if file.valid {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };
        let editor_block =
            Block::default().title(Span::styled(build_editor_block(file), title_style));

        let mut ta = file.textarea.clone();
        ta.set_block(editor_block);

        if is_active && app.focus == FocusArea::Editor {
            ta.set_cursor_line_style(Style::default().bg(Color::Rgb(30, 30, 30)));
            ta.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            ta.set_style(Style::default());
        } else {
            ta.set_cursor_line_style(Style::default());
            ta.set_cursor_style(Style::default().fg(Color::DarkGray).bg(Color::Reset));
            ta.set_style(Style::default().fg(Color::DarkGray));
        }

        // Рендеризуем с подсветкой ошибок
        render_textarea_with_errors(f, &ta, file_chunks[i], file);
    }
}

fn render_textarea_with_errors(f: &mut Frame, ta: &tui_textarea::TextArea, area: Rect, file: &crate::tui::editor::FileTab) {
    // Для теперь мы используем встроенную подсветку та, но добавим красный фон для строк с ошибками
    f.render_widget(ta, area);

    // Добавляем красный индикатор для строк с ошибками
    for line_num in &file.error_lines {
        let line_y = *line_num as u16;
        if line_y >= area.y && line_y < area.y + area.height {
            // Рисуем красный маркер слева от строки ошибки
            let marker = Paragraph::new("●")
                .style(Style::default().fg(Color::Red));
            f.render_widget(marker, Rect {
                x: area.x,
                y: area.y + (line_y - area.y),
                width: 1,
                height: 1,
            });
        }
    }
}

fn render_tabbed_view(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    let titles: Vec<Line> = app
        .files
        .iter()
        .map(|f| {
            let filename = f
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("new file");
            let color = if f.valid { Color::Green } else { Color::Red };
            Line::from(Span::styled(
                format!(" {filename} "),
                Style::default().fg(color),
            ))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(app.active_file_index.unwrap_or(0))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Rgb(40, 40, 40)),
        );
    f.render_widget(tabs, chunks[0]);

    if let Some(i) = app.active_file_index
        && let Some(file) = app.files.get_mut(i)
    {
        let mut ta = file.textarea.clone();
        ta.set_block(Block::default());
        if app.focus == FocusArea::Editor {
            ta.set_cursor_line_style(Style::default().bg(Color::Rgb(30, 30, 30)));
            ta.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            ta.set_style(Style::default());
        } else {
            ta.set_cursor_line_style(Style::default());
            ta.set_cursor_style(Style::default().fg(Color::DarkGray).bg(Color::Reset));
            ta.set_style(Style::default().fg(Color::DarkGray));
        }
        render_textarea_with_errors(f, &ta, chunks[1], file);
    }
}

fn render_editor_area(f: &mut Frame, app: &mut App, inner_editor_area: Rect) {
    let files_count = app.files.len();

    if files_count > 0 {
        match app.view_mode {
            ViewMode::Split => render_split_view(f, app, inner_editor_area),
            ViewMode::Tabbed => render_tabbed_view(f, app, inner_editor_area),
        }
    } else {
        let welcome = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Welcome to AAM CLI IDE",
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
}

fn build_diag_lines_for_error(file_err: &crate::tui::editor::FileError, file_path: &std::path::Path) -> Vec<Line<'static>> {
    let line_num_str = format!("{}:{} ", file_err.line, file_err.column);
    let error_code_str = format!("[{}] ", file_err.code);
    let path_str = file_path.display().to_string();
    // Очищаем ANSI коды из сообщений об ошибках
    let short_msg = strip_ansi_codes(&file_err.short_msg);
    let fix_hint = strip_ansi_codes(&file_err.fix_hint);
    let title = file_err.title.to_string();

    vec![
        Line::from(vec![
            Span::styled(
                " error ",
                Style::default()
                    .bg(Color::Red)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                error_code_str,
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                title,
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  --> ",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                path_str,
                Style::default().fg(Color::White),
            ),
            Span::styled(
                line_num_str,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("      "),
            Span::styled(short_msg, Style::default().fg(Color::LightRed)),
        ]),
        Line::from(vec![
            Span::styled(
                " help: ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(fix_hint, Style::default().fg(Color::Green)),
        ]),
    ]
}

fn build_diagnostic_lines(app: &App) -> Vec<Line<'static>> {
    let Some(file) = app.get_active_file() else {
        return vec![Line::from("No file open.")];
    };

    if file.valid {
        return vec![Line::from(Span::styled(
            "✓ No errors found in the active file.",
            Style::default().fg(Color::Green),
        ))];
    }

    let mut diag_lines = Vec::new();
    let err_count = file.file_errors.len();
    for (idx, file_err) in file.file_errors.iter().enumerate() {
        let mut error_lines = build_diag_lines_for_error(file_err, &file.path);
        diag_lines.append(&mut error_lines);
        if idx + 1 < err_count {
            diag_lines.push(Line::from(""));
        }
    }
    diag_lines
}

fn render_diagnostics(f: &mut Frame, app: &App, area: Rect) {
    let diag_paragraph = Paragraph::new(build_diagnostic_lines(app)).block(
        Block::default()
            .title(" Diagnostics (Esc to close) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );
    f.render_widget(diag_paragraph, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_style = if app.status_message.starts_with('✓') {
        Style::default().fg(Color::Green)
    } else if app.status_message.starts_with('✗') {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Gray)
    };

    let mode_str = match app.view_mode {
        ViewMode::Split => "SPLIT",
        ViewMode::Tabbed => "TABBED",
    };
    let anim_str = if app.show_animations { "ON" } else { "OFF" };
    let status_bar_text = format!(
        "F2:Mode({mode_str}) | F3:Anim({anim_str}) | Ctrl+S:Save | {}",
        app.status_message
    );

    let status_paragraph = Paragraph::new(status_bar_text.as_str())
        .style(status_style)
        .alignment(ratatui::layout::Alignment::Right);
    f.render_widget(status_paragraph, area);
}
fn build_input_spans<'a>(app: &'a App<'_>, is_focused: bool) -> Vec<Span<'a>> {
    let text_color = if is_focused {
        Color::White
    } else {
        Color::DarkGray
    };

    // Получаем hint для автодополнения
    let hint_str: String = if app.input_line.starts_with("open") || app.input_line.starts_with("o ") {
        let input_after_open = if app.input_line.starts_with("open ") {
            &app.input_line[5..]
        } else if app.input_line.starts_with("o ") {
            &app.input_line[2..]
        } else {
            ""
        };

        // Получаем первое автодополнение для пути
        crate::tui::get_path_completions(input_after_open)
            .first()
            .and_then(|c| c.strip_prefix(input_after_open))
            .unwrap_or("")
            .to_string()
    } else {
        KNOWN_COMMANDS
            .iter()
            .find_map(|cmd| cmd.strip_prefix(&app.input_line))
            .unwrap_or("")
            .to_string()
    };

    let mut input_spans = vec![Span::styled(
        if is_focused { "> " } else { "  " },
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];

    if app.input_line.is_empty() && !is_focused {
        input_spans.push(Span::styled(
            "Press Tab to enter command...",
            Style::default().fg(Color::DarkGray),
        ));
        return input_spans;
    }

    input_spans.push(Span::styled(app.input_line.clone(), Style::default().fg(text_color)));
    if !hint_str.is_empty() {
        input_spans.push(Span::styled(
            hint_str,
            Style::default().fg(Color::Rgb(80, 80, 80)),
        ));
    }
    input_spans
}

fn render_input_bar(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == FocusArea::Input;
    let input_spans = build_input_spans(app, is_focused);
    f.render_widget(Paragraph::new(Line::from(input_spans)), area);
}

fn render_error_popup(f: &mut Frame, area: Rect, error: &str) {
    let error_popup = Paragraph::new(error)
        .style(Style::default().fg(Color::Red))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Error ")
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: true });
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 4,
        width: area.width / 2,
        height: area.height / 2,
    };
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(error_popup, popup_area);
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(ratatui::widgets::Clear, area);

    let main_constraints = if app.show_diagnostics {
        vec![
            Constraint::Min(5),
            Constraint::Length(6), // Diagnostics panel
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Input
        ]
    } else {
        vec![
            Constraint::Min(5),
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Input
        ]
    };

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(main_constraints)
        .split(area);

    let editor_area = main_chunks[0];
    render_perimeter_animated_bar(f, editor_area, app);

    let inner_editor_area = Rect {
        x: editor_area.x + 1,
        y: editor_area.y + 1,
        width: editor_area.width.saturating_sub(2),
        height: editor_area.height.saturating_sub(2),
    };

    render_editor_area(f, app, inner_editor_area);

    let mut chunk_idx = 1;
    if app.show_diagnostics {
        render_diagnostics(f, app, main_chunks[chunk_idx]);
        chunk_idx += 1;
    }

    render_status_bar(f, app, main_chunks[chunk_idx]);
    render_input_bar(f, app, main_chunks[chunk_idx + 1]);

    if let Some(ref error) = app.error_message {
        render_error_popup(f, area, error);
    }

    if app.show_help {
        render_help_popup(f);
    }
}

struct BorderRenderer {
    head: usize,
    total_perimeter: usize,
    tail_len: usize,
    bg_color: Color,
    show_animations: bool,
}

impl BorderRenderer {
    fn new(head: usize, total_perimeter: usize, tail_len: usize, bg_color: Color, show_animations: bool) -> Self {
        Self {
            head,
            total_perimeter,
            tail_len,
            bg_color,
            show_animations,
        }
    }

    fn get_color(&self, pos: usize) -> Color {
        if self.show_animations {
            get_snake_color(self.head, pos, self.total_perimeter, self.tail_len)
                .unwrap_or(self.bg_color)
        } else {
            Color::DarkGray
        }
    }

    fn is_snake_active(&self, pos: usize) -> bool {
        self.show_animations && get_snake_color(self.head, pos, self.total_perimeter, self.tail_len).is_some()
    }
}
fn top_border_char(i: usize, width: usize, is_snake_active: bool) -> &'static str {
    if i == 0 {
        "╭"
    } else if i + 1 == width {
        "╮"
    } else if is_snake_active {
        "━"
    } else {
        "─"
    }
}

fn top_logo_char(i: usize, a1_pos: usize, a2_pos: usize, m_pos: usize) -> Option<&'static str> {
    if i == m_pos {
        Some("M")
    } else if i == a1_pos || i == a2_pos {
        Some("A")
    } else {
        None
    }
}

fn animated_logo_color(renderer: &BorderRenderer, pos: usize) -> Color {
    if !renderer.show_animations {
        return Color::Gray;
    }

    let dist_to_head = (renderer.head as isize - pos as isize).unsigned_abs();
    if dist_to_head <= 1 {
        return Color::Cyan;
    }
    if dist_to_head <= 3 {
        return Color::LightCyan;
    }

    get_snake_color(
        renderer.head,
        pos,
        renderer.total_perimeter,
        renderer.tail_len,
    )
    .unwrap_or(Color::Rgb(50, 50, 50))
}

fn render_top_border(f: &mut Frame, area: &Rect, width: usize, renderer: &BorderRenderer, a1_pos: usize, a2_pos: usize, m_pos: usize) {
    let mut top_spans = Vec::new();

    for i in 0..width {
        let is_snake_active = renderer.is_snake_active(i);
        let mut char_str = top_border_char(i, width, is_snake_active);
        let mut letter_color = renderer.get_color(i);
        let mut modifier = Modifier::empty();

        if let Some(logo_char) = top_logo_char(i, a1_pos, a2_pos, m_pos) {
            char_str = logo_char;
            letter_color = animated_logo_color(renderer, i);
            modifier = Modifier::BOLD;
        }

        top_spans.push(Span::styled(
            char_str,
            Style::default().fg(letter_color).add_modifier(modifier),
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
}

fn render_right_border(f: &mut Frame, area: &Rect, height: usize, base_pos: usize, renderer: &BorderRenderer) {
    for i in 0..height.saturating_sub(2) {
        let pos = base_pos + i;
        let color = renderer.get_color(pos);
        let char_str = if renderer.is_snake_active(pos) {
            "┃"
        } else {
            "│"
        };
        f.render_widget(
            Paragraph::new(Span::styled(char_str, Style::default().fg(color))),
            Rect {
                x: area.x + area.width - 1,
                y: area.y + 1 + i as u16,
                width: 1,
                height: 1,
            },
        );
    }
}
fn bottom_border_char(i: usize, width: usize, is_snake_active: bool) -> &'static str {
    if i == 0 {
        "╯"
    } else if i + 1 == width {
        "╰"
    } else if is_snake_active {
        "━"
    } else {
        "─"
    }
}

fn render_bottom_border(f: &mut Frame, area: &Rect, width: usize, base_pos: usize, renderer: &BorderRenderer) {
    let mut bottom_spans = Vec::new();
    let brand_text = format!(" INiNiDS v{VERSION} ");
    let brand_len = brand_text.chars().count();
    let brand_start = (width.saturating_sub(brand_len)) / 2;
    let brand_end = brand_start + brand_len;
    let mut i = 0;

    while i < width {
        if i == brand_start {
            let text_color = if renderer.show_animations {
                Color::Rgb(100, 100, 120)
            } else {
                renderer.bg_color
            };
            bottom_spans.push(Span::styled(
                &brand_text,
                Style::default().fg(text_color).add_modifier(Modifier::BOLD),
            ));
            i = brand_end;
            continue;
        }

        let pos = base_pos + (width - 1 - i);
        let color = renderer.get_color(pos);
        let char_str = bottom_border_char(i, width, renderer.is_snake_active(pos));
        bottom_spans.push(Span::styled(char_str, Style::default().fg(color)));
        i += 1;
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
}

fn render_left_border(f: &mut Frame, area: &Rect, height: usize, base_pos: usize, renderer: &BorderRenderer) {
    for i in 0..height.saturating_sub(2) {
        let pos = base_pos + i;
        let color = renderer.get_color(pos);
        let char_str = if renderer.is_snake_active(pos) {
            "┃"
        } else {
            "│"
        };
        f.render_widget(
            Paragraph::new(Span::styled(char_str, Style::default().fg(color))),
            Rect {
                x: area.x,
                y: area.y + 1 + (height - 3 - i) as u16,
                width: 1,
                height: 1,
            },
        );
    }
}

fn get_snake_color(head: usize, pos: usize, total_perimeter: usize, tail_len: usize) -> Option<Color> {
    let dist = if head >= pos {
        head - pos
    } else {
        total_perimeter - pos + head
    };

    if dist >= tail_len {
        return None;
    }

    Some(match dist {
        0..=2 => Color::Cyan,
        3..=6 => Color::LightCyan,
        7..=15 => Color::Blue,
        _ => Color::DarkGray,
    })
}

pub fn render_perimeter_animated_bar(f: &mut Frame, area: Rect, app: &App) {
    let width = area.width as usize;
    let height = area.height as usize;
    if width < 10 || height < 10 {
        return;
    }

    let total_perimeter = width * 2 + height * 2 - 4;
    let head = if app.show_animations {
        app.scanner_pos as usize % total_perimeter.max(1)
    } else {
        0
    };

    let renderer = BorderRenderer::new(
        head,
        total_perimeter,
        25,
        Color::Rgb(15, 15, 15),
        app.show_animations,
    );

    let a1_pos = width / 4;
    let a2_pos = width / 2;
    let m_pos = (width * 3) / 4;

    render_top_border(f, &area, width, &renderer, a1_pos, a2_pos, m_pos);
    render_right_border(f, &area, height, width, &renderer);
    render_bottom_border(f, &area, width, width + height.saturating_sub(2), &renderer);
    render_left_border(f, &area, height, width * 2 + height.saturating_sub(2) - 2, &renderer);
}

pub fn render_help_popup(f: &mut Frame) {
    let help_text = vec![
        Line::from(Span::styled(
            "IDE Hotkeys & Commands:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  open <file> - Open file"),
        Line::from("  save / check / format / quit"),
        Line::from(""),
        Line::from("F2     - Toggle Split/Tabbed Mode"),
        Line::from("F3     - Toggle Animation border"),
        Line::from("Ctrl+T - Check   | Ctrl+S - Save"),
        Line::from("Ctrl+F - Format  | Ctrl+Q - Quit"),
        Line::from("Ctrl+W - Close   | Tab - Switch focus"),
        Line::from("Ctrl+D - Diagnostics (Errors) Panel"),
        Line::from("Ctrl+Z - Undo    | Ctrl+Y - Redo"),
        Line::from("Ctrl+Right/Left  - Switch tabs"),
    ];

    let help_popup = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .style(Style::default().fg(Color::Cyan)),
    );

    let area = f.area();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 4,
        width: area.width / 2,
        height: 15,
    };
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(help_popup, popup_area);
}
