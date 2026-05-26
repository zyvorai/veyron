// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Change Approval Workflow View

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
        "Change Approval Workflow",
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
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    // Build change requests from recent_activity
    let header_cells = ["#", "Action", "VM", "Elapsed", "Status"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )
    });
    let table_header = Row::new(header_cells)
        .style(Style::default().bg(Color::Rgb(40, 35, 55)))
        .height(1);

    let rows: Vec<Row> = state
        .recent_activity
        .iter()
        .rev()
        .enumerate()
        .map(|(i, event)| {
            let elapsed = event.elapsed_display();
            let status = "Pending";
            let status_color = Color::Rgb(255, 200, 0);
            Row::new(vec![
                Cell::from(format!("CR-{:03}", i + 1))
                    .style(Style::default().fg(Color::Rgb(222, 115, 86))),
                Cell::from(event.action.as_str()),
                Cell::from(event.vm_name.as_str()),
                Cell::from(elapsed),
                Cell::from(status).style(
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Percentage(12),
        Constraint::Percentage(20),
        Constraint::Percentage(28),
        Constraint::Percentage(18),
        Constraint::Percentage(16),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    format!(" Recent Changes ({}) ", state.recent_activity.len()),
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, content_chunks[0]);

    // Details panel showing the first (most recent) activity or a summary
    let mut details = vec![Line::from("")];

    if let Some(latest) = state.recent_activity.last() {
        details.push(Line::from(Span::styled(
            format!("  CR-001: {} {}", latest.action, latest.vm_name),
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )));
        details.push(Line::from(""));
        details.push(Line::from(vec![
            Span::styled("  Action:     ", Style::default().fg(Color::Gray)),
            Span::styled(latest.action.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  VM:         ", Style::default().fg(Color::Gray)),
            Span::styled(latest.vm_name.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Elapsed:    ", Style::default().fg(Color::Gray)),
            Span::styled(latest.elapsed_display(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Icon:       ", Style::default().fg(Color::Gray)),
            Span::styled(latest.icon.as_str(), Style::default().fg(Color::White)),
        ]));
    } else {
        details.push(Line::from(Span::styled(
            "  No recent activity",
            Style::default().fg(Color::Gray),
        )));
    }

    details.push(Line::from(""));
    details.push(Line::from(Span::styled(
        "  Summary",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    details.push(Line::from(""));

    let stats = state.get_stats();
    details.push(Line::from(vec![
        Span::styled("  Total VMs:  ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.total),
            Style::default().fg(Color::White),
        ),
    ]));
    details.push(Line::from(vec![
        Span::styled("  Running:    ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.running),
            Style::default().fg(Color::Rgb(50, 205, 50)),
        ),
    ]));
    details.push(Line::from(vec![
        Span::styled("  Changes:    ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", state.recent_activity.len()),
            Style::default().fg(Color::White),
        ),
    ]));

    let details_widget = Paragraph::new(details).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Change Details ",
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
            "a",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Approve | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Reject | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "x",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Execute | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "n",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": New | ", Style::default().fg(Color::Gray)),
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
