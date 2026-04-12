// TUI State Management - Application state and data

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::kube::types::VirtualMachineInstanceStatus;

/// Format elapsed seconds into a human-readable string (e.g., "5s ago", "3m ago")
pub fn format_elapsed(secs: i64) -> String {
    let secs = secs.max(0);
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

/// Parse an age string like "5d3h10m2s" into total seconds for numeric comparison
fn parse_age_to_seconds(age: &str) -> u64 {
    let mut total = 0u64;
    let mut num = String::new();
    for c in age.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else {
            let n: u64 = num.parse().unwrap_or(0);
            match c {
                'd' => total += n * 86400,
                'h' => total += n * 3600,
                'm' => total += n * 60,
                's' => total += n,
                _ => {}
            }
            num.clear();
        }
    }
    total
}

/// Activity event recorded from real VM operations and status changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub icon: String,
    pub vm_name: String,
    pub action: String,
    pub timestamp: DateTime<Utc>,
}

impl ActivityEvent {
    pub fn new(icon: &str, vm_name: &str, action: &str) -> Self {
        Self {
            icon: icon.to_string(),
            vm_name: vm_name.to_string(),
            action: action.to_string(),
            timestamp: Utc::now(),
        }
    }

    /// Format the elapsed time since this event
    pub fn elapsed_display(&self) -> String {
        let secs = Utc::now().signed_duration_since(self.timestamp).num_seconds();
        format_elapsed(secs)
    }
}

/// VM information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub name: String,
    pub status: String,
    pub cpu: String,
    pub memory: String,
    pub age: String,
    pub ready: bool,
    pub disk: String,
    pub ip: String,
    pub node: String,
}

impl VmInfo {
    pub fn from_vm(vm: &crate::kube::types::VirtualMachine) -> Self {
        let name = vm.metadata.name.clone().unwrap_or_default();

        let status = vm
            .status
            .as_ref()
            .and_then(|s| s.printable_status.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let ready = vm.status.as_ref().and_then(|s| s.ready).unwrap_or(false);

        // Extract CPU and memory from spec
        let cpu = vm
            .spec
            .template
            .spec
            .domain
            .cpu
            .as_ref()
            .map(|c| format!("{} cores", c.cores.unwrap_or(1)))
            .unwrap_or_else(|| "1 core".to_string());

        let memory = vm
            .spec
            .template
            .spec
            .domain
            .resources
            .requests
            .as_ref()
            .and_then(|req| req.get("memory"))
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        // Calculate age
        let age = vm
            .metadata
            .creation_timestamp
            .as_ref()
            .map(|created| {
                let now = Utc::now();
                let duration = now.signed_duration_since(created.0);

                let days = duration.num_days();
                let hours = duration.num_hours() % 24;
                let minutes = duration.num_minutes() % 60;

                if days > 0 {
                    format!("{}d{}h", days, hours)
                } else if hours > 0 {
                    format!("{}h{}m", hours, minutes)
                } else {
                    format!("{}m", minutes)
                }
            })
            .unwrap_or_else(|| "Unknown".to_string());

        // Extract disk info from volumes
        let disk = vm
            .spec
            .template
            .spec
            .volumes
            .as_ref()
            .map(|volumes| {
                // Sum up empty disk capacities, count PVCs
                let mut parts = Vec::new();
                for vol in volumes {
                    if let Some(ref empty) = vol.empty_disk {
                        parts.push(empty.capacity.clone());
                    } else if let Some(ref pvc) = vol.persistent_volume_claim {
                        parts.push(format!("pvc:{}", pvc.claim_name));
                    } else if let Some(ref dv) = vol.data_volume {
                        parts.push(format!("dv:{}", dv.name));
                    } else if vol.container_disk.is_some() {
                        parts.push("container".to_string());
                    }
                }
                if parts.is_empty() {
                    "None".to_string()
                } else {
                    parts.join(", ")
                }
            })
            .unwrap_or_else(|| "None".to_string());

        // Extract node name from conditions or status
        let node = vm
            .status
            .as_ref()
            .and_then(|s| {
                s.conditions.as_ref().and_then(|conds| {
                    conds
                        .iter()
                        .find(|c| c.type_ == "Ready" && c.status == "True")
                        .and_then(|c| c.message.clone())
                })
            })
            .unwrap_or_else(|| "N/A".to_string());

        // IP is not available in VM spec/status - needs VMI status
        let ip = "N/A".to_string();

        Self {
            name,
            status,
            cpu,
            memory,
            age,
            ready,
            disk,
            ip,
            node,
        }
    }

    /// Create a VmInfo from a VM with an IP address resolved from the VMI
    pub fn from_vm_with_ip(vm: &crate::kube::types::VirtualMachine, ip: Option<String>) -> Self {
        let mut info = Self::from_vm(vm);
        if let Some(ip_addr) = ip {
            info.ip = ip_addr;
        }
        info
    }
}

/// Snapshot information for display
#[derive(Debug, Clone)]
pub struct SnapshotDisplayInfo {
    pub name: String,
    pub vm_name: String,
    pub status: String,
    pub age: String,
    pub ready: bool,
}

/// Application state
pub struct AppState {
    /// Current namespace
    pub namespace: String,

