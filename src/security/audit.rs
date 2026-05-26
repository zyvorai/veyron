// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Audit Logging - Security audit logging and event tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub severity: EventSeverity,
    pub source: EventSource,
    pub actor: String,
    pub target: String,
    pub action: String,
    pub outcome: EventOutcome,
    pub details: HashMap<String, String>,
    pub ip_address: Option<String>,
}

impl AuditEvent {
    pub fn new(
        event_type: EventType,
        actor: impl Into<String>,
        target: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        let event_id = {
            use rand::Rng;
            let random: u32 = rand::thread_rng().r#gen();
            format!("evt-{}-{:08x}", Utc::now().format("%Y%m%d-%H%M%S"), random)
        };
        Self {
            event_id,
            timestamp: Utc::now(),
            event_type,
            severity: EventSeverity::Info,
            source: EventSource::System,
            actor: actor.into(),
            target: target.into(),
            action: action.into(),
            outcome: EventOutcome::Success,
            details: HashMap::new(),
            ip_address: None,
        }
    }

    pub fn with_severity(mut self, severity: EventSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_source(mut self, source: EventSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_outcome(mut self, outcome: EventOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn add_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    pub fn is_security_event(&self) -> bool {
        matches!(
            self.event_type,
            EventType::Authentication
                | EventType::Authorization
                | EventType::SecurityViolation
                | EventType::PolicyViolation
        )
    }

    pub fn is_critical(&self) -> bool {
        self.severity == EventSeverity::Critical
    }
}

/// Event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    Authentication,
    Authorization,
    VMOperation,
    Configuration,
    SecurityViolation,
    PolicyViolation,
    DataAccess,
    NetworkAccess,
    SystemChange,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Authentication => write!(f, "Authentication"),
            EventType::Authorization => write!(f, "Authorization"),
            EventType::VMOperation => write!(f, "VM Operation"),
            EventType::Configuration => write!(f, "Configuration"),
            EventType::SecurityViolation => write!(f, "Security Violation"),
            EventType::PolicyViolation => write!(f, "Policy Violation"),
            EventType::DataAccess => write!(f, "Data Access"),
            EventType::NetworkAccess => write!(f, "Network Access"),
            EventType::SystemChange => write!(f, "System Change"),
        }
    }
}

/// Event severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for EventSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSeverity::Critical => write!(f, "Critical"),
            EventSeverity::High => write!(f, "High"),
            EventSeverity::Medium => write!(f, "Medium"),
            EventSeverity::Low => write!(f, "Low"),
            EventSeverity::Info => write!(f, "Info"),
        }
    }
}

/// Event source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventSource {
    System,
    User,
    API,
    Automation,
    External,
}

/// Event outcome
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventOutcome {
    Success,
    Failure,
    Partial,
}

impl std::fmt::Display for EventOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventOutcome::Success => write!(f, "Success"),
            EventOutcome::Failure => write!(f, "Failure"),
            EventOutcome::Partial => write!(f, "Partial"),
        }
    }
}

/// Audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub log_id: String,
    pub vm_name: Option<String>,
    pub events: VecDeque<AuditEvent>,
    pub max_entries: usize,
    pub events_dropped: u64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl AuditLog {
    pub fn new(vm_name: Option<String>) -> Self {
        let log_id = {
            use rand::Rng;
            let random: u16 = rand::thread_rng().r#gen();
            format!(
                "log-{}-{:04x}",
                Utc::now().format("%Y%m%d-%H%M%S%f"),
                random
            )
        };
        Self {
            log_id,
            vm_name,
            events: VecDeque::new(),
            max_entries: 10_000,
            events_dropped: 0,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    pub fn add_event(&mut self, event: AuditEvent) {
        if self.events.len() >= self.max_entries {
            // Find the first non-critical, non-security event to drop
            let drop_index = self
                .events
                .iter()
                .position(|e| !e.is_critical() && !e.is_security_event());
            if let Some(idx) = drop_index {
                self.events.remove(idx);
            } else {
                // All events are critical/security - drop oldest as last resort
                log::error!("Audit log full: dropping critical event due to capacity");
                self.events.pop_front();
            }
            self.events_dropped += 1;
        }
        self.events.push_back(event);
        self.last_updated = Utc::now();
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn security_events(&self) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.is_security_event())
            .collect()
    }

    pub fn critical_events(&self) -> Vec<&AuditEvent> {
        self.events.iter().filter(|e| e.is_critical()).collect()
    }

    pub fn events_by_type(&self, event_type: &EventType) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| &e.event_type == event_type)
            .collect()
    }

    pub fn failed_events(&self) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.outcome == EventOutcome::Failure)
            .collect()
    }
}

/// Audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_events: usize,
    pub security_events: usize,
    pub critical_events: usize,
    pub failed_events: usize,
    pub events_by_type: HashMap<String, usize>,
    pub events_by_severity: HashMap<String, usize>,
}

impl AuditStatistics {
    pub fn from_log(log: &AuditLog) -> Self {
        let mut stats = Self {
            total_events: log.event_count(),
            security_events: log.security_events().len(),
            critical_events: log.critical_events().len(),
            failed_events: log.failed_events().len(),
            events_by_type: HashMap::new(),
            events_by_severity: HashMap::new(),
        };

        // Count by type
        for event in &log.events {
            let type_name = event.event_type.to_string();
            *stats.events_by_type.entry(type_name).or_insert(0) += 1;

            let severity_name = event.severity.to_string();
            *stats.events_by_severity.entry(severity_name).or_insert(0) += 1;
        }

        stats
    }
}

/// Audit policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPolicy {
    pub name: String,
    pub enabled: bool,
    pub retention_days: u32,
    pub log_all_events: bool,
    pub log_security_events: bool,
    pub log_failed_events: bool,
    pub event_filters: Vec<EventFilter>,
}

impl AuditPolicy {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            retention_days: 90,
            log_all_events: false,
            log_security_events: true,
            log_failed_events: true,
            event_filters: Vec::new(),
        }
    }

    pub fn log_all(mut self) -> Self {
        self.log_all_events = true;
        self
    }

    pub fn with_retention(mut self, days: u32) -> Self {
        self.retention_days = days;
        self
    }

    pub fn add_filter(mut self, filter: EventFilter) -> Self {
        self.event_filters.push(filter);
        self
    }

    pub fn should_log(&self, event: &AuditEvent) -> bool {
        if !self.enabled {
            return false;
        }

        if self.log_all_events {
            return true;
        }

        if self.log_security_events && event.is_security_event() {
            return true;
        }

        if self.log_failed_events && event.outcome == EventOutcome::Failure {
            return true;
        }

        // Check filters
        for filter in &self.event_filters {
            if filter.matches(event) {
                return filter.action == FilterAction::Include;
            }
        }

        false
    }
}

impl Default for AuditPolicy {
    fn default() -> Self {
        Self::new("default")
    }
}

/// Event filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    pub event_type: Option<EventType>,
    pub severity: Option<EventSeverity>,
    pub actor: Option<String>,
    pub action: FilterAction,
}

impl EventFilter {
    pub fn new(action: FilterAction) -> Self {
        Self {
            event_type: None,
            severity: None,
            actor: None,
            action,
        }
    }

    pub fn for_type(mut self, event_type: EventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn for_severity(mut self, severity: EventSeverity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn for_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    pub fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(ref event_type) = self.event_type {
            if event_type != &event.event_type {
                return false;
            }
        }

        if let Some(ref severity) = self.severity {
            if severity != &event.severity {
                return false;
            }
        }

        if let Some(ref actor) = self.actor {
            if actor != &event.actor {
                return false;
            }
        }

        true
    }
}

/// Filter action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilterAction {
    Include,
    Exclude,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event() {
        let event = AuditEvent::new(
            EventType::Authentication,
            "user@example.com",
            "test-vm",
            "login",
        )
        .with_severity(EventSeverity::High)
        .with_source(EventSource::User)
        .with_ip("192.168.1.100")
        .add_detail("method", "ssh");

        assert_eq!(event.event_type, EventType::Authentication);
        assert_eq!(event.severity, EventSeverity::High);
        assert!(event.is_security_event());
        assert_eq!(event.details.get("method"), Some(&"ssh".to_string()));
    }

