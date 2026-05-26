// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Enhanced Dashboard View - Comprehensive system overview with charts and monitoring
use crate::tui::{
    colors::gradient,
    colors::tui as colors,
    config::TuiConfig,
    state::AppState,
    widgets::{BarChart, MultiGaugePanel, ResourceGauge, SparklineChart},
};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, state: &AppState, _config: &TuiConfig) {
    let size = f.area();

    // Main layout with optional stats bar
    let main_chunks = if state.show_stats_bar {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(2), // Stats bar
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(size)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(size)
    };

    let (header_chunk, content_chunk, footer_chunk) = if state.show_stats_bar {
        (main_chunks[0], main_chunks[2], main_chunks[3])
    } else {
        (main_chunks[0], main_chunks[1], main_chunks[2])
    };

    // Render header
    render_header(f, state, header_chunk);

    // Render stats bar if enabled
    if state.show_stats_bar {
        render_stats_bar(f, state, main_chunks[1]);
    }

    // Content area - 3-column layout
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33), // Left: VM Stats & Resource Gauges
            Constraint::Percentage(34), // Center: Charts & Trends
            Constraint::Percentage(33), // Right: Recent Activity & Quick Actions
        ])
        .split(content_chunk);

    // Left column: VM Stats + Resource Usage
    render_left_column(f, state, content_chunks[0]);

    // Center column: Charts and Trends
    render_center_column(f, state, content_chunks[1]);

    // Right column: Activity + Quick Actions
    render_right_column(f, state, content_chunks[2]);

    // Footer
    render_footer(f, state, footer_chunk);
}

fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
    // Build header with gradient brand name
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(" - ", Style::default().fg(colors::TEXT_MUTED)));
    header_spans.push(Span::styled(
        "KubeVirt VM Manager",
        Style::default().fg(colors::TEXT),
    ));
    header_spans.push(Span::styled(
        "  │  ",
        Style::default().fg(colors::TEXT_MUTED),
    ));
    // Gradient "Dashboard" label
    let mut dash_spans = gradient::sunset().text("📊 Dashboard");
    header_spans.append(&mut dash_spans);
    header_spans.push(Span::styled(
        format!(
            ": {} VMs • {} Running",
            state.vms.len(),
            state.get_stats().running
        ),
        Style::default().fg(colors::TEXT),
    ));
    header_spans.push(Span::styled(
        "  │  1-6: Views",
        Style::default().fg(colors::TEXT_MUTED),
    ));
    let header_text = Line::from(header_spans);

    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );

    f.render_widget(header, area);
}

