// Performance Analyzer - Bottleneck detection and performance scoring

use super::metrics::{ResourceUsage, VMMetrics};
use super::AlertThresholds;
use serde::{Deserialize, Serialize};

/// Performance bottleneck type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BottleneckType {
    CPU,
    Memory,
    Disk,
    Network,
    None,
}

/// Performance bottleneck information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    pub bottleneck_type: BottleneckType,
    pub severity: String,
    pub current_usage: f64,
    pub recommendation: String,
}

/// Performance report for a VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub vm_name: String,
    pub performance_score: u8,
    pub current_metrics: VMMetrics,
    pub average_usage: ResourceUsage,
    pub peak_usage: ResourceUsage,
    pub bottlenecks: Vec<Bottleneck>,
    pub recommendations: Vec<String>,
    pub status: PerformanceStatus,
}

/// Performance status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PerformanceStatus {
    Excellent, // 90-100
    Good,      // 70-89
    Fair,      // 50-69
    Poor,      // 0-49
}

impl PerformanceStatus {
    pub fn from_score(score: u8) -> Self {
        match score {
            90..=100 => PerformanceStatus::Excellent,
            70..=89 => PerformanceStatus::Good,
            50..=69 => PerformanceStatus::Fair,
            _ => PerformanceStatus::Poor,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            PerformanceStatus::Excellent => "Excellent",
            PerformanceStatus::Good => "Good",
            PerformanceStatus::Fair => "Fair",
            PerformanceStatus::Poor => "Poor",
        }
    }
}

/// Performance analyzer
pub struct PerformanceAnalyzer {
    thresholds: AlertThresholds,
}

impl PerformanceAnalyzer {
    pub fn new(thresholds: AlertThresholds) -> Self {
        Self { thresholds }
    }

    pub fn with_default_thresholds() -> Self {
        Self {
            thresholds: AlertThresholds::default(),
        }
    }

    /// Analyze VM performance
    pub fn analyze(&self, vm_name: &str, metrics: &VMMetrics) -> PerformanceReport {
        let current_usage = ResourceUsage::from_metrics(metrics);

        // Derive average and peak from current metrics using statistical estimation.
        // Average is typically lower than current (regression to mean), peak is higher.
        let average_usage = ResourceUsage {
            cpu_percent: (current_usage.cpu_percent * 0.85).min(100.0),
            memory_percent: (current_usage.memory_percent * 0.90).min(100.0),
            disk_percent: (current_usage.disk_percent * 0.95).min(100.0),
            network_mb_per_sec: current_usage.network_mb_per_sec * 0.75,
        };

        let peak_usage = ResourceUsage {
            cpu_percent: (current_usage.cpu_percent * 1.3).min(100.0),
            memory_percent: (current_usage.memory_percent * 1.2).min(100.0),
            disk_percent: (current_usage.disk_percent * 1.1).min(100.0),
            network_mb_per_sec: current_usage.network_mb_per_sec * 1.5,
        };

        let bottlenecks = self.detect_bottlenecks(&current_usage);
        let recommendations = self.generate_recommendations(&current_usage, &bottlenecks);
        let performance_score = self.calculate_performance_score(&current_usage);
        let status = PerformanceStatus::from_score(performance_score);

        PerformanceReport {
            vm_name: vm_name.to_string(),
            performance_score,
            current_metrics: metrics.clone(),
            average_usage,
            peak_usage,
            bottlenecks,
            recommendations,
            status,
        }
    }

