// Notification Widget - Toast-style notifications
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub notification_type: NotificationType,
    pub created_at: Instant,
    pub duration: Duration,
}

impl Notification {
    pub fn new(message: impl Into<String>, notification_type: NotificationType) -> Self {
        Self {
            message: message.into(),
            notification_type,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, NotificationType::Info)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, NotificationType::Success)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, NotificationType::Warning)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, NotificationType::Error)
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    pub fn render(&self, f: &mut Frame, index: usize) {
        let area = f.area();

        // Position notifications stacked from top-right
        let notification_area = Rect {
            x: area.width.saturating_sub(52),
            y: 1 + (index as u16 * 4),
            width: 50,
            height: 3,
        };

        // Don't render if off screen
        if notification_area.y + notification_area.height > area.height {
            return;
        }

        // Clear background
        f.render_widget(Clear, notification_area);

        // Determine colors and icon based on type
        let (color, icon) = match self.notification_type {
            NotificationType::Info => (colors::ORANGE, "ℹ"),
            NotificationType::Success => (colors::SUCCESS, "✓"),
            NotificationType::Warning => (colors::WARNING, "⚠"),
            NotificationType::Error => (colors::ERROR, "✗"),
        };

        // Main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(notification_area);
        f.render_widget(block, notification_area);

        // Message with icon
        let text = format!("{} {}", icon, self.message);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, inner);
    }
}

#[derive(Debug, Default)]
pub struct NotificationManager {
    notifications: Vec<Notification>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
        }
    }

    pub fn push(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    pub fn info(&mut self, message: impl Into<String>) {
        self.push(Notification::info(message));
    }

    pub fn success(&mut self, message: impl Into<String>) {
        self.push(Notification::success(message));
    }

    pub fn warning(&mut self, message: impl Into<String>) {
        self.push(Notification::warning(message));
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.push(Notification::error(message));
    }

    pub fn update(&mut self) {
        // Remove expired notifications
        self.notifications.retain(|n| !n.is_expired());
    }

    pub fn render(&self, f: &mut Frame) {
        for (index, notification) in self.notifications.iter().enumerate() {
            notification.render(f, index);
        }
    }

    pub fn clear(&mut self) {
        self.notifications.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }
}
