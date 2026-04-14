// Input Dialog Widget - Text input forms
use crate::tui::colors::tui as colors;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

#[derive(Debug, Clone, PartialEq)]
pub struct InputField {
    pub label: String,
    pub value: String,
    pub placeholder: String,
}

impl InputField {
    pub fn new(label: impl Into<String>, placeholder: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            placeholder: placeholder.into(),
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputDialog {
    pub title: String,
    pub fields: Vec<InputField>,
    pub current_field: usize,
    pub submitted: bool,
    pub cancelled: bool,
}

impl InputDialog {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            fields: Vec::new(),
            current_field: 0,
            submitted: false,
            cancelled: false,
        }
    }

    pub fn add_field(mut self, field: InputField) -> Self {
        self.fields.push(field);
        self
    }

    pub fn next_field(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        if self.current_field < self.fields.len() - 1 {
            self.current_field += 1;
        }
    }

    pub fn prev_field(&mut self) {
        if self.current_field > 0 {
            self.current_field -= 1;
        }
    }

    pub fn current_value_mut(&mut self) -> Option<&mut String> {
        self.fields
            .get_mut(self.current_field)
            .map(|f| &mut f.value)
    }

    pub fn add_char(&mut self, c: char) {
        if let Some(field) = self.fields.get_mut(self.current_field) {
            field.value.push(c);
        }
    }

    pub fn delete_char(&mut self) {
        if let Some(field) = self.fields.get_mut(self.current_field) {
            field.value.pop();
        }
    }

    pub fn submit(&mut self) {
        self.submitted = true;
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn get_value(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(|f| f.value.as_str())
    }

    pub fn render(&self, f: &mut Frame) {
        let area = centered_rect(70, 50, f.area());

        // Clear background
        f.render_widget(Clear, area);

        // Main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::ORANGE))
            .title(Span::styled(
                &self.title,
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Calculate constraints for fields + buttons
        let mut constraints = Vec::new();
        for _ in 0..self.fields.len() {
            constraints.push(Constraint::Length(3)); // Each field takes 3 lines
        }
        constraints.push(Constraint::Min(1)); // Spacer
        constraints.push(Constraint::Length(3)); // Buttons

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        // Render fields
        for (i, field) in self.fields.iter().enumerate() {
            self.render_field(f, field, chunks[i], i == self.current_field);
        }

        // Render buttons
        self.render_buttons(f, chunks[chunks.len() - 1]);
    }

    fn render_field(&self, f: &mut Frame, field: &InputField, area: Rect, is_active: bool) {
        let display_value = if field.value.is_empty() {
            &field.placeholder
        } else {
            &field.value
        };

        let style = if is_active {
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let cursor = if is_active { "█" } else { "" };

        let text = vec![
            Line::from(Span::styled(
                &field.label,
                Style::default().fg(Color::White),
            )),
            Line::from(vec![
                Span::styled("  ", style),
                Span::styled(display_value, style),
                Span::styled(cursor, Style::default().fg(colors::WARNING)),
            ]),
        ];

        let paragraph = Paragraph::new(text);
        f.render_widget(paragraph, area);
    }

    fn render_buttons(&self, f: &mut Frame, area: Rect) {
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(30),
            ])
            .split(area);

        // Submit button
        let submit_btn = Paragraph::new("[ Submit ]")
            .style(
                Style::default()
                    .fg(colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(submit_btn, button_chunks[1]);

        // Cancel button
        let cancel_btn = Paragraph::new("[ Cancel ]")
            .style(
                Style::default()
                    .fg(colors::ERROR)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(cancel_btn, button_chunks[2]);

        // Help text
        let help = Paragraph::new("Tab: Next field | Enter: Submit | Esc: Cancel")
            .style(Style::default().fg(colors::TEXT_MUTED))
            .alignment(Alignment::Center);

        let help_area = Rect {
            x: area.x,
            y: area.y.saturating_add(area.height.saturating_sub(1)),
            width: area.width,
            height: 1,
        };
        f.render_widget(help, help_area);
    }
}

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
