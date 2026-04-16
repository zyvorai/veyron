// Detailed Security Posture View

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
    let mut header_spans = gradient::brand().text("VMRogue");
    header_spans.push(Span::styled(
        " | ",
        Style::default().fg(Color::Rgb(128, 128, 128)),
    ));
    header_spans.push(Span::styled(
        "Security Posture Analysis",
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

    // Analyze VMs for security posture
    let stats = state.get_stats();
    let total_vms = stats.total.max(1);

    // Compute security scores from real VM data
    let vms_with_ip = state.vms.iter().filter(|vm| vm.ip != "N/A" && !vm.ip.is_empty()).count();
    let vms_ready = state.vms.iter().filter(|vm| vm.ready).count();
    let vms_on_node = state.vms.iter().filter(|vm| vm.node != "N/A" && !vm.node.is_empty()).count();

    // Network score: VMs with IP assigned (network connectivity)
    let network_score = if total_vms > 0 { (vms_with_ip * 100 / total_vms) as u16 } else { 0 };
    // Identity score: VMs in ready state (properly configured)
    let identity_score = if total_vms > 0 { (vms_ready * 100 / total_vms) as u16 } else { 0 };
    // Workload score: VMs assigned to nodes
    let workload_score = if total_vms > 0 { (vms_on_node * 100 / total_vms) as u16 } else { 0 };
    // Data score: VMs not in failed state
    let healthy_vms = stats.total - stats.failed;
    let data_score = if total_vms > 0 { (healthy_vms * 100 / total_vms) as u16 } else { 0 };

    fn score_color(pct: u16) -> Color {
        if pct >= 80 { Color::Rgb(50, 205, 50) }
        else if pct >= 50 { Color::Rgb(255, 200, 0) }
        else { Color::Rgb(220, 50, 47) }
    }

    // Category gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    let categories = [
        ("Network", network_score.min(100), score_color(network_score)),
        ("Identity", identity_score.min(100), score_color(identity_score)),
        ("Workload", workload_score.min(100), score_color(workload_score)),
        ("Data", data_score.min(100), score_color(data_score)),
    ];

    for (i, (name, pct, color)) in categories.iter().enumerate() {
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

    // Detailed posture breakdown
    let detail_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    // Build control checks from VM data
    let no_ip_vms: Vec<&str> = state.vms.iter()
        .filter(|vm| vm.ip == "N/A" || vm.ip.is_empty())
        .map(|vm| vm.name.as_str())
        .collect();
    let not_ready_vms: Vec<&str> = state.vms.iter()
        .filter(|vm| !vm.ready)
        .map(|vm| vm.name.as_str())
        .collect();
    let failed_vms: Vec<&str> = state.vms.iter()
        .filter(|vm| vm.status == "Failed" || vm.status == "Error")
        .map(|vm| vm.name.as_str())
        .collect();
    let no_node_vms: Vec<&str> = state.vms.iter()
        .filter(|vm| vm.node == "N/A" || vm.node.is_empty())
        .map(|vm| vm.name.as_str())
        .collect();

    let mut checks = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Network Security",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if no_ip_vms.is_empty() {
        checks.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("All VMs have IP addresses", Style::default().fg(Color::White)),
        ]));
    } else {
        checks.push(Line::from(vec![
            Span::styled("  [FAIL] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                format!("{} VM(s) without IP address", no_ip_vms.len()),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    if state.nodes.iter().all(|n| n.status == "Ready") && !state.nodes.is_empty() {
        checks.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("All nodes are Ready", Style::default().fg(Color::White)),
        ]));
    } else if !state.nodes.is_empty() {
        let not_ready = state.nodes.iter().filter(|n| n.status != "Ready").count();
        checks.push(Line::from(vec![
            Span::styled("  [FAIL] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                format!("{} node(s) not Ready", not_ready),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    checks.push(Line::from(""));
    checks.push(Line::from(Span::styled(
        "  Workload Security",
        Style::default()
            .fg(Color::Rgb(222, 115, 86))
            .add_modifier(Modifier::BOLD),
    )));
    checks.push(Line::from(""));

    if not_ready_vms.is_empty() {
        checks.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("All VMs are in Ready state", Style::default().fg(Color::White)),
        ]));
    } else {
        checks.push(Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                format!("{} VM(s) not ready", not_ready_vms.len()),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    if failed_vms.is_empty() {
        checks.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("No VMs in failed state", Style::default().fg(Color::White)),
        ]));
    } else {
        checks.push(Line::from(vec![
            Span::styled("  [FAIL] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                format!("{} VM(s) in failed state", failed_vms.len()),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    if no_node_vms.is_empty() {
        checks.push(Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("All VMs assigned to nodes", Style::default().fg(Color::White)),
        ]));
    } else {
        checks.push(Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                format!("{} VM(s) not assigned to a node", no_node_vms.len()),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    let network_widget = Paragraph::new(checks).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Control Checks ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(network_widget, detail_chunks[0]);

    // Recommendations based on findings
    let overall_score = (network_score as u32 + identity_score as u32 + workload_score as u32 + data_score as u32) / 4;

    let mut recommendations = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Recommendations",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let mut rec_num = 1;
    let mut potential_gain: u32 = 0;

    if !failed_vms.is_empty() {
        recommendations.push(Line::from(vec![
            Span::styled(format!("  {}. ", rec_num), Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                format!("Investigate {} failed VM(s)", failed_vms.len()),
                Style::default().fg(Color::White),
            ),
        ]));
        recommendations.push(Line::from(vec![
            Span::styled("     Impact: ", Style::default().fg(Color::Gray)),
            Span::styled("+5 posture score", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]));
        recommendations.push(Line::from(""));
        rec_num += 1;
        potential_gain += 5;
    }

    if !no_ip_vms.is_empty() {
        recommendations.push(Line::from(vec![
            Span::styled(format!("  {}. ", rec_num), Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled(
                format!("Assign IPs to {} VM(s)", no_ip_vms.len()),
                Style::default().fg(Color::White),
            ),
        ]));
        recommendations.push(Line::from(vec![
            Span::styled("     Impact: ", Style::default().fg(Color::Gray)),
            Span::styled("+4 posture score", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]));
        recommendations.push(Line::from(""));
        rec_num += 1;
        potential_gain += 4;
    }

    if !not_ready_vms.is_empty() {
        recommendations.push(Line::from(vec![
            Span::styled(format!("  {}. ", rec_num), Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                format!("Fix {} not-ready VM(s)", not_ready_vms.len()),
                Style::default().fg(Color::White),
            ),
        ]));
        recommendations.push(Line::from(vec![
            Span::styled("     Impact: ", Style::default().fg(Color::Gray)),
            Span::styled("+3 posture score", Style::default().fg(Color::Rgb(50, 205, 50))),
        ]));
        recommendations.push(Line::from(""));
        potential_gain += 3;
    }

    if rec_num == 1 {
        recommendations.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "No issues found - posture is healthy",
                Style::default().fg(Color::Rgb(50, 205, 50)),
            ),
        ]));
    }

    recommendations.push(Line::from(""));
    let new_score = (overall_score + potential_gain).min(100);
    recommendations.push(Line::from(vec![
        Span::styled("  Total potential: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{} -> {} (+{})", overall_score, new_score, potential_gain),
            Style::default()
                .fg(Color::Rgb(50, 205, 50))
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    let rec_widget = Paragraph::new(recommendations).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Remediation Plan ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(rec_widget, detail_chunks[1]);

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
            "r",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Remediate | ", Style::default().fg(Color::Gray)),
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
