use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Access permission
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Delete,
    Rotate,
    Admin,
}

/// Access policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub id: String,
    pub name: String,
    pub principal: String,
    pub secret_id: String,
    pub permissions: Vec<Permission>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl AccessPolicy {
    pub fn new(
        name: impl Into<String>,
        principal: impl Into<String>,
        secret_id: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "policy-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp_micros()
        );

        Self {
            id,
            name: name_str,
            principal: principal.into(),
            secret_id: secret_id.into(),
            permissions: Vec::new(),
            expires_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn add_permission(&mut self, permission: Permission) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission) || self.permissions.contains(&Permission::Admin)
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expires_at {
            Utc::now() > expiry
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.permissions.is_empty()
    }
}

/// Access log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub id: String,
    pub secret_id: String,
    pub principal: String,
    pub action: String,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
}

impl AccessLogEntry {
    pub fn new(
        secret_id: impl Into<String>,
        principal: impl Into<String>,
        action: impl Into<String>,
        success: bool,
    ) -> Self {
        let id = format!("log-{}", Utc::now().timestamp_micros());

        Self {
            id,
            secret_id: secret_id.into(),
            principal: principal.into(),
            action: action.into(),
            success,
            timestamp: Utc::now(),
        }
    }
}

/// Access manager
pub struct AccessManager {
    policies: HashMap<String, AccessPolicy>,
    logs: Vec<AccessLogEntry>,
    max_log_entries: usize,
}

impl AccessManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            logs: Vec::new(),
            max_log_entries: 10_000,
        }
    }

    pub fn add_policy(&mut self, policy: AccessPolicy) -> String {
        let id = policy.id.clone();
        self.policies.insert(id.clone(), policy);
        id
    }

    pub fn get_policy(&self, id: &str) -> Option<&AccessPolicy> {
        self.policies.get(id)
    }

    pub fn get_policy_mut(&mut self, id: &str) -> Option<&mut AccessPolicy> {
        self.policies.get_mut(id)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn log_access(&mut self, entry: AccessLogEntry) {
        if self.logs.len() >= self.max_log_entries {
            let drain_count = self.max_log_entries / 10; // Remove oldest 10%
            self.logs.drain(..drain_count);
            log::warn!(
                "AccessManager logs exceeded max_log_entries ({}), drained {} oldest entries",
                self.max_log_entries,
                drain_count
            );
        }
        self.logs.push(entry);
    }

    pub fn log_count(&self) -> usize {
        self.logs.len()
    }

    pub fn policies_for_secret(&self, secret_id: &str) -> Vec<&AccessPolicy> {
        self.policies
            .values()
            .filter(|p| p.secret_id == secret_id)
            .collect()
    }

    pub fn policies_for_principal(&self, principal: &str) -> Vec<&AccessPolicy> {
        self.policies
            .values()
            .filter(|p| p.principal == principal)
            .collect()
    }

    pub fn valid_policies(&self) -> Vec<&AccessPolicy> {
        self.policies.values().filter(|p| p.is_valid()).collect()
    }

    pub fn logs_for_secret(&self, secret_id: &str) -> Vec<&AccessLogEntry> {
        self.logs
            .iter()
            .filter(|l| l.secret_id == secret_id)
            .collect()
    }

    pub fn failed_access_attempts(&self) -> Vec<&AccessLogEntry> {
        self.logs.iter().filter(|l| !l.success).collect()
    }
}

impl Default for AccessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_policy() {
        let policy = AccessPolicy::new("read-policy", "user-1", "secret-123");

