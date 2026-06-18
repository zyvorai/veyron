// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Interactive TUI Application - Enhanced with dialogs, menus, and notifications

use super::config::TuiConfig;
use super::state::AppState;
use super::ui::vm_details::TAB_NAMES;
use super::widgets::{
    Dialog, InputDialog, InputField, Menu, NotificationManager, ProgressBar, QuickJumpMenu,
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use std::time::Duration;

/// Interactive mode - what the user is currently doing
#[derive(Debug, Clone, PartialEq)]
pub enum InteractiveMode {
    Normal,
    Dialog(Dialog),
    Input(InputDialog),
    Menu(Menu),
    Progress(ProgressBar),
    QuickJump(QuickJumpMenu),
}

/// TUI View enum - different screens in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    VmList,
    VmDetails,
    Snapshots,
    Profiles,
    Blueprints,
    ActivityLog,
    Help,
    // Extended views
    Nodes,
    Pods,
    Events,
    ClusterHealth,
    VmiTable,
    Topology,
    MigrationWizard,
    SecurityDashboard,
    // Additional views
    CostAnalytics,
    Compliance,
    VulnerabilityScanner,
    AuditTrail,
    Timeline,
    PerformanceProfiler,
    // More views
    NlpSearch,
    DependencyGraph,
    ForecastDashboard,
    RbacVisualizer,
    CustomMetrics,
    Autoscaler,
    SecurityPosture,
    AiTroubleshoot,
    ChangeApproval,
    MacroView,
    SessionSharing,
}

impl View {
    /// Display name for breadcrumb / status bar
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::VmList => "VMs",
            Self::VmDetails => "VM Details",
            Self::Snapshots => "Snapshots",
            Self::Profiles => "Profiles",
            Self::Blueprints => "Blueprints",
            Self::ActivityLog => "Activity",
            Self::Help => "Help",
            Self::Nodes => "Nodes",
            Self::Pods => "Pods",
            Self::Events => "Events",
            Self::ClusterHealth => "Cluster Health",
            Self::VmiTable => "VMIs",
            Self::Topology => "Topology",
            Self::MigrationWizard => "Migration",
            Self::SecurityDashboard => "Security",
            Self::CostAnalytics => "Costs",
            Self::Compliance => "Compliance",
            Self::VulnerabilityScanner => "Vulnerabilities",
            Self::AuditTrail => "Audit",
            Self::Timeline => "Timeline",
            Self::PerformanceProfiler => "Performance",
            Self::NlpSearch => "Search",
            Self::DependencyGraph => "Dependencies",
            Self::ForecastDashboard => "Forecast",
            Self::RbacVisualizer => "RBAC",
            Self::CustomMetrics => "Metrics",
            Self::Autoscaler => "Autoscaler",
            Self::SecurityPosture => "Posture",
            Self::AiTroubleshoot => "AI Debug",
            Self::ChangeApproval => "Approvals",
            Self::MacroView => "Macros",
            Self::SessionSharing => "Sessions",
        }
    }
}

/// Enhanced Interactive TUI Application
pub struct InteractiveApp {
    /// Should the application quit
    pub should_quit: bool,

    /// Current view
    pub current_view: View,

    /// Application state
    pub state: AppState,

    /// TUI configuration
    pub config: TuiConfig,

    /// Current interactive mode
    pub mode: InteractiveMode,

    /// Notification manager
    pub notifications: NotificationManager,

    /// Search filter text
    pub search_filter: String,

    /// Show search bar
    pub show_search: bool,

    /// Search case sensitive
    pub search_case_sensitive: bool,

    /// Search regex mode
    pub search_regex: bool,

    /// Current tab in VM Details view (0=Overview, 1=Network, 2=Events)
    pub detail_tab: usize,

    /// VM name targeted by the current dialog (to avoid race conditions)
    pub dialog_vm_name: Option<String>,

    /// View navigation history for breadcrumb back-navigation
    pub view_history: Vec<View>,

    /// Flag to trigger data refresh when switching views
    pub needs_view_refresh: bool,
}

impl InteractiveApp {
    /// Parse default view from config string
    fn parse_default_view(name: &str) -> View {
        match name {
            "vms" | "vmlist" | "vm_list" => View::VmList,
            "snapshots" => View::Snapshots,
            "profiles" => View::Profiles,
            "blueprints" => View::Blueprints,
            "activity" | "activity_log" => View::ActivityLog,
            "nodes" => View::Nodes,
            "events" => View::Events,
            _ => View::Dashboard,
        }
    }

    /// Create a new interactive TUI application
    pub fn new(namespace: String) -> Result<Self> {
        let config = TuiConfig::load()?;
        let state = AppState::new(namespace).with_refresh_interval(config.ui.auto_refresh_interval);
        let default_view = Self::parse_default_view(&config.ui.default_view);

        Ok(Self {
            should_quit: false,
            current_view: default_view,
            state,
            config,
            mode: InteractiveMode::Normal,
            notifications: NotificationManager::new(),
            search_filter: String::new(),
            show_search: false,
            search_case_sensitive: false,
            search_regex: false,
            detail_tab: 0,
            dialog_vm_name: None,
            view_history: Vec::new(),
            needs_view_refresh: false,
        })
    }

    /// Create with custom config
    pub fn with_config(namespace: String, config: TuiConfig) -> Self {
        let state = AppState::new(namespace).with_refresh_interval(config.ui.auto_refresh_interval);
        let default_view = Self::parse_default_view(&config.ui.default_view);

        Self {
            should_quit: false,
            current_view: default_view,
            state,
            config,
            mode: InteractiveMode::Normal,
            notifications: NotificationManager::new(),
            search_filter: String::new(),
            show_search: false,
            search_case_sensitive: false,
            search_regex: false,
            detail_tab: 0,
            dialog_vm_name: None,
            view_history: Vec::new(),
            needs_view_refresh: false,
        }
    }

