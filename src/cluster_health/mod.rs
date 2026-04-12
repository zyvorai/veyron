// Cluster Health Monitoring and Dashboard

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClusterMetrics {
    pub total_nodes: usize,
    pub ready_nodes: usize,
    pub total_vms: usize,
    pub running_vms: usize,
    pub total_pods: usize,
    pub running_pods: usize,
    pub cpu_capacity: f64,
    pub cpu_usage: f64,
    pub memory_capacity_gb: f64,
    pub memory_usage_gb: f64,
    pub storage_capacity_gb: f64,
    pub storage_usage_gb: f64,
}

impl ClusterMetrics {
    pub fn cpu_usage_percent(&self) -> f64 {
        if self.cpu_capacity > 0.0 { (self.cpu_usage / self.cpu_capacity) * 100.0 } else { 0.0 }
    }

    pub fn memory_usage_percent(&self) -> f64 {
        if self.memory_capacity_gb > 0.0 { (self.memory_usage_gb / self.memory_capacity_gb) * 100.0 } else { 0.0 }
    }

    pub fn storage_usage_percent(&self) -> f64 {
        if self.storage_capacity_gb > 0.0 { (self.storage_usage_gb / self.storage_capacity_gb) * 100.0 } else { 0.0 }
    }
}

#[derive(Debug, Clone)]
pub struct HealthIssue {
    pub severity: HealthStatus,
    pub category: String,
    pub message: String,
    pub resource: Option<String>,
    pub namespace: Option<String>,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ClusterHealth {
    pub context_name: String,
    pub cluster_name: String,
    pub overall_status: HealthStatus,
    pub metrics: ClusterMetrics,
    pub issues: Vec<HealthIssue>,
    pub last_check: DateTime<Utc>,
    pub api_server_reachable: bool,
    pub kubevirt_installed: bool,
}

impl ClusterHealth {
    pub fn new(context_name: String, cluster_name: String) -> Self {
        Self {
            context_name,
            cluster_name,
            overall_status: HealthStatus::Unknown,
            metrics: ClusterMetrics::default(),
            issues: Vec::new(),
            last_check: Utc::now(),
            api_server_reachable: false,
            kubevirt_installed: false,
        }
    }

    pub fn update_overall_status(&mut self) {
        if !self.api_server_reachable {
            self.overall_status = HealthStatus::Critical;
            return;
        }

        if self.issues.iter().any(|i| i.severity == HealthStatus::Critical) {
            self.overall_status = HealthStatus::Critical;
            return;
        }

        let cpu_pct = self.metrics.cpu_usage_percent();
        let mem_pct = self.metrics.memory_usage_percent();

        if cpu_pct > 90.0 || mem_pct > 90.0 {
            self.overall_status = HealthStatus::Critical;
            return;
        }

        if self.issues.iter().any(|i| i.severity == HealthStatus::Warning) || cpu_pct > 75.0 || mem_pct > 75.0 {
            self.overall_status = HealthStatus::Warning;
            return;
        }

        if self.metrics.ready_nodes < self.metrics.total_nodes {
            self.overall_status = HealthStatus::Warning;
            return;
        }

        self.overall_status = HealthStatus::Healthy;
    }

    pub fn add_issue(&mut self, issue: HealthIssue) {
        self.issues.push(issue);
        self.update_overall_status();
    }

    pub fn critical_issues(&self) -> Vec<&HealthIssue> {
        self.issues.iter().filter(|i| i.severity == HealthStatus::Critical).collect()
    }

    pub fn warning_issues(&self) -> Vec<&HealthIssue> {
        self.issues.iter().filter(|i| i.severity == HealthStatus::Warning).collect()
    }
}
