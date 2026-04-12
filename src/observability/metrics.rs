// Metrics - Metric collection and aggregation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metric type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricType {
    Counter,   // Monotonically increasing value
    Gauge,     // Value that can go up or down
    Histogram, // Distribution of values
    Summary,   // Similar to histogram with quantiles
}

/// Metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub labels: HashMap<String, String>,
    pub unit: Option<String>,
}

impl Metric {
    pub fn new(name: impl Into<String>, metric_type: MetricType, value: f64) -> Self {
        Self {
            name: name.into(),
            metric_type,
            value,
            timestamp: Utc::now(),
            labels: HashMap::new(),
            unit: None,
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

/// Metric collector
pub struct MetricCollector {
    metrics: std::collections::VecDeque<Metric>,
    max_metrics: usize,
}

impl MetricCollector {
    pub fn new() -> Self {
        Self {
            metrics: std::collections::VecDeque::new(),
            max_metrics: 50_000,
        }
    }

    pub fn record(&mut self, metric: Metric) {
        if self.metrics.len() >= self.max_metrics {
            self.metrics.pop_front();
        }
        self.metrics.push_back(metric);
    }

    pub fn counter(&mut self, name: impl Into<String>, value: f64) {
        self.record(Metric::new(name, MetricType::Counter, value));
    }

    pub fn gauge(&mut self, name: impl Into<String>, value: f64) {
        self.record(Metric::new(name, MetricType::Gauge, value));
    }

    pub fn histogram(&mut self, name: impl Into<String>, value: f64) {
        self.record(Metric::new(name, MetricType::Histogram, value));
    }

    pub fn get_metrics(&self) -> Vec<&Metric> {
        self.metrics.iter().collect()
    }

    pub fn clear(&mut self) {
        self.metrics.clear();
    }

    pub fn count(&self) -> usize {
        self.metrics.len()
    }

    /// Get metrics by name
    pub fn by_name(&self, name: &str) -> Vec<&Metric> {
        self.metrics.iter().filter(|m| m.name == name).collect()
    }

    /// Get metrics by type
    pub fn by_type(&self, metric_type: MetricType) -> Vec<&Metric> {
        self.metrics
            .iter()
            .filter(|m| m.metric_type == metric_type)
            .collect()
    }

    /// Get latest value for a metric
    pub fn latest_value(&self, name: &str) -> Option<f64> {
        self.metrics
            .iter()
            .filter(|m| m.name == name)
            .next_back()
            .map(|m| m.value)
    }
}

impl Default for MetricCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Metric aggregation
pub struct MetricAggregator;

impl MetricAggregator {
    /// Calculate average of metric values
    pub fn average(metrics: &[Metric]) -> f64 {
        if metrics.is_empty() {
            return 0.0;
        }

        let sum: f64 = metrics.iter().map(|m| m.value).sum();
        sum / metrics.len() as f64
    }

    /// Calculate sum of metric values
    pub fn sum(metrics: &[Metric]) -> f64 {
        metrics.iter().map(|m| m.value).sum()
    }

    /// Calculate max of metric values
    pub fn max(metrics: &[Metric]) -> Option<f64> {
        metrics
            .iter()
            .map(|m| m.value)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Calculate min of metric values
    pub fn min(metrics: &[Metric]) -> Option<f64> {
        metrics
            .iter()
            .map(|m| m.value)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Calculate rate of change (per second)
    pub fn rate(metrics: &[Metric]) -> Option<f64> {
        if metrics.len() < 2 {
            return None;
        }

        let first = metrics.first()?;
        let last = metrics.last()?;

        let value_diff = last.value - first.value;
        let time_diff = last
            .timestamp
            .signed_duration_since(first.timestamp)
            .num_seconds() as f64;

        if time_diff > 0.0 {
            Some(value_diff / time_diff)
        } else {
            None
        }
    }

    /// Calculate percentile
    pub fn percentile(metrics: &[Metric], p: f64) -> Option<f64> {
        if metrics.is_empty() {
            return None;
        }

        let mut values: Vec<f64> = metrics.iter().map(|m| m.value).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p_clamped = p.clamp(0.0, 100.0);
        let index = ((p_clamped / 100.0) * (values.len() - 1) as f64) as usize;
        Some(values[index.min(values.len() - 1)])
    }
}

/// Metric query
#[derive(Debug, Clone)]
pub struct MetricQuery {
    pub metric_name: Option<String>,
    pub metric_type: Option<MetricType>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub labels: HashMap<String, String>,
}

impl MetricQuery {
    pub fn new() -> Self {
        Self {
            metric_name: None,
            metric_type: None,
            start_time: None,
            end_time: None,
            labels: HashMap::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.metric_name = Some(name.into());
        self
    }

    pub fn with_type(mut self, metric_type: MetricType) -> Self {
        self.metric_type = Some(metric_type);
        self
    }

    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn matches(&self, metric: &Metric) -> bool {
        // Check metric name
        if let Some(ref name) = self.metric_name {
            if metric.name != *name {
                return false;
            }
        }

        // Check metric type
        if let Some(ref mtype) = self.metric_type {
            if metric.metric_type != *mtype {
                return false;
            }
        }

        // Check time range
        if let Some(start) = self.start_time {
            if metric.timestamp < start {
                return false;
            }
        }
        if let Some(end) = self.end_time {
            if metric.timestamp > end {
                return false;
            }
        }

        // Check labels
        for (key, value) in &self.labels {
            if metric.labels.get(key) != Some(value) {
                return false;
            }
        }

        true
    }
}

impl Default for MetricQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// VM metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMMetrics {
    pub vm_name: String,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub disk_read_bytes_per_sec: f64,
    pub disk_write_bytes_per_sec: f64,
    pub network_rx_bytes_per_sec: f64,
    pub network_tx_bytes_per_sec: f64,
    pub collected_at: DateTime<Utc>,
}

impl VMMetrics {
    pub fn new(vm_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            memory_usage_bytes: 0,
            disk_read_bytes_per_sec: 0.0,
            disk_write_bytes_per_sec: 0.0,
            network_rx_bytes_per_sec: 0.0,
            network_tx_bytes_per_sec: 0.0,
            collected_at: Utc::now(),
        }
    }

    pub fn with_cpu(mut self, usage_percent: f64) -> Self {
        self.cpu_usage_percent = usage_percent;
        self
    }

    pub fn with_memory(mut self, usage_percent: f64, usage_bytes: u64) -> Self {
        self.memory_usage_percent = usage_percent;
        self.memory_usage_bytes = usage_bytes;
        self
    }

    pub fn with_disk_io(mut self, read_bps: f64, write_bps: f64) -> Self {
        self.disk_read_bytes_per_sec = read_bps;
        self.disk_write_bytes_per_sec = write_bps;
        self
    }

    pub fn with_network_io(mut self, rx_bps: f64, tx_bps: f64) -> Self {
        self.network_rx_bytes_per_sec = rx_bps;
        self.network_tx_bytes_per_sec = tx_bps;
        self
    }

    pub fn is_high_cpu(&self, threshold: f64) -> bool {
        self.cpu_usage_percent > threshold
    }

    pub fn is_high_memory(&self, threshold: f64) -> bool {
        self.memory_usage_percent > threshold
    }
}

/// Metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub vm_metrics: Vec<VMMetrics>,
    pub cluster_cpu_usage: f64,
    pub cluster_memory_usage: f64,
    pub total_vms: usize,
}

impl MetricsSnapshot {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            vm_metrics: Vec::new(),
            cluster_cpu_usage: 0.0,
            cluster_memory_usage: 0.0,
            total_vms: 0,
        }
    }

    pub fn add_vm_metrics(&mut self, metrics: VMMetrics) {
        self.vm_metrics.push(metrics);
        self.total_vms = self.vm_metrics.len();
        self.recalculate_cluster_metrics();
    }

    fn recalculate_cluster_metrics(&mut self) {
        if self.vm_metrics.is_empty() {
            self.cluster_cpu_usage = 0.0;
            self.cluster_memory_usage = 0.0;
            return;
        }

        self.cluster_cpu_usage = self
            .vm_metrics
            .iter()
            .map(|m| m.cpu_usage_percent)
            .sum::<f64>()
            / self.vm_metrics.len() as f64;

        self.cluster_memory_usage = self
            .vm_metrics
            .iter()
            .map(|m| m.memory_usage_percent)
            .sum::<f64>()
            / self.vm_metrics.len() as f64;
    }

    pub fn high_cpu_vms(&self, threshold: f64) -> Vec<&VMMetrics> {
        self.vm_metrics
            .iter()
            .filter(|m| m.is_high_cpu(threshold))
            .collect()
    }

    pub fn high_memory_vms(&self, threshold: f64) -> Vec<&VMMetrics> {
        self.vm_metrics
            .iter()
            .filter(|m| m.is_high_memory(threshold))
            .collect()
    }
}

impl Default for MetricsSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta as Duration;

