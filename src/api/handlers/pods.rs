#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
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
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Json<Vec<PodLogEntry>> {
    use k8s_openapi::api::core::v1::Pod;
    use kube::api::LogParams;

    let s = state.read().await;
    let pods_api: kube::api::Api<Pod> =
        kube::api::Api::namespaced(s.client().client(), &s.namespace);

    let params = LogParams {
        tail_lines: Some(100),
        timestamps: true,
        ..Default::default()
    };

    let log_text = match pods_api.logs(&name, &params).await {
        Ok(text) => text,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<PodLogEntry> = log_text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            // With timestamps=true, each line is: "RFC3339_TIMESTAMP message..."
            let (timestamp, message) = match line.find(' ') {
                Some(pos) => (line[..pos].to_string(), line[pos + 1..].to_string()),
                None => (String::new(), line.to_string()),
            };
            PodLogEntry {
                timestamp,
                container: "default".to_string(),
                message,
            }
        })
        .collect();

    Json(results)
}
