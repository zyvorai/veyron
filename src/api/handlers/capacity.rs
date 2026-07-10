// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Cluster capacity headroom — total vs VM-allocated CPU/memory, utilization,
//! and how much room is left. Derived from live node capacity and VM specs.

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Serialize)]
pub struct ResourceHeadroom {
    pub total: f64,
    pub allocated: f64,
    pub headroom: f64,
    pub utilization_percent: f64,
    pub unit: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/capacity/headroom", get(capacity_headroom))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn capacity_headroom(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let kube = { state.read().await.kube_client.clone() };
    let nodes = kube.list_nodes().await.unwrap_or_default();
    let vms = kube.list_vms_for_scope("all").await;

    let mut total_cpu = 0f64;
    let mut total_mem_gi = 0f64;
    for node in &nodes {
        // Allocatable is what's schedulable (capacity minus system reserved).
        let alloc = node.status.as_ref().and_then(|s| s.allocatable.as_ref());
        if let Some(c) = alloc
            .and_then(|a| a.get("cpu"))
            .and_then(|v| parse_cpu(&v.0))
        {
            total_cpu += c;
        }
        if let Some(m) = alloc.and_then(|a| a.get("memory")).map(|v| &v.0) {
            total_mem_gi += crate::utils::parse_memory_gib(m);
        }
    }

    let mut vm_cpu = 0f64;
    let mut vm_mem_gi = 0f64;
    for vm in &vms {
        if let Some(cpu) = vm.spec.template.spec.domain.cpu.as_ref() {
            vm_cpu += cpu.cores.unwrap_or(0) as f64;
        }
        if let Some(mem) = vm.spec.template.spec.domain.memory.as_ref() {
            if let Some(guest) = mem.guest.as_ref() {
                vm_mem_gi += crate::utils::parse_memory_gib(guest);
            }
        }
    }

    let cpu = headroom(total_cpu, vm_cpu, "cores");
    let mem = headroom(total_mem_gi, vm_mem_gi, "GiB");

    // How many more "average" VMs fit, gated by the tighter of CPU/memory.
    let n = vms.len().max(1) as f64;
    let avg_cpu = (vm_cpu / n).max(0.1);
    let avg_mem = (vm_mem_gi / n).max(0.1);
    let fit_cpu = (cpu.headroom / avg_cpu).floor().max(0.0);
    let fit_mem = (mem.headroom / avg_mem).floor().max(0.0);

    Json(serde_json::json!({
        "nodes": nodes.len(),
        "vms": vms.len(),
        "cpu": cpu,
        "memory": mem,
        "additional_avg_vms": fit_cpu.min(fit_mem) as u64,
        "basis": "allocatable node capacity vs summed VM requests (point-in-time; trend/days-to-full requires metrics history via /metrics/timeline)"
    }))
}

#[cfg(feature = "web")]
fn headroom(total: f64, allocated: f64, unit: &str) -> ResourceHeadroom {
    let headroom = (total - allocated).max(0.0);
    let utilization_percent = if total > 0.0 {
        (allocated / total * 100.0 * 10.0).round() / 10.0
    } else {
        0.0
    };
    ResourceHeadroom {
        total: (total * 100.0).round() / 100.0,
        allocated: (allocated * 100.0).round() / 100.0,
        headroom: (headroom * 100.0).round() / 100.0,
        utilization_percent,
        unit: unit.to_string(),
    }
}

/// Parse a k8s CPU quantity (e.g. `"8"`, `"8000m"`) into whole/fractional cores.
#[cfg(feature = "web")]
fn parse_cpu(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(milli) = s.strip_suffix('m') {
        milli.parse::<f64>().ok().map(|m| m / 1000.0)
    } else {
        s.parse::<f64>().ok()
    }
}
