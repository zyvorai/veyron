// Resource Gauge Widget - Visual resource usage indicators with gradient support
use crate::tui::colors::gradient;
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

pub struct ResourceGauge {
    pub label: String,
    pub value: f64,
    pub max: f64,
    pub unit: String,
    pub show_percentage: bool,
    pub use_gradient: bool,
}

impl ResourceGauge {
    pub fn new(label: impl Into<String>, value: f64, max: f64, unit: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value,
            max,
            unit: unit.into(),
            show_percentage: true,
            use_gradient: true,
        }
    }

    pub fn percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let percentage = if self.max > 0.0 {
            ((self.value / self.max) * 100.0).min(100.0)
        } else {
            0.0
        };

        if self.use_gradient && area.width > 4 {
            // Gradient gauge rendering
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let label = if self.show_percentage {
                format!(
                    "{}: {:.1}/{} {} ({:.0}%)",
                    self.label, self.value, self.max, self.unit, percentage
                )
            } else {
                format!("{}: {:.1}/{} {}", self.label, self.value, self.max, self.unit)
            };

            let bar_width = inner.width as usize;
            let grad = gradient::health();
            let mut spans = grad.bar(bar_width, percentage / 100.0);

            // Overlay label text on the bar
            let label_chars: Vec<char> = label.chars().collect();
            let label_len = label_chars.len().min(bar_width);
            let start = bar_width.saturating_sub(label_len) / 2;
            for (j, ch) in label_chars.iter().take(label_len).enumerate() {
                let idx = start + j;
                if idx < spans.len() {
                    let bg = if idx < (percentage / 100.0 * bar_width as f64) as usize {
                        grad.at(idx as f64 / bar_width.max(1) as f64)
                    } else {
                        ratatui::style::Color::Rgb(60, 60, 60)
                    };
                    let mut buf = [0u8; 4];
                    spans[idx] = Span::styled(
                        ch.encode_utf8(&mut buf).to_string(),
                        Style::default().fg(colors::TEXT).bg(bg).add_modifier(Modifier::BOLD),
                    );
                }
            }

            let gauge_line = Paragraph::new(Line::from(spans));
            f.render_widget(gauge_line, inner);
        } else {
            // Fallback: standard ratatui Gauge
            let color = gradient::health().at(percentage / 100.0);
            let label = if self.show_percentage {
                format!("{}: {:.1}/{} {} ({:.0}%)", self.label, self.value, self.max, self.unit, percentage)
            } else {
                format!("{}: {:.1}/{} {}", self.label, self.value, self.max, self.unit)
            };

            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(colors::BORDER)))
                .gauge_style(Style::default().fg(color).bg(colors::TEXT_MUTED).add_modifier(Modifier::BOLD))
                .percent(percentage as u16)
                .label(label);
            f.render_widget(gauge, area);
        }
    }
}

pub struct MultiGaugePanel {
    pub title: String,
    pub gauges: Vec<ResourceGauge>,
}

impl MultiGaugePanel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            gauges: Vec::new(),
        }
    }

    pub fn add_gauge(mut self, gauge: ResourceGauge) -> Self {
        self.gauges.push(gauge);
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::text::Span;

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER))
            .title(Span::styled(
                &self.title,
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Create constraints for each gauge
        let constraints: Vec<Constraint> =
            self.gauges.iter().map(|_| Constraint::Length(3)).collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, gauge) in self.gauges.iter().enumerate() {
            if i < chunks.len() {
                gauge.render(f, chunks[i]);
            }
        }
    }
}
