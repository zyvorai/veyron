// Disk Management System - Expansion, health checks, and automation
// This feature provides comprehensive disk management for VMs

pub mod expansion;
pub mod health;
pub mod scripts;

pub use expansion::{DiskExpansion, ExpansionPlan, ExpansionStatus};
pub use health::{DiskHealth, DiskHealthCheck, DiskUsageAlert};
pub use scripts::{ExpansionScript, FilesystemType, ScriptGenerator};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Disk information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub size: String,
    pub used: String,
    pub available: String,
    pub usage_percent: f64,
    pub mount_point: String,
    pub filesystem: String,
    pub device: String,
}

impl DiskInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: "0Gi".to_string(),
            used: "0Gi".to_string(),
            available: "0Gi".to_string(),
            usage_percent: 0.0,
            mount_point: "/".to_string(),
            filesystem: "ext4".to_string(),
            device: "/dev/sda1".to_string(),
        }
    }

    /// Parse size string to bytes.
    /// Returns 0 and logs a warning if the numeric part cannot be parsed.
    pub fn parse_size(size_str: &str) -> u64 {
        let size_upper = size_str.to_uppercase();

        let parse = |value: &str, multiplier: f64| -> u64 {
            match value.parse::<f64>() {
                Ok(v) => (v * multiplier) as u64,
                Err(_) => {
                    log::warn!("Failed to parse size value '{}' from '{}'", value, size_str);
                    0
                }
            }
        };

        if let Some(value) = size_upper.strip_suffix("TI") {
            parse(value, 1024.0 * 1024.0 * 1024.0 * 1024.0)
        } else if let Some(value) = size_upper.strip_suffix("GI") {
            parse(value, 1024.0 * 1024.0 * 1024.0)
        } else if let Some(value) = size_upper.strip_suffix("MI") {
            parse(value, 1024.0 * 1024.0)
        } else if let Some(value) = size_upper.strip_suffix("T") {
            parse(value, 1000.0 * 1000.0 * 1000.0 * 1000.0)
        } else if let Some(value) = size_upper.strip_suffix("G") {
            parse(value, 1000.0 * 1000.0 * 1000.0)
        } else if let Some(value) = size_upper.strip_suffix("M") {
            parse(value, 1000.0 * 1000.0)
        } else {
            parse(size_str, 1.0)
        }
    }

    /// Format bytes to human-readable size
    pub fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.1}{}", size, UNITS[unit_index])
    }

    /// Check if disk needs expansion based on usage
    pub fn needs_expansion(&self, threshold: f64) -> bool {
        self.usage_percent >= threshold
    }
}

/// Disk configuration for VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskConfig {
    pub name: String,
    pub current_size: String,
    pub target_size: String,
    pub pvc_name: String,
    pub storage_class: Option<String>,
    pub labels: HashMap<String, String>,
}

impl DiskConfig {
    pub fn new(name: impl Into<String>, pvc_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            current_size: "20Gi".to_string(),
            target_size: "40Gi".to_string(),
            pvc_name: pvc_name.into(),
            storage_class: None,
            labels: HashMap::new(),
        }
    }

    pub fn with_sizes(mut self, current: impl Into<String>, target: impl Into<String>) -> Self {
        self.current_size = current.into();
        self.target_size = target.into();
        self
    }

    pub fn with_storage_class(mut self, class: impl Into<String>) -> Self {
        self.storage_class = Some(class.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(DiskInfo::parse_size("10Gi"), 10 * 1024 * 1024 * 1024);
        assert_eq!(DiskInfo::parse_size("5Ti"), 5 * 1024 * 1024 * 1024 * 1024);
        assert_eq!(DiskInfo::parse_size("100Mi"), 100 * 1024 * 1024);
        assert_eq!(DiskInfo::parse_size("10G"), 10 * 1000 * 1000 * 1000);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(DiskInfo::format_size(1024), "1.0KiB");
        assert_eq!(DiskInfo::format_size(1024 * 1024), "1.0MiB");
        assert_eq!(DiskInfo::format_size(10 * 1024 * 1024 * 1024), "10.0GiB");
    }

    #[test]
    fn test_needs_expansion() {
        let mut disk = DiskInfo::new("test");
        disk.usage_percent = 85.0;

        assert!(disk.needs_expansion(80.0));
        assert!(!disk.needs_expansion(90.0));
    }

    #[test]
    fn test_disk_config_creation() {
        let config = DiskConfig::new("disk1", "my-pvc")
            .with_sizes("20Gi", "40Gi")
            .with_storage_class("fast-ssd");

        assert_eq!(config.name, "disk1");
        assert_eq!(config.current_size, "20Gi");
        assert_eq!(config.target_size, "40Gi");
        assert_eq!(config.storage_class, Some("fast-ssd".to_string()));
    }
}
