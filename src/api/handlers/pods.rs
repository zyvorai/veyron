#[cfg(feature = "web")]
use axum::{Json, Router, extract::{Path, State}, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Pod response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodResponse {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub node_name: Option<String>,
    pub ip: Option<String>,
    pub containers: Vec<String>,
    pub restart_count: u32,
    pub created_at: String,
}

/// Pod log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodLogEntry {
    pub timestamp: String,
    pub container: String,
    pub message: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/pods", get(list_pods))
        .route("/pods/{name}", get(get_pod))
        .route("/pods/{name}/logs", get(get_pod_logs))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_pods(State(state): State<SharedState>) -> Json<Vec<PodResponse>> {
    let s = state.read().await;
    let pods = s.client().list_pods(&s.namespace).await.unwrap_or_default();

    let results: Vec<PodResponse> = pods
        .iter()
        .map(|pod| {
            let meta = &pod.metadata;
            let spec = pod.spec.as_ref();
            let status = pod.status.as_ref();

            let containers: Vec<String> = spec
                .map(|s| s.containers.iter().map(|c| c.name.clone()).collect())
                .unwrap_or_default();

            let restart_count: u32 = status
                .and_then(|s| s.container_statuses.as_ref())
                .map(|cs| cs.iter().map(|c| c.restart_count as u32).sum())
                .unwrap_or(0);

            PodResponse {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                phase: status
                    .and_then(|s| s.phase.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
                node_name: spec.and_then(|s| s.node_name.clone()),
                ip: status.and_then(|s| s.pod_ip.clone()),
                containers,
                restart_count,
                created_at: meta
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn get_pod(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Json<Option<PodResponse>> {
    let pods = list_pods(State(state)).await.0;
    Json(pods.into_iter().find(|p| p.name == name))
}

#[cfg(feature = "web")]
async fn get_pod_logs(
    State(_state): State<SharedState>,
    Path(name): Path<String>,
) -> Json<Vec<PodLogEntry>> {
    let _ = name;
    // Pod log streaming requires kube::Api<Pod>::logs() - will be wired in v0.5
    Json(vec![])
}
