use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{ComplianceFramework, Severity};

/// Policy enforcement action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementAction {
    Allow,
    Deny,
    Warn,
    Audit,
}

impl std::fmt::Display for EnforcementAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnforcementAction::Allow => write!(f, "Allow"),
            EnforcementAction::Deny => write!(f, "Deny"),
            EnforcementAction::Warn => write!(f, "Warn"),
            EnforcementAction::Audit => write!(f, "Audit"),
        }
    }
}

/// Compliance policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub framework: ComplianceFramework,
    pub enabled: bool,
    pub enforcement_action: EnforcementAction,
    pub severity: Severity,
    pub rules: Vec<PolicyRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub condition: String,
    pub message: String,
}

impl CompliancePolicy {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        framework: ComplianceFramework,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "policy-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            description: description.into(),
            framework,
            enabled: true,
            enforcement_action: EnforcementAction::Warn,
            severity: Severity::Medium,
            rules: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_enforcement(mut self, action: EnforcementAction) -> Self {
        self.enforcement_action = action;
        self.updated_at = Utc::now();
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self.updated_at = Utc::now();
        self
    }

    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
        self.updated_at = Utc::now();
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl PolicyRule {
    pub fn new(
        name: impl Into<String>,
        condition: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            condition: condition.into(),
            message: message.into(),
        }
    }
}

/// Policy manager
pub struct PolicyManager {
    policies: HashMap<String, CompliancePolicy>,
}

impl PolicyManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    pub fn add_policy(&mut self, policy: CompliancePolicy) -> String {
        let id = policy.id.clone();
        self.policies.insert(id.clone(), policy);
        id
    }

    pub fn get_policy(&self, id: &str) -> Option<&CompliancePolicy> {
        self.policies.get(id)
    }

    pub fn get_policy_mut(&mut self, id: &str) -> Option<&mut CompliancePolicy> {
        self.policies.get_mut(id)
    }

    pub fn remove_policy(&mut self, id: &str) -> bool {
        self.policies.remove(id).is_some()
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn enabled_policies(&self) -> Vec<&CompliancePolicy> {
        self.policies.values().filter(|p| p.enabled).collect()
    }

    pub fn by_framework(&self, framework: &ComplianceFramework) -> Vec<&CompliancePolicy> {
        self.policies
            .values()
            .filter(|p| &p.framework == framework)
            .collect()
    }

    pub fn by_enforcement(&self, action: &EnforcementAction) -> Vec<&CompliancePolicy> {
        self.policies
            .values()
            .filter(|p| &p.enforcement_action == action)
            .collect()
    }
}

