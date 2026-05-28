// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Help View - Comprehensive keybinding reference

use crate::tui::colors::tui as colors;
use crate::tui::config::TuiConfig;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, _config: &TuiConfig) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Help content
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // Header
    let header_text = Line::from(vec![
        Span::styled(
            "VMRogue",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" - ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("KubeVirt VM Manager", Style::default().fg(colors::TEXT)),
        Span::styled("  |  ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "Keyboard Reference",
            Style::default()
                .fg(colors::LIGHT_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );
    f.render_widget(header, chunks[0]);

    // Help content
    render_help_content(f, chunks[1]);

    // Footer
    let footer_text = Line::from(vec![
        Span::styled("Press any key to return", Style::default().fg(colors::TEXT)),
        Span::styled(" | ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "Ctrl+P",
            Style::default()
                .fg(colors::LIGHT_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Quick Jump to any view",
            Style::default().fg(colors::TEXT_MUTED),
        ),
    ]);
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );
    f.render_widget(footer, chunks[2]);
}

fn section(title: &str) -> Line<'_> {
    Line::from(vec![Span::styled(
        title,
        Style::default()
            .fg(colors::WARNING)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )])
}

fn key_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:<16}", key),
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(desc),
    ])
}

fn key_line_colored<'a>(key: &'a str, desc: &'a str, color: ratatui::style::Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:<16}", key),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(desc),
    ])
}

fn render_help_content(f: &mut Frame, area: ratatui::layout::Rect) {
    let help_text = vec![
        Line::from(""),
        // ── GLOBAL ──
        section("GLOBAL"),
        Line::from(""),
        key_line("q", "Quit application"),
        key_line("Esc", "Go back / quit from main views"),
        key_line("?", "Show this help screen"),
        key_line("Ctrl+R", "Refresh data from Kubernetes"),
        key_line("Ctrl+P", "Quick Jump menu (fuzzy search all views)"),
        key_line("/", "Open search bar"),
        Line::from(""),
        // ── VIEWS ──
        section("VIEW NAVIGATION"),
        Line::from(""),
        key_line_colored("1", "Dashboard - System overview", colors::INFO),
        key_line_colored("2", "VM List - Browse and manage VMs", colors::INFO),
        key_line_colored("3", "Snapshots - Backup and restore", colors::INFO),
        key_line_colored("4", "Profiles - Resource templates", colors::INFO),
        key_line_colored("5", "Blueprints - Multi-VM deployments", colors::INFO),
        key_line_colored("6", "Activity Log - Recent operations", colors::INFO),
        key_line_colored("7", "Nodes - Cluster node status", colors::INFO),
        key_line_colored("8", "Events - Kubernetes events", colors::INFO),
        key_line_colored("9", "Cluster Health - Health dashboard", colors::INFO),
        key_line_colored("0", "Topology - VM placement map", colors::INFO),
        Line::from(""),
        // ── VM LIST ──
        section("VM LIST"),
        Line::from(""),
        key_line("j/k  or  Up/Down", "Navigate VM list"),
        key_line("Enter", "View VM details"),
        key_line("m", "Context menu for selected VM"),
        key_line("c", "Create new VM"),
        key_line_colored("s", "Start VM (batch in multi-select)", colors::SUCCESS),
        key_line_colored("x", "Stop VM (batch in multi-select)", colors::ERROR),
        key_line_colored("d", "Delete VM (batch in multi-select)", colors::ERROR),
        key_line("o", "Cycle sort mode (Name/Status/Age)"),
        key_line("f", "Cycle status filter (All/Running/Stopped/Failed)"),
        key_line("E", "Export VM list to CSV in data dir"),
        key_line("v", "Toggle multi-select mode"),
        Line::from(""),
        // ── MULTI-SELECT ──
        section("MULTI-SELECT MODE (v)"),
        Line::from(""),
        key_line("Space", "Toggle selection on current VM"),
        key_line("Ctrl+A", "Select all VMs"),
        key_line("s / x / d", "Batch start / stop / delete selected"),
        key_line("v", "Exit multi-select mode"),
        Line::from(""),
        // ── SEARCH ──
        section("SEARCH (/)"),
        Line::from(""),
        key_line("Type", "Filter VMs by name or status"),
        key_line("Ctrl+I", "Toggle case sensitivity (aa/Aa)"),
        key_line("Ctrl+R", "Toggle regex/literal mode"),
        key_line("Enter", "Apply search and close"),
        key_line("Esc", "Cancel search"),
        Line::from(""),
        // ── VM DETAILS ──
        section("VM DETAILS"),
        Line::from(""),
        key_line("Tab / h/l", "Switch tabs (Overview, Network, Events)"),
        key_line("Backspace", "Go back to VM list"),
        Line::from(""),
        // ── SNAPSHOTS ──
        section("SNAPSHOTS"),
        Line::from(""),
        key_line("j/k", "Navigate snapshot list"),
        key_line_colored("c", "Create new snapshot", colors::SUCCESS),
        Line::from(""),
        // ── STATUS INDICATORS ──
        section("STATUS INDICATORS"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Running  ", Style::default().fg(colors::SUCCESS)),
            Span::styled("  Stopped  ", Style::default().fg(colors::TEXT_MUTED)),
            Span::styled("  Starting  ", Style::default().fg(colors::WARNING)),
            Span::styled("  Failed  ", Style::default().fg(colors::ERROR)),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title(Span::styled(
                    " Keybindings ",
                    Style::default()
                        .fg(colors::ORANGE)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .alignment(Alignment::Left);

    f.render_widget(help, area);
}
