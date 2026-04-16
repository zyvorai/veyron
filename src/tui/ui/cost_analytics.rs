// Cost Analytics View — wired to real VM data from AppState

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

// AWS-like pricing constants (monthly)
const CPU_MONTHLY: f64 = 30.0; // ~$30/core/month
const MEM_GB_MONTHLY: f64 = 3.75; // ~$3.75/GB/month
const STORAGE_GB_MONTHLY: f64 = 0.10; // $0.10/GB/month

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(7),
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
        "Cost Analytics",
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

    // Calculate costs from VM specs
    struct VmCost {
        name: String,
        cpu_cost: f64,
        mem_cost: f64,
        storage_cost: f64,
        total: f64,
    }

    let mut vm_costs: Vec<VmCost> = state
        .vms
        .iter()
        .map(|vm| {
            let cpu_cores: f64 = vm.cpu.parse().unwrap_or(0.0);
            let mem_gib = crate::utils::parse_memory_gib(&vm.memory);
            let disk_gib = crate::utils::parse_memory_gib(&vm.disk);

            let cpu_cost = cpu_cores * CPU_MONTHLY;
            let mem_cost = mem_gib * MEM_GB_MONTHLY;
            let storage_cost = disk_gib * STORAGE_GB_MONTHLY;
            let total = cpu_cost + mem_cost + storage_cost;

            VmCost {
                name: vm.name.clone(),
                cpu_cost,
                mem_cost,
                storage_cost,
                total,
            }
        })
        .collect();

    vm_costs.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap_or(std::cmp::Ordering::Equal));

    let total_monthly: f64 = vm_costs.iter().map(|c| c.total).sum();
    let daily_avg = total_monthly / 30.0;
    let stopped_vms = state.vms.iter().filter(|v| v.status.contains("Stop")).count();
    let stopped_savings: f64 = vm_costs
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            state.vms.get(*i).map(|v| v.status.contains("Stop")).unwrap_or(false)
        })
        .map(|(_, c)| c.total)
        .sum();

    // Cost summary panels
    let summary_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[1]);

    let monthly = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Total: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("${:.2}", total_monthly),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  VMs:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                state.vms.len().to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    let monthly_widget = Paragraph::new(monthly).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(" Monthly Estimate "),
    );
    f.render_widget(monthly_widget, summary_chunks[0]);

    let daily = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Avg Daily: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("${:.2}", daily_avg),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Per VM:    ", Style::default().fg(Color::Gray)),
            Span::styled(
                if state.vms.is_empty() {
                    "$0.00".to_string()
                } else {
                    format!("${:.2}", total_monthly / state.vms.len() as f64)
                },
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    let daily_widget = Paragraph::new(daily).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(" Daily Cost "),
    );
    f.render_widget(daily_widget, summary_chunks[1]);

    let savings = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Savings:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("${:.2}", stopped_savings),
                Style::default()
                    .fg(Color::Rgb(50, 205, 50))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Stopped:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} VMs", stopped_vms),
                Style::default().fg(if stopped_vms > 0 {
                    Color::Rgb(255, 200, 0)
                } else {
                    Color::Gray
                }),
            ),
        ]),
    ];
    let savings_widget = Paragraph::new(savings).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(" Stopped VM Costs "),
    );
    f.render_widget(savings_widget, summary_chunks[2]);

    // Cost breakdown table
    let header_cells = [
        "VM / Resource",
        "CPU Cost",
        "Memory Cost",
        "Storage Cost",
        "Total",
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

    let rows: Vec<Row> = if vm_costs.is_empty() {
        vec![Row::new(vec![Cell::from("  No VMs to analyze")
            .style(Style::default().fg(Color::Gray))])]
    } else {
        vm_costs
            .iter()
            .map(|c| {
                Row::new(vec![
                    Cell::from(c.name.as_str()),
                    Cell::from(format!("${:.2}", c.cpu_cost)),
                    Cell::from(format!("${:.2}", c.mem_cost)),
                    Cell::from(format!("${:.2}", c.storage_cost)),
                    Cell::from(format!("${:.2}", c.total)).style(
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
                .height(1)
            })
            .collect()
    };

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(18),
        Constraint::Percentage(19),
        Constraint::Percentage(19),
        Constraint::Percentage(16),
    ];

    let table = Table::new(rows, widths)
        .header(table_header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Cost Breakdown by VM ",
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
            "Backspace",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Back | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Refresh", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
    );
    f.render_widget(help, chunks[3]);
}
