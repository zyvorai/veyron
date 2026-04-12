use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{MeshConfig, MeshProvider};

/// Service mesh deployment status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshStatus {
    Installing,
    Ready,
    Updating,
    Failed,
    Uninstalling,
}

impl std::fmt::Display for MeshStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshStatus::Installing => write!(f, "Installing"),
            MeshStatus::Ready => write!(f, "Ready"),
            MeshStatus::Updating => write!(f, "Updating"),
            MeshStatus::Failed => write!(f, "Failed"),
            MeshStatus::Uninstalling => write!(f, "Uninstalling"),
        }
    }
}

/// Service mesh deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshDeployment {
    pub id: String,
    pub name: String,
    pub config: MeshConfig,
    pub status: MeshStatus,
    pub version: String,
    pub control_plane_nodes: u32,
    pub sidecar_injected_workloads: u32,
    pub services_monitored: u32,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl MeshDeployment {
    pub fn new(name: impl Into<String>, config: MeshConfig, version: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "mesh-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            config,
            status: MeshStatus::Installing,
            version: version.into(),
            control_plane_nodes: 3,
            sidecar_injected_workloads: 0,
            services_monitored: 0,
            last_updated: Utc::now(),
            created_at: Utc::now(),
        }
    }

    pub fn update_status(&mut self, status: MeshStatus) {
        self.status = status;
        self.last_updated = Utc::now();
    }

    pub fn is_ready(&self) -> bool {
        self.status == MeshStatus::Ready
    }

    pub fn update_workload_count(&mut self, count: u32) {
        self.sidecar_injected_workloads = count;
        self.last_updated = Utc::now();
    }

    pub fn update_service_count(&mut self, count: u32) {
        self.services_monitored = count;
        self.last_updated = Utc::now();
    }
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gateway {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub gateway_type: GatewayType,
    pub servers: Vec<GatewayServer>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatewayType {
    Ingress,
    Egress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayServer {
    pub port: u16,
    pub protocol: String,
    pub hosts: Vec<String>,
    pub tls_mode: Option<TLSMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TLSMode {
    Passthrough,
    Simple,
    Mutual,
}

impl Gateway {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        gateway_type: GatewayType,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "gw-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            gateway_type,
            servers: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_server(&mut self, server: GatewayServer) {
        self.servers.push(server);
    }

    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    pub fn is_ingress(&self) -> bool {
        self.gateway_type == GatewayType::Ingress
    }

    pub fn is_egress(&self) -> bool {
        self.gateway_type == GatewayType::Egress
    }
}

/// Destination rule for traffic policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationRule {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub host: String,
    pub subsets: Vec<Subset>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subset {
    pub name: String,
    pub labels: HashMap<String, String>,
}

impl DestinationRule {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        host: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "dr-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            host: host.into(),
            subsets: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_subset(&mut self, subset: Subset) {
        self.subsets.push(subset);
    }

    pub fn subset_count(&self) -> usize {
        self.subsets.len()
    }
}

/// Mesh manager coordinating all mesh components
pub struct MeshManager {
    deployments: HashMap<String, MeshDeployment>,
    gateways: HashMap<String, Gateway>,
    destination_rules: HashMap<String, DestinationRule>,
}

impl MeshManager {
    pub fn new() -> Self {
        Self {
            deployments: HashMap::new(),
            gateways: HashMap::new(),
            destination_rules: HashMap::new(),
        }
    }

    pub fn add_deployment(&mut self, deployment: MeshDeployment) -> String {
        let id = deployment.id.clone();
        self.deployments.insert(id.clone(), deployment);
        id
    }

    pub fn get_deployment(&self, id: &str) -> Option<&MeshDeployment> {
        self.deployments.get(id)
    }

    pub fn get_deployment_mut(&mut self, id: &str) -> Option<&mut MeshDeployment> {
        self.deployments.get_mut(id)
    }

    pub fn remove_deployment(&mut self, id: &str) -> bool {
        self.deployments.remove(id).is_some()
    }

    pub fn deployment_count(&self) -> usize {
        self.deployments.len()
    }

    pub fn add_gateway(&mut self, gateway: Gateway) -> String {
        let id = gateway.id.clone();
        self.gateways.insert(id.clone(), gateway);
        id
    }

    pub fn get_gateway(&self, id: &str) -> Option<&Gateway> {
        self.gateways.get(id)
    }

    pub fn remove_gateway(&mut self, id: &str) -> bool {
        self.gateways.remove(id).is_some()
    }

    pub fn gateway_count(&self) -> usize {
        self.gateways.len()
    }

    pub fn add_destination_rule(&mut self, rule: DestinationRule) -> String {
        let id = rule.id.clone();
        self.destination_rules.insert(id.clone(), rule);
        id
    }

    pub fn get_destination_rule(&self, id: &str) -> Option<&DestinationRule> {
        self.destination_rules.get(id)
    }

    pub fn remove_destination_rule(&mut self, id: &str) -> bool {
        self.destination_rules.remove(id).is_some()
    }

    pub fn destination_rule_count(&self) -> usize {
        self.destination_rules.len()
    }

    pub fn ready_deployments(&self) -> Vec<&MeshDeployment> {
        self.deployments.values().filter(|d| d.is_ready()).collect()
    }

    pub fn by_provider(&self, provider: &MeshProvider) -> Vec<&MeshDeployment> {
        self.deployments
            .values()
            .filter(|d| &d.config.provider == provider)
            .collect()
    }

    pub fn ingress_gateways(&self) -> Vec<&Gateway> {
        self.gateways.values().filter(|g| g.is_ingress()).collect()
    }

    pub fn egress_gateways(&self) -> Vec<&Gateway> {
        self.gateways.values().filter(|g| g.is_egress()).collect()
    }
}

impl Default for MeshManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_status_display() {
        assert_eq!(MeshStatus::Installing.to_string(), "Installing");
        assert_eq!(MeshStatus::Ready.to_string(), "Ready");
        assert_eq!(MeshStatus::Failed.to_string(), "Failed");
    }

    #[test]
    fn test_mesh_deployment() {
        let config = MeshConfig::new(MeshProvider::Istio, "default");
        let deployment = MeshDeployment::new("my-mesh", config, "1.18.0");

        assert_eq!(deployment.name, "my-mesh");
        assert_eq!(deployment.version, "1.18.0");
        assert_eq!(deployment.status, MeshStatus::Installing);
        assert!(!deployment.is_ready());
    }

    #[test]
    fn test_deployment_update_status() {
        let config = MeshConfig::new(MeshProvider::Istio, "default");
        let mut deployment = MeshDeployment::new("mesh", config, "1.18.0");

        deployment.update_status(MeshStatus::Ready);

        assert_eq!(deployment.status, MeshStatus::Ready);
        assert!(deployment.is_ready());
    }

    #[test]
    fn test_deployment_update_counts() {
        let config = MeshConfig::new(MeshProvider::Linkerd, "default");
        let mut deployment = MeshDeployment::new("mesh", config, "2.11.0");

        deployment.update_workload_count(10);
        deployment.update_service_count(5);

        assert_eq!(deployment.sidecar_injected_workloads, 10);
        assert_eq!(deployment.services_monitored, 5);
    }

    #[test]
    fn test_gateway() {
        let gateway = Gateway::new("my-gateway", "default", GatewayType::Ingress);

        assert_eq!(gateway.name, "my-gateway");
        assert_eq!(gateway.namespace, "default");
        assert_eq!(gateway.gateway_type, GatewayType::Ingress);
        assert!(gateway.is_ingress());
        assert!(!gateway.is_egress());
    }

    #[test]
    fn test_gateway_servers() {
        let mut gateway = Gateway::new("gateway", "default", GatewayType::Ingress);

        gateway.add_server(GatewayServer {
            port: 80,
            protocol: "HTTP".to_string(),
            hosts: vec!["example.com".to_string()],
            tls_mode: None,
        });

        gateway.add_server(GatewayServer {
            port: 443,
            protocol: "HTTPS".to_string(),
            hosts: vec!["example.com".to_string()],
            tls_mode: Some(TLSMode::Simple),
        });

        assert_eq!(gateway.server_count(), 2);
    }

    #[test]
    fn test_gateway_type() {
        assert_eq!(GatewayType::Ingress, GatewayType::Ingress);
        assert_ne!(GatewayType::Ingress, GatewayType::Egress);
    }

    #[test]
    fn test_tls_mode() {
        assert_eq!(TLSMode::Simple, TLSMode::Simple);
        assert_ne!(TLSMode::Simple, TLSMode::Mutual);
    }

    #[test]
    fn test_destination_rule() {
        let rule = DestinationRule::new("my-rule", "default", "service.example.com");

        assert_eq!(rule.name, "my-rule");
        assert_eq!(rule.namespace, "default");
        assert_eq!(rule.host, "service.example.com");
        assert_eq!(rule.subset_count(), 0);
    }

    #[test]
    fn test_destination_rule_subsets() {
        let mut rule = DestinationRule::new("rule", "default", "service");

        let mut v1_labels = HashMap::new();
        v1_labels.insert("version".to_string(), "v1".to_string());

        let mut v2_labels = HashMap::new();
        v2_labels.insert("version".to_string(), "v2".to_string());

        rule.add_subset(Subset {
            name: "v1".to_string(),
            labels: v1_labels,
        });

        rule.add_subset(Subset {
            name: "v2".to_string(),
            labels: v2_labels,
        });

        assert_eq!(rule.subset_count(), 2);
    }

    #[test]
    fn test_mesh_manager() {
        let mut manager = MeshManager::new();

        let config = MeshConfig::new(MeshProvider::Istio, "default");
        let deployment = MeshDeployment::new("mesh", config, "1.18.0");
        let id = manager.add_deployment(deployment);

        assert_eq!(manager.deployment_count(), 1);
        assert!(manager.get_deployment(&id).is_some());
    }

    #[test]
    fn test_manager_gateways() {
        let mut manager = MeshManager::new();

        let gateway = Gateway::new("gateway", "default", GatewayType::Ingress);
        let id = manager.add_gateway(gateway);

        assert_eq!(manager.gateway_count(), 1);
        assert!(manager.get_gateway(&id).is_some());
    }

    #[test]
    fn test_manager_destination_rules() {
        let mut manager = MeshManager::new();

        let rule = DestinationRule::new("rule", "default", "service");
        let id = manager.add_destination_rule(rule);

        assert_eq!(manager.destination_rule_count(), 1);
        assert!(manager.get_destination_rule(&id).is_some());
    }

    #[test]
    fn test_manager_remove_deployment() {
        let mut manager = MeshManager::new();

        let config = MeshConfig::new(MeshProvider::Consul, "default");
        let deployment = MeshDeployment::new("mesh", config, "1.0.0");
        let id = manager.add_deployment(deployment);

        assert!(manager.remove_deployment(&id));
        assert_eq!(manager.deployment_count(), 0);
    }

    #[test]
    fn test_manager_remove_gateway() {
        let mut manager = MeshManager::new();

        let gateway = Gateway::new("gateway", "default", GatewayType::Egress);
        let id = manager.add_gateway(gateway);

        assert!(manager.remove_gateway(&id));
        assert_eq!(manager.gateway_count(), 0);
    }

    #[test]
    fn test_manager_remove_destination_rule() {
        let mut manager = MeshManager::new();

        let rule = DestinationRule::new("rule", "default", "service");
        let id = manager.add_destination_rule(rule);

        assert!(manager.remove_destination_rule(&id));
        assert_eq!(manager.destination_rule_count(), 0);
    }

    #[test]
    fn test_manager_ready_deployments() {
        let mut manager = MeshManager::new();

        let config1 = MeshConfig::new(MeshProvider::Istio, "ns1");
        let mut deployment1 = MeshDeployment::new("mesh1", config1, "1.18.0");
        deployment1.update_status(MeshStatus::Ready);

        let config2 = MeshConfig::new(MeshProvider::Linkerd, "ns2");
        let deployment2 = MeshDeployment::new("mesh2", config2, "2.11.0");

        manager.add_deployment(deployment1);
        manager.add_deployment(deployment2);

        let ready = manager.ready_deployments();
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn test_manager_by_provider() {
        let mut manager = MeshManager::new();

        let config1 = MeshConfig::new(MeshProvider::Istio, "ns1");
        let config2 = MeshConfig::new(MeshProvider::Istio, "ns2");
        let config3 = MeshConfig::new(MeshProvider::Linkerd, "ns3");

        manager.add_deployment(MeshDeployment::new("m1", config1, "1.18.0"));
        manager.add_deployment(MeshDeployment::new("m2", config2, "1.18.0"));
        manager.add_deployment(MeshDeployment::new("m3", config3, "2.11.0"));

        let istio_deployments = manager.by_provider(&MeshProvider::Istio);
        assert_eq!(istio_deployments.len(), 2);
    }

    #[test]
    fn test_manager_ingress_gateways() {
        let mut manager = MeshManager::new();

        manager.add_gateway(Gateway::new("gw1", "ns1", GatewayType::Ingress));
        manager.add_gateway(Gateway::new("gw2", "ns2", GatewayType::Egress));
        manager.add_gateway(Gateway::new("gw3", "ns3", GatewayType::Ingress));

        let ingress = manager.ingress_gateways();
        assert_eq!(ingress.len(), 2);
    }

    #[test]
    fn test_manager_egress_gateways() {
        let mut manager = MeshManager::new();

        manager.add_gateway(Gateway::new("gw1", "ns1", GatewayType::Ingress));
        manager.add_gateway(Gateway::new("gw2", "ns2", GatewayType::Egress));
        manager.add_gateway(Gateway::new("gw3", "ns3", GatewayType::Egress));

        let egress = manager.egress_gateways();
        assert_eq!(egress.len(), 2);
    }

    #[test]
    fn test_manager_get_deployment_mut() {
        let mut manager = MeshManager::new();

        let config = MeshConfig::new(MeshProvider::Istio, "default");
        let deployment = MeshDeployment::new("mesh", config, "1.18.0");
        let id = manager.add_deployment(deployment);

        if let Some(deployment_mut) = manager.get_deployment_mut(&id) {
            deployment_mut.update_status(MeshStatus::Ready);
        }

        let deployment = manager.get_deployment(&id).unwrap();
        assert!(deployment.is_ready());
    }

    #[test]
    fn test_mesh_status_equality() {
        assert_eq!(MeshStatus::Ready, MeshStatus::Ready);
        assert_ne!(MeshStatus::Ready, MeshStatus::Failed);
    }
}
