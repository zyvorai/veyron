// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Virtual Machine Instance Table View

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
        format!("VM Instances ({})", state.vmis.len()),
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
    let header_cells = ["Name", "Phase", "Node", "IP"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )
    });
    let table_header = Row::new(header_cells)
        .style(Style::default().bg(Color::Rgb(40, 35, 55)))
        .height(1);

    if state.vmis.is_empty() {
        let empty = Paragraph::new("No VMIs found. Data will load on next refresh.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                    .title(Span::styled(
                        " VM Instances ",
                        Style::default()
                            .fg(Color::Rgb(222, 115, 86))
                            .add_modifier(Modifier::BOLD),
                    )),
            );
        f.render_widget(empty, chunks[1]);
    } else {
        let rows = state.vmis.iter().enumerate().map(|(i, vmi)| {
            let phase_color = match vmi.phase.as_str() {
                "Running" => Color::Rgb(50, 205, 50),
                "Scheduling" | "Scheduled" => Color::Rgb(255, 200, 0),
                "Failed" => Color::Rgb(220, 50, 47),
                "Succeeded" => Color::Rgb(100, 150, 255),
                _ => Color::Gray,
            };
            let style = if i == state.vmi_selected_index {
                Style::default()
                    .bg(Color::Rgb(60, 50, 75))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(vmi.name.as_str()),
                Cell::from(vmi.phase.as_str()).style(Style::default().fg(phase_color)),
                Cell::from(vmi.node.as_str()),
                Cell::from(vmi.ip.as_str()),
            ])
            .style(style)
            .height(1)
        });

        let widths = [
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ];

        let table = Table::new(rows, widths)
            .header(table_header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                    .title(Span::styled(
                        " VM Instances ",
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
