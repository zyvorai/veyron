// Cost Analytics View

use crate::tui::colors::gradient;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, area: Rect) {
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
    // Gradient brand header
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

    // Cost summary
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
                "$2,847.52",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  vs Last Month: ", Style::default().fg(Color::Gray)),
            Span::styled("+8.3%", Style::default().fg(Color::Rgb(220, 50, 47))),
        ]),
    ];
    let monthly_widget = Paragraph::new(monthly).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(" This Month "),
    );
    f.render_widget(monthly_widget, summary_chunks[0]);

    let daily = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Avg Daily: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "$94.92",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Today:     ", Style::default().fg(Color::Gray)),
            Span::styled("$88.14", Style::default().fg(Color::Rgb(50, 205, 50))),
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
            Span::styled("  Potential: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "$412.30",
                Style::default()
                    .fg(Color::Rgb(50, 205, 50))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Idle VMs:  ", Style::default().fg(Color::Gray)),
            Span::styled("3 detected", Style::default().fg(Color::Rgb(255, 200, 0))),
        ]),
    ];
    let savings_widget = Paragraph::new(savings).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(222, 115, 86)))
            .title(" Savings Opportunity "),
    );
    f.render_widget(savings_widget, summary_chunks[2]);

    // Cost breakdown table
    let header_cells = [
        "VM / Resource",
        "CPU Cost",
        "Memory Cost",
        "Storage Cost",
        "Network",
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

    let costs = [
        (
            "db-primary",
            "$185.20",
            "$312.40",
            "$142.80",
            "$18.50",
            "$658.90",
        ),
        (
            "web-server-01",
            "$92.60",
            "$156.20",
            "$42.10",
            "$45.30",
            "$336.20",
        ),
        (
            "web-server-02",
            "$92.60",
            "$156.20",
            "$42.10",
            "$38.70",
            "$329.60",
        ),
        (
            "api-gateway",
            "$148.10",
            "$208.30",
            "$28.40",
            "$62.10",
            "$446.90",
        ),
        (
            "worker-node-01",
            "$120.40",
            "$180.60",
            "$85.20",
            "$12.40",
            "$398.60",
        ),
        (
            "cache-server",
            "$74.20",
            "$312.40",
            "$14.20",
            "$8.60",
            "$409.40",
        ),
        (
            "test-vm-alpha",
            "$37.10",
            "$78.10",
            "$28.40",
            "$2.10",
            "$145.70",
        ),
        (
            "monitoring",
            "$48.30",
            "$52.10",
            "$18.60",
            "$3.22",
            "$122.22",
        ),
    ];

    let rows = costs.iter().map(|(name, cpu, mem, stor, net, total)| {
        Row::new(vec![
            Cell::from(*name),
            Cell::from(*cpu),
            Cell::from(*mem),
            Cell::from(*stor),
            Cell::from(*net),
            Cell::from(*total).style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
        .height(1)
    });

    let widths = [
        Constraint::Percentage(20),
        Constraint::Percentage(14),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
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
            "↑↓",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Navigate | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "p",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Period | ", Style::default().fg(Color::Gray)),
        Span::styled(
            "t",
            Style::default()
                .fg(Color::Rgb(222, 115, 86))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Trends | ", Style::default().fg(Color::Gray)),
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
