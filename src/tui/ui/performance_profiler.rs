// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Performance Profiler View

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

    let stats = state.get_stats();

    // Gradient brand header
    let first_vm_name = state.vms.first().map(|v| v.name.as_str()).unwrap_or("N/A");
    let mut header_spans = gradient::brand().text("Veyron");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "Performance Profiler",
        Style::default()
            .fg(Color::Rgb(255, 145, 115))
            .add_modifier(Modifier::BOLD),
    ));
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        format!("VM: {}", first_vm_name),
        Style::default().fg(Color::Rgb(222, 115, 86)),
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

    // Resource gauges from history
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    let cpu_pct = state.cpu_history.last().copied().unwrap_or(0).min(100) as u16;
    let mem_pct = state.memory_history.last().copied().unwrap_or(0).min(100) as u16;
    let disk_pct = state.disk_history.last().copied().unwrap_or(0).min(100) as u16;
    let net_pct = state.network_history.last().copied().unwrap_or(0).min(100) as u16;

    let gauges = [
        (
            "CPU",
            cpu_pct,
            gradient::health().at(cpu_pct as f64 / 100.0),
        ),
        ("Memory", mem_pct, Color::Rgb(100, 150, 255)),
        ("Disk I/O", disk_pct, Color::Rgb(50, 205, 50)),
        ("Network", net_pct, Color::Rgb(255, 200, 0)),
    ];

    for (i, (name, pct, color)) in gauges.iter().enumerate() {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                    .title(format!(" {} ", name)),
            )
            .gauge_style(Style::default().fg(*color).bg(Color::Rgb(40, 35, 55)))
            .percent(*pct)
            .label(format!("{}%", pct));
        f.render_widget(gauge, gauge_chunks[i]);
    }

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[2]);

    // Per-VM performance table
    let header_cells = ["VM Name", "CPU", "Memory", "Status", "Node"]
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

    let rows: Vec<Row> = state
        .vms
        .iter()
        .map(|vm| {
            let status_color = match vm.status.as_str() {
                "Running" => Color::Rgb(50, 205, 50),
                "Stopped" => Color::Gray,
                "Failed" | "Error" => Color::Rgb(220, 50, 47),
                "Starting" | "Pending" => Color::Rgb(255, 200, 0),
                _ => Color::Gray,
            };
            Row::new(vec![
                Cell::from(vm.name.as_str()),
                Cell::from(vm.cpu.as_str()),
                Cell::from(vm.memory.as_str()),
                Cell::from(vm.status.as_str()).style(Style::default().fg(status_color)),
                Cell::from(vm.node.as_str()),
            ])
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Percentage(28),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Percentage(18),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    format!(" VMs ({}) ", state.vms.len()),
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, main_chunks[0]);

    // Performance insights from real stats
    let running_label = if stats.running == stats.total && stats.total > 0 {
        (
            "  [INFO] ",
            Color::Rgb(100, 150, 255),
            format!("CPU: All {} VMs running, cluster healthy", stats.total),
        )
    } else if stats.failed > 0 {
        (
            "  [WARN] ",
            Color::Rgb(255, 200, 0),
            format!(
                "CPU: {} of {} VMs not running ({} failed)",
                stats.total - stats.running,
                stats.total,
                stats.failed
            ),
        )
    } else {
        (
            "  [INFO] ",
            Color::Rgb(100, 150, 255),
            format!("CPU: {} of {} VMs running", stats.running, stats.total),
        )
    };

    let mem_label = if mem_pct > 80 {
        (
            "  [WARN] ",
            Color::Rgb(255, 200, 0),
            format!("Memory: High utilization at {}%", mem_pct),
        )
    } else {
        (
            "  [INFO] ",
            Color::Rgb(100, 150, 255),
            format!("Memory: Normal utilization at {}%", mem_pct),
        )
    };

    let disk_label = if disk_pct > 80 {
        (
            "  [WARN] ",
            Color::Rgb(255, 200, 0),
            format!("Disk: High I/O at {}%", disk_pct),
        )
    } else {
        (
            "  [INFO] ",
            Color::Rgb(100, 150, 255),
            format!("Disk: Normal I/O at {}%", disk_pct),
        )
    };

    let net_label = if net_pct > 80 {
        (
            "  [WARN] ",
            Color::Rgb(255, 200, 0),
            format!("Network: High throughput at {}%", net_pct),
        )
    } else {
        (
            "  [INFO] ",
            Color::Rgb(100, 150, 255),
            format!("Network: Steady throughput at {}%", net_pct),
        )
    };

    let bottleneck = if stats.failed > 0 {
        ("Failed VMs detected", Color::Rgb(220, 50, 47))
    } else if mem_pct > 80 || cpu_pct > 80 {
        ("Resource pressure detected", Color::Rgb(255, 200, 0))
    } else {
        ("None detected", Color::Rgb(50, 205, 50))
    };

    let insights = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Performance Insights",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(running_label.0, Style::default().fg(running_label.1)),
            Span::styled(running_label.2.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(mem_label.0, Style::default().fg(mem_label.1)),
            Span::styled(mem_label.2.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(disk_label.0, Style::default().fg(disk_label.1)),
            Span::styled(disk_label.2.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(net_label.0, Style::default().fg(net_label.1)),
            Span::styled(net_label.2.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Bottleneck Analysis",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Primary:   ", Style::default().fg(Color::Gray)),
            Span::styled(bottleneck.0, Style::default().fg(bottleneck.1)),
        ]),
        Line::from(vec![
            Span::styled("  VMs Total: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!(
                    "{} (R:{} S:{} F:{})",
                    stats.total, stats.running, stats.stopped, stats.failed
                ),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Profiling: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Active",
                Style::default()
                    .fg(Color::Rgb(50, 205, 50))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({}s refresh)", state.refresh_interval),
                Style::default().fg(Color::Gray),
            ),
        ]),
    ];
    let insights_widget = Paragraph::new(insights).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Analysis ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(insights_widget, main_chunks[1]);

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
            "v",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Select VM | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "p",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Profile | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "e",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Export | ", Style::default().fg(Color::Gray)),
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
