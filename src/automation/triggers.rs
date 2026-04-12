// Triggers - Event triggers for automation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event trigger manager
pub struct TriggerManager {
    event_handlers: HashMap<String, Vec<EventHandler>>,
}

impl TriggerManager {
    pub fn new() -> Self {
        Self {
            event_handlers: HashMap::new(),
        }
    }

    pub fn register_handler(&mut self, event_type: impl Into<String>, handler: EventHandler) {
        self.event_handlers
            .entry(event_type.into())
            .or_default()
            .push(handler);
    }

    pub fn trigger_event(&self, event: &Event) -> Vec<String> {
        if let Some(handlers) = self.event_handlers.get(&event.event_type) {
            handlers
                .iter()
                .filter(|h| h.matches(event))
                .map(|h| h.rule_id.clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn handler_count(&self) -> usize {
        self.event_handlers.values().map(|v| v.len()).sum()
    }
}

impl Default for TriggerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Event handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandler {
    pub rule_id: String,
    pub event_type: String,
    pub filters: HashMap<String, String>,
}

impl EventHandler {
    pub fn new(rule_id: impl Into<String>, event_type: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            event_type: event_type.into(),
            filters: HashMap::new(),
        }
    }

    pub fn with_filter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.filters.insert(key.into(), value.into());
        self
    }

    pub fn matches(&self, event: &Event) -> bool {
        // Check if all filters match
        for (key, value) in &self.filters {
            if event.data.get(key) != Some(value) {
                return false;
            }
        }
        true
    }
}

/// Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub event_type: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub data: HashMap<String, String>,
}

impl Event {
    pub fn new(event_type: impl Into<String>, source: impl Into<String>) -> Self {
        let event_id = format!("evt-{}", Utc::now().format("%Y%m%d-%H%M%S-%f"));

        Self {
            event_id,
            event_type: event_type.into(),
            source: source.into(),
            timestamp: Utc::now(),
            data: HashMap::new(),
        }
    }

