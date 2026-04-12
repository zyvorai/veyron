use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ResourceType;

/// Usage pattern
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsagePattern {
    Steady,
    Increasing,
    Decreasing,
    Spiky,
    Seasonal,
}

/// Resource analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAnalysis {
    pub id: String,
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub average_utilization: f64,
    pub peak_utilization: f64,
    pub min_utilization: f64,
    pub pattern: UsagePattern,
    pub waste_percent: f64,
    pub analysis_period_days: u32,
    pub analyzed_at: DateTime<Utc>,
}

impl ResourceAnalysis {
    pub fn new(
        resource_type: ResourceType,
        resource_id: impl Into<String>,
        period_days: u32,
    ) -> Self {
        let res_id = resource_id.into();
        let id = format!("analysis-{}-{}", res_id, Utc::now().timestamp());

        Self {
            id,
            resource_type,
            resource_id: res_id,
            average_utilization: 0.0,
            peak_utilization: 0.0,
            min_utilization: 0.0,
            pattern: UsagePattern::Steady,
            waste_percent: 0.0,
            analysis_period_days: period_days,
            analyzed_at: Utc::now(),
        }
    }

    pub fn set_utilization(&mut self, avg: f64, peak: f64, min: f64) {
        self.average_utilization = avg;
        self.peak_utilization = peak;
        self.min_utilization = min;
        self.calculate_waste();
    }

    pub fn set_pattern(&mut self, pattern: UsagePattern) {
        self.pattern = pattern;
    }

    fn calculate_waste(&mut self) {
        if self.peak_utilization > 0.0 {
            self.waste_percent = ((self.peak_utilization - self.average_utilization)
                / self.peak_utilization)
                * 100.0;
        }
    }

    pub fn is_underutilized(&self) -> bool {
        self.average_utilization < 30.0
    }

    pub fn is_overutilized(&self) -> bool {
        self.average_utilization > 80.0
    }

    pub fn is_wasteful(&self) -> bool {
        self.waste_percent > 40.0
    }
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub resource_id: String,
    pub response_time_ms: f64,
    pub throughput_ops: f64,
    pub error_rate_percent: f64,
    pub saturation_percent: f64,
    pub timestamp: DateTime<Utc>,
}

impl PerformanceMetrics {
    pub fn new(resource_id: impl Into<String>) -> Self {
        Self {
            resource_id: resource_id.into(),
            response_time_ms: 0.0,
            throughput_ops: 0.0,
            error_rate_percent: 0.0,
            saturation_percent: 0.0,
            timestamp: Utc::now(),
        }
    }

    pub fn with_response_time(mut self, ms: f64) -> Self {
        self.response_time_ms = ms;
        self
    }

    pub fn with_throughput(mut self, ops: f64) -> Self {
        self.throughput_ops = ops;
        self
    }

    pub fn with_error_rate(mut self, percent: f64) -> Self {
        self.error_rate_percent = percent;
        self
    }

    pub fn with_saturation(mut self, percent: f64) -> Self {
        self.saturation_percent = percent;
        self
    }

    pub fn is_healthy(&self) -> bool {
        self.error_rate_percent < 1.0 && self.saturation_percent < 80.0
    }

    pub fn is_degraded(&self) -> bool {
        self.error_rate_percent >= 1.0 || self.saturation_percent >= 80.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_analysis() {
        let analysis = ResourceAnalysis::new(ResourceType::CPU, "vm-123", 30);

        assert_eq!(analysis.resource_type, ResourceType::CPU);
        assert_eq!(analysis.resource_id, "vm-123");
        assert_eq!(analysis.analysis_period_days, 30);
        assert_eq!(analysis.pattern, UsagePattern::Steady);
    }

    #[test]
    fn test_analysis_set_utilization() {
        let mut analysis = ResourceAnalysis::new(ResourceType::Memory, "vm-456", 30);

        analysis.set_utilization(45.0, 80.0, 20.0);

        assert_eq!(analysis.average_utilization, 45.0);
        assert_eq!(analysis.peak_utilization, 80.0);
        assert_eq!(analysis.min_utilization, 20.0);
        assert!(analysis.waste_percent > 0.0);
    }

    #[test]
    fn test_analysis_is_underutilized() {
        let mut analysis = ResourceAnalysis::new(ResourceType::CPU, "vm-1", 30);

        analysis.set_utilization(25.0, 50.0, 10.0);
        assert!(analysis.is_underutilized());

        analysis.set_utilization(45.0, 80.0, 20.0);
        assert!(!analysis.is_underutilized());
    }

    #[test]
    fn test_analysis_is_overutilized() {
        let mut analysis = ResourceAnalysis::new(ResourceType::CPU, "vm-1", 30);

        analysis.set_utilization(85.0, 95.0, 70.0);
        assert!(analysis.is_overutilized());

        analysis.set_utilization(65.0, 90.0, 40.0);
        assert!(!analysis.is_overutilized());
    }

    #[test]
    fn test_analysis_is_wasteful() {
        let mut analysis = ResourceAnalysis::new(ResourceType::Storage, "vol-1", 30);

        analysis.set_utilization(30.0, 100.0, 10.0);
        assert!(analysis.is_wasteful()); // 70% waste

        analysis.set_utilization(70.0, 100.0, 40.0);
        assert!(!analysis.is_wasteful()); // 30% waste
    }

    #[test]
    fn test_analysis_set_pattern() {
        let mut analysis = ResourceAnalysis::new(ResourceType::Network, "net-1", 30);

        assert_eq!(analysis.pattern, UsagePattern::Steady);

        analysis.set_pattern(UsagePattern::Spiky);
        assert_eq!(analysis.pattern, UsagePattern::Spiky);
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new("service-1");

        assert_eq!(metrics.resource_id, "service-1");
        assert_eq!(metrics.response_time_ms, 0.0);
        assert_eq!(metrics.error_rate_percent, 0.0);
    }

    #[test]
    fn test_metrics_builder() {
        let metrics = PerformanceMetrics::new("service-2")
            .with_response_time(150.0)
            .with_throughput(1000.0)
            .with_error_rate(0.5)
            .with_saturation(60.0);

        assert_eq!(metrics.response_time_ms, 150.0);
        assert_eq!(metrics.throughput_ops, 1000.0);
        assert_eq!(metrics.error_rate_percent, 0.5);
        assert_eq!(metrics.saturation_percent, 60.0);
    }

    #[test]
    fn test_metrics_is_healthy() {
        let metrics = PerformanceMetrics::new("svc-1")
            .with_error_rate(0.5)
            .with_saturation(60.0);

        assert!(metrics.is_healthy());
        assert!(!metrics.is_degraded());
    }

    #[test]
    fn test_metrics_is_degraded() {
        let metrics1 = PerformanceMetrics::new("svc-1")
            .with_error_rate(2.0)
            .with_saturation(60.0);

        let metrics2 = PerformanceMetrics::new("svc-2")
            .with_error_rate(0.5)
            .with_saturation(85.0);

        assert!(metrics1.is_degraded());
        assert!(metrics2.is_degraded());
    }

    #[test]
    fn test_usage_pattern_equality() {
        assert_eq!(UsagePattern::Steady, UsagePattern::Steady);
        assert_ne!(UsagePattern::Steady, UsagePattern::Spiky);
    }
}
