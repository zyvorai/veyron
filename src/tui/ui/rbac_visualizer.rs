// RBAC Visualizer View

use crate::tui::colors::gradient;
use crate::tui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, area: Rect, _state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
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
        "RBAC Visualizer",
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

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Roles table
    let role_header = ["Role", "Type", "Namespace", "Rules"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )
    });
    let role_header_row = Row::new(role_header)
        .style(Style::default().bg(Color::Rgb(40, 35, 55)))
        .height(1);

    let roles = [
        ("vm-admin", "ClusterRole", "*", "12"),
        ("vm-operator", "ClusterRole", "*", "8"),
        ("vm-viewer", "Role", "production", "3"),
        ("snapshot-mgr", "Role", "default", "5"),
        ("migration-exec", "ClusterRole", "*", "6"),
        ("network-admin", "Role", "kube-system", "9"),
    ];

    let role_rows = roles.iter().map(|(name, rtype, ns, rules)| {
        let type_color = if *rtype == "ClusterRole" {
            Color::Rgb(222, 115, 86)
        } else {
            Color::Rgb(100, 150, 255)
        };
        Row::new(vec![
            Cell::from(*name),
            Cell::from(*rtype).style(Style::default().fg(type_color)),
            Cell::from(*ns),
            Cell::from(*rules),
        ])
        .height(1)
    });

    let role_widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(20),
    ];

    let role_table = Table::new(role_rows, role_widths)
        .header(role_header_row)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
                .title(Span::styled(
                    " Roles ",
                    Style::default()
                        .fg(Color::Rgb(222, 115, 86))
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .column_spacing(1);
    f.render_widget(role_table, main_chunks[0]);

    // Bindings and subjects
    let binding_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Role Bindings",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  vm-admin-binding",
            Style::default().fg(Color::White),
        )]),
        Line::from(vec![
            Span::styled("    Role:     ", Style::default().fg(Color::Gray)),
            Span::styled("vm-admin", Style::default().fg(Color::Rgb(222, 115, 86))),
        ]),
        Line::from(vec![
            Span::styled("    Subjects: ", Style::default().fg(Color::Gray)),
            Span::styled("admin-group (Group)", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  vm-operator-binding",
            Style::default().fg(Color::White),
        )]),
        Line::from(vec![
            Span::styled("    Role:     ", Style::default().fg(Color::Gray)),
            Span::styled("vm-operator", Style::default().fg(Color::Rgb(222, 115, 86))),
        ]),
        Line::from(vec![
            Span::styled("    Subjects: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "ops-team (Group), svc-deployer (SA)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  vm-viewer-binding",
            Style::default().fg(Color::White),
        )]),
        Line::from(vec![
            Span::styled("    Role:     ", Style::default().fg(Color::Gray)),
            Span::styled("vm-viewer", Style::default().fg(Color::Rgb(100, 150, 255))),
        ]),
        Line::from(vec![
            Span::styled("    Subjects: ", Style::default().fg(Color::Gray)),
            Span::styled("dev-team (Group)", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Warnings",
            Style::default()
                .fg(Color::Rgb(255, 200, 0))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [!] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                "vm-admin has wildcard verb access",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  [!] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled(
                "2 unused service accounts found",
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    let bindings_widget = Paragraph::new(binding_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Bindings & Subjects ",
                Style::default()
                    .fg(Color::Rgb(222, 115, 86))
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(bindings_widget, main_chunks[1]);

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
            "Enter",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Details | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "a",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Audit | ", Style::default().fg(Color::Gray)),
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
    f.render_widget(help, chunks[2]);
}
