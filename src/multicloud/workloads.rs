use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::CloudProvider;

/// Workload type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkloadType {
    VM,
    Container,
    Serverless,
    Database,
    Storage,
    Network,
}

/// Workload placement strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlacementStrategy {
    CostOptimized,
    PerformanceOptimized,
    LatencyOptimized,
    ComplianceRequired,
    DataResidency,
    HighAvailability,
}

/// Cloud workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudWorkload {
    pub id: String,
    pub name: String,
    pub workload_type: WorkloadType,
    pub provider: CloudProvider,
    pub region: String,
    pub placement_strategy: PlacementStrategy,
    pub resource_requirements: ResourceRequirements,
    pub status: WorkloadStatus,
    pub cost_per_hour: f64,
    pub tags: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub network_bandwidth_mbps: Option<u32>,
    pub gpu_count: Option<u32>,
}

/// Workload status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkloadStatus {
    Pending,
    Provisioning,
    Running,
    Paused,
    Migrating,
    Failed,
    Terminated,
}

impl CloudWorkload {
    pub fn new(
        name: impl Into<String>,
        workload_type: WorkloadType,
        provider: CloudProvider,
        region: impl Into<String>,
        placement_strategy: PlacementStrategy,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "wl-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            workload_type,
            provider,
            region: region.into(),
            placement_strategy,
            resource_requirements: ResourceRequirements {
                cpu_cores: 1,
                memory_gb: 2,
                storage_gb: 20,
                network_bandwidth_mbps: None,
                gpu_count: None,
            },
            status: WorkloadStatus::Pending,
            cost_per_hour: 0.0,
            tags: HashMap::new(),
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    pub fn with_resources(mut self, cpu: u32, memory_gb: u32, storage_gb: u32) -> Self {
        self.resource_requirements.cpu_cores = cpu;
        self.resource_requirements.memory_gb = memory_gb;
        self.resource_requirements.storage_gb = storage_gb;
        self
    }

    pub fn with_network_bandwidth(mut self, bandwidth_mbps: u32) -> Self {
        self.resource_requirements.network_bandwidth_mbps = Some(bandwidth_mbps);
        self
    }

    pub fn with_gpu(mut self, gpu_count: u32) -> Self {
        self.resource_requirements.gpu_count = Some(gpu_count);
        self
    }

    pub fn with_cost(mut self, cost_per_hour: f64) -> Self {
        self.cost_per_hour = cost_per_hour;
        self
    }

    pub fn add_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }

    pub fn set_status(&mut self, status: WorkloadStatus) {
        self.status = status;
        self.last_updated = Utc::now();
    }

    pub fn is_running(&self) -> bool {
        self.status == WorkloadStatus::Running
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            WorkloadStatus::Running | WorkloadStatus::Paused | WorkloadStatus::Migrating
        )
    }

    pub fn estimated_monthly_cost(&self) -> f64 {
        self.cost_per_hour * 24.0 * 30.0
    }
}

/// Workload distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadDistribution {
    pub id: String,
    pub name: String,
    pub workload_ids: Vec<String>,
    pub distribution_policy: DistributionPolicy,
    pub target_providers: Vec<CloudProvider>,
    pub replication_factor: u32,
    pub auto_scale: bool,
    pub min_instances: u32,
    pub max_instances: u32,
    pub created_at: DateTime<Utc>,
}

/// Distribution policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributionPolicy {
    RoundRobin,
    WeightedDistribution,
    GeographicDistribution,
    CostOptimized,
    PerformanceBased,
}

impl WorkloadDistribution {
    pub fn new(name: impl Into<String>, distribution_policy: DistributionPolicy) -> Self {
        let name_str = name.into();
        let id = format!(
            "dist-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            workload_ids: Vec::new(),
            distribution_policy,
            target_providers: Vec::new(),
            replication_factor: 1,
            auto_scale: false,
            min_instances: 1,
            max_instances: 10,
            created_at: Utc::now(),
        }
    }

