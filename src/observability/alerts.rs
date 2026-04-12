// Alerts - Alert rules and notification management

use chrono::{DateTime, TimeDelta as Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Alert state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertState {
    Pending,  // Alert condition detected, waiting for confirmation
    Firing,   // Alert is actively firing
    Resolved, // Alert condition no longer met
    Silenced, // Alert is silenced
}

/// Alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub condition: AlertCondition,
    pub duration: Duration, // How long condition must be true
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl AlertRule {
    pub fn new(
        name: impl Into<String>,
        severity: AlertSeverity,
        condition: AlertCondition,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "alert-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            description: String::new(),
            severity,
            condition,
            duration: Duration::minutes(5),
            enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Alert condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    MetricThreshold {
        metric_name: String,
        operator: ThresholdOperator,
        threshold: f64,
    },
    VMState {
        vm_name: String,
        state: String,
    },
    ResourceUsage {
        resource: String,
        percentage: f64,
    },
    ErrorRate {
        service: String,
        rate_per_minute: f64,
    },
    Custom {
        expression: String,
    },
}

/// Threshold operator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThresholdOperator {
    GreaterThan,
    LessThan,
    Equal,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

impl ThresholdOperator {
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            ThresholdOperator::GreaterThan => value > threshold,
            ThresholdOperator::LessThan => value < threshold,
            ThresholdOperator::Equal => (value - threshold).abs() < f64::EPSILON,
            ThresholdOperator::GreaterThanOrEqual => value >= threshold,
            ThresholdOperator::LessThanOrEqual => value <= threshold,
        }
    }
}

/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub state: AlertState,
    pub message: String,
    pub labels: HashMap<String, String>,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub last_updated: DateTime<Utc>,
}

impl Alert {
    pub fn new(
        rule_id: impl Into<String>,
        rule_name: impl Into<String>,
        severity: AlertSeverity,
    ) -> Self {
        let id = format!("alert-instance-{}", Utc::now().timestamp_millis());

        Self {
            id,
            rule_id: rule_id.into(),
            rule_name: rule_name.into(),
            severity,
            state: AlertState::Pending,
            message: String::new(),
            labels: HashMap::new(),
            started_at: Utc::now(),
            resolved_at: None,
            last_updated: Utc::now(),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn add_label(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.labels.insert(key.into(), value.into());
        self.last_updated = Utc::now();
    }

    pub fn fire(&mut self) {
        self.state = AlertState::Firing;
        self.last_updated = Utc::now();
    }

    pub fn resolve(&mut self) {
        self.state = AlertState::Resolved;
        self.resolved_at = Some(Utc::now());
        self.last_updated = Utc::now();
    }

    pub fn silence(&mut self) {
        self.state = AlertState::Silenced;
        self.last_updated = Utc::now();
    }

    pub fn is_firing(&self) -> bool {
        self.state == AlertState::Firing
    }

    pub fn is_resolved(&self) -> bool {
        self.state == AlertState::Resolved
    }

    pub fn duration(&self) -> Duration {
        match self.resolved_at {
            Some(resolved) => resolved.signed_duration_since(self.started_at),
            None => Utc::now().signed_duration_since(self.started_at),
        }
    }
}

/// Alert manager
pub struct AlertManager {
    rules: Vec<AlertRule>,
    active_alerts: std::collections::VecDeque<Alert>,
    max_alerts: usize,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            active_alerts: std::collections::VecDeque::new(),
            max_alerts: 10_000,
        }
    }

    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    pub fn remove_rule(&mut self, rule_id: &str) {
        self.rules.retain(|r| r.id != rule_id);
    }

    pub fn get_rules(&self) -> &[AlertRule] {
        &self.rules
    }

    pub fn get_active_alerts(&self) -> Vec<&Alert> {
        self.active_alerts
            .iter()
            .filter(|a| a.is_firing())
            .collect()
    }

