use super::NetworkProtocol;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Policy action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyAction {
    Allow,
    Deny,
    Log,
    Reject,
}

/// Traffic direction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficDirection {
    Ingress,
    Egress,
    Both,
}

/// Port range
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

impl PortRange {
    pub fn new(start: u16, end: u16) -> Self {
        // Ensure start <= end by swapping if needed
        if start > end {
            Self { start: end, end: start }
        } else {
            Self { start, end }
        }
    }

    pub fn single(port: u16) -> Self {
        Self {
            start: port,
            end: port,
        }
    }

    pub fn contains(&self, port: u16) -> bool {
        port >= self.start && port <= self.end
    }

    pub fn is_valid(&self) -> bool {
        self.start <= self.end
    }
}

/// Network policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyRule {
    pub id: String,
    pub name: String,
    pub action: PolicyAction,
    pub direction: TrafficDirection,
    pub protocol: NetworkProtocol,
    pub source_cidrs: Vec<String>,
    pub destination_cidrs: Vec<String>,
    pub source_ports: Vec<PortRange>,
    pub destination_ports: Vec<PortRange>,
    pub priority: u32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl NetworkPolicyRule {
    pub fn new(name: impl Into<String>, action: PolicyAction, direction: TrafficDirection) -> Self {
        let name_str = name.into();
        let id = format!(
            "rule-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            action,
            direction,
            protocol: NetworkProtocol::All,
            source_cidrs: Vec::new(),
            destination_cidrs: Vec::new(),
            source_ports: Vec::new(),
            destination_ports: Vec::new(),
            priority: 100,
            enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_protocol(mut self, protocol: NetworkProtocol) -> Self {
        self.protocol = protocol;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn add_source_cidr(&mut self, cidr: impl Into<String>) {
        self.source_cidrs.push(cidr.into());
    }

    pub fn add_destination_cidr(&mut self, cidr: impl Into<String>) {
        self.destination_cidrs.push(cidr.into());
    }

    pub fn add_source_port(&mut self, port_range: PortRange) {
        self.source_ports.push(port_range);
    }

    pub fn add_destination_port(&mut self, port_range: PortRange) {
        self.destination_ports.push(port_range);
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn is_high_priority(&self) -> bool {
        self.priority < 10
    }
}

/// Network policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub description: String,
    pub rules: Vec<String>,
    pub pod_selector: HashMap<String, String>,
    pub enforcement_mode: EnforcementMode,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementMode {
    Audit,
    Enforce,
    Disabled,
}

impl NetworkPolicy {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "policy-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            description: String::new(),
            rules: Vec::new(),
            pod_selector: HashMap::new(),
            enforcement_mode: EnforcementMode::Enforce,
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_enforcement_mode(mut self, mode: EnforcementMode) -> Self {
        self.enforcement_mode = mode;
        self
    }

    pub fn add_rule(&mut self, rule_id: impl Into<String>) {
        self.rules.push(rule_id.into());
    }

    pub fn add_selector(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.pod_selector.insert(key.into(), value.into());
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn is_enforcing(&self) -> bool {
        self.enforcement_mode == EnforcementMode::Enforce
    }
}

/// Security group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityGroup {
    pub id: String,
    pub name: String,
    pub description: String,
    pub vpc_id: Option<String>,
    pub inbound_rules: Vec<String>,
    pub outbound_rules: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl SecurityGroup {
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "sg-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            description: String::new(),
            vpc_id: None,
            inbound_rules: Vec::new(),
            outbound_rules: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_vpc(mut self, vpc_id: impl Into<String>) -> Self {
        self.vpc_id = Some(vpc_id.into());
        self
    }

    pub fn add_inbound_rule(&mut self, rule_id: impl Into<String>) {
        self.inbound_rules.push(rule_id.into());
    }

    pub fn add_outbound_rule(&mut self, rule_id: impl Into<String>) {
        self.outbound_rules.push(rule_id.into());
    }

    pub fn total_rules(&self) -> usize {
        self.inbound_rules.len() + self.outbound_rules.len()
    }
}

/// Policy manager
pub struct PolicyManager {
    rules: HashMap<String, NetworkPolicyRule>,
    policies: HashMap<String, NetworkPolicy>,
    security_groups: HashMap<String, SecurityGroup>,
}

impl PolicyManager {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            policies: HashMap::new(),
            security_groups: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, rule: NetworkPolicyRule) -> String {
        let id = rule.id.clone();
        self.rules.insert(id.clone(), rule);
        id
    }

    pub fn get_rule(&self, id: &str) -> Option<&NetworkPolicyRule> {
        self.rules.get(id)
    }

    pub fn get_rule_mut(&mut self, id: &str) -> Option<&mut NetworkPolicyRule> {
        self.rules.get_mut(id)
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn add_policy(&mut self, policy: NetworkPolicy) -> String {
        let id = policy.id.clone();
        self.policies.insert(id.clone(), policy);
        id
    }

    pub fn get_policy(&self, id: &str) -> Option<&NetworkPolicy> {
        self.policies.get(id)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn add_security_group(&mut self, sg: SecurityGroup) -> String {
        let id = sg.id.clone();
        self.security_groups.insert(id.clone(), sg);
        id
    }

    pub fn get_security_group(&self, id: &str) -> Option<&SecurityGroup> {
        self.security_groups.get(id)
    }

    pub fn security_group_count(&self) -> usize {
        self.security_groups.len()
    }

    pub fn enabled_rules(&self) -> Vec<&NetworkPolicyRule> {
        self.rules.values().filter(|r| r.enabled).collect()
    }

    pub fn high_priority_rules(&self) -> Vec<&NetworkPolicyRule> {
        self.rules
            .values()
            .filter(|r| r.is_high_priority())
            .collect()
    }

    pub fn rules_by_action(&self, action: &PolicyAction) -> Vec<&NetworkPolicyRule> {
        self.rules
            .values()
            .filter(|r| &r.action == action)
            .collect()
    }

    pub fn policies_by_namespace(&self, namespace: &str) -> Vec<&NetworkPolicy> {
        self.policies
            .values()
            .filter(|p| p.namespace == namespace)
            .collect()
    }

    pub fn enforcing_policies(&self) -> Vec<&NetworkPolicy> {
        self.policies
            .values()
            .filter(|p| p.is_enforcing())
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
    fn test_port_range() {
        let range = PortRange::new(80, 443);

        assert_eq!(range.start, 80);
        assert_eq!(range.end, 443);
    }

    #[test]
    fn test_port_range_single() {
        let range = PortRange::single(8080);

        assert_eq!(range.start, 8080);
        assert_eq!(range.end, 8080);
    }

    #[test]
    fn test_port_range_contains() {
        let range = PortRange::new(8000, 9000);

        assert!(range.contains(8000));
        assert!(range.contains(8500));
        assert!(range.contains(9000));
        assert!(!range.contains(7999));
        assert!(!range.contains(9001));
    }

    #[test]
    fn test_port_range_is_valid() {
        let range1 = PortRange::new(80, 443);
        assert!(range1.is_valid());

        // PortRange::new auto-swaps start > end, so result is always valid
        let range2 = PortRange::new(443, 80);
        assert!(range2.is_valid());
        assert_eq!(range2.start, 80);
        assert_eq!(range2.end, 443);
    }

    #[test]
    fn test_network_policy_rule() {
        let rule =
            NetworkPolicyRule::new("allow-http", PolicyAction::Allow, TrafficDirection::Ingress);

        assert_eq!(rule.name, "allow-http");
        assert_eq!(rule.action, PolicyAction::Allow);
        assert_eq!(rule.direction, TrafficDirection::Ingress);
        assert!(rule.enabled);
    }

    #[test]
    fn test_rule_with_protocol() {
        let rule = NetworkPolicyRule::new("tcp-rule", PolicyAction::Allow, TrafficDirection::Both)
            .with_protocol(NetworkProtocol::TCP);

        assert_eq!(rule.protocol, NetworkProtocol::TCP);
    }

    #[test]
    fn test_rule_with_priority() {
        let rule =
            NetworkPolicyRule::new("high-prio", PolicyAction::Deny, TrafficDirection::Egress)
                .with_priority(5);

        assert_eq!(rule.priority, 5);
    }

    #[test]
    fn test_rule_add_source_cidr() {
        let mut rule =
            NetworkPolicyRule::new("test", PolicyAction::Allow, TrafficDirection::Ingress);

        rule.add_source_cidr("10.0.0.0/8");
        rule.add_source_cidr("192.168.0.0/16");

        assert_eq!(rule.source_cidrs.len(), 2);
    }

    #[test]
    fn test_rule_add_destination_cidr() {
        let mut rule =
            NetworkPolicyRule::new("test", PolicyAction::Allow, TrafficDirection::Egress);

        rule.add_destination_cidr("0.0.0.0/0");

        assert_eq!(rule.destination_cidrs.len(), 1);
    }

    #[test]
    fn test_rule_add_ports() {
        let mut rule =
            NetworkPolicyRule::new("test", PolicyAction::Allow, TrafficDirection::Ingress);

        rule.add_source_port(PortRange::new(1024, 65535));
        rule.add_destination_port(PortRange::single(80));

        assert_eq!(rule.source_ports.len(), 1);
        assert_eq!(rule.destination_ports.len(), 1);
    }

    #[test]
    fn test_rule_disable_enable() {
        let mut rule =
            NetworkPolicyRule::new("test", PolicyAction::Allow, TrafficDirection::Ingress);

        assert!(rule.enabled);

        rule.disable();
        assert!(!rule.enabled);

        rule.enable();
        assert!(rule.enabled);
    }

    #[test]
    fn test_rule_is_high_priority() {
        let rule1 = NetworkPolicyRule::new("high", PolicyAction::Deny, TrafficDirection::Both)
            .with_priority(5);
        assert!(rule1.is_high_priority());

        let rule2 = NetworkPolicyRule::new("low", PolicyAction::Allow, TrafficDirection::Both)
            .with_priority(100);
        assert!(!rule2.is_high_priority());
    }

    #[test]
    fn test_network_policy() {
        let policy = NetworkPolicy::new("web-policy", "default");

        assert_eq!(policy.name, "web-policy");
        assert_eq!(policy.namespace, "default");
        assert_eq!(policy.enforcement_mode, EnforcementMode::Enforce);
    }

    #[test]
    fn test_policy_with_description() {
        let policy = NetworkPolicy::new("test", "default").with_description("Test policy");

        assert_eq!(policy.description, "Test policy");
    }

    #[test]
    fn test_policy_with_enforcement_mode() {
        let policy =
            NetworkPolicy::new("test", "default").with_enforcement_mode(EnforcementMode::Audit);

        assert_eq!(policy.enforcement_mode, EnforcementMode::Audit);
    }

    #[test]
    fn test_policy_add_rule() {
        let mut policy = NetworkPolicy::new("test", "default");

        policy.add_rule("rule-1");
        policy.add_rule("rule-2");

        assert_eq!(policy.rule_count(), 2);
    }

    #[test]
    fn test_policy_add_selector() {
        let mut policy = NetworkPolicy::new("test", "default");

        policy.add_selector("app", "web");
        policy.add_selector("tier", "frontend");

        assert_eq!(policy.pod_selector.len(), 2);
    }

    #[test]
    fn test_policy_is_enforcing() {
        let policy1 = NetworkPolicy::new("test1", "default");
        assert!(policy1.is_enforcing());

        let policy2 =
            NetworkPolicy::new("test2", "default").with_enforcement_mode(EnforcementMode::Audit);
        assert!(!policy2.is_enforcing());
    }

    #[test]
    fn test_security_group() {
        let sg = SecurityGroup::new("web-sg");

        assert_eq!(sg.name, "web-sg");
        assert!(sg.vpc_id.is_none());
    }

    #[test]
    fn test_sg_with_description() {
        let sg = SecurityGroup::new("test-sg").with_description("Test security group");

        assert_eq!(sg.description, "Test security group");
    }

    #[test]
    fn test_sg_with_vpc() {
        let sg = SecurityGroup::new("test-sg").with_vpc("vpc-12345");

        assert_eq!(sg.vpc_id, Some("vpc-12345".to_string()));
    }

    #[test]
    fn test_sg_add_rules() {
        let mut sg = SecurityGroup::new("test-sg");

        sg.add_inbound_rule("rule-1");
        sg.add_inbound_rule("rule-2");
        sg.add_outbound_rule("rule-3");

        assert_eq!(sg.inbound_rules.len(), 2);
        assert_eq!(sg.outbound_rules.len(), 1);
        assert_eq!(sg.total_rules(), 3);
    }

    #[test]
    fn test_policy_manager() {
        let mut manager = PolicyManager::new();

        let rule = NetworkPolicyRule::new("test", PolicyAction::Allow, TrafficDirection::Ingress);
        let id = manager.add_rule(rule);

        assert_eq!(manager.rule_count(), 1);
        assert!(manager.get_rule(&id).is_some());
    }

    #[test]
    fn test_manager_add_policy() {
        let mut manager = PolicyManager::new();

        let policy = NetworkPolicy::new("test", "default");
        let id = manager.add_policy(policy);

        assert_eq!(manager.policy_count(), 1);
        assert!(manager.get_policy(&id).is_some());
    }

    #[test]
    fn test_manager_add_security_group() {
        let mut manager = PolicyManager::new();

        let sg = SecurityGroup::new("test-sg");
        let id = manager.add_security_group(sg);

        assert_eq!(manager.security_group_count(), 1);
        assert!(manager.get_security_group(&id).is_some());
    }

    #[test]
    fn test_manager_enabled_rules() {
        let mut manager = PolicyManager::new();

        let rule1 = NetworkPolicyRule::new("r1", PolicyAction::Allow, TrafficDirection::Ingress);
        let mut rule2 = NetworkPolicyRule::new("r2", PolicyAction::Deny, TrafficDirection::Egress);
        rule2.disable();

        manager.add_rule(rule1);
        manager.add_rule(rule2);

        let enabled = manager.enabled_rules();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_manager_high_priority_rules() {
        let mut manager = PolicyManager::new();

        manager.add_rule(
            NetworkPolicyRule::new("r1", PolicyAction::Deny, TrafficDirection::Both)
                .with_priority(5),
        );
        manager.add_rule(
            NetworkPolicyRule::new("r2", PolicyAction::Allow, TrafficDirection::Ingress)
                .with_priority(100),
        );

        let high_prio = manager.high_priority_rules();
        assert_eq!(high_prio.len(), 1);
    }

    #[test]
    fn test_manager_rules_by_action() {
        let mut manager = PolicyManager::new();

        manager.add_rule(NetworkPolicyRule::new(
            "r1",
            PolicyAction::Allow,
            TrafficDirection::Ingress,
        ));
        manager.add_rule(NetworkPolicyRule::new(
            "r2",
            PolicyAction::Deny,
            TrafficDirection::Egress,
        ));
        manager.add_rule(NetworkPolicyRule::new(
            "r3",
            PolicyAction::Allow,
            TrafficDirection::Both,
        ));

        let allow_rules = manager.rules_by_action(&PolicyAction::Allow);
        assert_eq!(allow_rules.len(), 2);
    }

    #[test]
    fn test_manager_policies_by_namespace() {
        let mut manager = PolicyManager::new();

        manager.add_policy(NetworkPolicy::new("p1", "default"));
        manager.add_policy(NetworkPolicy::new("p2", "kube-system"));
        manager.add_policy(NetworkPolicy::new("p3", "default"));

        let default_policies = manager.policies_by_namespace("default");
        assert_eq!(default_policies.len(), 2);
    }

    #[test]
    fn test_manager_enforcing_policies() {
        let mut manager = PolicyManager::new();

        manager.add_policy(NetworkPolicy::new("p1", "default"));
        manager.add_policy(
            NetworkPolicy::new("p2", "default").with_enforcement_mode(EnforcementMode::Audit),
        );

        let enforcing = manager.enforcing_policies();
        assert_eq!(enforcing.len(), 1);
    }

    #[test]
    fn test_policy_action_equality() {
        assert_eq!(PolicyAction::Allow, PolicyAction::Allow);
        assert_ne!(PolicyAction::Allow, PolicyAction::Deny);
    }

    #[test]
    fn test_traffic_direction_equality() {
        assert_eq!(TrafficDirection::Ingress, TrafficDirection::Ingress);
        assert_ne!(TrafficDirection::Ingress, TrafficDirection::Egress);
    }

    #[test]
    fn test_enforcement_mode_equality() {
        assert_eq!(EnforcementMode::Enforce, EnforcementMode::Enforce);
        assert_ne!(EnforcementMode::Enforce, EnforcementMode::Audit);
    }
}
