// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Splash Screen - Veyron startup animation
use crate::tui::colors::tui as colors;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::time::{Duration, Instant};

pub struct SplashScreen {
    created_at: Instant,
    duration: Duration,
}

impl SplashScreen {
    pub fn new(duration_ms: u64) -> Self {
        Self {
            created_at: Instant::now(),
            duration: Duration::from_millis(duration_ms),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    pub fn render(&self, f: &mut Frame) {
        let area = centered_rect(60, 50, f.area());

        // Clear background
        f.render_widget(Clear, area);

        // Main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER))
            .style(Style::default().bg(colors::BG));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // ASCII art logo
        let logo = vec![
            Line::from(Span::styled(
                "╔═══════════════════════════════════════╗",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║                                       ║",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║   ██╗   ██╗██╗██████╗ ████████╗      ║",
                Style::default()
                    .fg(colors::LIGHT_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║   ██║   ██║██║██╔══██╗╚══██╔══╝      ║",
                Style::default()
                    .fg(colors::LIGHT_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║   ██║   ██║██║██████╔╝   ██║         ║",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║   ╚██╗ ██╔╝██║██╔══██╗   ██║         ║",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║    ╚████╔╝ ██║██║  ██║   ██║         ║",
                Style::default()
                    .fg(colors::DARK_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║     ╚═══╝  ╚═╝╚═╝  ╚═╝   ╚═╝         ║",
                Style::default()
                    .fg(colors::DARK_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║                                       ║",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║   ██████╗██████╗  █████╗ ███████╗████████╗  ║",
                Style::default()
                    .fg(colors::LIGHT_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║  ██╔════╝██╔══██╗██╔══██╗██╔════╝╚══██╔══╝  ║",
                Style::default()
                    .fg(colors::LIGHT_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║  ██║     ██████╔╝███████║█████╗     ██║     ║",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║  ██║     ██╔══██╗██╔══██║██╔══╝     ██║     ║",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║  ╚██████╗██║  ██║██║  ██║██║        ██║     ║",
                Style::default()
                    .fg(colors::DARK_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║   ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝        ╚═╝     ║",
                Style::default()
                    .fg(colors::DARK_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "║                                       ║",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "╚═══════════════════════════════════════╝",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "KubeVirt VM Management & Orchestration",
                Style::default()
                    .fg(colors::TEXT)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(colors::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to continue...",
                Style::default()
                    .fg(colors::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            )),
        ];

        let paragraph = Paragraph::new(logo).alignment(Alignment::Center);
        f.render_widget(paragraph, inner);
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
