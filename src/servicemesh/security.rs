// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// mTLS mode for service mesh
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MTLSMode {
    Strict,     // Only mTLS traffic is allowed
    Permissive, // Both plaintext and mTLS traffic are allowed
    Disabled,   // mTLS is disabled
}

impl std::fmt::Display for MTLSMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MTLSMode::Strict => write!(f, "Strict"),
            MTLSMode::Permissive => write!(f, "Permissive"),
            MTLSMode::Disabled => write!(f, "Disabled"),
        }
    }
}

/// Peer authentication policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAuthentication {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub mtls_mode: MTLSMode,
    pub port_level_mtls: HashMap<u16, MTLSMode>,
    pub created_at: DateTime<Utc>,
}

impl PeerAuthentication {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "pa-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            mtls_mode: MTLSMode::Strict,
            port_level_mtls: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_mtls_mode(mut self, mode: MTLSMode) -> Self {
        self.mtls_mode = mode;
        self
    }

    pub fn add_port_mtls(&mut self, port: u16, mode: MTLSMode) {
        self.port_level_mtls.insert(port, mode);
    }

    pub fn has_port_level_config(&self) -> bool {
        !self.port_level_mtls.is_empty()
    }
}

/// Authorization policy action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationAction {
    Allow,
    Deny,
    Audit,
    Custom,
}

