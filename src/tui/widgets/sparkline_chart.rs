// Sparkline Chart Widget - Trend visualization with gradient titles
use crate::tui::colors::gradient;
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Sparkline},
    Frame,
};

pub struct SparklineChart {
    pub title: String,
    pub data: Vec<u64>,
    pub max_value: Option<u64>,
}

impl SparklineChart {
    pub fn new(title: impl Into<String>, data: Vec<u64>) -> Self {
        Self {
            title: title.into(),
            data,
            max_value: None,
        }
    }

    pub fn with_max(mut self, max: u64) -> Self {
        self.max_value = Some(max);
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let max = self
            .max_value
            .unwrap_or_else(|| *self.data.iter().max().unwrap_or(&1));

        // Determine sparkline color from latest value relative to max
        let latest = self.data.last().copied().unwrap_or(0);
        let ratio = if max > 0 { latest as f64 / max as f64 } else { 0.0 };
        let spark_color = gradient::health().at(ratio);

        // Gradient title
        let grad = gradient::brand();
        let title_spans = grad.text(&self.title);
        let mut title_line: Vec<Span> = title_spans;
        // Append current value
        let val_text = format!(" {}", latest);
        title_line.push(Span::styled(
            val_text,
            Style::default().fg(spark_color).add_modifier(Modifier::BOLD),
        ));

        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::BORDER))
                    .title(Line::from(title_line)),
            )
            .data(&self.data)
            .style(Style::default().fg(spark_color))
            .max(max);

        f.render_widget(sparkline, area);
    }
}
