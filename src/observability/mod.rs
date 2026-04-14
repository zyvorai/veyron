// Observability & Analytics - Comprehensive monitoring and insights

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod alerts;
pub mod anomaly;
pub mod custom_metrics;
pub mod forecasting;
pub mod insights;
pub mod leak_detector;
pub mod log_aggregation;
pub mod logs;
pub mod metrics;
pub mod profiler;
pub mod tracing;

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub log_retention_days: u32,
    pub metric_retention_days: u32,
    pub sampling_interval_seconds: u64,
    pub alert_channels: Vec<AlertChannel>,
}

impl ObservabilityConfig {
    pub fn new() -> Self {
        Self {
            log_retention_days: 30,
            metric_retention_days: 90,
            sampling_interval_seconds: 60,
            alert_channels: Vec::new(),
        }
    }

    pub fn with_log_retention(mut self, days: u32) -> Self {
        self.log_retention_days = days;
        self
    }

    pub fn with_metric_retention(mut self, days: u32) -> Self {
        self.metric_retention_days = days;
        self
    }

    pub fn add_alert_channel(mut self, channel: AlertChannel) -> Self {
        self.alert_channels.push(channel);
        self
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Alert channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertChannel {
    Email {
        recipients: Vec<String>,
    },
    Slack {
        #[serde(skip_serializing)]
        webhook_url: String,
    },
    PagerDuty {
        #[serde(skip_serializing)]
        integration_key: String,
    },
    Webhook {
        url: String,
    },
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

impl DataPoint {
    pub fn new(value: f64) -> Self {
        Self {
            timestamp: Utc::now(),
            value,
            labels: HashMap::new(),
        }
    }

    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

/// Time series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    pub metric_name: String,
    pub data_points: Vec<DataPoint>,
}

impl TimeSeries {
    pub fn new(metric_name: impl Into<String>) -> Self {
        Self {
            metric_name: metric_name.into(),
            data_points: Vec::new(),
        }
    }

    pub fn add_point(&mut self, point: DataPoint) {
        self.data_points.push(point);
    }

    pub fn latest_value(&self) -> Option<f64> {
        self.data_points.last().map(|p| p.value)
    }

    pub fn average(&self) -> f64 {
        if self.data_points.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.data_points.iter().map(|p| p.value).sum();
        sum / self.data_points.len() as f64
    }

    pub fn max(&self) -> Option<f64> {
        self.data_points
            .iter()
            .map(|p| p.value)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn min(&self) -> Option<f64> {
        self.data_points
            .iter()
            .map(|p| p.value)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn percentile(&self, p: f64) -> Option<f64> {
        if self.data_points.is_empty() {
            return None;
        }

        let mut values: Vec<f64> = self.data_points.iter().map(|dp| dp.value).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p_clamped = p.clamp(0.0, 100.0);
        let index = ((p_clamped / 100.0) * (values.len() - 1) as f64) as usize;
        Some(values[index.min(values.len() - 1)])
    }

    /// Get data points within a time range
    pub fn range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&DataPoint> {
        self.data_points
            .iter()
            .filter(|p| p.timestamp >= start && p.timestamp <= end)
            .collect()
    }

    pub fn point_count(&self) -> usize {
        self.data_points.len()
    }
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
            HealthStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub component: String,
    pub status: HealthStatus,
    pub message: String,
    pub checked_at: DateTime<Utc>,
    pub response_time_ms: Option<u64>,
}

impl HealthCheck {
    pub fn new(component: impl Into<String>, status: HealthStatus) -> Self {
        Self {
            component: component.into(),
            status,
            message: String::new(),
            checked_at: Utc::now(),
            response_time_ms: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn with_response_time(mut self, ms: u64) -> Self {
        self.response_time_ms = Some(ms);
        self
    }

    pub fn is_healthy(&self) -> bool {
        self.status == HealthStatus::Healthy
    }
}

/// System health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub evaluated_at: DateTime<Utc>,
}

impl SystemHealth {
    pub fn new() -> Self {
        Self {
            overall_status: HealthStatus::Unknown,
            checks: Vec::new(),
            evaluated_at: Utc::now(),
        }
    }

    pub fn add_check(&mut self, check: HealthCheck) {
        self.checks.push(check);
        self.update_overall_status();
    }

    fn update_overall_status(&mut self) {
        if self.checks.is_empty() {
            self.overall_status = HealthStatus::Unknown;
            return;
        }

        let has_unhealthy = self
            .checks
            .iter()
            .any(|c| c.status == HealthStatus::Unhealthy);
        let has_degraded = self
            .checks
            .iter()
            .any(|c| c.status == HealthStatus::Degraded);

        self.overall_status = if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
    }

    pub fn healthy_count(&self) -> usize {
        self.checks.iter().filter(|c| c.is_healthy()).count()
    }

    pub fn unhealthy_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == HealthStatus::Unhealthy)
            .count()
    }
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self::new()
    }
}

/// Trace span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub attributes: HashMap<String, String>,
}

impl Span {
    pub fn new(
        span_id: impl Into<String>,
        trace_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            span_id: span_id.into(),
            trace_id: trace_id.into(),
            parent_span_id: None,
            name: name.into(),
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            attributes: HashMap::new(),
        }
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_id.into());
        self
    }

