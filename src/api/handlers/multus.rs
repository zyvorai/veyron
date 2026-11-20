// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Multus NetworkAttachmentDefinition inventory and VM attach helpers.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadRecord {
    pub name: String,
    pub namespace: String,
    pub config: String,
    pub scope: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NadQuery {
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttachMultusRequest {
    pub nad_namespace: String,
    pub nad_name: String,
    #[serde(default)]
    pub interface_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachMultusResponse {
    pub interface_name: String,
    pub multus_network_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateNadRequest {
    pub name: String,
    #[serde(default = "default_nad_ns")]
    pub namespace: String,
    /// CNI JSON config string (e.g. bridge/macvlan). Optional — a simple bridge stub is used when empty.
    #[serde(default)]
    pub config: Option<String>,
    #[serde(default)]
    pub cidr: Option<String>,
}

fn default_nad_ns() -> String {
    "default".to_string()
}

#[cfg(feature = "web")]
fn nad_api_resource() -> kube::api::ApiResource {
    kube::api::ApiResource {
        group: "k8s.cni.cncf.io".to_string(),
        version: "v1".to_string(),
        api_version: "k8s.cni.cncf.io/v1".to_string(),
        kind: "NetworkAttachmentDefinition".to_string(),
        plural: "network-attachment-definitions".to_string(),
    }
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/network/nads", get(list_nads).post(create_nad))
        .route("/network/nads/:ns/:name", delete(delete_nad))
        .route("/vms/:ns/:name/network/multus", post(attach_multus_to_vm))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_nads(
    State(state): State<SharedState>,
    Query(q): Query<NadQuery>,
) -> Json<Vec<NadRecord>> {
    use kube::api::{Api, DynamicObject, ListParams};

    let s = state.read().await;
    let client = s.client().client();
    let scope = q.namespace.clone().unwrap_or_else(|| "all".to_string());
    let ar = nad_api_resource();

    let api: Api<DynamicObject> = if scope == "all" {
        Api::all_with(client, &ar)
    } else {
        Api::namespaced_with(client, &scope, &ar)
    };

    let list = match api.list(&ListParams::default().limit(200)).await {
        Ok(l) => l.items,
        Err(_) => return Json(vec![]),
    };

    let rows: Vec<NadRecord> = list
        .into_iter()
        .filter_map(|o| {
            let name = o.metadata.name?;
            let namespace = o
                .metadata
                .namespace
                .unwrap_or_else(|| "default".to_string());
            let config = o
                .data
                .get("spec")
                .and_then(|spec| spec.get("config"))
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            Some(NadRecord {
                name,
                namespace,
                config,
                scope: if scope == "all" {
                    "cluster".to_string()
                } else {
                    "namespace".to_string()
                },
            })
        })
        .collect();

    Json(rows)
}

#[cfg(feature = "web")]
async fn create_nad(
    State(state): State<SharedState>,
    Json(req): Json<CreateNadRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    use kube::api::{DynamicObject, PostParams};

    if req.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "name required" })),
        ));
    }
    let config = req.config.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| {
        let cidr = req.cidr.as_deref().unwrap_or("10.244.100.0/24");
        format!(
            r#"{{"cniVersion":"0.3.1","type":"bridge","bridge":"br-{}","ipam":{{"type":"host-local","subnet":"{}"}}}}"#,
            req.name, cidr
        )
    });

    let doc = serde_json::json!({
        "apiVersion": "k8s.cni.cncf.io/v1",
        "kind": "NetworkAttachmentDefinition",
        "metadata": {
            "name": req.name,
            "namespace": req.namespace,
            "labels": { "veyron.io/managed": "true" }
        },
        "spec": { "config": config }
    });

    let client = { state.read().await.client().client() };
    let api: kube::Api<DynamicObject> =
        kube::Api::namespaced_with(client, &req.namespace, &nad_api_resource());
    let obj: DynamicObject = serde_json::from_value(doc).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    match api.create(&PostParams::default(), &obj).await {
        Ok(created) => Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({
                "status": "created",
                "name": created.metadata.name,
                "namespace": req.namespace,
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

#[cfg(feature = "web")]
async fn delete_nad(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    use kube::api::{DeleteParams, DynamicObject};

    let client = { state.read().await.client().client() };
    let api: kube::Api<DynamicObject> =
        kube::Api::namespaced_with(client, &ns, &nad_api_resource());
    match api.delete(&name, &DeleteParams::default()).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "status": "deleted",
            "name": name,
            "namespace": ns,
        }))),
        Err(e) => {
            let msg = e.to_string();
            let code = if msg.contains("NotFound") || msg.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((code, Json(serde_json::json!({ "error": msg }))))
        }
    }
}

#[cfg(feature = "web")]
async fn attach_multus_to_vm(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<AttachMultusRequest>,
) -> Result<Json<AttachMultusResponse>, (StatusCode, Json<serde_json::Value>)> {
    if ns.is_empty() || name.is_empty() || req.nad_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "namespace, vm name, and nad_name required" })),
        ));
    }
    let nad_ns = if req.nad_namespace.is_empty() {
        ns.clone()
    } else {
        req.nad_namespace.clone()
    };
    let multus_network_name = if nad_ns == ns {
        req.nad_name.clone()
    } else {
        format!("{nad_ns}/{}", req.nad_name)
    };

    let s = state.read().await;
    let client = s.client().clone();
    drop(s);

    match client
        .attach_multus_nad(
            &ns,
            &name,
            &nad_ns,
            &req.nad_name,
            req.interface_name.as_deref(),
        )
        .await
    {
        Ok((_, iface)) => Ok(Json(AttachMultusResponse {
            interface_name: iface,
            multus_network_name,
            message: format!(
                "Attached Multus NAD {nad_ns}/{} to VM {ns}/{name}. Restart the VM if it was running.",
                req.nad_name
            ),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}
