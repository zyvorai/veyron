// Autoscaler Policy and Recommendations View

use crate::tui::colors::gradient;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
    Frame,
};


pub fn render(f: &mut Frame, area: Rect) {
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
    header_spans.push(Span::styled(" | ", Style::default().fg(Color::Rgb(128, 128, 128))));
    header_spans.push(Span::styled("Autoscaler", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
    let header_text = Line::from(header_spans);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
        );
    f.render_widget(header, chunks[0]);

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
        .gauge_style(Style::default().fg(gradient::health().at(0.72)).bg(Color::Rgb(40, 35, 55)))
        .percent(72)
        .label("72% (target: 70%)");
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Memory Utilization "),
        )
        .gauge_style(Style::default().fg(Color::Rgb(100, 150, 255)).bg(Color::Rgb(40, 35, 55)))
        .percent(58)
        .label("58% (target: 75%)");
    f.render_widget(mem_gauge, gauge_chunks[1]);

    let pods_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Pod Density "),
        )
        .gauge_style(Style::default().fg(Color::Rgb(50, 205, 50)).bg(Color::Rgb(40, 35, 55)))
        .percent(45)
        .label("73/160 pods");
    f.render_widget(pods_gauge, gauge_chunks[2]);

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .split(chunks[2]);

    // Policies table
    let header_cells = ["Policy", "Pool", "Min", "Max", "Current", "Status"]
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

    let policies = [("cpu-scale-web", "web-pool", "2", "8", "4", "Active"),
        ("cpu-scale-workers", "worker-pool", "3", "10", "5", "Scaling Up"),
        ("mem-scale-cache", "cache-pool", "1", "4", "2", "Active"),
        ("custom-api", "api-pool", "2", "6", "3", "Active"),
        ("burst-handling", "web-pool", "4", "12", "4", "Standby")];

    let rows = policies.iter().map(|(name, pool, min, max, cur, status)| {
        let status_color = match *status {
            "Active" => Color::Rgb(50, 205, 50),
            "Scaling Up" => Color::Rgb(255, 200, 0),
            "Scaling Down" => Color::Rgb(100, 150, 255),
            "Standby" => Color::Gray,
            "Disabled" => Color::Rgb(220, 50, 47),
            _ => Color::Gray,
        };
        Row::new(vec![
            Cell::from(*name).style(Style::default().fg(Color::White)),
            Cell::from(*pool),
            Cell::from(*min),
            Cell::from(*max),
            Cell::from(*cur).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::from(*status).style(Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(24),
        Constraint::Percentage(18),
        Constraint::Percentage(10),
        Constraint::Percentage(10),
        Constraint::Percentage(14),
        Constraint::Percentage(18),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Autoscaling Policies ",
                    Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, main_chunks[0]);

    // Recommendations and recent activity
    let recommendations = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Active Scaling Event",
            Style::default().fg(Color::Rgb(255, 200, 0)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Policy:  ", Style::default().fg(Color::Gray)),
            Span::styled("cpu-scale-workers", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Action:  ", Style::default().fg(Color::Gray)),
            Span::styled("Scale Up 5 -> 6", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
        Line::from(vec![
            Span::styled("  Trigger: ", Style::default().fg(Color::Gray)),
            Span::styled("CPU > 70% for 5m", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  ETA:     ", Style::default().fg(Color::Gray)),
            Span::styled("~2 minutes", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Recommendations",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [1] ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Increase web-pool max to 10", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("      ", Style::default()),
            Span::styled("Peak traffic approaching limit", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [2] ", Style::default().fg(Color::Rgb(222, 115, 86))),
            Span::styled("Enable burst-handling policy", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("      ", Style::default()),
            Span::styled("3 traffic spikes this week", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Recent Activity",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  11:20 ", Style::default().fg(Color::Gray)),
            Span::styled("worker-pool: 3 -> 5", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(vec![
            Span::styled("  08:45 ", Style::default().fg(Color::Gray)),
            Span::styled("web-pool: 3 -> 4", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]),
        Line::from(vec![
            Span::styled("  02:10 ", Style::default().fg(Color::Gray)),
            Span::styled("cache-pool: 3 -> 2", Style::default().fg(Color::Rgb(100, 150, 255))),
        ]),
    ];
    let rec_widget = Paragraph::new(recommendations).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Activity & Recommendations ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(rec_widget, main_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("e", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Edit Policy | ", Style::default().fg(Color::Gray)),
        Span::styled("n", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": New Policy | ", Style::default().fg(Color::Gray)),
        Span::styled("t", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Toggle | ", Style::default().fg(Color::Gray)),
        Span::styled("q", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
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
