use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod mesh;
pub mod observability;
pub mod policies;
pub mod security;
pub mod traffic;

/// Service mesh provider types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshProvider {
    Istio,
    Linkerd,
    Consul,
    OSM, // Open Service Mesh
    Kuma,
}

impl std::fmt::Display for MeshProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshProvider::Istio => write!(f, "Istio"),
            MeshProvider::Linkerd => write!(f, "Linkerd"),
            MeshProvider::Consul => write!(f, "Consul"),
            MeshProvider::OSM => write!(f, "Open Service Mesh"),
            MeshProvider::Kuma => write!(f, "Kuma"),
        }
    }
}

/// Service mesh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshConfig {
    pub provider: MeshProvider,
    pub namespace: String,
    pub mtls_enabled: bool,
    pub auto_inject: bool,
    pub ingress_enabled: bool,
    pub egress_enabled: bool,
    pub telemetry_enabled: bool,
    pub tracing_sample_rate: f64,
    pub created_at: DateTime<Utc>,
}

impl MeshConfig {
    pub fn new(provider: MeshProvider, namespace: impl Into<String>) -> Self {
        Self {
            provider,
            namespace: namespace.into(),
            mtls_enabled: true,
            auto_inject: true,
            ingress_enabled: true,
            egress_enabled: true,
            telemetry_enabled: true,
            tracing_sample_rate: 0.1,
            created_at: Utc::now(),
        }
    }

    pub fn with_mtls(mut self, enabled: bool) -> Self {
        self.mtls_enabled = enabled;
        self
    }

    pub fn with_auto_inject(mut self, enabled: bool) -> Self {
        self.auto_inject = enabled;
        self
    }

    pub fn with_tracing_rate(mut self, rate: f64) -> Self {
        self.tracing_sample_rate = rate.clamp(0.0, 1.0);
        self
    }
}

