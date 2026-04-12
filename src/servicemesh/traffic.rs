use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Traffic splitting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSplit {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub service: String,
    pub backends: Vec<TrafficBackend>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficBackend {
    pub service: String,
    pub weight: u32,
    pub version: String,
}

impl TrafficSplit {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "ts-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            service: service.into(),
            backends: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_backend(&mut self, backend: TrafficBackend) {
        self.backends.push(backend);
    }

    pub fn total_weight(&self) -> u32 {
        self.backends.iter().map(|b| b.weight).sum()
    }

    pub fn normalize_weights(&mut self) {
        let total = self.total_weight();
        if total > 0 && total != 100 {
            let scale = 100.0 / total as f64;
            for backend in &mut self.backends {
                backend.weight = (backend.weight as f64 * scale).round() as u32;
            }
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub target_service: String,
    pub max_connections: u32,
    pub max_pending_requests: u32,
    pub max_requests: u32,
    pub max_retries: u32,
    pub consecutive_errors: u32,
    pub interval_seconds: u64,
    pub base_ejection_time_seconds: u64,
    pub max_ejection_percent: u32,
    pub created_at: DateTime<Utc>,
}

impl CircuitBreaker {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "cb-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            target_service: target.into(),
            max_connections: 1024,
            max_pending_requests: 1024,
            max_requests: 1024,
            max_retries: 3,
            consecutive_errors: 5,
            interval_seconds: 10,
            base_ejection_time_seconds: 30,
            max_ejection_percent: 10,
            created_at: Utc::now(),
        }
    }

    pub fn with_limits(mut self, connections: u32, pending: u32, requests: u32) -> Self {
        self.max_connections = connections;
        self.max_pending_requests = pending;
        self.max_requests = requests;
        self
    }

    pub fn with_ejection(mut self, errors: u32, interval: u64, base_time: u64) -> Self {
        self.consecutive_errors = errors;
        self.interval_seconds = interval;
        self.base_ejection_time_seconds = base_time;
        self
    }
}

/// Traffic mirroring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficMirror {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub source_service: String,
    pub mirror_service: String,
    pub percentage: f64,
    pub created_at: DateTime<Utc>,
}

impl TrafficMirror {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        source: impl Into<String>,
        mirror: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "tm-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            source_service: source.into(),
            mirror_service: mirror.into(),
            percentage: 100.0,
            created_at: Utc::now(),
        }
    }

    pub fn with_percentage(mut self, percentage: f64) -> Self {
        self.percentage = percentage.clamp(0.0, 100.0);
        self
    }
}

/// Load balancing strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastRequest,
    Random,
    PassThrough,
    ConsistentHash,
}

