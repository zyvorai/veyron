// VM Details View - Tabbed detail view with Overview, Network, and Events tabs

use crate::tui::colors::tui as colors;
use crate::tui::{config::TuiConfig, state::AppState};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub const TAB_NAMES: [&str; 3] = ["Overview", "Network", "Events"];

pub fn render(f: &mut Frame, state: &AppState, _config: &TuiConfig, active_tab: usize) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(1), // Tab bar
            Constraint::Min(0),    // Tab content
            Constraint::Length(3), // Help
        ])
        .split(size);

    // Header
    let vm_name = state
        .selected_vm()
        .map(|vm| vm.name.clone())
        .unwrap_or_else(|| "No VM selected".to_string());

    let header = Paragraph::new(format!("VM Details: {}", vm_name))
        .style(
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );
    f.render_widget(header, chunks[0]);

    // Tab bar
    render_tab_bar(f, active_tab, chunks[1]);

    // Tab content
    if let Some(vm) = state.selected_vm() {
        match active_tab {
            0 => render_overview_tab(f, vm, chunks[2]),
            1 => render_network_tab(f, vm, state, chunks[2]),
            2 => render_events_tab(f, vm, state, chunks[2]),
            _ => render_overview_tab(f, vm, chunks[2]),
        }
    } else {
        let text = Paragraph::new("No VM selected")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::BORDER)),
            );
        f.render_widget(text, chunks[2]);
    }

    // Help
    let help = Paragraph::new("←→/hl/Tab: Switch tabs │ Backspace: Back │ q: Quit")
        .style(Style::default().fg(colors::TEXT_MUTED))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );
    f.render_widget(help, chunks[3]);
}

fn render_tab_bar(f: &mut Frame, active_tab: usize, area: ratatui::layout::Rect) {
    let mut spans = vec![Span::styled("  ", Style::default())];

    for (i, name) in TAB_NAMES.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)));
        }

        if i == active_tab {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(colors::TEXT_MUTED),
            ));
        }
    }

    let tab_line = Paragraph::new(Line::from(spans));
    f.render_widget(tab_line, area);
}

fn render_overview_tab(
    f: &mut Frame,
    vm: &crate::tui::state::VmInfo,
    area: ratatui::layout::Rect,
) {
    let status_color = match vm.status.as_str() {
        "Running" => colors::SUCCESS,
        "Stopped" => colors::TEXT_MUTED,
        "Failed" | "Error" => colors::ERROR,
        _ => colors::WARNING,
    };

    let text = vec![
        Line::from(""),
        detail_line("Name:       ", &vm.name, Style::default().fg(colors::TEXT)),
        Line::from(""),
        detail_line(
            "Status:     ",
            &vm.status,
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        detail_line("CPU:        ", &vm.cpu, Style::default().fg(colors::TEXT)),
        Line::from(""),
        detail_line(
            "Memory:     ",
            &vm.memory,
            Style::default().fg(colors::TEXT),
        ),
        Line::from(""),
        detail_line("Age:        ", &vm.age, Style::default().fg(colors::TEXT)),
        Line::from(""),
        detail_line(
            "Ready:      ",
            if vm.ready { "Yes" } else { "No" },
            if vm.ready {
                Style::default().fg(colors::SUCCESS)
            } else {
                Style::default().fg(colors::ERROR)
            },
        ),
        Line::from(""),
        detail_line("Disk:       ", &vm.disk, Style::default().fg(colors::TEXT)),
        Line::from(""),
        detail_line(
            "Node:       ",
            &vm.node,
            if vm.node != "N/A" {
                Style::default().fg(colors::TEXT)
            } else {
                Style::default().fg(colors::TEXT_MUTED)
            },
        ),
    ];

    let details = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title("Overview"),
        )
        .alignment(Alignment::Left);

    f.render_widget(details, area);
}

