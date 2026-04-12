use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ResourceType;

/// Forecast method
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForecastMethod {
    Linear,
    Exponential,
    MovingAverage,
    SeasonalTrend,
}

/// Resource forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceForecast {
    pub id: String,
    pub resource_type: ResourceType,
    pub method: ForecastMethod,
    pub current_usage: f64,
    pub predicted_usage: HashMap<u32, f64>, // days -> predicted usage
    pub confidence_level: f64,
    pub growth_rate_percent: f64,
    pub exhaustion_date: Option<DateTime<Utc>>,
    pub generated_at: DateTime<Utc>,
}

impl ResourceForecast {
    pub fn new(resource_type: ResourceType, current: f64, method: ForecastMethod) -> Self {
        let id = format!(
            "forecast-{}-{}",
            resource_type,
            Utc::now().timestamp_micros()
        );

        Self {
            id,
            resource_type,
            method,
            current_usage: current,
            predicted_usage: HashMap::new(),
            confidence_level: 0.85,
            growth_rate_percent: 0.0,
            exhaustion_date: None,
            generated_at: Utc::now(),
        }
    }

    pub fn add_prediction(&mut self, days: u32, usage: f64) {
        self.predicted_usage.insert(days, usage);
    }

    pub fn get_prediction(&self, days: u32) -> Option<f64> {
        self.predicted_usage.get(&days).copied()
    }

    pub fn set_growth_rate(&mut self, rate: f64) {
        self.growth_rate_percent = rate;
    }

    pub fn set_exhaustion_date(&mut self, date: DateTime<Utc>) {
        self.exhaustion_date = Some(date);
    }

    pub fn days_until_exhaustion(&self) -> Option<i64> {
        self.exhaustion_date
            .map(|date| (date - Utc::now()).num_days())
    }

    pub fn is_critical(&self) -> bool {
        if let Some(days) = self.days_until_exhaustion() {
            days <= 30
        } else {
            false
        }
    }
}

/// Forecast manager
pub struct ForecastManager {
    forecasts: HashMap<String, ResourceForecast>,
}

impl ForecastManager {
    pub fn new() -> Self {
        Self {
            forecasts: HashMap::new(),
        }
    }

    pub fn add_forecast(&mut self, forecast: ResourceForecast) -> String {
        let id = forecast.id.clone();
        self.forecasts.insert(id.clone(), forecast);
        id
    }

    pub fn get_forecast(&self, id: &str) -> Option<&ResourceForecast> {
        self.forecasts.get(id)
    }

    pub fn forecast_count(&self) -> usize {
        self.forecasts.len()
    }

    pub fn by_resource_type(&self, resource_type: &ResourceType) -> Vec<&ResourceForecast> {
        self.forecasts
            .values()
            .filter(|f| &f.resource_type == resource_type)
            .collect()
    }

    pub fn critical_forecasts(&self) -> Vec<&ResourceForecast> {
        self.forecasts
            .values()
            .filter(|f| f.is_critical())
            .collect()
    }

    pub fn high_growth_forecasts(&self, threshold: f64) -> Vec<&ResourceForecast> {
        self.forecasts
            .values()
            .filter(|f| f.growth_rate_percent > threshold)
            .collect()
    }
}

impl Default for ForecastManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_forecast() {
        let forecast = ResourceForecast::new(ResourceType::CPU, 75.0, ForecastMethod::Linear);

