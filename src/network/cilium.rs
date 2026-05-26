// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Cilium Integration - Cilium network policy and eBPF support

use super::policies::{Port, VMSelector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cilium Network Policy (extends standard Kubernetes NetworkPolicy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiliumNetworkPolicy {
    pub name: String,
    pub endpoint_selector: VMSelector,
    pub ingress: Vec<CiliumIngressRule>,
    pub egress: Vec<CiliumEgressRule>,
    pub labels: HashMap<String, String>,
}

impl CiliumNetworkPolicy {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            endpoint_selector: VMSelector::default(),
            ingress: Vec::new(),
            egress: Vec::new(),
            labels: HashMap::new(),
        }
    }

    pub fn with_selector(mut self, selector: VMSelector) -> Self {
        self.endpoint_selector = selector;
        self
    }

    pub fn add_ingress(mut self, rule: CiliumIngressRule) -> Self {
        self.ingress.push(rule);
        self
    }

    pub fn add_egress(mut self, rule: CiliumEgressRule) -> Self {
        self.egress.push(rule);
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

/// Cilium ingress rule with L7 support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiliumIngressRule {
    pub from_endpoints: Vec<EndpointSelector>,
    pub from_cidrs: Vec<String>,
    pub to_ports: Vec<PortRule>,
    pub to_fqdns: Vec<FQDNSelector>,
}

impl CiliumIngressRule {
    pub fn new() -> Self {
        Self {
            from_endpoints: Vec::new(),
            from_cidrs: Vec::new(),
            to_ports: Vec::new(),
            to_fqdns: Vec::new(),
        }
    }

    pub fn from_endpoint(mut self, selector: EndpointSelector) -> Self {
        self.from_endpoints.push(selector);
        self
    }

    pub fn from_cidr(mut self, cidr: impl Into<String>) -> Self {
        self.from_cidrs.push(cidr.into());
        self
    }

    pub fn to_port(mut self, port_rule: PortRule) -> Self {
        self.to_ports.push(port_rule);
        self
    }
}

impl Default for CiliumIngressRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Cilium egress rule with DNS and L7 support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiliumEgressRule {
    pub to_endpoints: Vec<EndpointSelector>,
    pub to_cidrs: Vec<String>,
    pub to_ports: Vec<PortRule>,
    pub to_fqdns: Vec<FQDNSelector>,
    pub to_services: Vec<ServiceSelector>,
}

impl CiliumEgressRule {
    pub fn new() -> Self {
        Self {
            to_endpoints: Vec::new(),
            to_cidrs: Vec::new(),
            to_ports: Vec::new(),
            to_fqdns: Vec::new(),
            to_services: Vec::new(),
        }
    }

    pub fn to_endpoint(mut self, selector: EndpointSelector) -> Self {
        self.to_endpoints.push(selector);
        self
    }

    pub fn to_cidr(mut self, cidr: impl Into<String>) -> Self {
        self.to_cidrs.push(cidr.into());
        self
    }

    pub fn to_port(mut self, port_rule: PortRule) -> Self {
        self.to_ports.push(port_rule);
        self
    }

    pub fn to_fqdn(mut self, fqdn: FQDNSelector) -> Self {
        self.to_fqdns.push(fqdn);
        self
    }

    pub fn to_service(mut self, service: ServiceSelector) -> Self {
        self.to_services.push(service);
        self
    }
}

impl Default for CiliumEgressRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Endpoint selector (identity-based)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointSelector {
    pub match_labels: HashMap<String, String>,
}

impl EndpointSelector {
    pub fn new() -> Self {
        Self {
            match_labels: HashMap::new(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.match_labels.insert(key.into(), value.into());
        self
    }
}

impl Default for EndpointSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Port rule with L7 protocol support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRule {
    pub ports: Vec<Port>,
    pub rules: Option<L7Rules>,
}

impl PortRule {
    pub fn new() -> Self {
        Self {
            ports: Vec::new(),
            rules: None,
        }
    }

    pub fn add_port(mut self, port: Port) -> Self {
        self.ports.push(port);
        self
    }

    pub fn with_http(mut self, http_rules: Vec<HTTPRule>) -> Self {
        self.rules = Some(L7Rules::HTTP(http_rules));
        self
    }

    pub fn with_kafka(mut self, kafka_rules: Vec<KafkaRule>) -> Self {
        self.rules = Some(L7Rules::Kafka(kafka_rules));
        self
    }

    pub fn with_dns(mut self, dns_rules: Vec<DNSRule>) -> Self {
        self.rules = Some(L7Rules::DNS(dns_rules));
        self
    }
}

impl Default for PortRule {
    fn default() -> Self {
        Self::new()
    }
}

/// L7 protocol rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum L7Rules {
    HTTP(Vec<HTTPRule>),
    Kafka(Vec<KafkaRule>),
    DNS(Vec<DNSRule>),
}

/// HTTP rule for L7 filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPRule {
    pub method: Option<String>,
    pub path: Option<String>,
    pub host: Option<String>,
    pub headers: Vec<String>,
}

impl HTTPRule {
    pub fn new() -> Self {
        Self {
            method: None,
            path: None,
            host: None,
            headers: Vec::new(),
        }
    }

    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }
}

impl Default for HTTPRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Kafka rule for L7 filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaRule {
    pub topic: Option<String>,
    pub api_key: Option<String>,
    pub api_version: Option<String>,
    pub client_id: Option<String>,
}

impl KafkaRule {
    pub fn new() -> Self {
        Self {
            topic: None,
            api_key: None,
            api_version: None,
            client_id: None,
        }
    }