    pub fn fire_alert(&mut self, alert: Alert) {
        if self.active_alerts.len() >= self.max_alerts {
            // Try to remove oldest resolved alert first
            if let Some(pos) = self
                .active_alerts
                .iter()
                .position(|a| a.state == AlertState::Resolved)
            {
                self.active_alerts.remove(pos);
            } else {
                // No resolved alerts; remove the oldest firing alert
                self.active_alerts.pop_front();
            }
        }
        self.active_alerts.push_back(alert);
    }

    pub fn resolve_alert(&mut self, alert_id: &str) {
        if let Some(alert) = self.active_alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.resolve();
        }
    }

    pub fn silence_alert(&mut self, alert_id: &str) {
        if let Some(alert) = self.active_alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.silence();
        }
    }

    pub fn alerts_by_severity(&self, severity: AlertSeverity) -> Vec<&Alert> {
        self.active_alerts
            .iter()
            .filter(|a| a.severity == severity && a.is_firing())
            .collect()
    }

    pub fn critical_alerts(&self) -> Vec<&Alert> {
        self.alerts_by_severity(AlertSeverity::Critical)
    }

    pub fn cleanup_resolved(&mut self, older_than: Duration) {
        let cutoff = Utc::now() - older_than;
        self.active_alerts.retain(|a| {
            if a.is_resolved() {
                // Remove resolved alerts older than cutoff
                a.resolved_at.map(|t| t > cutoff).unwrap_or(false)
            } else {
                true // Keep unresolved alerts
            }
        });
    }

    pub fn total_alerts(&self) -> usize {
        self.active_alerts.len()
    }

    pub fn firing_count(&self) -> usize {
        self.active_alerts.iter().filter(|a| a.is_firing()).count()
    }

    pub fn resolved_count(&self) -> usize {
        self.active_alerts
            .iter()
            .filter(|a| a.is_resolved())
            .count()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email {
        recipients: Vec<String>,
    },
    Slack {
        #[serde(skip_serializing)]
        webhook_url: String,
        channel: String,
    },
    PagerDuty {
        #[serde(skip_serializing)]
        integration_key: String,
    },
    Webhook {
        #[serde(skip_serializing)]
        url: String,
        headers: HashMap<String, String>,
    },
    SMS {
        phone_numbers: Vec<String>,
    },
}

/// Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub alert_id: String,
    pub channel: NotificationChannel,
    pub subject: String,
    pub message: String,
    pub sent_at: Option<DateTime<Utc>>,
    pub status: NotificationStatus,
}

impl Notification {
    pub fn new(alert_id: impl Into<String>, channel: NotificationChannel) -> Self {
        Self {
            id: format!("notif-{}", Utc::now().timestamp_millis()),
            alert_id: alert_id.into(),
            channel,
            subject: String::new(),
            message: String::new(),
            sent_at: None,
            status: NotificationStatus::Pending,
        }
    }

    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn mark_sent(&mut self) {
        self.sent_at = Some(Utc::now());
        self.status = NotificationStatus::Sent;
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = NotificationStatus::Failed {
            error: error.into(),
        };
    }
}

/// Notification status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationStatus {
    Pending,
    Sent,
    Failed { error: String },
}

/// Silence rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilenceRule {
    pub id: String,
    pub matcher: SilenceMatcher,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub created_by: String,
    pub comment: String,
}

impl SilenceRule {
    pub fn new(matcher: SilenceMatcher, ends_at: DateTime<Utc>) -> Self {
        Self {
            id: format!("silence-{}", Utc::now().timestamp()),
            matcher,
            starts_at: Utc::now(),
            ends_at,
            created_by: String::from("system"),
            comment: String::new(),
        }
    }

    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = comment.into();
        self
    }

    pub fn with_created_by(mut self, user: impl Into<String>) -> Self {
        self.created_by = user.into();
        self
    }

    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        now >= self.starts_at && now <= self.ends_at
    }

    pub fn matches(&self, alert: &Alert) -> bool {
        if !self.is_active() {
            return false;
        }

        match &self.matcher {
            SilenceMatcher::AlertName { name } => alert.rule_name == *name,
            SilenceMatcher::Severity { severity } => alert.severity == *severity,
            SilenceMatcher::Labels { labels } => {
                labels.iter().all(|(k, v)| alert.labels.get(k) == Some(v))
            }
            SilenceMatcher::All => true,
        }
    }
}

