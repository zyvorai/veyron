// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Multi-cluster discovery from kubeconfig and runtime context switching.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

pub const ACTIVE_CONTEXT_CM: &str = "vmrogue-active-kube-context";
pub const ACTIVE_CONTEXT_LABEL: &str = "vmrogue.io/type=active-kube-context";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub name: String,
    pub context: String,
    pub environment: String,
    pub is_primary: bool,
    pub is_active: bool,
    pub health: String,
    pub vm_count: u32,
    pub node_count: u32,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClustersResponse {
    pub vmrogue_context: super::feature_context::VmrogueFeatureContext,
    /// Context used for live API calls (override or kubeconfig default).
    pub current_context: String,
    /// `current_context` field from the kubeconfig file on disk.
    pub kubeconfig_current_context: String,
    pub clusters: Vec<ClusterSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateClusterResponse {
    pub status: String,
    pub cluster: String,
    pub context: String,
    pub message: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/clusters", get(list_clusters))
        .route("/clusters/{name}/sync", get(sync_cluster))
        .route("/clusters/{name}/activate", post(activate_cluster))
        .with_state(state)
}

#[cfg(feature = "web")]
fn clusters_context() -> super::feature_context::VmrogueFeatureContext {
    super::feature_context::VmrogueFeatureContext {
        data_source: "kubeconfig_contexts".to_string(),
        scope: "Contexts from KUBECONFIG on the API pod; POST activate switches the in-process API client.".to_string(),
        limitations: "Requires a kubeconfig file with multiple contexts (not in-cluster SA only). Active context is stored in a ConfigMap in the API namespace.".to_string(),
    }
}

#[cfg(feature = "web")]
fn kubeconfig_file_current_context() -> String {
    let path = std::env::var("KUBECONFIG")
        .ok()
        .filter(|p| !p.is_empty())
        .or_else(|| {
            crate::kube::KUBECONFIG_PATH
                .get()
                .map(|s| s.to_string())
        })
        .or_else(|| {
            dirs::home_dir().map(|h| h.join(".kube/config").to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    if path.is_empty() {
        return String::new();
    }
    kube::config::Kubeconfig::read_from(&path)
        .ok()
        .and_then(|k| k.current_context)
        .unwrap_or_default()
}

#[cfg(feature = "web")]
fn effective_current_context(active: Option<&str>) -> String {
    active
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(kubeconfig_file_current_context)
}

#[cfg(feature = "web")]
pub async fn load_persisted_kube_context(
    client: &crate::kube::KubeClient,
    namespace: &str,
) -> Option<String> {
    use k8s_openapi::api::core::v1::ConfigMap;

    let api: kube::api::Api<ConfigMap> = kube::api::Api::namespaced(client.client(), namespace);
    let cm = api.get(ACTIVE_CONTEXT_CM).await.ok()?;
    cm.data
        .as_ref()
        .and_then(|d| d.get("context"))
        .cloned()
        .filter(|c| !c.is_empty())
}

#[cfg(feature = "web")]
async fn persist_kube_context(
    client: &crate::kube::KubeClient,
    namespace: &str,
    cluster_name: &str,
    context: &str,
) -> anyhow::Result<()> {
    use k8s_openapi::api::core::v1::ConfigMap;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use kube::api::PatchParams;

    let mut labels = std::collections::BTreeMap::new();
    labels.insert("app.kubernetes.io/name".to_string(), "vmrogue".to_string());
    labels.insert(
        "app.kubernetes.io/component".to_string(),
        "api".to_string(),
    );
    labels.insert(ACTIVE_CONTEXT_LABEL.to_string(), "true".to_string());

    let mut data = std::collections::BTreeMap::new();
    data.insert("context".to_string(), context.to_string());
    data.insert("cluster_name".to_string(), cluster_name.to_string());

    let cm = ConfigMap {
        metadata: ObjectMeta {
            name: Some(ACTIVE_CONTEXT_CM.to_string()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };

    let api: kube::api::Api<ConfigMap> = kube::api::Api::namespaced(client.client(), namespace);
    let patch = kube::api::Patch::Apply(cm);
    api.patch(ACTIVE_CONTEXT_CM, &PatchParams::apply("vmrogue"), &patch)
        .await?;
    Ok(())
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
fn summary_from_cluster(
    c: &crate::multi_cluster::ClusterInfo,
    active_ctx: &str,
) -> ClusterSummary {
    ClusterSummary {
        name: c.name.clone(),
        context: c.context.clone(),
        environment: env_label(&c.environment),
        is_primary: c.is_primary,
        is_active: c.context == active_ctx,
        health: health_label(&c.health),
        vm_count: c.vm_count as u32,
        node_count: c.node_count as u32,
        region: c.region.clone(),
    }
}

#[cfg(feature = "web")]
async fn list_clusters(State(state): State<SharedState>) -> Json<ClustersResponse> {
    let s = state.read().await;
    let active = s.active_kube_context.as_deref();
    let current = effective_current_context(active);
    let file_ctx = kubeconfig_file_current_context();

    let mut manager = crate::multi_cluster::MultiClusterManager::new();
    let _ = manager.discover_from_kubeconfig().await;

    let clusters: Vec<ClusterSummary> = manager
        .clusters
        .iter()
        .map(|c| summary_from_cluster(c, &current))
        .collect();

    Json(ClustersResponse {
        vmrogue_context: clusters_context(),
        current_context: current,
        kubeconfig_current_context: file_ctx,
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
    let current = effective_current_context(crate::kube::active_kube_context().as_deref());
    Ok(Json(summary_from_cluster(c, &current)))
}

#[cfg(feature = "web")]
async fn activate_cluster(
    Path(name): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<ActivateClusterResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let mut manager = crate::multi_cluster::MultiClusterManager::new();
    let count = manager.discover_from_kubeconfig().await.unwrap_or(0);
    if count == 0 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "No kubeconfig contexts found. Mount a kubeconfig with multiple contexts on the API pod."
            })),
        ));
    }

    let ctx = manager
        .clusters
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.context.clone())
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("cluster '{}' not found", name) })),
        ))?;

    // Verify the context connects before switching global client.
    if let Err(e) = crate::kube::KubeClient::for_context(&ctx).await {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Cannot connect with context '{}': {}", ctx, e)
            })),
        ));
    }

    let ns = {
        let s = state.read().await;
        s.namespace.clone()
    };

    {
        let mut s = state.write().await;
        s.apply_kube_context(Some(ctx.clone()))
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?;
        if let Err(e) = persist_kube_context(s.client(), &ns, &name, &ctx).await {
            log::warn!("persist active kube context: {e}");
        }
    }

    Ok(Json(ActivateClusterResponse {
        status: "activated".to_string(),
        cluster: name,
        context: ctx,
        message: "API kube client switched to this context. Refresh dashboard views to see the new cluster.".to_string(),
    }))
}
