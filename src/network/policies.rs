// Network Policies - Network policy management and security

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Check if an IP address matches a CIDR notation string.
/// Supports: "*" (match all), "10.0.0.5" or "::1" (exact match),
/// "10.0.0.0/24" (IPv4 CIDR range), "2001:db8::/32" (IPv6 CIDR range).
fn ip_matches_cidr(ip: &str, cidr: &str) -> bool {
    if cidr == "*" {
        return true;
    }

    let ip_addr: std::net::IpAddr = match ip.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };

    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() == 1 {
        // Exact IP match
        return ip == cidr;
    }
    if parts.len() != 2 {
        return false;
    }

    let cidr_addr: std::net::IpAddr = match parts[0].parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let prefix_len: u32 = match parts[1].parse() {
        Ok(p) => p,
        Err(_) => return false,
    };

    match (ip_addr, cidr_addr) {
        (std::net::IpAddr::V4(ip4), std::net::IpAddr::V4(cidr4)) => {
            let ip_bits = u32::from(ip4);
            let cidr_bits = u32::from(cidr4);
            if prefix_len > 32 {
                return false;
            }
            if prefix_len == 0 {
                return true;
            }
            let mask = !0u32 << (32 - prefix_len);
            (ip_bits & mask) == (cidr_bits & mask)
        }
        (std::net::IpAddr::V6(ip6), std::net::IpAddr::V6(cidr6)) => {
            let ip_bits = u128::from(ip6);
            let cidr_bits = u128::from(cidr6);
            if prefix_len > 128 {
                return false;
            }
            if prefix_len == 0 {
                return true;
            }
            let mask = !0u128 << (128 - prefix_len);
            (ip_bits & mask) == (cidr_bits & mask)
        }
        _ => false, // Mismatched address families
    }
}

/// Network policy for VM traffic control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub name: String,
    pub vm_selector: VMSelector,
    pub ingress_rules: Vec<IngressRule>,
    pub egress_rules: Vec<EgressRule>,
    pub policy_type: PolicyType,
}

impl NetworkPolicy {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vm_selector: VMSelector::default(),
            ingress_rules: Vec::new(),
            egress_rules: Vec::new(),
            policy_type: PolicyType::Both,
        }
    }

    pub fn with_vm_selector(mut self, selector: VMSelector) -> Self {
        self.vm_selector = selector;
        self
    }

    pub fn add_ingress_rule(mut self, rule: IngressRule) -> Self {
        self.ingress_rules.push(rule);
        self
    }

    pub fn add_egress_rule(mut self, rule: EgressRule) -> Self {
        self.egress_rules.push(rule);
        self
    }

    pub fn with_policy_type(mut self, policy_type: PolicyType) -> Self {
        self.policy_type = policy_type;
        self
    }

    /// Check if policy allows ingress traffic
    pub fn allows_ingress(&self, source: &str, port: u16, protocol: &str) -> bool {
        if self.ingress_rules.is_empty() {
            return false; // No ingress rules = deny all ingress
        }

        self.ingress_rules
            .iter()
            .any(|rule| rule.matches(source, port, protocol))
    }

    /// Check if policy allows egress traffic
    pub fn allows_egress(&self, dest: &str, port: u16, protocol: &str) -> bool {
        if self.egress_rules.is_empty() {
            return false; // No egress rules = deny all egress
        }

        self.egress_rules
            .iter()
            .any(|rule| rule.matches(dest, port, protocol))
    }
}

/// VM selector for policy targeting
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VMSelector {
    pub labels: HashMap<String, String>,
}

impl VMSelector {
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn matches(&self, vm_labels: &HashMap<String, String>) -> bool {
        self.labels
            .iter()
            .all(|(k, v)| vm_labels.get(k).map(|val| val == v).unwrap_or(false))
    }
}

/// Policy type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyType {
    Ingress,
    Egress,
    Both,
}

/// Ingress rule for incoming traffic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRule {
    pub from_cidrs: Vec<String>,
    pub ports: Vec<Port>,
    pub description: Option<String>,
}

impl IngressRule {
    pub fn new() -> Self {
        Self {
            from_cidrs: Vec::new(),
            ports: Vec::new(),
            description: None,
        }
    }

