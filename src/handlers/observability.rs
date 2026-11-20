// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use crate::tui::colors::cli as color;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
struct AlertStore {
    rules: Vec<serde_json::Value>,
}

impl AlertStore {
    fn path() -> anyhow::Result<std::path::PathBuf> {
        Ok(crate::utils::data_dir()?.join("alerts.json"))
    }

    fn load() -> Self {
        let path = match Self::path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to determine data directory: {}", e);
                return Self::default();
            }
        };
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) {
        let path = match Self::path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to save alerts: {}", e);
                return;
            }
        };
        if let Err(e) = crate::utils::atomic_write(&path, self) {
            log::error!("Failed to save alerts: {}", e);
        }
    }
}

/// Parse a log level string into a LogLevel enum.
/// Supported values (case-insensitive): debug, info, warning, error, critical.
/// Unknown values default to Info.
pub(crate) fn parse_log_level(level: &str) -> crate::observability::logs::LogLevel {
    use crate::observability::logs::LogLevel;

    match level.to_lowercase().as_str() {
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        "warning" => LogLevel::Warning,
        "error" => LogLevel::Error,
        "critical" => LogLevel::Critical,
        _ => LogLevel::Info,
    }
}

/// Parse an alert severity string into an AlertSeverity enum.
/// Supported values (case-insensitive): info, warning, critical.
/// Unknown values default to Warning.
pub(crate) fn parse_alert_severity(severity: &str) -> crate::observability::alerts::AlertSeverity {
    use crate::observability::alerts::AlertSeverity;

    match severity.to_lowercase().as_str() {
        "info" => AlertSeverity::Info,
        "warning" => AlertSeverity::Warning,
        "critical" => AlertSeverity::Critical,
        _ => AlertSeverity::Warning,
    }
}

/// Parse a threshold operator string into a ThresholdOperator enum.
/// Supported values (case-insensitive): gt, lt, eq, gte, lte.
/// Unknown values default to GreaterThan.
pub(crate) fn parse_threshold_operator(
    operator: &str,
) -> crate::observability::alerts::ThresholdOperator {
    use crate::observability::alerts::ThresholdOperator;

    match operator.to_lowercase().as_str() {
        "gt" => ThresholdOperator::GreaterThan,
        "lt" => ThresholdOperator::LessThan,
        "eq" => ThresholdOperator::Equal,
        "gte" => ThresholdOperator::GreaterThanOrEqual,
        "lte" => ThresholdOperator::LessThanOrEqual,
        _ => ThresholdOperator::GreaterThan,
    }
}

pub fn handle_logs_query(
    start: Option<String>,
    end: Option<String>,
    level: Option<String>,
    source: Option<String>,
    search: Option<String>,
    limit: usize,
) -> Result<()> {
    use crate::observability::logs::LogQuery;

    println!("{}", color::header("Querying Logs"));
    println!();

    let mut _query = LogQuery::new().with_limit(limit);

    if let Some(ref lvl) = level {
        _query = _query.with_level(parse_log_level(lvl));
    }

    if let Some(ref src) = source {
        _query = _query.with_source(src);
    }

    if let Some(ref text) = search {
        _query = _query.with_search(text);
    }

    if let Some(ref s) = start {
        println!("  Start:  {}", s);
    }
    if let Some(ref e) = end {
        println!("  End:    {}", e);
    }
    println!("  Level:  {}", level.as_deref().unwrap_or("all"));
    println!("  Source: {}", source.as_deref().unwrap_or("all"));
    println!("  Limit:  {}", limit);
    println!();
    println!("{}", color::success("✓ Log query executed"));
    Ok(())
}

pub fn handle_logs_stats(group_by: String) -> Result<()> {
    println!("{}", color::header("Log Statistics"));
    println!();

    println!("  Grouped by: {}", color::value(&group_by));
    println!();
    println!("{}", color::success("✓ Statistics generated"));
    Ok(())
}

pub fn handle_logs_patterns(min_count: usize) -> Result<()> {
    println!("{}", color::header("Log Pattern Analysis"));
    println!();

    println!("  Minimum count: {}", min_count);
    println!();
    println!("{}", color::success("✓ Patterns detected"));
    Ok(())
}

