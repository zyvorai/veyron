// Stats Bar Widget - Display VM statistics at a glance
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone)]
pub struct StatsBar {
    pub total_vms: usize,
    pub running_vms: usize,
    pub stopped_vms: usize,
    pub pending_vms: usize,
    pub snapshots: usize,
    pub namespace: String,
}

impl StatsBar {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            total_vms: 0,
            running_vms: 0,
            stopped_vms: 0,
            pending_vms: 0,
            snapshots: 0,
            namespace: namespace.into(),
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        // Calculate health status
        let health_icon = if self.running_vms == 0 && self.total_vms > 0 {
            "🔴"
        } else if self.pending_vms > 0 {
            "🟡"
        } else if self.running_vms > 0 {
            "🟢"
        } else {
            "⚪"
        };

        let health_text = if self.running_vms == 0 && self.total_vms > 0 {
            "All Stopped"
        } else if self.pending_vms > 0 {
            "Starting"
        } else if self.running_vms > 0 {
            "Healthy"
        } else {
            "No VMs"
        };

        let stats_line = Line::from(vec![
            Span::styled("📊 ", Style::default().fg(colors::ORANGE)),
            Span::styled("VMs: ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{} ", self.total_vms),
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Running: ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{} ", self.running_vms),
                Style::default()
                    .fg(colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Stopped: ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{} ", self.stopped_vms),
                Style::default()
                    .fg(colors::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Snapshots: ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{} ", self.snapshots),
                Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("Status: ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{} {} ", health_icon, health_text),
                Style::default()
                    .fg(colors::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("NS: ", Style::default().fg(colors::TEXT)),
            Span::styled(
                &self.namespace,
                Style::default()
                    .fg(colors::LIGHT_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let paragraph = Paragraph::new(stats_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );

        f.render_widget(paragraph, area);
    }
}
