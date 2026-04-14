// Events Table View

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
        format!("Cluster Events ({})", state.events.len()),
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
    let header_cells = ["Time", "Type", "Reason", "Object", "Message"]
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

    if state.events.is_empty() {
        let empty = Paragraph::new("No events found. Data will load on next refresh.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                    .title(Span::styled(
                        " Events ",
                        Style::default()
                            .fg(Color::Rgb(222, 115, 86))
                            .add_modifier(Modifier::BOLD),
                    )),
            );
        f.render_widget(empty, chunks[1]);
    } else {
        let rows = state.events.iter().enumerate().map(|(i, event)| {
            let type_color = match event.event_type.as_str() {
                "Normal" => Color::Rgb(50, 205, 50),
                "Warning" => Color::Rgb(255, 200, 0),
                _ => Color::Gray,
            };
            let style = if i == state.event_selected_index {
                Style::default()
                    .bg(Color::Rgb(60, 50, 75))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(event.time.as_str()),
                Cell::from(event.event_type.as_str()).style(Style::default().fg(type_color)),
                Cell::from(event.reason.as_str()),
                Cell::from(event.object.as_str()),
                Cell::from(event.message.as_str()),
            ])
            .style(style)
            .height(1)
        });

        let widths = [
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(16),
            Constraint::Percentage(26),
            Constraint::Percentage(38),
        ];

        let table = Table::new(rows, widths)
            .header(table_header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                    .title(Span::styled(
                        " Events ",
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
