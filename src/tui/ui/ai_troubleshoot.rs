// AI Troubleshooting Wizard View

use crate::tui::colors::gradient;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
    header_spans.push(Span::styled("AI Troubleshooting Wizard", Style::default().fg(Color::Rgb(255, 145, 115)).add_modifier(Modifier::BOLD)));
    let header_text = Line::from(header_spans);
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(222, 115, 86))),
        );
    f.render_widget(header, chunks[0]);

    // Issue description input
    let input_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Issue: ", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
            Span::styled(
                "VM db-primary is experiencing high latency and intermittent connection drops",
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    let input_widget = Paragraph::new(input_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Describe the Issue ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(input_widget, chunks[1]);

    // Diagnosis and recommendations
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[2]);

    let diagnosis = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Diagnostic Analysis",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Checks Performed:",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("VM is running and responsive", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("Node resources are adequate", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("Disk I/O wait time elevated (12%)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [FAIL] ", Style::default().fg(Color::Rgb(220, 50, 47))),
            Span::styled("Network packet loss detected (2.3%)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [WARN] ", Style::default().fg(Color::Rgb(255, 200, 0))),
            Span::styled("Memory pressure: 89% utilized", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [PASS] ", Style::default().fg(Color::Rgb(50, 205, 50))),
            Span::styled("No recent migrations or restarts", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Root Cause Assessment",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Confidence: ", Style::default().fg(Color::Gray)),
            Span::styled("87%", Style::default().fg(Color::Rgb(50, 205, 50)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Likely cause: ", Style::default().fg(Color::Gray)),
            Span::styled("Network congestion on node-2", Style::default().fg(Color::Rgb(220, 50, 47))),
        ]),
        Line::from(vec![
            Span::styled("  Contributing: ", Style::default().fg(Color::Gray)),
            Span::styled("Memory pressure causing swap I/O", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
    ];
    let diagnosis_widget = Paragraph::new(diagnosis).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Diagnosis ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(diagnosis_widget, content_chunks[0]);

    let remediation = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Recommended Actions",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1. ", Style::default().fg(Color::Rgb(220, 50, 47)).add_modifier(Modifier::BOLD)),
            Span::styled("[URGENT] Migrate to less congested node", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled("Migrate db-primary from node-2 to node-3", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("     Impact: ", Style::default().fg(Color::Gray)),
            Span::styled("~30s downtime during migration", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(Color::Rgb(255, 200, 0)).add_modifier(Modifier::BOLD)),
            Span::styled("[HIGH] Increase VM memory allocation", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled("Increase from 32Gi to 48Gi", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(Color::Rgb(100, 150, 255)).add_modifier(Modifier::BOLD)),
            Span::styled("[INFO] Review network policies on node-2", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled("Check for bandwidth throttling rules", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Auto-Remediation",
            Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::Gray)),
            Span::styled("Enter", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
            Span::styled(" to apply action #1 automatically", Style::default().fg(Color::Gray)),
        ]),
    ];
    let remediation_widget = Paragraph::new(remediation).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(Span::styled(
                " Remediation ",
                Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(remediation_widget, content_chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Actions | ", Style::default().fg(Color::Gray)),
        Span::styled("Enter", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Apply Fix | ", Style::default().fg(Color::Gray)),
        Span::styled("r", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Re-diagnose | ", Style::default().fg(Color::Gray)),
        Span::styled("e", Style::default().fg(Color::Rgb(222, 115, 86)).add_modifier(Modifier::BOLD)),
        Span::styled(": Edit Issue | ", Style::default().fg(Color::Gray)),
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
