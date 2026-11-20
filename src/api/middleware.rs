// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Middleware type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MiddlewareType {
    Authentication,
    Authorization,
    RateLimit,
    Cors,
    Logging,
    RequestId,
    Compression,
    Validation,
    Cache,
    Timeout,
    Custom(String),
}

impl std::fmt::Display for MiddlewareType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiddlewareType::Authentication => write!(f, "authentication"),
            MiddlewareType::Authorization => write!(f, "authorization"),
            MiddlewareType::RateLimit => write!(f, "rate-limit"),
            MiddlewareType::Cors => write!(f, "cors"),
            MiddlewareType::Logging => write!(f, "logging"),
            MiddlewareType::RequestId => write!(f, "request-id"),
            MiddlewareType::Compression => write!(f, "compression"),
            MiddlewareType::Validation => write!(f, "validation"),
            MiddlewareType::Cache => write!(f, "cache"),
            MiddlewareType::Timeout => write!(f, "timeout"),
            MiddlewareType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Middleware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    pub name: String,
    pub middleware_type: MiddlewareType,
    pub enabled: bool,
    pub priority: u32,
    pub config: HashMap<String, String>,
}

impl MiddlewareConfig {
    pub fn new(name: &str, middleware_type: MiddlewareType) -> Self {
        Self {
            name: name.to_string(),
            middleware_type,
            enabled: true,
            priority: 100,
            config: HashMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_config(mut self, key: &str, value: &str) -> Self {
        self.config.insert(key.to_string(), value.to_string());
        self
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

/// Middleware chain for processing requests
pub struct MiddlewareChain {
    middleware: Vec<MiddlewareConfig>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    pub fn add(&mut self, config: MiddlewareConfig) {
        self.middleware.push(config);
        self.middleware.sort_by_key(|m| m.priority);
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let len_before = self.middleware.len();
        self.middleware.retain(|m| m.name != name);
        self.middleware.len() < len_before
    }

    pub fn get(&self, name: &str) -> Option<&MiddlewareConfig> {
        self.middleware.iter().find(|m| m.name == name)
    }

    pub fn enabled_middleware(&self) -> Vec<&MiddlewareConfig> {
        self.middleware.iter().filter(|m| m.enabled).collect()
    }

    pub fn count(&self) -> usize {
        self.middleware.len()
    }

    pub fn enabled_count(&self) -> usize {
        self.middleware.iter().filter(|m| m.enabled).count()
    }

    pub fn has_middleware(&self, name: &str) -> bool {
        self.middleware.iter().any(|m| m.name == name)
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Build default middleware chain for Veyron API
pub fn build_default_chain() -> MiddlewareChain {
    let mut chain = MiddlewareChain::new();

    chain.add(MiddlewareConfig::new("request-id", MiddlewareType::RequestId).with_priority(10));

    chain.add(
        MiddlewareConfig::new("logging", MiddlewareType::Logging)
            .with_priority(20)
            .with_config("level", "info"),
    );

    chain.add(
        MiddlewareConfig::new("cors", MiddlewareType::Cors)
            .with_priority(30)
            .with_config("origins", "")
            .with_config("methods", "GET,POST,PUT,DELETE,PATCH"),
    );

    chain.add(
        MiddlewareConfig::new("rate-limit", MiddlewareType::RateLimit)
            .with_priority(40)
            .with_config("requests_per_minute", "60")
            .with_config("burst", "30"),
    );

    chain.add(
        MiddlewareConfig::new("auth", MiddlewareType::Authentication)
            .with_priority(50)
            .with_config("method", "bearer"),
    );

    chain.add(
        MiddlewareConfig::new("compression", MiddlewareType::Compression)
            .with_priority(90)
            .with_config("algorithm", "gzip")
            .with_config("min_size", "1024"),
    );

    chain.add(
        MiddlewareConfig::new("timeout", MiddlewareType::Timeout)
            .with_priority(95)
            .with_config("timeout_secs", "30"),
    );

    chain
}

/// Request log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: f64,
    pub client_ip: String,
    pub user_agent: Option<String>,
    pub user: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl RequestLog {
    pub fn new(request_id: &str, method: &str, path: &str, status: u16, duration_ms: f64) -> Self {
        Self {
            request_id: request_id.to_string(),
            method: method.to_string(),
            path: path.to_string(),
            status,
            duration_ms,
            client_ip: "127.0.0.1".to_string(),
            user_agent: None,
            user: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_client(mut self, ip: &str) -> Self {
        self.client_ip = ip.to_string();
        self
    }

    pub fn with_user(mut self, user: &str) -> Self {
        self.user = Some(user.to_string());
        self
    }

    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 400
    }

    pub fn is_error(&self) -> bool {
        self.status >= 400
    }

    pub fn is_server_error(&self) -> bool {
        self.status >= 500
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_type_display() {
        assert_eq!(MiddlewareType::Authentication.to_string(), "authentication");
        assert_eq!(MiddlewareType::RateLimit.to_string(), "rate-limit");
        assert_eq!(MiddlewareType::Cors.to_string(), "cors");
        assert_eq!(MiddlewareType::Logging.to_string(), "logging");
        assert_eq!(
            MiddlewareType::Custom("my-mw".to_string()).to_string(),
            "my-mw"
        );
    }

    #[test]
    fn test_middleware_config_new() {
        let mw = MiddlewareConfig::new("auth", MiddlewareType::Authentication);
        assert_eq!(mw.name, "auth");
        assert!(mw.enabled);
        assert_eq!(mw.priority, 100);
    }

    #[test]
    fn test_middleware_config_with_priority() {
        let mw = MiddlewareConfig::new("auth", MiddlewareType::Authentication).with_priority(10);
        assert_eq!(mw.priority, 10);
    }

    #[test]
    fn test_middleware_config_with_config() {
        let mw = MiddlewareConfig::new("cors", MiddlewareType::Cors).with_config("origins", "*");
        assert_eq!(mw.config.get("origins"), Some(&"*".to_string()));
    }

    #[test]
    fn test_middleware_config_enable_disable() {
        let mut mw = MiddlewareConfig::new("auth", MiddlewareType::Authentication);
        assert!(mw.enabled);

        mw.disable();
        assert!(!mw.enabled);

        mw.enable();
        assert!(mw.enabled);
    }

    #[test]
    fn test_middleware_chain_new() {
        let chain = MiddlewareChain::new();
        assert_eq!(chain.count(), 0);
    }

    #[test]
    fn test_middleware_chain_add() {
        let mut chain = MiddlewareChain::new();
        chain.add(MiddlewareConfig::new(
            "auth",
            MiddlewareType::Authentication,
        ));
        chain.add(MiddlewareConfig::new("cors", MiddlewareType::Cors));

        assert_eq!(chain.count(), 2);
    }

    #[test]
    fn test_middleware_chain_priority_order() {
        let mut chain = MiddlewareChain::new();
        chain.add(MiddlewareConfig::new("b", MiddlewareType::Cors).with_priority(20));
        chain.add(MiddlewareConfig::new("a", MiddlewareType::Logging).with_priority(10));

        let enabled = chain.enabled_middleware();
        assert_eq!(enabled[0].name, "a");
        assert_eq!(enabled[1].name, "b");
    }

    #[test]
    fn test_middleware_chain_remove() {
        let mut chain = MiddlewareChain::new();
        chain.add(MiddlewareConfig::new(
            "auth",
            MiddlewareType::Authentication,
        ));

        assert!(chain.remove("auth"));
        assert!(!chain.remove("auth"));
        assert_eq!(chain.count(), 0);
    }

    #[test]
    fn test_middleware_chain_get() {
        let mut chain = MiddlewareChain::new();
        chain.add(MiddlewareConfig::new(
            "auth",
            MiddlewareType::Authentication,
        ));

        assert!(chain.get("auth").is_some());
        assert!(chain.get("nonexistent").is_none());
    }

    #[test]
    fn test_middleware_chain_enabled_count() {
        let mut chain = MiddlewareChain::new();

        let mut disabled = MiddlewareConfig::new("disabled", MiddlewareType::Cache);
        disabled.disable();

        chain.add(MiddlewareConfig::new(
            "auth",
            MiddlewareType::Authentication,
        ));
        chain.add(disabled);

        assert_eq!(chain.count(), 2);
        assert_eq!(chain.enabled_count(), 1);
    }

    #[test]
    fn test_middleware_chain_has_middleware() {
        let mut chain = MiddlewareChain::new();
        chain.add(MiddlewareConfig::new(
            "auth",
            MiddlewareType::Authentication,
        ));

        assert!(chain.has_middleware("auth"));
        assert!(!chain.has_middleware("cors"));
    }

    #[test]
    fn test_build_default_chain() {
        let chain = build_default_chain();
        assert!(chain.count() >= 5);
        assert!(chain.has_middleware("request-id"));
        assert!(chain.has_middleware("logging"));
        assert!(chain.has_middleware("cors"));
        assert!(chain.has_middleware("rate-limit"));
        assert!(chain.has_middleware("auth"));
    }

    #[test]
    fn test_request_log_new() {
        let log = RequestLog::new("req-1", "GET", "/api/v1/vms", 200, 15.5);
        assert_eq!(log.request_id, "req-1");
        assert_eq!(log.status, 200);
        assert!(log.is_success());
        assert!(!log.is_error());
    }

    #[test]
    fn test_request_log_with_client() {
        let log = RequestLog::new("req-1", "GET", "/api/v1/vms", 200, 10.0)
            .with_client("10.0.0.1")
            .with_user("admin");
        assert_eq!(log.client_ip, "10.0.0.1");
        assert_eq!(log.user, Some("admin".to_string()));
    }

    #[test]
    fn test_request_log_is_error() {
        let client_error = RequestLog::new("req-1", "GET", "/api/v1/vms", 404, 5.0);
        assert!(client_error.is_error());
        assert!(!client_error.is_server_error());

        let server_error = RequestLog::new("req-2", "GET", "/api/v1/vms", 500, 5.0);
        assert!(server_error.is_error());
        assert!(server_error.is_server_error());
    }
}
