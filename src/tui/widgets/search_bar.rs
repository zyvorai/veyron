// Search Bar Widget - Interactive search/filter input
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode {
    Literal,
    Regex,
}

pub struct SearchBar {
    pub query: String,
    pub mode: SearchMode,
    pub case_sensitive: bool,
    pub is_active: bool,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            mode: SearchMode::Literal,
            case_sensitive: false,
            is_active: false,
        }
    }

    pub fn activate(&mut self) {
        self.is_active = true;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    pub fn clear(&mut self) {
        self.query.clear();
    }

    pub fn add_char(&mut self, c: char) {
        self.query.push(c);
    }

    pub fn delete_char(&mut self) {
        self.query.pop();
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            SearchMode::Literal => SearchMode::Regex,
            SearchMode::Regex => SearchMode::Literal,
        };
    }

    pub fn toggle_case(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.is_active {
            return;
        }

        let mode_text = match self.mode {
            SearchMode::Literal => "literal",
            SearchMode::Regex => "regex",
        };

        let case_text = if self.case_sensitive { "Aa" } else { "aa" };

        let search_line = Line::from(vec![
            Span::styled(
                "🔍 Search: ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}] ", mode_text),
                Style::default().fg(colors::INFO),
            ),
            Span::styled(
                format!("[{}] ", case_text),
                Style::default().fg(if self.case_sensitive {
                    colors::WARNING
                } else {
                    colors::TEXT_MUTED
                }),
            ),
            Span::styled(
                &self.query,
                Style::default()
                    .fg(colors::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "█",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ]);

        let help_line = Line::from(vec![
            Span::styled("Ctrl+I: ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Case", Style::default().fg(colors::INFO)),
            Span::styled(" • ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Ctrl+R: ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Regex", Style::default().fg(colors::INFO)),
            Span::styled(" • ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("ESC: ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Cancel", Style::default().fg(colors::INFO)),
            Span::styled(" • ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Enter: ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Apply", Style::default().fg(colors::INFO)),
        ]);

        let paragraph = Paragraph::new(vec![search_line, help_line])
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::ORANGE)),
            );

        f.render_widget(paragraph, area);
    }
}

impl Default for SearchBar {
    fn default() -> Self {
        Self::new()
    }
}
