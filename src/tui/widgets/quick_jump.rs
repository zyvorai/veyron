// Quick Jump Menu - Fast navigation between views (Ctrl+P)
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub struct JumpItem {
    pub number: usize,
    pub name: String,
    pub icon: &'static str,
    pub description: String,
}

pub struct QuickJumpMenu {
    pub query: String,
    pub selected: usize,
    items: Vec<JumpItem>,
    filtered_indices: Vec<usize>,
}

impl QuickJumpMenu {
    pub fn new() -> Self {
        let items = vec![
            JumpItem {
                number: 1,
                name: "Dashboard".to_string(),
                icon: "📊",
                description: "System overview and quick stats".to_string(),
            },
            JumpItem {
                number: 2,
                name: "VM List".to_string(),
                icon: "💻",
                description: "Browse and manage virtual machines".to_string(),
            },
            JumpItem {
                number: 3,
                name: "VM Details".to_string(),
                icon: "🔍",
                description: "Detailed VM information".to_string(),
            },
            JumpItem {
                number: 4,
                name: "Snapshots".to_string(),
                icon: "📸",
                description: "Manage VM snapshots and backups".to_string(),
            },
            JumpItem {
                number: 5,
                name: "Profiles".to_string(),
                icon: "⚙️",
                description: "Resource profiles and templates".to_string(),
            },
            JumpItem {
                number: 6,
                name: "Blueprints".to_string(),
                icon: "🏗️",
                description: "Multi-VM deployment blueprints".to_string(),
            },
        ];

        let filtered_indices: Vec<usize> = (0..items.len()).collect();

        Self {
            query: String::new(),
            selected: 0,
            items,
            filtered_indices,
        }
    }

    pub fn update_query(&mut self, query: String) {
        self.query = query.to_lowercase();
        self.filter_items();
        self.selected = 0;
    }

    pub fn add_char(&mut self, c: char) {
        self.query.push(c);
        self.filter_items();
        self.selected = 0;
    }

    pub fn delete_char(&mut self) {
        self.query.pop();
        self.filter_items();
        self.selected = 0;
    }

    fn filter_items(&mut self) {
        if self.query.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            self.filtered_indices = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    item.name.to_lowercase().contains(&self.query)
                        || item.description.to_lowercase().contains(&self.query)
                })
                .map(|(i, _)| i)
                .collect();
        }
    }

    pub fn next(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = (self.selected + 1) % self.filtered_indices.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = if self.selected == 0 {
                self.filtered_indices.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn selected_number(&self) -> Option<usize> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.items.get(idx))
            .map(|item| item.number)
    }

    pub fn render(&self, f: &mut Frame) {
        let area = centered_rect(50, 60, f.area());

        // Clear background
        f.render_widget(Clear, area);

        // Main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::ORANGE))
            .title(Span::styled(
                format!(
                    "🚀 Quick Jump{}",
                    if !self.query.is_empty() {
                        format!(": {}", self.query)
                    } else {
                        String::new()
                    }
                ),
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Layout: Items + Help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Items
                Constraint::Length(3), // Help text
            ])
            .split(inner);

        // Render filtered items
        let list_items: Vec<ListItem> = self
            .filtered_indices
            .iter()
            .enumerate()
            .map(|(display_idx, &item_idx)| {
                let item = &self.items[item_idx];
                let is_selected = display_idx == self.selected;

                let style = if is_selected {
                    Style::default()
                        .fg(colors::TEXT)
                        .bg(colors::DARK_ORANGE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors::TEXT)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let number_style = Style::default()
                    .fg(colors::INFO)
                    .add_modifier(Modifier::BOLD);

                let content = vec![
                    Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(format!("{} ", item.number), number_style),
                        Span::raw(format!("{} {} ", item.icon, item.name)),
                    ]),
                    Line::from(vec![
                        Span::raw("    "),
                        Span::styled(&item.description, Style::default().fg(colors::TEXT_MUTED)),
                    ]),
                ];

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(list_items);
        f.render_widget(list, chunks[0]);

        // Help text
        let help = Paragraph::new(vec![Line::from(Span::styled(
            "↑↓: Navigate │ Enter: Select │ Esc: Cancel │ Type: Search",
            Style::default().fg(colors::TEXT_MUTED),
        ))])
        .alignment(Alignment::Center);

        f.render_widget(help, chunks[1]);
    }
}

impl Default for QuickJumpMenu {
    fn default() -> Self {
        Self::new()
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
