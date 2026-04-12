// SPDX-FileCopyrightText: 2026 Nikita Goncharov
// SPDX-License-Identifier: GPL-3.0-or-later

use aam_rs::aam::AAM;
use aam_rs::error::AamError;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::path::PathBuf;
use tui_textarea::TextArea;

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

pub struct FileError {
    pub error: AamError,
    pub line: usize,
    pub column: usize,
    pub code: &'static str,
    pub title: &'static str,
    pub short_msg: String,
    pub fix_hint: String,
}

impl FileError {
    fn from_error(err: AamError) -> Self {
        let (line, column) = Self::extract_position(&err);
        let code = Self::extract_code(&err);
        let title = Self::extract_title(&err);
        // Очищаем ANSI коды из сообщений об ошибках
        let short_msg = strip_ansi_codes(&err.short_message());
        let fix_hint = strip_ansi_codes(&err
            .diagnostics().map_or_else(|| Self::default_help_for_error(&err).to_string(), |d| d.fix.clone()));

        Self {
            error: err,
            line,
            column,
            code,
            title,
            short_msg,
            fix_hint,
        }
    }

    fn default_help_for_error(err: &AamError) -> &'static str {
        match err {
            AamError::CircularDependency { .. } => {
                "types in AAM must be acyclic. Consider using a primitive type or breaking the loop."
            }
            AamError::ParseError { .. } => {
                "check assignment/directive syntax near the highlighted line."
            }
            AamError::InvalidType { .. } => {
                "ensure the value matches the declared type or update the type declaration."
            }
            AamError::SchemaValidationError { .. } | AamError::MissingRequiredField { .. } => {
                "fill required fields and ensure each field value matches its declared type."
            }
            AamError::NotFound { .. } => {
                "verify the referenced key/type/schema exists and is in scope."
            }
            AamError::DirectiveError { .. } | AamError::DirectiveSyntaxError { .. } => {
                "check directive name and argument format."
            }
            AamError::MalformedLiteral { .. } => {
                "ensure object/list literals are balanced and well-formed."
            }
            AamError::TypeRegistrationConflict { .. } => {
                "rename the type or remove duplicate @type declarations."
            }
            AamError::TypeConversionError { .. } => {
                "provide a value that can be converted to the requested type."
            }
            AamError::InvalidValue { .. } => "provide a value in the expected format.",
            AamError::NestingDepthExceeded { .. } => "reduce recursion depth or split nested data.",
            AamError::LexError { .. } => "remove or replace unsupported characters.",
            AamError::IoError { .. } => "check file path and permissions.",
        }
    }

    fn extract_position(err: &AamError) -> (usize, usize) {
        match err {
            AamError::LexError { line, column, .. } => (*line, *column),
            AamError::ParseError { line, .. } => (*line, 1),
            _ => (1, 1),
        }
    }

    fn extract_code(err: &AamError) -> &'static str {
        match err {
            AamError::CircularDependency { .. } => "E001",
            AamError::ParseError { .. } => "E002",
            AamError::InvalidType { .. } => "E003",
            AamError::SchemaValidationError { .. } => "E004",
            AamError::MissingRequiredField { .. } => "E005",
            AamError::NotFound { .. } => "E006",
            AamError::DirectiveError { .. } => "E007",
            AamError::DirectiveSyntaxError { .. } => "E008",
            AamError::MalformedLiteral { .. } => "E009",
            AamError::TypeRegistrationConflict { .. } => "E010",
            AamError::TypeConversionError { .. } => "E011",
            AamError::InvalidValue { .. } => "E012",
            AamError::NestingDepthExceeded { .. } => "E013",
            AamError::LexError { .. } => "E014",
            AamError::IoError { .. } => "E015",
        }
    }

    fn extract_title(err: &AamError) -> &'static str {
        match err {
            AamError::CircularDependency { .. } => "cyclic dependency detected",
            AamError::ParseError { .. } => "parse error",
            AamError::InvalidType { .. } => "type validation failed",
            AamError::SchemaValidationError { .. } => "schema validation failed",
            AamError::MissingRequiredField { .. } => "missing required field",
            AamError::NotFound { .. } => "entry not found",
            AamError::DirectiveError { .. } => "directive execution failed",
            AamError::DirectiveSyntaxError { .. } => "directive syntax error",
            AamError::MalformedLiteral { .. } => "malformed literal",
            AamError::TypeRegistrationConflict { .. } => "type registration conflict",
            AamError::TypeConversionError { .. } => "type conversion failed",
            AamError::InvalidValue { .. } => "invalid value",
            AamError::NestingDepthExceeded { .. } => "nesting depth exceeded",
            AamError::LexError { .. } => "lexical analysis failed",
            AamError::IoError { .. } => "I/O operation failed",
        }
    }
}

