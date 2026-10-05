// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use std::io::IsTerminal;

/// Strip ANSI escape sequences to measure visible character width.
fn visible_len(s: &str) -> usize {
    let mut len = 0usize;
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if c == '\x1b' {
            in_escape = true;
        } else {
            len += 1;
        }
    }
    len
}

/// Pad a string that may contain ANSI escapes to a target *visible* width.
fn pad_visible(s: &str, target: usize) -> String {
    let vis = visible_len(s);
    if vis >= target {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(target - vis))
    }
}

/// A simple CLI table that auto-sizes columns and handles ANSI-colored content.
pub struct CliTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl CliTable {
    pub fn new(headers: Vec<&str>) -> Self {
        Self {
            headers: headers.into_iter().map(String::from).collect(),
            rows: Vec::new(),
        }
    }

    pub fn add_row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    /// Print the table to stdout.
    ///
    /// In a TTY, columns are padded and headers are styled.
    /// In a non-TTY (pipe), output is tab-separated with no styling.
    pub fn print(&self) {
        if !std::io::stdout().is_terminal() {
            self.print_plain();
            return;
        }

        let col_count = self.headers.len();
        let gap = 2; // padding between columns

        // Calculate max visible width per column
        let mut widths: Vec<usize> = self.headers.iter().map(|h| visible_len(h)).collect();

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_count {
                    widths[i] = widths[i].max(visible_len(cell));
                }
            }
        }

        // Cap total width at terminal width
        if let Ok((term_w, _)) = crossterm::terminal::size() {
            let total: usize = widths.iter().sum::<usize>() + gap * col_count.saturating_sub(1);
            if total > term_w as usize && col_count > 1 {
                let available = (term_w as usize).saturating_sub(gap * (col_count - 1));
                let scale = available as f64 / widths.iter().sum::<usize>() as f64;
                for w in &mut widths {
                    *w = (*w as f64 * scale).max(4.0) as usize;
                }
            }
        }

        // Add gap to each column width
        let padded: Vec<usize> = widths.iter().map(|w| w + gap).collect();

        // Print header
        use crate::tui::colors::cli as color;
        let header_line: String = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| pad_visible(&color::header(h), padded[i]))
            .collect();
        println!("{}", header_line);

        let sep_width: usize = padded.iter().sum();
        println!("{}", color::muted(&"─".repeat(sep_width)));

        // Print rows
        for row in &self.rows {
            let line: String = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    if i < col_count {
                        pad_visible(cell, padded[i])
                    } else {
                        cell.clone()
                    }
                })
                .collect();
            println!("{}", line);
        }
    }

    /// Tab-separated output for non-TTY contexts (grep/awk friendly).
    fn print_plain(&self) {
        println!("{}", self.headers.join("\t"));
        for row in &self.rows {
            println!("{}", row.join("\t"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visible_len_plain() {
        assert_eq!(visible_len("hello"), 5);
        assert_eq!(visible_len(""), 0);
    }

    #[test]
    fn test_visible_len_with_ansi() {
        // "\x1b[1;32mhello\x1b[0m" -> visible "hello" = 5
        assert_eq!(visible_len("\x1b[1;32mhello\x1b[0m"), 5);
        assert_eq!(visible_len("\x1b[31m\x1b[0m"), 0);
    }

    #[test]
    fn test_pad_visible_plain() {
        assert_eq!(pad_visible("hi", 5), "hi   ");
        assert_eq!(pad_visible("hello", 5), "hello");
        assert_eq!(pad_visible("toolong", 3), "toolong");
    }

    #[test]
    fn test_pad_visible_with_ansi() {
        let colored = "\x1b[1;32mhi\x1b[0m"; // visible "hi" = 2
        let padded = pad_visible(colored, 5);
        assert_eq!(visible_len(&padded), 5);
        assert!(padded.starts_with("\x1b[1;32mhi\x1b[0m"));
    }

    #[test]
    fn test_table_add_rows() {
        let mut table = CliTable::new(vec!["NAME", "STATUS"]);
        table.add_row(vec!["vm-1".into(), "Running".into()]);
        table.add_row(vec!["vm-2".into(), "Stopped".into()]);
        assert_eq!(table.rows.len(), 2);
    }
}
