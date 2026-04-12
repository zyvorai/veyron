// Virtual Machine Instance Table View

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
    header_spans.push(Span::styled("VM Instances", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
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
    let header_cells = ["Name", "Namespace", "Phase", "Pod", "IPs"]
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

    let sample_data = [("web-server-01", "default", "Running", "virt-launcher-web-01-abc12", "10.244.1.5"),
        ("db-primary", "production", "Running", "virt-launcher-db-pri-def34", "10.244.2.10"),
        ("worker-node-03", "staging", "Scheduling", "virt-launcher-wrk-03-ghi56", "—"),
        ("test-vm-alpha", "development", "Succeeded", "virt-launcher-test-a-jkl78", "10.244.3.22"),
        ("cache-server", "default", "Failed", "virt-launcher-cache-mno90", "10.244.1.8")];

    let rows = sample_data.iter().map(|(name, ns, phase, pod, ips)| {
        let phase_color = match *phase {
            "Running" => Color::Rgb(50, 205, 50),
            "Scheduling" => Color::Rgb(255, 200, 0),
            "Failed" => Color::Rgb(220, 50, 47),
            "Succeeded" => Color::Rgb(100, 150, 255),
            _ => Color::Gray,
        };
        Row::new(vec![
            Cell::from(*name),
            Cell::from(*ns),
            Cell::from(*phase).style(Style::default().fg(phase_color)),
            Cell::from(*pod),
            Cell::from(*ips),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(18),
        Constraint::Percentage(14),
        Constraint::Percentage(12),
        Constraint::Percentage(36),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " VM Instances ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("Enter", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Details | ", Style::default().fg(Color::Gray)),
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
