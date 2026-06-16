// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Snapshots View - List and manage VM snapshots

use crate::tui::colors::tui as colors;
use crate::tui::{config::TuiConfig, state::AppState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, state: &AppState, _config: &TuiConfig) {
    let size = f.area();

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Snapshot table
            Constraint::Length(4), // Help text
        ])
        .split(size);

    // Header
    let header_text = Line::from(vec![
        Span::styled(
            "Veyron",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" - ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("KubeVirt VM Manager", Style::default().fg(colors::TEXT)),
        Span::styled("  │  ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "📸 Snapshots",
            Style::default()
                .fg(colors::LIGHT_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(": {}", state.snapshots.len()),
            Style::default().fg(colors::TEXT),
        ),
    ]);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );
    f.render_widget(header, chunks[0]);

    // Snapshot table
    render_snapshot_table(f, state, chunks[1]);

    // Help text
    render_help(f, chunks[2]);
}

fn render_snapshot_table(f: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    if state.snapshots.is_empty() {
        let text = Paragraph::new("No snapshots found")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::BORDER))
                    .title("Snapshots"),
            );
        f.render_widget(text, area);
        return;
    }

    let header_cells = ["Name", "VM", "Status", "Age"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells)
        .style(Style::default().bg(colors::DARK_ORANGE))
        .height(1);

    let rows = state.snapshots.iter().enumerate().map(|(i, snapshot)| {
        let is_selected = i == state.snapshot_selected_index;

        let status_color = if snapshot.ready {
            colors::SUCCESS
        } else {
            colors::WARNING
        };

        let cells = vec![
            Cell::from(snapshot.name.clone()),
            Cell::from(snapshot.vm_name.clone()),
            Cell::from(snapshot.status.clone()).style(Style::default().fg(status_color)),
            Cell::from(snapshot.age.clone()),
        ];

        let style = if is_selected {
            Style::default()
                .bg(colors::DARK_ORANGE)
                .fg(colors::TEXT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        Row::new(cells).style(style).height(1)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30), // Name
            Constraint::Percentage(30), // VM
            Constraint::Percentage(20), // Status
            Constraint::Percentage(20), // Age
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER))
            .title("Snapshots"),
    )
    .column_spacing(1);

    f.render_widget(table, area);
}

fn render_help(f: &mut Frame, area: ratatui::layout::Rect) {
    let help_text = vec![
        Line::from(""),
        Line::from("↑↓/jk: Navigate | c: Create | d: Delete | r: Restore | q: Quit | ?: Help"),
    ];

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(colors::TEXT_MUTED))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );

    f.render_widget(help, area);
}
