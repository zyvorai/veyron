use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Timeout policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub service: String,
    pub request_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub per_try_timeout_seconds: Option<u64>,
    pub created_at: DateTime<Utc>,
}

impl TimeoutPolicy {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "tp-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            service: service.into(),
            request_timeout_seconds: 30,
            idle_timeout_seconds: 300,
            per_try_timeout_seconds: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_request_timeout(mut self, seconds: u64) -> Self {
        self.request_timeout_seconds = seconds;
        self
    }

    pub fn with_idle_timeout(mut self, seconds: u64) -> Self {
        self.idle_timeout_seconds = seconds;
        self
    }

    pub fn with_per_try_timeout(mut self, seconds: u64) -> Self {
        self.per_try_timeout_seconds = Some(seconds);
        self
    }
}

/// Retry policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub service: String,
    pub max_attempts: u32,
    pub per_try_timeout_seconds: u64,
    pub retry_on: Vec<RetryCondition>,
    pub backoff_base_interval_seconds: u64,
    pub backoff_max_interval_seconds: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetryCondition {
    ConnectFailure,
    RefusedStream,
    Unavailable,
    Cancelled,
    DeadlineExceeded,
    ResourceExhausted,
    Internal,
    Http5xx,
    Http429,
    Http503,
    Reset,
    Retriable4xx,
}

impl std::fmt::Display for RetryCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetryCondition::ConnectFailure => write!(f, "connect-failure"),
            RetryCondition::RefusedStream => write!(f, "refused-stream"),
            RetryCondition::Unavailable => write!(f, "unavailable"),
            RetryCondition::Cancelled => write!(f, "cancelled"),
            RetryCondition::DeadlineExceeded => write!(f, "deadline-exceeded"),
            RetryCondition::ResourceExhausted => write!(f, "resource-exhausted"),
            RetryCondition::Internal => write!(f, "internal"),
            RetryCondition::Http5xx => write!(f, "5xx"),
            RetryCondition::Http429 => write!(f, "429"),
            RetryCondition::Http503 => write!(f, "503"),
            RetryCondition::Reset => write!(f, "reset"),
            RetryCondition::Retriable4xx => write!(f, "retriable-4xx"),
        }
    }
}

impl RetryPolicy {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "rp-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            service: service.into(),
            max_attempts: 3,
            per_try_timeout_seconds: 10,
            retry_on: vec![RetryCondition::Http5xx, RetryCondition::Unavailable],
            backoff_base_interval_seconds: 1,
            backoff_max_interval_seconds: 10,
            created_at: Utc::now(),
        }
    }

    pub fn with_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts;
        self
    }

    pub fn with_per_try_timeout(mut self, seconds: u64) -> Self {
        self.per_try_timeout_seconds = seconds;
        self
    }

    pub fn add_retry_condition(&mut self, condition: RetryCondition) {
        if !self.retry_on.contains(&condition) {
            self.retry_on.push(condition);
        }
    }
}

/// Rate limiting policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitPolicy {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub service: String,
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub fill_interval_seconds: u64,
    pub created_at: DateTime<Utc>,
}

impl RateLimitPolicy {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "rl-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            service: service.into(),
            requests_per_second: 100,
            burst_size: 200,
            fill_interval_seconds: 1,
            created_at: Utc::now(),
        }
    }

    pub fn with_rate(mut self, rps: u32, burst: u32) -> Self {
        self.requests_per_second = rps;
        self.burst_size = burst;
        self
    }
}

/// Connection pool policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolPolicy {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub service: String,
    pub http1_max_pending_requests: u32,
    pub http2_max_requests: u32,
    pub tcp_max_connections: u32,
    pub connect_timeout_seconds: u64,
    pub created_at: DateTime<Utc>,
}