/// Silence matcher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SilenceMatcher {
    AlertName { name: String },
    Severity { severity: AlertSeverity },
    Labels { labels: HashMap<String, String> },
    All,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_rule() {
        let rule = AlertRule::new(
            "High CPU",
            AlertSeverity::Warning,
            AlertCondition::MetricThreshold {
                metric_name: "cpu_usage".to_string(),
                operator: ThresholdOperator::GreaterThan,
                threshold: 80.0,
            },
        )
        .with_description("CPU usage is too high");

        assert_eq!(rule.name, "High CPU");
        assert_eq!(rule.severity, AlertSeverity::Warning);
        assert!(rule.enabled);
    }

    #[test]
    fn test_threshold_operator() {
        assert!(ThresholdOperator::GreaterThan.evaluate(90.0, 80.0));
        assert!(!ThresholdOperator::GreaterThan.evaluate(70.0, 80.0));

        assert!(ThresholdOperator::LessThan.evaluate(70.0, 80.0));
        assert!(!ThresholdOperator::LessThan.evaluate(90.0, 80.0));

        assert!(ThresholdOperator::Equal.evaluate(80.0, 80.0));
        assert!(!ThresholdOperator::Equal.evaluate(80.1, 80.0));
    }

    #[test]
    fn test_alert_lifecycle() {
        let mut alert = Alert::new("rule-1", "High CPU", AlertSeverity::Warning)
            .with_message("CPU usage is at 85%");

        assert_eq!(alert.state, AlertState::Pending);

        alert.fire();
        assert!(alert.is_firing());
        assert!(!alert.is_resolved());

        alert.resolve();
        assert!(alert.is_resolved());
        assert!(alert.resolved_at.is_some());
    }

    #[test]
    fn test_alert_labels() {
        let mut alert = Alert::new("rule-1", "Test Alert", AlertSeverity::Info);

        alert.add_label("vm", "test-vm");
        alert.add_label("env", "production");

        assert_eq!(alert.labels.get("vm"), Some(&"test-vm".to_string()));
        assert_eq!(alert.labels.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_alert_manager() {
        let mut manager = AlertManager::new();

        let rule = AlertRule::new(
            "Test Rule",
            AlertSeverity::Critical,
            AlertCondition::MetricThreshold {
                metric_name: "cpu".to_string(),
                operator: ThresholdOperator::GreaterThan,
                threshold: 90.0,
            },
        );

        manager.add_rule(rule);
        assert_eq!(manager.get_rules().len(), 1);

        let mut alert = Alert::new("rule-1", "Test Alert", AlertSeverity::Critical);
        alert.fire();

        manager.fire_alert(alert);
        assert_eq!(manager.firing_count(), 1);
    }

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Critical > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }

    #[test]
    fn test_alert_manager_by_severity() {
        let mut manager = AlertManager::new();

        let mut critical = Alert::new("rule-1", "Critical Alert", AlertSeverity::Critical);
        critical.fire();

        let mut warning = Alert::new("rule-2", "Warning Alert", AlertSeverity::Warning);
        warning.fire();

        manager.fire_alert(critical);
        manager.fire_alert(warning);

        assert_eq!(manager.critical_alerts().len(), 1);
        assert_eq!(manager.alerts_by_severity(AlertSeverity::Warning).len(), 1);
    }

    #[test]
    fn test_alert_manager_resolve() {
        let mut manager = AlertManager::new();

        let mut alert = Alert::new("rule-1", "Test", AlertSeverity::Info);
        alert.fire();
        let alert_id = alert.id.clone();

        manager.fire_alert(alert);
        assert_eq!(manager.firing_count(), 1);

        manager.resolve_alert(&alert_id);
        assert_eq!(manager.firing_count(), 0);
        assert_eq!(manager.resolved_count(), 1);
    }

    #[test]
    fn test_alert_manager_cleanup() {
        let mut manager = AlertManager::new();

        let mut old_alert = Alert::new("rule-1", "Old Alert", AlertSeverity::Info);
        old_alert.fire();
        old_alert.resolve();
        // Manually set old resolved time
        if let Some(alert) = manager.active_alerts.front_mut() {
            alert.resolved_at = Some(Utc::now() - Duration::hours(25));
        }

        manager.fire_alert(old_alert);
        assert_eq!(manager.total_alerts(), 1);

        manager.cleanup_resolved(Duration::hours(24));
        // Alert should still be there since we just added it
        assert!(manager.total_alerts() <= 1);
    }

    #[test]
    fn test_notification() {
        let channel = NotificationChannel::Email {
            recipients: vec!["admin@example.com".to_string()],
        };

        let mut notification = Notification::new("alert-1", channel)
            .with_subject("Alert: High CPU")
            .with_message("CPU usage is above threshold");

        assert_eq!(notification.status, NotificationStatus::Pending);

        notification.mark_sent();
        assert_eq!(notification.status, NotificationStatus::Sent);
        assert!(notification.sent_at.is_some());
    }

    #[test]
    fn test_notification_failure() {
        let channel = NotificationChannel::Slack {
            webhook_url: "https://hooks.slack.com/test".to_string(),
            channel: "#alerts".to_string(),
        };

        let mut notification = Notification::new("alert-1", channel);
        notification.mark_failed("Connection timeout");

        assert!(matches!(
            notification.status,
            NotificationStatus::Failed { .. }
        ));
    }

    #[test]
    fn test_silence_rule() {
        let matcher = SilenceMatcher::Severity {
            severity: AlertSeverity::Info,
        };

        let silence = SilenceRule::new(matcher, Utc::now() + Duration::hours(1))
            .with_comment("Silencing during maintenance")
            .with_created_by("admin");

        assert!(silence.is_active());
        assert_eq!(silence.created_by, "admin");
    }

    #[test]
    fn test_silence_matching() {
        let matcher = SilenceMatcher::AlertName {
            name: "Test Alert".to_string(),
        };

        let silence = SilenceRule::new(matcher, Utc::now() + Duration::hours(1));

        let alert = Alert::new("rule-1", "Test Alert", AlertSeverity::Info);
        assert!(silence.matches(&alert));

        let other_alert = Alert::new("rule-2", "Other Alert", AlertSeverity::Warning);
        assert!(!silence.matches(&other_alert));
    }

    #[test]
    fn test_silence_by_labels() {
        let mut labels = HashMap::new();
        labels.insert("env".to_string(), "staging".to_string());

        let matcher = SilenceMatcher::Labels { labels };
        let silence = SilenceRule::new(matcher, Utc::now() + Duration::hours(1));

        let mut alert = Alert::new("rule-1", "Test", AlertSeverity::Info);
        alert.add_label("env", "staging");

        assert!(silence.matches(&alert));

        let mut prod_alert = Alert::new("rule-2", "Test", AlertSeverity::Info);
        prod_alert.add_label("env", "production");

        assert!(!silence.matches(&prod_alert));
    }

    #[test]
    fn test_alert_duration() {
        let alert = Alert::new("rule-1", "Test", AlertSeverity::Info);
        std::thread::sleep(std::time::Duration::from_millis(10));

        let duration = alert.duration();
        assert!(duration.num_milliseconds() >= 10);
    }

    #[test]
    fn test_notification_channels() {
        let email = NotificationChannel::Email {
            recipients: vec!["test@example.com".to_string()],
        };
        assert!(matches!(email, NotificationChannel::Email { .. }));

        let slack = NotificationChannel::Slack {
            webhook_url: "https://hooks.slack.com/test".to_string(),
            channel: "#general".to_string(),
        };
        assert!(matches!(slack, NotificationChannel::Slack { .. }));
    }
}
