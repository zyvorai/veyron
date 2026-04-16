#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Workload response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadResponse {
    pub name: String,
    pub namespace: String,
    pub workload_type: String,
    pub replicas: u32,
    pub ready_replicas: u32,
    pub cpu_request: String,
    pub memory_request: String,
    pub status: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/workloads", get(list_workloads))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_workloads(State(state): State<SharedState>) -> Json<Vec<WorkloadResponse>> {
    use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};

    let s = state.read().await;
    let ns = &s.namespace;
    let client = s.client().client();
    let mut results: Vec<WorkloadResponse> = Vec::new();

    // Fetch all workload types concurrently
    let deploy_api: kube::api::Api<Deployment> = kube::api::Api::namespaced(client.clone(), ns);
    let sts_api: kube::api::Api<StatefulSet> = kube::api::Api::namespaced(client.clone(), ns);
    let ds_api: kube::api::Api<DaemonSet> = kube::api::Api::namespaced(client.clone(), ns);

    let lp = kube::api::ListParams::default();
    let (deploy_result, sts_result, ds_result) = tokio::join!(
        deploy_api.list(&lp),
        sts_api.list(&lp),
        ds_api.list(&lp),
    );

    // Deployments
    if let Ok(deploys) = deploy_result {
        for d in &deploys.items {
            let status = d.status.as_ref();
            results.push(WorkloadResponse {
                name: d.metadata.name.clone().unwrap_or_default(),
                namespace: d.metadata.namespace.clone().unwrap_or_default(),
                workload_type: "Deployment".to_string(),
                replicas: d.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1) as u32,
                ready_replicas: status.and_then(|s| s.ready_replicas).unwrap_or(0) as u32,
                cpu_request: String::new(),
                memory_request: String::new(),
                status: if status.and_then(|s| s.ready_replicas).unwrap_or(0)
                    >= d.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1)
                {
                    "Ready".to_string()
                } else {
                    "Progressing".to_string()
                },
            });
        }
    }

    // StatefulSets
    if let Ok(stss) = sts_result {
        for s in &stss.items {
            let status = s.status.as_ref();
            results.push(WorkloadResponse {
                name: s.metadata.name.clone().unwrap_or_default(),
                namespace: s.metadata.namespace.clone().unwrap_or_default(),
                workload_type: "StatefulSet".to_string(),
                replicas: s.spec.as_ref().and_then(|sp| sp.replicas).unwrap_or(1) as u32,
                ready_replicas: status.and_then(|st| st.ready_replicas).unwrap_or(0) as u32,
                cpu_request: String::new(),
                memory_request: String::new(),
                status: if status.and_then(|st| st.ready_replicas).unwrap_or(0)
                    >= s.spec.as_ref().and_then(|sp| sp.replicas).unwrap_or(1)
                {
                    "Ready".to_string()
                } else {
                    "Progressing".to_string()
                },
            });
        }
    }

    // DaemonSets
    if let Ok(dss) = ds_result {
        for d in &dss.items {
            let status = d.status.as_ref();
            let desired = status.map(|s| s.desired_number_scheduled).unwrap_or(0);
            let ready = status.map(|s| s.number_ready).unwrap_or(0);
            results.push(WorkloadResponse {
                name: d.metadata.name.clone().unwrap_or_default(),
                namespace: d.metadata.namespace.clone().unwrap_or_default(),
                workload_type: "DaemonSet".to_string(),
                replicas: desired as u32,
                ready_replicas: ready as u32,
                cpu_request: String::new(),
                memory_request: String::new(),
                status: if ready >= desired {
                    "Ready".to_string()
                } else {
                    "Progressing".to_string()
                },
            });
        }
    }

    Json(results)
}