fn render_network_tab(
    f: &mut Frame,
    vm: &crate::tui::state::VmInfo,
    state: &AppState,
    area: ratatui::layout::Rect,
) {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Network Interfaces",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    // Show IP from VmInfo
    let ip_style = if vm.ip != "N/A" {
        Style::default().fg(colors::INFO)
    } else {
        Style::default().fg(colors::TEXT_MUTED)
    };
    lines.push(detail_line("  IP Address: ", &vm.ip, ip_style));
    lines.push(Line::from(""));

    // Network interface table header
    lines.push(Line::from(vec![Span::styled(
        "  INTERFACE          IP ADDRESS           MAC ADDRESS",
        Style::default()
            .fg(colors::LIGHT_ORANGE)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  ─────────────────────────────────────────────────────",
        Style::default().fg(colors::TEXT_MUTED),
    )]));

    // Use VMI detail if available, otherwise fall back to VmInfo
    if let Some(ref vmi_status) = state.selected_vmi_detail {
        if vmi_status.interfaces.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  No network interfaces reported",
                Style::default()
                    .fg(colors::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            )]));
        } else {
            for iface in &vmi_status.interfaces {
                let name = iface
                    .interface_name
                    .as_deref()
                    .or(iface.name.as_deref())
                    .unwrap_or("—");
                let ip = iface.ip_address.as_deref().unwrap_or("—");
                let mac = iface.mac.as_deref().unwrap_or("—");

                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<21}", name), Style::default().fg(colors::TEXT)),
                    Span::styled(format!("{:<21}", ip), Style::default().fg(colors::INFO)),
                    Span::styled(mac, Style::default().fg(colors::TEXT_MUTED)),
                ]));
            }
        }

        // Guest OS info
        if let Some(ref os_info) = vmi_status.guest_os_info {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  Guest OS",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            if let Some(ref name) = os_info.name {
                lines.push(detail_line(
                    "  OS:       ",
                    name,
                    Style::default().fg(colors::TEXT),
                ));
            }
            if let Some(ref version) = os_info.version {
                lines.push(detail_line(
                    "  Version:  ",
                    version,
                    Style::default().fg(colors::TEXT),
                ));
            }
            if let Some(ref kernel) = os_info.kernel_release {
                lines.push(detail_line(
                    "  Kernel:   ",
                    kernel,
                    Style::default().fg(colors::TEXT),
                ));
            }
        }

        // Node info from VMI
        if let Some(ref node) = vmi_status.node_name {
            lines.push(Line::from(""));
            lines.push(detail_line(
                "  Node:     ",
                node,
                Style::default().fg(colors::TEXT),
            ));
        }
    } else if vm.ip != "N/A" {
        lines.push(Line::from(vec![
            Span::styled("  eth0                 ", Style::default().fg(colors::TEXT)),
            Span::styled(
                format!("{:<21}", vm.ip),
                Style::default().fg(colors::INFO),
            ),
            Span::styled("—", Style::default().fg(colors::TEXT_MUTED)),
        ]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  No network interfaces available",
            Style::default()
                .fg(colors::TEXT_MUTED)
                .add_modifier(Modifier::ITALIC),
        )]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  VM must be running to show network details",
            Style::default().fg(colors::TEXT_MUTED),
        )]));
    }

    let details = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title("Network"),
        )
        .alignment(Alignment::Left);

    f.render_widget(details, area);
}

fn render_events_tab(
    f: &mut Frame,
    vm: &crate::tui::state::VmInfo,
    state: &AppState,
    area: ratatui::layout::Rect,
) {
    let vm_events: Vec<_> = state
        .recent_activity
        .iter()
        .filter(|e| e.vm_name == vm.name)
        .collect();

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  Activity for {}", vm.name),
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({} events)", vm_events.len()),
                Style::default().fg(colors::TEXT_MUTED),
            ),
        ]),
        Line::from(""),
    ];

    if vm_events.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                "  No activity recorded for this VM",
                Style::default()
                    .fg(colors::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    } else {
        for event in vm_events.iter().take(15) {
            let icon_color = match event.action.as_str() {
                "started" | "start requested" => colors::SUCCESS,
                "stopped" | "stop requested" => colors::WARNING,
                "failed" => colors::ERROR,
                "deleted" | "removed" => colors::ERROR,
                _ => colors::INFO,
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", event.icon), Style::default().fg(icon_color)),
                Span::styled(
                    format!("{:<20}", event.action),
                    Style::default().fg(colors::TEXT),
                ),
                Span::styled(
                    event.elapsed_display(),
                    Style::default()
                        .fg(colors::TEXT_MUTED)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        }
    }

    let details = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title("Events"),
        )
        .alignment(Alignment::Left);

    f.render_widget(details, area);
}

fn detail_line<'a>(label: &'a str, value: &'a str, value_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            label,
            Style::default()
                .fg(colors::TEXT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value, value_style),
    ])
}