    pub fn from_cidr(mut self, cidr: impl Into<String>) -> Self {
        self.from_cidrs.push(cidr.into());
        self
    }

    pub fn allow_port(mut self, port: Port) -> Self {
        self.ports.push(port);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Check if rule matches traffic
    pub fn matches(&self, source: &str, port: u16, protocol: &str) -> bool {
        let cidr_match = self.from_cidrs.is_empty()
            || self
                .from_cidrs
                .iter()
                .any(|cidr| ip_matches_cidr(source, cidr));

        let port_match =
            self.ports.is_empty() || self.ports.iter().any(|p| p.matches(port, protocol));

        cidr_match && port_match
    }
}

impl Default for IngressRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Egress rule for outgoing traffic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressRule {
    pub to_cidrs: Vec<String>,
    pub ports: Vec<Port>,
    pub description: Option<String>,
}

impl EgressRule {
    pub fn new() -> Self {
        Self {
            to_cidrs: Vec::new(),
            ports: Vec::new(),
            description: None,
        }
    }

    pub fn to_cidr(mut self, cidr: impl Into<String>) -> Self {
        self.to_cidrs.push(cidr.into());
        self
    }

    pub fn allow_port(mut self, port: Port) -> Self {
        self.ports.push(port);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Check if rule matches traffic
    pub fn matches(&self, dest: &str, port: u16, protocol: &str) -> bool {
        let cidr_match = self.to_cidrs.is_empty()
            || self.to_cidrs.iter().any(|cidr| ip_matches_cidr(dest, cidr));

        let port_match =
            self.ports.is_empty() || self.ports.iter().any(|p| p.matches(port, protocol));

        cidr_match && port_match
    }
}

impl Default for EgressRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Port specification for rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub port: u16,
    pub protocol: Protocol,
    pub end_port: Option<u16>, // For port ranges
}

impl Port {
    pub fn tcp(port: u16) -> Self {
        Self {
            port,
            protocol: Protocol::TCP,
            end_port: None,
        }
    }

    pub fn udp(port: u16) -> Self {
        Self {
            port,
            protocol: Protocol::UDP,
            end_port: None,
        }
    }

    pub fn range(start: u16, end: u16, protocol: Protocol) -> Self {
        Self {
            port: start,
            protocol,
            end_port: Some(end),
        }
    }

    /// Check if port matches
    pub fn matches(&self, port: u16, protocol: &str) -> bool {
        let proto_match = self.protocol == Protocol::Any
            || protocol.to_uppercase() == self.protocol.as_str();

        let port_match = if let Some(end) = self.end_port {
            port >= self.port && port <= end
        } else {
            port == self.port
        };

        proto_match && port_match
    }
}

/// Protocol for port rules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Protocol {
    TCP,
    UDP,
    Any,
}

impl Protocol {
    pub fn as_str(&self) -> &str {
        match self {
            Protocol::TCP => "TCP",
            Protocol::UDP => "UDP",
            Protocol::Any => "ANY",
        }
    }
}

/// Network policy manager
pub struct PolicyManager {
    policies: HashMap<String, NetworkPolicy>,
}

