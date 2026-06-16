// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Activity Log View - Full-screen scrollable activity event log

use crate::tui::colors::tui as colors;
use crate::tui::{config::TuiConfig, state::AppState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, state: &AppState, _config: &TuiConfig) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Activity list
            Constraint::Length(3), // Help
        ])
        .split(size);

    // Header
    let scroll_info = if state.recent_activity.len() > 1 {
        format!(
            " [{}/{}]",
            state.activity_scroll_offset + 1,
            state.recent_activity.len()
        )
    } else {
        String::new()
    };

    let header_text = Line::from(vec![
        Span::styled(
            "Veyron",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" | ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            format!("Activity Log: {} events", state.recent_activity.len()),
            Style::default()
                .fg(colors::LIGHT_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(scroll_info, Style::default().fg(colors::TEXT_MUTED)),
    ]);

    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );
    f.render_widget(header, chunks[0]);

    // Activity list
    if state.recent_activity.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from(vec![Span::styled(
                "No activity recorded yet",
                Style::default()
                    .fg(colors::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Activity events will appear here as VMs are started, stopped, or modified",
                Style::default().fg(colors::TEXT_MUTED),
            )]),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title(Span::styled(
                    " Activity Log ",
                    Style::default()
                        .fg(colors::ORANGE)
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = state
            .recent_activity
            .iter()
            .enumerate()
            .skip(state.activity_scroll_offset)
            .map(|(i, event)| {
                let icon_color = match event.action.as_str() {
                    "started" | "start requested" => colors::SUCCESS,
                    "stopped" | "stop requested" => colors::WARNING,
                    "failed" => colors::ERROR,
                    "deleted" | "removed" | "snapshot deleted" => colors::ERROR,
                    "discovered" => colors::INFO,
                    "starting" => colors::WARNING,
                    "restored from snapshot" => colors::SUCCESS,
                    _ => colors::INFO,
                };

                let num = format!("{:>3}. ", i + 1);
                let is_current = i == state.activity_scroll_offset;

                let row_style = if is_current {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(vec![Line::from(vec![
                    Span::styled(
                        if is_current { " > " } else { "   " },
                        Style::default().fg(colors::ORANGE),
                    ),
                    Span::styled(num, Style::default().fg(colors::TEXT_MUTED)),
                    Span::styled(format!("{} ", event.icon), Style::default().fg(icon_color)),
                    Span::styled(
                        &event.vm_name,
                        Style::default()
                            .fg(colors::TEXT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" {}", event.action),
                        Style::default().fg(colors::TEXT_MUTED),
                    ),
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        event.elapsed_display(),
                        Style::default()
                            .fg(colors::TEXT_MUTED)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ])])
                .style(row_style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title(Span::styled(
                    " Activity Log ",
                    Style::default()
                        .fg(colors::ORANGE)
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(list, chunks[1]);
    }

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "↑↓/jk",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Scroll | ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Back | ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "1-0",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Views | ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "Ctrl+P",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Jump", Style::default().fg(colors::TEXT_MUTED)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER)),
    );
    f.render_widget(help, chunks[2]);
}