impl ConnectionPoolPolicy {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "cp-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            service: service.into(),
            http1_max_pending_requests: 1024,
            http2_max_requests: 1024,
            tcp_max_connections: 1024,
            connect_timeout_seconds: 10,
            created_at: Utc::now(),
        }
    }

    pub fn with_http_limits(mut self, http1_pending: u32, http2_requests: u32) -> Self {
        self.http1_max_pending_requests = http1_pending;
        self.http2_max_requests = http2_requests;
        self
    }

    pub fn with_tcp_limit(mut self, max_connections: u32) -> Self {
        self.tcp_max_connections = max_connections;
        self
    }
}

/// Policy manager
pub struct PolicyManager {
    timeout_policies: HashMap<String, TimeoutPolicy>,
    retry_policies: HashMap<String, RetryPolicy>,
    rate_limit_policies: HashMap<String, RateLimitPolicy>,
    connection_pool_policies: HashMap<String, ConnectionPoolPolicy>,
}

impl PolicyManager {
    pub fn new() -> Self {
        Self {
            timeout_policies: HashMap::new(),
            retry_policies: HashMap::new(),
            rate_limit_policies: HashMap::new(),
            connection_pool_policies: HashMap::new(),
        }
    }

    pub fn add_timeout_policy(&mut self, policy: TimeoutPolicy) -> String {
        let id = policy.id.clone();
        self.timeout_policies.insert(id.clone(), policy);
        id
    }

    pub fn get_timeout_policy(&self, id: &str) -> Option<&TimeoutPolicy> {
        self.timeout_policies.get(id)
    }

    pub fn remove_timeout_policy(&mut self, id: &str) -> bool {
        self.timeout_policies.remove(id).is_some()
    }

    pub fn timeout_policy_count(&self) -> usize {
        self.timeout_policies.len()
    }

    pub fn add_retry_policy(&mut self, policy: RetryPolicy) -> String {
        let id = policy.id.clone();
        self.retry_policies.insert(id.clone(), policy);
        id
    }

    pub fn get_retry_policy(&self, id: &str) -> Option<&RetryPolicy> {
        self.retry_policies.get(id)
    }

    pub fn get_retry_policy_mut(&mut self, id: &str) -> Option<&mut RetryPolicy> {
        self.retry_policies.get_mut(id)
    }

    pub fn remove_retry_policy(&mut self, id: &str) -> bool {
        self.retry_policies.remove(id).is_some()
    }

    pub fn retry_policy_count(&self) -> usize {
        self.retry_policies.len()
    }

    pub fn add_rate_limit_policy(&mut self, policy: RateLimitPolicy) -> String {
        let id = policy.id.clone();
        self.rate_limit_policies.insert(id.clone(), policy);
        id
    }

    pub fn get_rate_limit_policy(&self, id: &str) -> Option<&RateLimitPolicy> {
        self.rate_limit_policies.get(id)
    }

    pub fn remove_rate_limit_policy(&mut self, id: &str) -> bool {
        self.rate_limit_policies.remove(id).is_some()
    }

    pub fn rate_limit_policy_count(&self) -> usize {
        self.rate_limit_policies.len()
    }

    pub fn add_connection_pool_policy(&mut self, policy: ConnectionPoolPolicy) -> String {
        let id = policy.id.clone();
        self.connection_pool_policies.insert(id.clone(), policy);
        id
    }

    pub fn get_connection_pool_policy(&self, id: &str) -> Option<&ConnectionPoolPolicy> {
        self.connection_pool_policies.get(id)
    }

    pub fn remove_connection_pool_policy(&mut self, id: &str) -> bool {
        self.connection_pool_policies.remove(id).is_some()
    }

    pub fn connection_pool_policy_count(&self) -> usize {
        self.connection_pool_policies.len()
    }

