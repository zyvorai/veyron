// Interactive TUI Application - Enhanced with dialogs, menus, and notifications

use super::config::TuiConfig;
use super::state::AppState;
use super::ui::vm_details::TAB_NAMES;
use super::widgets::{Dialog, InputDialog, InputField, Menu, NotificationManager, ProgressBar};
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

    /// Current tab in VM Details view (0=Overview, 1=Network, 2=Events)
    pub detail_tab: usize,

    /// VM name targeted by the current dialog (to avoid race conditions)
    pub dialog_vm_name: Option<String>,
}

impl InteractiveApp {
    /// Create a new interactive TUI application
    pub fn new(namespace: String) -> Result<Self> {
        let config = TuiConfig::load()?;
        let state = AppState::new(namespace).with_refresh_interval(config.ui.auto_refresh_interval);

        Ok(Self {
            should_quit: false,
            current_view: View::Dashboard,
            state,
            config,
            mode: InteractiveMode::Normal,
            notifications: NotificationManager::new(),
            search_filter: String::new(),
            show_search: false,
            detail_tab: 0,
            dialog_vm_name: None,
        })
    }

    /// Create with custom config
    pub fn with_config(namespace: String, config: TuiConfig) -> Self {
        let state = AppState::new(namespace).with_refresh_interval(config.ui.auto_refresh_interval);

        Self {
            should_quit: false,
            current_view: View::Dashboard,
            state,
            config,
            mode: InteractiveMode::Normal,
            notifications: NotificationManager::new(),
            search_filter: String::new(),
            show_search: false,
            detail_tab: 0,
            dialog_vm_name: None,
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

            // Render UI
            terminal.draw(|f| self.render(f))?;

            // Handle input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key).await?;
                }
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

        // Render current view
        match self.current_view {
            View::Dashboard => ui::dashboard::render(f, &self.state, &self.config),
            View::VmList => ui::vm_list::render(f, &mut self.state, &self.config),
            View::VmDetails => ui::vm_details::render(f, &self.state, &self.config, self.detail_tab),
            View::Snapshots => ui::snapshots::render(f, &self.state, &self.config),
            View::Profiles => ui::profiles::render(f, &self.state, &self.config),
            View::Blueprints => ui::blueprints::render(f, &self.state, &self.config),
            View::ActivityLog => ui::activity_log::render(f, &self.state, &self.config),
            View::Help => ui::help::render(f, &self.config),
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
            InteractiveMode::Normal => {}
        }
    }

    /// Render search bar
    fn render_search_bar(&self, f: &mut ratatui::Frame) {
        use ratatui::{
            layout::{Alignment, Rect},
            style::{Color, Modifier, Style},
            widgets::{Block, Borders, Clear, Paragraph},
        };

        let area = f.area();
        let search_area = Rect {
            x: area.width / 4,
            y: area.height.saturating_sub(4),
            width: area.width / 2,
            height: 3,
        };

        f.render_widget(Clear, search_area);

        let text = format!("Search: {}█", self.search_filter);
        let paragraph = Paragraph::new(text)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
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
                match self.current_view {
                    // In sub-views, go back instead of quitting
                    View::VmDetails | View::Snapshots | View::Profiles
                    | View::Blueprints | View::ActivityLog | View::Help => {
                        self.current_view = View::VmList;
                        self.detail_tab = 0;
                    }
                    // In main views, quit
                    _ => {
                        if self.show_search {
                            self.show_search = false;
                        } else {
                            self.should_quit = true;
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
            // View switching
            KeyCode::Char('1') => self.current_view = View::Dashboard,
            KeyCode::Char('2') => self.current_view = View::VmList,
            KeyCode::Char('3') => self.current_view = View::Snapshots,
            KeyCode::Char('4') => self.current_view = View::Profiles,
            KeyCode::Char('5') => self.current_view = View::Blueprints,
            KeyCode::Char('6') => self.current_view = View::ActivityLog,
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
                KeyCode::Char(c) => {
                    if menu.select_by_key(c).is_some() {
                        self.mode = InteractiveMode::Normal;
                        self.handle_menu_selection(c).await?;
                    }
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
            InteractiveMode::Normal => {}
        }

        Ok(())
    }

    /// Handle search bar keys
    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.search_filter.push(c);
                self.state.search_query = self.search_filter.clone();
            }
            KeyCode::Backspace => {
                self.search_filter.pop();
                self.state.search_query = self.search_filter.clone();
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

    /// Handle view-specific keys
    async fn handle_view_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.current_view {
            View::VmList => self.handle_vm_list_key(key).await?,
            View::VmDetails => self.handle_vm_details_key(key)?,
            View::Snapshots => self.handle_snapshots_key(key).await?,
            _ => {}
        }
        Ok(())
    }

    /// Handle VM list view keys
    async fn handle_vm_list_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next();
            }
            KeyCode::Enter => {
                self.current_view = View::VmDetails;
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
            KeyCode::Char('c') => {
                // Create VM - show input dialog
                self.show_create_vm_dialog();
            }
            KeyCode::Char('s') => {
                // Start VM with confirmation
                if let Some(vm) = self.state.selected_vm() {
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
                // Stop VM with confirmation (or skip if disabled in config)
                if let Some(vm) = self.state.selected_vm() {
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
                let filter_name = self
                    .state
                    .status_filter
                    .as_deref()
                    .unwrap_or("All");
                self.notifications
                    .info(format!("Filter: {}", filter_name));
            }
            KeyCode::Char('d') => {
                // Delete VM with confirmation (or skip if disabled in config)
                if let Some(vm) = self.state.selected_vm() {
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

    /// Handle VM details view keys
    fn handle_vm_details_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Backspace | KeyCode::Esc => {
                self.current_view = View::VmList;
                self.detail_tab = 0;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.detail_tab > 0 {
                    self.detail_tab -= 1;
                } else {
                    self.detail_tab = TAB_NAMES.len() - 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                self.detail_tab = (self.detail_tab + 1) % TAB_NAMES.len();
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
        // Use the stored VM name to avoid acting on wrong VM due to selection changes
        if let Some(vm_name) = self.dialog_vm_name.take() {
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
            'd' => {
                if let Some(vm) = self.state.selected_vm() {
                    let vm_name = vm.name.clone();
                    self.delete_vm(&vm_name).await?;
                }
            }
            'v' => {
                self.current_view = View::VmDetails;
            }
            _ => {}
        }
        Ok(())
    }

    /// Validate a VM name according to RFC 1123 DNS subdomain rules
    fn is_valid_vm_name(name: &str) -> bool {
        if name.is_empty() || name.len() > 253 {
            return false;
        }
        // RFC 1123 DNS subdomain: lowercase alphanumeric and hyphens, start/end with alphanumeric
        static RE: std::sync::LazyLock<regex::Regex> =
            std::sync::LazyLock::new(|| regex::Regex::new(r"^[a-z0-9]([a-z0-9\-]*[a-z0-9])?$").unwrap());
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

    /// Refresh data from Kubernetes
    async fn refresh_data(&mut self) -> Result<()> {
        if let Err(e) = self.state.refresh_vms().await {
            self.notifications
                .error(format!("Failed to refresh VMs: {}", e));
        }

        if let Err(e) = self.state.refresh_snapshots().await {
            // Snapshots might not be critical, just log
            eprintln!("Failed to refresh snapshots: {}", e);
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
                    self.state
                        .record_activity("▶ ", vm_name, "start requested");
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
                    self.state
                        .record_activity("⏹ ", vm_name, "stop requested");
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
