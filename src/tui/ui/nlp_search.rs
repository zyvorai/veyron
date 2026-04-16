// Natural Language Search Interface View

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
            Constraint::Length(5),
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
        "Natural Language Search",
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

    // Search input area
    let search_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  > ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Type to filter VMs...",
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let search_widget = Paragraph::new(search_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Query ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(search_widget, chunks[1]);

    // Results
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[2]);

    // Build results from state.vms
    let mut results = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Search Results ({} VMs)", state.vms.len()),
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, vm) in state.vms.iter().enumerate() {
        let status_color = match vm.status.as_str() {
            "Running" => Color::Rgb(50, 205, 50),
            "Stopped" => Color::Gray,
            "Failed" | "Error" => Color::Rgb(220, 50, 47),
            "Starting" | "Pending" => Color::Rgb(255, 200, 0),
            _ => Color::Gray,
        };

        results.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", i + 1),
                Style::default().fg(Color::Rgb(222, 115, 86)),
            ),
            Span::styled(
                vm.name.as_str(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        results.push(Line::from(vec![
            Span::styled("     Status: ", Style::default().fg(Color::Gray)),
            Span::styled(vm.status.as_str(), Style::default().fg(status_color)),
            Span::styled("  CPU: ", Style::default().fg(Color::Gray)),
            Span::styled(vm.cpu.as_str(), Style::default().fg(Color::White)),
            Span::styled("  Mem: ", Style::default().fg(Color::Gray)),
            Span::styled(vm.memory.as_str(), Style::default().fg(Color::White)),
        ]));
        results.push(Line::from(""));
    }

    if state.vms.is_empty() {
        results.push(Line::from(Span::styled(
            "  No VMs found",
            Style::default().fg(Color::Gray),
        )));
    }

    let results_widget = Paragraph::new(results).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Results ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(results_widget, content_chunks[0]);

    // Parsed query and suggestions
    let stats = state.get_stats();
    let parsed = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  VM Summary",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Total:    ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}", stats.total), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Running:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.running),
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Stopped:  ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}", stats.stopped), Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  Failed:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.failed),
                Style::default().fg(if stats.failed > 0 { Color::Rgb(220, 50, 47) } else { Color::Gray }),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Suggestions",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  - ", Style::default().fg(Color::Gray)),
            Span::styled(
                "\"VMs with high memory usage\"",
                Style::default().fg(Color::Rgb(100, 150, 255)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  - ", Style::default().fg(Color::Gray)),
            Span::styled(
                "\"failed or stopped VMs\"",
                Style::default().fg(Color::Rgb(100, 150, 255)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  - ", Style::default().fg(Color::Gray)),
            Span::styled(
                "\"VMs without IP address\"",
                Style::default().fg(Color::Rgb(100, 150, 255)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  - ", Style::default().fg(Color::Gray)),
            Span::styled(
                "\"running VMs on specific node\"",
                Style::default().fg(Color::Rgb(100, 150, 255)),
            ),
        ]),
    ];
    let parsed_widget = Paragraph::new(parsed).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Query Analysis ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(parsed_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "Type",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Search | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Execute | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Autocomplete | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Ctrl+H",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": History | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Esc",
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
    f.render_widget(help, chunks[3]);
}
