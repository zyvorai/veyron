// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Node response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResponse {
    pub name: String,
    pub status: String,
    pub roles: Vec<String>,
    pub cpu_capacity: String,
    pub memory_capacity: String,
    pub cpu_allocatable: String,
    pub memory_allocatable: String,
    pub kubelet_version: String,
    pub os_image: String,
    pub kernel_version: String,
    pub vm_count: u32,
    /// True when the node is cordoned (`spec.unschedulable`).
    #[serde(default)]
    pub unschedulable: bool,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/nodes", get(list_nodes))
        .route("/nodes/:name", get(get_node))
        .route("/nodes/:name/cordon", post(cordon_node))
        .route("/nodes/:name/uncordon", post(uncordon_node))
        .route("/nodes/:name/reboot", post(reboot_node))
        .with_state(state)
}

// Mounted directly in http_server.rs next to the live `/api/v1/nodes` route
// (nodes::router below is not merged, so these must be pub and wired there).
#[cfg(feature = "web")]
pub async fn cordon_node(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    set_schedulable(state, name, false).await
}

#[cfg(feature = "web")]
pub async fn uncordon_node(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    set_schedulable(state, name, true).await
}

#[derive(Debug, Deserialize)]
pub struct RebootNodeRequest {
    /// Must be true — host reboot is destructive.
    #[serde(default)]
    pub confirm: bool,
}

/// Cordon the node, then launch a privileged one-shot Job that reboots the host via nsenter.
#[cfg(feature = "web")]
pub async fn reboot_node(
    State(state): State<SharedState>,
    Path(name): Path<String>,
    Json(req): Json<RebootNodeRequest>,
) -> impl IntoResponse {
    use k8s_openapi::api::batch::v1::{Job, JobSpec};
    use k8s_openapi::api::core::v1::{
        Container, HostPathVolumeSource, PodSpec, PodTemplateSpec, SecurityContext, Volume,
        VolumeMount,
    };
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use kube::api::{Api, PostParams};

    if !req.confirm {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "confirm:true required to reboot a node"
            })),
        )
            .into_response();
    }
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "node name required" })),
        )
            .into_response();
    }

    let kube = { state.read().await.kube_client.clone() };
    if let Err(e) = kube.set_node_schedulable(&name, false).await {
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("cordon before reboot failed: {e}") })),
        )
            .into_response();
    }

    let job_name = format!(
        "veyron-reboot-{}",
        name.replace('.', "-")
            .chars()
            .take(40)
            .collect::<String>()
            .to_lowercase()
    );
    let job = Job {
        metadata: ObjectMeta {
            name: Some(job_name.clone()),
            namespace: Some("veyron-system".to_string()),
            labels: Some(
                [
                    ("app.kubernetes.io/name".to_string(), "veyron".to_string()),
                    ("veyron.io/node-reboot".to_string(), name.clone()),
                ]
                .into_iter()
                .collect(),
            ),
            ..Default::default()
        },
        spec: Some(JobSpec {
            ttl_seconds_after_finished: Some(300),
            backoff_limit: Some(0),
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(
                        [("veyron.io/node-reboot".to_string(), name.clone())]
                            .into_iter()
                            .collect(),
                    ),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    restart_policy: Some("Never".to_string()),
                    node_name: Some(name.clone()),
                    host_pid: Some(true),
                    host_network: Some(true),
                    containers: vec![Container {
                        name: "reboot".to_string(),
                        image: Some("busybox:1.36".to_string()),
                        command: Some(vec![
                            "nsenter".to_string(),
                            "--target".to_string(),
                            "1".to_string(),
                            "--mount".to_string(),
                            "--uts".to_string(),
                            "--ipc".to_string(),
                            "--net".to_string(),
                            "--pid".to_string(),
                            "--".to_string(),
                            "reboot".to_string(),
                        ]),
                        security_context: Some(SecurityContext {
                            privileged: Some(true),
                            ..Default::default()
                        }),
                        volume_mounts: Some(vec![VolumeMount {
                            name: "host-root".to_string(),
                            mount_path: "/host".to_string(),
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }],
                    volumes: Some(vec![Volume {
                        name: "host-root".to_string(),
                        host_path: Some(HostPathVolumeSource {
                            path: "/".to_string(),
                            type_: Some("Directory".to_string()),
                        }),
                        ..Default::default()
                    }]),
                    tolerations: Some(vec![k8s_openapi::api::core::v1::Toleration {
                        operator: Some("Exists".to_string()),
                        ..Default::default()
                    }]),
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        status: None,
    };

    let api: Api<Job> = Api::namespaced(kube.client(), "veyron-system");
    // Best-effort delete of a previous reboot job with the same name
    let _ = api
        .delete(&job_name, &kube::api::DeleteParams::default())
        .await;
    match api.create(&PostParams::default(), &job).await {
        Ok(_) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "ok": true,
                "node": name,
                "action": "reboot",
                "job": job_name,
                "message": "Node cordoned; reboot Job scheduled"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("reboot job create failed: {e}") })),
        )
            .into_response(),
    }
}