    #[test]
    fn test_audit_log() {
        let mut log = AuditLog::new(Some("test-vm".to_string()));

        log.add_event(AuditEvent::new(
            EventType::VMOperation,
            "admin",
            "test-vm",
            "start",
        ));
        log.add_event(
            AuditEvent::new(
                EventType::SecurityViolation,
                "user",
                "test-vm",
                "unauthorized",
            )
            .with_severity(EventSeverity::Critical),
        );
        log.add_event(
            AuditEvent::new(EventType::Configuration, "admin", "test-vm", "update")
                .with_outcome(EventOutcome::Failure),
        );

        assert_eq!(log.event_count(), 3);
        assert_eq!(log.security_events().len(), 1);
        assert_eq!(log.critical_events().len(), 1);
        assert_eq!(log.failed_events().len(), 1);
    }

    #[test]
    fn test_events_by_type() {
        let mut log = AuditLog::new(None);

        log.add_event(AuditEvent::new(
            EventType::VMOperation,
            "user",
            "vm1",
            "start",
        ));
        log.add_event(AuditEvent::new(
            EventType::VMOperation,
            "user",
            "vm2",
            "stop",
        ));
        log.add_event(AuditEvent::new(
            EventType::Authentication,
            "user",
            "system",
            "login",
        ));

        let vm_ops = log.events_by_type(&EventType::VMOperation);
        assert_eq!(vm_ops.len(), 2);
    }

    #[test]
    fn test_audit_statistics() {
        let mut log = AuditLog::new(None);

        log.add_event(
            AuditEvent::new(EventType::SecurityViolation, "user", "vm", "action")
                .with_severity(EventSeverity::Critical),
        );
        log.add_event(
            AuditEvent::new(EventType::VMOperation, "admin", "vm", "start")
                .with_severity(EventSeverity::Info),
        );

        let stats = AuditStatistics::from_log(&log);

        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.security_events, 1);
        assert_eq!(stats.critical_events, 1);
    }

    #[test]
    fn test_audit_policy() {
        let policy = AuditPolicy::new("test-policy")
            .log_all()
            .with_retention(180);

        assert!(policy.log_all_events);
        assert_eq!(policy.retention_days, 180);

        let event = AuditEvent::new(EventType::VMOperation, "user", "vm", "start");
        assert!(policy.should_log(&event));
    }

    #[test]
    fn test_policy_filtering() {
        let policy = AuditPolicy::new("filtered-policy").add_filter(
            EventFilter::new(FilterAction::Include).for_type(EventType::SecurityViolation),
        );

        let security_event =
            AuditEvent::new(EventType::SecurityViolation, "user", "vm", "violation");
        assert!(policy.should_log(&security_event));

        let normal_event = AuditEvent::new(EventType::VMOperation, "user", "vm", "start");
        assert!(!policy.should_log(&normal_event));
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter::new(FilterAction::Include)
            .for_type(EventType::Authentication)
            .for_severity(EventSeverity::High);

        let matching_event = AuditEvent::new(EventType::Authentication, "user", "vm", "login")
            .with_severity(EventSeverity::High);

        assert!(filter.matches(&matching_event));

        let non_matching = AuditEvent::new(EventType::VMOperation, "user", "vm", "start");

        assert!(!filter.matches(&non_matching));
    }

    #[test]
    fn test_security_event_detection() {
        let auth_event = AuditEvent::new(EventType::Authentication, "user", "system", "login");
        assert!(auth_event.is_security_event());

        let vm_event = AuditEvent::new(EventType::VMOperation, "user", "vm", "start");
        assert!(!vm_event.is_security_event());
    }

    #[test]
    fn test_critical_event_detection() {
        let critical_event = AuditEvent::new(EventType::SecurityViolation, "user", "vm", "breach")
            .with_severity(EventSeverity::Critical);

        assert!(critical_event.is_critical());

        let normal_event = AuditEvent::new(EventType::VMOperation, "user", "vm", "start");
        assert!(!normal_event.is_critical());
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::Authentication.to_string(), "Authentication");
        assert_eq!(
            EventType::SecurityViolation.to_string(),
            "Security Violation"
        );
    }

    #[test]
    fn test_severity_ordering() {
        assert!(EventSeverity::Critical > EventSeverity::High);
        assert!(EventSeverity::High > EventSeverity::Medium);
    }
}
