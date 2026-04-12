use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod connectivity;
pub mod federation;
pub mod portability;
pub mod providers;
pub mod workloads;

/// Cloud provider type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CloudProvider {
    AWS,
    Azure,
    GCP,
    VMware,
    OpenStack,
    KubeVirt,
    Custom(String),
}

impl std::fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudProvider::AWS => write!(f, "AWS"),
            CloudProvider::Azure => write!(f, "Azure"),
            CloudProvider::GCP => write!(f, "GCP"),
            CloudProvider::VMware => write!(f, "VMware"),
            CloudProvider::OpenStack => write!(f, "OpenStack"),
            CloudProvider::KubeVirt => write!(f, "KubeVirt"),
            CloudProvider::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Cloud region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudRegion {
    pub id: String,
    pub name: String,
    pub provider: CloudProvider,
    pub location: String,
    pub availability_zones: Vec<String>,
    pub endpoint: String,
    pub enabled: bool,
    pub latency_ms: Option<u32>,
    pub created_at: DateTime<Utc>,
}

impl CloudRegion {
    pub fn new(
        name: impl Into<String>,
        provider: CloudProvider,
        location: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "region-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            provider,
            location: location.into(),
            availability_zones: Vec::new(),
            endpoint: endpoint.into(),
            enabled: true,
            latency_ms: None,
            created_at: Utc::now(),
        }
    }

    pub fn add_zone(&mut self, zone: impl Into<String>) {
        self.availability_zones.push(zone.into());
    }

    pub fn with_latency(mut self, latency_ms: u32) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn zone_count(&self) -> usize {
        self.availability_zones.len()
    }
}

/// Deployment target
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentTarget {
    SingleCloud,
    MultiCloud,
    HybridCloud,
}

/// Multi-cloud deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCloudDeployment {
    pub id: String,
    pub name: String,
    pub target: DeploymentTarget,
    pub primary_provider: CloudProvider,
    pub secondary_providers: Vec<CloudProvider>,
    pub regions: Vec<String>,
    pub workload_ids: Vec<String>,
    pub failover_enabled: bool,
    pub load_balancing_enabled: bool,
    pub status: DeploymentStatus,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Deployment status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Pending,
    Deploying,
    Active,
    Degraded,
    Failed,
    Terminated,
}

