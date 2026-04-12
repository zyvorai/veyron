use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod applications;
pub mod devices;
pub mod nodes;
pub mod sync;
pub mod telemetry;

/// Edge location type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationType {
    DataCenter,
    EdgeSite,
    RemoteSite,
    Mobile,
    Gateway,
}

/// Connectivity status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectivityStatus {
    Connected,
    Disconnected,
    Intermittent,
    Degraded,
}

/// Edge deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDeployment {
    pub id: String,
    pub name: String,
    pub location_type: LocationType,
    pub region: String,
    pub connectivity: ConnectivityStatus,
    pub node_ids: Vec<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub bandwidth_mbps: Option<u32>,
    pub created_at: DateTime<Utc>,
}

impl EdgeDeployment {
    pub fn new(
        name: impl Into<String>,
        location_type: LocationType,
        region: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "edge-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            location_type,
            region: region.into(),
            connectivity: ConnectivityStatus::Connected,
            node_ids: Vec::new(),
            latitude: None,
            longitude: None,
            bandwidth_mbps: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_coordinates(mut self, lat: f64, lon: f64) -> Self {
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            log::warn!(
                "EdgeDeployment '{}': coordinates out of range (lat={}, lon={}), clamping to valid range",
                self.name,
                lat,
                lon
            );
        }
        self.latitude = Some(lat.clamp(-90.0, 90.0));
        self.longitude = Some(lon.clamp(-180.0, 180.0));
        self
    }

    pub fn with_bandwidth(mut self, mbps: u32) -> Self {
        self.bandwidth_mbps = Some(mbps);
        self
    }

    pub fn set_connectivity(&mut self, status: ConnectivityStatus) {
        self.connectivity = status;
    }

    pub fn add_node(&mut self, node_id: impl Into<String>) {
        self.node_ids.push(node_id.into());
    }

    pub fn node_count(&self) -> usize {
        self.node_ids.len()
    }

    pub fn is_connected(&self) -> bool {
        self.connectivity == ConnectivityStatus::Connected
    }

    pub fn has_coordinates(&self) -> bool {
        self.latitude.is_some() && self.longitude.is_some()
    }
}

/// Workload placement strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlacementStrategy {
    LatencySensitive,
    BandwidthOptimized,
    CostOptimized,
    DataLocality,
    HighAvailability,
}

/// Edge workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeWorkload {
    pub id: String,
    pub name: String,
    pub deployment_id: String,
    pub strategy: PlacementStrategy,
    pub resource_requirements: ResourceRequirements,
    pub replicas: u32,
    pub active_replicas: u32,
    pub status: WorkloadStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkloadStatus {
    Pending,
    Running,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: u32,
    pub memory_mb: u32,
    pub storage_gb: u32,
    pub gpu_required: bool,
}

impl EdgeWorkload {
    pub fn new(
        name: impl Into<String>,
        deployment_id: impl Into<String>,
        strategy: PlacementStrategy,
        cpu_cores: u32,
        memory_mb: u32,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "workload-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp_micros()
        );

        Self {
            id,
            name: name_str,
            deployment_id: deployment_id.into(),
            strategy,
            resource_requirements: ResourceRequirements {
                cpu_cores,
                memory_mb,
                storage_gb: 0,
                gpu_required: false,
            },
            replicas: 1,
            active_replicas: 0,
            status: WorkloadStatus::Pending,
            created_at: Utc::now(),
        }
    }

    pub fn with_storage(mut self, gb: u32) -> Self {
        self.resource_requirements.storage_gb = gb;
        self
    }

    pub fn require_gpu(mut self) -> Self {
        self.resource_requirements.gpu_required = true;
        self
    }

    pub fn set_replicas(&mut self, replicas: u32) {
        self.replicas = replicas;
    }

    pub fn set_status(&mut self, status: WorkloadStatus) {
        self.status = status;
    }

    pub fn is_healthy(&self) -> bool {
        self.status == WorkloadStatus::Running && self.active_replicas >= self.replicas
    }

    pub fn is_degraded(&self) -> bool {
        self.status == WorkloadStatus::Degraded
            || (self.status == WorkloadStatus::Running && self.active_replicas < self.replicas)
    }
}

