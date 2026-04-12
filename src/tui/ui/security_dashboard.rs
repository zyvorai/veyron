// Security Dashboard View

use crate::tui::colors::gradient;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
    Frame,
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
    header_spans.push(Span::styled(" | ", Style::default().fg(Color::Rgb(128, 128, 128))));
    header_spans.push(Span::styled("Security Dashboard", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
    let header_text = Line::from(header_spans);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
        );
    f.render_widget(header, chunks[0]);

    // Security posture score gauge
    let score_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(chunks[1]);

    let score_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Security Posture Score "),
        )
        .gauge_style(Style::default().fg(Color::Rgb(50, 205, 50)).bg(Color::Rgb(40, 35, 55)))
        .percent(82)
        .label("82/100 - Good");
    f.render_widget(score_gauge, score_chunks[0]);

    let summary = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Critical: ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled("1", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("  High: ", Style::default().fg(Color::Rgb(255, 165, 0))),
            Span::styled("3", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("  Medium: ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("7", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("  Low: ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled("12", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
    ];
    let summary_widget = Paragraph::new(summary).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(" Findings by Severity "),
    );
    f.render_widget(summary_widget, score_chunks[1]);

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .split(chunks[2]);

    // Findings table
    let header_cells = ["Severity", "Category", "Finding", "Status"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )
        });
    let table_header = Row::new(header_cells)
        .style(Style::default().bg(Color::Rgb(40, 35, 55)))
        .height(1);

    let findings = [("CRIT", "Network", "Pod network policy missing", "Open"),
        ("HIGH", "RBAC", "Overly permissive role", "In Progress"),
        ("HIGH", "Image", "Unscanned container image", "Open"),
        ("HIGH", "Config", "Secrets in env vars", "Open"),
        ("MED", "Network", "Ingress without TLS", "Remediated"),
        ("MED", "Runtime", "Privileged container", "Open"),
        ("LOW", "Config", "Resource limits not set", "Open")];

    let rows = findings.iter().map(|(sev, cat, finding, status)| {
        let sev_color = match *sev {
            "CRIT" => Color::Rgb(220, 50, 47),
            "HIGH" => Color::Rgb(255, 165, 0),
            "MED" => Color::Rgb(255, 200, 0),
            _ => Color::Rgb(100, 150, 255),
        };
        let status_color = match *status {
            "Open" => Color::Rgb(220, 50, 47),
            "In Progress" => Color::Rgb(255, 200, 0),
            "Remediated" => Color::Rgb(50, 205, 50),
            _ => Color::Gray,
        };
        Row::new(vec![
            Cell::from(*sev).style(Style::default().fg(sev_color).add_modifier(Modifier::BOLD)),
            Cell::from(*cat),
            Cell::from(*finding),
            Cell::from(*status).style(Style::default().fg(status_color)),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Length(6),
        Constraint::Percentage(18),
        Constraint::Percentage(48),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Security Findings ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, main_chunks[0]);

    // Compliance status
    let compliance = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Compliance Frameworks",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  CIS Benchmark    ", Style::default().fg(Color::White)),
            Span::styled("78%  ", Style::default().fg(Color::Rgb(255, 200, 0)).add_modifier(Modifier::BOLD)),
            Span::styled("(62/79 controls)", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  PCI DSS          ", Style::default().fg(Color::White)),
            Span::styled("91%  ", Style::default().fg(Color::Rgb(50, 205, 50)).add_modifier(Modifier::BOLD)),
            Span::styled("(221/243 controls)", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  SOC 2            ", Style::default().fg(Color::White)),
            Span::styled("85%  ", Style::default().fg(Color::Rgb(50, 205, 50)).add_modifier(Modifier::BOLD)),
            Span::styled("(102/120 controls)", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  HIPAA            ", Style::default().fg(Color::White)),
            Span::styled("73%  ", Style::default().fg(Color::Rgb(255, 200, 0)).add_modifier(Modifier::BOLD)),
            Span::styled("(44/60 controls)", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Last scan: 2h ago",
            Style::default().fg(Color::Gray),
        )),
    ];
    let compliance_widget = Paragraph::new(compliance).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Compliance Status ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(compliance_widget, main_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("s", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Scan | ", Style::default().fg(Color::Gray)),
        Span::styled("r", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Remediate | ", Style::default().fg(Color::Gray)),
        Span::styled("q", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
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
