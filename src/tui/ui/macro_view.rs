// Macro Recording and Playback View

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
    header_spans.push(Span::styled("Macro Manager", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
    let header_text = Line::from(header_spans);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
        );
    f.render_widget(header, chunks[0]);

    // Main content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .split(chunks[1]);

    // Saved macros table
    let header_cells = ["Name", "Steps", "Last Run", "Status"]
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

    let macros = [("deploy-web-stack", "8", "2h ago", "Ready"),
        ("scale-workers", "4", "1d ago", "Ready"),
        ("backup-all-vms", "12", "6h ago", "Ready"),
        ("restart-services", "6", "3d ago", "Ready"),
        ("health-check-all", "10", "30m ago", "Running"),
        ("cleanup-snapshots", "5", "7d ago", "Ready")];

    let rows = macros.iter().map(|(name, steps, last_run, status)| {
        let status_color = match *status {
            "Ready" => Color::Rgb(50, 205, 50),
            "Running" => Color::Rgb(255, 200, 0),
            "Failed" => Color::Rgb(220, 50, 47),
            _ => Color::Gray,
        };
        Row::new(vec![
            Cell::from(*name).style(Style::default().fg(Color::White)),
            Cell::from(*steps),
            Cell::from(*last_run),
            Cell::from(*status).style(Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(35),
        Constraint::Percentage(15),
        Constraint::Percentage(22),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Saved Macros ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, content_chunks[0]);

    // Macro details / recording
    let detail_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Selected: deploy-web-stack",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("  Steps:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  1. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Create VM from template 'web-ubuntu'", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Wait for VM ready (timeout: 120s)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Apply network policy 'web-allow'", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  4. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Configure load balancer service", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  5. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Run health check", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  6. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Create initial snapshot", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  7. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Enable monitoring", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  8. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Send notification on completion", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Avg Duration: ", Style::default().fg(Color::Gray)),
            Span::styled("3m 42s", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Success Rate: ", Style::default().fg(Color::Gray)),
            Span::styled("96%", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
    ];
    let detail_widget = Paragraph::new(detail_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Macro Details ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(detail_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("Enter", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Run | ", Style::default().fg(Color::Gray)),
        Span::styled("n", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": New | ", Style::default().fg(Color::Gray)),
        Span::styled("r", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Record | ", Style::default().fg(Color::Gray)),
        Span::styled("e", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Edit | ", Style::default().fg(Color::Gray)),
        Span::styled("d", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Delete | ", Style::default().fg(Color::Gray)),
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