/// Edge manager
pub struct EdgeManager {
    deployments: HashMap<String, EdgeDeployment>,
    workloads: HashMap<String, EdgeWorkload>,
}

impl EdgeManager {
    pub fn new() -> Self {
        Self {
            deployments: HashMap::new(),
            workloads: HashMap::new(),
        }
    }

    pub fn add_deployment(&mut self, deployment: EdgeDeployment) -> String {
        let id = deployment.id.clone();
        self.deployments.insert(id.clone(), deployment);
        id
    }

    pub fn get_deployment(&self, id: &str) -> Option<&EdgeDeployment> {
        self.deployments.get(id)
    }

    pub fn get_deployment_mut(&mut self, id: &str) -> Option<&mut EdgeDeployment> {
        self.deployments.get_mut(id)
    }

    pub fn deployment_count(&self) -> usize {
        self.deployments.len()
    }

    pub fn add_workload(&mut self, workload: EdgeWorkload) -> String {
        let id = workload.id.clone();
        self.workloads.insert(id.clone(), workload);
        id
    }

    pub fn get_workload(&self, id: &str) -> Option<&EdgeWorkload> {
        self.workloads.get(id)
    }

    pub fn workload_count(&self) -> usize {
        self.workloads.len()
    }

    pub fn deployments_by_location(&self, location_type: &LocationType) -> Vec<&EdgeDeployment> {
        self.deployments
            .values()
            .filter(|d| &d.location_type == location_type)
            .collect()
    }

    pub fn connected_deployments(&self) -> Vec<&EdgeDeployment> {
        self.deployments
            .values()
            .filter(|d| d.is_connected())
            .collect()
    }

    pub fn workloads_by_deployment(&self, deployment_id: &str) -> Vec<&EdgeWorkload> {
        self.workloads
            .values()
            .filter(|w| w.deployment_id == deployment_id)
            .collect()
    }

    pub fn healthy_workloads(&self) -> Vec<&EdgeWorkload> {
        self.workloads.values().filter(|w| w.is_healthy()).collect()
    }

    pub fn degraded_workloads(&self) -> Vec<&EdgeWorkload> {
        self.workloads
            .values()
            .filter(|w| w.is_degraded())
            .collect()
    }
}

impl Default for EdgeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_deployment() {
        let deployment = EdgeDeployment::new("Site-A", LocationType::EdgeSite, "us-west-1");

