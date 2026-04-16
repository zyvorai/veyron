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
    Production,
    Staging,
    Development,
    Testing,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClusterHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

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
        Self {
            auto_discover: true,
            sync_interval_secs: 300,
            primary_cluster: None,
            excluded_contexts: Vec::new(),
        }
    }
}

impl MultiClusterManager {
    pub fn new() -> Self {
        Self {
            clusters: Vec::new(),
            aggregated_metrics: AggregatedMetrics::default(),
            config: MultiClusterConfig::default(),
            last_sync: None,
        }
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
        self.clusters
            .iter()
            .filter(|c| c.health == ClusterHealth::Healthy)
            .collect()
    }

    pub fn unhealthy_clusters(&self) -> Vec<&ClusterInfo> {
        self.clusters
            .iter()
            .filter(|c| c.health != ClusterHealth::Healthy)
            .collect()
    }

    pub fn clusters_by_env(&self, env: &ClusterEnvironment) -> Vec<&ClusterInfo> {
        self.clusters
            .iter()
            .filter(|c| c.environment == *env)
            .collect()
    }

    fn update_aggregated_metrics(&mut self) {
        self.aggregated_metrics.total_clusters = self.clusters.len();
        self.aggregated_metrics.total_vms = self.clusters.iter().map(|c| c.vm_count).sum();
        self.aggregated_metrics.total_nodes = self.clusters.iter().map(|c| c.node_count).sum();

        self.aggregated_metrics.by_environment.clear();
        self.aggregated_metrics.by_region.clear();

        for cluster in &self.clusters {
            let env_key = format!("{:?}", cluster.environment);
            let env = self
                .aggregated_metrics
                .by_environment
                .entry(env_key)
                .or_insert(EnvironmentMetrics {
                    cluster_count: 0,
                    vm_count: 0,
                    healthy_clusters: 0,
                });
            env.cluster_count += 1;
            env.vm_count += cluster.vm_count;
            if cluster.health == ClusterHealth::Healthy {
                env.healthy_clusters += 1;
            }

            let region = self
                .aggregated_metrics
                .by_region
                .entry(cluster.region.clone())
                .or_insert(RegionMetrics {
                    cluster_count: 0,
                    vm_count: 0,
                    avg_cpu_usage: 0.0,
                    avg_memory_usage: 0.0,
                });
            region.cluster_count += 1;
            region.vm_count += cluster.vm_count;
        }
    }

    /// Infer cluster environment from context name using word-boundary matching
    /// to avoid false positives (e.g. "reproduce" matching "prod").
    fn infer_environment(name: &str) -> ClusterEnvironment {
        let lower = name.to_lowercase();
        let words: Vec<&str> = lower.split(|c: char| c == '-' || c == '_' || c == '.').collect();

        if words.iter().any(|w| *w == "prod" || *w == "production") {
            ClusterEnvironment::Production
        } else if words.iter().any(|w| *w == "stag" || *w == "staging") {
            ClusterEnvironment::Staging
        } else if words.iter().any(|w| *w == "dev" || *w == "development") {
            ClusterEnvironment::Development
        } else if words.iter().any(|w| *w == "test" || *w == "testing") {
            ClusterEnvironment::Testing
        } else {
            ClusterEnvironment::Custom(name.to_string())
        }
    }

    /// Resolve the kubeconfig file path from `KUBECONFIG` env or default location.
    fn kubeconfig_path() -> String {
        std::env::var("KUBECONFIG").unwrap_or_else(|_| {
            dirs::home_dir()
                .map(|h| h.join(".kube/config").to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    }

    /// Discover clusters from the current kubeconfig file.
    ///
    /// Reads all contexts from `~/.kube/config` (or KUBECONFIG) and creates
    /// a `ClusterInfo` entry for each. No K8s API calls are made during
    /// discovery — call `sync_cluster` to populate live metrics.
    pub async fn discover_from_kubeconfig(&mut self) -> anyhow::Result<usize> {
        let kubeconfig_path = Self::kubeconfig_path();

        if kubeconfig_path.is_empty() || !std::path::Path::new(&kubeconfig_path).exists() {
            return Ok(0);
        }

        let kubeconfig = kube::config::Kubeconfig::read_from(&kubeconfig_path)?;
        let mut added = 0;

        for ctx in &kubeconfig.contexts {
            let ctx_name = &ctx.name;

            // Skip excluded contexts
            if self.config.excluded_contexts.contains(ctx_name) {
                continue;
            }

            // Skip if already known
            if self.clusters.iter().any(|c| c.context == *ctx_name) {
                continue;
            }

            let env = Self::infer_environment(ctx_name);

            let is_primary = kubeconfig
                .current_context
                .as_ref()
                .map(|c| c == ctx_name)
                .unwrap_or(false);

            self.add_cluster(ClusterInfo {
                name: ctx_name.clone(),
                context: ctx_name.clone(),
                environment: env,
                region: "unknown".to_string(),
                health: ClusterHealth::Unknown,
                vm_count: 0,
                node_count: 0,
                cpu_usage_percent: 0.0,
                memory_usage_percent: 0.0,
                last_synced: Utc::now(),
                is_primary,
            });
            added += 1;
        }

        Ok(added)
    }

    /// Sync live data for a single cluster by connecting to its K8s API.
    pub async fn sync_cluster(&mut self, cluster_name: &str) -> anyhow::Result<()> {
        let ctx = match self.clusters.iter().find(|c| c.name == cluster_name) {
            Some(c) => c.context.clone(),
            None => anyhow::bail!("Cluster '{}' not found", cluster_name),
        };

        // Build a client for this specific context
        let kubeconfig = kube::config::Kubeconfig::read_from(&Self::kubeconfig_path())?;
        let opts = kube::config::KubeConfigOptions {
            context: Some(ctx),
            ..Default::default()
        };
        let config = kube::Config::from_custom_kubeconfig(kubeconfig, &opts).await?;
        let client = kube::Client::try_from(config)?;

        // Count nodes
        let nodes: kube::Api<k8s_openapi::api::core::v1::Node> = kube::Api::all(client.clone());
        let node_list = nodes.list(&kube::api::ListParams::default()).await?;
        let node_count = node_list.items.len();

        // Count VMs
        let vms: kube::Api<crate::kube::types::VirtualMachine> =
            kube::Api::all(client);
        let vm_count = vms
            .list(&kube::api::ListParams::default())
            .await
            .map(|l| l.items.len())
            .unwrap_or(0);

        // Update cluster info
        if let Some(cluster) = self.clusters.iter_mut().find(|c| c.name == cluster_name) {
            cluster.node_count = node_count;
            cluster.vm_count = vm_count;
            cluster.health = if node_count > 0 {
                ClusterHealth::Healthy
            } else {
                ClusterHealth::Degraded
            };
            cluster.last_synced = Utc::now();
        }

        self.update_aggregated_metrics();
        Ok(())
    }
}

impl Default for MultiClusterManager {
    fn default() -> Self {
        Self::new()
    }
}
