// Dialog Widget - Confirmation dialogs and alerts
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum DialogType {
    Confirm,
    Alert,
    Error,
    Success,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dialog {
    pub title: String,
    pub message: String,
    pub dialog_type: DialogType,
    pub selected: bool, // false = No/Cancel, true = Yes/OK
}

impl Dialog {
    pub fn new(
        title: impl Into<String>,
        message: impl Into<String>,
        dialog_type: DialogType,
    ) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            dialog_type,
            selected: false,
        }
    }

    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message, DialogType::Confirm)
    }

    pub fn alert(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message, DialogType::Alert)
    }

    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message, DialogType::Error)
    }

    pub fn success(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message, DialogType::Success)
    }

    pub fn toggle_selection(&mut self) {
        self.selected = !self.selected;
    }

    pub fn render(&self, f: &mut Frame) {
        let area = centered_rect(60, 30, f.area());

        // Clear background
        f.render_widget(Clear, area);

        // Determine colors based on dialog type
        let (border_color, title_color) = match self.dialog_type {
            DialogType::Confirm => (colors::WARNING, colors::WARNING),
            DialogType::Alert => (colors::ORANGE, colors::ORANGE),
            DialogType::Error => (colors::ERROR, colors::ERROR),
            DialogType::Success => (colors::SUCCESS, colors::SUCCESS),
        };

        // Main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                &self.title,
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Layout: Message + Buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Message
                Constraint::Length(3), // Buttons
            ])
            .split(inner);

        // Message
        let message = Paragraph::new(self.message.clone())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(message, chunks[0]);

        // Buttons
        self.render_buttons(f, chunks[1], border_color);
    }

    fn render_buttons(&self, f: &mut Frame, area: Rect, color: Color) {
        match self.dialog_type {
            DialogType::Confirm => {
                // Yes / No buttons
                let button_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(33),
                        Constraint::Percentage(17),
                        Constraint::Percentage(17),
                        Constraint::Percentage(33),
                    ])
                    .split(area);

                // Yes button
                let yes_style = if self.selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(colors::SUCCESS)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors::SUCCESS)
                };
                let yes_btn = Paragraph::new("[ Yes ]")
                    .style(yes_style)
                    .alignment(Alignment::Center);
                f.render_widget(yes_btn, button_chunks[1]);

                // No button
                let no_style = if !self.selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(colors::ERROR)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors::ERROR)
                };
                let no_btn = Paragraph::new("[ No ]")
                    .style(no_style)
                    .alignment(Alignment::Center);
                f.render_widget(no_btn, button_chunks[2]);
            }
            _ => {
                // OK button
                let ok_btn = Paragraph::new("[ OK ]")
                    .style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(color)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center);
                f.render_widget(ok_btn, area);
            }
        }
    }
}

/// Create a centered rect with given percentage width and height
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
