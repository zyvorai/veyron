// Resource Forecast Dashboard View

use crate::tui::colors::gradient;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

pub fn render(f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    // Gradient brand header
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "Resource Forecast",
        Style::default()
            .fg(Color::Rgb(255, 145, 115))
            .add_modifier(Modifier::BOLD),
    ));
    let header_text = Line::from(header_spans);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
        );
    f.render_widget(header, chunks[0]);

    // Current utilization gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[1]);

    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" CPU Now "),
        )
        .gauge_style(
            Style::default()
                .fg(gradient::health().at(0.52))
                .bg(Color::Rgb(40, 35, 55)),
        )
        .percent(52)
        .label("52% of 40 cores");
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Memory Now "),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(100, 150, 255))
                .bg(Color::Rgb(40, 35, 55)),
        )
        .percent(67)
        .label("67% of 160 GiB");
    f.render_widget(mem_gauge, gauge_chunks[1]);

    let storage_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Storage Now "),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(255, 200, 0))
                .bg(Color::Rgb(40, 35, 55)),
        )
        .percent(78)
        .label("78% of 2 TiB");
    f.render_widget(storage_gauge, gauge_chunks[2]);

    // Forecast details
    let forecast_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    let predictions = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Capacity Predictions",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  CPU (40 cores)",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("    7 days:  ", Style::default().fg(Color::Gray)),
            Span::styled("58%", Style::default().fg(Color::White)),
            Span::styled(" (+6%)", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
        Line::from(vec![
            Span::styled("    30 days: ", Style::default().fg(Color::Gray)),
            Span::styled("71%", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(" (+19%)", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
        Line::from(vec![
            Span::styled("    90 days: ", Style::default().fg(Color::Gray)),
            Span::styled("89%", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                " (+37%) CAPACITY WARNING",
                Style::default().fg(Color::Rgb(220, 50, 47)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Memory (160 GiB)",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("    7 days:  ", Style::default().fg(Color::Gray)),
            Span::styled("69%", Style::default().fg(Color::White)),
            Span::styled(" (+2%)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("    30 days: ", Style::default().fg(Color::Gray)),
            Span::styled("74%", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(" (+7%)", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
        Line::from(vec![
            Span::styled("    90 days: ", Style::default().fg(Color::Gray)),
            Span::styled("82%", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(" (+15%)", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Storage (2 TiB)",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("    7 days:  ", Style::default().fg(Color::Gray)),
            Span::styled("80%", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(" (+2%)", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
        Line::from(vec![
            Span::styled("    30 days: ", Style::default().fg(Color::Gray)),
            Span::styled("87%", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(" (+9%)", Style::default().fg(Color::Rgb(220, 50, 47))),
        ]),
        Line::from(vec![
            Span::styled("    90 days: ", Style::default().fg(Color::Gray)),
            Span::styled("95%", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                " (+17%) CRITICAL",
                Style::default()
                    .fg(Color::Rgb(220, 50, 47))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let predictions_widget = Paragraph::new(predictions).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Forecast ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(predictions_widget, forecast_chunks[0]);

    let recommendations = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Recommendations",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1. ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                "Expand storage within 30 days",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![Span::styled(
            "     Add 500 GiB to prevent capacity exhaustion",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                "Plan CPU capacity increase in 60 days",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![Span::styled(
            "     Add 1 worker node (16 cores)",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled(
                "Review idle VM resources",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![Span::styled(
            "     3 VMs using <5% CPU could be right-sized",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(Span::styled(
            "  Confidence",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  7-day:  ", Style::default().fg(Color::Gray)),
            Span::styled("95%", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(vec![
            Span::styled("  30-day: ", Style::default().fg(Color::Gray)),
            Span::styled("82%", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(vec![
            Span::styled("  90-day: ", Style::default().fg(Color::Gray)),
            Span::styled("68%", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
    ];
    let rec_widget = Paragraph::new(recommendations).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Actions & Confidence ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(rec_widget, forecast_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "p",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Period | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Refresh | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "e",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Export | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "q",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Back", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
    );
    f.render_widget(help, chunks[3]);
}
