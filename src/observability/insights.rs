// Insights - Operational insights and recommendations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Insight type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InsightType {
    Performance,
    Cost,
    Security,
    Availability,
    Capacity,
}

/// Insight severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum InsightSeverity {
    Low,
    Medium,
    High,
}

/// Insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: String,
    pub insight_type: InsightType,
    pub severity: InsightSeverity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    pub impact: String,
    pub affected_resources: Vec<String>,
    pub metrics: HashMap<String, f64>,
    pub created_at: DateTime<Utc>,
}

impl Insight {
    pub fn new(
        insight_type: InsightType,
        severity: InsightSeverity,
        title: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("insight-{}", Utc::now().timestamp_millis()),
            insight_type,
            severity,
            title: title.into(),
            description: String::new(),
            recommendation: String::new(),
            impact: String::new(),
            affected_resources: Vec::new(),
            metrics: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_recommendation(mut self, rec: impl Into<String>) -> Self {
        self.recommendation = rec.into();
        self
    }

    pub fn with_impact(mut self, impact: impl Into<String>) -> Self {
        self.impact = impact.into();
        self
    }

    pub fn add_resource(mut self, resource: impl Into<String>) -> Self {
        self.affected_resources.push(resource.into());
        self
    }

    pub fn add_metric(mut self, key: impl Into<String>, value: f64) -> Self {
        self.metrics.insert(key.into(), value);
        self
    }
}

/// Insight analyzer
pub struct InsightAnalyzer;

impl InsightAnalyzer {
    /// Analyze resource utilization and generate insights
    pub fn analyze_resource_utilization(
        cpu_usage: f64,
        memory_usage: f64,
        vm_name: &str,
    ) -> Vec<Insight> {
        let mut insights = Vec::new();

        // High CPU usage
        if cpu_usage > 90.0 {
            insights.push(
                Insight::new(
                    InsightType::Performance,
                    InsightSeverity::High,
                    "High CPU Usage Detected",
                )
                .with_description(format!(
                    "VM {} is experiencing high CPU usage at {:.1}%",
                    vm_name, cpu_usage
                ))
                .with_recommendation("Consider scaling up CPU resources or optimizing workload")
                .with_impact("May cause performance degradation and increased latency")
                .add_resource(vm_name.to_string())
                .add_metric("cpu_usage", cpu_usage),
            );
        }

        // High memory usage
        if memory_usage > 85.0 {
            insights.push(
                Insight::new(
                    InsightType::Performance,
                    InsightSeverity::High,
                    "High Memory Usage Detected",
                )
                .with_description(format!(
                    "VM {} is using {:.1}% of available memory",
                    vm_name, memory_usage
                ))
                .with_recommendation(
                    "Increase memory allocation or review memory-intensive processes",
                )
                .with_impact("Risk of OOM errors and application crashes")
                .add_resource(vm_name.to_string())
                .add_metric("memory_usage", memory_usage),
            );
        }

        // Low utilization
        if cpu_usage < 20.0 && memory_usage < 30.0 {
            insights.push(
                Insight::new(
                    InsightType::Cost,
                    InsightSeverity::Medium,
                    "Low Resource Utilization",
                )
                .with_description(format!(
                    "VM {} is underutilized (CPU: {:.1}%, Memory: {:.1}%)",
                    vm_name, cpu_usage, memory_usage
                ))
                .with_recommendation("Consider downsizing VM or consolidating workloads")
                .with_impact("Potential cost savings of 30-50%")
                .add_resource(vm_name.to_string())
                .add_metric("cpu_usage", cpu_usage)
                .add_metric("memory_usage", memory_usage),
            );
        }

        insights
    }

