// Profiles View - Browse and view resource profiles

use crate::tui::colors::tui as colors;
use crate::tui::{config::TuiConfig, state::AppState};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, state: &AppState, _config: &TuiConfig) {
    let size = f.area();

    // Get profiles list
    let profiles_manager = crate::profiles::PROFILES
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let profiles: Vec<String> = profiles_manager
        .list()
        .into_iter()
        .map(|p| p.name.clone())
        .collect();

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content (split into list and details)
            Constraint::Length(3), // Help text
        ])
        .split(size);

    // Header
    let header = Paragraph::new(format!("Resource Profiles ({})", profiles.len()))
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

    // Split content area
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Profile list
            Constraint::Percentage(50), // Profile details
        ])
        .split(chunks[1]);

    // Profile list
    render_profile_list(f, &profiles, state.selected_index, content_chunks[0]);

    // Profile details
    if let Some(profile_name) = profiles.get(state.selected_index) {
        render_profile_details(f, profile_name, &profiles_manager, content_chunks[1]);
    }

    // Help text
    render_help(f, chunks[2]);
}

fn render_profile_list(
    f: &mut Frame,
    profiles: &[String],
    selected_index: usize,
    area: ratatui::layout::Rect,
) {
    let header_cells = ["Name", "Type"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells)
        .style(Style::default().bg(colors::DARK_ORANGE))
        .height(1);

    let profiles_manager = crate::profiles::PROFILES
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let rows = profiles.iter().enumerate().map(|(i, name)| {
        let is_selected = i == selected_index;
        let is_builtin = profiles_manager.is_builtin(name);

        let type_cell = if is_builtin {
            Cell::from("Builtin").style(Style::default().fg(colors::SUCCESS))
        } else {
            Cell::from("Custom").style(Style::default().fg(colors::ORANGE))
        };

        let cells = vec![Cell::from(name.clone()), type_cell];

        let style = if is_selected {
            Style::default()
                .bg(colors::DARK_ORANGE)
                .fg(colors::TEXT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        Row::new(cells).style(style).height(1)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(60), // Name
            Constraint::Percentage(40), // Type
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER))
            .title("Profiles"),
    )
    .column_spacing(1);

    f.render_widget(table, area);
}

fn render_profile_details(
    f: &mut Frame,
    profile_name: &str,
    profiles_manager: &crate::profiles::ProfileManager,
    area: ratatui::layout::Rect,
) {
    if let Some(profile) = profiles_manager.get(profile_name) {
        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Name:        ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&profile.name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "CPU Cores:   ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}", profile.cpu_cores)),
            ]),
            Line::from(vec![
                Span::styled(
                    "CPU Sockets: ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}", profile.cpu_sockets)),
            ]),
            Line::from(vec![
                Span::styled(
                    "CPU Threads: ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}", profile.cpu_threads)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Memory:      ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&profile.memory),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Disk Size:   ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&profile.disk_size),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Description: ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&profile.description),
            ]),
        ];

        let details = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::BORDER))
                    .title("Profile Details"),
            )
            .alignment(Alignment::Left);

        f.render_widget(details, area);
    } else {
        let text = Paragraph::new("Profile not found")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::BORDER))
                    .title("Profile Details"),
            );
        f.render_widget(text, area);
    }
}

fn render_help(f: &mut Frame, area: ratatui::layout::Rect) {
    let help_text = "↑↓/jk: Navigate | Enter: View Details | q: Quit | ?: Help";

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(colors::TEXT_MUTED))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );

    f.render_widget(help, area);
}
