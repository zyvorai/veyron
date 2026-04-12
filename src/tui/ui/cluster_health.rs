// Cluster Health Dashboard View

use crate::tui::colors::gradient;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};


pub fn render(f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    // Gradient brand header
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(" | ", Style::default().fg(Color::Rgb(128, 128, 128))));
    header_spans.push(Span::styled("Cluster Health Dashboard", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
    let header_text = Line::from(header_spans);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
        );
    f.render_widget(header, chunks[0]);

    // Overall status
    let status_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Overall Status:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                "HEALTHY",
                Style::default()
                    .fg(Color::Rgb(50, 205, 50))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  API Server:       ", Style::default().fg(Color::Gray)),
            Span::styled("Responsive (12ms)", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(vec![
            Span::styled("  etcd:             ", Style::default().fg(Color::Gray)),
            Span::styled("Healthy (3/3 members)", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(vec![
            Span::styled("  KubeVirt:         ", Style::default().fg(Color::Gray)),
            Span::styled("v1.2.0 - Operational", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
    ];
    let status_widget = Paragraph::new(status_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Cluster Status ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(status_widget, chunks[1]);

    // Resource gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[2]);

    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" CPU "),
        )
        .gauge_style(Style::default().fg(gradient::health().at(0.52)).bg(Color::Rgb(40, 35, 55)))
        .percent(52)
        .label("52%");
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Memory "),
        )
        .gauge_style(Style::default().fg(Color::Rgb(100, 150, 255)).bg(Color::Rgb(40, 35, 55)))
        .percent(67)
        .label("67%");
    f.render_widget(mem_gauge, gauge_chunks[1]);

    let storage_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Storage "),
        )
        .gauge_style(Style::default().fg(Color::Rgb(255, 200, 0)).bg(Color::Rgb(40, 35, 55)))
        .percent(78)
        .label("78%");
    f.render_widget(storage_gauge, gauge_chunks[2]);

    // Node status
    let node_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[3]);

    let node_status = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  node-1  ", Style::default().fg(Color::White)),
            Span::styled("Ready  ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("control-plane  ", Style::default().fg(Color::Gray)),
            Span::styled("CPU: 45%  Mem: 62%", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  node-2  ", Style::default().fg(Color::White)),
            Span::styled("Ready  ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("worker         ", Style::default().fg(Color::Gray)),
            Span::styled("CPU: 72%  Mem: 58%", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  node-3  ", Style::default().fg(Color::White)),
            Span::styled("Ready  ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("worker         ", Style::default().fg(Color::Gray)),
            Span::styled("CPU: 38%  Mem: 41%", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  node-4  ", Style::default().fg(Color::White)),
            Span::styled("NotReady  ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled("worker      ", Style::default().fg(Color::Gray)),
            Span::styled("Unreachable", Style::default().fg(Color::Rgb(220, 50, 47))),
        ]),
    ];
    let nodes_widget = Paragraph::new(node_status).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Node Health ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(nodes_widget, node_chunks[0]);

    let alerts = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  [CRIT] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled("node-4 unreachable for 5m", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("Storage usage above 75%", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [INFO] ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled("2 VMs pending scheduling", Style::default().fg(Color::White)),
        ]),
    ];
    let alerts_widget = Paragraph::new(alerts).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Active Alerts ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(alerts_widget, node_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("r", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Refresh | ", Style::default().fg(Color::Gray)),
        Span::styled("n", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Nodes | ", Style::default().fg(Color::Gray)),
        Span::styled("a", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Alerts | ", Style::default().fg(Color::Gray)),
        Span::styled("q", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Back", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
    );
    f.render_widget(help, chunks[4]);
}