    pub fn by_service(&self, service: &str) -> Vec<&TimeoutPolicy> {
        self.timeout_policies
            .values()
            .filter(|p| p.service == service)
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
    fn test_timeout_policy() {
        let policy = TimeoutPolicy::new("my-timeout", "default", "my-service");

        assert_eq!(policy.name, "my-timeout");
        assert_eq!(policy.service, "my-service");
        assert_eq!(policy.request_timeout_seconds, 30);
        assert_eq!(policy.idle_timeout_seconds, 300);
    }

    #[test]
    fn test_timeout_policy_builder() {
        let policy = TimeoutPolicy::new("timeout", "default", "service")
            .with_request_timeout(60)
            .with_idle_timeout(600)
            .with_per_try_timeout(10);

        assert_eq!(policy.request_timeout_seconds, 60);
        assert_eq!(policy.idle_timeout_seconds, 600);
        assert_eq!(policy.per_try_timeout_seconds, Some(10));
    }

    #[test]
    fn test_retry_condition_display() {
        assert_eq!(RetryCondition::Http5xx.to_string(), "5xx");
        assert_eq!(
            RetryCondition::ConnectFailure.to_string(),
            "connect-failure"
        );
        assert_eq!(RetryCondition::Unavailable.to_string(), "unavailable");
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::new("my-retry", "default", "my-service");

        assert_eq!(policy.name, "my-retry");
        assert_eq!(policy.service, "my-service");
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.retry_on.len(), 2);
    }

