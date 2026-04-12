// Multi-Cluster Management - Manage VMs across multiple Kubernetes clusters

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiClusterManager {
    pub clusters: Vec<ClusterInfo>,
    pub aggregated_metrics: AggregatedMetrics,
    pub config: MultiClusterConfig,
    pub last_sync: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub name: String,
    pub context: String,
    pub environment: ClusterEnvironment,
    pub region: String,
    pub health: ClusterHealth,
    pub vm_count: usize,
    pub node_count: usize,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub last_synced: DateTime<Utc>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClusterEnvironment {
    Production, Staging, Development, Testing, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClusterHealth { Healthy, Degraded, Unhealthy, Unknown }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AggregatedMetrics {
    pub total_clusters: usize,
    pub total_vms: usize,
    pub total_nodes: usize,
    pub by_environment: HashMap<String, EnvironmentMetrics>,
    pub by_region: HashMap<String, RegionMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentMetrics {
    pub cluster_count: usize,
    pub vm_count: usize,
    pub healthy_clusters: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionMetrics {
    pub cluster_count: usize,
    pub vm_count: usize,
    pub avg_cpu_usage: f64,
    pub avg_memory_usage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiClusterConfig {
    pub auto_discover: bool,
    pub sync_interval_secs: u64,
    pub primary_cluster: Option<String>,
    pub excluded_contexts: Vec<String>,
}

impl Default for MultiClusterConfig {
    fn default() -> Self {
        Self { auto_discover: true, sync_interval_secs: 300, primary_cluster: None, excluded_contexts: Vec::new() }
    }
}

impl MultiClusterManager {
    pub fn new() -> Self {
        Self { clusters: Vec::new(), aggregated_metrics: AggregatedMetrics::default(), config: MultiClusterConfig::default(), last_sync: None }
    }

    pub fn add_cluster(&mut self, cluster: ClusterInfo) {
        self.clusters.push(cluster);
        self.update_aggregated_metrics();
    }

    pub fn remove_cluster(&mut self, name: &str) {
        self.clusters.retain(|c| c.name != name);
        self.update_aggregated_metrics();
    }

    pub fn get_cluster(&self, name: &str) -> Option<&ClusterInfo> {
        self.clusters.iter().find(|c| c.name == name)
    }

    pub fn healthy_clusters(&self) -> Vec<&ClusterInfo> {
        self.clusters.iter().filter(|c| c.health == ClusterHealth::Healthy).collect()
    }

    pub fn unhealthy_clusters(&self) -> Vec<&ClusterInfo> {
        self.clusters.iter().filter(|c| c.health != ClusterHealth::Healthy).collect()
    }

    pub fn clusters_by_env(&self, env: &ClusterEnvironment) -> Vec<&ClusterInfo> {
        self.clusters.iter().filter(|c| c.environment == *env).collect()
    }

    fn update_aggregated_metrics(&mut self) {
        self.aggregated_metrics.total_clusters = self.clusters.len();
        self.aggregated_metrics.total_vms = self.clusters.iter().map(|c| c.vm_count).sum();
        self.aggregated_metrics.total_nodes = self.clusters.iter().map(|c| c.node_count).sum();

        self.aggregated_metrics.by_environment.clear();
        self.aggregated_metrics.by_region.clear();

        for cluster in &self.clusters {
            let env_key = format!("{:?}", cluster.environment);
            let env = self.aggregated_metrics.by_environment.entry(env_key).or_insert(EnvironmentMetrics { cluster_count: 0, vm_count: 0, healthy_clusters: 0 });
            env.cluster_count += 1;
            env.vm_count += cluster.vm_count;
            if cluster.health == ClusterHealth::Healthy { env.healthy_clusters += 1; }

            let region = self.aggregated_metrics.by_region.entry(cluster.region.clone()).or_insert(RegionMetrics { cluster_count: 0, vm_count: 0, avg_cpu_usage: 0.0, avg_memory_usage: 0.0 });
            region.cluster_count += 1;
            region.vm_count += cluster.vm_count;
        }
    }
}

impl Default for MultiClusterManager {
    fn default() -> Self { Self::new() }
}
