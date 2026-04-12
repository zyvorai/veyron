// Custom Metrics - User-defined metric collection and tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetricsManager {
    pub metrics: HashMap<String, CustomMetric>,
    pub config: CustomMetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetric {
    pub name: String,
    pub description: String,
    pub unit: String,
    pub metric_type: MetricType,
    pub values: Vec<MetricValue>,
    pub thresholds: Option<MetricThresholds>,
    pub labels: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType { Counter, Gauge, Histogram, Summary }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricThresholds {
    pub warning: f64,
    pub critical: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetricsConfig {
    pub max_values_per_metric: usize,
    pub collection_interval_secs: u64,
}

impl Default for CustomMetricsConfig {
    fn default() -> Self { Self { max_values_per_metric: 1000, collection_interval_secs: 30 } }
}

const MAX_SERIES: usize = 10_000;

impl CustomMetricsManager {
    pub fn new() -> Self { Self { metrics: HashMap::new(), config: CustomMetricsConfig::default() } }

    pub fn register(&mut self, name: &str, description: &str, unit: &str, metric_type: MetricType) {
        self.metrics.insert(name.to_string(), CustomMetric {
            name: name.to_string(), description: description.to_string(), unit: unit.to_string(),
            metric_type, values: Vec::new(), thresholds: None, labels: HashMap::new(), created_at: Utc::now(),
        });
    }

    pub fn record(&mut self, name: &str, value: f64, labels: HashMap<String, String>) {
        // Check cardinality: count distinct label combinations across all metrics
        let total_series: usize = self
            .metrics
            .values()
            .map(|m| {
                let mut seen = std::collections::HashSet::new();
                for v in &m.values {
                    let mut key: Vec<(&String, &String)> = v.labels.iter().collect();
                    key.sort();
                    seen.insert(format!("{:?}", key));
                }
                seen.len()
            })
            .sum();

        if total_series >= MAX_SERIES {
            // Reject recording to prevent cardinality explosion
            return;
        }

        if let Some(metric) = self.metrics.get_mut(name) {
            metric.values.push(MetricValue { timestamp: Utc::now(), value, labels });
            if metric.values.len() > self.config.max_values_per_metric {
                let drain_count = metric.values.len().min(100);
                metric.values.drain(0..drain_count);
            }
        }
    }

    pub fn set_thresholds(&mut self, name: &str, warning: f64, critical: f64) {
        if let Some(metric) = self.metrics.get_mut(name) {
            metric.thresholds = Some(MetricThresholds { warning, critical });
        }
    }

    pub fn get(&self, name: &str) -> Option<&CustomMetric> { self.metrics.get(name) }
    pub fn list(&self) -> Vec<&CustomMetric> { self.metrics.values().collect() }

    pub fn latest_value(&self, name: &str) -> Option<f64> {
        self.metrics.get(name).and_then(|m| m.values.last().map(|v| v.value))
    }

    pub fn check_thresholds(&self, name: &str) -> Option<ThresholdStatus> {
        let metric = self.metrics.get(name)?;
        let thresholds = metric.thresholds.as_ref()?;
        let value = metric.values.last()?.value;
        Some(if value >= thresholds.critical { ThresholdStatus::Critical }
        else if value >= thresholds.warning { ThresholdStatus::Warning }
        else { ThresholdStatus::Normal })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThresholdStatus { Normal, Warning, Critical }

impl Default for CustomMetricsManager {
    fn default() -> Self { Self::new() }
}
