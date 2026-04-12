// Menu Widget - Context menus and action selection
use crate::tui::colors::tui as colors;

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub struct MenuItem {
    pub label: String,
    pub key: char,
    pub enabled: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<String>, key: char) -> Self {
        Self {
            label: label.into(),
            key,
            enabled: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    pub title: String,
    pub items: Vec<MenuItem>,
    pub selected: usize,
    pub x: u16,
    pub y: u16,
}

impl Menu {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
            selected: 0,
            x: 0,
            y: 0,
        }
    }

    pub fn add_item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn at_position(mut self, x: u16, y: u16) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }

        loop {
            self.selected = (self.selected + 1) % self.items.len();
            if self.items[self.selected].enabled {
                break;
            }
            // Prevent infinite loop if all items disabled
            if self.items.iter().all(|i| !i.enabled) {
                break;
            }
        }
    }

    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }

        loop {
            self.selected = if self.selected == 0 {
                self.items.len() - 1
            } else {
                self.selected - 1
            };
            if self.items[self.selected].enabled {
                break;
            }
            // Prevent infinite loop if all items disabled
            if self.items.iter().all(|i| !i.enabled) {
                break;
            }
        }
    }

    pub fn selected_item(&self) -> Option<&MenuItem> {
        self.items.get(self.selected)
    }

    pub fn select_by_key(&mut self, key: char) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.key == key && item.enabled)
            .inspect(|&index| {
                self.selected = index;
            })
    }

    pub fn render(&self, f: &mut Frame) {
        let area = f.area();

        // Calculate menu dimensions
        let max_label_len = self.items.iter().map(|i| i.label.len()).max().unwrap_or(10);
        let width = (max_label_len + 8).min(40) as u16; // +8 for borders and key indicator
        let height = (self.items.len() + 2).min(15) as u16; // +2 for borders

        // Position menu
        let menu_area = Rect {
            x: self.x.min(area.width.saturating_sub(width)),
            y: self.y.min(area.height.saturating_sub(height)),
            width,
            height,
        };

        // Clear background
        f.render_widget(Clear, menu_area);

        // Main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::ORANGE))
            .title(Span::styled(
                &self.title,
                Style::default()
                    .fg(colors::ORANGE)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center);

        let inner = block.inner(menu_area);
        f.render_widget(block, menu_area);

        // Render items
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = i == self.selected;
                let style = if !item.enabled {
                    Style::default().fg(colors::TEXT_MUTED)
                } else if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(colors::ORANGE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let content = format!("{}{} ({})", prefix, item.label, item.key);

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, inner);
    }
}

/// Create a context menu for VM actions
pub fn vm_action_menu(vm_name: &str, is_running: bool) -> Menu {
    let mut menu = Menu::new(format!("VM: {}", vm_name));

    if is_running {
        menu = menu
            .add_item(MenuItem::new("Stop", 's'))
            .add_item(MenuItem::new("Restart", 'r'))
            .add_item(MenuItem::new("Create Snapshot", 'c'))
            .add_item(MenuItem::new("Console", 'o').disabled());
    } else {
        menu = menu
            .add_item(MenuItem::new("Start", 's'))
            .add_item(MenuItem::new("Delete", 'd'))
            .add_item(MenuItem::new("Create Snapshot", 'c').disabled());
    }

    menu.add_item(MenuItem::new("View Details", 'v'))
        .add_item(MenuItem::new("Edit", 'e').disabled())
}
