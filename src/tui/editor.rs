// SPDX-FileCopyrightText: 2026 Nikita Goncharov
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::utils::strip_ansi_codes;
use aam_rs::aam::AAM;
use aam_rs::error::AamError;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::path::PathBuf;
use tui_textarea::TextArea;

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
        // Strip ANSI codes from error messages
        let short_msg = strip_ansi_codes(&err.short_message());
        let fix_hint = strip_ansi_codes(&err.diagnostics().map_or_else(
            || Self::default_help_for_error(&err).to_string(),
            |d| d.fix.clone(),
        ));

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

    const fn default_help_for_error(err: &AamError) -> &'static str {
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

    const fn extract_position(err: &AamError) -> (usize, usize) {
        match err {
            AamError::LexError { line, column, .. } => (*line, *column),
            AamError::ParseError { line, .. } => (*line, 1),
            _ => (1, 1),
        }
    }

    const fn extract_code(err: &AamError) -> &'static str {
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

    const fn extract_title(err: &AamError) -> &'static str {
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
    pub error_lines: std::collections::HashSet<usize>, // Lines with errors
}

impl<'a> FileTab<'a> {
    #[must_use]
    pub fn new(path: PathBuf, content: String) -> Self {
        let mut textarea = TextArea::default();
        for line in content.lines() {
            textarea.insert_str(line);
            textarea.insert_newline();
        }

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
                // Convert errors to FileError and store them
                self.file_errors = errors.into_iter().map(FileError::from_error).collect();
                // Update the set of error line numbers
                self.error_lines = self.file_errors.iter().map(|e| e.line).collect();
                // Clear old errors (they are now in file_errors)
                self.errors.clear();
            }
        }
        self.content = content;
        self.apply_syntax_highlighting();
    }

    pub fn apply_syntax_highlighting(&mut self) {
        self.textarea.clear_custom_highlight();

        let lines: Vec<String> = self.textarea.lines().to_vec();

        for (row, line_str) in lines.iter().enumerate() {
            if self.error_lines.contains(&(row + 1)) {
                self.textarea.custom_highlight(
                    ((row, 0), (row, line_str.len())),
                    Style::default().fg(Color::Red),
                    10,
                );
                continue;
            }

            if let Some(comment_idx) = line_str.find('#') {
                if comment_idx > 0 {
                    let before = &line_str[..comment_idx];
                    Self::highlight_aam_line_custom(&mut self.textarea, row, before);
                }
                self.textarea.custom_highlight(
                    ((row, comment_idx), (row, line_str.len())),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                    1,
                );
            } else {
                Self::highlight_aam_line_custom(&mut self.textarea, row, line_str);
            }
        }
    }

    fn highlight_aam_line_custom(textarea: &mut TextArea, row: usize, code: &str) {
        if let Some(eq_idx) = code.find('=') {
            textarea.custom_highlight(
                ((row, 0), (row, eq_idx)),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                1,
            );
            textarea.custom_highlight(
                ((row, eq_idx + 1), (row, code.len())),
                Style::default().fg(Color::Green),
                1,
            );
        } else if let Some(dir_idx) = code.find('@') {
            textarea.custom_highlight(
                ((row, dir_idx), (row, code.len())),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
                1,
            );
        }
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
