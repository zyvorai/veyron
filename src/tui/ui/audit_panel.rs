// Audit Trail Viewer

use crate::tui::colors::gradient;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
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
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "Audit Trail",
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

    // Audit log table
    let header_cells = [
        "Timestamp",
        "User",
        "Action",
        "Resource",
        "Result",
        "Details",
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

    let entries = vec![
        (
            "14:32:05",
            "admin",
            "CREATE",
            "VM/web-server-03",
            "Success",
            "From template ubuntu-22",
        ),
        (
            "14:28:12",
            "admin",
            "START",
            "VM/web-server-01",
            "Success",
            "Cold boot",
        ),
        (
            "14:15:44",
            "ci-bot",
            "SNAPSHOT",
            "VM/db-primary",
            "Success",
            "Auto backup",
        ),
        (
            "13:58:21",
            "ops-user",
            "MIGRATE",
            "VM/worker-01",
            "Success",
            "node-1 -> node-3",
        ),
        (
            "13:45:03",
            "admin",
            "DELETE",
            "Snapshot/old-snap-12",
            "Success",
            "Cleanup",
        ),
        (
            "13:30:18",
            "dev-user",
            "STOP",
            "VM/test-vm-alpha",
            "Success",
            "Graceful shutdown",
        ),
        (
            "13:12:55",
            "ci-bot",
            "SCALE",
            "VMPool/workers",
            "Success",
            "Replicas 3 -> 5",
        ),
        (
            "12:58:30",
            "ops-user",
            "UPDATE",
            "VM/api-gateway",
            "Failed",
            "Insufficient resources",
        ),
        (
            "12:45:11",
            "admin",
            "RBAC",
            "Role/vm-viewer",
            "Success",
            "Added read permissions",
        ),
        (
            "12:30:02",
            "system",
            "ALERT",
            "Node/node-4",
            "Warning",
            "Unreachable",
        ),
    ];

    let rows = entries
        .iter()
        .map(|(ts, user, action, resource, result, details)| {
            let result_color = match *result {
                "Success" => Color::Rgb(50, 205, 50),
                "Failed" => Color::Rgb(220, 50, 47),
                "Warning" => Color::Rgb(255, 200, 0),
                _ => Color::Gray,
            };
            let action_color = match *action {
                "CREATE" | "START" => Color::Rgb(50, 205, 50),
                "DELETE" | "STOP" => Color::Rgb(220, 50, 47),
                "MIGRATE" | "UPDATE" | "SCALE" => Color::Rgb(255, 200, 0),
                "SNAPSHOT" => Color::Rgb(100, 150, 255),
                _ => Color::Rgb(222, 115, 86),
            };
            Row::new(vec![
                Cell::from(*ts),
                Cell::from(*user).style(Style::default().fg(Color::Rgb(222, 115, 86))),
                Cell::from(*action).style(
                    Style::default()
                        .fg(action_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(*resource),
                Cell::from(*result).style(Style::default().fg(result_color)),
                Cell::from(*details),
            ])
            .height(1)
        });

    let widths = [
        Constraint::Percentage(12),
        Constraint::Percentage(12),
        Constraint::Percentage(12),
        Constraint::Percentage(22),
        Constraint::Percentage(12),
        Constraint::Percentage(26),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Audit Log ",
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, chunks[1]);

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
            "/",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Search | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "f",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Filter | ", Style::default().fg(Color::Gray)),
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
    f.render_widget(help, chunks[2]);
}