        assert_eq!(forecast.resource_type, ResourceType::CPU);
        assert_eq!(forecast.current_usage, 75.0);
        assert_eq!(forecast.method, ForecastMethod::Linear);
        assert_eq!(forecast.confidence_level, 0.85);
    }

    #[test]
    fn test_forecast_add_prediction() {
        let mut forecast =
            ResourceForecast::new(ResourceType::Memory, 500.0, ForecastMethod::Exponential);

        forecast.add_prediction(7, 550.0);
        forecast.add_prediction(14, 600.0);
        forecast.add_prediction(30, 700.0);

        assert_eq!(forecast.predicted_usage.len(), 3);
        assert_eq!(forecast.get_prediction(7), Some(550.0));
    }

    #[test]
    fn test_forecast_growth_rate() {
        let mut forecast =
            ResourceForecast::new(ResourceType::Storage, 1000.0, ForecastMethod::Linear);

        forecast.set_growth_rate(5.5);
        assert_eq!(forecast.growth_rate_percent, 5.5);
    }

    #[test]
    fn test_forecast_exhaustion() {
        let mut forecast = ResourceForecast::new(ResourceType::CPU, 85.0, ForecastMethod::Linear);

        let exhaustion = Utc::now() + chrono::TimeDelta::days(15);
        forecast.set_exhaustion_date(exhaustion);

        assert!(forecast.exhaustion_date.is_some());
        assert!(forecast.is_critical()); // Less than 30 days

        let days = forecast.days_until_exhaustion().unwrap();
        assert!(days <= 15);
    }

    #[test]
    fn test_forecast_not_critical() {
        let mut forecast =
            ResourceForecast::new(ResourceType::Memory, 60.0, ForecastMethod::SeasonalTrend);

        let exhaustion = Utc::now() + chrono::TimeDelta::days(90);
        forecast.set_exhaustion_date(exhaustion);

        assert!(!forecast.is_critical()); // More than 30 days
    }

    #[test]
    fn test_forecast_manager() {
        let mut manager = ForecastManager::new();

        let forecast = ResourceForecast::new(ResourceType::CPU, 75.0, ForecastMethod::Linear);
        let id = manager.add_forecast(forecast);

        assert_eq!(manager.forecast_count(), 1);
        assert!(manager.get_forecast(&id).is_some());
    }

    #[test]
    fn test_manager_by_resource_type() {
        let mut manager = ForecastManager::new();

        manager.add_forecast(ResourceForecast::new(
            ResourceType::CPU,
            75.0,
            ForecastMethod::Linear,
        ));
        manager.add_forecast(ResourceForecast::new(
            ResourceType::Memory,
            500.0,
            ForecastMethod::Exponential,
        ));
        manager.add_forecast(ResourceForecast::new(
            ResourceType::CPU,
            80.0,
            ForecastMethod::MovingAverage,
        ));

        let cpu_forecasts = manager.by_resource_type(&ResourceType::CPU);
        assert_eq!(cpu_forecasts.len(), 2);
    }

    #[test]
    fn test_manager_critical_forecasts() {
        let mut manager = ForecastManager::new();

        let mut forecast1 = ResourceForecast::new(ResourceType::CPU, 90.0, ForecastMethod::Linear);
        forecast1.set_exhaustion_date(Utc::now() + chrono::TimeDelta::days(20));

        let mut forecast2 =
            ResourceForecast::new(ResourceType::Memory, 70.0, ForecastMethod::Exponential);
        forecast2.set_exhaustion_date(Utc::now() + chrono::TimeDelta::days(60));

        manager.add_forecast(forecast1);
        manager.add_forecast(forecast2);

        let critical = manager.critical_forecasts();
        assert_eq!(critical.len(), 1);
    }

    #[test]
    fn test_manager_high_growth_forecasts() {
        let mut manager = ForecastManager::new();

        let mut forecast1 = ResourceForecast::new(ResourceType::CPU, 75.0, ForecastMethod::Linear);
        forecast1.set_growth_rate(8.5);

        let mut forecast2 =
            ResourceForecast::new(ResourceType::Memory, 500.0, ForecastMethod::Exponential);
        forecast2.set_growth_rate(3.2);

        let mut forecast3 =
            ResourceForecast::new(ResourceType::Storage, 1000.0, ForecastMethod::SeasonalTrend);
        forecast3.set_growth_rate(12.0);

        manager.add_forecast(forecast1);
        manager.add_forecast(forecast2);
        manager.add_forecast(forecast3);

        let high_growth = manager.high_growth_forecasts(5.0);
        assert_eq!(high_growth.len(), 2);
    }

    #[test]
    fn test_forecast_method_equality() {
        assert_eq!(ForecastMethod::Linear, ForecastMethod::Linear);
        assert_ne!(ForecastMethod::Linear, ForecastMethod::Exponential);
    }
}