    #[test]
    fn test_retry_policy_builder() {
        let policy = RetryPolicy::new("retry", "default", "service")
            .with_attempts(5)
            .with_per_try_timeout(15);

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.per_try_timeout_seconds, 15);
    }

    #[test]
    fn test_retry_add_condition() {
        let mut policy = RetryPolicy::new("retry", "default", "service");

        policy.add_retry_condition(RetryCondition::Reset);
        policy.add_retry_condition(RetryCondition::Http429);

        assert!(policy.retry_on.contains(&RetryCondition::Reset));
        assert!(policy.retry_on.contains(&RetryCondition::Http429));
    }

    #[test]
    fn test_retry_duplicate_condition() {
        let mut policy = RetryPolicy::new("retry", "default", "service");

        let initial_count = policy.retry_on.len();
        policy.add_retry_condition(RetryCondition::Http5xx);

        assert_eq!(policy.retry_on.len(), initial_count);
    }

    #[test]
    fn test_rate_limit_policy() {
        let policy = RateLimitPolicy::new("my-ratelimit", "default", "my-service");

        assert_eq!(policy.name, "my-ratelimit");
        assert_eq!(policy.service, "my-service");
        assert_eq!(policy.requests_per_second, 100);
        assert_eq!(policy.burst_size, 200);
    }

    #[test]
    fn test_rate_limit_builder() {
        let policy = RateLimitPolicy::new("ratelimit", "default", "service").with_rate(1000, 2000);

        assert_eq!(policy.requests_per_second, 1000);
        assert_eq!(policy.burst_size, 2000);
    }

    #[test]
    fn test_connection_pool_policy() {
        let policy = ConnectionPoolPolicy::new("my-pool", "default", "my-service");

        assert_eq!(policy.name, "my-pool");
        assert_eq!(policy.service, "my-service");
        assert_eq!(policy.http1_max_pending_requests, 1024);
        assert_eq!(policy.http2_max_requests, 1024);
        assert_eq!(policy.tcp_max_connections, 1024);
    }

    #[test]
    fn test_connection_pool_builder() {
        let policy = ConnectionPoolPolicy::new("pool", "default", "service")
            .with_http_limits(512, 2048)
            .with_tcp_limit(512);

        assert_eq!(policy.http1_max_pending_requests, 512);
        assert_eq!(policy.http2_max_requests, 2048);
        assert_eq!(policy.tcp_max_connections, 512);
    }

    #[test]
    fn test_policy_manager() {
        let mut manager = PolicyManager::new();

        let policy = TimeoutPolicy::new("timeout", "default", "service");
        let id = manager.add_timeout_policy(policy);

        assert_eq!(manager.timeout_policy_count(), 1);
        assert!(manager.get_timeout_policy(&id).is_some());
    }

    #[test]
    fn test_manager_retry_policies() {
        let mut manager = PolicyManager::new();

        let policy = RetryPolicy::new("retry", "default", "service");
        let id = manager.add_retry_policy(policy);

        assert_eq!(manager.retry_policy_count(), 1);
        assert!(manager.get_retry_policy(&id).is_some());
    }

    #[test]
    fn test_manager_rate_limit_policies() {
        let mut manager = PolicyManager::new();

        let policy = RateLimitPolicy::new("ratelimit", "default", "service");
        let id = manager.add_rate_limit_policy(policy);

        assert_eq!(manager.rate_limit_policy_count(), 1);
        assert!(manager.get_rate_limit_policy(&id).is_some());
    }

    #[test]
    fn test_manager_connection_pool_policies() {
        let mut manager = PolicyManager::new();

        let policy = ConnectionPoolPolicy::new("pool", "default", "service");
        let id = manager.add_connection_pool_policy(policy);

        assert_eq!(manager.connection_pool_policy_count(), 1);
        assert!(manager.get_connection_pool_policy(&id).is_some());
    }

    #[test]
    fn test_manager_remove_timeout_policy() {
        let mut manager = PolicyManager::new();

        let policy = TimeoutPolicy::new("timeout", "default", "service");
        let id = manager.add_timeout_policy(policy);

        assert!(manager.remove_timeout_policy(&id));
        assert_eq!(manager.timeout_policy_count(), 0);
    }

    #[test]
    fn test_manager_remove_retry_policy() {
        let mut manager = PolicyManager::new();

        let policy = RetryPolicy::new("retry", "default", "service");
        let id = manager.add_retry_policy(policy);

        assert!(manager.remove_retry_policy(&id));
        assert_eq!(manager.retry_policy_count(), 0);
    }

    #[test]
    fn test_manager_remove_rate_limit_policy() {
        let mut manager = PolicyManager::new();

        let policy = RateLimitPolicy::new("ratelimit", "default", "service");
        let id = manager.add_rate_limit_policy(policy);

        assert!(manager.remove_rate_limit_policy(&id));
        assert_eq!(manager.rate_limit_policy_count(), 0);
    }

    #[test]
    fn test_manager_remove_connection_pool_policy() {
        let mut manager = PolicyManager::new();

        let policy = ConnectionPoolPolicy::new("pool", "default", "service");
        let id = manager.add_connection_pool_policy(policy);

        assert!(manager.remove_connection_pool_policy(&id));
        assert_eq!(manager.connection_pool_policy_count(), 0);
    }

    #[test]
    fn test_manager_by_service() {
        let mut manager = PolicyManager::new();

        manager.add_timeout_policy(TimeoutPolicy::new("t1", "ns1", "svc1"));
        manager.add_timeout_policy(TimeoutPolicy::new("t2", "ns2", "svc1"));
        manager.add_timeout_policy(TimeoutPolicy::new("t3", "ns3", "svc2"));

        let svc1_policies = manager.by_service("svc1");
        assert_eq!(svc1_policies.len(), 2);
    }

    #[test]
    fn test_manager_get_retry_policy_mut() {
        let mut manager = PolicyManager::new();

        let policy = RetryPolicy::new("retry", "default", "service");
        let id = manager.add_retry_policy(policy);

        if let Some(policy_mut) = manager.get_retry_policy_mut(&id) {
            policy_mut.add_retry_condition(RetryCondition::Reset);
        }

        let policy = manager.get_retry_policy(&id).unwrap();
        assert!(policy.retry_on.contains(&RetryCondition::Reset));
    }

    #[test]
    fn test_retry_condition_equality() {
        assert_eq!(RetryCondition::Http5xx, RetryCondition::Http5xx);
        assert_ne!(RetryCondition::Http5xx, RetryCondition::Http429);
    }
}
