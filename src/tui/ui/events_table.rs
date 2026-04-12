// Events Table View

use crate::tui::colors::gradient;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};


pub fn render(f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    // Gradient brand header
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(" | ", Style::default().fg(Color::Rgb(128, 128, 128))));
    header_spans.push(Span::styled("Cluster Events", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
    let header_text = Line::from(header_spans);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
        );
    f.render_widget(header, chunks[0]);

    // Table
    let header_cells = ["Time", "Type", "Reason", "Object", "Message"]
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

    let sample_data = [("2m ago", "Normal", "Started", "Pod/virt-launcher-web-01", "Started container compute"),
        ("3m ago", "Normal", "Created", "Pod/virt-launcher-web-01", "Created container compute"),
        ("5m ago", "Warning", "FailedMount", "Pod/virt-launcher-db-01", "Unable to attach volume"),
        ("8m ago", "Normal", "Scheduled", "Pod/virt-launcher-wrk-03", "Assigned to node-2"),
        ("12m ago", "Warning", "Unhealthy", "Pod/virt-launcher-cache", "Readiness probe failed"),
        ("15m ago", "Normal", "SuccessfulCreate", "VMI/web-server-01", "Created virtual machine instance")];

    let rows = sample_data.iter().map(|(time, etype, reason, obj, msg)| {
        let type_color = match *etype {
            "Normal" => Color::Rgb(50, 205, 50),
            "Warning" => Color::Rgb(255, 200, 0),
            _ => Color::Gray,
        };
        Row::new(vec![
            Cell::from(*time),
            Cell::from(*etype).style(Style::default().fg(type_color)),
            Cell::from(*reason),
            Cell::from(*obj),
            Cell::from(*msg),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(10),
        Constraint::Percentage(10),
        Constraint::Percentage(16),
        Constraint::Percentage(26),
        Constraint::Percentage(38),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Events ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("f", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Filter | ", Style::default().fg(Color::Gray)),
        Span::styled("r", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Refresh | ", Style::default().fg(Color::Gray)),
        Span::styled("q", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Back", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
    );
    f.render_widget(help, chunks[2]);
}
