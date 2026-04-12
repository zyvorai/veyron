// Change Approval Workflow View

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
    header_spans.push(Span::styled("Change Approval Workflow", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
    let header_text = Line::from(header_spans);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
        );
    f.render_widget(header, chunks[0]);

    // Content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(chunks[1]);

    // Pending changes table
    let header_cells = ["ID", "Type", "Description", "Requester", "Status"]
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

    let changes = [("CR-042", "Scale", "Scale worker pool 3->5", "ops-user", "Pending"),
        ("CR-041", "Config", "Update VM memory limits", "dev-lead", "Approved"),
        ("CR-040", "Delete", "Remove stale snapshots", "ci-bot", "Pending"),
        ("CR-039", "Migrate", "Migrate db-primary to node-3", "ops-user", "Pending"),
        ("CR-038", "Create", "New staging environment", "dev-lead", "Approved"),
        ("CR-037", "Update", "Patch KubeVirt to v1.2.1", "admin", "Rejected"),
        ("CR-036", "Scale", "Scale API gateway replicas", "ops-user", "Executed")];

    let rows = changes.iter().map(|(id, ctype, desc, req, status)| {
        let status_color = match *status {
            "Pending" => Color::Rgb(255, 200, 0),
            "Approved" => Color::Rgb(50, 205, 50),
            "Rejected" => Color::Rgb(220, 50, 47),
            "Executed" => Color::Rgb(100, 150, 255),
            _ => Color::Gray,
        };
        Row::new(vec![
            Cell::from(*id).style(Style::default().fg(Color::Rgb(222, 115, 86))),
            Cell::from(*ctype),
            Cell::from(*desc),
            Cell::from(*req),
            Cell::from(*status).style(Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(12),
        Constraint::Percentage(12),
        Constraint::Percentage(36),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Change Requests ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, content_chunks[0]);

    // Selected change details
    let details = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  CR-042: Scale worker pool",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Type:       ", Style::default().fg(Color::Gray)),
            Span::styled("Scale Operation", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Requester:  ", Style::default().fg(Color::Gray)),
            Span::styled("ops-user", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Created:    ", Style::default().fg(Color::Gray)),
            Span::styled("2h ago", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Priority:   ", Style::default().fg(Color::Gray)),
            Span::styled("Medium", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Impact Analysis",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  CPU:        ", Style::default().fg(Color::Gray)),
            Span::styled("+8 cores requested", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Memory:     ", Style::default().fg(Color::Gray)),
            Span::styled("+16 GiB requested", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Available:  ", Style::default().fg(Color::Gray)),
            Span::styled("Sufficient", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Approvals (1/2 required)",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  [x] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("admin (approved 1h ago)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [ ] ", Style::default().fg(Color::Gray)),
            Span::styled("security-team (pending)", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
    ];
    let details_widget = Paragraph::new(details).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Change Details ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(details_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("a", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Approve | ", Style::default().fg(Color::Gray)),
        Span::styled("r", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Reject | ", Style::default().fg(Color::Gray)),
        Span::styled("x", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Execute | ", Style::default().fg(Color::Gray)),
        Span::styled("n", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": New | ", Style::default().fg(Color::Gray)),
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