    /// Run the TUI application event loop
    pub async fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<()> {
        // Initial data load
        self.refresh_data().await?;

        loop {
            // Update notifications
            self.notifications.update();

            // Animate spinner if in progress mode
            if let InteractiveMode::Progress(ref mut progress) = self.mode {
                progress.frame = progress.frame.wrapping_add(1);
            }

            // Render UI
            terminal.draw(|f| self.render(f))?;

            // Handle input
            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key).await?,
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    _ => {}
                }
            }

            // Refresh data when switching to a new view
            if self.needs_view_refresh {
                self.needs_view_refresh = false;
                self.refresh_data().await?;
            }

            // Auto-refresh
            if self.state.should_refresh() && matches!(self.mode, InteractiveMode::Normal) {
                self.refresh_data().await?;
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Render the UI
    fn render(&mut self, f: &mut ratatui::Frame) {
        use super::ui;
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::Paragraph,
        };

        let full_area = f.area();

        // Show breadcrumb bar for non-root views when there's navigation history
        let (breadcrumb_area, content_area) = if !self.view_history.is_empty()
            && !matches!(self.current_view, View::Dashboard | View::VmList)
        {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(full_area);

            // Build breadcrumb: "Dashboard > VMs > VM Details"
            let mut spans: Vec<Span> = Vec::new();
            spans.push(Span::styled(" ", Style::default()));

            // Show last 3 history items at most
            let history_start = self.view_history.len().saturating_sub(3);
            for (i, view) in self.view_history[history_start..].iter().enumerate() {
                if i > 0 {
                    spans.push(Span::styled(
                        " > ",
                        Style::default().fg(Color::Rgb(100, 90, 110)),
                    ));
                }
                spans.push(Span::styled(
                    view.label(),
                    Style::default().fg(Color::Rgb(140, 130, 150)),
                ));
            }
            spans.push(Span::styled(
                " > ",
                Style::default().fg(Color::Rgb(100, 90, 110)),
            ));
            spans.push(Span::styled(
                self.current_view.label(),
                Style::default()
                    .fg(Color::Rgb(255, 145, 115))
                    .add_modifier(Modifier::BOLD),
            ));

            let breadcrumb = Paragraph::new(Line::from(spans))
                .style(Style::default().bg(Color::Rgb(30, 25, 40)));
            f.render_widget(breadcrumb, layout[0]);

            (Some(layout[0]), layout[1])
        } else {
            (None, full_area)
        };

        let _ = breadcrumb_area; // used for rendering above

        // Render current view
        match self.current_view {
            View::Dashboard => ui::dashboard::render(f, &self.state, &self.config),
            View::VmList => ui::vm_list::render(f, &mut self.state, &self.config),
            View::VmDetails => {
                ui::vm_details::render(f, &self.state, &self.config, self.detail_tab)
            }
            View::Snapshots => ui::snapshots::render(f, &self.state, &self.config),
            View::Profiles => ui::profiles::render(f, &self.state, &self.config),
            View::Blueprints => ui::blueprints::render(f, &self.state, &self.config),
            View::ActivityLog => ui::activity_log::render(f, &self.state, &self.config),
            View::Help => ui::help::render(f, &self.config),
            // Extended views - real data where available
            View::Nodes => ui::nodes_table::render(f, content_area, &self.state),
            View::Pods => ui::pods_table::render(f, content_area, &self.state),
            View::Events => ui::events_table::render(f, content_area, &self.state),
            View::VmiTable => ui::vmi_table::render(f, content_area, &self.state),
            // Preview views - hardcoded sample data
            View::ClusterHealth => {
                ui::cluster_health::render(f, content_area);
                Self::render_preview_badge(f, content_area);
            }
            View::Topology => {
                ui::topology::render(f, content_area);
                Self::render_preview_badge(f, content_area);
            }
            View::MigrationWizard => {
                ui::migration_wizard::render(f, content_area, &self.state);
            }
            View::SecurityDashboard => {
                ui::security_dashboard::render(f, content_area, &self.state);
            }
            View::CostAnalytics => {
                ui::cost_analytics::render(f, content_area, &self.state);
            }
            View::Compliance => {
                ui::compliance_checker::render(f, content_area, &self.state);
            }
            View::VulnerabilityScanner => {
                ui::vulnerability_scanner::render(f, content_area, &self.state);
            }
            View::AuditTrail => {
                ui::audit_panel::render(f, content_area, &self.state);
            }
            View::Timeline => {
                ui::timeline::render(f, content_area, &self.state);
            }
            View::PerformanceProfiler => {
                ui::performance_profiler::render(f, content_area, &self.state);
            }
            View::NlpSearch => {
                ui::nlp_search::render(f, content_area, &self.state);
            }
            View::DependencyGraph => {
                ui::dependency_graph::render(f, content_area, &self.state);
            }
            View::ForecastDashboard => {
                ui::forecast_dashboard::render(f, content_area, &self.state);
            }
            View::RbacVisualizer => {
                ui::rbac_visualizer::render(f, content_area, &self.state);
            }
            View::CustomMetrics => {
                ui::custom_metrics::render(f, content_area, &self.state);
            }
            View::Autoscaler => {
                ui::autoscaler::render(f, content_area, &self.state);
            }
            View::SecurityPosture => {
                ui::security_posture::render(f, content_area, &self.state);
            }
            View::AiTroubleshoot => {
                ui::ai_troubleshoot::render(f, content_area, &self.state);
            }
            View::ChangeApproval => {
                ui::change_approval::render(f, content_area, &self.state);
            }
            View::MacroView => {
                ui::macro_view::render(f, content_area, &self.state);
            }
            View::SessionSharing => {
                ui::session_sharing::render(f, content_area, &self.state);
            }
        }

        // Render search bar if active
        if self.show_search {
            self.render_search_bar(f);
        }

        // Render notifications
        self.notifications.render(f);

        // Render interactive overlay
        match &self.mode {
            InteractiveMode::Dialog(dialog) => dialog.render(f),
            InteractiveMode::Input(input) => input.render(f),
            InteractiveMode::Menu(menu) => menu.render(f),
            InteractiveMode::Progress(progress) => progress.render(f),
            InteractiveMode::QuickJump(qj) => qj.render(f),
            InteractiveMode::Normal => {}
        }
    }

    /// Render search bar with mode indicators
    fn render_search_bar(&self, f: &mut ratatui::Frame) {
        use ratatui::{
            layout::{Alignment, Rect},
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Clear, Paragraph},
        };

        let area = f.area();
        let search_area = Rect {
            x: area.width / 4,
            y: area.height.saturating_sub(5),
            width: area.width / 2,
            height: 4,
        };

        f.render_widget(Clear, search_area);

        let mode_str = if self.search_regex {
            "regex"
        } else {
            "literal"
        };
        let case_str = if self.search_case_sensitive {
            "Aa"
        } else {
            "aa"
        };

        let search_line = Line::from(vec![
            Span::styled(
                format!("Search [{}|{}]: ", mode_str, case_str),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}█", self.search_filter),
                Style::default().fg(Color::White),
            ),
        ]);
        let help_line = Line::from(Span::styled(
            " Ctrl+R: mode │ Ctrl+I: case │ Enter: apply │ Esc: cancel",
            Style::default().fg(Color::DarkGray),
        ));

        let paragraph = Paragraph::new(vec![search_line, help_line])
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );

        f.render_widget(paragraph, search_area);
    }

    /// Handle keyboard input
    async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle interactive mode keys first
        if !matches!(self.mode, InteractiveMode::Normal) {
            return self.handle_interactive_key(key).await;
        }

        // Handle search mode
        if self.show_search {
            return self.handle_search_key(key);
        }

        // Global keys
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                if self.show_search {
                    self.show_search = false;
                } else {
                    match self.current_view {
                        // In main views, quit
                        View::Dashboard | View::VmList => {
                            self.should_quit = true;
                        }
                        // In sub-views, go back via history
                        _ => {
                            self.navigate_back();
                        }
                    }
                }
            }
            KeyCode::Char('?') => {
                self.current_view = View::Help;
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.notifications.info("Refreshing data...");
                self.refresh_data().await?;
                self.notifications.success("Data refreshed");
            }
            KeyCode::Char('/') => {
                self.show_search = true;
                self.search_filter.clear();
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.mode = InteractiveMode::QuickJump(QuickJumpMenu::new());
            }
            // View switching
            KeyCode::Char('1') => self.navigate_to(View::Dashboard),
            KeyCode::Char('2') => self.navigate_to(View::VmList),
            KeyCode::Char('3') => self.navigate_to(View::Snapshots),
            KeyCode::Char('4') => self.navigate_to(View::Profiles),
            KeyCode::Char('5') => self.navigate_to(View::Blueprints),
            KeyCode::Char('6') => self.navigate_to(View::ActivityLog),
            KeyCode::Char('7') => self.navigate_to(View::Nodes),
            KeyCode::Char('8') => self.navigate_to(View::Events),
            KeyCode::Char('9') => self.navigate_to(View::ClusterHealth),
            KeyCode::Char('0') => self.navigate_to(View::Topology),
            _ => {
                // View-specific keys
                self.handle_view_key(key).await?;
            }
        }

        Ok(())
    }

    /// Handle keys in interactive mode
    async fn handle_interactive_key(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.mode {
            InteractiveMode::Dialog(dialog) => match key.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                    dialog.toggle_selection();
                }
                KeyCode::Enter => {
                    let confirmed = dialog.selected;
                    let dialog_clone = dialog.clone();
                    self.mode = InteractiveMode::Normal;

                    if confirmed {
                        self.handle_dialog_confirm(&dialog_clone).await?;
                    }
                }
                KeyCode::Esc => {
                    self.mode = InteractiveMode::Normal;
                }
                _ => {}
            },
            InteractiveMode::Input(input) => match key.code {
                KeyCode::Char(c) => {
                    input.add_char(c);
                }
                KeyCode::Backspace => {
                    input.delete_char();
                }
                KeyCode::Tab => {
                    input.next_field();
                }
                KeyCode::BackTab => {
                    input.prev_field();
                }
                KeyCode::Enter => {
                    input.submit();
                    let input_clone = input.clone();
                    self.mode = InteractiveMode::Normal;
                    self.handle_input_submit(&input_clone).await?;
                }
                KeyCode::Esc => {
                    self.mode = InteractiveMode::Normal;
                }
                _ => {}
            },
            InteractiveMode::Menu(menu) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    menu.previous();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    menu.next();
                }
                KeyCode::Enter => {
                    if let Some(item) = menu.selected_item() {
                        let key_char = item.key;
                        self.mode = InteractiveMode::Normal;
                        self.handle_menu_selection(key_char).await?;
                    }
                }
                KeyCode::Char(c) if menu.select_by_key(c).is_some() => {
                    self.mode = InteractiveMode::Normal;
                    self.handle_menu_selection(c).await?;
                }
                KeyCode::Esc => {
                    self.mode = InteractiveMode::Normal;
                }
                _ => {}
            },
            InteractiveMode::Progress(_) => {
                // Progress bars don't accept input
                if matches!(key.code, KeyCode::Esc) {
                    self.mode = InteractiveMode::Normal;
                }
            }
            InteractiveMode::QuickJump(qj) => match key.code {
                KeyCode::Up => {
                    qj.previous();
                }
                KeyCode::Down => {
                    qj.next();
                }
                KeyCode::Backspace => {
                    qj.delete_char();
                }
                KeyCode::Enter => {
                    if let Some(num) = qj.selected_number() {
                        self.mode = InteractiveMode::Normal;
                        match num {
                            1 => self.navigate_to(View::Dashboard),
                            2 => self.navigate_to(View::VmList),
                            3 => self.navigate_to(View::Snapshots),
                            4 => self.navigate_to(View::Profiles),
                            5 => self.navigate_to(View::Blueprints),
                            6 => self.navigate_to(View::ActivityLog),
                            7 => self.navigate_to(View::Nodes),
                            8 => self.navigate_to(View::Events),
                            9 => self.navigate_to(View::ClusterHealth),
                            10 => self.navigate_to(View::Topology),
                            11 => self.navigate_to(View::Pods),
                            12 => self.navigate_to(View::VmiTable),
                            13 => self.navigate_to(View::MigrationWizard),
                            14 => self.navigate_to(View::SecurityDashboard),
                            15 => self.navigate_to(View::CostAnalytics),
                            16 => self.navigate_to(View::Compliance),
                            17 => self.navigate_to(View::VulnerabilityScanner),
                            18 => self.navigate_to(View::AuditTrail),
                            19 => self.navigate_to(View::Timeline),
                            20 => self.navigate_to(View::PerformanceProfiler),
                            21 => self.navigate_to(View::NlpSearch),
                            22 => self.navigate_to(View::DependencyGraph),
                            23 => self.navigate_to(View::ForecastDashboard),
                            24 => self.navigate_to(View::RbacVisualizer),
                            25 => self.navigate_to(View::CustomMetrics),
                            26 => self.navigate_to(View::Autoscaler),
                            27 => self.navigate_to(View::SecurityPosture),
                            28 => self.navigate_to(View::AiTroubleshoot),
                            29 => self.navigate_to(View::ChangeApproval),
                            30 => self.navigate_to(View::MacroView),
                            31 => self.navigate_to(View::SessionSharing),
                            _ => {}
                        }
                    }
                }
                KeyCode::Esc => {
                    self.mode = InteractiveMode::Normal;
                }
                KeyCode::Char(c) => {
                    qj.add_char(c);
                }
                _ => {}
            },
            InteractiveMode::Normal => {}
        }

        Ok(())
    }

    /// Handle search bar keys
    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search_regex = !self.search_regex;
                let mode = if self.search_regex {
                    "regex"
                } else {
                    "literal"
                };
                self.notifications.info(format!("Search mode: {}", mode));
            }
            KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search_case_sensitive = !self.search_case_sensitive;
                let mode = if self.search_case_sensitive {
                    "case-sensitive"
                } else {
                    "case-insensitive"
                };
                self.notifications.info(format!("Search: {}", mode));
            }
            KeyCode::Char(c) => {
                self.search_filter.push(c);
                self.apply_search_filter();
            }
            KeyCode::Backspace => {
                self.search_filter.pop();
                self.apply_search_filter();
            }
            KeyCode::Enter | KeyCode::Esc => {
                self.show_search = false;
                // Keep search applied if there is a filter, clear if empty
                if self.search_filter.is_empty() {
                    self.state.search_query.clear();
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Apply the current search filter to state
    fn apply_search_filter(&mut self) {
        self.state.search_query = self.search_filter.clone();
        self.state.search_case_sensitive = self.search_case_sensitive;
        self.state.search_regex = self.search_regex;
    }

    /// Handle view-specific keys
    async fn handle_view_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.current_view {
            View::Dashboard if key.code == KeyCode::Char('i') => {
                self.state.show_stats_bar = !self.state.show_stats_bar;
            }
            View::VmList => self.handle_vm_list_key(key).await?,
            View::VmDetails => self.handle_vm_details_key(key)?,
            View::Snapshots => self.handle_snapshots_key(key).await?,
            View::Profiles | View::Blueprints if key.code == KeyCode::Backspace => {
                self.navigate_back();
            }
            View::ActivityLog => match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.state.activity_scroll_down(),
                KeyCode::Up | KeyCode::Char('k') => self.state.activity_scroll_up(),
                KeyCode::Backspace => self.navigate_back(),
                _ => {}
            },
            View::Nodes => match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.state.select_next_node(),
                KeyCode::Up | KeyCode::Char('k') => self.state.select_previous_node(),
                KeyCode::Backspace => self.navigate_back(),
                _ => {}
            },
            View::Pods => match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.state.select_next_pod(),
                KeyCode::Up | KeyCode::Char('k') => self.state.select_previous_pod(),
                KeyCode::Backspace => self.navigate_back(),
                _ => {}
            },
            View::Events => match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.state.select_next_event(),
                KeyCode::Up | KeyCode::Char('k') => self.state.select_previous_event(),
                KeyCode::Backspace => self.navigate_back(),
                _ => {}
            },
            View::VmiTable => match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.state.select_next_vmi(),
                KeyCode::Up | KeyCode::Char('k') => self.state.select_previous_vmi(),
                KeyCode::Backspace => self.navigate_back(),
                _ => {}
            },
            View::ClusterHealth
            | View::Topology
            | View::MigrationWizard
            | View::SecurityDashboard
            | View::CostAnalytics
            | View::Compliance
            | View::VulnerabilityScanner
            | View::AuditTrail
            | View::Timeline
            | View::PerformanceProfiler
            | View::NlpSearch
            | View::DependencyGraph
            | View::ForecastDashboard
            | View::RbacVisualizer
            | View::CustomMetrics
            | View::Autoscaler
            | View::SecurityPosture
            | View::AiTroubleshoot
            | View::ChangeApproval
            | View::MacroView
            | View::SessionSharing
                if key.code == KeyCode::Backspace =>
            {
                self.navigate_back();
            }
            _ => {}
        }
        Ok(())
    }

    /// Mouse: scroll to move selection; left-click selects row on list views.
    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::{MouseButton, MouseEventKind};
        if !matches!(self.mode, InteractiveMode::Normal) {
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollDown => match self.current_view {
                View::VmList => self.state.select_next(),
                View::Nodes => self.state.select_next_node(),
                View::Pods => self.state.select_next_pod(),
                View::Events => self.state.select_next_event(),
                _ => {}
            },
            MouseEventKind::ScrollUp => match self.current_view {
                View::VmList => self.state.select_previous(),
                View::Nodes => self.state.select_previous_node(),
                View::Pods => self.state.select_previous_pod(),
                View::Events => self.state.select_previous_event(),
                _ => {}
            },
            MouseEventKind::Down(MouseButton::Left) if self.current_view == View::VmList => {
                let row = mouse.row.saturating_sub(4) as usize;
                if row < self.state.filtered_vms().len() {
                    self.state.selected_index = row;
                }
            }
            _ => {}
        }
    }

    fn export_vm_list_csv(&self) -> Result<std::path::PathBuf> {
        let dir = crate::utils::data_dir()?;
        let path = dir.join(format!(
            "vm-export-{}.csv",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));
        let mut lines = vec!["namespace,name,status,cpu,memory,node".to_string()];
        for vm in self.state.filtered_vms() {
            lines.push(format!(
                "{},{},{},{},{},{}",
                vm.namespace, vm.name, vm.status, vm.cpu, vm.memory, vm.node
            ));
        }
        std::fs::write(&path, lines.join("\n"))?;
        Ok(path)
    }

    /// Handle VM list view keys
    async fn handle_vm_list_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle Ctrl+ combinations first
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('a') => {
                    if self.state.multi_select_mode {
                        self.state.select_all();
                        let count = self.state.selected_items.len();
                        self.notifications
                            .info(format!("Selected all {} VMs", count));
                    }
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next();
            }
            KeyCode::Enter => {
                self.navigate_to(View::VmDetails);
                self.detail_tab = 0;
                // Clear cached VMI detail to trigger fresh fetch
                self.state.selected_vmi_name = None;
                self.state.selected_vmi_detail = None;
                let _ = self.state.refresh_selected_vm_detail().await;
            }
            KeyCode::Char('m') => {
                // Show context menu
                if let Some(vm) = self.state.selected_vm() {
                    let is_running = vm.status == "Running";
                    let menu = super::widgets::menu::vm_action_menu(&vm.name, is_running);
                    self.mode = InteractiveMode::Menu(menu);
                }
            }
            KeyCode::Char('v') => {
                // Toggle multi-select mode (vim visual mode style)
                self.state.toggle_multi_select();
                let mode = if self.state.multi_select_mode {
                    "ON"
                } else {
                    "OFF"
                };
                self.notifications.info(format!("Multi-select {}", mode));
            }
            KeyCode::Char(' ')
                // Toggle selection of current item in multi-select mode
                if self.state.multi_select_mode => {
                    self.state.toggle_current_selection();
                }
            KeyCode::Char('o') => {
                // Cycle sort mode
                self.state.cycle_sort_mode();
                self.notifications
                    .info(format!("Sort: {}", self.state.sort_mode.display()));
            }
            KeyCode::Char('c') => {
                // Create VM - show input dialog
                self.show_create_vm_dialog();
            }
            KeyCode::Char('s') => {
                if self.state.multi_select_mode && !self.state.selected_items.is_empty() {
                    // Batch start
                    let names: Vec<String> = self.collect_selected_vm_names();
                    let count = names.len();
                    for name in &names {
                        let _ = self.start_vm(name).await;
                    }
                    self.state.toggle_multi_select();
                    self.notifications.success(format!("Started {} VMs", count));
                } else if let Some(vm) = self.state.selected_vm() {
                    if vm.status != "Running" {
                        let vm_name = vm.name.clone();
                        let dialog =
                            Dialog::confirm("Start VM", format!("Start VM '{}'?", vm_name));
                        self.dialog_vm_name = Some(vm_name);
                        self.mode = InteractiveMode::Dialog(dialog);
                    }
                }
            }
            KeyCode::Char('x') => {
                if self.state.multi_select_mode && !self.state.selected_items.is_empty() {
                    // Batch stop
                    let names: Vec<String> = self.collect_selected_vm_names();
                    let count = names.len();
                    for name in &names {
                        let _ = self.stop_vm(name).await;
                    }
                    self.state.toggle_multi_select();
                    self.notifications.success(format!("Stopped {} VMs", count));
                } else if let Some(vm) = self.state.selected_vm() {
                    if vm.status == "Running" {
                        let vm_name = vm.name.clone();
                        if self.config.behavior.confirm_stop {
                            let dialog =
                                Dialog::confirm("Stop VM", format!("Stop VM '{}'?", vm_name));
                            self.dialog_vm_name = Some(vm_name);
                            self.mode = InteractiveMode::Dialog(dialog);
                        } else {
                            self.stop_vm(&vm_name).await?;
                        }
                    }
                }
            }
            KeyCode::Char('f') => {
                // Cycle status filter
                self.state.cycle_status_filter();
                let filter_name = self.state.status_filter.as_deref().unwrap_or("All");
                self.notifications.info(format!("Filter: {}", filter_name));
            }
            KeyCode::Char('E') => match self.export_vm_list_csv() {
                Ok(path) => self
                    .notifications
                    .success(format!("Exported VMs to {}", path.display())),
                Err(e) => self.notifications.error(format!("Export failed: {}", e)),
            },
            KeyCode::Char('d') => {
                if self.state.multi_select_mode && !self.state.selected_items.is_empty() {
                    // Batch delete with confirmation
                    let names: Vec<String> = self.collect_selected_vm_names();
                    let count = names.len();
                    let dialog = Dialog::confirm(
                        "Batch Delete VMs",
                        format!("⚠ Permanently delete {} VMs? This cannot be undone!", count),
                    );
                    self.mode = InteractiveMode::Dialog(dialog);
                } else if let Some(vm) = self.state.selected_vm() {
                    let vm_name = vm.name.clone();
                    if self.config.behavior.confirm_delete {
                        let dialog = Dialog::confirm(
                            "Delete VM",
                            format!(
                                "⚠ Permanently delete VM '{}'? This cannot be undone!",
                                vm_name
                            ),
                        );
                        self.dialog_vm_name = Some(vm_name);
                        self.mode = InteractiveMode::Dialog(dialog);
                    } else {
                        self.delete_vm(&vm_name).await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Collect VM names from multi-select
    fn collect_selected_vm_names(&self) -> Vec<String> {
        let filtered = self.state.filtered_vms();
        self.state
            .selected_items
            .iter()
            .filter_map(|&i| filtered.get(i).map(|vm| vm.name.clone()))
            .collect()
    }

    /// Handle VM details view keys
    fn handle_vm_details_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Backspace => {
                self.navigate_back();
                self.detail_tab = 0;
                self.state.detail_events_scroll = 0;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.detail_tab > 0 {
                    self.detail_tab -= 1;
                } else {
                    self.detail_tab = TAB_NAMES.len() - 1;
                }
                self.state.detail_events_scroll = 0;
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                self.detail_tab = (self.detail_tab + 1) % TAB_NAMES.len();
                self.state.detail_events_scroll = 0;
            }
            // Scroll within events tab
            KeyCode::Down | KeyCode::Char('j') if self.detail_tab == 2 => {
                self.state.detail_events_scroll = self.state.detail_events_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') if self.detail_tab == 2 => {
                self.state.detail_events_scroll = self.state.detail_events_scroll.saturating_sub(1);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle snapshots view keys
    async fn handle_snapshots_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_previous_snapshot();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next_snapshot();
            }
            KeyCode::Char('c') => {
                // Create snapshot
                self.show_create_snapshot_dialog();
            }
            KeyCode::Char('d') => {
                // Delete snapshot with confirmation
                if let Some(snap) = self.state.selected_snapshot() {
                    let snap_name = snap.name.clone();
                    let dialog = Dialog::confirm(
                        "Delete Snapshot",
                        format!("Delete snapshot '{}'? This cannot be undone!", snap_name),
                    );
                    self.dialog_vm_name = Some(snap_name);
                    self.mode = InteractiveMode::Dialog(dialog);
                }
            }
            KeyCode::Char('r') => {
                // Restore snapshot with confirmation
                if let Some(snap) = self.state.selected_snapshot() {
                    let snap_name = snap.name.clone();
                    let vm_name = snap.vm_name.clone();
                    let dialog = Dialog::confirm(
                        "Restore Snapshot",
                        format!("Restore VM '{}' from snapshot '{}'?", vm_name, snap_name),
                    );
                    self.dialog_vm_name = Some(snap_name);
                    self.mode = InteractiveMode::Dialog(dialog);
                }
            }
            KeyCode::Backspace => {
                self.navigate_back();
            }
            _ => {}
        }
        Ok(())
    }

    /// Show create VM dialog
    fn show_create_vm_dialog(&mut self) {
        let dialog = InputDialog::new("Create Virtual Machine")
            .add_field(InputField::new("VM Name:", "my-vm"))
            .add_field(InputField::new("Template:", "ubuntu-22.04"))
            .add_field(InputField::new("Profile:", "dev"))
            .add_field(InputField::new("Disk Size:", "20Gi"));

        self.mode = InteractiveMode::Input(dialog);
    }

    /// Show create snapshot dialog
    fn show_create_snapshot_dialog(&mut self) {
        if let Some(vm) = self.state.selected_vm() {
            let dialog = InputDialog::new("Create VM Snapshot")
                .add_field(InputField::new("VM Name:", &vm.name).with_value(&vm.name))
                .add_field(InputField::new("Snapshot Name:", "backup-YYYYMMDD"));

            self.mode = InteractiveMode::Input(dialog);
        }
    }

    /// Handle dialog confirmation
    async fn handle_dialog_confirm(&mut self, dialog: &Dialog) -> Result<()> {
        if dialog.title.contains("Batch Delete") {
            // Batch delete all selected VMs
            let names = self.collect_selected_vm_names();
            let count = names.len();
            for name in &names {
                let _ = self.delete_vm(name).await;
            }
            self.state.toggle_multi_select();
            self.notifications.success(format!("Deleted {} VMs", count));
        } else if dialog.title.contains("Delete Snapshot") {
            if let Some(snap_name) = self.dialog_vm_name.take() {
                self.delete_snapshot(&snap_name).await?;
            }
        } else if dialog.title.contains("Restore Snapshot") {
            if let Some(snap_name) = self.dialog_vm_name.take() {
                self.restore_snapshot(&snap_name).await?;
            }
        } else if let Some(vm_name) = self.dialog_vm_name.take() {
            // Single VM operations
            if dialog.title.contains("Start VM") {
                self.start_vm(&vm_name).await?;
            } else if dialog.title.contains("Stop VM") {
                self.stop_vm(&vm_name).await?;
            } else if dialog.title.contains("Delete VM") {
                self.delete_vm(&vm_name).await?;
            }
        }
        Ok(())
    }

    /// Handle input submission
    async fn handle_input_submit(&mut self, input: &InputDialog) -> Result<()> {
        if input.title.contains("Create Virtual Machine") {
            self.create_vm_from_input(input).await?;
        } else if input.title.contains("Create VM Snapshot") {
            self.create_snapshot_from_input(input).await?;
        }
        Ok(())
    }

    /// Handle menu selection
    async fn handle_menu_selection(&mut self, key: char) -> Result<()> {
        match key {
            's' => {
                if let Some(vm) = self.state.selected_vm() {
                    let vm_name = vm.name.clone();
                    if vm.status == "Running" {
                        self.stop_vm(&vm_name).await?;
                    } else {
                        self.start_vm(&vm_name).await?;
                    }
                }
            }
            'r' => {
                // Restart VM
                if let Some(vm) = self.state.selected_vm() {
                    let vm_name = vm.name.clone();
                    self.restart_vm(&vm_name).await?;
                }
            }
            'c' => {
                // Create snapshot from context menu
                self.show_create_snapshot_dialog();
            }
            'd' => {
                if let Some(vm) = self.state.selected_vm() {
                    let vm_name = vm.name.clone();
                    let dialog = Dialog::confirm(
                        "Delete VM",
                        format!(
                            "⚠ Permanently delete VM '{}'? This cannot be undone!",
                            vm_name
                        ),
                    );
                    self.dialog_vm_name = Some(vm_name);
                    self.mode = InteractiveMode::Dialog(dialog);
                }
            }
            'v' => {
                self.navigate_to(View::VmDetails);
                self.detail_tab = 0;
                self.state.selected_vmi_name = None;
                self.state.selected_vmi_detail = None;
                let _ = self.state.refresh_selected_vm_detail().await;
            }
            _ => {}
        }
        Ok(())
    }

    /// Restart a VM
    async fn restart_vm(&mut self, vm_name: &str) -> Result<()> {
        use crate::kube::KubeClient;

        self.notifications
            .info(format!("Restarting VM '{}'...", vm_name));

        match KubeClient::new().await {
            Ok(client) => match client.restart_vm(&self.state.namespace, vm_name).await {
                Ok(_) => {
                    self.state
                        .record_activity("🔄 ", vm_name, "restart requested");
                    self.notifications
                        .success(format!("VM '{}' restarted", vm_name));
                    self.refresh_data().await?;
                }
                Err(e) => {
                    self.notifications
                        .error(format!("Failed to restart VM: {}", e));
                }
            },
            Err(e) => {
                self.notifications.error(format!("Kubernetes error: {}", e));
            }
        }

        Ok(())
    }

    /// Validate a VM name according to RFC 1123 DNS subdomain rules
    fn is_valid_vm_name(name: &str) -> bool {
        if name.is_empty() || name.len() > 253 {
            return false;
        }
        // RFC 1123 DNS subdomain: lowercase alphanumeric and hyphens, start/end with alphanumeric
        static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(r"^[a-z0-9]([a-z0-9\-]*[a-z0-9])?$").unwrap()
        });
        RE.is_match(name)
    }

    /// Create VM from input dialog
    async fn create_vm_from_input(&mut self, input: &InputDialog) -> Result<()> {
        let name = input.get_value(0).unwrap_or("");
        let template_name = input.get_value(1).unwrap_or("ubuntu-22.04");
        let profile_name = input.get_value(2).unwrap_or("dev");

        if !Self::is_valid_vm_name(name) {
            self.notifications.error(
                "Invalid VM name: must be 1-253 chars, lowercase alphanumeric and hyphens, \
                 starting and ending with alphanumeric (RFC 1123)"
                    .to_string(),
            );
            return Ok(());
        }

        self.notifications
            .info(format!("Creating VM '{}'...", name));

        // Look up template
        let mut config = match crate::templates::TEMPLATES.get(template_name) {
            Some(t) => t,
            None => {
                self.notifications
                    .error(format!("Unknown template: {}", template_name));
                return Ok(());
            }
        };
        config.name = name.to_string();
        config.namespace = self.state.namespace.clone();

        // Apply profile overrides (drop lock before await)
        {
            let profiles = crate::profiles::PROFILES
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;
            if let Some(profile) = profiles.get(profile_name) {
                config.cpu.cores = profile.cpu_cores;
                config.cpu.sockets = profile.cpu_sockets;
                config.cpu.threads = profile.cpu_threads;
                config.memory.size = profile.memory.clone();
            }
        }

        // Create VM via Kubernetes API
        match crate::kube::KubeClient::new().await {
            Ok(client) => {
                match client.create_vm(&config).await {
                    Ok(_) => {
                        self.notifications
                            .success(format!("VM '{}' created successfully", name));
                        // Refresh VM list
                        let _ = self.state.refresh_vms().await;
                    }
                    Err(e) => {
                        self.notifications
                            .error(format!("Failed to create VM: {}", e));
                    }
                }
            }
            Err(e) => {
                self.notifications
                    .error(format!("Failed to connect to cluster: {}", e));
            }
        }

        Ok(())
    }

    /// Create snapshot from input dialog
    async fn create_snapshot_from_input(&mut self, input: &InputDialog) -> Result<()> {
        let vm_name = input.get_value(0).unwrap_or("");
        let snapshot_name = input.get_value(1).unwrap_or("");

        if vm_name.is_empty() || snapshot_name.is_empty() {
            self.notifications
                .error("VM name and snapshot name are required".to_string());
            return Ok(());
        }

        self.notifications
            .info(format!("Creating snapshot '{}'...", snapshot_name));

        // Create snapshot via KubeVirt API
        match crate::snapshots::SnapshotManager::new(&self.state.namespace).await {
            Ok(manager) => {
                let config = crate::snapshots::SnapshotConfig::new(vm_name, snapshot_name);
                match manager.create_snapshot(&config).await {
                    Ok(_) => {
                        self.notifications
                            .success(format!("Snapshot '{}' created", snapshot_name));
                        let _ = self.state.refresh_snapshots().await;
                    }
                    Err(e) => {
                        self.notifications
                            .error(format!("Failed to create snapshot: {}", e));
                    }
                }
            }
            Err(e) => {
                self.notifications
                    .error(format!("Failed to connect to cluster: {}", e));
            }
        }

        Ok(())
    }

    /// Render a "PREVIEW" badge in the top-right corner for views with sample data
    fn render_preview_badge(f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::{
            layout::Rect,
            style::{Color, Modifier, Style},
            widgets::{Clear, Paragraph},
        };

        let badge_width = 11u16;
        if area.width < badge_width + 4 {
            return;
        }
        let badge_area = Rect {
            x: area.x + area.width - badge_width - 2,
            y: area.y,
            width: badge_width,
            height: 1,
        };
        f.render_widget(Clear, badge_area);
        let badge = Paragraph::new(" PREVIEW ").style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(255, 200, 0))
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(badge, badge_area);
    }

    /// Delete a snapshot
    async fn delete_snapshot(&mut self, snapshot_name: &str) -> Result<()> {
        self.notifications
            .info(format!("Deleting snapshot '{}'...", snapshot_name));

        match crate::snapshots::SnapshotManager::new(&self.state.namespace).await {
            Ok(manager) => match manager.delete_snapshot(snapshot_name).await {
                Ok(_) => {
                    self.state
                        .record_activity("🗑 ", snapshot_name, "snapshot deleted");
                    self.notifications
                        .success(format!("Snapshot '{}' deleted", snapshot_name));
                    let _ = self.state.refresh_snapshots().await;
                }
                Err(e) => {
                    self.notifications
                        .error(format!("Failed to delete snapshot: {}", e));
                }
            },
            Err(e) => {
                self.notifications
                    .error(format!("Failed to connect to cluster: {}", e));
            }
        }

        Ok(())
    }

    /// Restore a VM from snapshot
    async fn restore_snapshot(&mut self, snapshot_name: &str) -> Result<()> {
        // Find the VM name associated with this snapshot
        let vm_name = self
            .state
            .snapshots
            .iter()
            .find(|s| s.name == snapshot_name)
            .map(|s| s.vm_name.clone());

        let vm_name = match vm_name {
            Some(name) => name,
            None => {
                self.notifications
                    .error("Could not determine VM for this snapshot".to_string());
                return Ok(());
            }
        };

        self.notifications
            .info(format!("Restoring '{}' from snapshot...", vm_name));

        match crate::snapshots::restore::RestoreManager::new(&self.state.namespace).await {
            Ok(manager) => match manager.restore_in_place(&vm_name, snapshot_name).await {
                Ok(_) => {
                    self.state
                        .record_activity("♻ ", &vm_name, "restored from snapshot");
                    self.notifications
                        .success(format!("Restored '{}' from snapshot", vm_name));
                    let _ = self.refresh_data().await;
                }
                Err(e) => {
                    self.notifications
                        .error(format!("Failed to restore: {}", e));
                }
            },
            Err(e) => {
                self.notifications
                    .error(format!("Failed to connect to cluster: {}", e));
            }
        }

        Ok(())
    }

    /// Navigate to a view, pushing current to history
    fn navigate_to(&mut self, view: View) {
        if self.current_view != view {
            self.view_history.push(self.current_view);
            self.current_view = view;
            self.needs_view_refresh = true;
        }
    }

    /// Navigate back via history, or to VmList as fallback
    fn navigate_back(&mut self) {
        if let Some(prev) = self.view_history.pop() {
            self.current_view = prev;
        } else {
            self.current_view = View::VmList;
        }
        self.detail_tab = 0;
    }

    /// Refresh data from Kubernetes
    async fn refresh_data(&mut self) -> Result<()> {
        // Always refresh VMs (needed for dashboard, vm_list, etc.)
        if let Err(e) = self.state.refresh_vms().await {
            self.notifications
                .error(format!("Failed to refresh VMs: {}", e));
        }

        if let Err(e) = self.state.refresh_snapshots().await {
            // Snapshots might not be critical, just log
            eprintln!("Failed to refresh snapshots: {}", e);
        }

        // Lazy refresh: only fetch data for the current view
        match self.current_view {
            View::Nodes | View::Topology | View::ClusterHealth => {
                let _ = self.state.refresh_nodes().await;
            }
            View::Pods => {
                let _ = self.state.refresh_pods().await;
            }
            View::Events => {
                let _ = self.state.refresh_events().await;
            }
            View::VmiTable => {
                let _ = self.state.refresh_vmis().await;
            }
            _ => {}
        }

        self.state.update_history();

        Ok(())
    }

    /// Start a VM
    async fn start_vm(&mut self, vm_name: &str) -> Result<()> {
        use crate::kube::KubeClient;

        self.notifications
            .info(format!("Starting VM '{}'...", vm_name));

        match KubeClient::new().await {
            Ok(client) => match client.start_vm(&self.state.namespace, vm_name).await {
                Ok(_) => {
                    self.state.record_activity("▶ ", vm_name, "start requested");
                    self.notifications
                        .success(format!("VM '{}' started", vm_name));
                    self.refresh_data().await?;
                }
                Err(e) => {
                    self.notifications
                        .error(format!("Failed to start VM: {}", e));
                }
            },
            Err(e) => {
                self.notifications.error(format!("Kubernetes error: {}", e));
            }
        }

        Ok(())
    }

    /// Stop a VM
    async fn stop_vm(&mut self, vm_name: &str) -> Result<()> {
        use crate::kube::KubeClient;

        self.notifications
            .info(format!("Stopping VM '{}'...", vm_name));

        match KubeClient::new().await {
            Ok(client) => match client.stop_vm(&self.state.namespace, vm_name).await {
                Ok(_) => {
                    self.state.record_activity("⏹ ", vm_name, "stop requested");
                    self.notifications
                        .success(format!("VM '{}' stopped", vm_name));
                    self.refresh_data().await?;
                }
                Err(e) => {
                    self.notifications
                        .error(format!("Failed to stop VM: {}", e));
                }
            },
            Err(e) => {
                self.notifications.error(format!("Kubernetes error: {}", e));
            }
        }

        Ok(())
    }

    /// Delete a VM
    async fn delete_vm(&mut self, vm_name: &str) -> Result<()> {
        use crate::kube::KubeClient;

        self.notifications
            .info(format!("Deleting VM '{}'...", vm_name));

        match KubeClient::new().await {
            Ok(client) => match client.delete_vm(&self.state.namespace, vm_name).await {
                Ok(_) => {
                    self.state.record_activity("🗑 ", vm_name, "deleted");
                    self.notifications
                        .success(format!("VM '{}' deleted", vm_name));
                    self.refresh_data().await?;
                }
                Err(e) => {
                    self.notifications
                        .error(format!("Failed to delete VM: {}", e));
                }
            },
            Err(e) => {
                self.notifications.error(format!("Kubernetes error: {}", e));
            }
        }

        Ok(())
    }
}
