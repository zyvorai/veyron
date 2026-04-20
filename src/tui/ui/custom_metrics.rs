// Custom Metrics Display View

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

/// Format a history buffer into a trend string (change from previous to latest)
fn trend_from_history(history: &[u64]) -> (String, bool) {
    if history.len() < 2 {
        return ("--".to_string(), false);
    }
    let prev = history[history.len() - 2] as i64;
    let curr = *history.last().unwrap() as i64;
    let diff = curr - prev;
    if diff > 0 {
        (format!("+{}%", diff), true)
    } else if diff < 0 {
        (format!("{}%", diff), false)
    } else {
        ("0%".to_string(), false)
    }
}

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
        "Custom Metrics",
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

    // Metrics content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    // Metrics table
    let header_cells = ["Metric", "Value", "Unit", "Trend", "Alert"]
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

    let stats = state.get_stats();
    let cpu_val = state.cpu_history.last().copied().unwrap_or(0);
    let mem_val = state.memory_history.last().copied().unwrap_or(0);
    let disk_val = state.disk_history.last().copied().unwrap_or(0);
    let net_val = state.network_history.last().copied().unwrap_or(0);

    let (cpu_trend, cpu_up) = trend_from_history(&state.cpu_history);
    let (mem_trend, mem_up) = trend_from_history(&state.memory_history);
    let (disk_trend, disk_up) = trend_from_history(&state.disk_history);
    let (net_trend, net_up) = trend_from_history(&state.network_history);

    let cpu_alert = if cpu_val > 80 { "WARN" } else { "OK" };
    let mem_alert = if mem_val > 80 { "WARN" } else { "OK" };
    let disk_alert = if disk_val > 80 { "WARN" } else { "OK" };
    let net_alert = if net_val > 80 { "WARN" } else { "OK" };

    let metrics: Vec<(&str, String, &str, String, bool, &str)> = vec![
        (
            "vm_cpu_utilization",
            format!("{}", cpu_val),
            "%",
            cpu_trend,
            cpu_up,
            cpu_alert,
        ),
        (
            "vm_memory_pressure",
            format!("{}", mem_val),
            "%",
            mem_trend,
            mem_up,
            mem_alert,
        ),
        (
            "vm_disk_io",
            format!("{}", disk_val),
            "%",
            disk_trend,
            disk_up,
            disk_alert,
        ),
        (
            "vm_network_throughput",
            format!("{}", net_val),
            "%",
            net_trend,
            net_up,
            net_alert,
        ),
        (
            "vm_total_count",
            format!("{}", stats.total),
            "vms",
            "--".to_string(),
            false,
            "OK",
        ),
        (
            "vm_running_count",
            format!("{}", stats.running),
            "vms",
            "--".to_string(),
            false,
            if stats.running < stats.total {
                "WARN"
            } else {
                "OK"
            },
        ),
        (
            "vm_failed_count",
            format!("{}", stats.failed),
            "vms",
            "--".to_string(),
            false,
            if stats.failed > 0 { "ALERT" } else { "OK" },
        ),
        (
            "vm_stopped_count",
            format!("{}", stats.stopped),
            "vms",
            "--".to_string(),
            false,
            "OK",
        ),
        (
            "node_count",
            format!("{}", state.nodes.len()),
            "nodes",
            "--".to_string(),
            false,
            "OK",
        ),
        (
            "snapshot_count",
            format!("{}", state.snapshots.len()),
            "snaps",
            "--".to_string(),
            false,
            "OK",
        ),
    ];

    let rows: Vec<Row> = metrics
        .iter()
        .map(|(name, val, unit, trend, is_up, alert)| {
            let alert_color = match *alert {
                "OK" => Color::Rgb(50, 205, 50),
                "WARN" => Color::Rgb(255, 200, 0),
                "ALERT" => Color::Rgb(220, 50, 47),
                _ => Color::Gray,
            };
            let trend_color = if *is_up {
                Color::Rgb(255, 200, 0)
            } else {
                Color::Rgb(50, 205, 50)
            };
            Row::new(vec![
                Cell::from(*name).style(Style::default().fg(Color::Rgb(222, 115, 86))),
                Cell::from(val.as_str()).style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(*unit),
                Cell::from(trend.as_str()).style(Style::default().fg(trend_color)),
                Cell::from(*alert).style(
                    Style::default()
                        .fg(alert_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(16),
        Constraint::Percentage(14),
        Constraint::Percentage(18),
        Constraint::Percentage(14),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Metrics ",
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, content_chunks[0]);

    // Metric configuration - show history sparklines as text
    let cpu_spark: String = state
        .cpu_history
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|v| {
            if *v > 75 {
                '#'
            } else if *v > 50 {
                '='
            } else if *v > 25 {
                '-'
            } else {
                '.'
            }
        })
        .collect();
    let mem_spark: String = state
        .memory_history
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|v| {
            if *v > 75 {
                '#'
            } else if *v > 50 {
                '='
            } else if *v > 25 {
                '-'
            } else {
                '.'
            }
        })
        .collect();
    let disk_spark: String = state
        .disk_history
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|v| {
            if *v > 75 {
                '#'
            } else if *v > 50 {
                '='
            } else if *v > 25 {
                '-'
            } else {
                '.'
            }
        })
        .collect();
    let net_spark: String = state
        .network_history
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|v| {
            if *v > 75 {
                '#'
            } else if *v > 50 {
                '='
            } else if *v > 25 {
                '-'
            } else {
                '.'
            }
        })
        .collect();

    let config_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Trend History (last 10)",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  CPU:  [", Style::default().fg(Color::Gray)),
            Span::styled(cpu_spark, Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled("]", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  Mem:  [", Style::default().fg(Color::Gray)),
            Span::styled(mem_spark, Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled("]", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  Disk: [", Style::default().fg(Color::Gray)),
            Span::styled(disk_spark, Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("]", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  Net:  [", Style::default().fg(Color::Gray)),
            Span::styled(net_spark, Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("]", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Alert Thresholds",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  CPU > 80%        (now: {}%)  ", cpu_val),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                if cpu_val > 80 { "TRIGGERED" } else { "OK" },
                Style::default()
                    .fg(if cpu_val > 80 {
                        Color::Rgb(220, 50, 47)
                    } else {
                        Color::Rgb(50, 205, 50)
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  Memory > 80%     (now: {}%)  ", mem_val),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                if mem_val > 80 { "TRIGGERED" } else { "OK" },
                Style::default()
                    .fg(if mem_val > 80 {
                        Color::Rgb(220, 50, 47)
                    } else {
                        Color::Rgb(50, 205, 50)
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  Failed VMs > 0   (now: {})   ", stats.failed),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                if stats.failed > 0 { "TRIGGERED" } else { "OK" },
                Style::default()
                    .fg(if stats.failed > 0 {
                        Color::Rgb(220, 50, 47)
                    } else {
                        Color::Rgb(50, 205, 50)
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Refresh: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("every {}s", state.refresh_interval),
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    let config_widget = Paragraph::new(config_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Configuration ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(config_widget, content_chunks[1]);

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
            "a",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Add Metric | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "t",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Thresholds | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Refresh | ", Style::default().fg(Color::Gray)),
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
