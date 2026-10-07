// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Multi-tenant workspace API — cluster-backed ConfigMaps + namespace bootstrap.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

pub const TENANT_CM_LABEL: &str = "veyron.io/type=tenant";
pub const TENANT_NS_LABEL: &str = "veyron.io/tenant";
const TENANT_DATA_KEY: &str = "tenant.json";
const TENANT_NS: &str = "veyron-system";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRecord {
    pub id: String,
    pub display_name: String,
    pub owner_email: String,
    pub status: String,
    pub namespaces: Vec<String>,
    pub billing_tags: std::collections::BTreeMap<String, String>,
    pub cpu_quota: String,
    pub memory_quota: String,
    pub max_vms: u32,
    #[serde(default)]
    pub gpu_quota: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateTenantRequest {
    pub id: String,
    pub display_name: String,
    pub owner_email: String,
    #[serde(default)]
    pub billing_tags: std::collections::BTreeMap<String, String>,
    #[serde(default = "default_cpu")]
    pub cpu_quota: String,
    #[serde(default = "default_mem")]
    pub memory_quota: String,
    #[serde(default = "default_max_vms")]
    pub max_vms: u32,
    #[serde(default)]
    pub gpu_quota: u32,
    #[serde(default = "default_true")]
    pub bootstrap_namespace: bool,
}

fn default_cpu() -> String {
    "20".to_string()
}
fn default_mem() -> String {
    "64Gi".to_string()
}
fn default_max_vms() -> u32 {
    50
}
fn default_true() -> bool {
    true
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/tenants", get(list_tenants).post(create_tenant))
        .route("/tenants/:id", get(get_tenant).delete(delete_tenant))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_tenant_cms(
    client: kube::Client,
) -> Result<Vec<k8s_openapi::api::core::v1::ConfigMap>, kube::Error> {
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::{Api, ListParams};
    let api: Api<ConfigMap> = Api::namespaced(client, TENANT_NS);
    let lp = ListParams::default().labels(TENANT_CM_LABEL);
    api.list(&lp).await.map(|l| l.items)
}

#[cfg(feature = "web")]
fn cm_to_tenant(cm: &k8s_openapi::api::core::v1::ConfigMap) -> Option<TenantRecord> {
    let raw = cm.data.as_ref()?.get(TENANT_DATA_KEY)?;
    serde_json::from_str(raw).ok()
}

#[cfg(feature = "web")]
async fn bootstrap_tenant_namespace(
    client: kube::Client,
    tenant_id: &str,
    cpu: &str,
    memory: &str,
    max_vms: u32,
    gpu_quota: u32,
) -> Result<String, String> {
    use k8s_openapi::api::core::v1::{Namespace, ResourceQuota};
    use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
    use kube::api::{Api, PostParams};
    use std::collections::BTreeMap;

    let ns_name = format!("tenant-{tenant_id}");
    let ns_api: Api<Namespace> = Api::all(client.clone());
    match ns_api.get_opt(&ns_name).await.map_err(|e| e.to_string())? {
        Some(ns) => {
            if ns
                .metadata
                .labels
                .as_ref()
                .and_then(|l| l.get("veyron.io/vm-backend"))
                .map(String::as_str)
                != Some(crate::api::vm_backend::selected().as_str())
            {
                return Err("existing namespace needs an explicit matching veyron.io/vm-backend label; inspect inventory before adopting".into());
            }
            if ns
                .metadata
                .labels
                .as_ref()
                .and_then(|l| l.get(TENANT_NS_LABEL))
                .map(String::as_str)
                != Some(tenant_id)
            {
                return Err("existing namespace is not owned by this tenant".into());
            }
        }
        None => {
            let mut labels = BTreeMap::new();
            labels.insert(TENANT_NS_LABEL.to_string(), tenant_id.to_string());
            labels.insert(
                "veyron.io/vm-backend".into(),
                crate::api::vm_backend::selected().as_str().into(),
            );
            labels.insert("veyron.io/managed-by".to_string(), "veyron".to_string());
            let ns = Namespace {
                metadata: kube::api::ObjectMeta {
                    name: Some(ns_name.clone()),
                    labels: Some(labels),
                    ..Default::default()
                },
                ..Default::default()
            };
            ns_api
                .create(&PostParams::default(), &ns)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    let rq_api: Api<ResourceQuota> = Api::namespaced(client, &ns_name);
    let rq_name = "tenant-quota";
    let existing = rq_api.get_opt(rq_name).await.map_err(|e| e.to_string())?;
    {
        let mut hard = BTreeMap::new();
        hard.insert("limits.cpu".to_string(), Quantity(cpu.to_string()));
        hard.insert("limits.memory".to_string(), Quantity(memory.to_string()));
        hard.insert(
            if crate::api::vm_backend::selected().as_str() == "kairon" {
                "count/machines.kairon.zyvor.dev"
            } else {
                "count/virtualmachines.kubevirt.io"
            }
            .to_string(),
            Quantity(max_vms.to_string()),
        );
        hard.insert(
            "requests.nvidia.com/gpu".into(),
            Quantity(gpu_quota.to_string()),
        );
        let mut rq = ResourceQuota {
            metadata: kube::api::ObjectMeta {
                name: Some(rq_name.to_string()),
                namespace: Some(ns_name.clone()),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::core::v1::ResourceQuotaSpec {
                hard: Some(hard),
                ..Default::default()
            }),
            ..Default::default()
        };
        if let Some(existing) = existing {
            rq.metadata = existing.metadata;
            rq_api
                .replace(rq_name, &PostParams::default(), &rq)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            rq_api
                .create(&PostParams::default(), &rq)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(ns_name)
}

#[cfg(feature = "web")]
async fn list_tenants(
    State(state): State<SharedState>,
) -> Result<Json<Vec<TenantRecord>>, (StatusCode, Json<serde_json::Value>)> {
    let client = {
        let s = state.read().await;
        s.client().client()
    };
    let cms = list_tenant_cms(client)
        .await
        .map_err(super::kube_list_error("tenants"))?;
    let mut rows: Vec<TenantRecord> = cms.iter().filter_map(cm_to_tenant).collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Json(rows))
}

#[cfg(feature = "web")]
async fn get_tenant(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<TenantRecord>, (StatusCode, Json<serde_json::Value>)> {
    let client = {
        let s = state.read().await;
        s.client().client()
    };
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::Api;
    let api: Api<ConfigMap> = Api::namespaced(client, TENANT_NS);
    let cm_name = format!("tenant-{id}");
    let cm = api.get(&cm_name).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "tenant not found" })),
        )
    })?;
    cm_to_tenant(&cm).map(Json).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "invalid tenant record" })),
    ))
}