/// Service entry for VM workloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub hosts: Vec<String>,
    pub ports: Vec<ServicePort>,
    pub location: ServiceLocation,
    pub resolution: ServiceResolution,
    pub endpoints: Vec<ServiceEndpoint>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePort {
    pub number: u16,
    pub protocol: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceLocation {
    MeshInternal,
    MeshExternal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceResolution {
    Static,
    DNS,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub address: String,
    pub port: u16,
    pub labels: HashMap<String, String>,
    pub weight: u32,
}

impl ServiceEntry {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "se-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            hosts: Vec::new(),
            ports: Vec::new(),
            location: ServiceLocation::MeshInternal,
            resolution: ServiceResolution::DNS,
            endpoints: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_host(&mut self, host: impl Into<String>) {
        self.hosts.push(host.into());
    }

    pub fn add_port(&mut self, port: ServicePort) {
        self.ports.push(port);
    }

    pub fn add_endpoint(&mut self, endpoint: ServiceEndpoint) {
        self.endpoints.push(endpoint);
    }

    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }
}

/// Virtual service for traffic routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualService {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub hosts: Vec<String>,
    pub gateways: Vec<String>,
    pub http_routes: Vec<HTTPRoute>,
    pub tcp_routes: Vec<TCPRoute>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPRoute {
    pub name: String,
    pub match_conditions: Vec<HTTPMatchRequest>,
    pub route: Vec<HTTPRouteDestination>,
    pub timeout_seconds: Option<u64>,
    pub retries: Option<HTTPRetry>,
    pub fault_injection: Option<HTTPFaultInjection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPMatchRequest {
    pub uri: Option<StringMatch>,
    pub method: Option<StringMatch>,
    pub headers: HashMap<String, StringMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StringMatch {
    Exact(String),
    Prefix(String),
    Regex(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPRouteDestination {
    pub host: String,
    pub subset: Option<String>,
    pub port: Option<u16>,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPRetry {
    pub attempts: u32,
    pub per_try_timeout_seconds: u64,
    pub retry_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPFaultInjection {
    pub delay: Option<FaultDelay>,
    pub abort: Option<FaultAbort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultDelay {
    pub percentage: f64,
    pub fixed_delay_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultAbort {
    pub percentage: f64,
    pub http_status: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TCPRoute {
    pub route: Vec<TCPRouteDestination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TCPRouteDestination {
    pub host: String,
    pub port: u16,
    pub weight: u32,
}

impl VirtualService {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "vs-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            hosts: Vec::new(),
            gateways: Vec::new(),
            http_routes: Vec::new(),
            tcp_routes: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_host(&mut self, host: impl Into<String>) {
        self.hosts.push(host.into());
    }

    pub fn add_gateway(&mut self, gateway: impl Into<String>) {
        self.gateways.push(gateway.into());
    }

    pub fn add_http_route(&mut self, route: HTTPRoute) {
        self.http_routes.push(route);
    }

    pub fn add_tcp_route(&mut self, route: TCPRoute) {
        self.tcp_routes.push(route);
    }

    pub fn route_count(&self) -> usize {
        self.http_routes.len() + self.tcp_routes.len()
    }
}

/// Service mesh manager
pub struct ServiceMeshManager {
    configs: HashMap<String, MeshConfig>,
    service_entries: HashMap<String, ServiceEntry>,
    virtual_services: HashMap<String, VirtualService>,
}

impl ServiceMeshManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            service_entries: HashMap::new(),
            virtual_services: HashMap::new(),
        }
    }

    pub fn add_config(&mut self, config: MeshConfig) -> String {
        let id = format!("mesh-{}-{}", config.namespace, Utc::now().timestamp());
        self.configs.insert(id.clone(), config);
        id
    }

    pub fn get_config(&self, id: &str) -> Option<&MeshConfig> {
        self.configs.get(id)
    }

    pub fn remove_config(&mut self, id: &str) -> bool {
        self.configs.remove(id).is_some()
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    pub fn add_service_entry(&mut self, entry: ServiceEntry) -> String {
        let id = entry.id.clone();
        self.service_entries.insert(id.clone(), entry);
        id
    }

    pub fn get_service_entry(&self, id: &str) -> Option<&ServiceEntry> {
        self.service_entries.get(id)
    }

    pub fn remove_service_entry(&mut self, id: &str) -> bool {
        self.service_entries.remove(id).is_some()
    }

    pub fn service_entry_count(&self) -> usize {
        self.service_entries.len()
    }

    pub fn add_virtual_service(&mut self, service: VirtualService) -> String {
        let id = service.id.clone();
        self.virtual_services.insert(id.clone(), service);
        id
    }

    pub fn get_virtual_service(&self, id: &str) -> Option<&VirtualService> {
        self.virtual_services.get(id)
    }

    pub fn remove_virtual_service(&mut self, id: &str) -> bool {
        self.virtual_services.remove(id).is_some()
    }

    pub fn virtual_service_count(&self) -> usize {
        self.virtual_services.len()
    }

    pub fn by_namespace(&self, namespace: &str) -> Vec<&ServiceEntry> {
        self.service_entries
            .values()
            .filter(|e| e.namespace == namespace)
            .collect()
    }

    pub fn by_provider(&self, provider: &MeshProvider) -> Vec<&MeshConfig> {
        self.configs
            .values()
            .filter(|c| &c.provider == provider)
            .collect()
    }
}

impl Default for ServiceMeshManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_provider_display() {
        assert_eq!(MeshProvider::Istio.to_string(), "Istio");
        assert_eq!(MeshProvider::Linkerd.to_string(), "Linkerd");
        assert_eq!(MeshProvider::Consul.to_string(), "Consul");
    }

    #[test]
    fn test_mesh_config() {
        let config = MeshConfig::new(MeshProvider::Istio, "default");

        assert_eq!(config.provider, MeshProvider::Istio);
        assert_eq!(config.namespace, "default");
        assert!(config.mtls_enabled);
        assert!(config.auto_inject);
    }

    #[test]
    fn test_mesh_config_builder() {
        let config = MeshConfig::new(MeshProvider::Linkerd, "production")
            .with_mtls(false)
            .with_tracing_rate(0.5);

        assert!(!config.mtls_enabled);
        assert_eq!(config.tracing_sample_rate, 0.5);
    }

    #[test]
    fn test_tracing_rate_clamping() {
        let config1 = MeshConfig::new(MeshProvider::Istio, "test").with_tracing_rate(1.5);
        assert_eq!(config1.tracing_sample_rate, 1.0);

        let config2 = MeshConfig::new(MeshProvider::Istio, "test").with_tracing_rate(-0.5);
        assert_eq!(config2.tracing_sample_rate, 0.0);
    }

    #[test]
    fn test_service_entry() {
        let mut entry = ServiceEntry::new("my-service", "default");

        entry.add_host("service.example.com");
        entry.add_port(ServicePort {
            number: 8080,
            protocol: "HTTP".to_string(),
            name: "http".to_string(),
        });

        assert_eq!(entry.name, "my-service");
        assert_eq!(entry.hosts.len(), 1);
        assert_eq!(entry.ports.len(), 1);
    }

    #[test]
    fn test_service_endpoint() {
        let mut entry = ServiceEntry::new("service", "default");

        let endpoint = ServiceEndpoint {
            address: "192.168.1.100".to_string(),
            port: 8080,
            labels: HashMap::new(),
            weight: 100,
        };

        entry.add_endpoint(endpoint);
        assert_eq!(entry.endpoint_count(), 1);
    }

    #[test]
    fn test_service_location() {
        assert_eq!(ServiceLocation::MeshInternal, ServiceLocation::MeshInternal);
        assert_ne!(ServiceLocation::MeshInternal, ServiceLocation::MeshExternal);
    }

    #[test]
    fn test_service_resolution() {
        assert_eq!(ServiceResolution::DNS, ServiceResolution::DNS);
        assert_ne!(ServiceResolution::DNS, ServiceResolution::Static);
    }

    #[test]
    fn test_virtual_service() {
        let mut vs = VirtualService::new("my-vs", "default");

        vs.add_host("example.com");
        vs.add_gateway("my-gateway");

        assert_eq!(vs.name, "my-vs");
        assert_eq!(vs.hosts.len(), 1);
        assert_eq!(vs.gateways.len(), 1);
    }

    #[test]
    fn test_http_route() {
        let route = HTTPRoute {
            name: "route-1".to_string(),
            match_conditions: vec![],
            route: vec![HTTPRouteDestination {
                host: "service-v1".to_string(),
                subset: Some("v1".to_string()),
                port: Some(8080),
                weight: 80,
            }],
            timeout_seconds: Some(30),
            retries: None,
            fault_injection: None,
        };

        assert_eq!(route.name, "route-1");
        assert_eq!(route.route.len(), 1);
        assert_eq!(route.timeout_seconds, Some(30));
    }

    #[test]
    fn test_string_match() {
        let exact = StringMatch::Exact("/api/v1".to_string());
        let prefix = StringMatch::Prefix("/api".to_string());
        let regex = StringMatch::Regex("^/api/.*".to_string());

        match exact {
            StringMatch::Exact(s) => assert_eq!(s, "/api/v1"),
            _ => panic!("Expected Exact match"),
        }

        match prefix {
            StringMatch::Prefix(s) => assert_eq!(s, "/api"),
            _ => panic!("Expected Prefix match"),
        }

        match regex {
            StringMatch::Regex(s) => assert_eq!(s, "^/api/.*"),
            _ => panic!("Expected Regex match"),
        }
    }

    #[test]
    fn test_fault_injection() {
        let fault = HTTPFaultInjection {
            delay: Some(FaultDelay {
                percentage: 10.0,
                fixed_delay_seconds: 5,
            }),
            abort: Some(FaultAbort {
                percentage: 5.0,
                http_status: 500,
            }),
        };

        assert!(fault.delay.is_some());
        assert!(fault.abort.is_some());
        assert_eq!(fault.delay.unwrap().percentage, 10.0);
        assert_eq!(fault.abort.unwrap().http_status, 500);
    }

    #[test]
    fn test_http_retry() {
        let retry = HTTPRetry {
            attempts: 3,
            per_try_timeout_seconds: 10,
            retry_on: vec!["5xx".to_string(), "reset".to_string()],
        };

        assert_eq!(retry.attempts, 3);
        assert_eq!(retry.retry_on.len(), 2);
    }

    #[test]
    fn test_tcp_route() {
        let route = TCPRoute {
            route: vec![TCPRouteDestination {
                host: "tcp-service".to_string(),
                port: 3306,
                weight: 100,
            }],
        };

        assert_eq!(route.route.len(), 1);
        assert_eq!(route.route[0].port, 3306);
    }

    #[test]
    fn test_virtual_service_routes() {
        let mut vs = VirtualService::new("test-vs", "default");

        vs.add_http_route(HTTPRoute {
            name: "route-1".to_string(),
            match_conditions: vec![],
            route: vec![],
            timeout_seconds: None,
            retries: None,
            fault_injection: None,
        });

        vs.add_tcp_route(TCPRoute { route: vec![] });

        assert_eq!(vs.route_count(), 2);
    }

    #[test]
    fn test_service_mesh_manager() {
        let mut manager = ServiceMeshManager::new();

        let config = MeshConfig::new(MeshProvider::Istio, "default");
        let id = manager.add_config(config);

        assert_eq!(manager.config_count(), 1);
        assert!(manager.get_config(&id).is_some());
    }

    #[test]
    fn test_manager_service_entries() {
        let mut manager = ServiceMeshManager::new();

        let entry = ServiceEntry::new("service-1", "default");
        let id = manager.add_service_entry(entry);

        assert_eq!(manager.service_entry_count(), 1);
        assert!(manager.get_service_entry(&id).is_some());
    }

    #[test]
    fn test_manager_virtual_services() {
        let mut manager = ServiceMeshManager::new();

        let vs = VirtualService::new("vs-1", "default");
        let id = manager.add_virtual_service(vs);

        assert_eq!(manager.virtual_service_count(), 1);
        assert!(manager.get_virtual_service(&id).is_some());
    }

    #[test]
    fn test_manager_by_namespace() {
        let mut manager = ServiceMeshManager::new();

        manager.add_service_entry(ServiceEntry::new("svc-1", "ns1"));
        manager.add_service_entry(ServiceEntry::new("svc-2", "ns1"));
        manager.add_service_entry(ServiceEntry::new("svc-3", "ns2"));

        let ns1_services = manager.by_namespace("ns1");
        assert_eq!(ns1_services.len(), 2);
    }

    #[test]
    fn test_manager_by_provider() {
        let mut manager = ServiceMeshManager::new();

        manager.add_config(MeshConfig::new(MeshProvider::Istio, "ns1"));
        manager.add_config(MeshConfig::new(MeshProvider::Istio, "ns2"));
        manager.add_config(MeshConfig::new(MeshProvider::Linkerd, "ns3"));

        let istio_configs = manager.by_provider(&MeshProvider::Istio);
        assert_eq!(istio_configs.len(), 2);
    }

    #[test]
    fn test_manager_remove_config() {
        let mut manager = ServiceMeshManager::new();

        let config = MeshConfig::new(MeshProvider::Consul, "default");
        let id = manager.add_config(config);

        assert!(manager.remove_config(&id));
        assert_eq!(manager.config_count(), 0);
    }

    #[test]
    fn test_manager_remove_service_entry() {
        let mut manager = ServiceMeshManager::new();

        let entry = ServiceEntry::new("service", "default");
        let id = manager.add_service_entry(entry);

        assert!(manager.remove_service_entry(&id));
        assert_eq!(manager.service_entry_count(), 0);
    }

    #[test]
    fn test_manager_remove_virtual_service() {
        let mut manager = ServiceMeshManager::new();

        let vs = VirtualService::new("vs", "default");
        let id = manager.add_virtual_service(vs);

        assert!(manager.remove_virtual_service(&id));
        assert_eq!(manager.virtual_service_count(), 0);
    }

    #[test]
    fn test_mesh_provider() {
        assert_eq!(MeshProvider::Istio, MeshProvider::Istio);
        assert_ne!(MeshProvider::Istio, MeshProvider::Linkerd);
    }
}