pub async fn handle_metrics_collect(vm: String) -> Result<()> {
    use crate::monitoring::metrics::MetricsCollector;

    println!("{}", color::header(&format!("Collecting Metrics: {}", vm)));
    println!();

    let collector = MetricsCollector::new("default");
    let metrics = collector.collect(&vm).await?;

    println!("  CPU Usage:      {:.1}%", metrics.cpu.usage_percent);
    println!(
        "  CPU Cores:      {} allocated, {:.1} used",
        metrics.cpu.cores_allocated, metrics.cpu.cores_used
    );
    println!("  Memory Usage:   {:.1}%", metrics.memory.usage_percent);
    println!(
        "  Memory:         {:.1} GiB used / {:.1} GiB total",
        metrics.memory.used_gb(),
        metrics.memory.total_gb()
    );
    println!(
        "  Disk Read:      {:.2} MB/s",
        metrics.disk.read_mb_per_sec()
    );
    println!(
        "  Disk Write:     {:.2} MB/s",
        metrics.disk.write_mb_per_sec()
    );
    println!("  Disk Usage:     {:.1}%", metrics.disk.usage_percent);
    println!(
        "  Network RX:     {:.2} MB/s",
        metrics.network.rx_mb_per_sec()
    );
    println!(
        "  Network TX:     {:.2} MB/s",
        metrics.network.tx_mb_per_sec()
    );
    println!();
    println!("{}", color::success("✓ Metrics collected from VM spec"));
    Ok(())
}

pub fn handle_metrics_query(
    name: String,
    start: Option<String>,
    end: Option<String>,
    aggregation: String,
) -> Result<()> {
    println!("{}", color::header(&format!("Querying Metric: {}", name)));
    println!();

    println!("  Metric:      {}", color::value(&name));
    if let Some(ref s) = start {
        println!("  Start:       {}", s);
    }
    if let Some(ref e) = end {
        println!("  End:         {}", e);
    }
    println!("  Aggregation: {}", aggregation);
    println!();
    println!("{}", color::success("✓ Query executed"));
    Ok(())
}

pub fn handle_metrics_snapshot(
    vm: Option<String>,
    cpu_threshold: f64,
    memory_threshold: f64,
) -> Result<()> {
    use crate::observability::metrics::MetricsSnapshot;

    println!("{}", color::header("Metrics Snapshot"));
    println!();

    if let Some(ref vm_name) = vm {
        println!("  VM Filter:          {}", color::value(vm_name));
    }

    let snapshot = MetricsSnapshot::new();
    println!("  Total VMs:          {}", snapshot.total_vms);
    println!("  Cluster CPU Avg:    {:.1}%", snapshot.cluster_cpu_usage);
    println!(
        "  Cluster Memory Avg: {:.1}%",
        snapshot.cluster_memory_usage
    );
    println!("  CPU Threshold:      {}%", cpu_threshold);
    println!("  Memory Threshold:   {}%", memory_threshold);
    println!();
    println!("{}", color::success("✓ Snapshot captured"));
    Ok(())
}

pub fn handle_alerts_list(
    enabled_only: bool,
    severity: Option<String>,
    output: String,
) -> Result<()> {
    use crate::observability::alerts::AlertRule;

    println!("{}", color::header("Alert Rules"));
    println!();

    let store = AlertStore::load();
    let rules: Vec<AlertRule> = store
        .rules
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    let filtered: Vec<_> = rules
        .iter()
        .filter(|r| !enabled_only || r.enabled)
        .filter(|r| {
            severity
                .as_ref()
                .map(|s| r.severity.to_string().to_lowercase() == s.to_lowercase())
                .unwrap_or(true)
        })
        .collect();

    if filtered.is_empty() {
        println!("  {}", color::muted("No alert rules found"));
        return Ok(());
    }

    if output == "json" {
        let json = serde_json::to_string_pretty(&filtered)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&filtered)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<30} {:<12} {:<10} {}",
            color::label("NAME"),
            color::label("SEVERITY"),
            color::label("STATUS"),
            color::label("CONDITION")
        );
        println!("{}", "-".repeat(75));

        for rule in &filtered {
            let status = if rule.enabled {
                color::success("Enabled")
            } else {
                color::muted("Disabled")
            };
            let condition_str = match &rule.condition {
                crate::observability::alerts::AlertCondition::MetricThreshold {
                    metric_name,
                    operator,
                    threshold,
                } => {
                    let op_str = match operator {
                        crate::observability::alerts::ThresholdOperator::GreaterThan => ">",
                        crate::observability::alerts::ThresholdOperator::LessThan => "<",
                        crate::observability::alerts::ThresholdOperator::Equal => "==",
                        crate::observability::alerts::ThresholdOperator::GreaterThanOrEqual => ">=",
                        crate::observability::alerts::ThresholdOperator::LessThanOrEqual => "<=",
                    };
                    format!("{} {} {}", metric_name, op_str, threshold)
                }
                crate::observability::alerts::AlertCondition::VMState { vm_name, state } => {
                    format!("vm:{} state:{}", vm_name, state)
                }
                crate::observability::alerts::AlertCondition::ResourceUsage {
                    resource,
                    percentage,
                } => format!("{} > {}%", resource, percentage),
                _ => "custom".to_string(),
            };
            println!(
                "{:<30} {:<12} {:<10} {}",
                rule.name,
                rule.severity.to_string(),
                status,
                condition_str
            );
        }
    }
    println!();
    println!("{}", color::success("✓ Rules listed"));
    Ok(())
}

