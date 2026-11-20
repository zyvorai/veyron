// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Nodes Table View

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
    let mut header_spans = gradient::brand().text("Veyron");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        format!("Cluster Nodes ({})", state.nodes.len()),
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

    // Table
    let header_cells = ["Name", "Status", "Role", "CPU", "Memory", "Pods", "Age"]
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

    if state.nodes.is_empty() {
        let empty = Paragraph::new("No nodes found. Data will load on next refresh.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                    .title(Span::styled(
                        " Nodes ",
                        Style::default()
                            .fg(Color::Rgb(222, 115, 86))
                            .add_modifier(Modifier::BOLD),
                    )),
            );
        f.render_widget(empty, chunks[1]);
    } else {
        let rows = state.nodes.iter().enumerate().map(|(i, node)| {
            let status_color = match node.status.as_str() {
                "Ready" => Color::Rgb(50, 205, 50),
                "NotReady" => Color::Rgb(220, 50, 47),
                _ => Color::Gray,
            };
            let style = if i == state.node_selected_index {
                Style::default()
                    .bg(Color::Rgb(60, 50, 75))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(node.name.as_str()),
                Cell::from(node.status.as_str()).style(Style::default().fg(status_color)),
                Cell::from(node.role.as_str()),
                Cell::from(node.cpu_capacity.as_str()),
                Cell::from(node.memory_capacity.as_str()),
                Cell::from(node.pod_count.as_str()),
                Cell::from(node.age.as_str()),
            ])
            .style(style)
            .height(1)
        });

        let widths = [
            Constraint::Percentage(20),
            Constraint::Percentage(12),
            Constraint::Percentage(15),
            Constraint::Percentage(12),
            Constraint::Percentage(15),
            Constraint::Percentage(12),
            Constraint::Percentage(14),
        ];

        let table = Table::new(rows, widths)
            .header(table_header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                    .title(Span::styled(
                        " Nodes ",
                        Style::default()
                            .fg(Color::Rgb(222, 115, 86))
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .column_spacing(1);
        f.render_widget(table, chunks[1]);
    }

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
            "Esc",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Back | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Ctrl+R",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Refresh", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
    );
    f.render_widget(help, chunks[2]);
}