    pub fn add_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn complete(&mut self) {
        let completed = Utc::now();
        self.completed_at = Some(completed);
        let duration = completed.signed_duration_since(self.started_at);
        // Guard against clock skew producing negative durations
        if duration.num_milliseconds() >= 0 {
            self.duration_ms = Some(duration.num_milliseconds() as u64);
        } else {
            self.duration_ms = Some(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta as Duration;

    #[test]
    fn test_observability_config() {
        let config = ObservabilityConfig::new()
            .with_log_retention(60)
            .with_metric_retention(180)
            .add_alert_channel(AlertChannel::Slack {
                webhook_url: "https://hooks.slack.com/test".to_string(),
            });

        assert_eq!(config.log_retention_days, 60);
        assert_eq!(config.metric_retention_days, 180);
        assert_eq!(config.alert_channels.len(), 1);
    }

    #[test]
    fn test_data_point() {
        let point = DataPoint::new(42.5)
            .with_label("vm", "test-vm")
            .with_label("namespace", "default");

        assert_eq!(point.value, 42.5);
        assert_eq!(point.labels.get("vm"), Some(&"test-vm".to_string()));
    }

    #[test]
    fn test_time_series() {
        let mut ts = TimeSeries::new("cpu_usage");

        ts.add_point(DataPoint::new(50.0));
        ts.add_point(DataPoint::new(60.0));
        ts.add_point(DataPoint::new(70.0));

        assert_eq!(ts.point_count(), 3);
        assert_eq!(ts.latest_value(), Some(70.0));
        assert_eq!(ts.average(), 60.0);
        assert_eq!(ts.max(), Some(70.0));
        assert_eq!(ts.min(), Some(50.0));
    }

    #[test]
    fn test_percentile() {
        let mut ts = TimeSeries::new("latency");

        for i in 1..=100 {
            ts.add_point(DataPoint::new(i as f64));
        }

        assert_eq!(ts.percentile(50.0), Some(50.0));
        assert_eq!(ts.percentile(95.0), Some(95.0));
        assert_eq!(ts.percentile(99.0), Some(99.0));
    }

    #[test]
    fn test_time_series_range() {
        let mut ts = TimeSeries::new("metric");

        let now = Utc::now();
        ts.add_point(DataPoint::new(1.0).with_timestamp(now - Duration::hours(2)));
        ts.add_point(DataPoint::new(2.0).with_timestamp(now - Duration::hours(1)));
        ts.add_point(DataPoint::new(3.0).with_timestamp(now));

        let range = ts.range(now - Duration::hours(90), now - Duration::minutes(30));
        assert_eq!(range.len(), 2);
    }

    #[test]
    fn test_health_check() {
        let check = HealthCheck::new("database", HealthStatus::Healthy)
            .with_message("Connection successful")
            .with_response_time(15);

        assert!(check.is_healthy());
        assert_eq!(check.response_time_ms, Some(15));
    }

    #[test]
    fn test_system_health() {
        let mut health = SystemHealth::new();

        health.add_check(HealthCheck::new("api", HealthStatus::Healthy));
        health.add_check(HealthCheck::new("database", HealthStatus::Healthy));
        health.add_check(HealthCheck::new("cache", HealthStatus::Degraded));

        assert_eq!(health.overall_status, HealthStatus::Degraded);
        assert_eq!(health.healthy_count(), 2);
        assert_eq!(health.unhealthy_count(), 0);
    }

    #[test]
    fn test_system_health_unhealthy() {
        let mut health = SystemHealth::new();

        health.add_check(HealthCheck::new("api", HealthStatus::Healthy));
        health.add_check(HealthCheck::new("database", HealthStatus::Unhealthy));

        assert_eq!(health.overall_status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_span() {
        let mut span = Span::new("span-1", "trace-1", "http.request")
            .with_parent("span-0")
            .add_attribute("http.method", "GET")
            .add_attribute("http.url", "/api/vms");

        assert_eq!(span.name, "http.request");
        assert_eq!(span.parent_span_id, Some("span-0".to_string()));
        assert!(span.duration_ms.is_none());

        span.complete();
        assert!(span.completed_at.is_some());
        assert!(span.duration_ms.is_some());
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "Healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "Degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "Unhealthy");
    }
}
