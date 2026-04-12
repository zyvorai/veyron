// Footer Widget - Contextual help and status information
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewContext {
    Dashboard,
    VmList,
    VmDetails,
    Snapshots,
    Profiles,
    Blueprints,
    Help,
}

pub struct Footer {
    last_refresh: Option<Instant>,
}

impl Footer {
    pub fn new() -> Self {
        Self { last_refresh: None }
    }

    pub fn update_refresh(&mut self) {
        self.last_refresh = Some(Instant::now());
    }

    pub fn render(&self, f: &mut Frame, area: Rect, context: ViewContext) {
        let help_text = match context {
            ViewContext::Dashboard => {
                "⌨  1-6: Jump │ r: Refresh │ ?: Help │ q: Quit"
            }
            ViewContext::VmList => {
                "⌨  ↑↓: Navigate │ Enter: Details │ s: Start │ x: Stop │ d: Delete │ c: Create │ r: Refresh │ ?: Help │ q: Quit"
            }
            ViewContext::VmDetails => {
                "⌨  s: Start │ x: Stop │ t: Restart │ d: Delete │ n: Snapshot │ Esc: Back │ r: Refresh │ ?: Help │ q: Quit"
            }
            ViewContext::Snapshots => {
                "⌨  ↑↓: Navigate │ c: Create │ r: Restore │ d: Delete │ Esc: Back │ r: Refresh │ ?: Help │ q: Quit"
            }
            ViewContext::Profiles => {
                "⌨  ↑↓: Navigate │ Enter: View │ c: Create │ e: Edit │ d: Delete │ Esc: Back │ ?: Help │ q: Quit"
            }
            ViewContext::Blueprints => {
                "⌨  ↑↓: Navigate │ Enter: Deploy │ c: Create │ e: Edit │ d: Delete │ Esc: Back │ ?: Help │ q: Quit"
            }
            ViewContext::Help => {
                "⌨  ↑↓: Scroll │ Esc: Back │ q: Quit"
            }
        };

        let time_since_refresh = if let Some(last) = self.last_refresh {
            let elapsed = last.elapsed().as_secs();
            if elapsed < 60 {
                format!("⏱  {}s ago", elapsed)
            } else {
                format!("⏱  {}m ago", elapsed / 60)
            }
        } else {
            "⏱  Never".to_string()
        };

        let footer_line = Line::from(vec![
            Span::styled(help_text, Style::default().fg(colors::TEXT)),
            Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled(
                time_since_refresh,
                Style::default()
                    .fg(colors::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]);

        let paragraph = Paragraph::new(footer_line);
        f.render_widget(paragraph, area);
    }
}

impl Default for Footer {
    fn default() -> Self {
        Self::new()
    }
}