    pub fn add_workload(&mut self, workload_id: impl Into<String>) {
        self.workload_ids.push(workload_id.into());
    }

    pub fn add_provider(&mut self, provider: CloudProvider) {
        if !self.target_providers.contains(&provider) {
            self.target_providers.push(provider);
        }
    }

    pub fn enable_auto_scale(mut self, min: u32, max: u32) -> Self {
        self.auto_scale = true;
        self.min_instances = min;
        self.max_instances = max;
        self
    }

    pub fn with_replication(mut self, factor: u32) -> Self {
        self.replication_factor = factor;
        self
    }

    pub fn workload_count(&self) -> usize {
        self.workload_ids.len()
    }

    pub fn provider_count(&self) -> usize {
        self.target_providers.len()
    }
}

/// Workload manager
pub struct WorkloadManager {
    workloads: HashMap<String, CloudWorkload>,
    distributions: HashMap<String, WorkloadDistribution>,
}

impl WorkloadManager {
    pub fn new() -> Self {
        Self {
            workloads: HashMap::new(),
            distributions: HashMap::new(),
        }
    }

    pub fn add_workload(&mut self, workload: CloudWorkload) -> String {
        let id = workload.id.clone();
        self.workloads.insert(id.clone(), workload);
        id
    }

    pub fn get_workload(&self, id: &str) -> Option<&CloudWorkload> {
        self.workloads.get(id)
    }

    pub fn get_workload_mut(&mut self, id: &str) -> Option<&mut CloudWorkload> {
        self.workloads.get_mut(id)
    }

    pub fn workload_count(&self) -> usize {
        self.workloads.len()
    }

    pub fn add_distribution(&mut self, distribution: WorkloadDistribution) -> String {
        let id = distribution.id.clone();
        self.distributions.insert(id.clone(), distribution);
        id
    }

    pub fn get_distribution(&self, id: &str) -> Option<&WorkloadDistribution> {
        self.distributions.get(id)
    }

    pub fn get_distribution_mut(&mut self, id: &str) -> Option<&mut WorkloadDistribution> {
        self.distributions.get_mut(id)
    }

    pub fn distribution_count(&self) -> usize {
        self.distributions.len()
    }

    pub fn workloads_by_provider(&self, provider: &CloudProvider) -> Vec<&CloudWorkload> {
        self.workloads
            .values()
            .filter(|w| &w.provider == provider)
            .collect()
    }

    pub fn workloads_by_type(&self, workload_type: &WorkloadType) -> Vec<&CloudWorkload> {
        self.workloads
            .values()
            .filter(|w| &w.workload_type == workload_type)
            .collect()
    }

    pub fn running_workloads(&self) -> Vec<&CloudWorkload> {
        self.workloads.values().filter(|w| w.is_running()).collect()
    }

    pub fn active_workloads(&self) -> Vec<&CloudWorkload> {
        self.workloads.values().filter(|w| w.is_active()).collect()
    }

    pub fn workloads_by_strategy(&self, strategy: &PlacementStrategy) -> Vec<&CloudWorkload> {
        self.workloads
            .values()
            .filter(|w| &w.placement_strategy == strategy)
            .collect()
    }

    pub fn total_monthly_cost(&self) -> f64 {
        self.active_workloads()
            .iter()
            .map(|w| w.estimated_monthly_cost())
            .sum()
    }

    pub fn distributions_with_auto_scale(&self) -> Vec<&WorkloadDistribution> {
        self.distributions
            .values()
            .filter(|d| d.auto_scale)
            .collect()
    }
}

impl Default for WorkloadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_workload() {
        let workload = CloudWorkload::new(
            "web-app",
            WorkloadType::Container,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        );

