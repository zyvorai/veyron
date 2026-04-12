// Resource Forecasting - Predictive modeling for CPU, memory, disk

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceForecaster {
    pub vm_forecasts: HashMap<String, VmForecast>,
    pub cluster_forecast: Option<ClusterForecast>,
    pub config: ForecastConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmForecast {
    pub vm_name: String,
    pub cpu_forecast: ResourcePrediction,
    pub memory_forecast: ResourcePrediction,
    pub disk_forecast: ResourcePrediction,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePrediction {
    pub current_value: f64,
    pub predicted_values: Vec<PredictionPoint>,
    pub trend: Trend,
    pub model: ForecastModel,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Trend { Increasing, Decreasing, Stable, Volatile }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForecastModel { LinearRegression, ExponentialSmoothing, MovingAverage, SimpleExtrapolation }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterForecast {
    pub total_cpu_forecast: ResourcePrediction,
    pub total_memory_forecast: ResourcePrediction,
    pub node_count_trend: Trend,
    pub capacity_exhaustion_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastConfig {
    pub horizon_hours: u64,
    pub model: ForecastModel,
    pub min_data_points: usize,
    pub confidence_level: f64,
}

impl Default for ForecastConfig {
    fn default() -> Self {
        Self { horizon_hours: 168, model: ForecastModel::LinearRegression, min_data_points: 10, confidence_level: 0.95 }
    }
}

impl ResourceForecaster {
    pub fn new() -> Self {
        Self { vm_forecasts: HashMap::new(), cluster_forecast: None, config: ForecastConfig::default() }
    }

    pub fn forecast_vm(&mut self, vm_name: &str, cpu_history: &[f64], mem_history: &[f64], disk_history: &[f64]) {
        let cpu_forecast = self.predict(cpu_history);
        let memory_forecast = self.predict(mem_history);
        let disk_forecast = self.predict(disk_history);

        self.vm_forecasts.insert(vm_name.to_string(), VmForecast {
            vm_name: vm_name.to_string(), cpu_forecast, memory_forecast, disk_forecast, generated_at: Utc::now(),
        });
    }

    fn predict(&self, history: &[f64]) -> ResourcePrediction {
        if history.len() < 2 {
            return ResourcePrediction {
                current_value: history.last().copied().unwrap_or(0.0),
                predicted_values: Vec::new(), trend: Trend::Stable,
                model: self.config.model.clone(), confidence: 0.0,
            };
        }

        let current = history.last().copied().unwrap_or(0.0);
        let n = history.len() as f64;
        let sum_x: f64 = (0..history.len()).map(|i| i as f64).sum();
        let sum_y: f64 = history.iter().sum();
        let sum_xy: f64 = history.iter().enumerate().map(|(i, v)| i as f64 * v).sum();
        let sum_x2: f64 = (0..history.len()).map(|i| (i as f64).powi(2)).sum();

        let denominator = n * sum_x2 - sum_x.powi(2);
        if denominator.abs() < f64::EPSILON {
            // Degenerate case: all x values are the same; return flat prediction
            return ResourcePrediction {
                current_value: current,
                predicted_values: Vec::new(),
                trend: Trend::Stable,
                model: self.config.model.clone(),
                confidence: 0.0,
            };
        }
        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        let intercept = (sum_y - slope * sum_x) / n;

        let trend = if slope > 0.01 { Trend::Increasing }
        else if slope < -0.01 { Trend::Decreasing }
        else { Trend::Stable };

        let points: Vec<PredictionPoint> = (1..=24).map(|h| {
            let x = history.len() as f64 + h as f64;
            let val = (intercept + slope * x).max(0.0);
            let margin = val * (1.0 - self.config.confidence_level) * 2.0;
            PredictionPoint {
                timestamp: Utc::now() + chrono::TimeDelta::hours(h),
                value: val, lower_bound: (val - margin).max(0.0), upper_bound: val + margin,
            }
        }).collect();

        ResourcePrediction { current_value: current, predicted_values: points, trend, model: self.config.model.clone(), confidence: 0.85 }
    }

    pub fn get_vm_forecast(&self, vm_name: &str) -> Option<&VmForecast> { self.vm_forecasts.get(vm_name) }
}

impl Default for ResourceForecaster {
    fn default() -> Self { Self::new() }
}