        assert_eq!(deployment.name, "Site-A");
        assert_eq!(deployment.location_type, LocationType::EdgeSite);
        assert_eq!(deployment.region, "us-west-1");
        assert_eq!(deployment.connectivity, ConnectivityStatus::Connected);
    }

    #[test]
    fn test_deployment_with_coordinates() {
        let deployment = EdgeDeployment::new("Site-A", LocationType::RemoteSite, "us-west-1")
            .with_coordinates(37.7749, -122.4194);

        assert_eq!(deployment.latitude, Some(37.7749));
        assert_eq!(deployment.longitude, Some(-122.4194));
        assert!(deployment.has_coordinates());
    }

    #[test]
    fn test_deployment_with_bandwidth() {
        let deployment =
            EdgeDeployment::new("Site-A", LocationType::EdgeSite, "us-west-1").with_bandwidth(1000);

        assert_eq!(deployment.bandwidth_mbps, Some(1000));
    }

    #[test]
    fn test_deployment_set_connectivity() {
        let mut deployment = EdgeDeployment::new("Site-A", LocationType::EdgeSite, "us-west-1");

        deployment.set_connectivity(ConnectivityStatus::Degraded);
        assert_eq!(deployment.connectivity, ConnectivityStatus::Degraded);
    }

    #[test]
    fn test_deployment_add_node() {
        let mut deployment = EdgeDeployment::new("Site-A", LocationType::EdgeSite, "us-west-1");

        deployment.add_node("node-1");
        deployment.add_node("node-2");

        assert_eq!(deployment.node_count(), 2);
    }

    #[test]
    fn test_deployment_is_connected() {
        let deployment1 = EdgeDeployment::new("Site-A", LocationType::EdgeSite, "us-west-1");
        assert!(deployment1.is_connected());

        let mut deployment2 = EdgeDeployment::new("Site-B", LocationType::RemoteSite, "us-east-1");
        deployment2.set_connectivity(ConnectivityStatus::Disconnected);
        assert!(!deployment2.is_connected());
    }

    #[test]
    fn test_edge_workload() {
        let workload = EdgeWorkload::new(
            "app-1",
            "edge-site-a",
            PlacementStrategy::LatencySensitive,
            2,
            4096,
        );

        assert_eq!(workload.name, "app-1");
        assert_eq!(workload.deployment_id, "edge-site-a");
        assert_eq!(workload.strategy, PlacementStrategy::LatencySensitive);
        assert_eq!(workload.resource_requirements.cpu_cores, 2);
        assert_eq!(workload.resource_requirements.memory_mb, 4096);
        assert_eq!(workload.status, WorkloadStatus::Pending);
    }

    #[test]
    fn test_workload_with_storage() {
        let workload =
            EdgeWorkload::new("app-1", "edge-1", PlacementStrategy::DataLocality, 2, 4096)
                .with_storage(100);

        assert_eq!(workload.resource_requirements.storage_gb, 100);
    }

    #[test]
    fn test_workload_require_gpu() {
        let workload = EdgeWorkload::new(
            "ml-app",
            "edge-1",
            PlacementStrategy::LatencySensitive,
            4,
            8192,
        )
        .require_gpu();

        assert!(workload.resource_requirements.gpu_required);
    }

    #[test]
    fn test_workload_set_replicas() {
        let mut workload = EdgeWorkload::new(
            "app-1",
            "edge-1",
            PlacementStrategy::HighAvailability,
            2,
            4096,
        );

        workload.set_replicas(3);
        assert_eq!(workload.replicas, 3);
    }

    #[test]
    fn test_workload_set_status() {
        let mut workload = EdgeWorkload::new(
            "app-1",
            "edge-1",
            PlacementStrategy::LatencySensitive,
            2,
            4096,
        );

        workload.set_status(WorkloadStatus::Running);
        assert_eq!(workload.status, WorkloadStatus::Running);
    }

    #[test]
    fn test_workload_is_healthy() {
        let mut workload = EdgeWorkload::new(
            "app-1",
            "edge-1",
            PlacementStrategy::LatencySensitive,
            2,
            4096,
        );
        workload.set_replicas(2);
        workload.active_replicas = 2;
        workload.set_status(WorkloadStatus::Running);

        assert!(workload.is_healthy());
    }

    #[test]
    fn test_workload_is_degraded() {
        let mut workload1 = EdgeWorkload::new(
            "app-1",
            "edge-1",
            PlacementStrategy::HighAvailability,
            2,
            4096,
        );
        workload1.set_status(WorkloadStatus::Degraded);
        assert!(workload1.is_degraded());

        let mut workload2 = EdgeWorkload::new(
            "app-2",
            "edge-1",
            PlacementStrategy::HighAvailability,
            2,
            4096,
        );
        workload2.set_replicas(3);
        workload2.active_replicas = 1;
        workload2.set_status(WorkloadStatus::Running);
        assert!(workload2.is_degraded());
    }

    #[test]
    fn test_edge_manager() {
        let mut manager = EdgeManager::new();

        let deployment = EdgeDeployment::new("Site-A", LocationType::EdgeSite, "us-west-1");
        let id = manager.add_deployment(deployment);

        assert_eq!(manager.deployment_count(), 1);
        assert!(manager.get_deployment(&id).is_some());
    }

    #[test]
    fn test_manager_add_workload() {
        let mut manager = EdgeManager::new();

        let workload = EdgeWorkload::new(
            "app-1",
            "edge-1",
            PlacementStrategy::LatencySensitive,
            2,
            4096,
        );
        let id = manager.add_workload(workload);

        assert_eq!(manager.workload_count(), 1);
        assert!(manager.get_workload(&id).is_some());
    }

    #[test]
    fn test_manager_deployments_by_location() {
        let mut manager = EdgeManager::new();

        manager.add_deployment(EdgeDeployment::new(
            "Site-A",
            LocationType::EdgeSite,
            "us-west-1",
        ));
        manager.add_deployment(EdgeDeployment::new(
            "Site-B",
            LocationType::RemoteSite,
            "us-east-1",
        ));
        manager.add_deployment(EdgeDeployment::new(
            "Site-C",
            LocationType::EdgeSite,
            "eu-west-1",
        ));

        let edge_sites = manager.deployments_by_location(&LocationType::EdgeSite);
        assert_eq!(edge_sites.len(), 2);
    }

    #[test]
    fn test_manager_connected_deployments() {
        let mut manager = EdgeManager::new();

        let deployment1 = EdgeDeployment::new("Site-A", LocationType::EdgeSite, "us-west-1");
        let mut deployment2 = EdgeDeployment::new("Site-B", LocationType::RemoteSite, "us-east-1");
        deployment2.set_connectivity(ConnectivityStatus::Disconnected);

        manager.add_deployment(deployment1);
        manager.add_deployment(deployment2);

        let connected = manager.connected_deployments();
        assert_eq!(connected.len(), 1);
    }

    #[test]
    fn test_manager_workloads_by_deployment() {
        let mut manager = EdgeManager::new();

        manager.add_workload(EdgeWorkload::new(
            "app-1",
            "edge-1",
            PlacementStrategy::LatencySensitive,
            2,
            4096,
        ));
        manager.add_workload(EdgeWorkload::new(
            "app-2",
            "edge-2",
            PlacementStrategy::DataLocality,
            2,
            4096,
        ));
        manager.add_workload(EdgeWorkload::new(
            "app-3",
            "edge-1",
            PlacementStrategy::HighAvailability,
            2,
            4096,
        ));

        let edge1_workloads = manager.workloads_by_deployment("edge-1");
        assert_eq!(edge1_workloads.len(), 2);
    }

    #[test]
    fn test_manager_healthy_workloads() {
        let mut manager = EdgeManager::new();

        let mut workload1 = EdgeWorkload::new(
            "app-1",
            "edge-1",
            PlacementStrategy::LatencySensitive,
            2,
            4096,
        );
        workload1.set_replicas(2);
        workload1.active_replicas = 2;
        workload1.set_status(WorkloadStatus::Running);

        let workload2 =
            EdgeWorkload::new("app-2", "edge-1", PlacementStrategy::DataLocality, 2, 4096);

        manager.add_workload(workload1);
        manager.add_workload(workload2);

        let healthy = manager.healthy_workloads();
        assert_eq!(healthy.len(), 1);
    }

    #[test]
    fn test_manager_degraded_workloads() {
        let mut manager = EdgeManager::new();

        let mut workload1 = EdgeWorkload::new(
            "app-1",
            "edge-1",
            PlacementStrategy::HighAvailability,
            2,
            4096,
        );
        workload1.set_status(WorkloadStatus::Degraded);

        let mut workload2 = EdgeWorkload::new(
            "app-2",
            "edge-1",
            PlacementStrategy::LatencySensitive,
            2,
            4096,
        );
        workload2.set_replicas(2);
        workload2.active_replicas = 2;
        workload2.set_status(WorkloadStatus::Running);

        manager.add_workload(workload1);
        manager.add_workload(workload2);

        let degraded = manager.degraded_workloads();
        assert_eq!(degraded.len(), 1);
    }

    #[test]
    fn test_location_type_equality() {
        assert_eq!(LocationType::EdgeSite, LocationType::EdgeSite);
        assert_ne!(LocationType::EdgeSite, LocationType::RemoteSite);
    }

    #[test]
    fn test_connectivity_status_equality() {
        assert_eq!(ConnectivityStatus::Connected, ConnectivityStatus::Connected);
        assert_ne!(
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected
        );
    }

    #[test]
    fn test_placement_strategy_equality() {
        assert_eq!(
            PlacementStrategy::LatencySensitive,
            PlacementStrategy::LatencySensitive
        );
        assert_ne!(
            PlacementStrategy::LatencySensitive,
            PlacementStrategy::CostOptimized
        );
    }

    #[test]
    fn test_workload_status_equality() {
        assert_eq!(WorkloadStatus::Running, WorkloadStatus::Running);
        assert_ne!(WorkloadStatus::Running, WorkloadStatus::Failed);
    }
}
