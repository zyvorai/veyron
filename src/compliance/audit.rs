use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Audit event type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    ResourceCreated,
    ResourceModified,
    ResourceDeleted,
    AccessGranted,
    AccessDenied,
    ConfigurationChanged,
    PolicyViolation,
    SecurityIncident,
    ComplianceCheck,
    Custom(String),
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditEventType::ResourceCreated => write!(f, "Resource Created"),
            AuditEventType::ResourceModified => write!(f, "Resource Modified"),
            AuditEventType::ResourceDeleted => write!(f, "Resource Deleted"),
            AuditEventType::AccessGranted => write!(f, "Access Granted"),
            AuditEventType::AccessDenied => write!(f, "Access Denied"),
            AuditEventType::ConfigurationChanged => write!(f, "Configuration Changed"),
            AuditEventType::PolicyViolation => write!(f, "Policy Violation"),
            AuditEventType::SecurityIncident => write!(f, "Security Incident"),
            AuditEventType::ComplianceCheck => write!(f, "Compliance Check"),
            AuditEventType::Custom(name) => write!(f, "Custom: {}", name),
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user: String,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub result: ActionResult,
    pub source_ip: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionResult {
    Success,
    Failure,
    PartialSuccess,
}

impl std::fmt::Display for ActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionResult::Success => write!(f, "Success"),
            ActionResult::Failure => write!(f, "Failure"),
            ActionResult::PartialSuccess => write!(f, "Partial Success"),
        }
    }
}

