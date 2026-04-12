use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Telemetry type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelemetryType {
    Metric,
    Log,
    Trace,
    Event,
}

/// Metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub source: String,
    pub tags: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

impl MetricDataPoint {
    pub fn new(
        name: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!("metric-{}-{}", name_str, Utc::now().timestamp_micros());

        Self {
            id,
            name: name_str,
            value,
            unit: unit.into(),
            source: source.into(),
            tags: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn add_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }
}

/// Telemetry stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryStream {
    pub id: String,
    pub name: String,
    pub telemetry_type: TelemetryType,
    pub source_id: String,
    pub destination: String,
    pub sample_rate_seconds: u32,
    pub buffer_size: u32,
    pub compression_enabled: bool,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl TelemetryStream {
    pub fn new(
        name: impl Into<String>,
        telemetry_type: TelemetryType,
        source_id: impl Into<String>,
        destination: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "stream-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            telemetry_type,
            source_id: source_id.into(),
            destination: destination.into(),
            sample_rate_seconds: 60,
            buffer_size: 1000,
            compression_enabled: true,
            active: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_sample_rate(mut self, seconds: u32) -> Self {
        self.sample_rate_seconds = seconds;
        self
    }

    pub fn with_buffer_size(mut self, size: u32) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn disable_compression(mut self) -> Self {
        self.compression_enabled = false;
        self
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn activate(&mut self) {
        self.active = true;
    }
}

/// Telemetry manager
pub struct TelemetryManager {
    streams: HashMap<String, TelemetryStream>,
    metrics: Vec<MetricDataPoint>,
}

impl TelemetryManager {
    pub fn new() -> Self {
        Self {
            streams: HashMap::new(),
            metrics: Vec::new(),
        }
    }

    pub fn add_stream(&mut self, stream: TelemetryStream) -> String {
        let id = stream.id.clone();
        self.streams.insert(id.clone(), stream);
        id
    }

    pub fn get_stream(&self, id: &str) -> Option<&TelemetryStream> {
        self.streams.get(id)
    }

    pub fn get_stream_mut(&mut self, id: &str) -> Option<&mut TelemetryStream> {
        self.streams.get_mut(id)
    }

    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }

    pub fn add_metric(&mut self, metric: MetricDataPoint) {
        self.metrics.push(metric);
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }

    pub fn streams_by_type(&self, telemetry_type: &TelemetryType) -> Vec<&TelemetryStream> {
        self.streams
            .values()
            .filter(|s| &s.telemetry_type == telemetry_type)
            .collect()
    }

    pub fn active_streams(&self) -> Vec<&TelemetryStream> {
        self.streams.values().filter(|s| s.active).collect()
    }

    pub fn streams_by_source(&self, source_id: &str) -> Vec<&TelemetryStream> {
        self.streams
            .values()
            .filter(|s| s.source_id == source_id)
            .collect()
    }

    pub fn metrics_by_name(&self, name: &str) -> Vec<&MetricDataPoint> {
        self.metrics.iter().filter(|m| m.name == name).collect()
    }

    pub fn metrics_by_source(&self, source: &str) -> Vec<&MetricDataPoint> {
        self.metrics.iter().filter(|m| m.source == source).collect()
    }
}

impl Default for TelemetryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_data_point() {
        let metric = MetricDataPoint::new("cpu_usage", 75.5, "percent", "edge-node-1");

        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.value, 75.5);
        assert_eq!(metric.unit, "percent");
        assert_eq!(metric.source, "edge-node-1");
    }

    #[test]
    fn test_metric_add_tag() {
        let mut metric = MetricDataPoint::new("memory", 4096.0, "MB", "node-1");

        metric.add_tag("region", "us-west");
        metric.add_tag("env", "prod");

        assert_eq!(metric.tags.len(), 2);
        assert_eq!(metric.tags.get("region"), Some(&"us-west".to_string()));
    }

    #[test]
    fn test_telemetry_stream() {
        let stream = TelemetryStream::new(
            "metrics-stream",
            TelemetryType::Metric,
            "edge-1",
            "cloud-storage",
        );

        assert_eq!(stream.name, "metrics-stream");
        assert_eq!(stream.telemetry_type, TelemetryType::Metric);
        assert_eq!(stream.source_id, "edge-1");
        assert_eq!(stream.destination, "cloud-storage");
        assert!(stream.active);
        assert!(stream.compression_enabled);
    }