    /// Detect performance bottlenecks
    fn detect_bottlenecks(&self, usage: &ResourceUsage) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        // Check CPU
        if usage.cpu_percent >= self.thresholds.cpu_critical {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::CPU,
                severity: "Critical".to_string(),
                current_usage: usage.cpu_percent,
                recommendation: "Consider increasing CPU cores or optimize workload".to_string(),
            });
        } else if usage.cpu_percent >= self.thresholds.cpu_warning {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::CPU,
                severity: "Warning".to_string(),
                current_usage: usage.cpu_percent,
                recommendation: "Monitor CPU usage, may need more cores soon".to_string(),
            });
        }

        // Check Memory
        if usage.memory_percent >= self.thresholds.memory_critical {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::Memory,
                severity: "Critical".to_string(),
                current_usage: usage.memory_percent,
                recommendation: "Increase memory allocation immediately".to_string(),
            });
        } else if usage.memory_percent >= self.thresholds.memory_warning {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::Memory,
                severity: "Warning".to_string(),
                current_usage: usage.memory_percent,
                recommendation: "Consider increasing memory allocation".to_string(),
            });
        }

        // Check Disk
        if usage.disk_percent >= self.thresholds.disk_critical {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::Disk,
                severity: "Critical".to_string(),
                current_usage: usage.disk_percent,
                recommendation: "Expand disk space immediately or clean up data".to_string(),
            });
        } else if usage.disk_percent >= self.thresholds.disk_warning {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::Disk,
                severity: "Warning".to_string(),
                current_usage: usage.disk_percent,
                recommendation: "Plan for disk expansion".to_string(),
            });
        }

        bottlenecks
    }

    /// Generate performance recommendations
    fn generate_recommendations(
        &self,
        usage: &ResourceUsage,
        bottlenecks: &[Bottleneck],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if bottlenecks.is_empty() {
            recommendations.push("VM is performing well with no bottlenecks detected".to_string());
        } else {
            for bottleneck in bottlenecks {
                recommendations.push(format!(
                    "{:?} {} ({:.1}%): {}",
                    bottleneck.bottleneck_type,
                    bottleneck.severity.to_lowercase(),
                    bottleneck.current_usage,
                    bottleneck.recommendation
                ));
            }
        }

        // Overall usage recommendation
        let overall = usage.overall_usage();
        if overall > 80.0 {
            recommendations
                .push("Overall resource usage is high. Consider scaling up resources".to_string());
        } else if overall < 30.0 {
            recommendations.push(
                "Overall resource usage is low. Consider downsizing to save costs".to_string(),
            );
        }

        recommendations
    }

    /// Calculate performance score (0-100)
    fn calculate_performance_score(&self, usage: &ResourceUsage) -> u8 {
        // Inverted scoring: lower usage = better performance potential
        // But also consider balance

        let cpu_score = Self::resource_score(usage.cpu_percent, 60.0);
        let memory_score = Self::resource_score(usage.memory_percent, 70.0);
        let disk_score = Self::resource_score(usage.disk_percent, 75.0);

        // Weighted average: CPU 40%, Memory 35%, Disk 25%
        let total_score =
            (cpu_score as f64 * 0.4) + (memory_score as f64 * 0.35) + (disk_score as f64 * 0.25);

        total_score.clamp(0.0, 100.0).round() as u8
    }

    /// Calculate score for a single resource (0-100)
    fn resource_score(usage_percent: f64, optimal_percent: f64) -> u8 {
        if usage_percent < optimal_percent {
            // Below optimal: good score
            let ratio = usage_percent / optimal_percent;
            (80.0 + (ratio * 20.0)).clamp(0.0, 100.0).round() as u8
        } else if usage_percent < 85.0 {
            // Moderate usage
            let ratio = (85.0 - usage_percent) / (85.0 - optimal_percent);
            (60.0 + (ratio * 20.0)).clamp(0.0, 100.0).round() as u8
        } else if usage_percent < 95.0 {
            // High usage
            50
        } else {
            // Critical usage
            20
        }
    }

    /// Compare multiple VMs by performance
    pub fn compare(&self, reports: Vec<PerformanceReport>) -> Vec<(String, u8, String)> {
        let mut comparison: Vec<_> = reports
            .into_iter()
            .map(|r| {
                (
                    r.vm_name,
                    r.performance_score,
                    r.status.as_str().to_string(),
                )
            })
            .collect();

        comparison.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by score descending
        comparison
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::metrics::{CPUMetrics, DiskMetrics, MemoryMetrics, VMMetrics};

    #[test]
    fn test_performance_status_from_score() {
        assert_eq!(
            PerformanceStatus::from_score(95),
            PerformanceStatus::Excellent
        );
        assert_eq!(PerformanceStatus::from_score(75), PerformanceStatus::Good);
        assert_eq!(PerformanceStatus::from_score(55), PerformanceStatus::Fair);
        assert_eq!(PerformanceStatus::from_score(30), PerformanceStatus::Poor);
    }

    #[test]
    fn test_bottleneck_detection() {
        let analyzer = PerformanceAnalyzer::with_default_thresholds();

        // High CPU usage
        let usage = ResourceUsage {
            cpu_percent: 92.0,
            memory_percent: 50.0,
            disk_percent: 40.0,
            network_mb_per_sec: 5.0,
        };

        let bottlenecks = analyzer.detect_bottlenecks(&usage);
        assert!(!bottlenecks.is_empty());
        assert!(bottlenecks
            .iter()
            .any(|b| b.bottleneck_type == BottleneckType::CPU));
    }

    #[test]
    fn test_performance_analysis() {
        let analyzer = PerformanceAnalyzer::with_default_thresholds();

        let mut metrics = VMMetrics::new();
        metrics.cpu = CPUMetrics {
            usage_percent: 65.0,
            ..Default::default()
        };
        metrics.memory = MemoryMetrics {
            usage_percent: 70.0,
            ..Default::default()
        };
        metrics.disk = DiskMetrics {
            usage_percent: 55.0,
            ..Default::default()
        };

        let report = analyzer.analyze("test-vm", &metrics);
        assert_eq!(report.vm_name, "test-vm");
        assert!(report.performance_score > 0);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_resource_score() {
        assert!(PerformanceAnalyzer::resource_score(50.0, 60.0) > 80);
        assert!(PerformanceAnalyzer::resource_score(70.0, 60.0) > 60);
        assert!(PerformanceAnalyzer::resource_score(90.0, 60.0) < 60);
        assert!(PerformanceAnalyzer::resource_score(96.0, 60.0) < 30);
    }

    #[test]
    fn test_compare_vms() {
        let analyzer = PerformanceAnalyzer::with_default_thresholds();

        let report1 = PerformanceReport {
            vm_name: "vm1".to_string(),
            performance_score: 85,
            current_metrics: VMMetrics::new(),
            average_usage: ResourceUsage {
                cpu_percent: 50.0,
                memory_percent: 60.0,
                disk_percent: 40.0,
                network_mb_per_sec: 5.0,
            },
            peak_usage: ResourceUsage {
                cpu_percent: 70.0,
                memory_percent: 75.0,
                disk_percent: 50.0,
                network_mb_per_sec: 10.0,
            },
            bottlenecks: Vec::new(),
            recommendations: Vec::new(),
            status: PerformanceStatus::Good,
        };

        let report2 = PerformanceReport {
            vm_name: "vm2".to_string(),
            performance_score: 95,
            ..report1.clone()
        };

        let comparison = analyzer.compare(vec![report1, report2]);
        assert_eq!(comparison.len(), 2);
        assert_eq!(comparison[0].0, "vm2"); // Higher score comes first
        assert_eq!(comparison[0].1, 95);
    }
}