    #[test]
    fn test_metric_creation() {
        let metric = Metric::new("cpu_usage", MetricType::Gauge, 75.5)
            .with_label("vm", "test-vm")
            .with_unit("percent");

        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.metric_type, MetricType::Gauge);
        assert_eq!(metric.value, 75.5);
        assert_eq!(metric.unit, Some("percent".to_string()));
    }

    #[test]
    fn test_metric_collector() {
        let mut collector = MetricCollector::new();

        collector.counter("requests_total", 100.0);
        collector.gauge("cpu_usage", 50.0);
        collector.histogram("response_time", 0.15);

        assert_eq!(collector.count(), 3);
        assert_eq!(collector.by_type(MetricType::Counter).len(), 1);
        assert_eq!(collector.by_type(MetricType::Gauge).len(), 1);
    }

    #[test]
    fn test_metric_aggregation() {
        let metrics = vec![
            Metric::new("cpu", MetricType::Gauge, 50.0),
            Metric::new("cpu", MetricType::Gauge, 60.0),
            Metric::new("cpu", MetricType::Gauge, 70.0),
        ];

        assert_eq!(MetricAggregator::average(&metrics), 60.0);
        assert_eq!(MetricAggregator::sum(&metrics), 180.0);
        assert_eq!(MetricAggregator::max(&metrics), Some(70.0));
        assert_eq!(MetricAggregator::min(&metrics), Some(50.0));
    }

    #[test]
    fn test_metric_percentile() {
        let mut metrics = Vec::new();
        for i in 1..=100 {
            metrics.push(Metric::new("latency", MetricType::Histogram, i as f64));
        }

        assert_eq!(MetricAggregator::percentile(&metrics, 50.0), Some(50.0));
        assert_eq!(MetricAggregator::percentile(&metrics, 95.0), Some(95.0));
        assert_eq!(MetricAggregator::percentile(&metrics, 99.0), Some(99.0));
    }

    #[test]
    fn test_metric_rate() {
        let now = Utc::now();
        let metrics = vec![
            Metric::new("requests", MetricType::Counter, 100.0).with_timestamp(now),
            Metric::new("requests", MetricType::Counter, 200.0)
                .with_timestamp(now + Duration::seconds(10)),
        ];

        let rate = MetricAggregator::rate(&metrics).unwrap();
        assert!((rate - 10.0).abs() < 0.1); // ~10 requests per second
    }

    #[test]
    fn test_metric_query() {
        let now = Utc::now();
        let metric = Metric::new("cpu_usage", MetricType::Gauge, 75.0)
            .with_label("vm", "test-vm")
            .with_timestamp(now);

        let query = MetricQuery::new()
            .with_name("cpu_usage")
            .with_type(MetricType::Gauge)
            .with_label("vm", "test-vm");

        assert!(query.matches(&metric));

        let wrong_query = MetricQuery::new().with_name("memory_usage");

        assert!(!wrong_query.matches(&metric));
    }

    #[test]
    fn test_metric_query_time_range() {
        let now = Utc::now();
        let metric = Metric::new("cpu", MetricType::Gauge, 50.0).with_timestamp(now);

        let query =
            MetricQuery::new().with_time_range(now - Duration::hours(1), now + Duration::hours(1));

        assert!(query.matches(&metric));

        let past_query =
            MetricQuery::new().with_time_range(now - Duration::hours(2), now - Duration::hours(1));

        assert!(!past_query.matches(&metric));
    }

    #[test]
    fn test_vm_metrics() {
        let metrics = VMMetrics::new("test-vm")
            .with_cpu(85.0)
            .with_memory(60.0, 4_000_000_000)
            .with_disk_io(1_000_000.0, 500_000.0)
            .with_network_io(2_000_000.0, 1_000_000.0);

        assert_eq!(metrics.vm_name, "test-vm");
        assert_eq!(metrics.cpu_usage_percent, 85.0);
        assert_eq!(metrics.memory_usage_percent, 60.0);
        assert!(metrics.is_high_cpu(80.0));
        assert!(!metrics.is_high_memory(70.0));
    }

    #[test]
    fn test_metrics_snapshot() {
        let mut snapshot = MetricsSnapshot::new();

        snapshot.add_vm_metrics(
            VMMetrics::new("vm1")
                .with_cpu(50.0)
                .with_memory(40.0, 2_000_000_000),
        );
        snapshot.add_vm_metrics(
            VMMetrics::new("vm2")
                .with_cpu(70.0)
                .with_memory(60.0, 3_000_000_000),
        );
        snapshot.add_vm_metrics(
            VMMetrics::new("vm3")
                .with_cpu(90.0)
                .with_memory(80.0, 4_000_000_000),
        );

        assert_eq!(snapshot.total_vms, 3);
        assert_eq!(snapshot.cluster_cpu_usage, 70.0);
        assert_eq!(snapshot.cluster_memory_usage, 60.0);
    }

    #[test]
    fn test_high_resource_vms() {
        let mut snapshot = MetricsSnapshot::new();

        snapshot.add_vm_metrics(
            VMMetrics::new("vm1")
                .with_cpu(50.0)
                .with_memory(40.0, 2_000_000_000),
        );
        snapshot.add_vm_metrics(
            VMMetrics::new("vm2")
                .with_cpu(85.0)
                .with_memory(90.0, 4_000_000_000),
        );
        snapshot.add_vm_metrics(
            VMMetrics::new("vm3")
                .with_cpu(30.0)
                .with_memory(50.0, 2_500_000_000),
        );

        let high_cpu = snapshot.high_cpu_vms(80.0);
        assert_eq!(high_cpu.len(), 1);
        assert_eq!(high_cpu[0].vm_name, "vm2");

        let high_memory = snapshot.high_memory_vms(80.0);
        assert_eq!(high_memory.len(), 1);
        assert_eq!(high_memory[0].vm_name, "vm2");
    }

    #[test]
    fn test_collector_by_name() {
        let mut collector = MetricCollector::new();

        collector.gauge("cpu_usage", 50.0);
        collector.gauge("cpu_usage", 60.0);
        collector.gauge("memory_usage", 70.0);

        let cpu_metrics = collector.by_name("cpu_usage");
        assert_eq!(cpu_metrics.len(), 2);

        let latest = collector.latest_value("cpu_usage");
        assert_eq!(latest, Some(60.0));
    }

    #[test]
    fn test_collector_clear() {
        let mut collector = MetricCollector::new();

        collector.counter("requests", 100.0);
        collector.gauge("cpu", 50.0);

        assert_eq!(collector.count(), 2);

        collector.clear();
        assert_eq!(collector.count(), 0);
    }

    #[test]
    fn test_metric_types() {
        assert_eq!(MetricType::Counter, MetricType::Counter);
        assert_ne!(MetricType::Counter, MetricType::Gauge);
    }
}
