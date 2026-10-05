// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Event Timeline Visualization View

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

    // Gradient brand header
    let mut header_spans = gradient::brand().text("Veyron");
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
        format!("{} events", state.events.len()),
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

    // --- Build timeline from state.events ---
    let mut timeline_events: Vec<Line> = vec![
        Line::from(""),
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
    ];

    if state.events.is_empty() {
        timeline_events.push(Line::from(Span::styled(
            "  No events recorded yet",
            Style::default().fg(Color::Gray),
        )));
    } else {
        // Show up to 15 most recent events as timeline entries
        for event in state.events.iter().take(15) {
            let (dot_color, text_color) = match event.event_type.as_str() {
                "Warning" => (Color::Rgb(220, 50, 47), Color::Rgb(255, 200, 0)),
                _ => match event.reason.as_str() {
                    "Created" | "Started" | "SuccessfulCreate" => {
                        (Color::Rgb(50, 205, 50), Color::White)
                    }
                    "Killing" | "Deleted" | "FailedCreate" => {
                        (Color::Rgb(220, 50, 47), Color::Rgb(220, 50, 47))
                    }
                    "Scheduled" | "Pulled" | "Pulling" => (Color::Rgb(100, 150, 255), Color::White),
                    _ => (Color::Rgb(222, 115, 86), Color::White),
                },
            };

            let time_display = format!("  {:<8} ", event.time);
            let message_display = if event.message.len() > 45 {
                format!("{}...", &event.message[..42])
            } else {
                event.message.clone()
            };

            timeline_events.push(Line::from(vec![
                Span::styled(time_display, Style::default().fg(Color::Gray)),
                Span::styled("*---", Style::default().fg(dot_color)),
                Span::styled(
                    format!("  [{}] {}", event.object, message_display),
                    Style::default().fg(text_color),
                ),
            ]));
            // Connector line between events
            timeline_events.push(Line::from(vec![
                Span::styled("         ", Style::default()),
                Span::styled("|", Style::default().fg(Color::Gray)),
            ]));
        }
    }

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

    // Event details panel - show selected/first event details and legend
    let first_event = state.events.first();
    let mut details: Vec<Line> = Vec::new();

    details.push(Line::from(""));
    details.push(Line::from(Span::styled(
        "  Selected Event",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    details.push(Line::from(""));

    if let Some(event) = first_event {
        details.push(Line::from(vec![
            Span::styled("  Time:     ", Style::default().fg(Color::Gray)),
            Span::styled(event.time.as_str(), Style::default().fg(Color::White)),
        ]));
        let type_color = if event.event_type == "Warning" {
            Color::Rgb(220, 50, 47)
        } else {
            Color::Rgb(50, 205, 50)
        };
        details.push(Line::from(vec![
            Span::styled("  Type:     ", Style::default().fg(Color::Gray)),
            Span::styled(
                event.event_type.as_str(),
                Style::default().fg(type_color).add_modifier(Modifier::BOLD),
            ),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Resource: ", Style::default().fg(Color::Gray)),
            Span::styled(event.object.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Reason:   ", Style::default().fg(Color::Gray)),
            Span::styled(event.reason.as_str(), Style::default().fg(Color::White)),
        ]));
        let msg_display = if event.message.len() > 30 {
            format!("{}...", &event.message[..27])
        } else {
            event.message.clone()
        };
        details.push(Line::from(vec![
            Span::styled("  Message:  ", Style::default().fg(Color::Gray)),
            Span::styled(msg_display, Style::default().fg(Color::White)),
        ]));
    } else {
        details.push(Line::from(Span::styled(
            "  No events available",
            Style::default().fg(Color::Gray),
        )));
    }

    details.push(Line::from(""));
    details.push(Line::from(Span::styled(
        "  Legend",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    details.push(Line::from(""));
    details.push(Line::from(vec![
        Span::styled("  * ", Style::default().fg(Color::Rgb(50, 205, 50))),
        Span::styled("Create / Start / Scale", Style::default().fg(Color::White)),
    ]));
    details.push(Line::from(vec![
        Span::styled("  * ", Style::default().fg(Color::Rgb(100, 150, 255))),
        Span::styled("Scheduled / Pulled", Style::default().fg(Color::White)),
    ]));
    details.push(Line::from(vec![
        Span::styled("  * ", Style::default().fg(Color::Rgb(255, 200, 0))),
        Span::styled("Warning (text)", Style::default().fg(Color::White)),
    ]));
    details.push(Line::from(vec![
        Span::styled("  * ", Style::default().fg(Color::Rgb(220, 50, 47))),
        Span::styled("Error / Delete / Kill", Style::default().fg(Color::White)),
    ]));
    details.push(Line::from(vec![
        Span::styled("  * ", Style::default().fg(Color::Rgb(222, 115, 86))),
        Span::styled("Other", Style::default().fg(Color::White)),
    ]));
    details.push(Line::from(""));
    details.push(Line::from(vec![
        Span::styled("  Total events: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", state.events.len()),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

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
