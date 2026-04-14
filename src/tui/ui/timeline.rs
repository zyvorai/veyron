// Event Timeline Visualization View

use crate::tui::colors::gradient;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

    // Gradient brand header
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "Event Timeline",
        Style::default()
            .fg(Color::Rgb(255, 145, 115))
            .add_modifier(Modifier::BOLD),
    ));
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "Last 24 hours",
        Style::default().fg(Color::Gray),
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

    // Timeline content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[1]);

    let timeline_events = vec![
        Line::from(""),
        // Time axis header
        Line::from(Span::styled(
            "  Time          Event",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  ----          -----",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  14:32  ", Style::default().fg(Color::Gray)),
            Span::styled("*---", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "  VM web-server-03 created and started",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("|", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  14:15  ", Style::default().fg(Color::Gray)),
            Span::styled("*---", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled(
                "  Auto-snapshot db-primary (scheduled)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("|", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  13:58  ", Style::default().fg(Color::Gray)),
            Span::styled("*---", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                "  Live migration: worker-01 (node-1 -> node-3)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("|", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  13:30  ", Style::default().fg(Color::Gray)),
            Span::styled("*---", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                "  ALERT: node-4 became unreachable",
                Style::default().fg(Color::Rgb(220, 50, 47)),
            ),
        ]),
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("|", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  12:58  ", Style::default().fg(Color::Gray)),
            Span::styled("*---", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                "  VM api-gateway update failed (resources)",
                Style::default().fg(Color::Rgb(255, 200, 0)),
            ),
        ]),
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("|", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  12:45  ", Style::default().fg(Color::Gray)),
            Span::styled("*---", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled(
                "  RBAC: vm-viewer role updated",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("|", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  11:20  ", Style::default().fg(Color::Gray)),
            Span::styled("*---", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "  Autoscaler: worker pool scaled 3 -> 5",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("|", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  09:00  ", Style::default().fg(Color::Gray)),
            Span::styled("*---", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled(
                "  Scheduled backup completed (all VMs)",
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    let timeline_widget = Paragraph::new(timeline_events).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Timeline ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(timeline_widget, content_chunks[0]);

    // Event details and legend
    let details = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Selected Event",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Time:     ", Style::default().fg(Color::Gray)),
            Span::styled("13:30:18", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Type:     ", Style::default().fg(Color::Gray)),
            Span::styled(
                "ALERT",
                Style::default()
                    .fg(Color::Rgb(220, 50, 47))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Resource: ", Style::default().fg(Color::Gray)),
            Span::styled("Node/node-4", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Actor:    ", Style::default().fg(Color::Gray)),
            Span::styled("system", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Message:  ", Style::default().fg(Color::Gray)),
            Span::styled("Node became unreachable", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Legend",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  * ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("Create / Start / Scale", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  * ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled("Snapshot / Backup", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  * ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("Migration / Update", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  * ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled("Alert / Error", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  * ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Config / RBAC", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Events today: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "24",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let details_widget = Paragraph::new(details).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Details ",
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
            "Enter",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Details | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "f",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Filter | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "p",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Period | ", Style::default().fg(Color::Gray)),
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
