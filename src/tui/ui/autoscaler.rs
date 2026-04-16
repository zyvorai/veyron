// Autoscaler Policy and Recommendations View

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
    // Gradient brand header
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "Autoscaler",
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

    // --- Derive utilization from state ---
    let stats = state.get_stats();
    let total_vms = stats.total.max(1);

    let cpu_now = state.cpu_history.last().copied().unwrap_or(0).min(100) as u16;
    let mem_now = state.memory_history.last().copied().unwrap_or(0).min(100) as u16;
    let vm_density = if total_vms > 0 {
        ((stats.running as f64 / total_vms as f64) * 100.0).round() as u16
    } else {
        0
    };

    // Cluster utilization gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[1]);

    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" CPU Utilization "),
        )
        .gauge_style(
            Style::default()
                .fg(gradient::health().at(cpu_now as f64 / 100.0))
                .bg(Color::Rgb(40, 35, 55)),
        )
        .percent(cpu_now)
        .label(format!("{}%", cpu_now));
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Memory Utilization "),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(100, 150, 255))
                .bg(Color::Rgb(40, 35, 55)),
        )
        .percent(mem_now)
        .label(format!("{}%", mem_now));
    f.render_widget(mem_gauge, gauge_chunks[1]);

    let vm_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" VM Active Ratio "),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(50, 205, 50))
                .bg(Color::Rgb(40, 35, 55)),
        )
        .percent(vm_density)
        .label(format!("{}/{} running", stats.running, stats.total));
    f.render_widget(vm_gauge, gauge_chunks[2]);

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[2]);

    // VM resource status table
    let header_cells = ["VM Name", "Status", "CPU", "Memory", "Node", "Ready"]
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

    let rows: Vec<Row> = state.vms.iter().map(|vm| {
        let status_color = match vm.status.as_str() {
            "Running" => Color::Rgb(50, 205, 50),
            "Stopped" => Color::Gray,
            "Starting" | "Pending" => Color::Rgb(255, 200, 0),
            "Failed" | "Error" => Color::Rgb(220, 50, 47),
            _ => Color::Gray,
        };
        let ready_text = if vm.ready { "Yes" } else { "No" };
        let ready_color = if vm.ready { Color::Rgb(50, 205, 50) } else { Color::Rgb(220, 50, 47) };

        Row::new(vec![
            Cell::from(vm.name.as_str()).style(Style::default().fg(Color::White)),
            Cell::from(vm.status.as_str()).style(
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from(vm.cpu.as_str()),
            Cell::from(vm.memory.as_str()),
            Cell::from(vm.node.as_str()),
            Cell::from(ready_text).style(Style::default().fg(ready_color)),
        ])
        .height(1)
    }).collect();

    let widths = [
        Constraint::Percentage(24),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
        Constraint::Percentage(20),
        Constraint::Percentage(10),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    format!(" VM Resources ({} VMs) ", stats.total),
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, main_chunks[0]);

    // Recommendations and recent activity
    let mut recommendations: Vec<Line> = Vec::new();
    recommendations.push(Line::from(""));

    // Scaling status
    let scaling_status = if cpu_now > 80 {
        ("Scaling Recommended", Color::Rgb(255, 200, 0))
    } else if stats.failed > 0 {
        ("Attention Needed", Color::Rgb(220, 50, 47))
    } else {
        ("Stable", Color::Rgb(50, 205, 50))
    };
    recommendations.push(Line::from(Span::styled(
        format!("  Cluster Status: {}", scaling_status.0),
        Style::default()
            .fg(scaling_status.1)
            .add_modifier(Modifier::BOLD),
    )));
    recommendations.push(Line::from(""));

    // Stats summary
    recommendations.push(Line::from(vec![
        Span::styled("  Running: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.running),
            Style::default().fg(Color::Rgb(50, 205, 50)),
        ),
        Span::styled("  Stopped: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.stopped),
            Style::default().fg(Color::Rgb(255, 200, 0)),
        ),
        Span::styled("  Failed: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.failed),
            Style::default().fg(Color::Rgb(220, 50, 47)),
        ),
    ]));
    recommendations.push(Line::from(""));

    // Recommendations based on state
    recommendations.push(Line::from(Span::styled(
        "  Recommendations",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    recommendations.push(Line::from(""));

    if stats.failed > 0 {
        recommendations.push(Line::from(vec![
            Span::styled("  [!] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                format!("{} failed VM(s) need investigation", stats.failed),
                Style::default().fg(Color::White),
            ),
        ]));
    }
    if stats.stopped > 2 {
        recommendations.push(Line::from(vec![
            Span::styled("  [i] ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled(
                format!("{} stopped VMs could be cleaned up", stats.stopped),
                Style::default().fg(Color::White),
            ),
        ]));
    }
    if cpu_now > 75 {
        recommendations.push(Line::from(vec![
            Span::styled("  [!] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                format!("CPU at {}% - consider scaling", cpu_now),
                Style::default().fg(Color::White),
            ),
        ]));
    }
    if stats.failed == 0 && stats.stopped <= 2 && cpu_now <= 75 {
        recommendations.push(Line::from(vec![
            Span::styled("  [ok] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled(
                "No scaling actions needed",
                Style::default().fg(Color::White),
            ),
        ]));
    }

    recommendations.push(Line::from(""));

    // Recent activity from state
    recommendations.push(Line::from(Span::styled(
        "  Recent Activity",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));

    if state.recent_activity.is_empty() {
        recommendations.push(Line::from(Span::styled(
            "  No recent activity",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for activity in state.recent_activity.iter().rev().take(5) {
            let action_color = match activity.action.as_str() {
                "started" | "discovered" => Color::Rgb(50, 205, 50),
                "stopped" | "removed" => Color::Rgb(220, 50, 47),
                "failed" => Color::Rgb(220, 50, 47),
                _ => Color::Rgb(255, 200, 0),
            };
            recommendations.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", activity.elapsed_display()),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!("{}: {}", activity.vm_name, activity.action),
                    Style::default().fg(action_color),
                ),
            ]));
        }
    }

    let rec_widget = Paragraph::new(recommendations).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Activity & Recommendations ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(rec_widget, main_chunks[1]);

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
            "e",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Edit Policy | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "n",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": New Policy | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "t",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Toggle | ", Style::default().fg(Color::Gray)),
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