impl Default for PolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforcement_action_display() {
        assert_eq!(EnforcementAction::Allow.to_string(), "Allow");
        assert_eq!(EnforcementAction::Deny.to_string(), "Deny");
        assert_eq!(EnforcementAction::Warn.to_string(), "Warn");
    }

    #[test]
    fn test_compliance_policy() {
        let policy = CompliancePolicy::new(
            "Encryption Required",
            "All data must be encrypted at rest",
            ComplianceFramework::HIPAA,
        );

        assert_eq!(policy.name, "Encryption Required");
        assert_eq!(policy.framework, ComplianceFramework::HIPAA);
        assert!(policy.enabled);
        assert_eq!(policy.enforcement_action, EnforcementAction::Warn);
    }

    #[test]
    fn test_policy_builder() {
        let policy = CompliancePolicy::new("Test", "Description", ComplianceFramework::SOC2)
            .with_enforcement(EnforcementAction::Deny)
            .with_severity(Severity::Critical);

        assert_eq!(policy.enforcement_action, EnforcementAction::Deny);
        assert_eq!(policy.severity, Severity::Critical);
    }

    #[test]
    fn test_policy_enable_disable() {
        let mut policy = CompliancePolicy::new("Test", "Description", ComplianceFramework::GDPR);

        assert!(policy.enabled);

        policy.disable();
        assert!(!policy.enabled);

        policy.enable();
        assert!(policy.enabled);
    }

    #[test]
    fn test_policy_add_rules() {
        let mut policy = CompliancePolicy::new("Test", "Description", ComplianceFramework::PciDss);

        policy.add_rule(PolicyRule::new(
            "Encryption Check",
            "encryption.enabled == true",
            "Encryption must be enabled",
        ));

        policy.add_rule(PolicyRule::new(
            "Access Control",
            "mfa.enabled == true",
            "MFA must be enabled",
        ));

        assert_eq!(policy.rule_count(), 2);
    }

    #[test]
    fn test_policy_rule() {
        let rule = PolicyRule::new(
            "Encryption Check",
            "disk.encrypted == true",
            "Disk must be encrypted",
        );

        assert_eq!(rule.name, "Encryption Check");
        assert_eq!(rule.condition, "disk.encrypted == true");
        assert_eq!(rule.message, "Disk must be encrypted");
    }

    #[test]
    fn test_policy_manager() {
        let mut manager = PolicyManager::new();

        let policy = CompliancePolicy::new("Test", "Description", ComplianceFramework::SOC2);
        let id = manager.add_policy(policy);

        assert_eq!(manager.policy_count(), 1);
        assert!(manager.get_policy(&id).is_some());
    }

    #[test]
    fn test_manager_enabled_policies() {
        let mut manager = PolicyManager::new();

        let mut policy1 = CompliancePolicy::new("P1", "D1", ComplianceFramework::SOC2);
        policy1.enable();

        let mut policy2 = CompliancePolicy::new("P2", "D2", ComplianceFramework::HIPAA);
        policy2.disable();

        let policy3 = CompliancePolicy::new("P3", "D3", ComplianceFramework::GDPR);

        manager.add_policy(policy1);
        manager.add_policy(policy2);
        manager.add_policy(policy3);

        let enabled = manager.enabled_policies();
        assert_eq!(enabled.len(), 2);
    }

    #[test]
    fn test_manager_by_framework() {
        let mut manager = PolicyManager::new();

        manager.add_policy(CompliancePolicy::new("P1", "D1", ComplianceFramework::SOC2));
        manager.add_policy(CompliancePolicy::new(
            "P2",
            "D2",
            ComplianceFramework::HIPAA,
        ));
        manager.add_policy(CompliancePolicy::new("P3", "D3", ComplianceFramework::SOC2));

        let soc2 = manager.by_framework(&ComplianceFramework::SOC2);
        assert_eq!(soc2.len(), 2);
    }

    #[test]
    fn test_manager_by_enforcement() {
        let mut manager = PolicyManager::new();

        manager.add_policy(
            CompliancePolicy::new("P1", "D1", ComplianceFramework::SOC2)
                .with_enforcement(EnforcementAction::Deny),
        );
        manager.add_policy(
            CompliancePolicy::new("P2", "D2", ComplianceFramework::HIPAA)
                .with_enforcement(EnforcementAction::Warn),
        );
        manager.add_policy(
            CompliancePolicy::new("P3", "D3", ComplianceFramework::GDPR)
                .with_enforcement(EnforcementAction::Deny),
        );

        let deny = manager.by_enforcement(&EnforcementAction::Deny);
        assert_eq!(deny.len(), 2);
    }

    #[test]
    fn test_manager_remove_policy() {
        let mut manager = PolicyManager::new();

        let policy = CompliancePolicy::new("Test", "Description", ComplianceFramework::SOC2);
        let id = manager.add_policy(policy);

        assert!(manager.remove_policy(&id));
        assert_eq!(manager.policy_count(), 0);
    }

    #[test]
    fn test_manager_get_policy_mut() {
        let mut manager = PolicyManager::new();

        let policy = CompliancePolicy::new("Test", "Description", ComplianceFramework::SOC2);
        let id = manager.add_policy(policy);

        if let Some(policy_mut) = manager.get_policy_mut(&id) {
            policy_mut.disable();
        }

        let policy = manager.get_policy(&id).unwrap();
        assert!(!policy.enabled);
    }

    #[test]
    fn test_enforcement_action_equality() {
        assert_eq!(EnforcementAction::Deny, EnforcementAction::Deny);
        assert_ne!(EnforcementAction::Deny, EnforcementAction::Allow);
    }
}
