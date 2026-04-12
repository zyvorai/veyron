// Monitoring Reporter - Report generation and formatting

use super::analyzer::PerformanceReport;
use super::metrics::VMMetrics;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Report format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportFormat {
    Table,
    Json,
    Yaml,
    Summary,
}

/// Monitoring reporter
pub struct MonitoringReporter;

impl MonitoringReporter {
    pub fn new() -> Self {
        Self
    }

    /// Format performance report
    pub fn format_performance_report(
        &self,
        report: &PerformanceReport,
        format: &ReportFormat,
    ) -> Result<String> {
        match format {
            ReportFormat::Table => Ok(self.format_table(report)),
            ReportFormat::Json => {
                let json = serde_json::to_string_pretty(report)?;
                Ok(json)
            }
            ReportFormat::Yaml => {
                let yaml = serde_yml::to_string(report)?;
                Ok(yaml)
            }
            ReportFormat::Summary => Ok(self.format_summary(report)),
        }
    }

    /// Format as table
    fn format_table(&self, report: &PerformanceReport) -> String {
        let mut output = String::new();

        output.push_str("╔═══════════════════════════════════════════════╗\n");
        output.push_str(&format!("║  Performance Report: {:<24}║\n", report.vm_name));
        output.push_str("╚═══════════════════════════════════════════════╝\n\n");

        output.push_str(&format!(
            "Performance Score: {}/100 [{}]\n\n",
            report.performance_score,
            report.status.as_str()
        ));

        // Current metrics
        output.push_str("Current Resource Usage:\n");
        let metrics = &report.current_metrics;

        output.push_str(&format!("  CPU:     {:>6.1}%  ", metrics.cpu.usage_percent));
        output.push_str(&self.create_bar(metrics.cpu.usage_percent, 30));
        output.push_str(&format!(
            "  ({:.1}/{} cores)\n",
            metrics.cpu.cores_used, metrics.cpu.cores_allocated
        ));

        output.push_str(&format!(
            "  Memory:  {:>6.1}%  ",
            metrics.memory.usage_percent
        ));
        output.push_str(&self.create_bar(metrics.memory.usage_percent, 30));
        output.push_str(&format!(
            "  ({:.1}/{:.1} GiB)\n",
            metrics.memory.used_gb(),
            metrics.memory.total_gb()
        ));

        output.push_str(&format!(
            "  Disk:    {:>6.1}%  ",
            metrics.disk.usage_percent
        ));
        output.push_str(&self.create_bar(metrics.disk.usage_percent, 30));
        output.push('\n');

        output.push_str(&format!(
            "\n  Disk I/O:     ↑ {:.1} MB/s  ↓ {:.1} MB/s  ({} IOPS)\n",
            metrics.disk.read_mb_per_sec(),
            metrics.disk.write_mb_per_sec(),
            metrics.disk.total_iops()
        ));

        output.push_str(&format!(
            "  Network I/O:  ↑ {:.1} MB/s  ↓ {:.1} MB/s\n\n",
            metrics.network.rx_mb_per_sec(),
            metrics.network.tx_mb_per_sec()
        ));

        // Bottlenecks
        if !report.bottlenecks.is_empty() {
            output.push_str("Bottlenecks Detected:\n");
            for bottleneck in &report.bottlenecks {
                output.push_str(&format!(
                    "  ⚠ {:?}: {} ({:.1}%)\n",
                    bottleneck.bottleneck_type, bottleneck.severity, bottleneck.current_usage
                ));
            }
            output.push('\n');
        }

        // Recommendations
        if !report.recommendations.is_empty() {
            output.push_str("Recommendations:\n");
            for (i, rec) in report.recommendations.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", i + 1, rec));
            }
        }