    pub fn add_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

/// Common event types
pub struct EventTypes;

impl EventTypes {
    pub const VM_CREATED: &'static str = "vm.created";
    pub const VM_STARTED: &'static str = "vm.started";
    pub const VM_STOPPED: &'static str = "vm.stopped";
    pub const VM_DELETED: &'static str = "vm.deleted";
    pub const VM_FAILED: &'static str = "vm.failed";
    pub const SNAPSHOT_CREATED: &'static str = "snapshot.created";
    pub const SNAPSHOT_DELETED: &'static str = "snapshot.deleted";
    pub const BACKUP_COMPLETED: &'static str = "backup.completed";
    pub const BACKUP_FAILED: &'static str = "backup.failed";
    pub const METRIC_THRESHOLD: &'static str = "metric.threshold";
    pub const BUDGET_EXCEEDED: &'static str = "budget.exceeded";
    pub const SECURITY_ALERT: &'static str = "security.alert";
}

/// Webhook trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTrigger {
    pub id: String,
    pub url: String,
    #[serde(skip_serializing)]
    pub secret: Option<String>,
    pub events: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl WebhookTrigger {
    pub fn new(url: impl Into<String>) -> Self {
        let id = format!("webhook-{}", Utc::now().timestamp());

        Self {
            id,
            url: url.into(),
            secret: None,
            events: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    pub fn for_event(mut self, event: impl Into<String>) -> Self {
        self.events.push(event.into());
        self
    }

    pub fn accepts_event(&self, event_type: &str) -> bool {
        self.events.is_empty() || self.events.contains(&event_type.to_string())
    }
}

/// Metric threshold trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricThresholdTrigger {
    pub metric_name: String,
    pub threshold: f64,
    pub operator: ThresholdOperator,
    pub duration_seconds: u64,
    pub evaluation_period: u64,
}

impl MetricThresholdTrigger {
    pub fn new(
        metric_name: impl Into<String>,
        threshold: f64,
        operator: ThresholdOperator,
    ) -> Self {
        Self {
            metric_name: metric_name.into(),
            threshold,
            operator,
            duration_seconds: 300, // 5 minutes default
            evaluation_period: 60, // 1 minute default
        }
    }

    pub fn with_duration(mut self, seconds: u64) -> Self {
        self.duration_seconds = seconds;
        self
    }

    pub fn with_evaluation_period(mut self, seconds: u64) -> Self {
        self.evaluation_period = seconds;
        self
    }

    pub fn evaluate(&self, value: f64) -> bool {
        match self.operator {
            ThresholdOperator::GreaterThan => value > self.threshold,
            ThresholdOperator::LessThan => value < self.threshold,
            ThresholdOperator::GreaterThanOrEqual => value >= self.threshold,
            ThresholdOperator::LessThanOrEqual => value <= self.threshold,
            ThresholdOperator::Equal => (value - self.threshold).abs() < f64::EPSILON,
        }
    }
}

/// Threshold operator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThresholdOperator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_manager() {
        let mut manager = TriggerManager::new();

        let handler = EventHandler::new("rule-1", EventTypes::VM_STARTED)
            .with_filter("namespace", "production");

        manager.register_handler(EventTypes::VM_STARTED, handler);

        assert_eq!(manager.handler_count(), 1);

        let event =
            Event::new(EventTypes::VM_STARTED, "test-vm").add_data("namespace", "production");

        let triggered = manager.trigger_event(&event);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0], "rule-1");
    }

    #[test]
    fn test_event_handler_matching() {
        let handler = EventHandler::new("rule-1", EventTypes::VM_STARTED)
            .with_filter("namespace", "production")
            .with_filter("tier", "frontend");

        let matching_event = Event::new(EventTypes::VM_STARTED, "vm1")
            .add_data("namespace", "production")
            .add_data("tier", "frontend");

        assert!(handler.matches(&matching_event));

        let non_matching_event = Event::new(EventTypes::VM_STARTED, "vm2")
            .add_data("namespace", "development")
            .add_data("tier", "frontend");

        assert!(!handler.matches(&non_matching_event));
    }

    #[test]
    fn test_event_creation() {
        let event = Event::new(EventTypes::VM_CREATED, "new-vm")
            .add_data("namespace", "default")
            .add_data("cpu", "4");

        assert_eq!(event.event_type, EventTypes::VM_CREATED);
        assert_eq!(event.source, "new-vm");
        assert_eq!(event.data.get("namespace"), Some(&"default".to_string()));
    }

    #[test]
    fn test_webhook_trigger() {
        let webhook = WebhookTrigger::new("https://example.com/webhook")
            .with_secret("secret123")
            .for_event(EventTypes::VM_STARTED)
            .for_event(EventTypes::VM_STOPPED);

        assert_eq!(webhook.url, "https://example.com/webhook");
        assert_eq!(webhook.secret, Some("secret123".to_string()));
        assert!(webhook.accepts_event(EventTypes::VM_STARTED));
        assert!(!webhook.accepts_event(EventTypes::VM_CREATED));
    }

    #[test]
    fn test_webhook_accepts_all_events() {
        let webhook = WebhookTrigger::new("https://example.com/webhook");

        // Empty events list means accept all
        assert!(webhook.accepts_event(EventTypes::VM_STARTED));
        assert!(webhook.accepts_event(EventTypes::VM_CREATED));
    }

    #[test]
    fn test_metric_threshold_trigger() {
        let trigger =
            MetricThresholdTrigger::new("cpu_usage", 80.0, ThresholdOperator::GreaterThan)
                .with_duration(600)
                .with_evaluation_period(120);

        assert_eq!(trigger.metric_name, "cpu_usage");
        assert_eq!(trigger.threshold, 80.0);
        assert_eq!(trigger.duration_seconds, 600);

        assert!(trigger.evaluate(85.0));
        assert!(!trigger.evaluate(75.0));
        assert!(!trigger.evaluate(80.0));
    }

    #[test]
    fn test_threshold_operators() {
        let gt = MetricThresholdTrigger::new("metric", 50.0, ThresholdOperator::GreaterThan);
        assert!(gt.evaluate(51.0));
        assert!(!gt.evaluate(50.0));

        let lt = MetricThresholdTrigger::new("metric", 50.0, ThresholdOperator::LessThan);
        assert!(lt.evaluate(49.0));
        assert!(!lt.evaluate(50.0));

        let gte =
            MetricThresholdTrigger::new("metric", 50.0, ThresholdOperator::GreaterThanOrEqual);
        assert!(gte.evaluate(50.0));
        assert!(gte.evaluate(51.0));

        let lte = MetricThresholdTrigger::new("metric", 50.0, ThresholdOperator::LessThanOrEqual);
        assert!(lte.evaluate(50.0));
        assert!(lte.evaluate(49.0));
    }

    #[test]
    fn test_event_types() {
        assert_eq!(EventTypes::VM_CREATED, "vm.created");
        assert_eq!(EventTypes::VM_STARTED, "vm.started");
        assert_eq!(EventTypes::METRIC_THRESHOLD, "metric.threshold");
    }

    #[test]
    fn test_multiple_handlers() {
        let mut manager = TriggerManager::new();

        manager.register_handler(
            EventTypes::VM_STARTED,
            EventHandler::new("rule-1", EventTypes::VM_STARTED),
        );
        manager.register_handler(
            EventTypes::VM_STARTED,
            EventHandler::new("rule-2", EventTypes::VM_STARTED),
        );

        let event = Event::new(EventTypes::VM_STARTED, "test-vm");
        let triggered = manager.trigger_event(&event);

        assert_eq!(triggered.len(), 2);
    }

    #[test]
    fn test_no_matching_handlers() {
        let manager = TriggerManager::new();
        let event = Event::new(EventTypes::VM_STARTED, "test-vm");
        let triggered = manager.trigger_event(&event);

        assert_eq!(triggered.len(), 0);
    }
}