/// Authorization policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationPolicy {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub action: AuthorizationAction,
    pub rules: Vec<AuthorizationRule>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRule {
    pub from: Vec<AuthorizationSource>,
    pub to: Vec<AuthorizationOperation>,
    pub when: Vec<AuthorizationCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationSource {
    pub principals: Vec<String>,
    pub namespaces: Vec<String>,
    pub ip_blocks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationOperation {
    pub hosts: Vec<String>,
    pub ports: Vec<u16>,
    pub methods: Vec<String>,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCondition {
    pub key: String,
    pub values: Vec<String>,
}

impl AuthorizationPolicy {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "ap-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            action: AuthorizationAction::Allow,
            rules: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_action(mut self, action: AuthorizationAction) -> Self {
        self.action = action;
        self
    }

    pub fn add_rule(&mut self, rule: AuthorizationRule) {
        self.rules.push(rule);
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// Certificate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub subject: String,
    pub issuer: String,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub san_dns: Vec<String>,
    pub san_ips: Vec<String>,
    pub key_size: u32,
    pub created_at: DateTime<Utc>,
}

impl Certificate {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "cert-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            subject: subject.into(),
            issuer: "Veyron CA".to_string(),
            valid_from: Utc::now(),
            valid_until: Utc::now() + chrono::TimeDelta::days(365),
            san_dns: Vec::new(),
            san_ips: Vec::new(),
            key_size: 2048,
            created_at: Utc::now(),
        }
    }

    pub fn add_san_dns(&mut self, dns: impl Into<String>) {
        self.san_dns.push(dns.into());
    }

    pub fn add_san_ip(&mut self, ip: impl Into<String>) {
        self.san_ips.push(ip.into());
    }

    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        now >= self.valid_from && now <= self.valid_until
    }

    pub fn days_until_expiry(&self) -> i64 {
        (self.valid_until - Utc::now()).num_days()
    }
}

/// Security manager for service mesh
pub struct SecurityManager {
    peer_authentications: HashMap<String, PeerAuthentication>,
    authorization_policies: HashMap<String, AuthorizationPolicy>,
    certificates: HashMap<String, Certificate>,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            peer_authentications: HashMap::new(),
            authorization_policies: HashMap::new(),
            certificates: HashMap::new(),
        }
    }

    pub fn add_peer_authentication(&mut self, auth: PeerAuthentication) -> String {
        let id = auth.id.clone();
        self.peer_authentications.insert(id.clone(), auth);
        id
    }

    pub fn get_peer_authentication(&self, id: &str) -> Option<&PeerAuthentication> {
        self.peer_authentications.get(id)
    }

    pub fn remove_peer_authentication(&mut self, id: &str) -> bool {
        self.peer_authentications.remove(id).is_some()
    }

    pub fn peer_authentication_count(&self) -> usize {
        self.peer_authentications.len()
    }

    pub fn add_authorization_policy(&mut self, policy: AuthorizationPolicy) -> String {
        let id = policy.id.clone();
        self.authorization_policies.insert(id.clone(), policy);
        id
    }

    pub fn get_authorization_policy(&self, id: &str) -> Option<&AuthorizationPolicy> {
        self.authorization_policies.get(id)
    }

    pub fn remove_authorization_policy(&mut self, id: &str) -> bool {
        self.authorization_policies.remove(id).is_some()
    }

    pub fn authorization_policy_count(&self) -> usize {
        self.authorization_policies.len()
    }

    pub fn add_certificate(&mut self, cert: Certificate) -> String {
        let id = cert.id.clone();
        self.certificates.insert(id.clone(), cert);
        id
    }

    pub fn get_certificate(&self, id: &str) -> Option<&Certificate> {
        self.certificates.get(id)
    }

    pub fn remove_certificate(&mut self, id: &str) -> bool {
        self.certificates.remove(id).is_some()
    }

    pub fn certificate_count(&self) -> usize {
        self.certificates.len()
    }

    pub fn strict_mtls_policies(&self) -> Vec<&PeerAuthentication> {
        self.peer_authentications
            .values()
            .filter(|p| p.mtls_mode == MTLSMode::Strict)
            .collect()
    }

    pub fn expiring_certificates(&self, days: i64) -> Vec<&Certificate> {
        self.certificates
            .values()
            .filter(|c| c.days_until_expiry() <= days && c.is_valid())
            .collect()
    }

    pub fn deny_policies(&self) -> Vec<&AuthorizationPolicy> {
        self.authorization_policies
            .values()
            .filter(|p| p.action == AuthorizationAction::Deny)
            .collect()
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mtls_mode_display() {
        assert_eq!(MTLSMode::Strict.to_string(), "Strict");
        assert_eq!(MTLSMode::Permissive.to_string(), "Permissive");
        assert_eq!(MTLSMode::Disabled.to_string(), "Disabled");
    }

    #[test]
    fn test_peer_authentication() {
        let auth = PeerAuthentication::new("my-auth", "default");

        assert_eq!(auth.name, "my-auth");
        assert_eq!(auth.namespace, "default");
        assert_eq!(auth.mtls_mode, MTLSMode::Strict);
        assert!(!auth.has_port_level_config());
    }

    #[test]
    fn test_peer_auth_builder() {
        let auth = PeerAuthentication::new("auth", "default").with_mtls_mode(MTLSMode::Permissive);

        assert_eq!(auth.mtls_mode, MTLSMode::Permissive);
    }

    #[test]
    fn test_peer_auth_port_level() {
        let mut auth = PeerAuthentication::new("auth", "default");

        auth.add_port_mtls(8080, MTLSMode::Permissive);
        auth.add_port_mtls(9090, MTLSMode::Strict);

        assert!(auth.has_port_level_config());
        assert_eq!(auth.port_level_mtls.len(), 2);
        assert_eq!(auth.port_level_mtls.get(&8080), Some(&MTLSMode::Permissive));
    }

    #[test]
    fn test_authorization_policy() {
        let policy = AuthorizationPolicy::new("my-policy", "default");

        assert_eq!(policy.name, "my-policy");
        assert_eq!(policy.action, AuthorizationAction::Allow);
        assert_eq!(policy.rule_count(), 0);
    }

    #[test]
    fn test_authorization_policy_builder() {
        let policy =
            AuthorizationPolicy::new("policy", "default").with_action(AuthorizationAction::Deny);

        assert_eq!(policy.action, AuthorizationAction::Deny);
    }

    #[test]
    fn test_authorization_rule() {
        let mut policy = AuthorizationPolicy::new("policy", "default");

        let rule = AuthorizationRule {
            from: vec![AuthorizationSource {
                principals: vec!["cluster.local/ns/default/sa/myapp".to_string()],
                namespaces: vec!["default".to_string()],
                ip_blocks: vec![],
            }],
            to: vec![AuthorizationOperation {
                hosts: vec!["myservice.default.svc.cluster.local".to_string()],
                ports: vec![8080],
                methods: vec!["GET".to_string(), "POST".to_string()],
                paths: vec!["/api/*".to_string()],
            }],
            when: vec![],
        };

        policy.add_rule(rule);
        assert_eq!(policy.rule_count(), 1);
    }

    #[test]
    fn test_authorization_source() {
        let source = AuthorizationSource {
            principals: vec!["user1".to_string()],
            namespaces: vec!["ns1".to_string()],
            ip_blocks: vec!["10.0.0.0/8".to_string()],
        };

        assert_eq!(source.principals.len(), 1);
        assert_eq!(source.namespaces.len(), 1);
        assert_eq!(source.ip_blocks.len(), 1);
    }

    #[test]
    fn test_authorization_operation() {
        let operation = AuthorizationOperation {
            hosts: vec!["service.local".to_string()],
            ports: vec![80, 443],
            methods: vec!["GET".to_string()],
            paths: vec!["/api".to_string()],
        };

        assert_eq!(operation.ports.len(), 2);
        assert_eq!(operation.methods.len(), 1);
    }

    #[test]
    fn test_authorization_condition() {
        let condition = AuthorizationCondition {
            key: "request.headers[version]".to_string(),
            values: vec!["v1".to_string(), "v2".to_string()],
        };

        assert_eq!(condition.key, "request.headers[version]");
        assert_eq!(condition.values.len(), 2);
    }

    #[test]
    fn test_certificate() {
        let cert = Certificate::new("my-cert", "default", "CN=myservice");

        assert_eq!(cert.name, "my-cert");
        assert_eq!(cert.subject, "CN=myservice");
        assert_eq!(cert.issuer, "Veyron CA");
        assert_eq!(cert.key_size, 2048);
        assert!(cert.is_valid());
    }

    #[test]
    fn test_certificate_san() {
        let mut cert = Certificate::new("cert", "default", "CN=service");

        cert.add_san_dns("service.example.com");
        cert.add_san_dns("*.service.example.com");
        cert.add_san_ip("192.168.1.100");

        assert_eq!(cert.san_dns.len(), 2);
        assert_eq!(cert.san_ips.len(), 1);
    }

    #[test]
    fn test_certificate_validity() {
        let cert = Certificate::new("cert", "default", "CN=test");

        assert!(cert.is_valid());
        assert!(cert.days_until_expiry() > 0);
    }

    #[test]
    fn test_security_manager() {
        let mut manager = SecurityManager::new();

        let auth = PeerAuthentication::new("auth", "default");
        let id = manager.add_peer_authentication(auth);

        assert_eq!(manager.peer_authentication_count(), 1);
        assert!(manager.get_peer_authentication(&id).is_some());
    }

    #[test]
    fn test_manager_authorization_policies() {
        let mut manager = SecurityManager::new();

        let policy = AuthorizationPolicy::new("policy", "default");
        let id = manager.add_authorization_policy(policy);

        assert_eq!(manager.authorization_policy_count(), 1);
        assert!(manager.get_authorization_policy(&id).is_some());
    }

    #[test]
    fn test_manager_certificates() {
        let mut manager = SecurityManager::new();

        let cert = Certificate::new("cert", "default", "CN=test");
        let id = manager.add_certificate(cert);

        assert_eq!(manager.certificate_count(), 1);
        assert!(manager.get_certificate(&id).is_some());
    }

    #[test]
    fn test_manager_remove_peer_authentication() {
        let mut manager = SecurityManager::new();

        let auth = PeerAuthentication::new("auth", "default");
        let id = manager.add_peer_authentication(auth);

        assert!(manager.remove_peer_authentication(&id));
        assert_eq!(manager.peer_authentication_count(), 0);
    }

    #[test]
    fn test_manager_remove_authorization_policy() {
        let mut manager = SecurityManager::new();

        let policy = AuthorizationPolicy::new("policy", "default");
        let id = manager.add_authorization_policy(policy);

        assert!(manager.remove_authorization_policy(&id));
        assert_eq!(manager.authorization_policy_count(), 0);
    }

    #[test]
    fn test_manager_remove_certificate() {
        let mut manager = SecurityManager::new();

        let cert = Certificate::new("cert", "default", "CN=test");
        let id = manager.add_certificate(cert);

        assert!(manager.remove_certificate(&id));
        assert_eq!(manager.certificate_count(), 0);
    }

    #[test]
    fn test_manager_strict_mtls_policies() {
        let mut manager = SecurityManager::new();

        manager.add_peer_authentication(PeerAuthentication::new("auth1", "ns1"));
        manager.add_peer_authentication(
            PeerAuthentication::new("auth2", "ns2").with_mtls_mode(MTLSMode::Permissive),
        );
        manager.add_peer_authentication(PeerAuthentication::new("auth3", "ns3"));

        let strict = manager.strict_mtls_policies();
        assert_eq!(strict.len(), 2);
    }

    #[test]
    fn test_manager_expiring_certificates() {
        let mut manager = SecurityManager::new();

        let mut cert = Certificate::new("cert", "default", "CN=test");
        cert.valid_until = Utc::now() + chrono::TimeDelta::days(10);
        manager.add_certificate(cert);

        let expiring = manager.expiring_certificates(30);
        assert_eq!(expiring.len(), 1);
    }

    #[test]
    fn test_manager_deny_policies() {
        let mut manager = SecurityManager::new();

        manager.add_authorization_policy(AuthorizationPolicy::new("policy1", "ns1"));
        manager.add_authorization_policy(
            AuthorizationPolicy::new("policy2", "ns2").with_action(AuthorizationAction::Deny),
        );
        manager.add_authorization_policy(
            AuthorizationPolicy::new("policy3", "ns3").with_action(AuthorizationAction::Deny),
        );

        let deny = manager.deny_policies();
        assert_eq!(deny.len(), 2);
    }

    #[test]
    fn test_mtls_mode_equality() {
        assert_eq!(MTLSMode::Strict, MTLSMode::Strict);
        assert_ne!(MTLSMode::Strict, MTLSMode::Permissive);
    }

    #[test]
    fn test_authorization_action_equality() {
        assert_eq!(AuthorizationAction::Allow, AuthorizationAction::Allow);
        assert_ne!(AuthorizationAction::Allow, AuthorizationAction::Deny);
    }
}