    /// List of VMs
    pub vms: Vec<VmInfo>,

    /// List of snapshots
    pub snapshots: Vec<SnapshotDisplayInfo>,

    /// Selected index in current list
    pub selected_index: usize,

    /// Selected index in snapshots list
    pub snapshot_selected_index: usize,

    /// Last refresh time
    pub last_refresh: DateTime<Utc>,

    /// Auto-refresh interval in seconds
    pub refresh_interval: u64,

    /// Sort mode
    pub sort_mode: SortMode,

    /// Search query
    pub search_query: String,

    /// Multi-select mode enabled
    pub multi_select_mode: bool,

    /// Selected items in multi-select mode
    pub selected_items: Vec<usize>,

    /// Show stats bar
    pub show_stats_bar: bool,

    /// CPU usage history (last 30 data points)
    pub cpu_history: Vec<u64>,

    /// Memory usage history (last 30 data points)
    pub memory_history: Vec<u64>,

    /// VM count history (last 30 data points)
    pub vm_count_history: Vec<u64>,

    /// Disk usage history (last 30 data points)
    pub disk_history: Vec<u64>,

    /// Network usage history (last 30 data points)
    pub network_history: Vec<u64>,

    /// Recent activity events (newest first, max 50)
    pub recent_activity: Vec<ActivityEvent>,

    /// Cached VMI detail for the selected VM (Network tab)
    pub selected_vmi_detail: Option<VirtualMachineInstanceStatus>,

    /// Name of VM whose VMI detail is cached
    pub selected_vmi_name: Option<String>,

    /// Status filter for VM list (None = show all)
    pub status_filter: Option<String>,

    /// Previous VM statuses for change detection
    previous_vm_statuses: HashMap<String, String>,
}

impl AppState {
    /// Create a new application state
    pub fn new(namespace: String) -> Self {
        Self {
            namespace,
            vms: Vec::new(),
            snapshots: Vec::new(),
            selected_index: 0,
            snapshot_selected_index: 0,
            last_refresh: Utc::now(),
            refresh_interval: 5, // 5 seconds
            sort_mode: SortMode::Default,
            search_query: String::new(),
            multi_select_mode: false,
            selected_items: Vec::new(),
            show_stats_bar: true,
            cpu_history: vec![0; 30],
            memory_history: vec![0; 30],
            vm_count_history: vec![0; 30],
            disk_history: vec![0; 30],
            network_history: vec![0; 30],
            recent_activity: Vec::new(),
            selected_vmi_detail: None,
            selected_vmi_name: None,
            status_filter: None,
            previous_vm_statuses: HashMap::new(),
        }
    }

    /// Set the auto-refresh interval (builder pattern)
    pub fn with_refresh_interval(mut self, interval: u64) -> Self {
        if interval > 0 {
            self.refresh_interval = interval;
        }
        self
    }

