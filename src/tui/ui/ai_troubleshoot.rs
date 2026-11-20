// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// AI Troubleshooting Wizard View

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
    let mut header_spans = gradient::brand().text("Veyron");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "AI Troubleshooting Wizard",
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

    // Find problematic VMs
    let failed_vms: Vec<&str> = state
        .vms
        .iter()
        .filter(|vm| vm.status == "Failed" || vm.status == "Error")
        .map(|vm| vm.name.as_str())
        .collect();
    let not_ready_vms: Vec<&str> = state
        .vms
        .iter()
        .filter(|vm| !vm.ready && vm.status != "Stopped")
        .map(|vm| vm.name.as_str())
        .collect();
    let no_ip_vms: Vec<&str> = state
        .vms
        .iter()
        .filter(|vm| (vm.ip == "N/A" || vm.ip.is_empty()) && vm.status == "Running")
        .map(|vm| vm.name.as_str())
        .collect();
    let no_node_vms: Vec<&str> = state
        .vms
        .iter()
        .filter(|vm| (vm.node == "N/A" || vm.node.is_empty()) && vm.status != "Stopped")
        .map(|vm| vm.name.as_str())
        .collect();

    let total_issues = failed_vms.len() + not_ready_vms.len() + no_ip_vms.len() + no_node_vms.len();

    // Issue description
    let issue_text = if total_issues > 0 {
        format!(
            "Detected {} issue(s) across {} VM(s)",
            total_issues,
            state.vms.len()
        )
    } else {
        format!("All {} VM(s) appear healthy", state.vms.len())
    };

    let input_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Issue: ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(issue_text, Style::default().fg(Color::White)),
        ]),
    ];
    let input_widget = Paragraph::new(input_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Automated Diagnosis ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(input_widget, chunks[1]);

    // Diagnosis and recommendations
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    let mut diagnosis = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Diagnostic Analysis",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Checks Performed:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    // Check: failed VMs
    if failed_vms.is_empty() {
        diagnosis.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("No VMs in failed state", Style::default().fg(Color::White)),
        ]));
    } else {
        diagnosis.push(Line::from(vec![
            Span::styled("  [FAIL] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                format!(
                    "{} VM(s) failed: {}",
                    failed_vms.len(),
                    failed_vms.join(", ")
                ),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    // Check: not ready VMs
    if not_ready_vms.is_empty() {
        diagnosis.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "All active VMs are ready",
                Style::default().fg(Color::White),
            ),
        ]));
    } else {
        diagnosis.push(Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                format!(
                    "{} VM(s) not ready: {}",
                    not_ready_vms.len(),
                    not_ready_vms.join(", ")
                ),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    // Check: no IP
    if no_ip_vms.is_empty() {
        diagnosis.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "All running VMs have IPs",
                Style::default().fg(Color::White),
            ),
        ]));
    } else {
        diagnosis.push(Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                format!(
                    "{} running VM(s) without IP: {}",
                    no_ip_vms.len(),
                    no_ip_vms.join(", ")
                ),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    // Check: no node
    if no_node_vms.is_empty() {
        diagnosis.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "All active VMs assigned to nodes",
                Style::default().fg(Color::White),
            ),
        ]));
    } else {
        diagnosis.push(Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                format!(
                    "{} VM(s) unassigned: {}",
                    no_node_vms.len(),
                    no_node_vms.join(", ")
                ),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    // Check: node health
    let unhealthy_nodes: Vec<&str> = state
        .nodes
        .iter()
        .filter(|n| n.status != "Ready")
        .map(|n| n.name.as_str())
        .collect();
    if unhealthy_nodes.is_empty() && !state.nodes.is_empty() {
        diagnosis.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("All nodes healthy", Style::default().fg(Color::White)),
        ]));
    } else if !unhealthy_nodes.is_empty() {
        diagnosis.push(Line::from(vec![
            Span::styled("  [FAIL] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                format!(
                    "{} unhealthy node(s): {}",
                    unhealthy_nodes.len(),
                    unhealthy_nodes.join(", ")
                ),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    diagnosis.push(Line::from(""));
    diagnosis.push(Line::from(Span::styled(
        "  Root Cause Assessment",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    diagnosis.push(Line::from(""));

    if total_issues == 0 {
        diagnosis.push(Line::from(vec![
            Span::styled("  Confidence: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "100%",
                Style::default()
                    .fg(Color::Rgb(50, 205, 50))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        diagnosis.push(Line::from(vec![
            Span::styled("  Result: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "No issues detected",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]));
    } else {
        let confidence = if !failed_vms.is_empty() {
            "High"
        } else {
            "Medium"
        };
        diagnosis.push(Line::from(vec![
            Span::styled("  Confidence: ", Style::default().fg(Color::Gray)),
            Span::styled(
                confidence,
                Style::default()
                    .fg(Color::Rgb(255, 200, 0))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        if !failed_vms.is_empty() {
            diagnosis.push(Line::from(vec![
                Span::styled("  Likely cause: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("VM failure in {}", failed_vms[0]),
                    Style::default().fg(Color::Rgb(220, 50, 47)),
                ),
            ]));
        }
        if !not_ready_vms.is_empty() {
            diagnosis.push(Line::from(vec![
                Span::styled("  Contributing: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{} VM(s) not ready", not_ready_vms.len()),
                    Style::default().fg(Color::Rgb(255, 200, 0)),
                ),
            ]));
        }
    }

    let diagnosis_widget = Paragraph::new(diagnosis).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Diagnosis ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(diagnosis_widget, content_chunks[0]);

    // Remediation suggestions
    let mut remediation = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Recommended Actions",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let mut action_num = 1;

    if !failed_vms.is_empty() {
        for vm_name in &failed_vms {
            remediation.push(Line::from(vec![
                Span::styled(
                    format!("  {}. ", action_num),
                    Style::default()
                        .fg(Color::Rgb(220, 50, 47))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("[URGENT] Restart failed VM: {}", vm_name),
                    Style::default().fg(Color::White),
                ),
            ]));
            action_num += 1;
        }
        remediation.push(Line::from(""));
    }

    if !no_ip_vms.is_empty() {
        remediation.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", action_num),
                Style::default()
                    .fg(Color::Rgb(255, 200, 0))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "[HIGH] Check networking for {} VM(s) without IP",
                    no_ip_vms.len()
                ),
                Style::default().fg(Color::White),
            ),
        ]));
        action_num += 1;
        remediation.push(Line::from(""));
    }

    if !not_ready_vms.is_empty() {
        remediation.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", action_num),
                Style::default()
                    .fg(Color::Rgb(100, 150, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[INFO] Investigate {} not-ready VM(s)", not_ready_vms.len()),
                Style::default().fg(Color::White),
            ),
        ]));
        remediation.push(Line::from(""));
    }

    if total_issues == 0 {
        remediation.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "No actions needed - all VMs healthy",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]));
        remediation.push(Line::from(""));
    }

    remediation.push(Line::from(Span::styled(
        "  Auto-Remediation",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    remediation.push(Line::from(""));
    if total_issues > 0 {
        remediation.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to apply action #1 automatically",
                Style::default().fg(Color::Gray),
            ),
        ]));
    } else {
        remediation.push(Line::from(vec![Span::styled(
            "  No remediation needed",
            Style::default().fg(Color::Gray),
        )]));
    }

    let remediation_widget = Paragraph::new(remediation).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Remediation ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(remediation_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "↑↓",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Actions | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Apply Fix | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Re-diagnose | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "e",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Edit Issue | ", Style::default().fg(Color::Gray)),
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
    f.render_widget(help, chunks[3]);
}
