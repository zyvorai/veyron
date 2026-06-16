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

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

/// Health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub service: String,
    pub uptime_seconds: u64,
}

/// Readiness/liveness probe response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResponse {
    pub ready: bool,
    pub checks: Vec<HealthCheck>,
}

/// Individual health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
        .route("/health/live", get(liveness_check))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn health_check(State(state): State<SharedState>) -> Json<HealthResponse> {
    let s = state.read().await;

    // Check K8s connectivity
    let k8s_status = match s.client().list_nodes().await {
        Ok(_) => "healthy",
        Err(_) => "degraded",
    };

    // Read process uptime from /proc/self/stat on Linux
    let uptime = read_process_uptime().unwrap_or(0);

    Json(HealthResponse {
        status: k8s_status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "veyron-api".to_string(),
        uptime_seconds: uptime,
    })
}

#[cfg(feature = "web")]
async fn readiness_check(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<ProbeResponse> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);

    // Check K8s API connectivity
    let k8s_check = match s.client().list_nodes().await {
        Ok(_) => HealthCheck {
            name: "kubernetes".into(),
            status: "healthy".into(),
            message: None,
        },
        Err(_) => HealthCheck {
            name: "kubernetes".into(),
            status: "unhealthy".into(),
            message: Some("Kubernetes API unavailable".into()),
        },
    };

    // Check KubeVirt API availability (cluster-scoped list when `namespace=all`)
    let kubevirt_check = if namespace_scope::is_all_namespaces(&scope) {
        match s.client().list_all_vms().await {
            Ok(_) => HealthCheck {
                name: "kubevirt".into(),
                status: "healthy".into(),
                message: None,
            },
            Err(_) => HealthCheck {
                name: "kubevirt".into(),
                status: "unhealthy".into(),
                message: Some("KubeVirt API unavailable".into()),
            },
        }
    } else {
        match s.client().list_vms(&scope).await {
            Ok(_) => HealthCheck {
                name: "kubevirt".into(),
                status: "healthy".into(),
                message: None,
            },
            Err(_) => HealthCheck {
                name: "kubevirt".into(),
                status: "unhealthy".into(),
                message: Some("KubeVirt API unavailable".into()),
            },
        }
    };

    let all_healthy = k8s_check.status == "healthy" && kubevirt_check.status == "healthy";

    Json(ProbeResponse {
        ready: all_healthy,
        checks: vec![k8s_check, kubevirt_check],
    })
}

#[cfg(feature = "web")]
async fn liveness_check(State(_state): State<SharedState>) -> Json<ProbeResponse> {
    // Basic liveness: process is alive and can respond
    let process_check = HealthCheck {
        name: "process".into(),
        status: "healthy".into(),
        message: Some("Process is running".into()),
    };

    Json(ProbeResponse {
        ready: true,
        checks: vec![process_check],
    })
}

/// Read process uptime in seconds from /proc/self/stat (Linux only).
/// Returns None on non-Linux or if parsing fails.
#[cfg(feature = "web")]
fn read_process_uptime() -> Option<u64> {
    // Read process start time from /proc/self/stat
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // Field 22 (0-indexed: 21) is starttime in clock ticks
    let fields: Vec<&str> = stat.split_whitespace().collect();
    let starttime_ticks: u64 = fields.get(21)?.parse().ok()?;

    // Read system uptime from /proc/uptime
    let uptime_str = std::fs::read_to_string("/proc/uptime").ok()?;
    let system_uptime_secs: f64 = uptime_str.split_whitespace().next()?.parse().ok()?;

    // Clock ticks per second (usually 100 on Linux)
    let ticks_per_sec: u64 = 100;
    let starttime_secs = starttime_ticks / ticks_per_sec;
    let process_uptime = (system_uptime_secs as u64).saturating_sub(starttime_secs);

    Some(process_uptime)
}
