// Pods Table View

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
    header_spans.push(Span::styled("Pods", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
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
    let header_cells = ["Name", "Namespace", "Status", "Node", "Age"]
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

    let sample_data = [("virt-launcher-web-01-abc12", "default", "Running", "node-1", "2d"),
        ("virt-launcher-db-pri-def34", "production", "Running", "node-2", "5d"),
        ("virt-launcher-wrk-03-ghi56", "staging", "Pending", "node-1", "10m"),
        ("virt-handler-jkl78", "kubevirt", "Running", "node-3", "14d"),
        ("virt-api-mno90", "kubevirt", "Running", "node-1", "14d"),
        ("cdi-operator-pqr12", "cdi", "Running", "node-2", "14d")];

    let rows = sample_data.iter().map(|(name, ns, status, node, age)| {
        let status_color = match *status {
            "Running" => Color::Rgb(50, 205, 50),
            "Pending" => Color::Rgb(255, 200, 0),
            "Failed" | "CrashLoopBackOff" => Color::Rgb(220, 50, 47),
            _ => Color::Gray,
        };
        Row::new(vec![
            Cell::from(*name),
            Cell::from(*ns),
            Cell::from(*status).style(Style::default().fg(status_color)),
            Cell::from(*node),
            Cell::from(*age),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(35),
        Constraint::Percentage(18),
        Constraint::Percentage(15),
        Constraint::Percentage(18),
        Constraint::Percentage(14),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Pods ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("l", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Logs | ", Style::default().fg(Color::Gray)),
        Span::styled("d", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Describe | ", Style::default().fg(Color::Gray)),
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