    /// Refresh VMs from Kubernetes
    pub async fn refresh_vms(&mut self) -> Result<()> {
        use crate::kube::KubeClient;

        let client = KubeClient::new().await?;
        let vm_list = client.list_vms(&self.namespace).await?;

        let mut vm_infos = Vec::with_capacity(vm_list.len());
        for vm in &vm_list {
            let vm_name = vm.metadata.name.clone().unwrap_or_default();
            let ip = match client.get_vm_ip(&self.namespace, &vm_name).await {
                Ok(ip) => ip,
                Err(e) => {
                    log::debug!("Failed to get IP for VM '{}': {}", vm_name, e);
                    None
                }
            };
            vm_infos.push(VmInfo::from_vm_with_ip(vm, ip));
        }

        // Detect status changes and record activity events
        self.detect_status_changes(&vm_infos);

        self.vms = vm_infos;
        self.last_refresh = Utc::now();

        // Reset selection if out of bounds (clamp against filtered list)
        let filtered_count = self.filtered_vms().len();
        if filtered_count == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= filtered_count {
            self.selected_index = filtered_count - 1;
        }

        Ok(())
    }

    /// Refresh snapshots from Kubernetes
    pub async fn refresh_snapshots(&mut self) -> Result<()> {
        use crate::snapshots::SnapshotManager;

        let manager = SnapshotManager::new(&self.namespace).await?;
        let snapshot_list = manager.list_all_snapshots().await?;

        self.snapshots = snapshot_list
            .into_iter()
            .map(|s| {
                let age = s.age();
                SnapshotDisplayInfo {
                    name: s.name,
                    vm_name: s.vm_name,
                    status: s.status.to_string(),
                    age,
                    ready: s.ready_to_use,
                }
            })
            .collect();

        // Clamp snapshot_selected_index
        if self.snapshots.is_empty() {
            self.snapshot_selected_index = 0;
        } else if self.snapshot_selected_index >= self.snapshots.len() {
            self.snapshot_selected_index = self.snapshots.len() - 1;
        }

        Ok(())
    }

    /// Select next snapshot in the list
    pub fn select_next_snapshot(&mut self) {
        let count = self.snapshots.len();
        if count == 0 {
            return;
        }
        if self.snapshot_selected_index < count - 1 {
            self.snapshot_selected_index += 1;
        } else {
            self.snapshot_selected_index = 0;
        }
    }

    /// Select previous snapshot in the list
    pub fn select_previous_snapshot(&mut self) {
        let count = self.snapshots.len();
        if count == 0 {
            return;
        }
        if self.snapshot_selected_index > 0 {
            self.snapshot_selected_index -= 1;
        } else {
            self.snapshot_selected_index = count - 1;
        }
    }

    /// Select next item in the list (respects active filter)
    pub fn select_next(&mut self) {
        let count = self.filtered_vms().len();
        if count == 0 {
            return;
        }

        if self.selected_index < count - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0; // Wrap around
        }
    }

