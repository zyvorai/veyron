// Bar Chart Widget - Distribution visualization with gradient coloring
use crate::tui::colors::gradient;
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{BarChart as RatatuiBarChart, Block, Borders},
    Frame,
};

pub struct BarChart {
    pub title: String,
    pub data: Vec<(String, u64)>,
    pub bar_width: u16,
    pub bar_gap: u16,
}

impl BarChart {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            data: Vec::new(),
            bar_width: 5,
            bar_gap: 2,
        }
    }

    pub fn add_bar(mut self, label: impl Into<String>, value: u64) -> Self {
        self.data.push((label.into(), value));
        self
    }

    pub fn bar_width(mut self, width: u16) -> Self {
        self.bar_width = width;
        self
    }

    pub fn bar_gap(mut self, gap: u16) -> Self {
        self.bar_gap = gap;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let bar_data: Vec<(&str, u64)> = self
            .data
            .iter()
            .map(|(label, value)| (label.as_str(), *value))
            .collect();

        // Gradient title
        let title_spans = gradient::brand().text(&self.title);

        // Determine bar color from the max value's proportion
        let max_val = self.data.iter().map(|(_, v)| *v).max().unwrap_or(1);
        let grad = gradient::sunset();
        let bar_color = grad.at(0.5);

        let chart = RatatuiBarChart::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::BORDER))
                    .title(Line::from(title_spans)),
            )
            .data(&bar_data)
            .bar_width(self.bar_width)
            .bar_gap(self.bar_gap)
            .bar_style(Style::default().fg(bar_color))
            .value_style(
                Style::default()
                    .fg(colors::TEXT)
                    .add_modifier(Modifier::BOLD),
            );

        let _ = max_val; // used for potential per-bar gradient in future
        f.render_widget(chart, area);
    }
}