fn render_stats_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let stats = state.get_stats();

    let health_icon = if stats.running == 0 && stats.total > 0 {
        "🔴"
    } else if stats.failed > 0 {
        "🟠"
    } else if stats.starting > 0 {
        "🟡"
    } else if stats.running > 0 {
        "🟢"
    } else {
        "⚪"
    };

    let health_text = if stats.running == 0 && stats.total > 0 {
        "All Stopped"
    } else if stats.failed > 0 {
        "Failures"
    } else if stats.starting > 0 {
        "Starting"
    } else if stats.running > 0 {
        "Healthy"
    } else {
        "No VMs"
    };

    let stats_line = Line::from(vec![
        Span::styled("📊 ", Style::default().fg(colors::ORANGE)),
        Span::styled("VMs: ", Style::default().fg(colors::TEXT)),
        Span::styled(
            format!("{} ", stats.total),
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("🟢 ", Style::default().fg(colors::SUCCESS)),
        Span::styled(
            format!("{} ", stats.running),
            Style::default()
                .fg(colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("⏸  ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            format!("{} ", stats.stopped),
            Style::default()
                .fg(colors::TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("🟡 ", Style::default().fg(colors::WARNING)),
        Span::styled(
            format!("{} ", stats.starting),
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("🔴 ", Style::default().fg(colors::ERROR)),
        Span::styled(
            format!("{} ", stats.failed),
            Style::default()
                .fg(colors::ERROR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            format!("{} {} ", health_icon, health_text),
            Style::default()
                .fg(colors::TEXT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("🌐 ", Style::default().fg(colors::INFO)),
        Span::styled(
            &state.namespace,
            Style::default()
                .fg(colors::LIGHT_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let paragraph = Paragraph::new(stats_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER)),
    );

    f.render_widget(paragraph, area);
}

fn render_left_column(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // VM Stats
            Constraint::Min(0),     // Resource Gauges
            Constraint::Length(5),  // Cluster Health
        ])
        .split(area);

    // VM Statistics Box
    render_vm_stats_box(f, state, chunks[0]);

    // Resource Usage Gauges
    render_resource_gauges(f, state, chunks[1]);

    // Cluster Health
    render_cluster_health(f, state, chunks[2]);
}

fn render_cluster_health(f: &mut Frame, state: &AppState, area: Rect) {
    let stats = state.get_stats();
    let (health_icon, health_text, health_color) = if stats.total == 0 {
        ("⚪", "No VMs", colors::TEXT_MUTED)
    } else if stats.failed > 0 {
        ("🔴", "Degraded", colors::ERROR)
    } else if stats.running == stats.total {
        ("🟢", "Healthy", colors::SUCCESS)
    } else if stats.running > 0 {
        ("🟡", "Partial", colors::WARNING)
    } else {
        ("⚪", "Idle", colors::TEXT_MUTED)
    };

    let text = vec![Line::from(vec![
        Span::styled(format!("{} ", health_icon), Style::default()),
        Span::styled(
            health_text,
            Style::default()
                .fg(health_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ns:{}", state.namespace),
            Style::default().fg(colors::TEXT_MUTED),
        ),
    ])];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::BORDER))
        .title(Span::styled(
            "🏥 Cluster Health",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);
    f.render_widget(paragraph, area);
}

fn render_vm_stats_box(f: &mut Frame, state: &AppState, area: Rect) {
    let stats = state.get_stats();

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("💻 Total VMs:     ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{}", stats.total),
                Style::default()
                    .fg(colors::LIGHT_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("🟢 Running:       ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{}", stats.running),
                Style::default()
                    .fg(colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("⏸  Stopped:       ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{}", stats.stopped),
                Style::default()
                    .fg(colors::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("🟡 Starting:      ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{}", stats.starting),
                Style::default()
                    .fg(colors::WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("🔴 Failed:        ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{}", stats.failed),
                Style::default()
                    .fg(colors::ERROR)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("📸 Snapshots:     ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{}", state.snapshots.len()),
                Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let title = Span::styled(
        "📊 VM Statistics",
        Style::default()
            .fg(colors::ORANGE)
            .add_modifier(Modifier::BOLD),
    );
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title(title),
        )
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_resource_gauges(f: &mut Frame, state: &AppState, area: Rect) {
    let cpu_current = state.cpu_history.last().copied().unwrap_or(0) as f64;
    let memory_current = state.memory_history.last().copied().unwrap_or(0) as f64;
    let disk_current = state.disk_history.last().copied().unwrap_or(0) as f64;
    let network_current = state.network_history.last().copied().unwrap_or(0) as f64;

    let panel = MultiGaugePanel::new("💾 Resource Usage")
        .add_gauge(ResourceGauge::new("CPU", cpu_current, 100.0, "%"))
        .add_gauge(ResourceGauge::new("Memory", memory_current, 100.0, "%"))
        .add_gauge(ResourceGauge::new("Disk", disk_current, 100.0, "%"))
        .add_gauge(ResourceGauge::new("Network", network_current, 100.0, "%"));

    panel.render(f, area);
}

fn render_center_column(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20), // Status Distribution
            Constraint::Percentage(20), // CPU Trend
            Constraint::Percentage(20), // Memory Trend
            Constraint::Percentage(20), // Disk Trend
            Constraint::Percentage(20), // Network Trend
        ])
        .split(area);

    // Status Distribution Bar Chart
    render_status_chart(f, state, chunks[0]);

    // CPU Usage Sparkline
    let cpu_chart =
        SparklineChart::new("📈 CPU Usage (30min)", state.cpu_history.clone()).with_max(100);
    cpu_chart.render(f, chunks[1]);

    // Memory Usage Sparkline
    let memory_chart =
        SparklineChart::new("📉 Memory Usage (30min)", state.memory_history.clone()).with_max(100);
    memory_chart.render(f, chunks[2]);

    // Disk Usage Sparkline
    let disk_chart =
        SparklineChart::new("💿 Disk Usage (30min)", state.disk_history.clone()).with_max(100);
    disk_chart.render(f, chunks[3]);

    // Network Usage Sparkline
    let network_chart =
        SparklineChart::new("🌐 Network Usage (30min)", state.network_history.clone())
            .with_max(100);
    network_chart.render(f, chunks[4]);
}

fn render_status_chart(f: &mut Frame, state: &AppState, area: Rect) {
    let stats = state.get_stats();

    let chart = BarChart::new("📊 VM Status Distribution")
        .add_bar("Run", stats.running as u64)
        .add_bar("Stop", stats.stopped as u64)
        .add_bar("Start", stats.starting as u64)
        .add_bar("Fail", stats.failed as u64)
        .bar_width(8)
        .bar_gap(2);

    chart.render(f, area);
}

fn render_right_column(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Recent Activity
            Constraint::Percentage(50), // Quick Actions
        ])
        .split(area);

    // Recent Activity
    render_recent_activity(f, state, chunks[0]);

    // Quick Actions
    render_quick_actions(f, state, chunks[1]);
}

fn render_recent_activity(f: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = if state.recent_activity.is_empty() {
        vec![ListItem::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  No activity yet",
                Style::default()
                    .fg(colors::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            )]),
        ])]
    } else {
        state
            .recent_activity
            .iter()
            .rev()
            .take(8)
            .map(|event| {
                let icon_color = match event.action.as_str() {
                    "started" => colors::SUCCESS,
                    "stopped" => colors::WARNING,
                    "failed" => colors::ERROR,
                    "starting" => colors::WARNING,
                    "removed" | "deleted" => colors::ERROR,
                    _ => colors::INFO,
                };
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("{} ", event.icon), Style::default().fg(icon_color)),
                        Span::styled(
                            &event.vm_name,
                            Style::default()
                                .fg(colors::TEXT)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" {}", event.action),
                            Style::default().fg(colors::TEXT_MUTED),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("   ", Style::default()),
                        Span::styled(
                            event.elapsed_display(),
                            Style::default()
                                .fg(colors::TEXT_MUTED)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]),
                ])
            })
            .collect()
    };

    let title = Span::styled(
        "⏱  Recent Activity",
        Style::default()
            .fg(colors::ORANGE)
            .add_modifier(Modifier::BOLD),
    );
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER))
            .title(title),
    );

    f.render_widget(list, area);
}

