// Migration Wizard View

use crate::tui::colors::gradient;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};


pub fn render(f: &mut Frame, area: Rect) {
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
    header_spans.push(Span::styled(" | ", Style::default().fg(Color::Rgb(128, 128, 128))));
    header_spans.push(Span::styled("Live Migration Wizard", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
    let header_text = Line::from(header_spans);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
        );
    f.render_widget(header, chunks[0]);

    // Steps progress bar
    let steps = [("1. Select VM", true),
        ("2. Select Target", true),
        ("3. Pre-checks", false),
        ("4. Confirm", false),
        ("5. Migrate", false)];
    let current_step = 2; // 0-indexed, currently on step 3

    let step_spans: Vec<Span> = steps
        .iter()
        .enumerate()
        .flat_map(|(i, (label, completed))| {
            let style = if i == current_step {
                Style::default()
                    .fg(Color::Rgb(255, 200, 0))
                    .add_modifier(Modifier::BOLD)
            } else if *completed {
                Style::default().fg(Color::Rgb(50, 205, 50))
            } else {
                Style::default().fg(Color::Gray)
            };
            let connector = if i < steps.len() - 1 { " --> " } else { "" };
            vec![
                Span::styled(*label, style),
                Span::styled(connector, Style::default().fg(Color::Gray)),
            ]
        })
        .collect();

    let steps_widget = Paragraph::new(vec![Line::from(""), Line::from(step_spans)])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Migration Steps ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(steps_widget, chunks[1]);

    // Current step content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[2]);

    // Pre-check results
    let prechecks = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("Sufficient CPU on target node", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("Sufficient memory on target node", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("Network connectivity verified", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("Storage latency > 5ms", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("No anti-affinity conflicts", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Result: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "4 passed, 1 warning, 0 failed",
                Style::default().fg(Color::Rgb(50, 205, 50)).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let prechecks_widget = Paragraph::new(prechecks).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Pre-flight Checks ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(prechecks_widget, content_chunks[0]);

    // Migration details
    let details = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  VM:          ", Style::default().fg(Color::Gray)),
            Span::styled("web-server-01", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Source:      ", Style::default().fg(Color::Gray)),
            Span::styled("node-1", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Target:      ", Style::default().fg(Color::Gray)),
            Span::styled("node-3", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Strategy:    ", Style::default().fg(Color::Gray)),
            Span::styled("Live Migration (pre-copy)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Est. Time:   ", Style::default().fg(Color::Gray)),
            Span::styled("~45 seconds", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Bandwidth:   ", Style::default().fg(Color::Gray)),
            Span::styled("64 MiB/s", Style::default().fg(Color::White)),
        ]),
    ];
    let details_widget = Paragraph::new(details).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Migration Details ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(details_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Tab", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Next Step | ", Style::default().fg(Color::Gray)),
        Span::styled("Shift+Tab", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Prev Step | ", Style::default().fg(Color::Gray)),
        Span::styled("Enter", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Confirm | ", Style::default().fg(Color::Gray)),
        Span::styled("Esc", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Cancel", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
    );
    f.render_widget(help, chunks[3]);
}