impl MultiCloudDeployment {
    pub fn new(
        name: impl Into<String>,
        target: DeploymentTarget,
        primary_provider: CloudProvider,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "mcd-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            target,
            primary_provider,
            secondary_providers: Vec::new(),
            regions: Vec::new(),
            workload_ids: Vec::new(),
            failover_enabled: false,
            load_balancing_enabled: false,
            status: DeploymentStatus::Pending,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    pub fn add_provider(&mut self, provider: CloudProvider) {
        if !self.secondary_providers.contains(&provider) {
            self.secondary_providers.push(provider);
        }
    }

    pub fn add_region(&mut self, region_id: impl Into<String>) {
        self.regions.push(region_id.into());
    }

    pub fn add_workload(&mut self, workload_id: impl Into<String>) {
        self.workload_ids.push(workload_id.into());
    }

    pub fn enable_failover(mut self) -> Self {
        self.failover_enabled = true;
        self
    }

    pub fn enable_load_balancing(mut self) -> Self {
        self.load_balancing_enabled = true;
        self
    }

    pub fn set_status(&mut self, status: DeploymentStatus) {
        self.status = status;
        self.last_updated = Utc::now();
    }

    pub fn is_active(&self) -> bool {
        self.status == DeploymentStatus::Active
    }

    pub fn is_multi_provider(&self) -> bool {
        !self.secondary_providers.is_empty()
    }

    pub fn provider_count(&self) -> usize {
        1 + self.secondary_providers.len()
    }

    pub fn workload_count(&self) -> usize {
        self.workload_ids.len()
    }
}

/// Multi-cloud manager
pub struct MultiCloudManager {
    regions: HashMap<String, CloudRegion>,
    deployments: HashMap<String, MultiCloudDeployment>,
}

impl MultiCloudManager {
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            deployments: HashMap::new(),
        }
    }

    pub fn add_region(&mut self, region: CloudRegion) -> String {
        let id = region.id.clone();
        self.regions.insert(id.clone(), region);
        id
    }

    pub fn get_region(&self, id: &str) -> Option<&CloudRegion> {
        self.regions.get(id)
    }

    pub fn get_region_mut(&mut self, id: &str) -> Option<&mut CloudRegion> {
        self.regions.get_mut(id)
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    pub fn add_deployment(&mut self, deployment: MultiCloudDeployment) -> String {
        let id = deployment.id.clone();
        self.deployments.insert(id.clone(), deployment);
        id
    }

    pub fn get_deployment(&self, id: &str) -> Option<&MultiCloudDeployment> {
        self.deployments.get(id)
    }

    pub fn get_deployment_mut(&mut self, id: &str) -> Option<&mut MultiCloudDeployment> {
        self.deployments.get_mut(id)
    }

    pub fn deployment_count(&self) -> usize {
        self.deployments.len()
    }

    pub fn regions_by_provider(&self, provider: &CloudProvider) -> Vec<&CloudRegion> {
        self.regions
            .values()
            .filter(|r| &r.provider == provider)
            .collect()
    }

    pub fn enabled_regions(&self) -> Vec<&CloudRegion> {
        self.regions.values().filter(|r| r.enabled).collect()
    }

    pub fn deployments_by_provider(&self, provider: &CloudProvider) -> Vec<&MultiCloudDeployment> {
        self.deployments
            .values()
            .filter(|d| &d.primary_provider == provider)
            .collect()
    }

    pub fn active_deployments(&self) -> Vec<&MultiCloudDeployment> {
        self.deployments
            .values()
            .filter(|d| d.is_active())
            .collect()
    }

    pub fn multi_provider_deployments(&self) -> Vec<&MultiCloudDeployment> {
        self.deployments
            .values()
            .filter(|d| d.is_multi_provider())
            .collect()
    }

    pub fn deployments_with_failover(&self) -> Vec<&MultiCloudDeployment> {
        self.deployments
            .values()
            .filter(|d| d.failover_enabled)
            .collect()
    }
}

impl Default for MultiCloudManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_region() {
        let region = CloudRegion::new(
            "us-east-1",
            CloudProvider::AWS,
            "Virginia",
            "https://aws-us-east-1.com",
        );

