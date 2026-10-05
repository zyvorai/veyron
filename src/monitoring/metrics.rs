// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Metrics Collector - Real-time resource usage tracking

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use kube::api::Api;
use serde::{Deserialize, Serialize};

/// VM resource metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu: CPUMetrics,
    pub memory: MemoryMetrics,
    pub disk: DiskMetrics,
    pub network: NetworkMetrics,
}

impl VMMetrics {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            cpu: CPUMetrics::default(),
            memory: MemoryMetrics::default(),
            disk: DiskMetrics::default(),
            network: NetworkMetrics::default(),
        }
    }
}

impl Default for VMMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// CPU metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CPUMetrics {
    pub usage_percent: f64,
    pub cores_allocated: u32,
    pub cores_used: f64,
    pub system_percent: f64,
    pub user_percent: f64,
    pub idle_percent: f64,
}

impl CPUMetrics {
    pub fn usage_description(&self) -> String {
        if self.usage_percent < 50.0 {
            "Low".to_string()
        } else if self.usage_percent < 70.0 {
            "Moderate".to_string()
        } else if self.usage_percent < 90.0 {
            "High".to_string()
        } else {
            "Critical".to_string()
        }
    }
}

/// Memory metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryMetrics {
    pub usage_percent: f64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub total_bytes: u64,
    pub cache_bytes: u64,
    pub swap_used_bytes: u64,
}

impl MemoryMetrics {
    pub fn used_gb(&self) -> f64 {
        self.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn total_gb(&self) -> f64 {
        self.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn available_gb(&self) -> f64 {
        self.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

/// Disk I/O metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiskMetrics {
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub read_ops_per_sec: u64,
    pub write_ops_per_sec: u64,
    pub usage_percent: f64,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

impl DiskMetrics {
    pub fn read_mb_per_sec(&self) -> f64 {
        self.read_bytes_per_sec as f64 / (1024.0 * 1024.0)
    }

    pub fn write_mb_per_sec(&self) -> f64 {
        self.write_bytes_per_sec as f64 / (1024.0 * 1024.0)
    }

    pub fn total_iops(&self) -> u64 {
        self.read_ops_per_sec + self.write_ops_per_sec
    }
}

/// Network I/O metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkMetrics {
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub rx_packets_per_sec: u64,
    pub tx_packets_per_sec: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}

impl NetworkMetrics {
    pub fn rx_mb_per_sec(&self) -> f64 {
        self.rx_bytes_per_sec as f64 / (1024.0 * 1024.0)
    }

    pub fn tx_mb_per_sec(&self) -> f64 {
        self.tx_bytes_per_sec as f64 / (1024.0 * 1024.0)
    }

    pub fn total_bandwidth_mb_per_sec(&self) -> f64 {
        self.rx_mb_per_sec() + self.tx_mb_per_sec()
    }
}

/// Resource usage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub disk_percent: f64,
    pub network_mb_per_sec: f64,
}

impl ResourceUsage {
    pub fn from_metrics(metrics: &VMMetrics) -> Self {
        Self {
            cpu_percent: metrics.cpu.usage_percent,
            memory_percent: metrics.memory.usage_percent,
            disk_percent: metrics.disk.usage_percent,
            network_mb_per_sec: metrics.network.total_bandwidth_mb_per_sec(),
        }
    }

    pub fn overall_usage(&self) -> f64 {
        (self.cpu_percent + self.memory_percent + self.disk_percent) / 3.0
    }
}

/// Metrics collector for VMs
pub struct MetricsCollector {
    namespace: String,
}

impl MetricsCollector {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
        }
    }

    /// Collect current metrics for a VM using real K8s VM spec data
    pub async fn collect(&self, vm_name: &str) -> Result<VMMetrics> {
        self.create_metrics_from_vm(vm_name)
            .await
            .with_context(|| format!("Failed to collect metrics for VM '{}'", vm_name))
    }