fn render_quick_actions(f: &mut Frame, _state: &AppState, area: Rect) {
    fn nav_line<'a>(key: &'a str, icon: &'a str, label: &'a str) -> Line<'a> {
        Line::from(vec![
            Span::styled(
                format!(" {} ", key),
                Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{} ", icon), Style::default().fg(colors::ORANGE)),
            Span::styled(label, Style::default().fg(colors::TEXT)),
        ])
    }

    let text = vec![
        Line::from(""),
        nav_line("2", "💻", "VM List"),
        nav_line("3", "📸", "Snapshots"),
        nav_line("4", "⚙️", "Profiles"),
        nav_line("5", "🏗️", "Blueprints"),
        nav_line("6", "📋", "Activity"),
        nav_line("7", "🖥️", "Nodes"),
        nav_line("8", "⚡", "Events"),
        nav_line("9", "💚", "Health"),
        nav_line("0", "🗺️", "Topology"),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " Ctrl+P ",
                Style::default()
                    .fg(colors::LIGHT_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("All views", Style::default().fg(colors::TEXT_MUTED)),
        ]),
    ];

    let title = Span::styled(
        "🚀 Quick Navigation",
        Style::default()
            .fg(colors::ORANGE)
            .add_modifier(Modifier::BOLD),
    );
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title(title),
        )
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, state: &AppState, area: Rect) {
    let footer_line = Line::from(vec![
        Span::styled("⌨  ", Style::default().fg(colors::ORANGE)),
        Span::styled(
            "1-0",
            Style::default()
                .fg(colors::INFO)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Jump", Style::default().fg(colors::TEXT)),
        Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "r",
            Style::default()
                .fg(colors::INFO)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Refresh", Style::default().fg(colors::TEXT)),
        Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "i",
            Style::default()
                .fg(colors::INFO)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Stats Bar", Style::default().fg(colors::TEXT)),
        Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "?",
            Style::default()
                .fg(colors::INFO)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Help", Style::default().fg(colors::TEXT)),
        Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "q",
            Style::default()
                .fg(colors::INFO)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Quit", Style::default().fg(colors::TEXT)),
        Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("⏱  ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            format!("{}", state.last_refresh.format("%H:%M:%S")),
            Style::default()
                .fg(colors::TEXT_MUTED)
                .add_modifier(Modifier::ITALIC),
        ),
    ]);

    let footer = Paragraph::new(footer_line)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );

    f.render_widget(footer, area);
}
