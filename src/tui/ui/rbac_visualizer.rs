// RBAC Visualizer View

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use std::collections::HashMap;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
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
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "RBAC Visualizer",
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

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Node access table: show nodes with VM counts
    let role_header = ["Node", "Role", "Status", "VMs"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )
    });
    let role_header_row = Row::new(role_header)
        .style(Style::default().bg(Color::Rgb(40, 35, 55)))
        .height(1);

    // Count VMs per node
    let mut vms_per_node: HashMap<String, usize> = HashMap::new();
    for vm in &state.vms {
        if vm.node != "N/A" && !vm.node.is_empty() {
            *vms_per_node.entry(vm.node.clone()).or_insert(0) += 1;
        }
    }

    let role_rows: Vec<Row> = state.nodes.iter().map(|node| {
        let status_color = if node.status == "Ready" {
            Color::Rgb(50, 205, 50)
        } else {
            Color::Rgb(220, 50, 47)
        };
        let role_color = if node.role.contains("control-plane") || node.role.contains("master") {
            Color::Rgb(222, 115, 86)
        } else {
            Color::Rgb(100, 150, 255)
        };
        let vm_count = vms_per_node.get(&node.name).copied().unwrap_or(0);
        Row::new(vec![
            Cell::from(node.name.as_str()),
            Cell::from(node.role.as_str()).style(Style::default().fg(role_color)),
            Cell::from(node.status.as_str()).style(Style::default().fg(status_color)),
            Cell::from(format!("{}", vm_count)),
        ])
        .height(1)
    }).collect();

    let role_widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(20),
    ];

    let role_table = Table::new(role_rows, role_widths)
        .header(role_header_row)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    format!(" Nodes ({}) ", state.nodes.len()),
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(role_table, main_chunks[0]);

    // Access overview from VM data
    let stats = state.get_stats();
    let unassigned_vms = state.vms.iter().filter(|vm| vm.node == "N/A" || vm.node.is_empty()).count();
    let nodes_with_vms = vms_per_node.len();

    let mut binding_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Resource Overview",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Total VMs:      ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.total),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Running:        ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.running),
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Nodes w/ VMs:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", nodes_with_vms),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Unassigned VMs: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", unassigned_vms),
                Style::default().fg(if unassigned_vms > 0 { Color::Rgb(255, 200, 0) } else { Color::Rgb(50, 205, 50) }),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Node Capacity",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for node in &state.nodes {
        binding_lines.push(Line::from(vec![
            Span::styled(format!("  {}: ", node.name), Style::default().fg(Color::White)),
            Span::styled(format!("CPU={}", node.cpu_capacity), Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled(format!(" Mem={}", node.memory_capacity), Style::default().fg(Color::Rgb(100, 150, 255))),
        ]));
    }

    if state.nodes.is_empty() {
        binding_lines.push(Line::from(Span::styled(
            "  No node data available",
            Style::default().fg(Color::Gray),
        )));
    }

    binding_lines.push(Line::from(""));
    binding_lines.push(Line::from(Span::styled(
        "  Warnings",
        Style::default()
            .fg(Color::Rgb(255, 200, 0))
            .add_modifier(Modifier::BOLD),
    )));
    binding_lines.push(Line::from(""));

    if stats.failed > 0 {
        binding_lines.push(Line::from(vec![
            Span::styled("  [!] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                format!("{} VM(s) in failed state", stats.failed),
                Style::default().fg(Color::White),
            ),
        ]));
    }
    if unassigned_vms > 0 {
        binding_lines.push(Line::from(vec![
            Span::styled("  [!] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                format!("{} VM(s) not assigned to any node", unassigned_vms),
                Style::default().fg(Color::White),
            ),
        ]));
    }
    if stats.failed == 0 && unassigned_vms == 0 {
        binding_lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "No warnings",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]));
    }

    let bindings_widget = Paragraph::new(binding_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Access Overview ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(bindings_widget, main_chunks[1]);

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
            "a",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Audit | ", Style::default().fg(Color::Gray)),
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