    /// Collect metrics for multiple VMs
    pub async fn collect_multiple(&self, vm_names: &[String]) -> Result<Vec<(String, VMMetrics)>> {
        let mut results = Vec::new();
        for vm_name in vm_names {
            let metrics = self.collect(vm_name).await?;
            results.push((vm_name.clone(), metrics));
        }
        Ok(results)
    }

    /// Collect historical metrics for a VM.
    ///
    /// Returns only the current data point since real time-series storage is
    /// not yet implemented. Callers should not expect a full history — the
    /// single-element vec distinguishes "VM exists but no history" from an
    /// error.
    pub async fn collect_historical(
        &self,
        vm_name: &str,
        _duration_seconds: u64,
    ) -> Result<Vec<VMMetrics>> {
        // Return the current snapshot as a single data point.
        // Real historical storage (Prometheus integration) is a future task.
        let current = self.collect(vm_name).await?;
        Ok(vec![current])
    }

    /// Create metrics from real K8s VM spec
    async fn create_metrics_from_vm(&self, vm_name: &str) -> Result<VMMetrics> {
        use crate::kube::types::VirtualMachine;

        let client = crate::kube::get_client().await?;

        let vms: Api<VirtualMachine> = Api::namespaced(client, &self.namespace);
        let vm = vms
            .get(vm_name)
            .await
            .with_context(|| format!("Failed to get VM '{}' for metrics", vm_name))?;

        // Extract CPU cores from VM spec
        let cores_allocated = vm
            .spec
            .template
            .spec
            .domain
            .cpu
            .as_ref()
            .and_then(|c| c.cores)
            .unwrap_or(1);

        // Extract memory from VM spec
        let memory_str = vm
            .spec
            .template
            .spec
            .domain
            .resources
            .requests
            .as_ref()
            .and_then(|req| req.get("memory"))
            .map(|s| s.as_str())
            .unwrap_or("1Gi");

        let total_memory_bytes = crate::disk::DiskInfo::parse_size(memory_str);

        // Extract disk size from volumes
        let total_disk_bytes = vm
            .spec
            .template
            .spec
            .volumes
            .as_ref()
            .map(|volumes| {
                volumes
                    .iter()
                    .map(|v| {
                        if let Some(ref empty) = v.empty_disk {
                            crate::disk::DiskInfo::parse_size(&empty.capacity)
                        } else {
                            // PVCs/DataVolumes: estimate 20Gi default
                            if v.persistent_volume_claim.is_some() || v.data_volume.is_some() {
                                20 * 1024 * 1024 * 1024
                            } else {
                                0
                            }
                        }
                    })
                    .sum::<u64>()
            })
            .unwrap_or(20 * 1024 * 1024 * 1024);

        Ok(self
            .collect_real_pod_metrics(
                vm_name,
                cores_allocated,
                total_memory_bytes,
                total_disk_bytes,
            )
            .await)
    }