    pub fn topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }
}

impl Default for KafkaRule {
    fn default() -> Self {
        Self::new()
    }
}

/// DNS rule for L7 filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNSRule {
    pub match_name: Option<String>,
    pub match_pattern: Option<String>,
}

impl DNSRule {
    pub fn new() -> Self {
        Self {
            match_name: None,
            match_pattern: None,
        }
    }

    pub fn match_name(mut self, name: impl Into<String>) -> Self {
        self.match_name = Some(name.into());
        self
    }

    pub fn match_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.match_pattern = Some(pattern.into());
        self
    }
}

impl Default for DNSRule {
    fn default() -> Self {
        Self::new()
    }
}

/// FQDN selector for DNS-based policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FQDNSelector {
    pub match_name: Option<String>,
    pub match_pattern: Option<String>,
}

impl FQDNSelector {
    pub fn name(name: impl Into<String>) -> Self {
        Self {
            match_name: Some(name.into()),
            match_pattern: None,
        }
    }

    pub fn pattern(pattern: impl Into<String>) -> Self {
        Self {
            match_name: None,
            match_pattern: Some(pattern.into()),
        }
    }
}

/// Service selector for Kubernetes services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSelector {
    pub namespace: String,
    pub name: String,
}

impl ServiceSelector {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }
}

/// Cilium policy manager
pub struct CiliumPolicyManager {
    policies: HashMap<String, CiliumNetworkPolicy>,
}

impl CiliumPolicyManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    pub fn add_policy(&mut self, policy: CiliumNetworkPolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    pub fn remove_policy(&mut self, name: &str) -> bool {
        self.policies.remove(name).is_some()
    }

    pub fn get_policy(&self, name: &str) -> Option<&CiliumNetworkPolicy> {
        self.policies.get(name)
    }

    pub fn list_policies(&self) -> Vec<&CiliumNetworkPolicy> {
        self.policies.values().collect()
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// Generate Cilium NetworkPolicy YAML
    pub fn generate_yaml(&self, policy_name: &str) -> Option<String> {
        self.get_policy(policy_name)
            .and_then(|policy| serde_yml::to_string(policy).ok())
    }
}

impl Default for CiliumPolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cilium_policy_creation() {
        let selector = VMSelector::default().with_label("app", "web");

        let policy = CiliumNetworkPolicy::new("web-policy")
            .with_selector(selector)
            .with_label("env", "production");

        assert_eq!(policy.name, "web-policy");
        assert_eq!(policy.labels.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_endpoint_selector() {
        let selector = EndpointSelector::new()
            .with_label("role", "frontend")
            .with_label("tier", "web");

        assert_eq!(selector.match_labels.len(), 2);
        assert_eq!(
            selector.match_labels.get("role"),
            Some(&"frontend".to_string())
        );
    }

    #[test]
    fn test_http_rule() {
        let http_rule = HTTPRule::new()
            .method("GET")
            .path("/api/*")
            .host("example.com");

        assert_eq!(http_rule.method, Some("GET".to_string()));
        assert_eq!(http_rule.path, Some("/api/*".to_string()));
        assert_eq!(http_rule.host, Some("example.com".to_string()));
    }

    #[test]
    fn test_l7_port_rule() {
        let http_rule = HTTPRule::new().method("POST").path("/api/users");

        let port_rule = PortRule::new()
            .add_port(Port::tcp(443))
            .with_http(vec![http_rule]);

        assert_eq!(port_rule.ports.len(), 1);
        assert!(port_rule.rules.is_some());
    }

    #[test]
    fn test_fqdn_selector() {
        let fqdn = FQDNSelector::name("api.example.com");
        assert_eq!(fqdn.match_name, Some("api.example.com".to_string()));

        let pattern_fqdn = FQDNSelector::pattern("*.example.com");
        assert_eq!(
            pattern_fqdn.match_pattern,
            Some("*.example.com".to_string())
        );
    }

    #[test]
    fn test_cilium_egress_rule() {
        let egress = CiliumEgressRule::new()
            .to_cidr("0.0.0.0/0")
            .to_fqdn(FQDNSelector::pattern("*.googleapis.com"));

        assert_eq!(egress.to_cidrs.len(), 1);
        assert_eq!(egress.to_fqdns.len(), 1);
    }

    #[test]
    fn test_service_selector() {
        let service = ServiceSelector::new("default", "kubernetes");
        assert_eq!(service.namespace, "default");
        assert_eq!(service.name, "kubernetes");
    }

    #[test]
    fn test_cilium_policy_manager() {
        let mut manager = CiliumPolicyManager::new();

        let policy = CiliumNetworkPolicy::new("test-policy");
        manager.add_policy(policy);

        assert_eq!(manager.policy_count(), 1);
        assert!(manager.get_policy("test-policy").is_some());

        assert!(manager.remove_policy("test-policy"));
        assert_eq!(manager.policy_count(), 0);
    }

    #[test]
    fn test_kafka_rule() {
        let kafka_rule = KafkaRule::new().topic("payments");
        assert_eq!(kafka_rule.topic, Some("payments".to_string()));
    }

    #[test]
    fn test_dns_rule() {
        let dns_rule = DNSRule::new()
            .match_name("api.example.com")
            .match_pattern("*.example.com");

        assert_eq!(dns_rule.match_name, Some("api.example.com".to_string()));
        assert_eq!(dns_rule.match_pattern, Some("*.example.com".to_string()));
    }
}