#[cfg(feature = "web")]
async fn create_tenant(
    State(state): State<SharedState>,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<TenantRecord>, (StatusCode, Json<serde_json::Value>)> {
    if !crate::enterprise::planning::dns_label(&req.id)
        || req.id.len() > 56
        || req.max_vms == 0
        || !crate::enterprise::planning::quantity(&req.cpu_quota, true).is_ok_and(|n| n > 0)
        || !crate::enterprise::planning::quantity(&req.memory_quota, false).is_ok_and(|n| n > 0)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error":"tenant needs lowercase DNS id <=56 characters, positive max_vms and integer resource quantities"}),
            ),
        ));
    }

    let client = {
        let s = state.read().await;
        s.client().client()
    };

    let mut namespaces = Vec::new();
    if req.bootstrap_namespace {
        match bootstrap_tenant_namespace(
            client.clone(),
            &req.id,
            &req.cpu_quota,
            &req.memory_quota,
            req.max_vms,
            req.gpu_quota,
        )
        .await
        {
            Ok(ns) => namespaces.push(ns),
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e })),
                ));
            }
        }
    }

    let record = TenantRecord {
        id: req.id.clone(),
        display_name: req.display_name,
        owner_email: req.owner_email,
        status: "active".to_string(),
        namespaces,
        billing_tags: req.billing_tags,
        cpu_quota: req.cpu_quota,
        memory_quota: req.memory_quota,
        max_vms: req.max_vms,
        gpu_quota: req.gpu_quota,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::{Api, PostParams};
    use std::collections::BTreeMap;

    let mut labels = BTreeMap::new();
    labels.insert("veyron.io/type".to_string(), "tenant".to_string());
    labels.insert(TENANT_NS_LABEL.to_string(), req.id.clone());

    let mut data = BTreeMap::new();
    data.insert(
        TENANT_DATA_KEY.to_string(),
        serde_json::to_string_pretty(&record).unwrap_or_default(),
    );

    let cm = ConfigMap {
        metadata: kube::api::ObjectMeta {
            name: Some(format!("tenant-{}", req.id)),
            namespace: Some(TENANT_NS.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };

    let api: Api<ConfigMap> = Api::namespaced(client, TENANT_NS);
    api.create(&PostParams::default(), &cm).await.map_err(|e| {
        (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(record))
}

#[cfg(feature = "web")]
async fn delete_tenant(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let client = {
        let s = state.read().await;
        s.client().client()
    };
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::{Api, DeleteParams};
    let api: Api<ConfigMap> = Api::namespaced(client, TENANT_NS);
    api.delete(&format!("tenant-{id}"), &DeleteParams::default())
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "tenant not found" })),
            )
        })?;
    Ok(Json(
        serde_json::json!({ "message": "tenant deleted", "id": id }),
    ))
}
