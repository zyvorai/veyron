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

pub fn render(f: &mut Frame, area: Rect, _state: &AppState) {
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
    // Gradient brand header
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
                "show me all VMs using more than 4 cores that are running",
                Style::default().fg(Color::White),
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

    let results = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Search Results (3 matches)",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled(
                "db-primary",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("     Status: ", Style::default().fg(Color::Gray)),
            Span::styled("Running", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("  CPU: ", Style::default().fg(Color::Gray)),
            Span::styled("8 cores", Style::default().fg(Color::White)),
            Span::styled("  Mem: ", Style::default().fg(Color::Gray)),
            Span::styled("32Gi", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled(
                "api-gateway",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("     Status: ", Style::default().fg(Color::Gray)),
            Span::styled("Running", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("  CPU: ", Style::default().fg(Color::Gray)),
            Span::styled("6 cores", Style::default().fg(Color::White)),
            Span::styled("  Mem: ", Style::default().fg(Color::Gray)),
            Span::styled("16Gi", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled(
                "worker-node-01",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("     Status: ", Style::default().fg(Color::Gray)),
            Span::styled("Running", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("  CPU: ", Style::default().fg(Color::Gray)),
            Span::styled("4 cores", Style::default().fg(Color::White)),
            Span::styled("  Mem: ", Style::default().fg(Color::Gray)),
            Span::styled("8Gi", Style::default().fg(Color::White)),
        ]),
    ];
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
    let parsed = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Parsed Query",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Resource: ", Style::default().fg(Color::Gray)),
            Span::styled("VirtualMachine", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Filter:   ", Style::default().fg(Color::Gray)),
            Span::styled("cpu >= 4", Style::default().fg(Color::Rgb(222, 115, 86))),
        ]),
        Line::from(vec![
            Span::styled("  Filter:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                "status = Running",
                Style::default().fg(Color::Rgb(222, 115, 86)),
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
                "\"failed migrations this week\"",
                Style::default().fg(Color::Rgb(100, 150, 255)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  - ", Style::default().fg(Color::Gray)),
            Span::styled(
                "\"snapshots older than 7 days\"",
                Style::default().fg(Color::Rgb(100, 150, 255)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  - ", Style::default().fg(Color::Gray)),
            Span::styled(
                "\"nodes with most pods\"",
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