        assert_eq!(workload.name, "web-app");
        assert_eq!(workload.workload_type, WorkloadType::Container);
        assert_eq!(workload.provider, CloudProvider::AWS);
        assert_eq!(workload.region, "us-east-1");
        assert_eq!(workload.status, WorkloadStatus::Pending);
    }

    #[test]
    fn test_workload_with_resources() {
        let workload = CloudWorkload::new(
            "app",
            WorkloadType::VM,
            CloudProvider::Azure,
            "eastus",
            PlacementStrategy::PerformanceOptimized,
        )
        .with_resources(8, 16, 100);

        assert_eq!(workload.resource_requirements.cpu_cores, 8);
        assert_eq!(workload.resource_requirements.memory_gb, 16);
        assert_eq!(workload.resource_requirements.storage_gb, 100);
    }

    #[test]
    fn test_workload_with_network_bandwidth() {
        let workload = CloudWorkload::new(
            "app",
            WorkloadType::VM,
            CloudProvider::GCP,
            "us-central1",
            PlacementStrategy::LatencyOptimized,
        )
        .with_network_bandwidth(1000);

        assert_eq!(
            workload.resource_requirements.network_bandwidth_mbps,
            Some(1000)
        );
    }

    #[test]
    fn test_workload_with_gpu() {
        let workload = CloudWorkload::new(
            "ml-app",
            WorkloadType::Container,
            CloudProvider::AWS,
            "us-west-2",
            PlacementStrategy::PerformanceOptimized,
        )
        .with_gpu(4);

        assert_eq!(workload.resource_requirements.gpu_count, Some(4));
    }

    #[test]
    fn test_workload_with_cost() {
        let workload = CloudWorkload::new(
            "app",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        )
        .with_cost(0.5);

        assert_eq!(workload.cost_per_hour, 0.5);
    }

    #[test]
    fn test_workload_add_tag() {
        let mut workload = CloudWorkload::new(
            "app",
            WorkloadType::Container,
            CloudProvider::Azure,
            "eastus",
            PlacementStrategy::CostOptimized,
        );

        workload.add_tag("environment", "production");
        workload.add_tag("team", "backend");

        assert_eq!(workload.tags.len(), 2);
    }

    #[test]
    fn test_workload_set_status() {
        let mut workload = CloudWorkload::new(
            "app",
            WorkloadType::VM,
            CloudProvider::GCP,
            "us-central1",
            PlacementStrategy::PerformanceOptimized,
        );

        workload.set_status(WorkloadStatus::Running);
        assert_eq!(workload.status, WorkloadStatus::Running);
    }

    #[test]
    fn test_workload_is_running() {
        let mut workload = CloudWorkload::new(
            "app",
            WorkloadType::Container,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        );

        assert!(!workload.is_running());

        workload.set_status(WorkloadStatus::Running);
        assert!(workload.is_running());
    }

    #[test]
    fn test_workload_is_active() {
        let mut workload = CloudWorkload::new(
            "app",
            WorkloadType::VM,
            CloudProvider::Azure,
            "eastus",
            PlacementStrategy::PerformanceOptimized,
        );

        workload.set_status(WorkloadStatus::Running);
        assert!(workload.is_active());

        workload.set_status(WorkloadStatus::Paused);
        assert!(workload.is_active());

        workload.set_status(WorkloadStatus::Terminated);
        assert!(!workload.is_active());
    }

    #[test]
    fn test_workload_estimated_monthly_cost() {
        let workload = CloudWorkload::new(
            "app",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        )
        .with_cost(1.0);

        let monthly_cost = workload.estimated_monthly_cost();
        assert_eq!(monthly_cost, 720.0); // 1.0 * 24 * 30
    }

    #[test]
    fn test_workload_distribution() {
        let distribution = WorkloadDistribution::new(
            "global-distribution",
            DistributionPolicy::GeographicDistribution,
        );

        assert_eq!(distribution.name, "global-distribution");
        assert_eq!(
            distribution.distribution_policy,
            DistributionPolicy::GeographicDistribution
        );
        assert_eq!(distribution.replication_factor, 1);
        assert!(!distribution.auto_scale);
    }

    #[test]
    fn test_distribution_add_workload() {
        let mut distribution = WorkloadDistribution::new("dist", DistributionPolicy::RoundRobin);

        distribution.add_workload("wl-1");
        distribution.add_workload("wl-2");

        assert_eq!(distribution.workload_count(), 2);
    }

    #[test]
    fn test_distribution_add_provider() {
        let mut distribution =
            WorkloadDistribution::new("dist", DistributionPolicy::WeightedDistribution);

        distribution.add_provider(CloudProvider::AWS);
        distribution.add_provider(CloudProvider::Azure);
        distribution.add_provider(CloudProvider::AWS); // Duplicate

        assert_eq!(distribution.provider_count(), 2);
    }

    #[test]
    fn test_distribution_enable_auto_scale() {
        let distribution = WorkloadDistribution::new("dist", DistributionPolicy::PerformanceBased)
            .enable_auto_scale(2, 10);

        assert!(distribution.auto_scale);
        assert_eq!(distribution.min_instances, 2);
        assert_eq!(distribution.max_instances, 10);
    }

    #[test]
    fn test_distribution_with_replication() {
        let distribution =
            WorkloadDistribution::new("dist", DistributionPolicy::GeographicDistribution)
                .with_replication(3);

        assert_eq!(distribution.replication_factor, 3);
    }

    #[test]
    fn test_workload_manager() {
        let mut manager = WorkloadManager::new();

        let workload = CloudWorkload::new(
            "app",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        );
        let id = manager.add_workload(workload);

        assert_eq!(manager.workload_count(), 1);
        assert!(manager.get_workload(&id).is_some());
    }

    #[test]
    fn test_manager_add_distribution() {
        let mut manager = WorkloadManager::new();

        let distribution = WorkloadDistribution::new("dist", DistributionPolicy::RoundRobin);
        let id = manager.add_distribution(distribution);

        assert_eq!(manager.distribution_count(), 1);
        assert!(manager.get_distribution(&id).is_some());
    }

    #[test]
    fn test_manager_workloads_by_provider() {
        let mut manager = WorkloadManager::new();

        manager.add_workload(CloudWorkload::new(
            "w1",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        ));
        manager.add_workload(CloudWorkload::new(
            "w2",
            WorkloadType::Container,
            CloudProvider::Azure,
            "eastus",
            PlacementStrategy::PerformanceOptimized,
        ));
        manager.add_workload(CloudWorkload::new(
            "w3",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-west-2",
            PlacementStrategy::CostOptimized,
        ));

        let aws_workloads = manager.workloads_by_provider(&CloudProvider::AWS);
        assert_eq!(aws_workloads.len(), 2);
    }

    #[test]
    fn test_manager_workloads_by_type() {
        let mut manager = WorkloadManager::new();

        manager.add_workload(CloudWorkload::new(
            "w1",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        ));
        manager.add_workload(CloudWorkload::new(
            "w2",
            WorkloadType::Container,
            CloudProvider::Azure,
            "eastus",
            PlacementStrategy::PerformanceOptimized,
        ));
        manager.add_workload(CloudWorkload::new(
            "w3",
            WorkloadType::VM,
            CloudProvider::GCP,
            "us-central1",
            PlacementStrategy::CostOptimized,
        ));

        let vms = manager.workloads_by_type(&WorkloadType::VM);
        assert_eq!(vms.len(), 2);
    }

    #[test]
    fn test_manager_running_workloads() {
        let mut manager = WorkloadManager::new();

        let mut workload1 = CloudWorkload::new(
            "w1",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        );
        workload1.set_status(WorkloadStatus::Running);

        let workload2 = CloudWorkload::new(
            "w2",
            WorkloadType::Container,
            CloudProvider::Azure,
            "eastus",
            PlacementStrategy::PerformanceOptimized,
        );

        manager.add_workload(workload1);
        manager.add_workload(workload2);

        let running = manager.running_workloads();
        assert_eq!(running.len(), 1);
    }

    #[test]
    fn test_manager_active_workloads() {
        let mut manager = WorkloadManager::new();

        let mut workload1 = CloudWorkload::new(
            "w1",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        );
        workload1.set_status(WorkloadStatus::Running);

        let mut workload2 = CloudWorkload::new(
            "w2",
            WorkloadType::Container,
            CloudProvider::Azure,
            "eastus",
            PlacementStrategy::PerformanceOptimized,
        );
        workload2.set_status(WorkloadStatus::Paused);

        let mut workload3 = CloudWorkload::new(
            "w3",
            WorkloadType::VM,
            CloudProvider::GCP,
            "us-central1",
            PlacementStrategy::CostOptimized,
        );
        workload3.set_status(WorkloadStatus::Terminated);

        manager.add_workload(workload1);
        manager.add_workload(workload2);
        manager.add_workload(workload3);

        let active = manager.active_workloads();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_manager_workloads_by_strategy() {
        let mut manager = WorkloadManager::new();

        manager.add_workload(CloudWorkload::new(
            "w1",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        ));
        manager.add_workload(CloudWorkload::new(
            "w2",
            WorkloadType::Container,
            CloudProvider::Azure,
            "eastus",
            PlacementStrategy::PerformanceOptimized,
        ));
        manager.add_workload(CloudWorkload::new(
            "w3",
            WorkloadType::VM,
            CloudProvider::GCP,
            "us-central1",
            PlacementStrategy::CostOptimized,
        ));

        let cost_optimized = manager.workloads_by_strategy(&PlacementStrategy::CostOptimized);
        assert_eq!(cost_optimized.len(), 2);
    }

    #[test]
    fn test_manager_total_monthly_cost() {
        let mut manager = WorkloadManager::new();

        let mut workload1 = CloudWorkload::new(
            "w1",
            WorkloadType::VM,
            CloudProvider::AWS,
            "us-east-1",
            PlacementStrategy::CostOptimized,
        )
        .with_cost(1.0);
        workload1.set_status(WorkloadStatus::Running);

        let mut workload2 = CloudWorkload::new(
            "w2",
            WorkloadType::Container,
            CloudProvider::Azure,
            "eastus",
            PlacementStrategy::PerformanceOptimized,
        )
        .with_cost(0.5);
        workload2.set_status(WorkloadStatus::Running);

        manager.add_workload(workload1);
        manager.add_workload(workload2);

        let total_cost = manager.total_monthly_cost();
        assert_eq!(total_cost, 1080.0); // (1.0 + 0.5) * 24 * 30
    }

    #[test]
    fn test_manager_distributions_with_auto_scale() {
        let mut manager = WorkloadManager::new();

        let distribution1 =
            WorkloadDistribution::new("d1", DistributionPolicy::RoundRobin).enable_auto_scale(1, 5);
        let distribution2 = WorkloadDistribution::new("d2", DistributionPolicy::CostOptimized);

        manager.add_distribution(distribution1);
        manager.add_distribution(distribution2);

        let auto_scaled = manager.distributions_with_auto_scale();
        assert_eq!(auto_scaled.len(), 1);
    }

    #[test]
    fn test_workload_type_equality() {
        assert_eq!(WorkloadType::VM, WorkloadType::VM);
        assert_ne!(WorkloadType::VM, WorkloadType::Container);
    }

    #[test]
    fn test_placement_strategy_equality() {
        assert_eq!(
            PlacementStrategy::CostOptimized,
            PlacementStrategy::CostOptimized
        );
        assert_ne!(
            PlacementStrategy::CostOptimized,
            PlacementStrategy::PerformanceOptimized
        );
    }

    #[test]
    fn test_workload_status_equality() {
        assert_eq!(WorkloadStatus::Running, WorkloadStatus::Running);
        assert_ne!(WorkloadStatus::Running, WorkloadStatus::Paused);
    }

    #[test]
    fn test_distribution_policy_equality() {
        assert_eq!(
            DistributionPolicy::RoundRobin,
            DistributionPolicy::RoundRobin
        );
        assert_ne!(
            DistributionPolicy::RoundRobin,
            DistributionPolicy::WeightedDistribution
        );
    }
}
