// Compliance Checker View

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
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
        "Compliance Checker",
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

    // Framework gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    let frameworks = [
        ("CIS K8s", 78, Color::Rgb(255, 200, 0)),
        ("PCI DSS", 91, Color::Rgb(50, 205, 50)),
        ("SOC 2", 85, Color::Rgb(50, 205, 50)),
        ("HIPAA", 73, Color::Rgb(255, 200, 0)),
    ];

    for (i, (name, pct, color)) in frameworks.iter().enumerate() {
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

    // Controls table
    let header_cells = [
        "Control ID",
        "Framework",
        "Description",
        "Status",
        "Severity",
    ]
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

    let controls = vec![
        (
            "CIS-5.2.1",
            "CIS K8s",
            "Minimize admission of privileged containers",
            "FAIL",
            "HIGH",
        ),
        (
            "CIS-5.2.3",
            "CIS K8s",
            "Minimize containers with added capabilities",
            "PASS",
            "MED",
        ),
        (
            "CIS-5.3.2",
            "CIS K8s",
            "Ensure NetworkPolicy for every namespace",
            "FAIL",
            "HIGH",
        ),
        (
            "PCI-2.2.1",
            "PCI DSS",
            "Implement only one primary function per server",
            "PASS",
            "HIGH",
        ),
        (
            "PCI-6.5.1",
            "PCI DSS",
            "Address common coding vulnerabilities",
            "PASS",
            "CRIT",
        ),
        (
            "SOC-CC6.1",
            "SOC 2",
            "Logical and physical access controls",
            "PASS",
            "HIGH",
        ),
        (
            "SOC-CC7.2",
            "SOC 2",
            "Monitor system components for anomalies",
            "WARN",
            "MED",
        ),
        (
            "HIPAA-164.312a",
            "HIPAA",
            "Access control - unique user identification",
            "PASS",
            "CRIT",
        ),
        (
            "HIPAA-164.312e",
            "HIPAA",
            "Transmission security",
            "FAIL",
            "CRIT",
        ),
    ];

    let rows = controls.iter().map(|(id, fw, desc, status, sev)| {
        let status_color = match *status {
            "PASS" => Color::Rgb(50, 205, 50),
            "FAIL" => Color::Rgb(220, 50, 47),
            "WARN" => Color::Rgb(255, 200, 0),
            _ => Color::Gray,
        };
        let sev_color = match *sev {
            "CRIT" => Color::Rgb(220, 50, 47),
            "HIGH" => Color::Rgb(255, 165, 0),
            "MED" => Color::Rgb(255, 200, 0),
            _ => Color::Rgb(100, 150, 255),
        };
        Row::new(vec![
            Cell::from(*id).style(Style::default().fg(Color::Rgb(222, 115, 86))),
            Cell::from(*fw),
            Cell::from(*desc),
            Cell::from(*status).style(
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from(*sev).style(Style::default().fg(sev_color)),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(14),
        Constraint::Percentage(12),
        Constraint::Percentage(44),
        Constraint::Percentage(12),
        Constraint::Percentage(12),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Compliance Controls ",
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, chunks[2]);

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
            "f",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Filter Framework | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "s",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Run Scan | ", Style::default().fg(Color::Gray)),
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
