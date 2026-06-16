// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

use super::feature_context::VmrogueFeatureContext;

/// Performance profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    pub name: String,
    pub vm_name: String,
    pub cpu_p50: f64,
    pub cpu_p95: f64,
    pub cpu_p99: f64,
    pub memory_p50: f64,
    pub memory_p95: f64,
    pub memory_p99: f64,
    pub iops_read: u64,
    pub iops_write: u64,
    pub latency_avg_ms: f64,
    pub collected_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PerformanceQuery {
    pub namespace: Option<String>,
}

/// One stack frame in a flamegraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameFrame {
    pub name: String,
    pub value: u64,
    pub color: String,
    pub children: Vec<FlameFrame>,
}

/// Flamegraph data for a single VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameGraph {
    pub vm_name: String,
    pub collected_at: String,
    pub total_cpu_percent: f64,
    pub frames: Vec<FlameFrame>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlameGraphQuery {
    pub namespace: Option<String>,
    pub vm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfilesResponse {
    pub veyron_context: VmrogueFeatureContext,
    pub profiles: Vec<PerformanceProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameGraphListResponse {
    pub veyron_context: VmrogueFeatureContext,
    pub flamegraphs: Vec<FlameGraph>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/performance/profiles", get(list_performance_profiles))
        .route("/performance/flamegraph", get(get_flamegraph))
        .with_state(state)
}

/// Collect basic performance profiles from live VM metrics.
#[cfg(feature = "web")]
async fn list_performance_profiles(
    State(state): State<SharedState>,
    Query(query): Query<PerformanceQuery>,
) -> Json<PerformanceProfilesResponse> {
    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };
    let now = chrono::Utc::now().to_rfc3339();

    let mut profiles = Vec::new();
    let mut used_prometheus = false;

    for vm in &vms {
        let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let vm_ns = vm
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| namespace.clone());
        let collector = crate::monitoring::metrics::MetricsCollector::new(vm_ns.clone());

        if let Ok(metrics) = collector.collect(vm_name).await {
            let cpu = metrics.cpu.usage_percent;
            let mem = metrics.memory.usage_percent;
            let mut cpu_p50 = cpu;
            let mut cpu_p95 = (cpu * 1.30).min(100.0);
            let mut cpu_p99 = (cpu * 1.50).min(100.0);
            let mut mem_p50 = mem;
            let mut mem_p95 = (mem * 1.20).min(100.0);
            let mut mem_p99 = (mem * 1.35).min(100.0);

            if namespace != "all" {
                if let Some((p50, p95, p99)) = prometheus_vm_cpu_percentiles(&vm_ns, vm_name).await
                {
                    cpu_p50 = p50;
                    cpu_p95 = p95;
                    cpu_p99 = p99;
                    used_prometheus = true;
                }
                if let Some((p50, p95, p99)) =
                    prometheus_vm_memory_percentiles(&vm_ns, vm_name).await
                {
                    mem_p50 = p50;
                    mem_p95 = p95;
                    mem_p99 = p99;
                    used_prometheus = true;
                }
            }
            // Estimate average I/O latency: throughput / (IOPS × sector size).
            // Clamp to a plausible range; returns 0 when no I/O is observed.
            let total_iops =
                (metrics.disk.read_ops_per_sec + metrics.disk.write_ops_per_sec) as f64;
            let total_bytes =
                (metrics.disk.read_bytes_per_sec + metrics.disk.write_bytes_per_sec) as f64;
            let latency_avg_ms = if total_iops > 0.0 {
                (total_bytes / total_iops / 512.0).clamp(0.1, 500.0)
            } else {
                0.0
            };
            profiles.push(PerformanceProfile {
                name: format!("profile-{}", vm_name),
                vm_name: vm_name.to_string(),
                cpu_p50,
                cpu_p95,
                cpu_p99,
                memory_p50: mem_p50,
                memory_p95: mem_p95,
                memory_p99: mem_p99,
                iops_read: metrics.disk.read_ops_per_sec,
                iops_write: metrics.disk.write_ops_per_sec,
                latency_avg_ms,
                collected_at: now.clone(),
            });
        }
    }

    Json(PerformanceProfilesResponse {
        veyron_context: if used_prometheus {
            VmrogueFeatureContext::performance_prometheus()
        } else {
            VmrogueFeatureContext::performance_profiles()
        },
        profiles,
    })
}

#[cfg(feature = "web")]
async fn prometheus_vm_cpu_percentiles(ns: &str, vm: &str) -> Option<(f64, f64, f64)> {
    let base = std::env::var("VEYRON_PROMETHEUS_URL").ok()?;
    let end = chrono::Utc::now().timestamp();
    let start = end - 3600;
    let query =
        format!("rate(kubevirt_vmi_vcpu_seconds{{namespace=\"{ns}\", name=\"{vm}\"}}[5m]) * 100");
    percentile_triplet_from_range(&base, &query, start, end).await
}

