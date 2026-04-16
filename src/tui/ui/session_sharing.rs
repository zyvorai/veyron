// Session Sharing Management View

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
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "Session Sharing",
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

    // Content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    // VMs available for sharing
    let header_cells = ["VM Name", "Status", "CPU", "Memory", "Node"]
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

    let rows: Vec<Row> = state.vms.iter().map(|vm| {
        let status_color = match vm.status.as_str() {
            "Running" => Color::Rgb(50, 205, 50),
            "Stopped" => Color::Gray,
            "Failed" | "Error" => Color::Rgb(220, 50, 47),
            "Starting" | "Pending" => Color::Rgb(255, 200, 0),
            _ => Color::Gray,
        };
        Row::new(vec![
            Cell::from(vm.name.as_str()).style(Style::default().fg(Color::Rgb(222, 115, 86))),
            Cell::from(vm.status.as_str()).style(Style::default().fg(status_color)),
            Cell::from(vm.cpu.as_str()),
            Cell::from(vm.memory.as_str()),
            Cell::from(vm.node.as_str()),
        ])
        .height(1)
    }).collect();

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(18),
        Constraint::Percentage(18),
        Constraint::Percentage(18),
        Constraint::Percentage(18),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    format!(" VMs Available for Sharing ({}) ", state.vms.len()),
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, content_chunks[0]);

    // Session details - show selected VM info
    let mut details = vec![
        Line::from(""),
    ];

    if let Some(vm) = state.vms.first() {
        let status_color = match vm.status.as_str() {
            "Running" => Color::Rgb(50, 205, 50),
            "Stopped" => Color::Gray,
            "Failed" | "Error" => Color::Rgb(220, 50, 47),
            _ => Color::Rgb(255, 200, 0),
        };

        details.push(Line::from(Span::styled(
            format!("  VM: {}", vm.name),
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )));
        details.push(Line::from(""));
        details.push(Line::from(vec![
            Span::styled("  Status:     ", Style::default().fg(Color::Gray)),
            Span::styled(vm.status.as_str(), Style::default().fg(status_color)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  CPU:        ", Style::default().fg(Color::Gray)),
            Span::styled(vm.cpu.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Memory:     ", Style::default().fg(Color::Gray)),
            Span::styled(vm.memory.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Node:       ", Style::default().fg(Color::Gray)),
            Span::styled(vm.node.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  IP:         ", Style::default().fg(Color::Gray)),
            Span::styled(vm.ip.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Age:        ", Style::default().fg(Color::Gray)),
            Span::styled(vm.age.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Ready:      ", Style::default().fg(Color::Gray)),
            Span::styled(
                if vm.ready { "Yes" } else { "No" },
                Style::default().fg(if vm.ready { Color::Rgb(50, 205, 50) } else { Color::Rgb(255, 200, 0) }),
            ),
        ]));
        details.push(Line::from(""));
        details.push(Line::from(Span::styled(
            "  Sharing Options",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )));
        details.push(Line::from(""));
        details.push(Line::from(vec![
            Span::styled("  Mode:       ", Style::default().fg(Color::Gray)),
            Span::styled(
                if vm.status == "Running" { "Read-Write" } else { "Read-Only" },
                Style::default().fg(if vm.status == "Running" { Color::Rgb(50, 205, 50) } else { Color::Rgb(100, 150, 255) }),
            ),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Namespace:  ", Style::default().fg(Color::Gray)),
            Span::styled(state.namespace.as_str(), Style::default().fg(Color::White)),
        ]));
    } else {
        details.push(Line::from(Span::styled(
            "  No VMs available",
            Style::default().fg(Color::Gray),
        )));
    }

    let details_widget = Paragraph::new(details).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " VM Details ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(details_widget, content_chunks[1]);

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
            "n",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": New Session | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "j",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Join | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "c",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Copy Link | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "x",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": End | ", Style::default().fg(Color::Gray)),
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
