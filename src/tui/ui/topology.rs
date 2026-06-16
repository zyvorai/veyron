// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Topology Map View - Shows nodes and VMs in a visual layout

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

    // Header
    // Gradient brand header
    let mut header_spans = gradient::brand().text("Veyron");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "Cluster Topology",
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

    // Topology visualization
    let node_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[1]);

    // Node 1
    let node1_vms = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Role: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "control-plane",
                Style::default().fg(Color::Rgb(222, 115, 86)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(Color::Gray)),
            Span::styled("Ready", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  --- VMs ---",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  [*] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("web-server-01", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [*] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("api-gateway", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  CPU: ", Style::default().fg(Color::Gray)),
            Span::styled("45%", Style::default().fg(Color::White)),
            Span::styled("  Mem: ", Style::default().fg(Color::Gray)),
            Span::styled("62%", Style::default().fg(Color::White)),
        ]),
    ];
    let node1 = Paragraph::new(node1_vms).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " node-1 ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(node1, node_chunks[0]);

    // Node 2
    let node2_vms = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Role: ", Style::default().fg(Color::Gray)),
            Span::styled("worker", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(Color::Gray)),
            Span::styled("Ready", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  --- VMs ---",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  [*] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("db-primary", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [*] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("worker-node-01", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [!] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("worker-node-02", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  CPU: ", Style::default().fg(Color::Gray)),
            Span::styled("72%", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("  Mem: ", Style::default().fg(Color::Gray)),
            Span::styled("58%", Style::default().fg(Color::White)),
        ]),
    ];
    let node2 = Paragraph::new(node2_vms).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " node-2 ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(node2, node_chunks[1]);

    // Node 3
    let node3_vms = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Role: ", Style::default().fg(Color::Gray)),
            Span::styled("worker", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(Color::Gray)),
            Span::styled("Ready", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  --- VMs ---",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  [*] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("cache-server", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  CPU: ", Style::default().fg(Color::Gray)),
            Span::styled("38%", Style::default().fg(Color::White)),
            Span::styled("  Mem: ", Style::default().fg(Color::Gray)),
            Span::styled("41%", Style::default().fg(Color::White)),
        ]),
    ];
    let node3 = Paragraph::new(node3_vms).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " node-3 ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(node3, node_chunks[2]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Cycle Nodes | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Node Details | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "m",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Migrate | ", Style::default().fg(Color::Gray)),
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
