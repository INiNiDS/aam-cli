// SPDX-FileCopyrightText: 2026 Nikita Goncharov
// SPDX-License-Identifier: GPL-3.0-or-later

/// Strips ANSI escape codes from a string.
///
/// This function parses the input character by character, detecting ANSI escape
/// sequences (starting with `\u{001b}`) and skipping them entirely.
pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{001b}' {
            chars.next(); // skip '[' or next char
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
