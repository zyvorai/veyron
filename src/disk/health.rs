// Disk Health Checks - Monitor disk space and alert on issues

use super::DiskInfo;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Disk health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiskHealthStatus {
    Healthy,
    Warning,
    Critical,
    Full,
}

impl std::fmt::Display for DiskHealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiskHealthStatus::Healthy => write!(f, "Healthy"),
            DiskHealthStatus::Warning => write!(f, "Warning"),
            DiskHealthStatus::Critical => write!(f, "Critical"),
            DiskHealthStatus::Full => write!(f, "Full"),
        }
    }
}

/// Disk usage alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsageAlert {
    pub severity: DiskHealthStatus,
    pub disk_name: String,
    pub mount_point: String,
    pub usage_percent: f64,
    pub available: String,
    pub message: String,
    pub recommendation: String,
    pub timestamp: DateTime<Utc>,
}

impl DiskUsageAlert {
    pub fn new(disk: &DiskInfo, status: DiskHealthStatus) -> Self {
        let (message, recommendation) = Self::generate_messages(disk, &status);

        Self {
            severity: status,
            disk_name: disk.name.clone(),
            mount_point: disk.mount_point.clone(),
            usage_percent: disk.usage_percent,
            available: disk.available.clone(),
            message,
            recommendation,
            timestamp: Utc::now(),
        }
    }

    fn generate_messages(disk: &DiskInfo, status: &DiskHealthStatus) -> (String, String) {
        match status {
            DiskHealthStatus::Full => (
                format!(
                    "Disk {} is full ({}% used)",
                    disk.mount_point, disk.usage_percent
                ),
                "Immediate action required: Expand disk or free up space".to_string(),
            ),
            DiskHealthStatus::Critical => (
                format!(
                    "Disk {} is critically full ({:.1}% used, {} available)",
                    disk.mount_point, disk.usage_percent, disk.available
                ),
                format!(
                    "Expand disk immediately or free up space. Only {} remaining",
                    disk.available
                ),
            ),
            DiskHealthStatus::Warning => (
                format!(
                    "Disk {} usage is high ({:.1}% used, {} available)",
                    disk.mount_point, disk.usage_percent, disk.available
                ),
                "Consider expanding disk or cleaning up unused data".to_string(),
            ),
            DiskHealthStatus::Healthy => (
                format!(
                    "Disk {} has sufficient space ({:.1}% used, {} available)",
                    disk.mount_point, disk.usage_percent, disk.available
                ),
                "No action needed".to_string(),
            ),
        }
    }
}

/// Disk health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskHealthConfig {
    pub warning_threshold: f64,  // e.g., 75.0
    pub critical_threshold: f64, // e.g., 90.0
    pub full_threshold: f64,     // e.g., 98.0
    pub min_free_space_gb: f64,  // e.g., 5.0
}

impl Default for DiskHealthConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 75.0,
            critical_threshold: 90.0,
            full_threshold: 98.0,
            min_free_space_gb: 5.0,
        }
    }
}

/// Disk health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskHealth {
    pub vm_name: String,
    pub disks: Vec<DiskHealthItem>,
    pub overall_status: DiskHealthStatus,
    pub alerts: Vec<DiskUsageAlert>,
    pub needs_expansion: bool,
    pub checked_at: DateTime<Utc>,
}

impl DiskHealth {
    pub fn new(vm_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            disks: Vec::new(),
            overall_status: DiskHealthStatus::Healthy,
            alerts: Vec::new(),
            needs_expansion: false,
            checked_at: Utc::now(),
        }
    }

    pub fn add_disk(&mut self, disk: DiskInfo, status: DiskHealthStatus) {
        if status != DiskHealthStatus::Healthy {
            let alert = DiskUsageAlert::new(&disk, status.clone());
            self.alerts.push(alert);
        }

        if status == DiskHealthStatus::Critical || status == DiskHealthStatus::Full {
            self.needs_expansion = true;
        }

        self.disks.push(DiskHealthItem { info: disk, status });

        self.update_overall_status();
    }

    fn update_overall_status(&mut self) {
        if self
            .disks
            .iter()
            .any(|d| d.status == DiskHealthStatus::Full)
        {
            self.overall_status = DiskHealthStatus::Full;
        } else if self
            .disks
            .iter()
            .any(|d| d.status == DiskHealthStatus::Critical)
        {
            self.overall_status = DiskHealthStatus::Critical;
        } else if self
            .disks
            .iter()
            .any(|d| d.status == DiskHealthStatus::Warning)
        {
            self.overall_status = DiskHealthStatus::Warning;
        } else {
            self.overall_status = DiskHealthStatus::Healthy;
        }
    }
}

/// Individual disk health item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskHealthItem {
    pub info: DiskInfo,
    pub status: DiskHealthStatus,
}