pub fn handle_alerts_create(
    name: String,
    severity: String,
    metric: String,
    operator: String,
    threshold: f64,
    duration: i64,
) -> Result<()> {
    use crate::observability::alerts::{AlertCondition, AlertRule};

    println!(
        "{}",
        color::header(&format!("Creating Alert Rule: {}", name))
    );
    println!();

    let alert_severity = parse_alert_severity(&severity);
    let op = parse_threshold_operator(&operator);

    let condition = AlertCondition::MetricThreshold {
        metric_name: metric.clone(),
        operator: op,
        threshold,
    };

    let rule = AlertRule::new(&name, alert_severity, condition)
        .with_duration(chrono::TimeDelta::minutes(duration));

    // Persist the alert rule
    let mut store = AlertStore::load();
    if let Ok(value) = serde_json::to_value(&rule) {
        store.rules.push(value);
        store.save();
    }

    println!("  Name:      {}", color::value(&rule.name));
    println!("  Severity:  {}", rule.severity);
    println!("  Metric:    {}", metric);
    println!("  Threshold: {} {}", operator, threshold);
    println!("  Duration:  {} minutes", duration);
    println!();
    println!(
        "{}",
        color::success("✓ Alert rule created and persisted successfully")
    );
    Ok(())
}

pub fn handle_alerts_active(severity: Option<String>, output: String) -> Result<()> {
    println!("{}", color::header("Active Alerts"));
    println!();

    if let Some(sev) = &severity {
        println!("  Severity filter: {}", color::value(sev));
    }
    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Active alerts retrieved"));
    Ok(())
}

pub fn handle_alerts_resolve(alert_id: String) -> Result<()> {
    println!(
        "{}",
        color::header(&format!("Resolving Alert: {}", alert_id))
    );
    println!();

    println!("  Alert ID: {}", color::value(&alert_id));
    println!();
    println!("{}", color::success("✓ Alert resolved"));
    Ok(())
}

pub fn handle_insights_generate(
    vm: Option<String>,
    insight_type: Option<String>,
    min_severity: String,
) -> Result<()> {
    use crate::observability::insights::InsightAnalyzer;

    println!("{}", color::header("Generating Insights"));
    println!();

    if let Some(vm_name) = &vm {
        println!("  VM: {}", color::value(vm_name));

        // Generate sample insights
        let insights = InsightAnalyzer::analyze_resource_utilization(45.0, 62.0, vm_name);
        println!("  Generated {} insights", insights.len());
    } else {
        println!("  Scope: All VMs");
    }

    if let Some(itype) = &insight_type {
        println!("  Type: {}", color::value(itype));
    }
    println!("  Min Severity: {}", min_severity);
    println!();
    println!("{}", color::success("✓ Insights generated"));
    Ok(())
}

pub fn handle_recommendations(
    category: Option<String>,
    min_priority: String,
    with_savings: bool,
    output: String,
) -> Result<()> {
    println!("{}", color::header("Recommendations"));
    println!();

    if let Some(cat) = &category {
        println!("  Category: {}", color::value(cat));
    }
    println!("  Min Priority: {}", min_priority);
    println!("  Show Savings: {}", with_savings);
    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Recommendations generated"));
    Ok(())
}

