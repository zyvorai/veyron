// Enhanced VM List View - Advanced table with search, filter, sort, multi-select
use crate::tui::{colors::tui as colors, config::TuiConfig, state::AppState};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

pub fn render(f: &mut Frame, state: &mut AppState, _config: &TuiConfig) {
    let size = f.area();

    // Main layout with optional stats bar
    let main_chunks = if state.show_stats_bar {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(2), // Stats bar
                Constraint::Min(0),    // VM table
                Constraint::Length(5), // Help text
            ])
            .split(size)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // VM table
                Constraint::Length(5), // Help text
            ])
            .split(size)
    };

    let mut current_idx = 0;

    // Header
    render_header(f, state, main_chunks[current_idx]);
    current_idx += 1;

    // Stats bar (if enabled)
    if state.show_stats_bar {
        render_stats_bar(f, state, main_chunks[current_idx]);
        current_idx += 1;
    }

    // VM table (with stateful scroll-to-selection)
    render_vm_table(f, state, main_chunks[current_idx]);
    current_idx += 1;

    // Help text
    render_help(f, state, main_chunks[current_idx]);
}

fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
    let filtered_count = state.filtered_vms().len();
    let total_count = state.vms.len();
    let vm_count_text = if state.status_filter.is_some() {
        format!(": {}/{} VMs", filtered_count, total_count)
    } else {
        format!(": {} VMs", total_count)
    };
    let sort_indicator = if state.sort_mode != crate::tui::state::SortMode::Default {
        format!(" [Sort: {}]", state.sort_mode.display())
    } else {
        String::new()
    };

    let multi_select_indicator = if state.multi_select_mode {
        format!(" [Multi: {}]", state.selected_items.len())
    } else {
        String::new()
    };

    let filter_indicator = match &state.status_filter {
        Some(f) => format!(" [Filter: {}]", f),
        None => String::new(),
    };

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
            "💻 VM List".to_string(),
            Style::default()
                .fg(colors::LIGHT_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(vm_count_text, Style::default().fg(colors::TEXT)),
        Span::styled(sort_indicator, Style::default().fg(colors::INFO)),
        Span::styled(filter_indicator, Style::default().fg(colors::WARNING)),
        Span::styled(multi_select_indicator, Style::default().fg(colors::WARNING)),
    ]);

    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );

    f.render_widget(header, area);
}

