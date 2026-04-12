use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub logging_enabled: bool,
    pub sampling_rate: f64,
    pub created_at: DateTime<Utc>,
}

impl TelemetryConfig {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "telem-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            tracing_enabled: true,
            metrics_enabled: true,
            logging_enabled: true,
            sampling_rate: 1.0,
            created_at: Utc::now(),
        }
    }

    pub fn with_tracing(mut self, enabled: bool) -> Self {
        self.tracing_enabled = enabled;
        self
    }

    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.metrics_enabled = enabled;
        self
    }

    pub fn with_logging(mut self, enabled: bool) -> Self {
        self.logging_enabled = enabled;
        self
    }

    pub fn with_sampling_rate(mut self, rate: f64) -> Self {
        self.sampling_rate = rate.clamp(0.0, 1.0);
        self
    }
}

/// Service mesh metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshMetrics {
    pub service_name: String,
    pub namespace: String,
    pub request_count: u64,
    pub error_count: u64,
    pub request_duration_ms: Vec<f64>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active_connections: u32,
    pub timestamp: DateTime<Utc>,
}

impl MeshMetrics {
    pub fn new(service: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            service_name: service.into(),
            namespace: namespace.into(),
            request_count: 0,
            error_count: 0,
            request_duration_ms: Vec::new(),
            bytes_sent: 0,
            bytes_received: 0,
            active_connections: 0,
            timestamp: Utc::now(),
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.request_count == 0 {
            0.0
        } else {
            (self.error_count as f64 / self.request_count as f64) * 100.0
        }
    }

    pub fn success_rate(&self) -> f64 {
        100.0 - self.error_rate()
    }

    pub fn avg_duration_ms(&self) -> f64 {
        if self.request_duration_ms.is_empty() {
            0.0
        } else {
            self.request_duration_ms.iter().sum::<f64>() / self.request_duration_ms.len() as f64
        }
    }

    pub fn p95_duration_ms(&self) -> f64 {
        if self.request_duration_ms.is_empty() {
            return 0.0;
        }

        let mut sorted = self.request_duration_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let index = ((sorted.len() as f64 * 0.95) as usize).min(sorted.len() - 1);
        sorted[index]
    }

    pub fn p99_duration_ms(&self) -> f64 {
        if self.request_duration_ms.is_empty() {
            return 0.0;
        }

        let mut sorted = self.request_duration_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let index = ((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1);
        sorted[index]
    }

    pub fn record_request(&mut self, duration_ms: f64, is_error: bool) {
        self.request_count += 1;
        if is_error {
            self.error_count += 1;
        }
        self.request_duration_ms.push(duration_ms);
    }
}

/// Distributed tracing span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub duration_ms: f64,
    pub tags: HashMap<String, String>,
    pub status_code: u16,
}

impl TracingSpan {
    pub fn new(
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        service: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self {
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id: None,
            service_name: service.into(),
            operation_name: operation.into(),
            start_time: Utc::now(),
            duration_ms: 0.0,
            tags: HashMap::new(),
            status_code: 200,
        }
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_id.into());
        self
    }

    pub fn add_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }

    pub fn finish(&mut self, duration_ms: f64, status_code: u16) {
        self.duration_ms = duration_ms;
        self.status_code = status_code;
    }

    pub fn is_error(&self) -> bool {
        self.status_code >= 400
    }
}

/// Service mesh observability manager
pub struct ObservabilityManager {
    telemetry_configs: HashMap<String, TelemetryConfig>,
    metrics: HashMap<String, MeshMetrics>,
    spans: HashMap<String, Vec<TracingSpan>>,
}

impl ObservabilityManager {
    pub fn new() -> Self {
        Self {
            telemetry_configs: HashMap::new(),
            metrics: HashMap::new(),
            spans: HashMap::new(),
        }
    }

    pub fn add_telemetry_config(&mut self, config: TelemetryConfig) -> String {
        let id = config.id.clone();
        self.telemetry_configs.insert(id.clone(), config);
        id
    }

    pub fn get_telemetry_config(&self, id: &str) -> Option<&TelemetryConfig> {
        self.telemetry_configs.get(id)
    }

    pub fn remove_telemetry_config(&mut self, id: &str) -> bool {
        self.telemetry_configs.remove(id).is_some()
    }

    pub fn telemetry_config_count(&self) -> usize {
        self.telemetry_configs.len()
    }

    pub fn add_metrics(&mut self, service: impl Into<String>, metrics: MeshMetrics) {
        self.metrics.insert(service.into(), metrics);
    }

    pub fn get_metrics(&self, service: &str) -> Option<&MeshMetrics> {
        self.metrics.get(service)
    }