impl std::fmt::Display for LoadBalancingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadBalancingStrategy::RoundRobin => write!(f, "Round Robin"),
            LoadBalancingStrategy::LeastRequest => write!(f, "Least Request"),
            LoadBalancingStrategy::Random => write!(f, "Random"),
            LoadBalancingStrategy::PassThrough => write!(f, "Pass Through"),
            LoadBalancingStrategy::ConsistentHash => write!(f, "Consistent Hash"),
        }
    }
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancer {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub service: String,
    pub strategy: LoadBalancingStrategy,
    pub session_affinity: bool,
    pub session_affinity_cookie: Option<String>,
    pub health_check_enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl LoadBalancer {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "lb-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            service: service.into(),
            strategy: LoadBalancingStrategy::RoundRobin,
            session_affinity: false,
            session_affinity_cookie: None,
            health_check_enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_strategy(mut self, strategy: LoadBalancingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_session_affinity(mut self, cookie_name: impl Into<String>) -> Self {
        self.session_affinity = true;
        self.session_affinity_cookie = Some(cookie_name.into());
        self
    }
}

/// Traffic management policies
pub struct TrafficManager {
    splits: HashMap<String, TrafficSplit>,
    circuit_breakers: HashMap<String, CircuitBreaker>,
    mirrors: HashMap<String, TrafficMirror>,
    load_balancers: HashMap<String, LoadBalancer>,
}

impl TrafficManager {
    pub fn new() -> Self {
        Self {
            splits: HashMap::new(),
            circuit_breakers: HashMap::new(),
            mirrors: HashMap::new(),
            load_balancers: HashMap::new(),
        }
    }

    pub fn add_split(&mut self, split: TrafficSplit) -> String {
        let id = split.id.clone();
        self.splits.insert(id.clone(), split);
        id
    }

    pub fn get_split(&self, id: &str) -> Option<&TrafficSplit> {
        self.splits.get(id)
    }

    pub fn get_split_mut(&mut self, id: &str) -> Option<&mut TrafficSplit> {
        self.splits.get_mut(id)
    }

    pub fn remove_split(&mut self, id: &str) -> bool {
        self.splits.remove(id).is_some()
    }

    pub fn split_count(&self) -> usize {
        self.splits.len()
    }

    pub fn add_circuit_breaker(&mut self, cb: CircuitBreaker) -> String {
        let id = cb.id.clone();
        self.circuit_breakers.insert(id.clone(), cb);
        id
    }

    pub fn get_circuit_breaker(&self, id: &str) -> Option<&CircuitBreaker> {
        self.circuit_breakers.get(id)
    }

    pub fn remove_circuit_breaker(&mut self, id: &str) -> bool {
        self.circuit_breakers.remove(id).is_some()
    }

    pub fn circuit_breaker_count(&self) -> usize {
        self.circuit_breakers.len()
    }

    pub fn add_mirror(&mut self, mirror: TrafficMirror) -> String {
        let id = mirror.id.clone();
        self.mirrors.insert(id.clone(), mirror);
        id
    }

    pub fn get_mirror(&self, id: &str) -> Option<&TrafficMirror> {
        self.mirrors.get(id)
    }

    pub fn remove_mirror(&mut self, id: &str) -> bool {
        self.mirrors.remove(id).is_some()
    }

    pub fn mirror_count(&self) -> usize {
        self.mirrors.len()
    }

    pub fn add_load_balancer(&mut self, lb: LoadBalancer) -> String {
        let id = lb.id.clone();
        self.load_balancers.insert(id.clone(), lb);
        id
    }

    pub fn get_load_balancer(&self, id: &str) -> Option<&LoadBalancer> {
        self.load_balancers.get(id)
    }

    pub fn remove_load_balancer(&mut self, id: &str) -> bool {
        self.load_balancers.remove(id).is_some()
    }

    pub fn load_balancer_count(&self) -> usize {
        self.load_balancers.len()
    }

    pub fn by_namespace(&self, namespace: &str) -> Vec<&TrafficSplit> {
        self.splits
            .values()
            .filter(|s| s.namespace == namespace)
            .collect()
    }
}

impl Default for TrafficManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_split() {
        let split = TrafficSplit::new("canary-split", "default", "my-service");

        assert_eq!(split.name, "canary-split");
        assert_eq!(split.namespace, "default");
        assert_eq!(split.service, "my-service");
        assert_eq!(split.backends.len(), 0);
    }

    #[test]
    fn test_traffic_backend() {
        let mut split = TrafficSplit::new("split", "default", "service");

        split.add_backend(TrafficBackend {
            service: "service-v1".to_string(),
            weight: 90,
            version: "v1".to_string(),
        });

        split.add_backend(TrafficBackend {
            service: "service-v2".to_string(),
            weight: 10,
            version: "v2".to_string(),
        });

        assert_eq!(split.backends.len(), 2);
        assert_eq!(split.total_weight(), 100);
    }

    #[test]
    fn test_normalize_weights() {
        let mut split = TrafficSplit::new("split", "default", "service");

        split.add_backend(TrafficBackend {
            service: "v1".to_string(),
            weight: 3,
            version: "v1".to_string(),
        });

        split.add_backend(TrafficBackend {
            service: "v2".to_string(),
            weight: 1,
            version: "v2".to_string(),
        });

        split.normalize_weights();
        assert_eq!(split.total_weight(), 100);
        assert_eq!(split.backends[0].weight, 75);
        assert_eq!(split.backends[1].weight, 25);
    }

    #[test]
    fn test_circuit_breaker() {
        let cb = CircuitBreaker::new("my-cb", "default", "my-service");

        assert_eq!(cb.name, "my-cb");
        assert_eq!(cb.target_service, "my-service");
        assert_eq!(cb.consecutive_errors, 5);
        assert_eq!(cb.max_retries, 3);
    }

    #[test]
    fn test_circuit_breaker_builder() {
        let cb = CircuitBreaker::new("cb", "default", "service")
            .with_limits(512, 256, 1024)
            .with_ejection(10, 20, 60);

        assert_eq!(cb.max_connections, 512);
        assert_eq!(cb.max_pending_requests, 256);
        assert_eq!(cb.max_requests, 1024);
        assert_eq!(cb.consecutive_errors, 10);
        assert_eq!(cb.interval_seconds, 20);
        assert_eq!(cb.base_ejection_time_seconds, 60);
    }

    #[test]
    fn test_traffic_mirror() {
        let mirror = TrafficMirror::new("mirror", "default", "source-svc", "mirror-svc");

        assert_eq!(mirror.name, "mirror");
        assert_eq!(mirror.source_service, "source-svc");
        assert_eq!(mirror.mirror_service, "mirror-svc");
        assert_eq!(mirror.percentage, 100.0);
    }

    #[test]
    fn test_traffic_mirror_percentage() {
        let mirror =
            TrafficMirror::new("mirror", "default", "source", "mirror").with_percentage(50.0);

        assert_eq!(mirror.percentage, 50.0);
    }

    #[test]
    fn test_mirror_percentage_clamping() {
        let mirror1 = TrafficMirror::new("m1", "default", "s1", "m1").with_percentage(150.0);
        assert_eq!(mirror1.percentage, 100.0);

        let mirror2 = TrafficMirror::new("m2", "default", "s2", "m2").with_percentage(-10.0);
        assert_eq!(mirror2.percentage, 0.0);
    }

    #[test]
    fn test_load_balancing_strategy() {
        assert_eq!(LoadBalancingStrategy::RoundRobin.to_string(), "Round Robin");
        assert_eq!(
            LoadBalancingStrategy::LeastRequest.to_string(),
            "Least Request"
        );
        assert_eq!(
            LoadBalancingStrategy::ConsistentHash.to_string(),
            "Consistent Hash"
        );
    }

    #[test]
    fn test_load_balancer() {
        let lb = LoadBalancer::new("my-lb", "default", "my-service");

        assert_eq!(lb.name, "my-lb");
        assert_eq!(lb.service, "my-service");
        assert_eq!(lb.strategy, LoadBalancingStrategy::RoundRobin);
        assert!(!lb.session_affinity);
        assert!(lb.health_check_enabled);
    }

    #[test]
    fn test_load_balancer_builder() {
        let lb = LoadBalancer::new("lb", "default", "service")
            .with_strategy(LoadBalancingStrategy::LeastRequest)
            .with_session_affinity("JSESSIONID");

        assert_eq!(lb.strategy, LoadBalancingStrategy::LeastRequest);
        assert!(lb.session_affinity);
        assert_eq!(lb.session_affinity_cookie, Some("JSESSIONID".to_string()));
    }

    #[test]
    fn test_traffic_manager() {
        let mut manager = TrafficManager::new();

        let split = TrafficSplit::new("split", "default", "service");
        let id = manager.add_split(split);

        assert_eq!(manager.split_count(), 1);
        assert!(manager.get_split(&id).is_some());
    }

    #[test]
    fn test_manager_circuit_breakers() {
        let mut manager = TrafficManager::new();

        let cb = CircuitBreaker::new("cb", "default", "service");
        let id = manager.add_circuit_breaker(cb);

        assert_eq!(manager.circuit_breaker_count(), 1);
        assert!(manager.get_circuit_breaker(&id).is_some());
    }

    #[test]
    fn test_manager_mirrors() {
        let mut manager = TrafficManager::new();

        let mirror = TrafficMirror::new("mirror", "default", "source", "mirror");
        let id = manager.add_mirror(mirror);

        assert_eq!(manager.mirror_count(), 1);
        assert!(manager.get_mirror(&id).is_some());
    }

    #[test]
    fn test_manager_load_balancers() {
        let mut manager = TrafficManager::new();

        let lb = LoadBalancer::new("lb", "default", "service");
        let id = manager.add_load_balancer(lb);

        assert_eq!(manager.load_balancer_count(), 1);
        assert!(manager.get_load_balancer(&id).is_some());
    }

    #[test]
    fn test_manager_remove_split() {
        let mut manager = TrafficManager::new();

        let split = TrafficSplit::new("split", "default", "service");
        let id = manager.add_split(split);

        assert!(manager.remove_split(&id));
        assert_eq!(manager.split_count(), 0);
    }

    #[test]
    fn test_manager_remove_circuit_breaker() {
        let mut manager = TrafficManager::new();

        let cb = CircuitBreaker::new("cb", "default", "service");
        let id = manager.add_circuit_breaker(cb);

        assert!(manager.remove_circuit_breaker(&id));
        assert_eq!(manager.circuit_breaker_count(), 0);
    }

    #[test]
    fn test_manager_remove_mirror() {
        let mut manager = TrafficManager::new();

        let mirror = TrafficMirror::new("mirror", "default", "source", "mirror");
        let id = manager.add_mirror(mirror);

        assert!(manager.remove_mirror(&id));
        assert_eq!(manager.mirror_count(), 0);
    }

    #[test]
    fn test_manager_remove_load_balancer() {
        let mut manager = TrafficManager::new();

        let lb = LoadBalancer::new("lb", "default", "service");
        let id = manager.add_load_balancer(lb);

        assert!(manager.remove_load_balancer(&id));
        assert_eq!(manager.load_balancer_count(), 0);
    }

    #[test]
    fn test_manager_by_namespace() {
        let mut manager = TrafficManager::new();

        manager.add_split(TrafficSplit::new("split-1", "ns1", "svc1"));
        manager.add_split(TrafficSplit::new("split-2", "ns1", "svc2"));
        manager.add_split(TrafficSplit::new("split-3", "ns2", "svc3"));

        let ns1_splits = manager.by_namespace("ns1");
        assert_eq!(ns1_splits.len(), 2);
    }

    #[test]
    fn test_manager_get_split_mut() {
        let mut manager = TrafficManager::new();

        let split = TrafficSplit::new("split", "default", "service");
        let id = manager.add_split(split);

        if let Some(split_mut) = manager.get_split_mut(&id) {
            split_mut.add_backend(TrafficBackend {
                service: "v1".to_string(),
                weight: 100,
                version: "v1".to_string(),
            });
        }

        let split = manager.get_split(&id).unwrap();
        assert_eq!(split.backends.len(), 1);
    }

    #[test]
    fn test_load_balancing_strategy_equality() {
        assert_eq!(
            LoadBalancingStrategy::RoundRobin,
            LoadBalancingStrategy::RoundRobin
        );
        assert_ne!(
            LoadBalancingStrategy::RoundRobin,
            LoadBalancingStrategy::Random
        );
    }
}
