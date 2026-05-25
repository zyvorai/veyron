// Migration Wizard View

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
        "Live Migration Wizard",
        Style::default()
            .fg(Color::Rgb(255, 145, 115))
            .add_modifier(Modifier::BOLD),
    ));
    header_spans.push(Span::styled(
        " (KubeVirt only)",
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

    // Eligible VMs for migration (running VMs)
    let running_vms: Vec<&crate::tui::state::VmInfo> = state
        .vms
        .iter()
        .filter(|vm| vm.status == "Running")
        .collect();

    // Steps progress bar
    let has_vms = !running_vms.is_empty();
    let has_nodes = !state.nodes.is_empty();
    let steps = [
        ("1. Select VM", has_vms),
        ("2. Select Target", has_vms && has_nodes),
        ("3. Pre-checks", false),
        ("4. Confirm", false),
        ("5. Migrate", false),
    ];
    let current_step = if has_vms && has_nodes {
        2
    } else if has_vms {
        1
    } else {
        0
    };

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
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(steps_widget, chunks[1]);

    // Current step content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    // Left panel: eligible VMs
    let mut vm_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Eligible VMs ({} running)", running_vms.len()),
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if running_vms.is_empty() {
        vm_lines.push(Line::from(Span::styled(
            "  No running VMs available for migration",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for vm in &running_vms {
            vm_lines.push(Line::from(vec![
                Span::styled("  [", Style::default().fg(Color::Gray)),
                Span::styled("*", Style::default().fg(Color::Rgb(50, 205, 50))),
                Span::styled("] ", Style::default().fg(Color::Gray)),
                Span::styled(vm.name.as_str(), Style::default().fg(Color::White)),
            ]));
            vm_lines.push(Line::from(vec![
                Span::styled("      Node: ", Style::default().fg(Color::Gray)),
                Span::styled(vm.node.as_str(), Style::default().fg(Color::White)),
                Span::styled("  CPU: ", Style::default().fg(Color::Gray)),
                Span::styled(vm.cpu.as_str(), Style::default().fg(Color::White)),
                Span::styled("  Mem: ", Style::default().fg(Color::Gray)),
                Span::styled(vm.memory.as_str(), Style::default().fg(Color::White)),
            ]));
        }
    }

    vm_lines.push(Line::from(""));
    vm_lines.push(Line::from(Span::styled(
        format!("  Target Nodes ({} available)", state.nodes.len()),
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    vm_lines.push(Line::from(""));

    for node in &state.nodes {
        let status_color = if node.status == "Ready" {
            Color::Rgb(50, 205, 50)
        } else {
            Color::Rgb(220, 50, 47)
        };
        vm_lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(node.name.as_str(), Style::default().fg(Color::White)),
            Span::styled(" (", Style::default().fg(Color::Gray)),
            Span::styled(node.status.as_str(), Style::default().fg(status_color)),
            Span::styled(
                format!(", CPU={}, Mem={})", node.cpu_capacity, node.memory_capacity),
                Style::default().fg(Color::Gray),
            ),
        ]));
    }

    let vm_widget = Paragraph::new(vm_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " VM & Node Selection ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(vm_widget, content_chunks[0]);

    // Right panel: migration details for first running VM
    let mut details = vec![Line::from("")];

    if let Some(vm) = running_vms.first() {
        // Find a target node different from the VM's current node
        let target_node = state
            .nodes
            .iter()
            .find(|n| n.name != vm.node && n.status == "Ready")
            .map(|n| n.name.as_str())
            .unwrap_or("N/A");

        details.push(Line::from(vec![
            Span::styled("  VM:          ", Style::default().fg(Color::Gray)),
            Span::styled(vm.name.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Source:      ", Style::default().fg(Color::Gray)),
            Span::styled(vm.node.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Target:      ", Style::default().fg(Color::Gray)),
            Span::styled(
                target_node,
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Strategy:    ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Live Migration (pre-copy)",
                Style::default().fg(Color::White),
            ),
        ]));
        details.push(Line::from(vec![
            Span::styled("  CPU:         ", Style::default().fg(Color::Gray)),
            Span::styled(vm.cpu.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Memory:      ", Style::default().fg(Color::Gray)),
            Span::styled(vm.memory.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Disk:        ", Style::default().fg(Color::Gray)),
            Span::styled(vm.disk.as_str(), Style::default().fg(Color::White)),
        ]));
        details.push(Line::from(vec![
            Span::styled("  Ready:       ", Style::default().fg(Color::Gray)),
            Span::styled(
                if vm.ready { "Yes" } else { "No" },
                Style::default().fg(if vm.ready {
                    Color::Rgb(50, 205, 50)
                } else {
                    Color::Rgb(255, 200, 0)
                }),
            ),
        ]));

        // Pre-check summary
        details.push(Line::from(""));
        details.push(Line::from(Span::styled(
            "  Pre-flight Checks",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )));
        details.push(Line::from(""));

        if target_node != "N/A" {
            details.push(Line::from(vec![
                Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
                Span::styled("Target node available", Style::default().fg(Color::White)),
            ]));
        } else {
            details.push(Line::from(vec![
                Span::styled("  [FAIL] ", Style::default().fg(Color::Rgb(220, 50, 47))),
                Span::styled(
                    "No alternative target node",
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        if vm.ready {
            details.push(Line::from(vec![
                Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
                Span::styled("VM is in Ready state", Style::default().fg(Color::White)),
            ]));
        } else {
            details.push(Line::from(vec![
                Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
                Span::styled("VM not in Ready state", Style::default().fg(Color::White)),
            ]));
        }

        details.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("VM is Running", Style::default().fg(Color::White)),
        ]));
    } else {
        details.push(Line::from(Span::styled(
            "  No running VMs to migrate",
            Style::default().fg(Color::Gray),
        )));
        details.push(Line::from(""));
        details.push(Line::from(Span::styled(
            "  Start a VM first, then return to",
            Style::default().fg(Color::Gray),
        )));
        details.push(Line::from(Span::styled(
            "  the migration wizard.",
            Style::default().fg(Color::Gray),
        )));
    }

    let details_widget = Paragraph::new(details).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Migration Details ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(details_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Next Step | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Shift+Tab",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Prev Step | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Confirm | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Cancel | ", Style::default().fg(Color::Gray)),
        Span::styled("VMware/Hyper-V import: HyperSDK", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
    );
    f.render_widget(help, chunks[3]);
}