impl AuditLogEntry {
    pub fn new(
        event_type: AuditEventType,
        user: impl Into<String>,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        let id = format!("audit-{}", Utc::now().timestamp_micros());

        Self {
            id,
            timestamp: Utc::now(),
            event_type,
            user: user.into(),
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            action: action.into(),
            result: ActionResult::Success,
            source_ip: String::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_result(mut self, result: ActionResult) -> Self {
        self.result = result;
        self
    }

    pub fn with_source_ip(mut self, ip: impl Into<String>) -> Self {
        self.source_ip = ip.into();
        self
    }

    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    pub fn is_successful(&self) -> bool {
        self.result == ActionResult::Success
    }

    pub fn is_failed(&self) -> bool {
        self.result == ActionResult::Failure
    }
}

/// Audit trail
pub struct AuditTrail {
    entries: Vec<AuditLogEntry>,
    retention_days: i64,
    max_entries: usize,
}

impl AuditTrail {
    pub fn new(retention_days: i64) -> Self {
        Self {
            entries: Vec::new(),
            retention_days,
            max_entries: 10_000,
        }
    }

    pub fn log(&mut self, entry: AuditLogEntry) {
        if self.entries.len() >= self.max_entries {
            let drain_count = self.max_entries / 10; // Remove oldest 10%
            self.entries.drain(..drain_count);
            log::warn!(
                "AuditTrail exceeded max_entries ({}), drained {} oldest entries",
                self.max_entries,
                drain_count
            );
        }
        self.entries.push(entry);
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn get_entries(&self) -> &[AuditLogEntry] {
        &self.entries
    }

    pub fn by_user(&self, user: &str) -> Vec<&AuditLogEntry> {
        self.entries.iter().filter(|e| e.user == user).collect()
    }

    pub fn by_resource(&self, resource_id: &str) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.resource_id == resource_id)
            .collect()
    }

    pub fn by_event_type(&self, event_type: &AuditEventType) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| &e.event_type == event_type)
            .collect()
    }

    pub fn failed_actions(&self) -> Vec<&AuditLogEntry> {
        self.entries.iter().filter(|e| e.is_failed()).collect()
    }

    pub fn security_incidents(&self) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.event_type == AuditEventType::SecurityIncident)
            .collect()
    }

    pub fn policy_violations(&self) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.event_type == AuditEventType::PolicyViolation)
            .collect()
    }

    pub fn purge_old_entries(&mut self) {
        let cutoff = Utc::now() - chrono::TimeDelta::days(self.retention_days);
        self.entries.retain(|e| e.timestamp > cutoff);
    }

    pub fn entries_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .collect()
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new(90) // 90 days retention by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_type_display() {
        assert_eq!(
            AuditEventType::ResourceCreated.to_string(),
            "Resource Created"
        );
        assert_eq!(AuditEventType::AccessDenied.to_string(), "Access Denied");
        assert_eq!(
            AuditEventType::Custom("MyEvent".to_string()).to_string(),
            "Custom: MyEvent"
        );
    }

    #[test]
    fn test_action_result_display() {
        assert_eq!(ActionResult::Success.to_string(), "Success");
        assert_eq!(ActionResult::Failure.to_string(), "Failure");
        assert_eq!(ActionResult::PartialSuccess.to_string(), "Partial Success");
    }

    #[test]
    fn test_audit_log_entry() {
        let entry = AuditLogEntry::new(
            AuditEventType::ResourceCreated,
            "admin@example.com",
            "VirtualMachine",
            "vm-123",
            "create",
        );

        assert_eq!(entry.event_type, AuditEventType::ResourceCreated);
        assert_eq!(entry.user, "admin@example.com");
        assert_eq!(entry.resource_id, "vm-123");
        assert!(entry.is_successful());
        assert!(!entry.is_failed());
    }

    #[test]
    fn test_entry_builder() {
        let entry = AuditLogEntry::new(
            AuditEventType::AccessDenied,
            "user@example.com",
            "VM",
            "vm-456",
            "delete",
        )
        .with_result(ActionResult::Failure)
        .with_source_ip("192.168.1.100");

        assert_eq!(entry.result, ActionResult::Failure);
        assert_eq!(entry.source_ip, "192.168.1.100");
        assert!(entry.is_failed());
    }

    #[test]
    fn test_entry_metadata() {
        let mut entry = AuditLogEntry::new(
            AuditEventType::ConfigurationChanged,
            "admin",
            "Config",
            "cfg-1",
            "update",
        );

        entry.add_metadata("old_value", "value1");
        entry.add_metadata("new_value", "value2");

        assert_eq!(entry.metadata.len(), 2);
        assert_eq!(entry.metadata.get("old_value"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_audit_trail() {
        let trail = AuditTrail::new(90);

        assert_eq!(trail.entry_count(), 0);
        assert_eq!(trail.retention_days, 90);
    }

    #[test]
    fn test_trail_log() {
        let mut trail = AuditTrail::new(90);

        trail.log(AuditLogEntry::new(
            AuditEventType::ResourceCreated,
            "user1",
            "VM",
            "vm-1",
            "create",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::ResourceDeleted,
            "user2",
            "VM",
            "vm-2",
            "delete",
        ));

        assert_eq!(trail.entry_count(), 2);
    }

    #[test]
    fn test_trail_by_user() {
        let mut trail = AuditTrail::new(90);

        trail.log(AuditLogEntry::new(
            AuditEventType::ResourceCreated,
            "alice",
            "VM",
            "vm-1",
            "create",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::ResourceModified,
            "bob",
            "VM",
            "vm-2",
            "modify",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::ResourceDeleted,
            "alice",
            "VM",
            "vm-3",
            "delete",
        ));

        let alice_entries = trail.by_user("alice");
        assert_eq!(alice_entries.len(), 2);
    }

    #[test]
    fn test_trail_by_resource() {
        let mut trail = AuditTrail::new(90);

        trail.log(AuditLogEntry::new(
            AuditEventType::ResourceCreated,
            "user1",
            "VM",
            "vm-123",
            "create",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::ResourceModified,
            "user2",
            "VM",
            "vm-123",
            "modify",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::ResourceDeleted,
            "user3",
            "VM",
            "vm-456",
            "delete",
        ));

        let vm123_entries = trail.by_resource("vm-123");
        assert_eq!(vm123_entries.len(), 2);
    }

    #[test]
    fn test_trail_by_event_type() {
        let mut trail = AuditTrail::new(90);

        trail.log(AuditLogEntry::new(
            AuditEventType::SecurityIncident,
            "user1",
            "VM",
            "vm-1",
            "action",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::PolicyViolation,
            "user2",
            "VM",
            "vm-2",
            "action",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::SecurityIncident,
            "user3",
            "VM",
            "vm-3",
            "action",
        ));

        let incidents = trail.by_event_type(&AuditEventType::SecurityIncident);
        assert_eq!(incidents.len(), 2);
    }

    #[test]
    fn test_trail_failed_actions() {
        let mut trail = AuditTrail::new(90);

        trail.log(
            AuditLogEntry::new(
                AuditEventType::AccessGranted,
                "user1",
                "VM",
                "vm-1",
                "access",
            )
            .with_result(ActionResult::Success),
        );

        trail.log(
            AuditLogEntry::new(
                AuditEventType::AccessDenied,
                "user2",
                "VM",
                "vm-2",
                "access",
            )
            .with_result(ActionResult::Failure),
        );

        trail.log(
            AuditLogEntry::new(
                AuditEventType::ResourceDeleted,
                "user3",
                "VM",
                "vm-3",
                "delete",
            )
            .with_result(ActionResult::Failure),
        );

        let failed = trail.failed_actions();
        assert_eq!(failed.len(), 2);
    }

    #[test]
    fn test_trail_security_incidents() {
        let mut trail = AuditTrail::new(90);

        trail.log(AuditLogEntry::new(
            AuditEventType::SecurityIncident,
            "user1",
            "VM",
            "vm-1",
            "action",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::PolicyViolation,
            "user2",
            "VM",
            "vm-2",
            "action",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::SecurityIncident,
            "user3",
            "VM",
            "vm-3",
            "action",
        ));

        let incidents = trail.security_incidents();
        assert_eq!(incidents.len(), 2);
    }

    #[test]
    fn test_trail_policy_violations() {
        let mut trail = AuditTrail::new(90);

        trail.log(AuditLogEntry::new(
            AuditEventType::PolicyViolation,
            "user1",
            "VM",
            "vm-1",
            "action",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::SecurityIncident,
            "user2",
            "VM",
            "vm-2",
            "action",
        ));

        trail.log(AuditLogEntry::new(
            AuditEventType::PolicyViolation,
            "user3",
            "VM",
            "vm-3",
            "action",
        ));

        let violations = trail.policy_violations();
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn test_trail_purge_old_entries() {
        let mut trail = AuditTrail::new(90);

        let mut old_entry = AuditLogEntry::new(
            AuditEventType::ResourceCreated,
            "user1",
            "VM",
            "vm-1",
            "create",
        );
        old_entry.timestamp = Utc::now() - chrono::TimeDelta::days(100);

        let recent_entry = AuditLogEntry::new(
            AuditEventType::ResourceModified,
            "user2",
            "VM",
            "vm-2",
            "modify",
        );

        trail.log(old_entry);
        trail.log(recent_entry);

        assert_eq!(trail.entry_count(), 2);

        trail.purge_old_entries();

        assert_eq!(trail.entry_count(), 1);
    }

    #[test]
    fn test_trail_entries_in_range() {
        let mut trail = AuditTrail::new(90);

        let start = Utc::now() - chrono::TimeDelta::hours(2);
        let end = Utc::now();

        let mut old_entry = AuditLogEntry::new(
            AuditEventType::ResourceCreated,
            "user1",
            "VM",
            "vm-1",
            "create",
        );
        old_entry.timestamp = start - chrono::TimeDelta::hours(1);

        let mut in_range_entry1 = AuditLogEntry::new(
            AuditEventType::ResourceModified,
            "user2",
            "VM",
            "vm-2",
            "modify",
        );
        in_range_entry1.timestamp = start + chrono::TimeDelta::minutes(30);

        let mut in_range_entry2 = AuditLogEntry::new(
            AuditEventType::ResourceDeleted,
            "user3",
            "VM",
            "vm-3",
            "delete",
        );
        in_range_entry2.timestamp = start + chrono::TimeDelta::hours(1);

        trail.log(old_entry);
        trail.log(in_range_entry1);
        trail.log(in_range_entry2);

        let in_range = trail.entries_in_range(start, end);
        assert_eq!(in_range.len(), 2);
    }

    #[test]
    fn test_trail_default() {
        let trail = AuditTrail::default();

        assert_eq!(trail.retention_days, 90);
        assert_eq!(trail.entry_count(), 0);
    }

    #[test]
    fn test_audit_event_type_equality() {
        assert_eq!(
            AuditEventType::ResourceCreated,
            AuditEventType::ResourceCreated
        );
        assert_ne!(
            AuditEventType::ResourceCreated,
            AuditEventType::ResourceModified
        );
    }

    #[test]
    fn test_action_result_equality() {
        assert_eq!(ActionResult::Success, ActionResult::Success);
        assert_ne!(ActionResult::Success, ActionResult::Failure);
    }
}
