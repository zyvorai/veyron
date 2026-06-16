// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
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

    // Dependency graph - show VMs grouped by node
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    // --- Build node -> VM mapping from state.vms ---
    use std::collections::BTreeMap;
    let mut node_vms: BTreeMap<String, Vec<(&str, &str, Color)>> = BTreeMap::new();

    for vm in &state.vms {
        let node_name = if vm.node == "N/A" || vm.node.is_empty() {
            "unassigned".to_string()
        } else {
            vm.node.clone()
        };
        let status_color = match vm.status.as_str() {
            "Running" => Color::Rgb(50, 205, 50),
            "Stopped" => Color::Rgb(255, 200, 0),
            "Starting" | "Pending" => Color::Rgb(100, 150, 255),
            "Failed" | "Error" => Color::Rgb(220, 50, 47),
            _ => Color::Gray,
        };
        node_vms
            .entry(node_name)
            .or_default()
            .push((&vm.name, &vm.status, status_color));
    }

    let mut graph_lines: Vec<Line> = Vec::new();
    graph_lines.push(Line::from(""));

    if state.vms.is_empty() {
        graph_lines.push(Line::from(Span::styled(
            "  No VMs discovered",
            Style::default().fg(Color::Gray),
        )));
    } else {
        // Show cluster topology: nodes with their VMs
        graph_lines.push(Line::from(Span::styled(
            "                    [cluster]",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )));

        for (node_name, vms) in &node_vms {
            graph_lines.push(Line::from(Span::styled(
                "                       |",
                Style::default().fg(Color::Gray),
            )));
            graph_lines.push(Line::from(Span::styled(
                "                       v",
                Style::default().fg(Color::Gray),
            )));

            let node_color = if node_name == "unassigned" {
                Color::Rgb(255, 200, 0)
            } else {
                Color::Rgb(100, 150, 255)
            };
            graph_lines.push(Line::from(Span::styled(
                format!("               [{}]", node_name),
                Style::default().fg(node_color).add_modifier(Modifier::BOLD),
            )));

            for (vm_name, _status, color) in vms {
                graph_lines.push(Line::from(vec![
                    Span::styled("                  +-- ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("[{}]", vm_name), Style::default().fg(*color)),
                ]));
            }
        }
    }

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
    let stats = state.get_stats();
    let unique_nodes = node_vms.len();

    let mut legend: Vec<Line> = Vec::new();
    legend.push(Line::from(""));
    legend.push(Line::from(Span::styled(
        "  Legend",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    legend.push(Line::from(""));
    legend.push(Line::from(vec![
        Span::styled("  [*] ", Style::default().fg(Color::Rgb(50, 205, 50))),
        Span::styled("Running", Style::default().fg(Color::White)),
    ]));
    legend.push(Line::from(vec![
        Span::styled("  [*] ", Style::default().fg(Color::Rgb(255, 200, 0))),
        Span::styled("Stopped / Unassigned", Style::default().fg(Color::White)),
    ]));
    legend.push(Line::from(vec![
        Span::styled("  [*] ", Style::default().fg(Color::Rgb(100, 150, 255))),
        Span::styled("Node / Starting", Style::default().fg(Color::White)),
    ]));
    legend.push(Line::from(vec![
        Span::styled("  [*] ", Style::default().fg(Color::Rgb(220, 50, 47))),
        Span::styled("Failed / Error", Style::default().fg(Color::White)),
    ]));
    legend.push(Line::from(vec![
        Span::styled("  [*] ", Style::default().fg(Color::Rgb(222, 115, 86))),
        Span::styled("Cluster root", Style::default().fg(Color::White)),
    ]));
    legend.push(Line::from(""));
    legend.push(Line::from(Span::styled(
        "  Cluster Summary",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    legend.push(Line::from(""));
    legend.push(Line::from(vec![
        Span::styled("  Total VMs:   ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.total),
            Style::default().fg(Color::White),
        ),
    ]));
    legend.push(Line::from(vec![
        Span::styled("  Running:     ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.running),
            Style::default().fg(Color::Rgb(50, 205, 50)),
        ),
    ]));
    legend.push(Line::from(vec![
        Span::styled("  Stopped:     ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.stopped),
            Style::default().fg(Color::Rgb(255, 200, 0)),
        ),
    ]));
    legend.push(Line::from(vec![
        Span::styled("  Failed:      ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.failed),
            Style::default().fg(Color::Rgb(220, 50, 47)),
        ),
    ]));
    legend.push(Line::from(vec![
        Span::styled("  Nodes used:  ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", unique_nodes),
            Style::default().fg(Color::White),
        ),
    ]));

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
