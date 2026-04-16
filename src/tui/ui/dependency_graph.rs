// VM Dependency Graph View

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
        "VM Dependency Graph",
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

    // Dependency graph (ASCII art representation)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    let graph_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "                   [load-balancer]",
            Style::default()
                .fg(Color::Rgb(50, 205, 50))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "                    /           \\",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "                   v             v",
            Style::default().fg(Color::Gray),
        )),
        Line::from(vec![
            Span::styled(
                "           [web-server-01]",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
            Span::styled(
                "   [web-server-02]",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]),
        Line::from(Span::styled(
            "                   \\           /",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "                    v         v",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "                  [api-gateway]",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "                   /         \\",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "                  v           v",
            Style::default().fg(Color::Gray),
        )),
        Line::from(vec![
            Span::styled(
                "           [db-primary]",
                Style::default().fg(Color::Rgb(100, 150, 255)),
            ),
            Span::styled(
                "      [cache-server]",
                Style::default().fg(Color::Rgb(255, 200, 0)),
            ),
        ]),
        Line::from(Span::styled(
            "                  |",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "                  v",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "             [db-replica]",
            Style::default().fg(Color::Rgb(100, 150, 255)),
        )),
    ];
    let graph_widget = Paragraph::new(graph_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Dependency Tree ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(graph_widget, content_chunks[0]);

    // Legend and details
    let legend = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Legend",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [*] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("Running / Healthy", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [*] ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled("Database tier", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [*] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("Cache tier", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [*] ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("API tier", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Selected: api-gateway",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Depends on:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                "db-primary, cache-server",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Required by: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "web-server-01, web-server-02",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Impact:      ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Stopping affects 4 VMs",
                Style::default().fg(Color::Rgb(255, 200, 0)),
            ),
        ]),
    ];
    let legend_widget = Paragraph::new(legend).block(
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
    f.render_widget(legend_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Cycle VMs | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Inspect | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "e",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Edit Deps | ", Style::default().fg(Color::Gray)),
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
