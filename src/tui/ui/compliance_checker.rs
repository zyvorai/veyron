// Compliance Checker View

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
        "Compliance Checker",
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

    // --- Derive compliance checks from state.vms ---
    let total_vms = state.vms.len();

    // Check each VM for compliance issues
    let mut has_resource_limits = 0usize;
    let mut has_eviction_strategy = 0usize; // VMs that are Running considered compliant
    let mut has_rng_device = 0usize; // VMs with disk info (non-"None") considered compliant
    let mut has_node_assignment = 0usize;

    for vm in &state.vms {
        // Resource limits: VM has explicit CPU and memory set (not defaults)
        if vm.cpu != "1 core" || vm.memory != "Unknown" {
            has_resource_limits += 1;
        }
        // Eviction strategy: Running or Stopped VMs are considered to have one
        if vm.status == "Running" || vm.status == "Stopped" {
            has_eviction_strategy += 1;
        }
        // RNG device proxy: VMs with disk info beyond "None"
        if vm.disk != "None" {
            has_rng_device += 1;
        }
        // Node assignment
        if vm.node != "N/A" && !vm.node.is_empty() {
            has_node_assignment += 1;
        }
    }

    let safe_div = |num: usize, den: usize| -> u16 {
        if den == 0 { 100 } else { ((num as f64 / den as f64) * 100.0).round() as u16 }
    };

    let resource_pct = safe_div(has_resource_limits, total_vms);
    let eviction_pct = safe_div(has_eviction_strategy, total_vms);
    let rng_pct = safe_div(has_rng_device, total_vms);
    let node_pct = safe_div(has_node_assignment, total_vms);

    let gauge_color = |pct: u16| -> Color {
        if pct >= 90 { Color::Rgb(50, 205, 50) }
        else if pct >= 70 { Color::Rgb(255, 200, 0) }
        else { Color::Rgb(220, 50, 47) }
    };

    // Framework gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    let frameworks = [
        ("Resource Limits", resource_pct, gauge_color(resource_pct)),
        ("Eviction Strategy", eviction_pct, gauge_color(eviction_pct)),
        ("Storage Config", rng_pct, gauge_color(rng_pct)),
        ("Node Placement", node_pct, gauge_color(node_pct)),
    ];

    for (i, (name, pct, color)) in frameworks.iter().enumerate() {
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

    // Controls table - one row per VM with compliance findings
    let header_cells = [
        "VM Name",
        "Status",
        "Resource Limits",
        "Eviction",
        "Storage",
    ]
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
        let has_limits = vm.cpu != "1 core" || vm.memory != "Unknown";
        let has_eviction = vm.status == "Running" || vm.status == "Stopped";
        let has_storage = vm.disk != "None";

        let pass_fail = |ok: bool| -> (&str, Color) {
            if ok {
                ("PASS", Color::Rgb(50, 205, 50))
            } else {
                ("FAIL", Color::Rgb(220, 50, 47))
            }
        };

        let (limits_text, limits_color) = pass_fail(has_limits);
        let (eviction_text, eviction_color) = pass_fail(has_eviction);
        let (storage_text, storage_color) = pass_fail(has_storage);

        let status_color = match vm.status.as_str() {
            "Running" => Color::Rgb(50, 205, 50),
            "Stopped" => Color::Rgb(255, 200, 0),
            "Failed" | "Error" => Color::Rgb(220, 50, 47),
            _ => Color::Gray,
        };

        Row::new(vec![
            Cell::from(vm.name.as_str()).style(Style::default().fg(Color::Rgb(222, 115, 86))),
            Cell::from(vm.status.as_str()).style(Style::default().fg(status_color)),
            Cell::from(limits_text).style(
                Style::default()
                    .fg(limits_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from(eviction_text).style(
                Style::default()
                    .fg(eviction_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from(storage_text).style(
                Style::default()
                    .fg(storage_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
        .height(1)
    }).collect();

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ];

    let overall_score = if total_vms == 0 {
        100
    } else {
        (resource_pct as u32 + eviction_pct as u32 + rng_pct as u32 + node_pct as u32) / 4
    };

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    format!(" Compliance Controls ({} VMs, Score: {}%) ", total_vms, overall_score),
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(table, chunks[2]);

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
            "f",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Filter Framework | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "s",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Run Scan | ", Style::default().fg(Color::Gray)),
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