    /// Analyze availability patterns
    pub fn analyze_availability(uptime_percent: f64, vm_name: &str) -> Vec<Insight> {
        let mut insights = Vec::new();

        if uptime_percent < 99.0 {
            let severity = if uptime_percent < 95.0 {
                InsightSeverity::High
            } else {
                InsightSeverity::Medium
            };

            insights.push(
                Insight::new(
                    InsightType::Availability,
                    severity,
                    "Low Availability Detected",
                )
                .with_description(format!("VM {} has {:.2}% uptime", vm_name, uptime_percent))
                .with_recommendation("Implement high availability with redundancy and failover")
                .with_impact("Service disruptions affecting user experience")
                .add_resource(vm_name.to_string())
                .add_metric("uptime_percent", uptime_percent),
            );
        }

        insights
    }

    /// Analyze cost trends
    pub fn analyze_cost_trends(
        current_cost: f64,
        previous_cost: f64,
        vm_name: &str,
    ) -> Vec<Insight> {
        let mut insights = Vec::new();

        if previous_cost > 0.0 {
            let increase_percent = ((current_cost - previous_cost) / previous_cost) * 100.0;

            if increase_percent > 20.0 {
                insights.push(
                    Insight::new(
                        InsightType::Cost,
                        InsightSeverity::Medium,
                        "Significant Cost Increase",
                    )
                    .with_description(format!(
                        "VM {} costs increased by {:.1}% (${:.2} -> ${:.2})",
                        vm_name, increase_percent, previous_cost, current_cost
                    ))
                    .with_recommendation("Review recent configuration changes and usage patterns")
                    .with_impact(format!(
                        "Additional ${:.2} per month",
                        current_cost - previous_cost
                    ))
                    .add_resource(vm_name.to_string())
                    .add_metric("cost_increase_percent", increase_percent)
                    .add_metric("current_cost", current_cost),
                );
            }
        }

        insights
    }

    /// Analyze capacity trends
    pub fn analyze_capacity(current_usage: f64, capacity: f64, growth_rate: f64) -> Vec<Insight> {
        let mut insights = Vec::new();

        if capacity == 0.0 {
            return insights;
        }
        let usage_percent = (current_usage / capacity) * 100.0;

        if usage_percent > 80.0 {
            let days_to_full = if growth_rate > 0.0 {
                ((capacity - current_usage) / growth_rate).ceil()
            } else {
                f64::INFINITY
            };

            let severity = if days_to_full < 7.0 {
                InsightSeverity::High
            } else if days_to_full < 30.0 {
                InsightSeverity::Medium
            } else {
                InsightSeverity::Low
            };

            insights.push(
                Insight::new(
                    InsightType::Capacity,
                    severity,
                    "Capacity Approaching Limit",
                )
                .with_description(format!(
                    "Current capacity usage at {:.1}% ({:.0}/{:.0})",
                    usage_percent, current_usage, capacity
                ))
                .with_recommendation("Plan capacity expansion or optimize resource usage")
                .with_impact(format!(
                    "Estimated {:.0} days until capacity limit",
                    days_to_full
                ))
                .add_metric("usage_percent", usage_percent)
                .add_metric("days_to_full", days_to_full),
            );
        }

        insights
    }
}

/// Trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    pub metric_name: String,
    pub direction: TrendDirection,
    pub change_percent: f64,
    pub current_value: f64,
    pub previous_value: f64,
    pub analyzed_at: DateTime<Utc>,
}

impl Trend {
    pub fn new(metric_name: impl Into<String>, current: f64, previous: f64) -> Self {
        let change = current - previous;
        let change_percent = if previous != 0.0 {
            (change / previous) * 100.0
        } else {
            0.0
        };

        let direction = if change > 0.0 {
            TrendDirection::Increasing
        } else if change < 0.0 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        Self {
            metric_name: metric_name.into(),
            direction,
            change_percent,
            current_value: current,
            previous_value: previous,
            analyzed_at: Utc::now(),
        }
    }