fn render_stats_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let stats = state.get_stats();

    let stats_line = Line::from(vec![
        Span::styled("📊 ", Style::default().fg(colors::ORANGE)),
        Span::styled(
            format!("Total: {} ", stats.total),
            Style::default().fg(colors::TEXT),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("🟢 ", Style::default().fg(colors::SUCCESS)),
        Span::styled(
            format!("{} ", stats.running),
            Style::default()
                .fg(colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("⏸  ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled(
            format!("{} ", stats.stopped),
            Style::default()
                .fg(colors::TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("🟡 ", Style::default().fg(colors::WARNING)),
        Span::styled(
            format!("{} ", stats.starting),
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(colors::TEXT_MUTED)),
        Span::styled("🔴 ", Style::default().fg(colors::ERROR)),
        Span::styled(
            format!("{} ", stats.failed),
            Style::default()
                .fg(colors::ERROR)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let paragraph = Paragraph::new(stats_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::BORDER)),
    );

    f.render_widget(paragraph, area);
}

fn render_vm_table(f: &mut Frame, state: &mut AppState, area: Rect) {
    // Enhanced header with icons
    let header_cells = ["", "Name", "Status", "CPU", "Memory", "Disk", "Age", "IP"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(colors::LIGHT_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header = Row::new(header_cells)
        .style(Style::default().bg(colors::DARK_ORANGE))
        .height(1);

    let filtered_vms = state.filtered_vms();
    let rows = filtered_vms.iter().enumerate().map(|(i, vm)| {
        let is_selected = i == state.selected_index;
        let is_multi_selected = state.is_selected(i);

        let (status_color, status_icon) = match vm.status.as_str() {
            "Running" => (colors::SUCCESS, "🟢"),
            "Stopped" => (colors::TEXT_MUTED, "⏸ "),
            "Starting" | "Pending" => (colors::WARNING, "🟡"),
            "Failed" | "Error" => (colors::ERROR, "🔴"),
            _ => (colors::TEXT, "⚪"),
        };

        // Selection indicators
        let selection_indicator = if state.multi_select_mode {
            if is_multi_selected { "☑ " } else { "☐ " }
        } else if is_selected {
            "▶ "
        } else {
            "  "
        };

        let cells = vec![
            Cell::from(selection_indicator),
            Cell::from(format!("💾 {}", vm.name)),
            Cell::from(format!("{} {}", status_icon, vm.status)).style(
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from(vm.cpu.clone()),
            Cell::from(vm.memory.clone()),
            Cell::from(vm.disk.clone()),
            Cell::from(format!("⏱  {}", vm.age)),
            Cell::from(format!("🌐 {}", vm.ip)),
        ];

        let mut style = if is_selected {
            Style::default()
                .bg(colors::DARK_ORANGE)
                .fg(colors::TEXT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        // Highlight multi-selected rows
        if is_multi_selected && !is_selected {
            style = style.fg(colors::LIGHT_ORANGE);
        }

        Row::new(cells).style(style).height(1)
    });

    let widths = [
        Constraint::Length(3),      // Selection
        Constraint::Percentage(20), // Name
        Constraint::Percentage(15), // Status
        Constraint::Percentage(10), // CPU
        Constraint::Percentage(12), // Memory
        Constraint::Percentage(10), // Disk
        Constraint::Percentage(13), // Age
        Constraint::Percentage(20), // IP
    ];

    let title = if state.multi_select_mode {
        Span::styled(
            format!(
                "💻 Virtual Machines [Multi-Select: {} selected]",
                state.selected_items.len()
            ),
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "💻 Virtual Machines",
            Style::default()
                .fg(colors::ORANGE)
                .add_modifier(Modifier::BOLD),
        )
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER))
                .title(title),
        )
        .highlight_style(Style::default()) // We already style selected row inline
        .column_spacing(1);

    // Use TableState for auto-scroll to selected row
    let mut table_state = TableState::default().with_selected(Some(state.selected_index));
    f.render_stateful_widget(table, area, &mut table_state);
}

fn render_help(f: &mut Frame, state: &AppState, area: Rect) {
    let sort_text = format!(" [{}]", state.sort_mode.display());
    let help_lines = if state.multi_select_mode {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "☑  Multi-Select Mode",
                    Style::default()
                        .fg(colors::WARNING)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" - ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "Space",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Toggle, ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "Ctrl+A",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": All, ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "v",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Exit", Style::default().fg(colors::TEXT_MUTED)),
            ]),
            Line::from(vec![
                Span::styled("  Actions: ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "s",
                    Style::default()
                        .fg(colors::SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Start All, ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "x",
                    Style::default()
                        .fg(colors::ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Stop All, ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "d",
                    Style::default()
                        .fg(colors::ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Delete All", Style::default().fg(colors::TEXT_MUTED)),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("⌨  ", Style::default().fg(colors::ORANGE)),
                Span::styled(
                    "↑↓/jk",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Navigate", Style::default().fg(colors::TEXT)),
                Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Details", Style::default().fg(colors::TEXT)),
                Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "s",
                    Style::default()
                        .fg(colors::SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Start", Style::default().fg(colors::TEXT)),
                Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "x",
                    Style::default()
                        .fg(colors::ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Stop", Style::default().fg(colors::TEXT)),
                Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "d",
                    Style::default()
                        .fg(colors::ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Delete", Style::default().fg(colors::TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    "/",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Search", Style::default().fg(colors::TEXT)),
                Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "o",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Sort", Style::default().fg(colors::TEXT)),
                Span::styled(sort_text, Style::default().fg(colors::INFO)),
                Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "v",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Multi-select", Style::default().fg(colors::TEXT)),
                Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "f",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Filter", Style::default().fg(colors::TEXT)),
                Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "Ctrl+P",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Jump", Style::default().fg(colors::TEXT)),
                Span::styled(" │ ", Style::default().fg(colors::TEXT_MUTED)),
                Span::styled(
                    "Ctrl+R",
                    Style::default()
                        .fg(colors::INFO)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(": Refresh", Style::default().fg(colors::TEXT)),
            ]),
        ]
    };

    let help = Paragraph::new(help_lines)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BORDER)),
        );

    f.render_widget(help, area);
}
