// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Resource Forecast Dashboard View

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
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
        "Resource Forecast",
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

    // --- Derive current utilization from state ---
    let stats = state.get_stats();
    let total_vms = stats.total.max(1);

    // Current CPU/memory/disk from latest history data points
    let cpu_now = state.cpu_history.last().copied().unwrap_or(0).min(100) as u16;
    let mem_now = state.memory_history.last().copied().unwrap_or(0).min(100) as u16;
    let disk_now = state.disk_history.last().copied().unwrap_or(0).min(100) as u16;

    // Current utilization gauges
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
                .title(" CPU Now "),
        )
        .gauge_style(
            Style::default()
                .fg(gradient::health().at(cpu_now as f64 / 100.0))
                .bg(Color::Rgb(40, 35, 55)),
        )
        .percent(cpu_now)
        .label(format!("{}% ({} VMs)", cpu_now, total_vms));
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Memory Now "),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(100, 150, 255))
                .bg(Color::Rgb(40, 35, 55)),
        )
        .percent(mem_now)
        .label(format!("{}%", mem_now));
    f.render_widget(mem_gauge, gauge_chunks[1]);

    let storage_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(" Disk Now "),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(255, 200, 0))
                .bg(Color::Rgb(40, 35, 55)),
        )
        .percent(disk_now)
        .label(format!("{}%", disk_now));
    f.render_widget(storage_gauge, gauge_chunks[2]);

    // --- Compute simple growth projections ---
    // Compute trend from history: compare first half average vs second half average
    let compute_trend = |history: &[u64]| -> f64 {
        if history.len() < 4 {
            return 0.0;
        }
        let mid = history.len() / 2;
        let first_half: f64 = history[..mid].iter().map(|&v| v as f64).sum::<f64>() / mid as f64;
        let second_half: f64 =
            history[mid..].iter().map(|&v| v as f64).sum::<f64>() / (history.len() - mid) as f64;
        second_half - first_half
    };

    let cpu_trend = compute_trend(&state.cpu_history);
    let mem_trend = compute_trend(&state.memory_history);
    let disk_trend = compute_trend(&state.disk_history);

    // Project values at 7, 30, 90 days (trend per refresh cycle, rough scaling)
    let project = |current: u16, trend: f64, days: u16| -> u16 {
        let projected = current as f64 + trend * days as f64 / 7.0;
        (projected.round() as u16).min(100)
    };

    let forecast_color = |pct: u16| -> Color {
        if pct >= 90 {
            Color::Rgb(220, 50, 47)
        } else if pct >= 75 {
            Color::Rgb(255, 200, 0)
        } else {
            Color::White
        }
    };

    let delta_color = |delta: i16| -> Color {
        if delta >= 15 {
            Color::Rgb(220, 50, 47)
        } else if delta >= 5 {
            Color::Rgb(255, 200, 0)
        } else {
            Color::White
        }
    };

    // Forecast details
    let forecast_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    let cpu_7 = project(cpu_now, cpu_trend, 7);
    let cpu_30 = project(cpu_now, cpu_trend, 30);
    let cpu_90 = project(cpu_now, cpu_trend, 90);
    let mem_7 = project(mem_now, mem_trend, 7);
    let mem_30 = project(mem_now, mem_trend, 30);
    let mem_90 = project(mem_now, mem_trend, 90);
    let disk_7 = project(disk_now, disk_trend, 7);
    let disk_30 = project(disk_now, disk_trend, 30);
    let disk_90 = project(disk_now, disk_trend, 90);

    let mut predictions: Vec<Line> = Vec::new();
    predictions.push(Line::from(""));
    predictions.push(Line::from(Span::styled(
        "  Capacity Predictions",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    predictions.push(Line::from(""));

    // CPU predictions
    predictions.push(Line::from(Span::styled(
        format!("  CPU (current {}%)", cpu_now),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    for (label, val, base) in [
        ("7 days", cpu_7, cpu_now),
        ("30 days", cpu_30, cpu_now),
        ("90 days", cpu_90, cpu_now),
    ] {
        let delta = val as i16 - base as i16;
        let warning = if val >= 90 { " CAPACITY WARNING" } else { "" };
        predictions.push(Line::from(vec![
            Span::styled(
                format!("    {:<7} ", label),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                format!("{}%", val),
                Style::default().fg(forecast_color(val)),
            ),
            Span::styled(
                format!(" ({:+}%){}", delta, warning),
                Style::default().fg(delta_color(delta)),
            ),
        ]));
    }

    predictions.push(Line::from(""));

    // Memory predictions
    predictions.push(Line::from(Span::styled(
        format!("  Memory (current {}%)", mem_now),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    for (label, val, base) in [
        ("7 days", mem_7, mem_now),
        ("30 days", mem_30, mem_now),
        ("90 days", mem_90, mem_now),
    ] {
        let delta = val as i16 - base as i16;
        let warning = if val >= 90 { " CAPACITY WARNING" } else { "" };
        predictions.push(Line::from(vec![
            Span::styled(
                format!("    {:<7} ", label),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                format!("{}%", val),
                Style::default().fg(forecast_color(val)),
            ),
            Span::styled(
                format!(" ({:+}%){}", delta, warning),
                Style::default().fg(delta_color(delta)),
            ),
        ]));
    }

    predictions.push(Line::from(""));

    // Disk predictions
    predictions.push(Line::from(Span::styled(
        format!("  Disk (current {}%)", disk_now),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    for (label, val, base) in [
        ("7 days", disk_7, disk_now),
        ("30 days", disk_30, disk_now),
        ("90 days", disk_90, disk_now),
    ] {
        let delta = val as i16 - base as i16;
        let warning = if val >= 90 { " CRITICAL" } else { "" };
        predictions.push(Line::from(vec![
            Span::styled(
                format!("    {:<7} ", label),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                format!("{}%", val),
                Style::default().fg(forecast_color(val)),
            ),
            Span::styled(
                format!(" ({:+}%){}", delta, warning),
                Style::default().fg(delta_color(delta)),
            ),
        ]));
    }

    let predictions_widget = Paragraph::new(predictions).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Forecast ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(predictions_widget, forecast_chunks[0]);

    // Recommendations panel - derived from actual data
    let mut recommendations: Vec<Line> = Vec::new();
    recommendations.push(Line::from(""));
    recommendations.push(Line::from(Span::styled(
        "  Recommendations",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    recommendations.push(Line::from(""));

    let mut rec_num = 0u32;

    if disk_90 >= 90 {
        rec_num += 1;
        recommendations.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", rec_num),
                Style::default().fg(Color::Rgb(220, 50, 47)),
            ),
            Span::styled(
                "Expand storage capacity soon",
                Style::default().fg(Color::White),
            ),
        ]));
        recommendations.push(Line::from(vec![Span::styled(
            format!("     Disk projected to reach {}% in 90 days", disk_90),
            Style::default().fg(Color::Gray),
        )]));
        recommendations.push(Line::from(""));
    }

    if cpu_90 >= 85 {
        rec_num += 1;
        recommendations.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", rec_num),
                Style::default().fg(Color::Rgb(255, 200, 0)),
            ),
            Span::styled(
                "Plan CPU capacity increase",
                Style::default().fg(Color::White),
            ),
        ]));
        recommendations.push(Line::from(vec![Span::styled(
            format!("     CPU projected to reach {}% in 90 days", cpu_90),
            Style::default().fg(Color::Gray),
        )]));
        recommendations.push(Line::from(""));
    }

    if stats.stopped > 0 {
        rec_num += 1;
        recommendations.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", rec_num),
                Style::default().fg(Color::Rgb(100, 150, 255)),
            ),
            Span::styled("Review stopped VMs", Style::default().fg(Color::White)),
        ]));
        recommendations.push(Line::from(vec![Span::styled(
            format!("     {} VMs are stopped, consider cleanup", stats.stopped),
            Style::default().fg(Color::Gray),
        )]));
        recommendations.push(Line::from(""));
    }

    if stats.failed > 0 {
        rec_num += 1;
        recommendations.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", rec_num),
                Style::default().fg(Color::Rgb(220, 50, 47)),
            ),
            Span::styled("Investigate failed VMs", Style::default().fg(Color::White)),
        ]));
        recommendations.push(Line::from(vec![Span::styled(
            format!("     {} VMs in failed state need attention", stats.failed),
            Style::default().fg(Color::Gray),
        )]));
        recommendations.push(Line::from(""));
    }

    if rec_num == 0 {
        recommendations.push(Line::from(Span::styled(
            "  No action items - cluster looks healthy",
            Style::default().fg(Color::Rgb(50, 205, 50)),
        )));
        recommendations.push(Line::from(""));
    }

    // VM growth info
    recommendations.push(Line::from(Span::styled(
        "  VM Growth",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    recommendations.push(Line::from(vec![
        Span::styled("  Current VMs: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.total),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    recommendations.push(Line::from(vec![
        Span::styled("  Running:     ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.running),
            Style::default().fg(Color::Rgb(50, 205, 50)),
        ),
    ]));

    let rec_widget = Paragraph::new(recommendations).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Actions & Growth ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(rec_widget, forecast_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "p",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Period | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Refresh | ", Style::default().fg(Color::Gray)),
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
