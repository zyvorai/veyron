// Help View - Comprehensive keybinding reference

use crate::tui::colors::tui as colors;
use crate::tui::config::TuiConfig;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
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
            "VMRogue".to_string(),
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" - ".to_string(), Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "KubeVirt VM Manager".to_string(),
            Style::default().fg(colors::TEXT),
        ),
        Span::styled("  │  ".to_string(), Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "📖 Help".to_string(),
            Style::default()
                .fg(colors::LIGHT_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            ": Keyboard Shortcuts & Reference".to_string(),
            Style::default().fg(colors::TEXT),
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
        Span::styled("⌨  ".to_string(), Style::default().fg(colors::ORANGE)),
        Span::styled(
            "Press any key to return".to_string(),
            Style::default().fg(colors::TEXT),
        ),
        Span::styled(" │ ".to_string(), Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "💡 Tip: ".to_string(),
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Use ".to_string(), Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            "Ctrl+P".to_string(),
            Style::default()
                .fg(colors::LIGHT_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " for quick jump menu".to_string(),
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

fn render_help_content(f: &mut Frame, area: ratatui::layout::Rect) {
    let help_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "⌨  GLOBAL KEYBINDINGS",
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  q / Esc       ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("🚪  ", Style::default().fg(colors::TEXT)),
            Span::raw("Quit application"),
        ]),
        Line::from(vec![
            Span::styled(
                "  ?             ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("📖  ", Style::default().fg(colors::TEXT)),
            Span::raw("Show this help screen"),
        ]),
        Line::from(vec![
            Span::styled(
                "  r / Ctrl+R    ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("🔄  ", Style::default().fg(colors::TEXT)),
            Span::raw("Refresh data from Kubernetes"),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+P        ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("🚀  ", Style::default().fg(colors::TEXT)),
            Span::raw("Quick jump menu (fuzzy search)"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "🧭  VIEW NAVIGATION",
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  1             ",
                Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("📊  ", Style::default().fg(colors::ORANGE)),
            Span::raw("Dashboard - System overview"),
        ]),
        Line::from(vec![
            Span::styled(
                "  2             ",
                Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("💻  ", Style::default().fg(colors::ORANGE)),
            Span::raw("VM List - Browse and manage VMs"),
        ]),
        Line::from(vec![
            Span::styled(
                "  3             ",
                Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("📸  ", Style::default().fg(colors::ORANGE)),
            Span::raw("Snapshots - Backup and restore"),
        ]),
        Line::from(vec![
            Span::styled(
                "  4             ",
                Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⚙️  ", Style::default().fg(colors::ORANGE)),
            Span::raw("Profiles - Resource templates"),
        ]),
        Line::from(vec![
            Span::styled(
                "  5             ",
                Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("🏗️  ", Style::default().fg(colors::ORANGE)),
            Span::raw("Blueprints - Multi-VM deployments"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "💻  VM LIST VIEW",
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  ↑↓ / j/k      ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Navigate VM list"),
        ]),
        Line::from(vec![
            Span::styled(
                "  Enter         ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("View VM details"),
        ]),
        Line::from(vec![
            Span::styled(
                "  s             ",
                Style::default()
                    .fg(colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("🟢  ", Style::default().fg(colors::TEXT)),
            Span::raw("Start selected VM"),
        ]),
        Line::from(vec![
            Span::styled(
                "  x             ",
                Style::default()
                    .fg(colors::ERROR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⏸   ", Style::default().fg(colors::TEXT)),
            Span::raw("Stop selected VM"),
        ]),
        Line::from(vec![
            Span::styled(
                "  d             ",
                Style::default()
                    .fg(colors::ERROR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("🗑️  ", Style::default().fg(colors::TEXT)),
            Span::raw("Delete selected VM (with confirmation)"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "📸  SNAPSHOTS VIEW",
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  ↑↓ / j/k      ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Navigate snapshot list"),
        ]),
        Line::from(vec![
            Span::styled(
                "  c             ",
                Style::default()
                    .fg(colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("📸  ", Style::default().fg(colors::TEXT)),
            Span::raw("Create new snapshot"),
        ]),
        Line::from(vec![
            Span::styled(
                "  d             ",
                Style::default()
                    .fg(colors::ERROR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("🗑️  ", Style::default().fg(colors::TEXT)),
            Span::raw("Delete selected snapshot"),
        ]),
        Line::from(vec![
            Span::styled(
                "  r             ",
                Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("♻️  ", Style::default().fg(colors::TEXT)),
            Span::raw("Restore selected snapshot"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "⚙️  PROFILES & BLUEPRINTS",
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  ↑↓ / j/k      ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Navigate list"),
        ]),
        Line::from(vec![
            Span::styled(
                "  Enter         ",
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("🔍  ", Style::default().fg(colors::TEXT)),
            Span::raw("View details"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "💡 STATUS INDICATORS",
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🟢 ", Style::default().fg(colors::SUCCESS)),
            Span::raw("Running / OK        "),
            Span::styled("🟡 ", Style::default().fg(colors::WARNING)),
            Span::raw("Starting / Warning        "),
            Span::styled("🔴 ", Style::default().fg(colors::ERROR)),
            Span::raw("Failed / Error"),
        ]),
        Line::from(vec![
            Span::styled("  ⏸  ", Style::default().fg(colors::TEXT_MUTED)),
            Span::raw("Stopped             "),
            Span::styled("⚪ ", Style::default().fg(colors::TEXT_MUTED)),
            Span::raw("Unknown / Inactive"),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title("Keybindings"),
        )
        .alignment(Alignment::Left);

    f.render_widget(help, area);
}
