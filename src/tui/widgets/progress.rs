// Progress Bar Widget - Show progress for long-running operations
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressBar {
    pub title: String,
    pub message: String,
    pub progress: f64, // 0.0 to 1.0
    pub indeterminate: bool,
}

impl ProgressBar {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            progress: 0.0,
            indeterminate: false,
        }
    }

    pub fn indeterminate(mut self) -> Self {
        self.indeterminate = true;
        self
    }

    pub fn with_progress(mut self, progress: f64) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    pub fn set_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    pub fn render(&self, f: &mut Frame) {
        let area = centered_rect(60, 25, f.area());

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

        // Layout: Message + Progress bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Message
                Constraint::Length(3), // Progress bar
                Constraint::Min(0),    // Spacer
            ])
            .split(inner);

        // Message
        let message = Paragraph::new(self.message.clone())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));
        f.render_widget(message, chunks[0]);

        // Progress bar
        if self.indeterminate {
            // Spinner animation
            let spinner = Paragraph::new("⠋ Working...")
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(colors::ORANGE)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(spinner, chunks[1]);
        } else {
            let percent = (self.progress * 100.0) as u16;
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::NONE))
                .gauge_style(
                    Style::default()
                        .fg(colors::ORANGE)
                        .bg(colors::TEXT_MUTED)
                        .add_modifier(Modifier::BOLD),
                )
                .percent(percent)
                .label(format!("{}%", percent));
            f.render_widget(gauge, chunks[1]);
        }
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
