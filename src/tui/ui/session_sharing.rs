// Session Sharing Management View

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
    header_spans.push(Span::styled("Session Sharing", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
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
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .split(chunks[1]);

    // Active sessions table
    let header_cells = ["Session ID", "Owner", "Participants", "View", "Duration"]
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

    let sessions = [("sess-a1b2", "admin", "3", "VM List", "1h 22m"),
        ("sess-c3d4", "ops-user", "2", "Dashboard", "45m"),
        ("sess-e5f6", "dev-lead", "1", "Topology", "12m"),
        ("sess-g7h8", "admin", "4", "Migration", "2h 05m")];

    let rows = sessions.iter().map(|(id, owner, parts, view, dur)| {
        Row::new(vec![
            Cell::from(*id).style(Style::default().fg(Color::Rgb(222, 115, 86))),
            Cell::from(*owner),
            Cell::from(*parts),
            Cell::from(*view),
            Cell::from(*dur),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(20),
        Constraint::Percentage(18),
        Constraint::Percentage(18),
        Constraint::Percentage(22),
        Constraint::Percentage(18),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Active Sessions ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, content_chunks[0]);

    // Session details
    let details = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Session: sess-a1b2",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Owner:      ", Style::default().fg(Color::Gray)),
            Span::styled("admin", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Mode:       ", Style::default().fg(Color::Gray)),
            Span::styled("Read-Write", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(vec![
            Span::styled("  Started:    ", Style::default().fg(Color::Gray)),
            Span::styled("1h 22m ago", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Participants",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [RW] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("admin (owner)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [RO] ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled("ops-user", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [RO] ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled("dev-lead", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Share Link",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  vmrogue://share/sess-a1b2", Style::default().fg(Color::Rgb(100, 150, 255))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Expires: ", Style::default().fg(Color::Gray)),
            Span::styled("in 4h 38m", Style::default().fg(Color::White)),
        ]),
    ];
    let details_widget = Paragraph::new(details).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Session Details ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(details_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("n", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": New Session | ", Style::default().fg(Color::Gray)),
        Span::styled("j", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Join | ", Style::default().fg(Color::Gray)),
        Span::styled("c", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Copy Link | ", Style::default().fg(Color::Gray)),
        Span::styled("x", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": End | ", Style::default().fg(Color::Gray)),
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