    /// Select previous item in the list (respects active filter)
    pub fn select_previous(&mut self) {
        let count = self.filtered_vms().len();
        if count == 0 {
            return;
        }

        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = count - 1; // Wrap around
        }
    }

    /// Get the currently selected VM (respects active filter)
    pub fn selected_vm(&self) -> Option<&VmInfo> {
        let filtered = self.filtered_vms();
        filtered.get(self.selected_index).copied()
    }

    /// Check if data should be refreshed
    pub fn should_refresh(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_refresh)
            .num_seconds();

        elapsed >= self.refresh_interval as i64
    }

    /// Get VM statistics (single pass)
    pub fn get_stats(&self) -> VmStats {
        let mut running = 0;
        let mut stopped = 0;
        let mut starting = 0;
        let mut failed = 0;

        for vm in &self.vms {
            match vm.status.as_str() {
                "Running" => running += 1,
                "Stopped" => stopped += 1,
                "Starting" | "Pending" => starting += 1,
                "Failed" | "Error" => failed += 1,
                _ => {}
            }
        }

        VmStats {
            total: self.vms.len(),
            running,
            stopped,
            starting,
            failed,
        }
    }

    /// Toggle multi-select mode
    pub fn toggle_multi_select(&mut self) {
        self.multi_select_mode = !self.multi_select_mode;
        if !self.multi_select_mode {
            self.selected_items.clear();
        }
    }

    /// Toggle selection of current item
    pub fn toggle_current_selection(&mut self) {
        if self.multi_select_mode {
            if let Some(pos) = self
                .selected_items
                .iter()
                .position(|&i| i == self.selected_index)
            {
                self.selected_items.remove(pos);
            } else {
                self.selected_items.push(self.selected_index);
            }
        }
    }

    /// Select all items
    pub fn select_all(&mut self) {
        if self.multi_select_mode {
            self.selected_items = (0..self.vms.len()).collect();
        }
    }

    /// Deselect all items
    pub fn deselect_all(&mut self) {
        self.selected_items.clear();
    }

    /// Check if item is selected
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_items.contains(&index)
    }

    /// Cycle sort mode
    pub fn cycle_sort_mode(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.apply_sort();
    }

    /// Apply current sort mode
    pub fn apply_sort(&mut self) {
        match self.sort_mode {
            SortMode::Default => {}
            SortMode::NameAsc => self.vms.sort_by(|a, b| a.name.cmp(&b.name)),
            SortMode::NameDesc => self.vms.sort_by(|a, b| b.name.cmp(&a.name)),
            SortMode::StatusAsc => self.vms.sort_by(|a, b| a.status.cmp(&b.status)),
            SortMode::StatusDesc => self.vms.sort_by(|a, b| b.status.cmp(&a.status)),
            SortMode::AgeAsc => self.vms.sort_by(|a, b| {
                parse_age_to_seconds(&a.age).cmp(&parse_age_to_seconds(&b.age))
            }),
            SortMode::AgeDesc => self.vms.sort_by(|a, b| {
                parse_age_to_seconds(&b.age).cmp(&parse_age_to_seconds(&a.age))
            }),
        }
    }

    /// Fetch VMI detail for the currently selected VM
    pub async fn refresh_selected_vm_detail(&mut self) -> Result<()> {
        use crate::kube::KubeClient;

        let vm_name = match self.selected_vm() {
            Some(vm) => vm.name.clone(),
            None => {
                self.selected_vmi_detail = None;
                self.selected_vmi_name = None;
                return Ok(());
            }
        };

        // Skip if already cached for this VM
        if self.selected_vmi_name.as_deref() == Some(&vm_name) {
            return Ok(());
        }

        match KubeClient::new().await {
            Ok(client) => match client.get_vmi(&self.namespace, &vm_name).await {
                Ok(vmi) => {
                    self.selected_vmi_detail = vmi.status;
                    self.selected_vmi_name = Some(vm_name);
                }
                Err(_) => {
                    self.selected_vmi_detail = None;
                    self.selected_vmi_name = Some(vm_name);
                }
            },
            Err(_) => {
                self.selected_vmi_detail = None;
                self.selected_vmi_name = Some(vm_name);
            }
        }

        Ok(())
    }

    /// Get VMs filtered by status_filter and search_query
    pub fn filtered_vms(&self) -> Vec<&VmInfo> {
        let mut result: Vec<&VmInfo> = match &self.status_filter {
            None => self.vms.iter().collect(),
            Some(filter) => self
                .vms
                .iter()
                .filter(|vm| vm.status == *filter)
                .collect(),
        };
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            result.retain(|vm| vm.name.to_lowercase().contains(&query));
        }
        result
    }

    /// Cycle through status filters: All -> Running -> Stopped -> Failed -> All
    pub fn cycle_status_filter(&mut self) {
        self.status_filter = match &self.status_filter {
            None => Some("Running".to_string()),
            Some(s) if s == "Running" => Some("Stopped".to_string()),
            Some(s) if s == "Stopped" => Some("Failed".to_string()),
            _ => None,
        };
        // Reset selection, clamping to filtered list bounds
        let filtered_len = self.filtered_vms().len();
        let _ = filtered_len;
        self.selected_index = 0;
    }

    /// Toggle stats bar visibility
    pub fn toggle_stats_bar(&mut self) {
        self.show_stats_bar = !self.show_stats_bar;
    }

    /// Record a new activity event (newest at end, avoids O(n) shift)
    pub fn record_activity(&mut self, icon: &str, vm_name: &str, action: &str) {
        self.recent_activity
            .push(ActivityEvent::new(icon, vm_name, action));
        // Keep at most 50 events, drop oldest from front
        if self.recent_activity.len() > 50 {
            let excess = self.recent_activity.len().saturating_sub(50);
            if excess > 0 {
                self.recent_activity.drain(0..excess);
            }
        }
    }

    /// Detect VM status changes between refreshes and record as activity events
    fn detect_status_changes(&mut self, new_vms: &[VmInfo]) {
        let mut new_statuses = HashMap::new();

        for vm in new_vms {
            new_statuses.insert(vm.name.clone(), vm.status.clone());

            match self.previous_vm_statuses.get(&vm.name) {
                None => {
                    // New VM discovered
                    if !self.previous_vm_statuses.is_empty() {
                        self.record_activity("🆕", &vm.name, "discovered");
                    }
                }
                Some(old_status) if old_status != &vm.status => {
                    let (icon, action) = match vm.status.as_str() {
                        "Running" => ("🟢", "started"),
                        "Stopped" => ("⏸ ", "stopped"),
                        "Starting" | "Pending" => ("🟡", "starting"),
                        "Failed" | "Error" => ("🔴", "failed"),
                        _ => ("🔄", "status changed"),
                    };
                    self.record_activity(icon, &vm.name, action);
                }
                _ => {}
            }
        }

        // Detect removed VMs
        let removed: Vec<String> = self
            .previous_vm_statuses
            .keys()
            .filter(|name| !new_statuses.contains_key(*name))
            .cloned()
            .collect();
        for name in &removed {
            self.record_activity("🗑 ", name, "removed");
        }

        self.previous_vm_statuses = new_statuses;
    }

    /// Update history data derived from current VM state
    pub fn update_history(&mut self) {
        let stats = self.get_stats();
        let total = stats.total.max(1) as f64;

        // Derive usage estimates from running VM ratio
        let running_ratio = stats.running as f64 / total;
        let cpu_usage = (running_ratio * 75.0).round() as u64;
        let memory_usage = (running_ratio * 70.0).round() as u64;
        let disk_usage = (running_ratio * 55.0).round() as u64;
        let network_usage = (running_ratio * 40.0).round() as u64;

        // Use rotate_left + overwrite last element to avoid O(n) remove(0)
        fn push_history(buf: &mut [u64], value: u64) {
            if !buf.is_empty() {
                buf.rotate_left(1);
                if let Some(last) = buf.last_mut() {
                    *last = value;
                }
            }
        }

        push_history(&mut self.cpu_history, cpu_usage);
        push_history(&mut self.memory_history, memory_usage);
        push_history(&mut self.vm_count_history, self.vms.len() as u64);
        push_history(&mut self.disk_history, disk_usage);
        push_history(&mut self.network_history, network_usage);
    }
}