        assert_eq!(policy.name, "read-policy");
        assert_eq!(policy.principal, "user-1");
        assert_eq!(policy.secret_id, "secret-123");
    }

    #[test]
    fn test_policy_with_expiry() {
        let expiry = Utc::now() + chrono::TimeDelta::days(30);
        let policy = AccessPolicy::new("temp-access", "user-1", "secret-1").with_expiry(expiry);

        assert_eq!(policy.expires_at, Some(expiry));
    }

    #[test]
    fn test_policy_add_permission() {
        let mut policy = AccessPolicy::new("policy", "user-1", "secret-1");

        policy.add_permission(Permission::Read);
        policy.add_permission(Permission::Write);
        policy.add_permission(Permission::Read); // Duplicate

        assert_eq!(policy.permissions.len(), 2);
    }

    #[test]
    fn test_policy_has_permission() {
        let mut policy = AccessPolicy::new("policy", "user-1", "secret-1");

        policy.add_permission(Permission::Read);

        assert!(policy.has_permission(&Permission::Read));
        assert!(!policy.has_permission(&Permission::Delete));
    }

    #[test]
    fn test_policy_admin_has_all_permissions() {
        let mut policy = AccessPolicy::new("admin-policy", "admin", "secret-1");

        policy.add_permission(Permission::Admin);

        assert!(policy.has_permission(&Permission::Read));
        assert!(policy.has_permission(&Permission::Write));
        assert!(policy.has_permission(&Permission::Delete));
    }

    #[test]
    fn test_policy_is_expired() {
        let past = Utc::now() - chrono::TimeDelta::days(1);
        let policy1 = AccessPolicy::new("expired", "user-1", "secret-1").with_expiry(past);
        assert!(policy1.is_expired());

        let future = Utc::now() + chrono::TimeDelta::days(30);
        let policy2 = AccessPolicy::new("active", "user-1", "secret-1").with_expiry(future);
        assert!(!policy2.is_expired());
    }

    #[test]
    fn test_policy_is_valid() {
        let mut policy1 = AccessPolicy::new("valid", "user-1", "secret-1");
        policy1.add_permission(Permission::Read);
        assert!(policy1.is_valid());

        let policy2 = AccessPolicy::new("invalid", "user-1", "secret-1");
        assert!(!policy2.is_valid()); // No permissions

        let past = Utc::now() - chrono::TimeDelta::days(1);
        let mut policy3 = AccessPolicy::new("expired", "user-1", "secret-1").with_expiry(past);
        policy3.add_permission(Permission::Read);
        assert!(!policy3.is_valid()); // Expired
    }

    #[test]
    fn test_access_log_entry() {
        let entry = AccessLogEntry::new("secret-123", "user-1", "read", true);

        assert_eq!(entry.secret_id, "secret-123");
        assert_eq!(entry.principal, "user-1");
        assert_eq!(entry.action, "read");
        assert!(entry.success);
    }

    #[test]
    fn test_access_manager() {
        let mut manager = AccessManager::new();

        let policy = AccessPolicy::new("test", "user-1", "secret-1");
        let id = manager.add_policy(policy);

        assert_eq!(manager.policy_count(), 1);
        assert!(manager.get_policy(&id).is_some());
    }

    #[test]
    fn test_manager_log_access() {
        let mut manager = AccessManager::new();

        let entry = AccessLogEntry::new("secret-1", "user-1", "read", true);
        manager.log_access(entry);

        assert_eq!(manager.log_count(), 1);
    }

    #[test]
    fn test_manager_policies_for_secret() {
        let mut manager = AccessManager::new();

        manager.add_policy(AccessPolicy::new("p1", "user-1", "secret-1"));
        manager.add_policy(AccessPolicy::new("p2", "user-2", "secret-2"));
        manager.add_policy(AccessPolicy::new("p3", "user-3", "secret-1"));

        let policies = manager.policies_for_secret("secret-1");
        assert_eq!(policies.len(), 2);
    }

    #[test]
    fn test_manager_policies_for_principal() {
        let mut manager = AccessManager::new();

        manager.add_policy(AccessPolicy::new("p1", "user-1", "secret-1"));
        manager.add_policy(AccessPolicy::new("p2", "user-2", "secret-2"));
        manager.add_policy(AccessPolicy::new("p3", "user-1", "secret-3"));

        let policies = manager.policies_for_principal("user-1");
        assert_eq!(policies.len(), 2);
    }

    #[test]
    fn test_manager_valid_policies() {
        let mut manager = AccessManager::new();

        let mut policy1 = AccessPolicy::new("p1", "user-1", "secret-1");
        policy1.add_permission(Permission::Read);

        let policy2 = AccessPolicy::new("p2", "user-2", "secret-2"); // No permissions

        manager.add_policy(policy1);
        manager.add_policy(policy2);

        let valid = manager.valid_policies();
        assert_eq!(valid.len(), 1);
    }

    #[test]
    fn test_manager_logs_for_secret() {
        let mut manager = AccessManager::new();

        manager.log_access(AccessLogEntry::new("secret-1", "user-1", "read", true));
        manager.log_access(AccessLogEntry::new("secret-2", "user-2", "write", true));
        manager.log_access(AccessLogEntry::new("secret-1", "user-3", "delete", false));

        let logs = manager.logs_for_secret("secret-1");
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_manager_failed_access_attempts() {
        let mut manager = AccessManager::new();

        manager.log_access(AccessLogEntry::new("secret-1", "user-1", "read", true));
        manager.log_access(AccessLogEntry::new("secret-2", "user-2", "write", false));
        manager.log_access(AccessLogEntry::new("secret-3", "user-3", "delete", false));

        let failed = manager.failed_access_attempts();
        assert_eq!(failed.len(), 2);
    }

    #[test]
    fn test_permission_equality() {
        assert_eq!(Permission::Read, Permission::Read);
        assert_ne!(Permission::Read, Permission::Write);
    }
}