        output
    }

    /// Format as summary
    fn format_summary(&self, report: &PerformanceReport) -> String {
        format!(
            "{}: Score {}/100 [{}] - CPU: {:.1}%, Mem: {:.1}%, Disk: {:.1}%",
            report.vm_name,
            report.performance_score,
            report.status.as_str(),
            report.current_metrics.cpu.usage_percent,
            report.current_metrics.memory.usage_percent,
            report.current_metrics.disk.usage_percent
        )
    }

    /// Create ASCII progress bar
    fn create_bar(&self, percentage: f64, width: usize) -> String {
        let clamped = percentage.clamp(0.0, 100.0);
        let filled = ((clamped / 100.0) * width as f64) as usize;
        let empty = width - filled;

        let bar_char = "▓";

        let mut bar = String::new();
        bar.push('[');
        for _ in 0..filled {
            bar.push_str(bar_char);
        }
        for _ in 0..empty {
            bar.push('░');
        }
        bar.push(']');
        bar
    }

    /// Format comparison table
    pub fn format_comparison(&self, vms: Vec<(String, u8, String)>) -> String {
        let mut output = String::new();

        output.push_str("╔════════════════════════════════════════════╗\n");
        output.push_str("║     VM Performance Comparison              ║\n");
        output.push_str("╚════════════════════════════════════════════╝\n\n");

        output.push_str(&format!(
            "{:<30} {:>10} {:>15}\n",
            "VM NAME", "SCORE", "STATUS"
        ));
        output.push_str(&"-".repeat(58));
        output.push('\n');

        for (name, score, status) in vms {
            output.push_str(&format!(
                "{:<30} {:>10} {:>15}\n",
                name,
                format!("{}/100", score),
                status
            ));
        }

        output
    }

    /// Format metrics for live display
    pub fn format_live_metrics(&self, vm_name: &str, metrics: &VMMetrics) -> String {
        let mut output = String::new();

        output.push_str(&format!("═══ Live Metrics: {} ═══\n\n", vm_name));

        output.push_str(&format!(
            "CPU Usage:  [{:6.1}%] ",
            metrics.cpu.usage_percent
        ));
        output.push_str(&self.create_bar(metrics.cpu.usage_percent, 40));
        output.push_str(&format!(
            "  {:.1}/{} cores\n",
            metrics.cpu.cores_used, metrics.cpu.cores_allocated
        ));

        output.push_str(&format!(
            "Memory:     [{:6.1}%] ",
            metrics.memory.usage_percent
        ));
        output.push_str(&self.create_bar(metrics.memory.usage_percent, 40));
        output.push_str(&format!(
            "  {:.1}/{:.1} GiB\n",
            metrics.memory.used_gb(),
            metrics.memory.total_gb()
        ));

        output.push_str(&format!(
            "Disk:       [{:6.1}%] ",
            metrics.disk.usage_percent
        ));
        output.push_str(&self.create_bar(metrics.disk.usage_percent, 40));
        output.push_str("\n\n");

        output.push_str(&format!(
            "Disk I/O:    ↑ {:7.1} MB/s  ↓ {:7.1} MB/s  ({} IOPS)\n",
            metrics.disk.read_mb_per_sec(),
            metrics.disk.write_mb_per_sec(),
            metrics.disk.total_iops()
        ));

        output.push_str(&format!(
            "Network I/O: ↑ {:7.1} MB/s  ↓ {:7.1} MB/s\n",
            metrics.network.rx_mb_per_sec(),
            metrics.network.tx_mb_per_sec()
        ));

        output.push_str(&format!(
            "\nTimestamp: {}\n",
            metrics.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        output
    }
}

impl Default for MonitoringReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::monitoring::analyzer::{Bottleneck, PerformanceStatus};
    #[allow(unused_imports)]
    use crate::monitoring::metrics::{
        CPUMetrics, DiskMetrics, MemoryMetrics, NetworkMetrics, VMMetrics,
    };

    #[test]
    fn test_create_bar() {
        let reporter = MonitoringReporter::new();

        let bar = reporter.create_bar(50.0, 10);
        assert!(bar.contains('['));
        assert!(bar.contains(']'));
        assert!(bar.len() > 10);

        let bar_high = reporter.create_bar(95.0, 10);
        assert!(bar_high.contains('▓'));
    }

    #[test]
    fn test_format_summary() {
        let reporter = MonitoringReporter::new();

        let report = PerformanceReport {
            vm_name: "test-vm".to_string(),
            performance_score: 85,
            current_metrics: VMMetrics::new(),
            average_usage: crate::monitoring::metrics::ResourceUsage {
                cpu_percent: 60.0,
                memory_percent: 65.0,
                disk_percent: 45.0,
                network_mb_per_sec: 10.0,
            },
            peak_usage: crate::monitoring::metrics::ResourceUsage {
                cpu_percent: 75.0,
                memory_percent: 80.0,
                disk_percent: 55.0,
                network_mb_per_sec: 20.0,
            },
            bottlenecks: Vec::new(),
            recommendations: Vec::new(),
            status: PerformanceStatus::Good,
        };

        let summary = reporter.format_summary(&report);
        assert!(summary.contains("test-vm"));
        assert!(summary.contains("85/100"));
        assert!(summary.contains("Good"));
    }

    #[test]
    fn test_format_comparison() {
        let reporter = MonitoringReporter::new();

        let vms = vec![
            ("vm1".to_string(), 85, "Good".to_string()),
            ("vm2".to_string(), 95, "Excellent".to_string()),
            ("vm3".to_string(), 60, "Fair".to_string()),
        ];

        let output = reporter.format_comparison(vms);
        assert!(output.contains("VM NAME"));
        assert!(output.contains("SCORE"));
        assert!(output.contains("vm1"));
        assert!(output.contains("85/100"));
    }

    #[test]
    fn test_format_live_metrics() {
        let reporter = MonitoringReporter::new();
        let metrics = VMMetrics::new();

        let output = reporter.format_live_metrics("test-vm", &metrics);
        assert!(output.contains("Live Metrics"));
        assert!(output.contains("CPU Usage"));
        assert!(output.contains("Memory"));
        assert!(output.contains("Disk I/O"));
    }
}