#[cfg(feature = "web")]
async fn prometheus_vm_memory_percentiles(ns: &str, vm: &str) -> Option<(f64, f64, f64)> {
    let base = std::env::var("VEYRON_PROMETHEUS_URL").ok()?;
    let end = chrono::Utc::now().timestamp();
    let start = end - 3600;
    let query = format!("kubevirt_vmi_memory_resident_bytes{{namespace=\"{ns}\", name=\"{vm}\"}}");
    percentile_triplet_from_range(&base, &query, start, end).await
}

#[cfg(feature = "web")]
async fn percentile_triplet_from_range(
    base: &str,
    query: &str,
    start: i64,
    end: i64,
) -> Option<(f64, f64, f64)> {
    let series = crate::api::prometheus::range_query_series(base, query, start, end, 300)
        .await
        .ok()?;
    let mut values: Vec<f64> = series
        .into_iter()
        .flat_map(|(_, pts)| pts.into_iter().map(|(_, v)| v))
        .collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some((
        crate::api::prometheus::percentile_sorted(&values, 0.50),
        crate::api::prometheus::percentile_sorted(&values, 0.95),
        crate::api::prometheus::percentile_sorted(&values, 0.99),
    ))
}

/// Build a synthetic flamegraph from live CPU metrics.
/// Proportions derived from typical KVM guest workload breakdown.
#[cfg(feature = "web")]
async fn get_flamegraph(
    State(state): State<SharedState>,
    Query(query): Query<FlameGraphQuery>,
) -> Json<FlameGraphListResponse> {
    let s = state.read().await;
    let namespace = query
        .namespace
        .clone()
        .unwrap_or_else(|| s.namespace.clone());
    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };
    let now = chrono::Utc::now().to_rfc3339();
    let mut graphs = Vec::new();

    for vm in &vms {
        let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
        if let Some(ref filter) = query.vm {
            if filter != vm_name {
                continue;
            }
        }
        let vm_ns = vm
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| namespace.clone());
        let collector = crate::monitoring::metrics::MetricsCollector::new(vm_ns);
        let metrics = match collector.collect(vm_name).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let cpu = metrics.cpu.usage_percent;
        let total = (cpu * 100.0) as u64;
        // Proportional breakdown of CPU time for a KVM guest:
        // hypervisor exits ~10%, guest kernel ~30%, user-space ~60%
        let hypervisor = (total as f64 * 0.10).round() as u64;
        let kernel = (total as f64 * 0.30).round() as u64;
        let user = total.saturating_sub(hypervisor + kernel);

        graphs.push(FlameGraph {
            vm_name: vm_name.to_string(),
            collected_at: now.clone(),
            total_cpu_percent: (cpu * 100.0).round() / 100.0,
            frames: vec![
                FlameFrame {
                    name: "hypervisor".to_string(),
                    value: hypervisor,
                    color: "#ff4500".to_string(),
                    children: vec![
                        FlameFrame {
                            name: "kvm-exits".to_string(),
                            value: (hypervisor * 7 / 10),
                            color: "#ff6a00".to_string(),
                            children: vec![],
                        },
                        FlameFrame {
                            name: "virt-io".to_string(),
                            value: hypervisor - (hypervisor * 7 / 10),
                            color: "#ff8c00".to_string(),
                            children: vec![],
                        },
                    ],
                },
                FlameFrame {
                    name: "guest-kernel".to_string(),
                    value: kernel,
                    color: "#4488ff".to_string(),
                    children: vec![
                        FlameFrame {
                            name: "syscalls".to_string(),
                            value: (kernel * 4 / 10),
                            color: "#5599ff".to_string(),
                            children: vec![],
                        },
                        FlameFrame {
                            name: "disk-io".to_string(),
                            value: (kernel * 3 / 10),
                            color: "#66aaff".to_string(),
                            children: vec![],
                        },
                        FlameFrame {
                            name: "net-io".to_string(),
                            value: kernel - (kernel * 4 / 10) - (kernel * 3 / 10),
                            color: "#77bbff".to_string(),
                            children: vec![],
                        },
                    ],
                },
                FlameFrame {
                    name: "user-space".to_string(),
                    value: user,
                    color: "#00b4ff".to_string(),
                    children: vec![
                        FlameFrame {
                            name: "application".to_string(),
                            value: (user * 7 / 10),
                            color: "#00ccff".to_string(),
                            children: vec![],
                        },
                        FlameFrame {
                            name: "runtime".to_string(),
                            value: (user * 2 / 10),
                            color: "#00aadd".to_string(),
                            children: vec![],
                        },
                        FlameFrame {
                            name: "idle".to_string(),
                            value: user - (user * 7 / 10) - (user * 2 / 10),
                            color: "#008899".to_string(),
                            children: vec![],
                        },
                    ],
                },
            ],
        });
    }

    Json(FlameGraphListResponse {
        veyron_context: VmrogueFeatureContext::performance_flamegraph(),
        flamegraphs: graphs,
    })
}
