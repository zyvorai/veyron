// Custom Metrics Display View

use crate::tui::colors::gradient;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
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
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(" | ", Style::default().fg(Color::Rgb(128, 128, 128))));
    header_spans.push(Span::styled("Custom Metrics", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
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
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
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

    let metrics = vec![
        ("vm_cpu_utilization", "52.3", "%", "+2.1%", "OK"),
        ("vm_memory_pressure", "67.8", "%", "+0.5%", "OK"),
        ("vm_disk_iops", "4,521", "ops/s", "-12.3%", "OK"),
        ("vm_network_throughput", "248.5", "MiB/s", "+8.7%", "OK"),
        ("vm_boot_time_avg", "18.2", "sec", "+1.4s", "WARN"),
        ("vm_migration_success", "98.5", "%", "-0.2%", "OK"),
        ("snapshot_create_time", "12.8", "sec", "+3.1s", "WARN"),
        ("api_latency_p99", "142", "ms", "+22ms", "ALERT"),
        ("storage_iops_avg", "8,240", "ops/s", "-5.1%", "OK"),
        ("network_packet_loss", "0.02", "%", "+0.01%", "OK"),
    ];

    let rows = metrics.iter().map(|(name, val, unit, trend, alert)| {
        let alert_color = match *alert {
            "OK" => Color::Rgb(50, 205, 50),
            "WARN" => Color::Rgb(255, 200, 0),
            "ALERT" => Color::Rgb(220, 50, 47),
            _ => Color::Gray,
        };
        let trend_color = if trend.starts_with('+') {
            Color::Rgb(255, 200, 0)
        } else {
            Color::Rgb(50, 205, 50)
        };
        Row::new(vec![
            Cell::from(*name).style(Style::default().fg(Color::Rgb(222, 115, 86))),
            Cell::from(*val).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::from(*unit),
            Cell::from(*trend).style(Style::default().fg(trend_color)),
            Cell::from(*alert).style(Style::default().fg(alert_color).add_modifier(Modifier::BOLD)),
        ])
        .height(1)
    });

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
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, content_chunks[0]);

    // Metric configuration
    let config_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Active Dashboards",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [1] ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("VM Performance Overview", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [2] ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Storage Analytics", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [3] ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Network Metrics", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Alert Thresholds",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  api_latency_p99 > 100ms  ", Style::default().fg(Color::White)),
            Span::styled("TRIGGERED", Style::default().fg(Color::Rgb(220, 50, 47)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  vm_boot_time_avg > 15s   ", Style::default().fg(Color::White)),
            Span::styled("TRIGGERED", Style::default().fg(Color::Rgb(220, 50, 47)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  vm_cpu_utilization > 80% ", Style::default().fg(Color::White)),
            Span::styled("OK", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Refresh: ", Style::default().fg(Color::Gray)),
            Span::styled("every 30s", Style::default().fg(Color::White)),
        ]),
    ];
    let config_widget = Paragraph::new(config_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Configuration ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(config_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("a", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Add Metric | ", Style::default().fg(Color::Gray)),
        Span::styled("t", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Thresholds | ", Style::default().fg(Color::Gray)),
        Span::styled("r", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Refresh | ", Style::default().fg(Color::Gray)),
        Span::styled("q", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
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
