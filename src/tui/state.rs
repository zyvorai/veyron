// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
        let secs = Utc::now()
            .signed_duration_since(self.timestamp)
            .num_seconds();
        format_elapsed(secs)
    }
}

/// VM information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub name: String,
    /// Kubernetes namespace containing the VirtualMachine
    #[serde(default)]
    pub namespace: String,
    pub status: String,
    pub cpu: String,
    pub memory: String,
    pub age: String,
    pub ready: bool,
    pub disk: String,
    pub ip: String,
    pub node: String,
    /// Set when a VeyronVM CR exists for this KubeVirt VM (operator-managed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub veyron_managed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drift_detected: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drift_message: Option<String>,
    /// True when VMI reports AgentConnected=True condition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_agent_connected: Option<bool>,
    /// Passthrough devices attached (`devices.gpus` + `devices.hostDevices`).
    /// Non-zero means the VM can never live-migrate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_count: Option<u32>,
}

impl VmInfo {
    pub fn from_vm(vm: &crate::kube::types::VirtualMachine) -> Self {
        let name = vm.metadata.name.clone().unwrap_or_default();
        let namespace = vm
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());

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

        let gpu_count = vm
            .spec
            .template
            .spec
            .domain
            .devices
            .as_ref()
            .map(|d| {
                (d.gpus.as_ref().map(|g| g.len()).unwrap_or(0)
                    + d.host_devices.as_ref().map(|h| h.len()).unwrap_or(0)) as u32
            })
            .filter(|n| *n > 0);