    /// Collect real metrics from Kubernetes Metrics Server for the virt-launcher pod.
    ///
    /// Falls back to allocation-only metrics (usage = 0) if the Metrics Server
    /// is unavailable, rather than generating random data.
    async fn collect_real_pod_metrics(
        &self,
        vm_name: &str,
        cores_allocated: u32,
        total_memory_bytes: u64,
        total_disk_bytes: u64,
    ) -> VMMetrics {
        use k8s_openapi::api::core::v1::Pod;

        // Try to find the virt-launcher pod for this VM
        let client = match crate::kube::get_client().await {
            Ok(c) => c,
            Err(_) => {
                return self.create_allocation_only_metrics(
                    cores_allocated,
                    total_memory_bytes,
                    total_disk_bytes,
                );
            }
        };

        let pods: Api<Pod> = Api::namespaced(client.clone(), &self.namespace);
        let label = format!("{}={}", crate::kube::VM_NAME_LABEL, vm_name);
        let lp = kube::api::ListParams::default().labels(&label);

        let pod_name = match pods.list(&lp).await {
            Ok(list) => list.items.first().and_then(|p| p.metadata.name.clone()),
            Err(_) => None,
        };

        // Try querying the Metrics Server API (metrics.k8s.io/v1beta1)
        if let Some(ref pod) = pod_name {
            if let Ok(metrics) = self.fetch_pod_metrics(&client, pod).await {
                let (cpu_nano, mem_bytes) = metrics;
                let cpu_usage_cores = cpu_nano as f64 / 1_000_000_000.0;
                let cpu_percent = if cores_allocated > 0 {
                    (cpu_usage_cores / cores_allocated as f64 * 100.0).min(100.0)
                } else {
                    0.0
                };
                let mem_percent = if total_memory_bytes > 0 {
                    (mem_bytes as f64 / total_memory_bytes as f64 * 100.0).min(100.0)
                } else {
                    0.0
                };

                return VMMetrics {
                    timestamp: Utc::now(),
                    cpu: CPUMetrics {
                        usage_percent: cpu_percent,
                        cores_allocated,
                        cores_used: cpu_usage_cores,
                        system_percent: 0.0,
                        user_percent: cpu_percent,
                        idle_percent: 100.0 - cpu_percent,
                    },
                    memory: MemoryMetrics {
                        usage_percent: mem_percent,
                        used_bytes: mem_bytes,
                        available_bytes: total_memory_bytes.saturating_sub(mem_bytes),
                        total_bytes: total_memory_bytes,
                        cache_bytes: 0,
                        swap_used_bytes: 0,
                    },
                    disk: DiskMetrics {
                        usage_percent: 0.0,
                        used_bytes: 0,
                        total_bytes: total_disk_bytes,
                        ..Default::default()
                    },
                    network: NetworkMetrics::default(),
                };
            }
        }

        // Metrics Server unavailable: return allocation-only data (no random numbers)
        self.create_allocation_only_metrics(cores_allocated, total_memory_bytes, total_disk_bytes)
    }

    /// Fetch CPU (nanocores) and memory (bytes) from the Kubernetes Metrics Server
    /// using a dynamic API request.
    async fn fetch_pod_metrics(&self, client: &kube::Client, pod_name: &str) -> Result<(u64, u64)> {
        // Use kube's dynamic API to fetch PodMetrics
        let gvk = kube::api::GroupVersionKind::gvk("metrics.k8s.io", "v1beta1", "PodMetrics");
        let ar = kube::api::ApiResource::from_gvk(&gvk);
        let api: kube::Api<kube::api::DynamicObject> =
            kube::Api::namespaced_with(client.clone(), &self.namespace, &ar);

        let pod_metrics = api
            .get(pod_name)
            .await
            .context("Metrics Server unavailable or pod not found")?;

        // Parse from dynamic object data
        let containers = pod_metrics
            .data
            .get("containers")
            .and_then(|c| c.as_array())
            .context("No containers in metrics response")?;

        let mut total_cpu: u64 = 0;
        let mut total_mem: u64 = 0;

        for container in containers {
            if let Some(usage) = container.get("usage") {
                if let Some(cpu_str) = usage.get("cpu").and_then(|v| v.as_str()) {
                    total_cpu += parse_k8s_cpu_nanocores(cpu_str);
                }
                if let Some(mem_str) = usage.get("memory").and_then(|v| v.as_str()) {
                    total_mem += parse_k8s_memory_bytes(mem_str);
                }
            }
        }

        Ok((total_cpu, total_mem))
    }

    /// Create metrics showing only allocations with zero usage.
    /// Used when the Metrics Server is unavailable.
    fn create_allocation_only_metrics(
        &self,
        cores_allocated: u32,
        total_memory_bytes: u64,
        total_disk_bytes: u64,
    ) -> VMMetrics {
        VMMetrics {
            timestamp: Utc::now(),
            cpu: CPUMetrics {
                usage_percent: 0.0,
                cores_allocated,
                cores_used: 0.0,
                system_percent: 0.0,
                user_percent: 0.0,
                idle_percent: 100.0,
            },
            memory: MemoryMetrics {
                usage_percent: 0.0,
                used_bytes: 0,
                available_bytes: total_memory_bytes,
                total_bytes: total_memory_bytes,
                cache_bytes: 0,
                swap_used_bytes: 0,
            },
            disk: DiskMetrics {
                usage_percent: 0.0,
                used_bytes: 0,
                total_bytes: total_disk_bytes,
                ..Default::default()
            },
            network: NetworkMetrics::default(),
        }
    }
}