/// Disk health checker
pub struct DiskHealthCheck {
    config: DiskHealthConfig,
}

impl DiskHealthCheck {
    pub fn new(config: DiskHealthConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self {
            config: DiskHealthConfig::default(),
        }
    }

    /// Check disk health based on usage
    pub fn check_disk(&self, disk: &DiskInfo) -> DiskHealthStatus {
        if disk.usage_percent >= self.config.full_threshold {
            DiskHealthStatus::Full
        } else if disk.usage_percent >= self.config.critical_threshold {
            DiskHealthStatus::Critical
        } else if disk.usage_percent >= self.config.warning_threshold {
            DiskHealthStatus::Warning
        } else {
            DiskHealthStatus::Healthy
        }
    }

    /// Check all disks for a VM
    pub fn check_vm(&self, vm_name: &str, disks: Vec<DiskInfo>) -> DiskHealth {
        let mut health = DiskHealth::new(vm_name);

        for disk in disks {
            let status = self.check_disk(&disk);
            health.add_disk(disk, status);
        }

        health
    }

    /// Calculate recommended expansion size
    pub fn recommend_expansion_size(&self, disk: &DiskInfo, target_usage: f64) -> String {
        // Parse current size
        let current_bytes = DiskInfo::parse_size(&disk.size);
        let used_bytes = (current_bytes as f64 * (disk.usage_percent / 100.0)) as u64;

        // Calculate target size to achieve target_usage percent
        if target_usage <= 0.0 {
            return DiskInfo::format_size(current_bytes);
        }
        let target_bytes = (used_bytes as f64 / (target_usage / 100.0)) as u64;

        // Add 20% buffer
        let recommended_bytes = (target_bytes as f64 * 1.2) as u64;

        // Round up to nearest 10Gi
        let gi = recommended_bytes / (1024 * 1024 * 1024);
        let rounded_gi = gi.div_ceil(10) * 10;

        format!("{}Gi", rounded_gi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_health_status() {
        let checker = DiskHealthCheck::with_defaults();

        let mut disk = DiskInfo::new("test");
        disk.usage_percent = 50.0;
        assert_eq!(checker.check_disk(&disk), DiskHealthStatus::Healthy);

        disk.usage_percent = 80.0;
        assert_eq!(checker.check_disk(&disk), DiskHealthStatus::Warning);

        disk.usage_percent = 95.0;
        assert_eq!(checker.check_disk(&disk), DiskHealthStatus::Critical);

        disk.usage_percent = 99.0;
        assert_eq!(checker.check_disk(&disk), DiskHealthStatus::Full);
    }

    #[test]
    fn test_disk_health_vm_check() {
        let checker = DiskHealthCheck::with_defaults();

        let mut disk1 = DiskInfo::new("disk1");
        disk1.usage_percent = 50.0;

        let mut disk2 = DiskInfo::new("disk2");
        disk2.usage_percent = 92.0;

        let health = checker.check_vm("test-vm", vec![disk1, disk2]);

        assert_eq!(health.overall_status, DiskHealthStatus::Critical);
        assert!(health.needs_expansion);
        assert_eq!(health.alerts.len(), 1);
    }

    #[test]
    fn test_recommend_expansion_size() {
        let checker = DiskHealthCheck::with_defaults();

        let mut disk = DiskInfo::new("test");
        disk.size = "100Gi".to_string();
        disk.usage_percent = 90.0; // 90Gi used

        // Recommend size to bring usage down to 60%
        let recommended = checker.recommend_expansion_size(&disk, 60.0);

        // 90Gi is 60% of ~150Gi, plus 20% buffer = ~180Gi
        assert!(recommended.contains("Gi"));
        assert!(DiskInfo::parse_size(&recommended) >= 150 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_disk_usage_alert_creation() {
        let mut disk = DiskInfo::new("test");
        disk.mount_point = "/".to_string();
        disk.usage_percent = 92.0;
        disk.available = "8Gi".to_string();

        let alert = DiskUsageAlert::new(&disk, DiskHealthStatus::Critical);

        assert_eq!(alert.severity, DiskHealthStatus::Critical);
        assert_eq!(alert.usage_percent, 92.0);
        assert!(alert.message.contains("critically full"));
        assert!(alert.recommendation.contains("Expand"));
    }

    #[test]
    fn test_custom_health_config() {
        let config = DiskHealthConfig {
            warning_threshold: 60.0,
            critical_threshold: 80.0,
            full_threshold: 95.0,
            min_free_space_gb: 10.0,
        };

        let checker = DiskHealthCheck::new(config);

        let mut disk = DiskInfo::new("test");
        disk.usage_percent = 70.0;

        assert_eq!(checker.check_disk(&disk), DiskHealthStatus::Warning);
    }
}
