// Nodes Table View

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
            Constraint::Min(0),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    // Gradient brand header
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(" | ", Style::default().fg(Color::Rgb(128, 128, 128))));
    header_spans.push(Span::styled("Cluster Nodes", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
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
    let header_cells = ["Name", "Status", "Role", "CPU", "Memory", "Pods"]
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

    let sample_data = [("node-1", "Ready", "control-plane", "45%", "62%", "24/110"),
        ("node-2", "Ready", "worker", "72%", "58%", "31/110"),
        ("node-3", "Ready", "worker", "38%", "41%", "18/110"),
        ("node-4", "NotReady", "worker", "—", "—", "0/110")];

    let rows = sample_data.iter().map(|(name, status, role, cpu, mem, pods)| {
        let status_color = match *status {
            "Ready" => Color::Rgb(50, 205, 50),
            "NotReady" => Color::Rgb(220, 50, 47),
            _ => Color::Gray,
        };
        Row::new(vec![
            Cell::from(*name),
            Cell::from(*status).style(Style::default().fg(status_color)),
            Cell::from(*role),
            Cell::from(*cpu),
            Cell::from(*mem),
            Cell::from(*pods),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(20),
        Constraint::Percentage(14),
        Constraint::Percentage(18),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Nodes ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, chunks[1]);

    // Cluster resource gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[2]);

    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Cluster CPU "),
        )
        .gauge_style(Style::default().fg(gradient::health().at(0.52)).bg(Color::Rgb(40, 35, 55)))
        .percent(52)
        .label("52% (20.8 / 40 cores)");
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Cluster Memory "),
        )
        .gauge_style(Style::default().fg(gradient::health().at(0.54)).bg(Color::Rgb(40, 35, 55)))
        .percent(54)
        .label("54% (86 / 160 GiB)");
    f.render_widget(mem_gauge, gauge_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("Enter", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Details | ", Style::default().fg(Color::Gray)),
        Span::styled("c", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Cordon | ", Style::default().fg(Color::Gray)),
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
