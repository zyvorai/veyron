// Bandwidth Monitoring - Network bandwidth usage and monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Bandwidth metrics for a network interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthMetrics {
    pub interface_name: String,
    pub rx_bytes: u64,   // Received bytes
    pub tx_bytes: u64,   // Transmitted bytes
    pub rx_packets: u64, // Received packets
    pub tx_packets: u64, // Transmitted packets
    pub rx_errors: u64,  // Receive errors
    pub tx_errors: u64,  // Transmit errors
    pub rx_dropped: u64, // Dropped receive packets
    pub tx_dropped: u64, // Dropped transmit packets
    pub timestamp: DateTime<Utc>,
}

impl BandwidthMetrics {
    pub fn new(interface_name: impl Into<String>) -> Self {
        Self {
            interface_name: interface_name.into(),
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
            rx_errors: 0,
            tx_errors: 0,
            rx_dropped: 0,
            tx_dropped: 0,
            timestamp: Utc::now(),
        }
    }

    /// Calculate total bytes (rx + tx)
    pub fn total_bytes(&self) -> u64 {
        self.rx_bytes + self.tx_bytes
    }

    /// Calculate total packets (rx + tx)
    pub fn total_packets(&self) -> u64 {
        self.rx_packets + self.tx_packets
    }

    /// Calculate total errors (rx + tx)
    pub fn total_errors(&self) -> u64 {
        self.rx_errors + self.tx_errors
    }

    /// Calculate total dropped (rx + tx)
    pub fn total_dropped(&self) -> u64 {
        self.rx_dropped + self.tx_dropped
    }

    /// Calculate error rate percentage
    pub fn error_rate(&self) -> f64 {
        let total_packets = self.total_packets();
        if total_packets == 0 {
            0.0
        } else {
            (self.total_errors() as f64 / total_packets as f64) * 100.0
        }
    }

    /// Calculate drop rate percentage
    pub fn drop_rate(&self) -> f64 {
        let total_packets = self.total_packets();
        if total_packets == 0 {
            0.0
        } else {
            (self.total_dropped() as f64 / total_packets as f64) * 100.0
        }
    }

    /// Format bytes to human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Format bandwidth rate (bytes/sec)
    pub fn format_rate(bytes_per_sec: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes_per_sec >= GB {
            format!("{:.2} GB/s", bytes_per_sec as f64 / GB as f64)
        } else if bytes_per_sec >= MB {
            format!("{:.2} MB/s", bytes_per_sec as f64 / MB as f64)
        } else if bytes_per_sec >= KB {
            format!("{:.2} KB/s", bytes_per_sec as f64 / KB as f64)
        } else {
            format!("{} B/s", bytes_per_sec)
        }
    }
}

/// Bandwidth statistics over a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthStats {
    pub interface_name: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub rx_rate: u64,      // Bytes per second
    pub tx_rate: u64,      // Bytes per second
    pub peak_rx_rate: u64, // Peak receive rate
    pub peak_tx_rate: u64, // Peak transmit rate
    pub avg_rx_rate: u64,  // Average receive rate
    pub avg_tx_rate: u64,  // Average transmit rate
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
}

impl BandwidthStats {
    pub fn new(interface_name: impl Into<String>) -> Self {
        Self {
            interface_name: interface_name.into(),
            period_start: Utc::now(),
            period_end: Utc::now(),
            rx_rate: 0,
            tx_rate: 0,
            peak_rx_rate: 0,
            peak_tx_rate: 0,
            avg_rx_rate: 0,
            avg_tx_rate: 0,
            total_rx_bytes: 0,
            total_tx_bytes: 0,
        }
    }

    /// Calculate total rate
    pub fn total_rate(&self) -> u64 {
        self.rx_rate + self.tx_rate
    }

    /// Calculate peak rate
    pub fn peak_rate(&self) -> u64 {
        std::cmp::max(self.peak_rx_rate, self.peak_tx_rate)
    }

    /// Calculate average rate
    pub fn avg_rate(&self) -> u64 {
        self.avg_rx_rate + self.avg_tx_rate
    }