        Self {
            name,
            namespace,
            status,
            cpu,
            memory,
            age,
            ready,
            disk,
            ip,
            node,
            veyron_managed: None,
            drift_detected: None,
            drift_message: None,
            guest_agent_connected: None,
            gpu_count,
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

    pub fn from_vm_with_vmi_data(
        vm: &crate::kube::types::VirtualMachine,
        ip: Option<String>,
        node: Option<String>,
    ) -> Self {
        let mut info = Self::from_vm(vm);
        if let Some(ip_addr) = ip {
            info.ip = ip_addr;
        }
        if let Some(node_name) = node {
            info.node = node_name;
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

/// Node information for display
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub name: String,
    pub status: String,
    pub role: String,
    pub cpu_capacity: String,
    pub memory_capacity: String,
    pub pod_count: String,
    pub age: String,
}

/// Pod information for display
#[derive(Debug, Clone)]
pub struct PodInfo {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub node: String,
    pub age: String,
}

/// K8s event for display
#[derive(Debug, Clone)]
pub struct EventInfo {
    pub time: String,
    pub event_type: String,
    pub reason: String,
    pub object: String,
    pub message: String,
}

/// VMI information for display
#[derive(Debug, Clone)]
pub struct VmiInfo {
    pub name: String,
    pub phase: String,
    pub node: String,
    pub ip: String,
}

/// Application state
pub struct AppState {
    /// Current namespace
    pub namespace: String,

    /// List of VMs
    pub vms: Vec<VmInfo>,

    /// List of snapshots
    pub snapshots: Vec<SnapshotDisplayInfo>,

    /// Cluster nodes
    pub nodes: Vec<NodeInfo>,

    /// Pods in namespace
    pub pods: Vec<PodInfo>,

    /// K8s events
    pub events: Vec<EventInfo>,

    /// Running VMIs
    pub vmis: Vec<VmiInfo>,

    /// Selected index in current list
    pub selected_index: usize,

    /// Selected index in snapshots list
    pub snapshot_selected_index: usize,

    /// Activity log scroll offset
    pub activity_scroll_offset: usize,

    /// VM details events tab scroll offset
    pub detail_events_scroll: u16,

    /// Selection indices for extended views
    pub node_selected_index: usize,
    pub pod_selected_index: usize,
    pub event_selected_index: usize,
    pub vmi_selected_index: usize,

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

    /// Search case sensitive
    pub search_case_sensitive: bool,

    /// Search regex mode
    pub search_regex: bool,

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
            nodes: Vec::new(),
            pods: Vec::new(),
            events: Vec::new(),
            vmis: Vec::new(),
            selected_index: 0,
            snapshot_selected_index: 0,
            activity_scroll_offset: 0,
            detail_events_scroll: 0,
            node_selected_index: 0,
            pod_selected_index: 0,
            event_selected_index: 0,
            vmi_selected_index: 0,
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
            search_case_sensitive: false,
            search_regex: false,
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
            let (ip, node) = match client.get_vm_ip_and_node(&self.namespace, &vm_name).await {
                Ok(result) => result,
                Err(e) => {
                    log::debug!("Failed to get VMI data for VM '{}': {}", vm_name, e);
                    (None, None)
                }
            };
            vm_infos.push(VmInfo::from_vm_with_vmi_data(vm, ip, node));
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

    /// Scroll activity log down
    pub fn activity_scroll_down(&mut self) {
        if self.activity_scroll_offset < self.recent_activity.len().saturating_sub(1) {
            self.activity_scroll_offset += 1;
        }
    }

    /// Scroll activity log up
    pub fn activity_scroll_up(&mut self) {
        self.activity_scroll_offset = self.activity_scroll_offset.saturating_sub(1);
    }

    /// Navigate selection in extended views
    pub fn select_next_node(&mut self) {
        if !self.nodes.is_empty() {
            self.node_selected_index = (self.node_selected_index + 1) % self.nodes.len();
        }
    }

    pub fn select_previous_node(&mut self) {
        if !self.nodes.is_empty() {
            self.node_selected_index = if self.node_selected_index == 0 {
                self.nodes.len() - 1
            } else {
                self.node_selected_index - 1
            };
        }
    }

    pub fn select_next_pod(&mut self) {
        if !self.pods.is_empty() {
            self.pod_selected_index = (self.pod_selected_index + 1) % self.pods.len();
        }
    }

    pub fn select_previous_pod(&mut self) {
        if !self.pods.is_empty() {
            self.pod_selected_index = if self.pod_selected_index == 0 {
                self.pods.len() - 1
            } else {
                self.pod_selected_index - 1
            };
        }
    }

    pub fn select_next_event(&mut self) {
        if !self.events.is_empty() {
            self.event_selected_index = (self.event_selected_index + 1) % self.events.len();
        }
    }

    pub fn select_previous_event(&mut self) {
        if !self.events.is_empty() {
            self.event_selected_index = if self.event_selected_index == 0 {
                self.events.len() - 1
            } else {
                self.event_selected_index - 1
            };
        }
    }

    pub fn select_next_vmi(&mut self) {
        if !self.vmis.is_empty() {
            self.vmi_selected_index = (self.vmi_selected_index + 1) % self.vmis.len();
        }
    }

    pub fn select_previous_vmi(&mut self) {
        if !self.vmis.is_empty() {
            self.vmi_selected_index = if self.vmi_selected_index == 0 {
                self.vmis.len() - 1
            } else {
                self.vmi_selected_index - 1
            };
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

    /// Get the currently selected snapshot
    pub fn selected_snapshot(&self) -> Option<&SnapshotDisplayInfo> {
        self.snapshots.get(self.snapshot_selected_index)
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
            SortMode::AgeAsc => self.vms.sort_by_key(|a| parse_age_to_seconds(&a.age)),
            SortMode::AgeDesc => self
                .vms
                .sort_by_key(|b| std::cmp::Reverse(parse_age_to_seconds(&b.age))),
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
            Some(filter) => self.vms.iter().filter(|vm| vm.status == *filter).collect(),
        };
        if !self.search_query.is_empty() {
            if self.search_regex {
                // Regex search mode
                let pattern = if self.search_case_sensitive {
                    regex::Regex::new(&self.search_query)
                } else {
                    regex::Regex::new(&format!("(?i){}", self.search_query))
                };
                if let Ok(re) = pattern {
                    result.retain(|vm| re.is_match(&vm.name) || re.is_match(&vm.status));
                }
            } else if self.search_case_sensitive {
                result.retain(|vm| {
                    vm.name.contains(&self.search_query) || vm.status.contains(&self.search_query)
                });
            } else {
                let query = self.search_query.to_lowercase();
                result.retain(|vm| {
                    vm.name.to_lowercase().contains(&query)
                        || vm.status.to_lowercase().contains(&query)
                });
            }
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
        // Reset selection
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
                None
                    // New VM discovered
                    if !self.previous_vm_statuses.is_empty() => {
                        self.record_activity("🆕", &vm.name, "discovered");
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

    /// Refresh cluster nodes
    pub async fn refresh_nodes(&mut self) -> Result<()> {
        use crate::kube::KubeClient;

        let client = KubeClient::new().await?;
        let node_list = client.list_nodes().await?;

        self.nodes = node_list
            .iter()
            .map(|node| {
                let name = node
                    .metadata
                    .name
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                let status = node
                    .status
                    .as_ref()
                    .and_then(|s| {
                        s.conditions.as_ref().and_then(|conds| {
                            conds.iter().find(|c| c.type_ == "Ready").map(|c| {
                                if c.status == "True" {
                                    "Ready"
                                } else {
                                    "NotReady"
                                }
                            })
                        })
                    })
                    .unwrap_or("Unknown")
                    .to_string();
                let role = node
                    .metadata
                    .labels
                    .as_ref()
                    .map(|labels| {
                        if labels.contains_key("node-role.kubernetes.io/control-plane") {
                            "control-plane"
                        } else if labels.contains_key("node-role.kubernetes.io/master") {
                            "master"
                        } else {
                            "worker"
                        }
                    })
                    .unwrap_or("worker")
                    .to_string();
                let cpu_capacity = node
                    .status
                    .as_ref()
                    .and_then(|s| {
                        s.capacity
                            .as_ref()
                            .and_then(|c| c.get("cpu").map(|q| q.0.clone()))
                    })
                    .unwrap_or_else(|| "?".to_string());
                let memory_capacity = node
                    .status
                    .as_ref()
                    .and_then(|s| {
                        s.capacity
                            .as_ref()
                            .and_then(|c| c.get("memory").map(|q| q.0.clone()))
                    })
                    .unwrap_or_else(|| "?".to_string());
                let pod_count = node
                    .status
                    .as_ref()
                    .and_then(|s| {
                        s.capacity
                            .as_ref()
                            .and_then(|c| c.get("pods").map(|q| q.0.clone()))
                    })
                    .unwrap_or_else(|| "?".to_string());
                let age = node
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| {
                        let duration = Utc::now().signed_duration_since(ts.0);
                        let days = duration.num_days();
                        if days > 0 {
                            format!("{}d", days)
                        } else {
                            let hours = duration.num_hours();
                            format!("{}h", hours)
                        }
                    })
                    .unwrap_or_else(|| "?".to_string());

                NodeInfo {
                    name,
                    status,
                    role,
                    cpu_capacity,
                    memory_capacity,
                    pod_count,
                    age,
                }
            })
            .collect();

        Ok(())
    }

    /// Refresh pods in namespace
    pub async fn refresh_pods(&mut self) -> Result<()> {
        use crate::kube::KubeClient;

        let client = KubeClient::new().await?;
        let pod_list = client.list_pods(&self.namespace).await?;

        self.pods = pod_list
            .iter()
            .map(|pod| {
                let name = pod
                    .metadata
                    .name
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                let namespace = pod
                    .metadata
                    .namespace
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                let status = pod
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                let node = pod
                    .spec
                    .as_ref()
                    .and_then(|s| s.node_name.clone())
                    .unwrap_or_else(|| "N/A".to_string());
                let age = pod
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| {
                        let duration = Utc::now().signed_duration_since(ts.0);
                        let days = duration.num_days();
                        if days > 0 {
                            format!("{}d", days)
                        } else {
                            let hours = duration.num_hours();
                            if hours > 0 {
                                format!("{}h", hours)
                            } else {
                                format!("{}m", duration.num_minutes())
                            }
                        }
                    })
                    .unwrap_or_else(|| "?".to_string());

                PodInfo {
                    name,
                    namespace,
                    status,
                    node,
                    age,
                }
            })
            .collect();

        Ok(())
    }

    /// Refresh K8s events in namespace
    pub async fn refresh_events(&mut self) -> Result<()> {
        use crate::kube::KubeClient;

        let client = KubeClient::new().await?;
        let event_list = client.list_events(&self.namespace).await?;

        self.events = event_list
            .iter()
            .map(|event| {
                let time = event
                    .last_timestamp
                    .as_ref()
                    .map(|ts| {
                        let secs = Utc::now().signed_duration_since(ts.0).num_seconds();
                        format_elapsed(secs)
                    })
                    .unwrap_or_else(|| "?".to_string());
                let event_type = event.type_.clone().unwrap_or_else(|| "Normal".to_string());
                let reason = event
                    .reason
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());
                let object = event
                    .involved_object
                    .name
                    .clone()
                    .unwrap_or_else(|| "?".to_string());
                let message = event.message.clone().unwrap_or_default();

                EventInfo {
                    time,
                    event_type,
                    reason,
                    object,
                    message,
                }
            })
            .collect();

        // Sort newest first
        self.events.reverse();

        Ok(())
    }

    /// Refresh VMIs in namespace
    pub async fn refresh_vmis(&mut self) -> Result<()> {
        use crate::kube::KubeClient;

        let client = KubeClient::new().await?;
        let vmi_list = client.list_vmis(&self.namespace).await?;

        self.vmis = vmi_list
            .iter()
            .map(|vmi| {
                let name = vmi
                    .metadata
                    .name
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                let phase = vmi
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                let node = vmi
                    .status
                    .as_ref()
                    .and_then(|s| s.node_name.clone())
                    .unwrap_or_else(|| "N/A".to_string());
                let ip = vmi
                    .status
                    .as_ref()
                    .map(|s| {
                        s.interfaces
                            .iter()
                            .find_map(|iface| {
                                iface
                                    .ip_address
                                    .as_ref()
                                    .filter(|ip| !ip.is_empty())
                                    .cloned()
                            })
                            .unwrap_or_else(|| "N/A".to_string())
                    })
                    .unwrap_or_else(|| "N/A".to_string());

                VmiInfo {
                    name,
                    phase,
                    node,
                    ip,
                }
            })
            .collect();

        Ok(())
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