#[cfg(feature = "web")]
async fn set_schedulable(state: SharedState, name: String, schedulable: bool) -> impl IntoResponse {
    let kube = { state.read().await.kube_client.clone() };
    match kube.set_node_schedulable(&name, schedulable).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true, "node": name,
                "schedulable": schedulable,
                "action": if schedulable { "uncordon" } else { "cordon" }
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!(
                "{} failed: {e}",
                if schedulable { "uncordon" } else { "cordon" }
            ),
        )
            .into_response(),
    }
}

#[cfg(feature = "web")]
async fn list_nodes(State(state): State<SharedState>) -> Json<Vec<NodeResponse>> {
    use crate::kube::types::VirtualMachineInstance;

    let kube_client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    let nodes = kube_client.list_nodes().await.unwrap_or_default();

    // Count running VMs per node via cluster-wide VMI listing
    let mut vmi_per_node: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let vmis_api: kube::api::Api<VirtualMachineInstance> =
        kube::api::Api::all(kube_client.client());
    if let Ok(vmi_list) = vmis_api.list(&kube::api::ListParams::default()).await {
        for vmi in &vmi_list.items {
            if let Some(ref status) = vmi.status {
                if let Some(ref node) = status.node_name {
                    *vmi_per_node.entry(node.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    let results: Vec<NodeResponse> = nodes
        .iter()
        .map(|node| {
            let name = node.metadata.name.clone().unwrap_or_default();
            let status = node
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .and_then(|c| c.iter().find(|c| c.type_ == "Ready"))
                .map(|c| {
                    if c.status == "True" {
                        "Ready"
                    } else {
                        "NotReady"
                    }
                })
                .unwrap_or("Unknown")
                .to_string();

            let roles: Vec<String> = node
                .metadata
                .labels
                .as_ref()
                .map(|labels| {
                    labels
                        .keys()
                        .filter_map(|k| {
                            k.strip_prefix("node-role.kubernetes.io/")
                                .map(|r| r.to_string())
                        })
                        .collect()
                })
                .unwrap_or_default();

            let capacity = node.status.as_ref().and_then(|s| s.capacity.as_ref());
            let allocatable = node.status.as_ref().and_then(|s| s.allocatable.as_ref());

            let cpu_capacity = capacity
                .and_then(|c| c.get("cpu"))
                .map(|v| v.0.clone())
                .unwrap_or_default();
            let memory_capacity = capacity
                .and_then(|c| c.get("memory"))
                .map(|v| v.0.clone())
                .unwrap_or_default();
            let cpu_allocatable = allocatable
                .and_then(|a| a.get("cpu"))
                .map(|v| v.0.clone())
                .unwrap_or_default();
            let memory_allocatable = allocatable
                .and_then(|a| a.get("memory"))
                .map(|v| v.0.clone())
                .unwrap_or_default();

            let node_info = node.status.as_ref().and_then(|s| s.node_info.as_ref());
            let kubelet_version = node_info
                .map(|i| i.kubelet_version.clone())
                .unwrap_or_default();
            let os_image = node_info.map(|i| i.os_image.clone()).unwrap_or_default();
            let kernel_version = node_info
                .map(|i| i.kernel_version.clone())
                .unwrap_or_default();

            let vm_count = vmi_per_node.get(name.as_str()).copied().unwrap_or(0);
            let unschedulable = node
                .spec
                .as_ref()
                .and_then(|s| s.unschedulable)
                .unwrap_or(false);

            NodeResponse {
                name,
                status,
                roles,
                cpu_capacity,
                memory_capacity,
                cpu_allocatable,
                memory_allocatable,
                kubelet_version,
                os_image,
                kernel_version,
                vm_count,
                unschedulable,
            }
        })
        .collect();

    Json(results)
}

// Mounted directly in http_server.rs next to /api/v1/nodes (nodes::router isn't merged).
#[cfg(feature = "web")]
pub async fn get_node(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Json<Option<NodeResponse>> {
    let nodes = list_nodes(State(state)).await.0;
    Json(nodes.into_iter().find(|n| n.name == name))
}
