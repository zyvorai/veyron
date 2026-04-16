// Audit Trail Viewer

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
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

    // --- Build audit entries from state.events and state.recent_activity ---

    // Collect entries: combine K8s events + recent activity into a unified audit log
    struct AuditEntry {
        timestamp: String,
        user: String,
        action: String,
        resource: String,
        result: String,
        details: String,
    }

    let mut entries: Vec<AuditEntry> = Vec::new();

    // Add recent_activity entries (newest last in the vec, we reverse for display)
    for activity in state.recent_activity.iter().rev() {
        let action = activity.action.to_uppercase();
        let result_str = if activity.action.contains("fail") {
            "Failed"
        } else {
            "Success"
        };
        entries.push(AuditEntry {
            timestamp: activity.elapsed_display(),
            user: "system".to_string(),
            action,
            resource: format!("VM/{}", activity.vm_name),
            result: result_str.to_string(),
            details: format!("{} {}", activity.icon, activity.action),
        });
    }

    // Add K8s events as audit entries
    for event in &state.events {
        let action = event.reason.to_uppercase();
        let result_str = match event.event_type.as_str() {
            "Warning" => "Warning",
            "Normal" => "Success",
            _ => "Info",
        };
        entries.push(AuditEntry {
            timestamp: event.time.clone(),
            user: "k8s".to_string(),
            action,
            resource: event.object.clone(),
            result: result_str.to_string(),
            details: if event.message.len() > 50 {
                format!("{}...", &event.message[..47])
            } else {
                event.message.clone()
            },
        });
    }

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

    let rows: Vec<Row> = entries
        .iter()
        .map(|entry| {
            let result_color = match entry.result.as_str() {
                "Success" => Color::Rgb(50, 205, 50),
                "Failed" => Color::Rgb(220, 50, 47),
                "Warning" => Color::Rgb(255, 200, 0),
                _ => Color::Gray,
            };
            let action_color = match entry.action.as_str() {
                "CREATED" | "STARTED" | "DISCOVERED" => Color::Rgb(50, 205, 50),
                "DELETED" | "STOPPED" | "REMOVED" | "KILLING" => Color::Rgb(220, 50, 47),
                "MIGRATED" | "UPDATED" | "SCALED" | "SCHEDULED" | "PULLED" => {
                    Color::Rgb(255, 200, 0)
                }
                "SNAPSHOT" | "SUCCESSFULCREATE" => Color::Rgb(100, 150, 255),
                _ => Color::Rgb(222, 115, 86),
            };
            Row::new(vec![
                Cell::from(entry.timestamp.as_str()),
                Cell::from(entry.user.as_str())
                    .style(Style::default().fg(Color::Rgb(222, 115, 86))),
                Cell::from(entry.action.as_str()).style(
                    Style::default()
                        .fg(action_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(entry.resource.as_str()),
                Cell::from(entry.result.as_str()).style(Style::default().fg(result_color)),
                Cell::from(entry.details.as_str()),
            ])
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Percentage(12),
        Constraint::Percentage(12),
        Constraint::Percentage(12),
        Constraint::Percentage(22),
        Constraint::Percentage(12),
        Constraint::Percentage(26),
    ];

    let entry_count = entries.len();
    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    format!(" Audit Log ({} entries) ", entry_count),
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
