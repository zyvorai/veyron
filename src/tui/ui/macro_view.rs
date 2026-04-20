// Macro Recording and Playback View

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
        "Macro Manager",
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

    // Main content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    // Recent actions as potential macro steps
    let header_cells = ["Step", "VM", "Action", "Elapsed"].iter().map(|h| {
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
            Row::new(vec![
                Cell::from(format!("{}", i + 1))
                    .style(Style::default().fg(Color::Rgb(222, 115, 86))),
                Cell::from(event.vm_name.as_str()).style(Style::default().fg(Color::White)),
                Cell::from(event.action.as_str()),
                Cell::from(elapsed),
            ])
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Percentage(10),
        Constraint::Percentage(35),
        Constraint::Percentage(30),
        Constraint::Percentage(22),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    format!(" Recorded Actions ({}) ", state.recent_activity.len()),
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, content_chunks[0]);

    // Macro details / summary
    let mut detail_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Macro Summary",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if state.recent_activity.is_empty() {
        detail_lines.push(Line::from(Span::styled(
            "  No actions recorded yet.",
            Style::default().fg(Color::Gray),
        )));
        detail_lines.push(Line::from(Span::styled(
            "  Perform VM operations to record steps.",
            Style::default().fg(Color::Gray),
        )));
    } else {
        detail_lines.push(Line::from(Span::styled(
            "  Recorded Steps:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));

        for (i, event) in state.recent_activity.iter().rev().take(8).enumerate() {
            detail_lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}. ", i + 1),
                    Style::default().fg(Color::Rgb(222, 115, 86)),
                ),
                Span::styled(
                    format!("{} {}", event.action, event.vm_name),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        if state.recent_activity.len() > 8 {
            detail_lines.push(Line::from(Span::styled(
                format!("  ... and {} more", state.recent_activity.len() - 8),
                Style::default().fg(Color::Gray),
            )));
        }

        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(vec![
            Span::styled("  Total Steps:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", state.recent_activity.len()),
                Style::default().fg(Color::White),
            ),
        ]));

        // Show distinct VMs involved
        let mut unique_vms: Vec<&str> = state
            .recent_activity
            .iter()
            .map(|e| e.vm_name.as_str())
            .collect();
        unique_vms.sort();
        unique_vms.dedup();
        detail_lines.push(Line::from(vec![
            Span::styled("  VMs Involved: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", unique_vms.len()),
                Style::default().fg(Color::White),
            ),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("  Status:       ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Ready",
                Style::default()
                    .fg(Color::Rgb(50, 205, 50))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    let detail_widget = Paragraph::new(detail_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Macro Details ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(detail_widget, content_chunks[1]);

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
            "Enter",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Run | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "n",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": New | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Record | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "e",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Edit | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "d",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Delete | ", Style::default().fg(Color::Gray)),
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
