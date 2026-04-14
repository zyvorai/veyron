// Quick Jump Menu - Fast navigation between views (Ctrl+P)
use crate::tui::colors::tui as colors;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

#[derive(Debug, Clone, PartialEq)]
pub struct JumpItem {
    pub number: usize,
    pub name: String,
    pub icon: &'static str,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
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
                name: "Snapshots".to_string(),
                icon: "📸",
                description: "Manage VM snapshots and backups".to_string(),
            },
            JumpItem {
                number: 4,
                name: "Profiles".to_string(),
                icon: "⚙️",
                description: "Resource profiles and templates".to_string(),
            },
            JumpItem {
                number: 5,
                name: "Blueprints".to_string(),
                icon: "🏗️",
                description: "Multi-VM deployment blueprints".to_string(),
            },
            JumpItem {
                number: 6,
                name: "Activity Log".to_string(),
                icon: "📋",
                description: "Recent VM operations and events".to_string(),
            },
            JumpItem {
                number: 7,
                name: "Nodes".to_string(),
                icon: "🖥️",
                description: "Cluster node status and resources".to_string(),
            },
            JumpItem {
                number: 8,
                name: "Events".to_string(),
                icon: "⚡",
                description: "Kubernetes cluster events".to_string(),
            },
            JumpItem {
                number: 9,
                name: "Cluster Health".to_string(),
                icon: "💚",
                description: "Overall cluster health dashboard".to_string(),
            },
            JumpItem {
                number: 10,
                name: "Topology".to_string(),
                icon: "🗺️",
                description: "Cluster topology and VM placement".to_string(),
            },
            JumpItem {
                number: 11,
                name: "Pods".to_string(),
                icon: "📦",
                description: "Kubernetes pods and containers".to_string(),
            },
            JumpItem {
                number: 12,
                name: "VMIs".to_string(),
                icon: "🔄",
                description: "Running VM instances".to_string(),
            },
            JumpItem {
                number: 13,
                name: "Migration".to_string(),
                icon: "🚀",
                description: "Live migration wizard".to_string(),
            },
            JumpItem {
                number: 14,
                name: "Security".to_string(),
                icon: "🛡️",
                description: "Security posture dashboard".to_string(),
            },
            JumpItem {
                number: 15,
                name: "Costs".to_string(),
                icon: "💰",
                description: "Cost analytics and breakdown".to_string(),
            },
            JumpItem {
                number: 16,
                name: "Compliance".to_string(),
                icon: "📋",
                description: "Compliance frameworks checker".to_string(),
            },
            JumpItem {
                number: 17,
                name: "Vulnerabilities".to_string(),
                icon: "🔍",
                description: "Vulnerability scanner".to_string(),
            },
            JumpItem {
                number: 18,
                name: "Audit".to_string(),
                icon: "📜",
                description: "Audit trail and events".to_string(),
            },
            JumpItem {
                number: 19,
                name: "Timeline".to_string(),
                icon: "📅",
                description: "Event timeline view".to_string(),
            },
            JumpItem {
                number: 20,
                name: "Performance".to_string(),
                icon: "📈",
                description: "Performance profiler".to_string(),
            },
            JumpItem {
                number: 21,
                name: "Search".to_string(),
                icon: "🔎",
                description: "Natural language search".to_string(),
            },
            JumpItem {
                number: 22,
                name: "Dependencies".to_string(),
                icon: "🔗",
                description: "VM dependency graph".to_string(),
            },
            JumpItem {
                number: 23,
                name: "Forecast".to_string(),
                icon: "🔮",
                description: "Resource forecasting".to_string(),
            },
            JumpItem {
                number: 24,
                name: "RBAC".to_string(),
                icon: "👥",
                description: "Role-based access control".to_string(),
            },
            JumpItem {
                number: 25,
                name: "Custom Metrics".to_string(),
                icon: "📊",
                description: "Custom metric dashboards".to_string(),
            },
            JumpItem {
                number: 26,
                name: "Autoscaler".to_string(),
                icon: "⚖️",
                description: "Auto-scaling policies".to_string(),
            },
            JumpItem {
                number: 27,
                name: "Security Posture".to_string(),
                icon: "🔒",
                description: "Security posture analysis".to_string(),
            },
            JumpItem {
                number: 28,
                name: "AI Troubleshoot".to_string(),
                icon: "🤖",
                description: "AI-powered diagnostics".to_string(),
            },
            JumpItem {
                number: 29,
                name: "Approvals".to_string(),
                icon: "✅",
                description: "Change approval workflows".to_string(),
            },
            JumpItem {
                number: 30,
                name: "Macros".to_string(),
                icon: "🎬",
                description: "Recorded action macros".to_string(),
            },
            JumpItem {
                number: 31,
                name: "Sessions".to_string(),
                icon: "🤝",
                description: "Shared session management".to_string(),
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