impl PolicyManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    pub fn add_policy(&mut self, policy: NetworkPolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    pub fn remove_policy(&mut self, name: &str) -> bool {
        self.policies.remove(name).is_some()
    }

    pub fn get_policy(&self, name: &str) -> Option<&NetworkPolicy> {
        self.policies.get(name)
    }

    pub fn list_policies(&self) -> Vec<&NetworkPolicy> {
        self.policies.values().collect()
    }

    /// Find policies that apply to a VM
    pub fn find_policies_for_vm(&self, vm_labels: &HashMap<String, String>) -> Vec<&NetworkPolicy> {
        self.policies
            .values()
            .filter(|p| p.vm_selector.matches(vm_labels))
            .collect()
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
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
    fn test_network_policy_creation() {
        let policy = NetworkPolicy::new("web-policy").with_policy_type(PolicyType::Both);

        assert_eq!(policy.name, "web-policy");
        assert_eq!(policy.policy_type, PolicyType::Both);
    }

    #[test]
    fn test_vm_selector() {
        let selector = VMSelector::default()
            .with_label("app", "web")
            .with_label("tier", "frontend");

        let mut vm_labels = HashMap::new();
        vm_labels.insert("app".to_string(), "web".to_string());
        vm_labels.insert("tier".to_string(), "frontend".to_string());

        assert!(selector.matches(&vm_labels));

        let mut other_labels = HashMap::new();
        other_labels.insert("app".to_string(), "database".to_string());
        assert!(!selector.matches(&other_labels));
    }

    #[test]
    fn test_ingress_rule() {
        let rule = IngressRule::new()
            .from_cidr("10.0.0.0/24")
            .allow_port(Port::tcp(80))
            .with_description("Allow HTTP");

        assert!(rule.matches("10.0.0.5", 80, "TCP"));
        assert!(!rule.matches("10.0.0.5", 443, "TCP"));
        assert!(!rule.matches("192.168.1.1", 80, "TCP"));
    }

    #[test]
    fn test_egress_rule() {
        let rule = EgressRule::new()
            .to_cidr("*")
            .allow_port(Port::tcp(443))
            .with_description("Allow HTTPS");

        assert!(rule.matches("8.8.8.8", 443, "TCP"));
        assert!(!rule.matches("8.8.8.8", 80, "TCP"));
    }

    #[test]
    fn test_port_matching() {
        let tcp_port = Port::tcp(80);
        assert!(tcp_port.matches(80, "TCP"));
        assert!(!tcp_port.matches(80, "UDP"));
        assert!(!tcp_port.matches(443, "TCP"));

        let port_range = Port::range(8000, 9000, Protocol::TCP);
        assert!(port_range.matches(8080, "TCP"));
        assert!(port_range.matches(8000, "TCP"));
        assert!(port_range.matches(9000, "TCP"));
        assert!(!port_range.matches(7999, "TCP"));
        assert!(!port_range.matches(9001, "TCP"));
    }

    #[test]
    fn test_policy_allows_traffic() {
        let mut policy = NetworkPolicy::new("test-policy");

        let ingress = IngressRule::new()
            .from_cidr("10.0.0.0/24")
            .allow_port(Port::tcp(80));

        policy.ingress_rules.push(ingress);

        assert!(policy.allows_ingress("10.0.0.5", 80, "TCP"));
        assert!(!policy.allows_ingress("10.0.0.5", 443, "TCP"));
    }

    #[test]
    fn test_policy_manager() {
        let mut manager = PolicyManager::new();

        let policy = NetworkPolicy::new("web-policy");
        manager.add_policy(policy);

        assert_eq!(manager.policy_count(), 1);
        assert!(manager.get_policy("web-policy").is_some());

        assert!(manager.remove_policy("web-policy"));
        assert_eq!(manager.policy_count(), 0);
    }

    #[test]
    fn test_ipv6_cidr_matching() {
        let rule = IngressRule::new()
            .from_cidr("2001:db8::/32")
            .allow_port(Port::tcp(80));

        assert!(rule.matches("2001:db8::1", 80, "TCP"));
        assert!(rule.matches("2001:db8:1::1", 80, "TCP"));
        assert!(!rule.matches("2001:db9::1", 80, "TCP"));
        // Mismatched address families should not match
        assert!(!rule.matches("10.0.0.1", 80, "TCP"));
    }

    #[test]
    fn test_ipv6_exact_match() {
        let rule = IngressRule::new()
            .from_cidr("::1")
            .allow_port(Port::tcp(80));

        assert!(rule.matches("::1", 80, "TCP"));
        assert!(!rule.matches("::2", 80, "TCP"));
    }

    #[test]
    fn test_find_policies_for_vm() {
        let mut manager = PolicyManager::new();

        let selector = VMSelector::default().with_label("app", "web");
        let policy = NetworkPolicy::new("web-policy").with_vm_selector(selector);
        manager.add_policy(policy);

        let mut vm_labels = HashMap::new();
        vm_labels.insert("app".to_string(), "web".to_string());

        let policies = manager.find_policies_for_vm(&vm_labels);
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].name, "web-policy");
    }
}