    #[test]
    fn test_stream_with_sample_rate() {
        let stream = TelemetryStream::new("stream", TelemetryType::Metric, "src", "dst")
            .with_sample_rate(30);

        assert_eq!(stream.sample_rate_seconds, 30);
    }

    #[test]
    fn test_stream_with_buffer_size() {
        let stream =
            TelemetryStream::new("stream", TelemetryType::Log, "src", "dst").with_buffer_size(5000);

        assert_eq!(stream.buffer_size, 5000);
    }

    #[test]
    fn test_stream_disable_compression() {
        let stream = TelemetryStream::new("stream", TelemetryType::Event, "src", "dst")
            .disable_compression();

        assert!(!stream.compression_enabled);
    }

    #[test]
    fn test_stream_deactivate_activate() {
        let mut stream = TelemetryStream::new("stream", TelemetryType::Trace, "src", "dst");

        assert!(stream.active);

        stream.deactivate();
        assert!(!stream.active);

        stream.activate();
        assert!(stream.active);
    }

    #[test]
    fn test_telemetry_manager() {
        let mut manager = TelemetryManager::new();

        let stream = TelemetryStream::new("stream", TelemetryType::Metric, "src", "dst");
        let id = manager.add_stream(stream);

        assert_eq!(manager.stream_count(), 1);
        assert!(manager.get_stream(&id).is_some());
    }

    #[test]
    fn test_manager_add_metric() {
        let mut manager = TelemetryManager::new();

        let metric = MetricDataPoint::new("cpu", 50.0, "percent", "node-1");
        manager.add_metric(metric);

        assert_eq!(manager.metric_count(), 1);
    }

    #[test]
    fn test_manager_streams_by_type() {
        let mut manager = TelemetryManager::new();

        manager.add_stream(TelemetryStream::new(
            "s1",
            TelemetryType::Metric,
            "src",
            "dst",
        ));
        manager.add_stream(TelemetryStream::new("s2", TelemetryType::Log, "src", "dst"));
        manager.add_stream(TelemetryStream::new(
            "s3",
            TelemetryType::Metric,
            "src",
            "dst",
        ));

        let metrics = manager.streams_by_type(&TelemetryType::Metric);
        assert_eq!(metrics.len(), 2);
    }

    #[test]
    fn test_manager_active_streams() {
        let mut manager = TelemetryManager::new();

        let stream1 = TelemetryStream::new("s1", TelemetryType::Metric, "src", "dst");
        let mut stream2 = TelemetryStream::new("s2", TelemetryType::Log, "src", "dst");
        stream2.deactivate();

        manager.add_stream(stream1);
        manager.add_stream(stream2);

        let active = manager.active_streams();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_streams_by_source() {
        let mut manager = TelemetryManager::new();

        manager.add_stream(TelemetryStream::new(
            "s1",
            TelemetryType::Metric,
            "edge-1",
            "dst",
        ));
        manager.add_stream(TelemetryStream::new(
            "s2",
            TelemetryType::Log,
            "edge-2",
            "dst",
        ));
        manager.add_stream(TelemetryStream::new(
            "s3",
            TelemetryType::Event,
            "edge-1",
            "dst",
        ));

        let edge1_streams = manager.streams_by_source("edge-1");
        assert_eq!(edge1_streams.len(), 2);
    }

    #[test]
    fn test_manager_metrics_by_name() {
        let mut manager = TelemetryManager::new();

        manager.add_metric(MetricDataPoint::new("cpu", 50.0, "percent", "node-1"));
        manager.add_metric(MetricDataPoint::new("memory", 4096.0, "MB", "node-1"));
        manager.add_metric(MetricDataPoint::new("cpu", 60.0, "percent", "node-2"));

        let cpu_metrics = manager.metrics_by_name("cpu");
        assert_eq!(cpu_metrics.len(), 2);
    }

    #[test]
    fn test_manager_metrics_by_source() {
        let mut manager = TelemetryManager::new();

        manager.add_metric(MetricDataPoint::new("cpu", 50.0, "percent", "node-1"));
        manager.add_metric(MetricDataPoint::new("memory", 4096.0, "MB", "node-2"));
        manager.add_metric(MetricDataPoint::new("disk", 100.0, "GB", "node-1"));

        let node1_metrics = manager.metrics_by_source("node-1");
        assert_eq!(node1_metrics.len(), 2);
    }

    #[test]
    fn test_telemetry_type_equality() {
        assert_eq!(TelemetryType::Metric, TelemetryType::Metric);
        assert_ne!(TelemetryType::Metric, TelemetryType::Log);
    }
}