    pub fn is_significant(&self, threshold: f64) -> bool {
        self.change_percent.abs() > threshold
    }
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// Anomaly detection
pub struct AnomalyDetector;

impl AnomalyDetector {
    /// Detect anomalies using simple statistical method
    pub fn detect(values: &[f64], threshold_stddev: f64) -> Vec<usize> {
        if values.len() < 3 {
            return Vec::new();
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let stddev = variance.sqrt();

        values
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| {
                if (v - mean).abs() > threshold_stddev * stddev {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Detect anomalies with timestamps
    pub fn detect_with_threshold(values: &[f64], threshold: f64) -> Vec<usize> {
        values
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| if v > threshold { Some(i) } else { None })
            .collect()
    }
}

/// Recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: RecommendationCategory,
    pub priority: InsightSeverity,
    pub estimated_savings: Option<f64>,
    pub implementation_effort: ImplementationEffort,
    pub created_at: DateTime<Utc>,
}

impl Recommendation {
    pub fn new(
        title: impl Into<String>,
        category: RecommendationCategory,
        priority: InsightSeverity,
    ) -> Self {
        Self {
            id: format!("rec-{}", Utc::now().timestamp_millis()),
            title: title.into(),
            description: String::new(),
            category,
            priority,
            estimated_savings: None,
            implementation_effort: ImplementationEffort::Medium,
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_savings(mut self, savings: f64) -> Self {
        self.estimated_savings = Some(savings);
        self
    }

    pub fn with_effort(mut self, effort: ImplementationEffort) -> Self {
        self.implementation_effort = effort;
        self
    }
}

/// Recommendation category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationCategory {
    CostOptimization,
    PerformanceImprovement,
    SecurityEnhancement,
    ReliabilityImprovement,
    Sustainability,
}

/// Implementation effort
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImplementationEffort {
    Low,    // < 1 hour
    Medium, // 1-4 hours
    High,   // > 4 hours
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insight_creation() {
        let insight = Insight::new(
            InsightType::Performance,
            InsightSeverity::High,
            "High CPU Usage",
        )
        .with_description("CPU usage is above 90%")
        .with_recommendation("Scale up CPU resources")
        .with_impact("Performance degradation")
        .add_resource("vm1")
        .add_metric("cpu_usage", 95.0);

        assert_eq!(insight.insight_type, InsightType::Performance);
        assert_eq!(insight.severity, InsightSeverity::High);
        assert_eq!(insight.affected_resources.len(), 1);
        assert_eq!(insight.metrics.get("cpu_usage"), Some(&95.0));
    }

    #[test]
    fn test_resource_utilization_high_cpu() {
        let insights = InsightAnalyzer::analyze_resource_utilization(95.0, 50.0, "test-vm");

        assert!(!insights.is_empty());
        assert!(insights.iter().any(|i| i.title.contains("CPU")));
    }

    #[test]
    fn test_resource_utilization_high_memory() {
        let insights = InsightAnalyzer::analyze_resource_utilization(50.0, 90.0, "test-vm");

        assert!(!insights.is_empty());
        assert!(insights.iter().any(|i| i.title.contains("Memory")));
    }

    #[test]
    fn test_resource_utilization_low() {
        let insights = InsightAnalyzer::analyze_resource_utilization(15.0, 25.0, "test-vm");

        assert!(!insights.is_empty());
        let cost_insight = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Cost);
        assert!(cost_insight.is_some());
    }

    #[test]
    fn test_availability_analysis() {
        let insights = InsightAnalyzer::analyze_availability(97.5, "test-vm");

        assert!(!insights.is_empty());
        let insight = &insights[0];
        assert_eq!(insight.insight_type, InsightType::Availability);
        assert_eq!(insight.severity, InsightSeverity::Medium);
    }

    #[test]
    fn test_availability_critical() {
        let insights = InsightAnalyzer::analyze_availability(90.0, "test-vm");

        assert!(!insights.is_empty());
        let insight = &insights[0];
        assert_eq!(insight.severity, InsightSeverity::High);
    }

    #[test]
    fn test_cost_trends_increase() {
        let insights = InsightAnalyzer::analyze_cost_trends(125.0, 100.0, "test-vm");

        assert!(!insights.is_empty());
        let insight = &insights[0];
        assert_eq!(insight.insight_type, InsightType::Cost);
        assert!(insight.metrics.get("cost_increase_percent").unwrap() > &0.0);
    }

    #[test]
    fn test_cost_trends_no_significant_change() {
        let insights = InsightAnalyzer::analyze_cost_trends(105.0, 100.0, "test-vm");
        assert!(insights.is_empty()); // 5% increase shouldn't trigger insight
    }

    #[test]
    fn test_capacity_analysis() {
        let insights = InsightAnalyzer::analyze_capacity(850.0, 1000.0, 10.0);

        assert!(!insights.is_empty());
        let insight = &insights[0];
        assert_eq!(insight.insight_type, InsightType::Capacity);
        assert!(insight.metrics.contains_key("usage_percent"));
    }

    #[test]
    fn test_capacity_within_limits() {
        let insights = InsightAnalyzer::analyze_capacity(500.0, 1000.0, 5.0);
        assert!(insights.is_empty()); // 50% usage shouldn't trigger insight
    }

    #[test]
    fn test_trend_analysis() {
        let trend = Trend::new("cpu_usage", 80.0, 60.0);

        assert_eq!(trend.direction, TrendDirection::Increasing);
        assert!((trend.change_percent - 33.33).abs() < 0.1);
        assert!(trend.is_significant(10.0));
    }

    #[test]
    fn test_trend_decreasing() {
        let trend = Trend::new("memory_usage", 40.0, 60.0);

        assert_eq!(trend.direction, TrendDirection::Decreasing);
        assert!(trend.change_percent < 0.0);
    }

    #[test]
    fn test_trend_stable() {
        let trend = Trend::new("disk_usage", 50.0, 50.0);

        assert_eq!(trend.direction, TrendDirection::Stable);
        assert_eq!(trend.change_percent, 0.0);
    }

    #[test]
    fn test_anomaly_detection() {
        let values = vec![10.0, 12.0, 11.0, 13.0, 100.0, 11.0, 12.0];
        let anomalies = AnomalyDetector::detect(&values, 2.0);

        assert!(!anomalies.is_empty());
        assert!(anomalies.contains(&4)); // Index 4 has value 100.0
    }

    #[test]
    fn test_anomaly_detection_threshold() {
        let values = vec![10.0, 20.0, 15.0, 90.0, 12.0];
        let anomalies = AnomalyDetector::detect_with_threshold(&values, 50.0);

        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0], 3); // Index 3 has value 90.0
    }

    #[test]
    fn test_anomaly_detection_no_anomalies() {
        let values = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let anomalies = AnomalyDetector::detect(&values, 3.0);

        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_recommendation() {
        let rec = Recommendation::new(
            "Downsize VM",
            RecommendationCategory::CostOptimization,
            InsightSeverity::Medium,
        )
        .with_description("VM is underutilized")
        .with_savings(150.0)
        .with_effort(ImplementationEffort::Low);

        assert_eq!(rec.category, RecommendationCategory::CostOptimization);
        assert_eq!(rec.estimated_savings, Some(150.0));
        assert_eq!(rec.implementation_effort, ImplementationEffort::Low);
    }

    #[test]
    fn test_insight_severity_ordering() {
        assert!(InsightSeverity::High > InsightSeverity::Medium);
        assert!(InsightSeverity::Medium > InsightSeverity::Low);
    }

    #[test]
    fn test_multiple_insights() {
        let insights = InsightAnalyzer::analyze_resource_utilization(95.0, 90.0, "test-vm");

        assert!(insights.len() >= 2); // Should have both CPU and memory insights
        assert!(insights.iter().any(|i| i.title.contains("CPU")));
        assert!(insights.iter().any(|i| i.title.contains("Memory")));
    }
}