    pub fn get_metrics_mut(&mut self, service: &str) -> Option<&mut MeshMetrics> {
        self.metrics.get_mut(service)
    }

    pub fn metrics_count(&self) -> usize {
        self.metrics.len()
    }

    pub fn add_span(&mut self, trace_id: impl Into<String>, span: TracingSpan) {
        let trace = trace_id.into();
        self.spans.entry(trace).or_default().push(span);
    }

    pub fn get_trace(&self, trace_id: &str) -> Option<&Vec<TracingSpan>> {
        self.spans.get(trace_id)
    }

    pub fn trace_count(&self) -> usize {
        self.spans.len()
    }

    pub fn span_count(&self) -> usize {
        self.spans.values().map(|v| v.len()).sum()
    }

    pub fn high_error_services(&self, threshold: f64) -> Vec<(&String, &MeshMetrics)> {
        self.metrics
            .iter()
            .filter(|(_, m)| m.error_rate() > threshold)
            .collect()
    }

    pub fn slow_services(&self, threshold_ms: f64) -> Vec<(&String, &MeshMetrics)> {
        self.metrics
            .iter()
            .filter(|(_, m)| m.avg_duration_ms() > threshold_ms)
            .collect()
    }
}

impl Default for ObservabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config() {
        let config = TelemetryConfig::new("my-telemetry", "default");

