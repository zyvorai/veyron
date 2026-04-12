// VM Performance Monitoring & Analytics - Real-time resource tracking
// This feature provides operational insights and capacity planning

pub mod analyzer;
pub mod metrics;
pub mod reporter;

pub use analyzer::{Bottleneck, PerformanceAnalyzer, PerformanceReport};
pub use metrics::{MetricsCollector, ResourceUsage, VMMetrics};
pub use reporter::{MonitoringReporter, ReportFormat};

use serde::{Deserialize, Serialize};

/// Monitoring time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimePeriod {
    Last5Minutes,
    Last15Minutes,
    Last1Hour,
    Last6Hours,
    Last24Hours,
    Last7Days,
    Custom { start: String, end: String },
}

impl TimePeriod {
    pub fn to_seconds(&self) -> u64 {
        match self {
            TimePeriod::Last5Minutes => 300,
            TimePeriod::Last15Minutes => 900,
            TimePeriod::Last1Hour => 3600,
            TimePeriod::Last6Hours => 21600,
            TimePeriod::Last24Hours => 86400,
            TimePeriod::Last7Days => 604800,
            TimePeriod::Custom { .. } => 3600, // Default to 1 hour
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "5m" => TimePeriod::Last5Minutes,
            "15m" => TimePeriod::Last15Minutes,
            "1h" => TimePeriod::Last1Hour,
            "6h" => TimePeriod::Last6Hours,
            "24h" | "1d" => TimePeriod::Last24Hours,
            "7d" => TimePeriod::Last7Days,
            _ => TimePeriod::Last1Hour,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub interval_seconds: u64,
    pub enable_cpu: bool,
    pub enable_memory: bool,
    pub enable_disk: bool,
    pub enable_network: bool,
    pub alert_thresholds: AlertThresholds,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 5,
            enable_cpu: true,
            enable_memory: true,
            enable_disk: true,
            enable_network: true,
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

/// Alert thresholds for resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub cpu_warning: f64,     // Percentage
    pub cpu_critical: f64,    // Percentage
    pub memory_warning: f64,  // Percentage
    pub memory_critical: f64, // Percentage
    pub disk_warning: f64,    // Percentage
    pub disk_critical: f64,   // Percentage
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_warning: 70.0,
            cpu_critical: 90.0,
            memory_warning: 75.0,
            memory_critical: 90.0,
            disk_warning: 80.0,
            disk_critical: 95.0,
        }
    }
}

/// Monitoring summary for a VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSummary {
    pub vm_name: String,
    pub namespace: String,
    pub current_metrics: Option<VMMetrics>,
    pub average_metrics: Option<VMMetrics>,
    pub peak_metrics: Option<VMMetrics>,
    pub alerts: Vec<String>,
    pub performance_score: u8,
}

impl MonitoringSummary {
    pub fn new(vm_name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            namespace: namespace.into(),
            current_metrics: None,
            average_metrics: None,
            peak_metrics: None,
            alerts: Vec::new(),
            performance_score: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_period_seconds() {
        assert_eq!(TimePeriod::Last5Minutes.to_seconds(), 300);
        assert_eq!(TimePeriod::Last1Hour.to_seconds(), 3600);
        assert_eq!(TimePeriod::Last24Hours.to_seconds(), 86400);
    }

    #[test]
    fn test_time_period_from_string() {
        match TimePeriod::from_string("5m") {
            TimePeriod::Last5Minutes => {}
            _ => panic!("Expected Last5Minutes"),
        }

        match TimePeriod::from_string("1h") {
            TimePeriod::Last1Hour => {}
            _ => panic!("Expected Last1Hour"),
        }
    }

    #[test]
    fn test_default_config() {
        let config = MonitoringConfig::default();
        assert_eq!(config.interval_seconds, 5);
        assert!(config.enable_cpu);
        assert!(config.enable_memory);
    }

    #[test]
    fn test_default_thresholds() {
        let thresholds = AlertThresholds::default();
        assert_eq!(thresholds.cpu_warning, 70.0);
        assert_eq!(thresholds.cpu_critical, 90.0);
        assert_eq!(thresholds.memory_warning, 75.0);
    }

    #[test]
    fn test_monitoring_summary_creation() {
        let summary = MonitoringSummary::new("test-vm", "default");
        assert_eq!(summary.vm_name, "test-vm");
        assert_eq!(summary.namespace, "default");
        assert!(summary.current_metrics.is_none());
        assert_eq!(summary.performance_score, 0);
    }
}
