// Detailed Security Posture View

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

pub fn render(f: &mut Frame, area: Rect, _state: &AppState) {
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
        "Security Posture Analysis",
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

    // Category gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    let categories = [
        ("Network", 75, Color::Rgb(255, 200, 0)),
        ("Identity", 90, Color::Rgb(50, 205, 50)),
        ("Workload", 68, Color::Rgb(255, 200, 0)),
        ("Data", 85, Color::Rgb(50, 205, 50)),
    ];

    for (i, (name, pct, color)) in categories.iter().enumerate() {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                    .title(format!(" {} ", name)),
            )
            .gauge_style(Style::default().fg(*color).bg(Color::Rgb(40, 35, 55)))
            .percent(*pct)
            .label(format!("{}%", pct));
        f.render_widget(gauge, gauge_chunks[i]);
    }

    // Detailed posture breakdown
    let detail_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    let network_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Network Security",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "Network policies enforced",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("Ingress TLS termination", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [FAIL] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                "Default deny egress missing",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "Service mesh mTLS enabled",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Workload Security",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "Pod security standards enforced",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  [FAIL] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                "3 containers run as root",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                "5 pods without resource limits",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "Image pull policy: Always",
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    let network_widget = Paragraph::new(network_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Control Checks ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(network_widget, detail_chunks[0]);

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
                "Add default-deny egress NetworkPolicy",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("     Impact: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "+5 posture score",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                "Set runAsNonRoot for 3 containers",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("     Impact: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "+4 posture score",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                "Define resource limits on 5 pods",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("     Impact: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "+3 posture score",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Total potential: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "82 -> 94 (+12)",
                Style::default()
                    .fg(Color::Rgb(50, 205, 50))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let rec_widget = Paragraph::new(recommendations).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Remediation Plan ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(rec_widget, detail_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "↑↓",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Remediate | ", Style::default().fg(Color::Gray)),
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