pub struct FileTab<'a> {
    pub path: PathBuf,
    pub content: String,
    pub textarea: TextArea<'a>,
    pub valid: bool,
    pub error_count: usize,
    pub errors: Vec<AamError>,
    pub file_errors: Vec<FileError>,
    pub error_lines: std::collections::HashSet<usize>, // Линии с ошибками
}

impl<'a> FileTab<'a> {
    #[must_use] 
    pub fn new(path: PathBuf, content: String) -> Self {
        let mut textarea = TextArea::default();
        for line in content.lines() {
            textarea.insert_str(line);
            textarea.insert_newline();
        }

        // Improved AAM syntax highlighting hack: Matches Keys, Directives (@), and comments (#)
        let _ = textarea.set_search_pattern(r"(?m)^(?:[\w\.\-]+)\s*(?==)|#.*$|@\w+");
        textarea.set_search_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        let mut tab = Self {
            path,
            content,
            textarea,
            valid: true,
            error_count: 0,
            errors: Vec::new(),
            file_errors: Vec::new(),
            error_lines: std::collections::HashSet::new(),
        };
        tab.check_validity();
        tab
    }

    pub fn check_validity(&mut self) {
        let content = self.textarea.lines().join("\n");
        match AAM::parse(&content) {
            Ok(_) => {
                self.valid = true;
                self.error_count = 0;
                self.errors.clear();
                self.file_errors.clear();
                self.error_lines.clear();
            }
            Err(errors) => {
                self.valid = false;
                self.error_count = errors.len();
                // Convert errors в FileError и сохраняем их
                self.file_errors = errors.into_iter().map(FileError::from_error).collect();
                // Обновляем множество линий с ошибками
                self.error_lines = self.file_errors.iter().map(|e| e.line).collect();
                // Очищаем старые ошибки (они теперь в file_errors)
                self.errors.clear();
            }
        }
        self.content = content;
    }

    // Helper for beautiful syntax highlighting of read-only/inactive tabs
    pub fn get_syntax_highlighted_lines(&self) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        for line_str in self.textarea.lines() {
            let mut spans = Vec::new();
            if let Some(comment_idx) = line_str.find('#') {
                if comment_idx > 0 {
                    let before = &line_str[..comment_idx];
                    Self::highlight_aam_code(before, &mut spans);
                }
                spans.push(Span::styled(
                    line_str[comment_idx..].to_string(),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ));
            } else {
                Self::highlight_aam_code(line_str, &mut spans);
            }
            lines.push(Line::from(spans));
        }
        lines
    }

    fn highlight_aam_code(code: &str, spans: &mut Vec<Span<'a>>) {
        if let Some(eq_idx) = code.find('=') {
            let key = &code[..eq_idx];
            spans.push(Span::styled(
                key.to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw("=".to_string()));
            let val = &code[eq_idx + 1..];
            spans.push(Span::styled(
                val.to_string(),
                Style::default().fg(Color::Green),
            ));
        } else {
            spans.push(Span::raw(code.to_string()));
        }
    }
}