        assert_eq!(config.name, "my-telemetry");
        assert_eq!(config.namespace, "default");
        assert!(config.tracing_enabled);
        assert!(config.metrics_enabled);
        assert!(config.logging_enabled);
        assert_eq!(config.sampling_rate, 1.0);
    }

    #[test]
    fn test_telemetry_config_builder() {
        let config = TelemetryConfig::new("config", "default")
            .with_tracing(false)
            .with_sampling_rate(0.1);

        assert!(!config.tracing_enabled);
        assert_eq!(config.sampling_rate, 0.1);
    }

    #[test]
    fn test_sampling_rate_clamping() {
        let config1 = TelemetryConfig::new("c1", "default").with_sampling_rate(1.5);
        assert_eq!(config1.sampling_rate, 1.0);

        let config2 = TelemetryConfig::new("c2", "default").with_sampling_rate(-0.5);
        assert_eq!(config2.sampling_rate, 0.0);
    }

    #[test]
    fn test_mesh_metrics() {
        let metrics = MeshMetrics::new("my-service", "default");

        assert_eq!(metrics.service_name, "my-service");
        assert_eq!(metrics.namespace, "default");
        assert_eq!(metrics.request_count, 0);
        assert_eq!(metrics.error_count, 0);
    }

    #[test]
    fn test_metrics_error_rate() {
        let mut metrics = MeshMetrics::new("service", "default");
        metrics.request_count = 100;
        metrics.error_count = 5;

        assert_eq!(metrics.error_rate(), 5.0);
        assert_eq!(metrics.success_rate(), 95.0);
    }

    #[test]
    fn test_metrics_zero_requests() {
        let metrics = MeshMetrics::new("service", "default");

        assert_eq!(metrics.error_rate(), 0.0);
        assert_eq!(metrics.success_rate(), 100.0);
    }

    #[test]
    fn test_metrics_avg_duration() {
        let mut metrics = MeshMetrics::new("service", "default");
        metrics.request_duration_ms = vec![100.0, 200.0, 300.0];

        assert_eq!(metrics.avg_duration_ms(), 200.0);
    }

    #[test]
    fn test_metrics_percentiles() {
        let mut metrics = MeshMetrics::new("service", "default");
        metrics.request_duration_ms =
            vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];

        let p95 = metrics.p95_duration_ms();
        let p99 = metrics.p99_duration_ms();

        assert!(p95 >= 90.0);
        assert!(p99 >= 90.0);
    }

    #[test]
    fn test_metrics_record_request() {
        let mut metrics = MeshMetrics::new("service", "default");

        metrics.record_request(150.0, false);
        metrics.record_request(200.0, true);

        assert_eq!(metrics.request_count, 2);
        assert_eq!(metrics.error_count, 1);
        assert_eq!(metrics.request_duration_ms.len(), 2);
    }

    #[test]
    fn test_tracing_span() {
        let span = TracingSpan::new("trace-123", "span-1", "my-service", "GET /api");

        assert_eq!(span.trace_id, "trace-123");
        assert_eq!(span.span_id, "span-1");
        assert_eq!(span.service_name, "my-service");
        assert_eq!(span.operation_name, "GET /api");
        assert!(span.parent_span_id.is_none());
    }

    #[test]
    fn test_span_with_parent() {
        let span =
            TracingSpan::new("trace-1", "span-2", "service", "operation").with_parent("span-1");

        assert_eq!(span.parent_span_id, Some("span-1".to_string()));
    }

    #[test]
    fn test_span_tags() {
        let mut span = TracingSpan::new("trace-1", "span-1", "service", "operation");

        span.add_tag("http.method", "GET");
        span.add_tag("http.url", "/api/v1");

        assert_eq!(span.tags.len(), 2);
        assert_eq!(span.tags.get("http.method"), Some(&"GET".to_string()));
    }

    #[test]
    fn test_span_finish() {
        let mut span = TracingSpan::new("trace-1", "span-1", "service", "operation");

        span.finish(150.5, 200);

        assert_eq!(span.duration_ms, 150.5);
        assert_eq!(span.status_code, 200);
        assert!(!span.is_error());
    }

    #[test]
    fn test_span_error() {
        let mut span = TracingSpan::new("trace-1", "span-1", "service", "operation");

        span.finish(100.0, 500);

        assert!(span.is_error());
    }

    #[test]
    fn test_observability_manager() {
        let mut manager = ObservabilityManager::new();

        let config = TelemetryConfig::new("config", "default");
        let id = manager.add_telemetry_config(config);

        assert_eq!(manager.telemetry_config_count(), 1);
        assert!(manager.get_telemetry_config(&id).is_some());
    }

    #[test]
    fn test_manager_metrics() {
        let mut manager = ObservabilityManager::new();

        let metrics = MeshMetrics::new("service-1", "default");
        manager.add_metrics("service-1", metrics);

        assert_eq!(manager.metrics_count(), 1);
        assert!(manager.get_metrics("service-1").is_some());
    }

    #[test]
    fn test_manager_spans() {
        let mut manager = ObservabilityManager::new();

        let span1 = TracingSpan::new("trace-1", "span-1", "service", "op1");
        let span2 = TracingSpan::new("trace-1", "span-2", "service", "op2");

        manager.add_span("trace-1", span1);
        manager.add_span("trace-1", span2);

        assert_eq!(manager.trace_count(), 1);
        assert_eq!(manager.span_count(), 2);

        let trace = manager.get_trace("trace-1").unwrap();
        assert_eq!(trace.len(), 2);
    }

    #[test]
    fn test_manager_remove_telemetry_config() {
        let mut manager = ObservabilityManager::new();

        let config = TelemetryConfig::new("config", "default");
        let id = manager.add_telemetry_config(config);

        assert!(manager.remove_telemetry_config(&id));
        assert_eq!(manager.telemetry_config_count(), 0);
    }

    #[test]
    fn test_manager_high_error_services() {
        let mut manager = ObservabilityManager::new();

        let mut metrics1 = MeshMetrics::new("service-1", "default");
        metrics1.request_count = 100;
        metrics1.error_count = 10;

        let mut metrics2 = MeshMetrics::new("service-2", "default");
        metrics2.request_count = 100;
        metrics2.error_count = 1;

        manager.add_metrics("service-1", metrics1);
        manager.add_metrics("service-2", metrics2);

        let high_error = manager.high_error_services(5.0);
        assert_eq!(high_error.len(), 1);
    }

    #[test]
    fn test_manager_slow_services() {
        let mut manager = ObservabilityManager::new();

        let mut metrics1 = MeshMetrics::new("service-1", "default");
        metrics1.request_duration_ms = vec![100.0, 200.0, 300.0];

        let mut metrics2 = MeshMetrics::new("service-2", "default");
        metrics2.request_duration_ms = vec![10.0, 20.0, 30.0];

        manager.add_metrics("service-1", metrics1);
        manager.add_metrics("service-2", metrics2);

        let slow = manager.slow_services(100.0);
        assert_eq!(slow.len(), 1);
    }

    #[test]
    fn test_manager_get_metrics_mut() {
        let mut manager = ObservabilityManager::new();

        let metrics = MeshMetrics::new("service", "default");
        manager.add_metrics("service", metrics);

        if let Some(metrics_mut) = manager.get_metrics_mut("service") {
            metrics_mut.record_request(100.0, false);
        }

        let metrics = manager.get_metrics("service").unwrap();
        assert_eq!(metrics.request_count, 1);
    }

    #[test]
    fn test_empty_duration_percentiles() {
        let metrics = MeshMetrics::new("service", "default");

        assert_eq!(metrics.p95_duration_ms(), 0.0);
        assert_eq!(metrics.p99_duration_ms(), 0.0);
        assert_eq!(metrics.avg_duration_ms(), 0.0);
    }
}
