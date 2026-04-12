// Activity Log View - Full-screen scrollable activity event log

use crate::tui::colors::tui as colors;
use crate::tui::{config::TuiConfig, state::AppState};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
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
    let header_text = Line::from(vec![
        Span::styled(
            "VMRogue",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" - ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "KubeVirt VM Manager",
            Style::default().fg(colors::TEXT),
        ),
        Span::styled("  │  ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "⏱  Activity Log",
            Style::default()
                .fg(colors::LIGHT_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(": {} events", state.recent_activity.len()),
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
                    "⏱  Activity Log",
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
            .map(|(i, event)| {
                let icon_color = match event.action.as_str() {
                    "started" | "start requested" => colors::SUCCESS,
                    "stopped" | "stop requested" => colors::WARNING,
                    "failed" => colors::ERROR,
                    "deleted" | "removed" => colors::ERROR,
                    "discovered" => colors::INFO,
                    "starting" => colors::WARNING,
                    _ => colors::INFO,
                };

                let num = format!("{:>3}. ", i + 1);

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(num, Style::default().fg(colors::TEXT_MUTED)),
                        Span::styled(
                            format!("{} ", event.icon),
                            Style::default().fg(icon_color),
                        ),
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
                    ]),
                ])
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title(Span::styled(
                    "⏱  Activity Log",
                    Style::default()
                        .fg(colors::ORANGE)
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(list, chunks[1]);
    }

    // Help
    let help = Paragraph::new("1-6: Views │ q: Quit")
        .style(Style::default().fg(colors::TEXT_MUTED))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );
    f.render_widget(help, chunks[2]);
}