    /// Calculate utilization percentage (assuming 1 Gbps link)
    pub fn utilization_percent(&self, link_speed_mbps: u64) -> f64 {
        if link_speed_mbps == 0 {
            return 0.0;
        }
        let link_speed_bytes = link_speed_mbps.saturating_mul(1_000_000) / 8;
        (self.total_rate() as f64 / link_speed_bytes as f64) * 100.0
    }
}

/// Bandwidth monitor for tracking network usage
pub struct BandwidthMonitor {
    interface_name: String,
    samples: std::collections::VecDeque<BandwidthMetrics>,
    max_samples: usize,
}

impl BandwidthMonitor {
    pub fn new(interface_name: impl Into<String>) -> Self {
        Self {
            interface_name: interface_name.into(),
            samples: std::collections::VecDeque::new(),
            max_samples: 100,
        }
    }

    pub fn with_max_samples(mut self, max_samples: usize) -> Self {
        self.max_samples = max_samples;
        self
    }

    /// Add a bandwidth sample
    pub fn add_sample(&mut self, metrics: BandwidthMetrics) {
        self.samples.push_back(metrics);

        // Keep only the latest samples (O(1) removal from front)
        while self.samples.len() > self.max_samples {
            self.samples.pop_front();
        }
    }

    /// Calculate bandwidth statistics between two samples
    pub fn calculate_stats(&self, start_idx: usize, end_idx: usize) -> Option<BandwidthStats> {
        if start_idx >= self.samples.len() || end_idx >= self.samples.len() || start_idx >= end_idx
        {
            return None;
        }

        let start = &self.samples[start_idx];
        let end = &self.samples[end_idx];

        let duration = end.timestamp.signed_duration_since(start.timestamp);
        let duration_secs = duration.num_seconds();

        if duration_secs <= 0 {
            return None;
        }
        let duration_secs = duration_secs as u64;

        let rx_bytes_diff = end.rx_bytes.saturating_sub(start.rx_bytes);
        let tx_bytes_diff = end.tx_bytes.saturating_sub(start.tx_bytes);

        let rx_rate = rx_bytes_diff / duration_secs;
        let tx_rate = tx_bytes_diff / duration_secs;

        // Calculate peak and average rates
        let mut peak_rx = 0;
        let mut peak_tx = 0;
        let mut sum_rx = 0;
        let mut sum_tx = 0;
        let mut count = 0;

        for i in start_idx..end_idx {
            if i + 1 < self.samples.len() {
                let curr = &self.samples[i];
                let next = &self.samples[i + 1];

                let dur_secs = next
                    .timestamp
                    .signed_duration_since(curr.timestamp)
                    .num_seconds();
                if dur_secs > 0 {
                    let dur = dur_secs as u64;
                    let rx = next.rx_bytes.saturating_sub(curr.rx_bytes) / dur;
                    let tx = next.tx_bytes.saturating_sub(curr.tx_bytes) / dur;

                    peak_rx = std::cmp::max(peak_rx, rx);
                    peak_tx = std::cmp::max(peak_tx, tx);
                    sum_rx += rx;
                    sum_tx += tx;
                    count += 1;
                }
            }
        }

        let avg_rx = if count > 0 { sum_rx / count } else { rx_rate };
        let avg_tx = if count > 0 { sum_tx / count } else { tx_rate };

        Some(BandwidthStats {
            interface_name: self.interface_name.clone(),
            period_start: start.timestamp,
            period_end: end.timestamp,
            rx_rate,
            tx_rate,
            peak_rx_rate: peak_rx,
            peak_tx_rate: peak_tx,
            avg_rx_rate: avg_rx,
            avg_tx_rate: avg_tx,
            total_rx_bytes: rx_bytes_diff,
            total_tx_bytes: tx_bytes_diff,
        })
    }

    /// Get current bandwidth usage
    pub fn current_stats(&self) -> Option<BandwidthStats> {
        if self.samples.len() < 2 {
            return None;
        }
        self.calculate_stats(self.samples.len() - 2, self.samples.len() - 1)
    }

    /// Get overall statistics for all samples
    pub fn overall_stats(&self) -> Option<BandwidthStats> {
        if self.samples.len() < 2 {
            return None;
        }
        self.calculate_stats(0, self.samples.len() - 1)
    }

