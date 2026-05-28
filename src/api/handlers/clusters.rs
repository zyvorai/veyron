// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Multi-cluster discovery from kubeconfig (read-only for dashboard).

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub name: String,
    pub context: String,
    pub environment: String,
    pub is_primary: bool,
    pub health: String,
    pub vm_count: u32,
    pub node_count: u32,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClustersResponse {
    pub vmrogue_context: super::feature_context::VmrogueFeatureContext,
    pub current_context: String,
    pub clusters: Vec<ClusterSummary>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/clusters", get(list_clusters))
        .route("/clusters/{name}/sync", get(sync_cluster))
        .with_state(state)
}

#[cfg(feature = "web")]
fn clusters_context() -> super::feature_context::VmrogueFeatureContext {
    super::feature_context::VmrogueFeatureContext {
        data_source: "kubeconfig_contexts".to_string(),
        scope: "Contexts from KUBECONFIG on the API pod; sync populates VM/node counts per context.".to_string(),
        limitations: "Switching active context requires restarting the API with a different kubeconfig or in-cluster SA. This endpoint is inventory only.".to_string(),
    }
}

#[cfg(feature = "web")]
fn env_label(env: &crate::multi_cluster::ClusterEnvironment) -> String {
    match env {
        crate::multi_cluster::ClusterEnvironment::Production => "production".to_string(),
        crate::multi_cluster::ClusterEnvironment::Staging => "staging".to_string(),
        crate::multi_cluster::ClusterEnvironment::Development => "development".to_string(),
        crate::multi_cluster::ClusterEnvironment::Testing => "testing".to_string(),
        crate::multi_cluster::ClusterEnvironment::Custom(s) => s.clone(),
    }
}

#[cfg(feature = "web")]
fn health_label(h: &crate::multi_cluster::ClusterHealth) -> String {
    match h {
        crate::multi_cluster::ClusterHealth::Healthy => "healthy".to_string(),
        crate::multi_cluster::ClusterHealth::Degraded => "degraded".to_string(),
        crate::multi_cluster::ClusterHealth::Unhealthy => "unhealthy".to_string(),
        crate::multi_cluster::ClusterHealth::Unknown => "unknown".to_string(),
    }
}

#[cfg(feature = "web")]
async fn list_clusters(State(_state): State<SharedState>) -> Json<ClustersResponse> {
    let mut manager = crate::multi_cluster::MultiClusterManager::new();
    let _ = manager.discover_from_kubeconfig().await;

    let current = kube::config::Kubeconfig::read_from(
        std::env::var("KUBECONFIG").unwrap_or_else(|_| {
            dirs::home_dir()
                .map(|h| h.join(".kube/config").to_string_lossy().into_owned())
                .unwrap_or_default()
        }),
    )
    .ok()
    .and_then(|k| k.current_context)
    .unwrap_or_default();

    let clusters: Vec<ClusterSummary> = manager
        .clusters
        .iter()
        .map(|c| ClusterSummary {
            name: c.name.clone(),
            context: c.context.clone(),
            environment: env_label(&c.environment),
            is_primary: c.is_primary,
            health: health_label(&c.health),
            vm_count: c.vm_count as u32,
            node_count: c.node_count as u32,
            region: c.region.clone(),
        })
        .collect();

    Json(ClustersResponse {
        vmrogue_context: clusters_context(),
        current_context: current,
        clusters,
    })
}

#[cfg(feature = "web")]
async fn sync_cluster(
    Path(name): Path<String>,
    State(_state): State<SharedState>,
) -> Result<Json<ClusterSummary>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let mut manager = crate::multi_cluster::MultiClusterManager::new();
    let _ = manager.discover_from_kubeconfig().await;
    manager.sync_cluster(&name).await.map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    let c = manager.get_cluster(&name).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "cluster not found" })),
    ))?;
    Ok(Json(ClusterSummary {
        name: c.name.clone(),
        context: c.context.clone(),
        environment: env_label(&c.environment),
        is_primary: c.is_primary,
        health: health_label(&c.health),
        vm_count: c.vm_count as u32,
        node_count: c.node_count as u32,
        region: c.region.clone(),
    }))
}