// Re-use consolidated K8s quantity parsers from crate::utils
use crate::utils::parse_cpu_nanocores as parse_k8s_cpu_nanocores;
use crate::utils::parse_memory_bytes as parse_k8s_memory_bytes;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_metrics_creation() {
        let metrics = VMMetrics::new();
        assert!(metrics.cpu.usage_percent >= 0.0);
    }

    #[test]
    fn test_cpu_usage_description() {
        let mut cpu = CPUMetrics {
            usage_percent: 40.0,
            ..Default::default()
        };
        assert_eq!(cpu.usage_description(), "Low");

        cpu.usage_percent = 60.0;
        assert_eq!(cpu.usage_description(), "Moderate");

        cpu.usage_percent = 80.0;
        assert_eq!(cpu.usage_description(), "High");

        cpu.usage_percent = 95.0;
        assert_eq!(cpu.usage_description(), "Critical");
    }

    #[test]
    fn test_memory_metrics_gb_conversion() {
        let memory = MemoryMetrics {
            used_bytes: 4_294_967_296,       // 4 GiB
            total_bytes: 17_179_869_184,     // 16 GiB
            available_bytes: 12_884_901_888, // 12 GiB
            ..Default::default()
        };

        assert!((memory.used_gb() - 4.0).abs() < 0.1);
        assert!((memory.total_gb() - 16.0).abs() < 0.1);
        assert!((memory.available_gb() - 12.0).abs() < 0.1);
    }

    #[test]
    fn test_disk_metrics_conversions() {
        let disk = DiskMetrics {
            read_bytes_per_sec: 10_485_760, // 10 MiB/s
            write_bytes_per_sec: 5_242_880, // 5 MiB/s
            read_ops_per_sec: 100,
            write_ops_per_sec: 50,
            ..Default::default()
        };

        assert!((disk.read_mb_per_sec() - 10.0).abs() < 0.1);
        assert!((disk.write_mb_per_sec() - 5.0).abs() < 0.1);
        assert_eq!(disk.total_iops(), 150);
    }

    #[test]
    fn test_network_metrics_conversions() {
        let network = NetworkMetrics {
            rx_bytes_per_sec: 2_097_152, // 2 MiB/s
            tx_bytes_per_sec: 4_194_304, // 4 MiB/s
            ..Default::default()
        };

        assert!((network.rx_mb_per_sec() - 2.0).abs() < 0.1);
        assert!((network.tx_mb_per_sec() - 4.0).abs() < 0.1);
        assert!((network.total_bandwidth_mb_per_sec() - 6.0).abs() < 0.1);
    }

    #[test]
    fn test_resource_usage_from_metrics() {
        let metrics = VMMetrics {
            timestamp: Utc::now(),
            cpu: CPUMetrics {
                usage_percent: 75.0,
                ..Default::default()
            },
            memory: MemoryMetrics {
                usage_percent: 65.0,
                ..Default::default()
            },
            disk: DiskMetrics {
                usage_percent: 55.0,
                ..Default::default()
            },
            network: NetworkMetrics::default(),
        };

        let usage = ResourceUsage::from_metrics(&metrics);
        assert_eq!(usage.cpu_percent, 75.0);
        assert_eq!(usage.memory_percent, 65.0);
        assert_eq!(usage.disk_percent, 55.0);
        assert!((usage.overall_usage() - 65.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new("default");
        // Without a K8s cluster, collect should return an error
        let result = collector.collect("test-vm").await;
        assert!(result.is_err());
    }
}
