// Blueprints View - Browse and view multi-VM blueprints

use crate::tui::colors::tui as colors;
use crate::tui::{config::TuiConfig, state::AppState};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, state: &AppState, _config: &TuiConfig) {
    let size = f.area();

    // Get blueprints list
    let blueprints_manager = crate::blueprints::BLUEPRINTS
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let blueprints: Vec<String> = blueprints_manager
        .list()
        .into_iter()
        .map(|b| b.name.clone())
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
    let header = Paragraph::new(format!("VM Blueprints ({})", blueprints.len()))
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
            Constraint::Percentage(50), // Blueprint list
            Constraint::Percentage(50), // Blueprint details
        ])
        .split(chunks[1]);

    // Blueprint list
    render_blueprint_list(f, &blueprints, state.selected_index, content_chunks[0]);

    // Blueprint details
    if let Some(blueprint_name) = blueprints.get(state.selected_index) {
        render_blueprint_details(f, blueprint_name, &blueprints_manager, content_chunks[1]);
    }

    // Help text
    render_help(f, chunks[2]);
}

fn render_blueprint_list(
    f: &mut Frame,
    blueprints: &[String],
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

    let blueprints_manager = crate::blueprints::BLUEPRINTS
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let rows = blueprints.iter().enumerate().map(|(i, name)| {
        let is_selected = i == selected_index;
        let is_builtin = blueprints_manager.is_builtin(name);

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
            .title("Blueprints"),
    )
    .column_spacing(1);

    f.render_widget(table, area);
}

fn render_blueprint_details(
    f: &mut Frame,
    blueprint_name: &str,
    blueprints_manager: &crate::blueprints::BlueprintManager,
    area: ratatui::layout::Rect,
) {
    if let Some(blueprint) = blueprints_manager.get(blueprint_name) {
        // Split details area
        let detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Basic info
                Constraint::Min(0),    // VM list
            ])
            .split(area);

        // Basic info
        let info_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Name:        ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&blueprint.name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Description: ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&blueprint.description),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "VMs:         ",
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}", blueprint.vms.len())),
            ]),
        ];

        let info = Paragraph::new(info_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::BORDER))
                    .title("Blueprint Details"),
            )
            .alignment(Alignment::Left);

        f.render_widget(info, detail_chunks[0]);

        // VM list
        let vm_items: Vec<ListItem> = blueprint
            .vms
            .iter()
            .map(|vm| {
                let profile = vm.profile.as_deref().unwrap_or("default");
                let content = format!("  • {} ({})", vm.name, profile);
                ListItem::new(content).style(Style::default().fg(colors::TEXT))
            })
            .collect();

        let vm_list = List::new(vm_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title("Virtual Machines"),
        );

        f.render_widget(vm_list, detail_chunks[1]);
    } else {
        let text = Paragraph::new("Blueprint not found")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::BORDER))
                    .title("Blueprint Details"),
            );
        f.render_widget(text, area);
    }
}

fn render_help(f: &mut Frame, area: ratatui::layout::Rect) {
    let help_text = "↑↓/jk: Navigate | Enter: View Details | d: Deploy | q: Quit | ?: Help";

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
