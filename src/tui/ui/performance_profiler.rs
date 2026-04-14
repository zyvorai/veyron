// Performance Profiler View

use crate::tui::colors::gradient;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
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

    // Gradient brand header
    let mut header_spans = gradient::brand().text("VMRogue");
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
        "VM: web-server-01",
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

    // Resource gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    let gauges = [
        ("CPU", 45, gradient::health().at(0.45)),
        ("Memory", 62, Color::Rgb(100, 150, 255)),
        ("Disk I/O", 28, Color::Rgb(50, 205, 50)),
        ("Network", 51, Color::Rgb(255, 200, 0)),
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

    // Process/thread table
    let header_cells = ["Process", "CPU%", "Mem%", "Threads", "State"]
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

    let processes = [
        ("nginx", "12.4", "8.2", "16", "Running"),
        ("postgres", "18.6", "32.1", "24", "Running"),
        ("node-exporter", "2.1", "1.8", "4", "Running"),
        ("qemu-system", "8.3", "15.4", "8", "Running"),
        ("systemd", "0.4", "1.2", "1", "Running"),
        ("sshd", "0.1", "0.8", "2", "Sleeping"),
        ("cron", "0.0", "0.3", "1", "Sleeping"),
    ];

    let rows = processes.iter().map(|(name, cpu, mem, threads, state)| {
        let state_color = match *state {
            "Running" => Color::Rgb(50, 205, 50),
            "Sleeping" => Color::Gray,
            _ => Color::Rgb(255, 200, 0),
        };
        Row::new(vec![
            Cell::from(*name),
            Cell::from(*cpu),
            Cell::from(*mem),
            Cell::from(*threads),
            Cell::from(*state).style(Style::default().fg(state_color)),
        ])
        .height(1)
    });

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
                    " Processes ",
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, main_chunks[0]);

    // Performance insights
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
            Span::styled("  [INFO] ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled(
                "CPU: Normal utilization pattern",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                "Memory: postgres using 32% of VM RAM",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  [INFO] ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled(
                "Disk: Low I/O wait time (0.8%)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  [INFO] ", Style::default().fg(Color::Rgb(100, 150, 255))),
            Span::styled(
                "Network: Steady throughput",
                Style::default().fg(Color::White),
            ),
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
            Span::styled(
                "None detected",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Watch:     ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Memory pressure from postgres",
                Style::default().fg(Color::Rgb(255, 200, 0)),
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
            Span::styled(" (5m window)", Style::default().fg(Color::Gray)),
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