    /// Get sample count
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta as Duration;

    #[test]
    fn test_bandwidth_metrics_creation() {
        let metrics = BandwidthMetrics::new("eth0");
        assert_eq!(metrics.interface_name, "eth0");
        assert_eq!(metrics.rx_bytes, 0);
        assert_eq!(metrics.tx_bytes, 0);
    }

    #[test]
    fn test_bandwidth_metrics_totals() {
        let mut metrics = BandwidthMetrics::new("eth0");
        metrics.rx_bytes = 1000;
        metrics.tx_bytes = 2000;
        metrics.rx_packets = 10;
        metrics.tx_packets = 20;

        assert_eq!(metrics.total_bytes(), 3000);
        assert_eq!(metrics.total_packets(), 30);
    }

    #[test]
    fn test_error_rate() {
        let mut metrics = BandwidthMetrics::new("eth0");
        metrics.rx_packets = 100;
        metrics.tx_packets = 100;
        metrics.rx_errors = 5;
        metrics.tx_errors = 5;

        assert_eq!(metrics.error_rate(), 5.0);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(BandwidthMetrics::format_bytes(500), "500 B");
        assert_eq!(BandwidthMetrics::format_bytes(2048), "2.00 KB");
        assert_eq!(BandwidthMetrics::format_bytes(1048576), "1.00 MB");
        assert_eq!(BandwidthMetrics::format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_rate() {
        assert_eq!(BandwidthMetrics::format_rate(500), "500 B/s");
        assert_eq!(BandwidthMetrics::format_rate(2048), "2.00 KB/s");
        assert_eq!(BandwidthMetrics::format_rate(1048576), "1.00 MB/s");
    }

    #[test]
    fn test_bandwidth_stats() {
        let stats = BandwidthStats::new("eth0");
        assert_eq!(stats.interface_name, "eth0");
        assert_eq!(stats.total_rate(), 0);
    }

    #[test]
    fn test_utilization_percent() {
        let mut stats = BandwidthStats::new("eth0");
        stats.rx_rate = 62_500_000; // 500 Mbps
        stats.tx_rate = 62_500_000; // 500 Mbps

        // Total 1 Gbps on a 1 Gbps link = 100%
        assert!(stats.utilization_percent(1000).abs() - 100.0 < 1.0);
    }

    #[test]
    fn test_bandwidth_monitor() {
        let mut monitor = BandwidthMonitor::new("eth0");
        assert_eq!(monitor.sample_count(), 0);

        let mut metrics1 = BandwidthMetrics::new("eth0");
        metrics1.rx_bytes = 1000;
        metrics1.tx_bytes = 2000;
        monitor.add_sample(metrics1);

        assert_eq!(monitor.sample_count(), 1);
    }

    #[test]
    fn test_calculate_stats() {
        let mut monitor = BandwidthMonitor::new("eth0");

        let mut metrics1 = BandwidthMetrics::new("eth0");
        metrics1.rx_bytes = 1000;
        metrics1.tx_bytes = 2000;
        metrics1.timestamp = Utc::now();
        monitor.add_sample(metrics1);

        let mut metrics2 = BandwidthMetrics::new("eth0");
        metrics2.rx_bytes = 2000; // +1000 bytes
        metrics2.tx_bytes = 4000; // +2000 bytes
        metrics2.timestamp = Utc::now() + Duration::seconds(1);
        monitor.add_sample(metrics2);

        let stats = monitor.current_stats().unwrap();
        assert_eq!(stats.rx_rate, 1000);
        assert_eq!(stats.tx_rate, 2000);
        assert_eq!(stats.total_rx_bytes, 1000);
        assert_eq!(stats.total_tx_bytes, 2000);
    }

    #[test]
    fn test_max_samples() {
        let mut monitor = BandwidthMonitor::new("eth0").with_max_samples(5);

        for i in 0..10 {
            let mut metrics = BandwidthMetrics::new("eth0");
            metrics.rx_bytes = i * 1000;
            monitor.add_sample(metrics);
        }

        assert_eq!(monitor.sample_count(), 5);
    }
}
