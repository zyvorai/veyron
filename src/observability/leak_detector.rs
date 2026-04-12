// Memory Leak Detector - Heap growth pattern analysis

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakDetector {
    pub vm_snapshots: std::collections::HashMap<String, Vec<MemorySnapshot>>,
    pub detected_leaks: Vec<LeakReport>,
    pub config: LeakDetectorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub timestamp: DateTime<Utc>,
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub swap_used_bytes: u64,
    pub page_faults: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakReport {
    pub vm_name: String,
    pub detected_at: DateTime<Utc>,
    pub growth_rate_bytes_per_hour: f64,
    pub confidence: f64,
    pub severity: LeakSeverity,
    pub estimated_oom_hours: Option<f64>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LeakSeverity { Critical, High, Medium, Low }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakDetectorConfig {
    pub min_snapshots: usize,
    pub growth_threshold_percent: f64,
    pub check_interval_secs: u64,
}

impl Default for LeakDetectorConfig {
    fn default() -> Self { Self { min_snapshots: 10, growth_threshold_percent: 5.0, check_interval_secs: 300 } }
}

impl LeakDetector {
    pub fn new() -> Self {
        Self { vm_snapshots: std::collections::HashMap::new(), detected_leaks: Vec::new(), config: LeakDetectorConfig::default() }
    }

    pub fn record_snapshot(&mut self, vm_name: &str, snapshot: MemorySnapshot) {
        let snapshots = self.vm_snapshots.entry(vm_name.to_string()).or_default();
        snapshots.push(snapshot);
        if snapshots.len() > 1000 {
            log::debug!("Memory snapshot history for '{}' exceeded 1,000 entries, trimming", vm_name);
            let drain_count = snapshots.len().min(100);
            snapshots.drain(0..drain_count);
        }
    }

    pub fn check_vm(&mut self, vm_name: &str) -> Option<LeakReport> {
        let snapshots = self.vm_snapshots.get(vm_name)?;
        if snapshots.len() < self.config.min_snapshots { return None; }

        let values: Vec<f64> = snapshots.iter().map(|s| s.used_memory_bytes as f64).collect();
        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate().map(|(i, v)| i as f64 * v).sum();
        let sum_x2: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        let growth_rate_per_hour = slope * 3600.0 / self.config.check_interval_secs as f64;

        let first = values.first().unwrap_or(&1.0);
        let last = values.last().unwrap_or(&1.0);
        let growth_percent = ((last - first) / first) * 100.0;

        if growth_percent < self.config.growth_threshold_percent { return None; }

        let total_memory = snapshots.last()?.total_memory_bytes as f64;
        let remaining = total_memory - last;
        let estimated_oom = if growth_rate_per_hour > 0.0 { Some(remaining / growth_rate_per_hour) } else { None };

        let severity = match estimated_oom {
            Some(h) if h < 1.0 => LeakSeverity::Critical,
            Some(h) if h < 6.0 => LeakSeverity::High,
            Some(h) if h < 24.0 => LeakSeverity::Medium,
            _ => LeakSeverity::Low,
        };

        let report = LeakReport {
            vm_name: vm_name.to_string(), detected_at: Utc::now(),
            growth_rate_bytes_per_hour: growth_rate_per_hour, confidence: 0.8,
            severity, estimated_oom_hours: estimated_oom,
            recommendation: format!("Memory growing at {:.0} bytes/hour. Consider restarting or investigating the application.", growth_rate_per_hour),
        };
        self.detected_leaks.push(report.clone());
        if self.detected_leaks.len() > 1000 {
            log::warn!("Leak report history exceeded 1,000 entries, trimming oldest 100");
            let drain_count = self.detected_leaks.len().min(100);
            self.detected_leaks.drain(0..drain_count);
        }
        Some(report)
    }

    pub fn active_leaks(&self) -> Vec<&LeakReport> { self.detected_leaks.iter().collect() }
}

impl Default for LeakDetector {
    fn default() -> Self { Self::new() }
}