        assert_eq!(region.name, "us-east-1");
        assert_eq!(region.provider, CloudProvider::AWS);
        assert_eq!(region.location, "Virginia");
        assert!(region.enabled);
    }

    #[test]
    fn test_region_add_zone() {
        let mut region = CloudRegion::new(
            "us-west-2",
            CloudProvider::AWS,
            "Oregon",
            "https://endpoint",
        );

        region.add_zone("us-west-2a");
        region.add_zone("us-west-2b");

        assert_eq!(region.zone_count(), 2);
    }

    #[test]
    fn test_region_with_latency() {
        let region = CloudRegion::new(
            "eu-west-1",
            CloudProvider::AWS,
            "Ireland",
            "https://endpoint",
        )
        .with_latency(50);

        assert_eq!(region.latency_ms, Some(50));
    }

    #[test]
    fn test_region_enable_disable() {
        let mut region =
            CloudRegion::new("test", CloudProvider::Azure, "East US", "https://endpoint");

        assert!(region.enabled);

        region.disable();
        assert!(!region.enabled);

        region.enable();
        assert!(region.enabled);
    }

    #[test]
    fn test_multicloud_deployment() {
        let deployment = MultiCloudDeployment::new(
            "hybrid-app",
            DeploymentTarget::HybridCloud,
            CloudProvider::AWS,
        );

        assert_eq!(deployment.name, "hybrid-app");
        assert_eq!(deployment.target, DeploymentTarget::HybridCloud);
        assert_eq!(deployment.primary_provider, CloudProvider::AWS);
        assert_eq!(deployment.status, DeploymentStatus::Pending);
    }

    #[test]
    fn test_deployment_add_provider() {
        let mut deployment =
            MultiCloudDeployment::new("app", DeploymentTarget::MultiCloud, CloudProvider::AWS);

        deployment.add_provider(CloudProvider::Azure);
        deployment.add_provider(CloudProvider::GCP);
        deployment.add_provider(CloudProvider::Azure); // Duplicate

        assert_eq!(deployment.secondary_providers.len(), 2);
        assert_eq!(deployment.provider_count(), 3);
    }

    #[test]
    fn test_deployment_add_region() {
        let mut deployment =
            MultiCloudDeployment::new("app", DeploymentTarget::SingleCloud, CloudProvider::GCP);

        deployment.add_region("us-central1");
        deployment.add_region("europe-west1");

        assert_eq!(deployment.regions.len(), 2);
    }

    #[test]
    fn test_deployment_add_workload() {
        let mut deployment =
            MultiCloudDeployment::new("app", DeploymentTarget::MultiCloud, CloudProvider::AWS);

        deployment.add_workload("workload-1");
        deployment.add_workload("workload-2");

        assert_eq!(deployment.workload_count(), 2);
    }

    #[test]
    fn test_deployment_enable_failover() {
        let deployment =
            MultiCloudDeployment::new("app", DeploymentTarget::MultiCloud, CloudProvider::AWS)
                .enable_failover();

        assert!(deployment.failover_enabled);
    }

    #[test]
    fn test_deployment_enable_load_balancing() {
        let deployment =
            MultiCloudDeployment::new("app", DeploymentTarget::MultiCloud, CloudProvider::AWS)
                .enable_load_balancing();

        assert!(deployment.load_balancing_enabled);
    }

    #[test]
    fn test_deployment_set_status() {
        let mut deployment =
            MultiCloudDeployment::new("app", DeploymentTarget::SingleCloud, CloudProvider::Azure);

        deployment.set_status(DeploymentStatus::Active);
        assert_eq!(deployment.status, DeploymentStatus::Active);
    }

    #[test]
    fn test_deployment_is_active() {
        let mut deployment =
            MultiCloudDeployment::new("app", DeploymentTarget::SingleCloud, CloudProvider::GCP);

        assert!(!deployment.is_active());

        deployment.set_status(DeploymentStatus::Active);
        assert!(deployment.is_active());
    }

    #[test]
    fn test_deployment_is_multi_provider() {
        let mut deployment =
            MultiCloudDeployment::new("app", DeploymentTarget::MultiCloud, CloudProvider::AWS);

        assert!(!deployment.is_multi_provider());

        deployment.add_provider(CloudProvider::Azure);
        assert!(deployment.is_multi_provider());
    }

    #[test]
    fn test_multicloud_manager() {
        let mut manager = MultiCloudManager::new();

        let region = CloudRegion::new(
            "us-east-1",
            CloudProvider::AWS,
            "Virginia",
            "https://endpoint",
        );
        let id = manager.add_region(region);

        assert_eq!(manager.region_count(), 1);
        assert!(manager.get_region(&id).is_some());
    }

    #[test]
    fn test_manager_add_deployment() {
        let mut manager = MultiCloudManager::new();

        let deployment =
            MultiCloudDeployment::new("app", DeploymentTarget::MultiCloud, CloudProvider::AWS);
        let id = manager.add_deployment(deployment);

        assert_eq!(manager.deployment_count(), 1);
        assert!(manager.get_deployment(&id).is_some());
    }

    #[test]
    fn test_manager_regions_by_provider() {
        let mut manager = MultiCloudManager::new();

        manager.add_region(CloudRegion::new("r1", CloudProvider::AWS, "US", "e1"));
        manager.add_region(CloudRegion::new("r2", CloudProvider::Azure, "EU", "e2"));
        manager.add_region(CloudRegion::new("r3", CloudProvider::AWS, "Asia", "e3"));

        let aws_regions = manager.regions_by_provider(&CloudProvider::AWS);
        assert_eq!(aws_regions.len(), 2);
    }

    #[test]
    fn test_manager_enabled_regions() {
        let mut manager = MultiCloudManager::new();

        let region1 = CloudRegion::new("r1", CloudProvider::AWS, "US", "e1");
        let mut region2 = CloudRegion::new("r2", CloudProvider::Azure, "EU", "e2");
        region2.disable();

        manager.add_region(region1);
        manager.add_region(region2);

        let enabled = manager.enabled_regions();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_manager_deployments_by_provider() {
        let mut manager = MultiCloudManager::new();

        manager.add_deployment(MultiCloudDeployment::new(
            "d1",
            DeploymentTarget::SingleCloud,
            CloudProvider::AWS,
        ));
        manager.add_deployment(MultiCloudDeployment::new(
            "d2",
            DeploymentTarget::MultiCloud,
            CloudProvider::Azure,
        ));
        manager.add_deployment(MultiCloudDeployment::new(
            "d3",
            DeploymentTarget::HybridCloud,
            CloudProvider::AWS,
        ));

        let aws_deployments = manager.deployments_by_provider(&CloudProvider::AWS);
        assert_eq!(aws_deployments.len(), 2);
    }

    #[test]
    fn test_manager_active_deployments() {
        let mut manager = MultiCloudManager::new();

        let mut deployment1 =
            MultiCloudDeployment::new("d1", DeploymentTarget::SingleCloud, CloudProvider::AWS);
        deployment1.set_status(DeploymentStatus::Active);

        let deployment2 =
            MultiCloudDeployment::new("d2", DeploymentTarget::MultiCloud, CloudProvider::Azure);

        manager.add_deployment(deployment1);
        manager.add_deployment(deployment2);

        let active = manager.active_deployments();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_multi_provider_deployments() {
        let mut manager = MultiCloudManager::new();

        let mut deployment1 =
            MultiCloudDeployment::new("d1", DeploymentTarget::MultiCloud, CloudProvider::AWS);
        deployment1.add_provider(CloudProvider::Azure);

        let deployment2 =
            MultiCloudDeployment::new("d2", DeploymentTarget::SingleCloud, CloudProvider::GCP);

        manager.add_deployment(deployment1);
        manager.add_deployment(deployment2);

        let multi = manager.multi_provider_deployments();
        assert_eq!(multi.len(), 1);
    }

    #[test]
    fn test_manager_deployments_with_failover() {
        let mut manager = MultiCloudManager::new();

        let deployment1 =
            MultiCloudDeployment::new("d1", DeploymentTarget::MultiCloud, CloudProvider::AWS)
                .enable_failover();
        let deployment2 =
            MultiCloudDeployment::new("d2", DeploymentTarget::SingleCloud, CloudProvider::Azure);

        manager.add_deployment(deployment1);
        manager.add_deployment(deployment2);

        let with_failover = manager.deployments_with_failover();
        assert_eq!(with_failover.len(), 1);
    }

    #[test]
    fn test_cloud_provider_equality() {
        assert_eq!(CloudProvider::AWS, CloudProvider::AWS);
        assert_ne!(CloudProvider::AWS, CloudProvider::Azure);
    }

    #[test]
    fn test_deployment_target_equality() {
        assert_eq!(DeploymentTarget::MultiCloud, DeploymentTarget::MultiCloud);
        assert_ne!(DeploymentTarget::MultiCloud, DeploymentTarget::SingleCloud);
    }

    #[test]
    fn test_deployment_status_equality() {
        assert_eq!(DeploymentStatus::Active, DeploymentStatus::Active);
        assert_ne!(DeploymentStatus::Active, DeploymentStatus::Failed);
    }
}