/// VM statistics
#[derive(Debug, Clone)]
pub struct VmStats {
    pub total: usize,
    pub running: usize,
    pub stopped: usize,
    pub starting: usize,
    pub failed: usize,
}

/// Sort mode for VM list
#[derive(Debug, Clone, PartialEq)]
pub enum SortMode {
    Default,
    NameAsc,
    NameDesc,
    StatusAsc,
    StatusDesc,
    AgeAsc,
    AgeDesc,
}

impl SortMode {
    pub fn next(&self) -> Self {
        match self {
            Self::Default => Self::NameAsc,
            Self::NameAsc => Self::NameDesc,
            Self::NameDesc => Self::StatusAsc,
            Self::StatusAsc => Self::StatusDesc,
            Self::StatusDesc => Self::AgeAsc,
            Self::AgeAsc => Self::AgeDesc,
            Self::AgeDesc => Self::Default,
        }
    }

    pub fn display(&self) -> &str {
        match self {
            Self::Default => "Default",
            Self::NameAsc => "Name ↑",
            Self::NameDesc => "Name ↓",
            Self::StatusAsc => "Status ↑",
            Self::StatusDesc => "Status ↓",
            Self::AgeAsc => "Age ↑",
            Self::AgeDesc => "Age ↓",
        }
    }
}
