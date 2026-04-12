// Anomaly Detection - Statistical anomaly detection for VM metrics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetector {
    pub anomalies: Vec<Anomaly>,
    pub config: AnomalyConfig,
    pub baselines: std::collections::HashMap<String, Baseline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub id: String,
    pub vm_name: String,
    pub metric_name: String,
    pub severity: AnomalySeverity,
    pub detected_at: DateTime<Utc>,
    pub value: f64,
    pub expected_range: (f64, f64),
    pub deviation: f64,
    pub description: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnomalySeverity { Critical, High, Medium, Low }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub metric_name: String,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub sample_count: usize,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyConfig {
    pub enabled: bool,
    pub z_score_threshold: f64,
    pub min_samples: usize,
    pub detection_interval_secs: u64,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self { enabled: true, z_score_threshold: 3.0, min_samples: 30, detection_interval_secs: 60 }
    }
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self { anomalies: Vec::new(), config: AnomalyConfig::default(), baselines: std::collections::HashMap::new() }
    }

    pub fn update_baseline(&mut self, key: &str, metric: &str, values: &[f64]) {
        if values.len() < self.config.min_samples { return; }
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        self.baselines.insert(key.to_string(), Baseline {
            metric_name: metric.to_string(), mean, std_dev, min, max,
            sample_count: values.len(), updated_at: Utc::now(),
        });
    }

    pub fn check(&mut self, vm_name: &str, metric: &str, value: f64) -> Option<Anomaly> {
        let key = format!("{}:{}", vm_name, metric);
        let baseline = self.baselines.get(&key)?;
        if baseline.std_dev == 0.0 { return None; }

        let z_score = ((value - baseline.mean) / baseline.std_dev).abs();
        if z_score < self.config.z_score_threshold { return None; }

        let severity = if z_score > 5.0 { AnomalySeverity::Critical }
        else if z_score > 4.0 { AnomalySeverity::High }
        else if z_score > 3.5 { AnomalySeverity::Medium }
        else { AnomalySeverity::Low };

        let anomaly = Anomaly {
            id: format!("anomaly-{}", Utc::now().timestamp_micros()),
            vm_name: vm_name.to_string(), metric_name: metric.to_string(),
            severity, detected_at: Utc::now(), value,
            expected_range: (baseline.mean - baseline.std_dev * 2.0, baseline.mean + baseline.std_dev * 2.0),
            deviation: z_score,
            description: format!("{} value {:.2} deviates {:.1} standard deviations from mean {:.2}", metric, value, z_score, baseline.mean),
            resolved: false,
        };
        self.anomalies.push(anomaly.clone());
        // Cap anomaly history
        if self.anomalies.len() > 10000 {
            log::warn!("Anomaly history exceeded 10,000 entries, dropping oldest 1,000");
            let drain_count = self.anomalies.len().min(1000);
            self.anomalies.drain(0..drain_count);
        }
        Some(anomaly)
    }

    pub fn active_anomalies(&self) -> Vec<&Anomaly> { self.anomalies.iter().filter(|a| !a.resolved).collect() }
    pub fn resolve(&mut self, id: &str) { if let Some(a) = self.anomalies.iter_mut().find(|a| a.id == id) { a.resolved = true; } }
}

impl Default for AnomalyDetector {
    fn default() -> Self { Self::new() }
}
