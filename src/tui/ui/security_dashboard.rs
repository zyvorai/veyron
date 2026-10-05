// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Security Dashboard View — wired to real VM data from AppState

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
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
        "Security Dashboard",
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

    // Analyze VMs for security findings
    let mut critical = 0u32;
    let mut high = 0u32;
    let mut medium = 0u32;
    let mut low = 0u32;
    let mut findings: Vec<(&str, String, String, &str)> = Vec::new();

    for vm in &state.vms {
        let name = &vm.name;

        // Check resource limits (disk field used as proxy for config completeness)
        if vm.disk.is_empty() || vm.disk == "0" {
            high += 1;
            findings.push((
                "HIGH",
                "Resource".to_string(),
                format!("No disk configured for '{}'", name),
                "Open",
            ));
        }

        // Check if VM is in failed state
        if vm.status.contains("Fail") || vm.status.contains("Error") {
            critical += 1;
            findings.push((
                "CRIT",
                "Runtime".to_string(),
                format!("VM '{}' in {} state", name, vm.status),
                "Open",
            ));
        }

        // Check for VMs without IP (possibly misconfigured networking)
        if vm.ip.is_empty() && vm.status.contains("Running") {
            medium += 1;
            findings.push((
                "MED",
                "Network".to_string(),
                format!("Running VM '{}' has no IP assigned", name),
                "Open",
            ));
        }

        // Check for VMs on unknown nodes
        if vm.node.is_empty() && vm.status.contains("Running") {
            low += 1;
            findings.push((
                "LOW",
                "Scheduling".to_string(),
                format!("VM '{}' node not reported", name),
                "Open",
            ));
        }
    }

    let total = critical + high + medium + low;
    let score = if total == 0 {
        100u16
    } else {
        let deductions =
            critical as u16 * 20 + high as u16 * 10 + medium as u16 * 5 + low as u16 * 2;
        100u16.saturating_sub(deductions)
    };
    let score_label = match score {
        90..=100 => "Excellent",
        70..=89 => "Good",
        50..=69 => "Fair",
        _ => "Poor",
    };

    // Score gauge
    let score_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    let gauge_color = match score {
        90..=100 => Color::Rgb(50, 205, 50),
        70..=89 => Color::Rgb(255, 200, 0),
        _ => Color::Rgb(220, 50, 47),
    };

    let score_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Security Posture Score "),
        )
        .gauge_style(Style::default().fg(gauge_color).bg(Color::Rgb(40, 35, 55)))
        .percent(score)
        .label(format!("{}/100 - {}", score, score_label));
    f.render_widget(score_gauge, score_chunks[0]);

    let summary = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Critical: ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                critical.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  High: ", Style::default().fg(Color::Rgb(255, 165, 0))),
            Span::styled(
                high.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Medium: ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                medium.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Low: ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled(
                low.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let summary_widget = Paragraph::new(summary).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(format!(" Findings ({} VMs analyzed) ", state.vms.len())),
    );
    f.render_widget(summary_widget, score_chunks[1]);

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[2]);

    // Findings table
    let header_cells = ["Severity", "Category", "Finding", "Status"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )
        });
    let table_header = Row::new(header_cells)
        .style(Style::default().bg(Color::Rgb(40, 35, 55)))
        .height(1);

    let rows = findings.iter().map(|(sev, cat, finding, status)| {
        let sev_color = match *sev {
            "CRIT" => Color::Rgb(220, 50, 47),
            "HIGH" => Color::Rgb(255, 165, 0),
            "MED" => Color::Rgb(255, 200, 0),
            _ => Color::Rgb(100, 150, 255),
        };
        Row::new(vec![
            Cell::from(*sev).style(Style::default().fg(sev_color).add_modifier(Modifier::BOLD)),
            Cell::from(cat.as_str()),
            Cell::from(finding.as_str()),
            Cell::from(*status).style(Style::default().fg(Color::Rgb(220, 50, 47))),
        ])
        .height(1)
    });

    let empty_msg = if findings.is_empty() {
        vec![Row::new(vec![
            Cell::from("  No security findings - all VMs healthy")
                .style(Style::default().fg(Color::Rgb(50, 205, 50))),
        ])]
    } else {
        vec![]
    };

    let all_rows: Vec<Row> = rows.chain(empty_msg).collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Percentage(18),
        Constraint::Percentage(48),
        Constraint::Percentage(20),
    ];

    let table = Table::new(all_rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Security Findings ",
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, main_chunks[0]);

    // VM Status Summary
    let stats = state.get_stats();
    let vm_status_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  VM Fleet Status",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Total VMs:    ", Style::default().fg(Color::White)),
            Span::styled(
                stats.total.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Running:      ", Style::default().fg(Color::White)),
            Span::styled(
                stats.running.to_string(),
                Style::default()
                    .fg(Color::Rgb(50, 205, 50))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Stopped:      ", Style::default().fg(Color::White)),
            Span::styled(
                stats.stopped.to_string(),
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Failed:       ", Style::default().fg(Color::White)),
            Span::styled(
                stats.failed.to_string(),
                Style::default()
                    .fg(if stats.failed > 0 {
                        Color::Rgb(220, 50, 47)
                    } else {
                        Color::Gray
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Last refresh: {}", state.last_refresh.format("%H:%M:%S")),
            Style::default().fg(Color::Gray),
        )),
    ];
    let status_widget = Paragraph::new(vm_status_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Fleet Overview ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(status_widget, main_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "Backspace",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Back | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Refresh", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
    );
    f.render_widget(help, chunks[3]);
}