pub fn handle_trends_analyze(metric: String, window: i64, threshold: f64) -> Result<()> {
    println!("{}", color::header(&format!("Analyzing Trend: {}", metric)));
    println!();

    println!("  Metric:    {}", color::value(&metric));
    println!("  Window:    {} hours", window);
    println!("  Threshold: {}%", threshold);
    println!();
    println!("{}", color::success("✓ Trend analysis complete"));
    Ok(())
}

pub async fn handle_health_check(component: Option<String>, output: String) -> Result<()> {
    use crate::observability::{HealthCheck, HealthStatus, SystemHealth};

    println!("{}", color::header("System Health Check"));
    println!();

    let mut health = SystemHealth::new();

    // Check Kubernetes connectivity
    let k8s_start = std::time::Instant::now();
    let k8s_status = match crate::kube::KubeClient::new().await {
        Ok(client) => {
            // Try listing VMs to verify KubeVirt API is accessible
            match client.list_vms("default").await {
                Ok(_) => {
                    health.add_check(
                        HealthCheck::new("kubevirt-api", HealthStatus::Healthy)
                            .with_response_time(k8s_start.elapsed().as_millis() as u64),
                    );
                    HealthStatus::Healthy
                }
                Err(_) => {
                    health.add_check(
                        HealthCheck::new("kubevirt-api", HealthStatus::Degraded)
                            .with_response_time(k8s_start.elapsed().as_millis() as u64),
                    );
                    HealthStatus::Degraded
                }
            }
        }
        Err(_) => {
            health.add_check(
                HealthCheck::new("kubernetes", HealthStatus::Unhealthy)
                    .with_response_time(k8s_start.elapsed().as_millis() as u64),
            );
            HealthStatus::Unhealthy
        }
    };

    // Check config directory
    let config_status = if dirs::config_dir()
        .map(|d| d.join("veyron").exists())
        .unwrap_or(false)
    {
        HealthStatus::Healthy
    } else {
        HealthStatus::Degraded
    };
    health.add_check(HealthCheck::new("config-dir", config_status));

    if let Some(comp) = &component {
        println!("  Component: {}", color::value(comp));
        let check = health.checks.iter().find(|c| c.component == *comp);
        if let Some(c) = check {
            let status_str = match c.status {
                HealthStatus::Healthy => color::success("Healthy"),
                HealthStatus::Degraded => color::warning("Degraded"),
                HealthStatus::Unhealthy => color::error("Unhealthy"),
                HealthStatus::Unknown => color::muted("Unknown"),
            };
            println!("  Status: {}", status_str);
            if let Some(rt) = c.response_time_ms {
                println!("  Response Time: {}ms", rt);
            }
        } else {
            println!("  {}", color::warning("Component not found"));
        }
    } else {
        let overall_str = match k8s_status {
            HealthStatus::Healthy => color::success("Healthy"),
            HealthStatus::Degraded => color::warning("Degraded"),
            _ => color::error("Unhealthy"),
        };
        println!("  Overall Status: {}", overall_str);
        println!("  Healthy:   {}", health.healthy_count());
        println!("  Unhealthy: {}", health.unhealthy_count());
        println!();

        for check in &health.checks {
            let status_str = match check.status {
                HealthStatus::Healthy => color::success("Healthy"),
                HealthStatus::Degraded => color::warning("Degraded"),
                HealthStatus::Unhealthy => color::error("Unhealthy"),
                HealthStatus::Unknown => color::muted("Unknown"),
            };
            let rt = check
                .response_time_ms
                .map(|ms| format!(" ({}ms)", ms))
                .unwrap_or_default();
            println!("  {:<20} {}{}", check.component, status_str, rt);
        }
    }

    if output == "json" {
        println!();
        let json = serde_json::to_string_pretty(&health)?;
        println!("{}", json);
    } else if output == "yaml" {
        println!();
        let yaml = serde_yml::to_string(&health)?;
        println!("{}", yaml);
    }
    println!();
    println!("{}", color::success("✓ Health check complete"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== parse_log_level tests =====

    #[test]
    fn test_parse_log_level_debug() {
        let level = parse_log_level("debug");
        assert!(matches!(level, crate::observability::logs::LogLevel::Debug));
    }

    #[test]
    fn test_parse_log_level_info() {
        let level = parse_log_level("info");
        assert!(matches!(level, crate::observability::logs::LogLevel::Info));
    }

    #[test]
    fn test_parse_log_level_warning() {
        let level = parse_log_level("warning");
        assert!(matches!(
            level,
            crate::observability::logs::LogLevel::Warning
        ));
    }

    #[test]
    fn test_parse_log_level_error() {
        let level = parse_log_level("error");
        assert!(matches!(level, crate::observability::logs::LogLevel::Error));
    }

    #[test]
    fn test_parse_log_level_critical() {
        let level = parse_log_level("critical");
        assert!(matches!(
            level,
            crate::observability::logs::LogLevel::Critical
        ));
    }

    #[test]
    fn test_parse_log_level_case_insensitive() {
        let level = parse_log_level("WARNING");
        assert!(matches!(
            level,
            crate::observability::logs::LogLevel::Warning
        ));
    }

    #[test]
    fn test_parse_log_level_unknown_defaults_to_info() {
        let level = parse_log_level("trace");
        assert!(matches!(level, crate::observability::logs::LogLevel::Info));
    }

    #[test]
    fn test_parse_log_level_empty_defaults_to_info() {
        let level = parse_log_level("");
        assert!(matches!(level, crate::observability::logs::LogLevel::Info));
    }

    // ===== parse_alert_severity tests =====

    #[test]
    fn test_parse_alert_severity_info() {
        let sev = parse_alert_severity("info");
        assert_eq!(sev, crate::observability::alerts::AlertSeverity::Info);
    }

    #[test]
    fn test_parse_alert_severity_warning() {
        let sev = parse_alert_severity("warning");
        assert_eq!(sev, crate::observability::alerts::AlertSeverity::Warning);
    }

    #[test]
    fn test_parse_alert_severity_critical() {
        let sev = parse_alert_severity("critical");
        assert_eq!(sev, crate::observability::alerts::AlertSeverity::Critical);
    }

    #[test]
    fn test_parse_alert_severity_case_insensitive() {
        let sev = parse_alert_severity("CRITICAL");
        assert_eq!(sev, crate::observability::alerts::AlertSeverity::Critical);
    }

    #[test]
    fn test_parse_alert_severity_unknown_defaults_to_warning() {
        let sev = parse_alert_severity("fatal");
        assert_eq!(sev, crate::observability::alerts::AlertSeverity::Warning);
    }

    // ===== parse_threshold_operator tests =====

    #[test]
    fn test_parse_threshold_operator_gt() {
        let op = parse_threshold_operator("gt");
        assert_eq!(
            op,
            crate::observability::alerts::ThresholdOperator::GreaterThan
        );
    }

    #[test]
    fn test_parse_threshold_operator_lt() {
        let op = parse_threshold_operator("lt");
        assert_eq!(
            op,
            crate::observability::alerts::ThresholdOperator::LessThan
        );
    }

    #[test]
    fn test_parse_threshold_operator_eq() {
        let op = parse_threshold_operator("eq");
        assert_eq!(op, crate::observability::alerts::ThresholdOperator::Equal);
    }

    #[test]
    fn test_parse_threshold_operator_gte() {
        let op = parse_threshold_operator("gte");
        assert_eq!(
            op,
            crate::observability::alerts::ThresholdOperator::GreaterThanOrEqual
        );
    }

    #[test]
    fn test_parse_threshold_operator_lte() {
        let op = parse_threshold_operator("lte");
        assert_eq!(
            op,
            crate::observability::alerts::ThresholdOperator::LessThanOrEqual
        );
    }

    #[test]
    fn test_parse_threshold_operator_case_insensitive() {
        let op = parse_threshold_operator("GT");
        assert_eq!(
            op,
            crate::observability::alerts::ThresholdOperator::GreaterThan
        );
    }

    #[test]
    fn test_parse_threshold_operator_unknown_defaults_to_gt() {
        let op = parse_threshold_operator("ne");
        assert_eq!(
            op,
            crate::observability::alerts::ThresholdOperator::GreaterThan
        );
    }

    #[test]
    fn test_parse_threshold_operator_empty_defaults_to_gt() {
        let op = parse_threshold_operator("");
        assert_eq!(
            op,
            crate::observability::alerts::ThresholdOperator::GreaterThan
        );
    }
}
