// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! KubeVirt cluster-scoped `MigrationPolicy` CRs (`migrations.kubevirt.io/v1alpha1`).

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
#[cfg(feature = "web")]
use kube::{
    Api,
    api::{ApiResource, DeleteParams, DynamicObject, GroupVersionKind, ListParams, PostParams},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPolicySummary {
    pub name: String,
    pub labels: std::collections::BTreeMap<String, String>,
    pub spec: serde_json::Value,
}

#[cfg(feature = "web")]
fn migration_policy_api(client: kube::Client) -> Api<DynamicObject> {
    let gvk = GroupVersionKind::gvk("migrations.kubevirt.io", "v1alpha1", "MigrationPolicy");
    let mut ar = ApiResource::from_gvk(&gvk);
    ar.plural = "migrationpolicies".to_string();
    ar.api_version = "migrations.kubevirt.io/v1alpha1".to_string();
    Api::all_with(client, &ar)
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route(
            "/kubevirt/migration-policies",
            get(list_migration_policies).post(create_migration_policy),
        )
        .route(
            "/kubevirt/migration-policies/:name",
            get(get_migration_policy)
                .put(replace_migration_policy)
                .delete(delete_migration_policy),
        )
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_migration_policies(
    State(state): State<SharedState>,
) -> Result<Json<Vec<MigrationPolicySummary>>, StatusCode> {
    let s = state.read().await;
    let api = migration_policy_api(s.client().client());
    let list = api
        .list(&ListParams::default().limit(500))
        .await
        .map_err(|e| {
            log::warn!("list MigrationPolicy: {}", e);
            StatusCode::SERVICE_UNAVAILABLE
        })?;

    let mut out = Vec::with_capacity(list.items.len());
    for obj in list.items {
        let meta = obj.metadata.clone();
        let name = meta.name.clone().unwrap_or_default();
        let labels = meta.labels.clone().unwrap_or_default();
        let spec = obj
            .data
            .get("spec")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        out.push(MigrationPolicySummary { name, labels, spec });
    }
    Ok(Json(out))
}

#[cfg(feature = "web")]
async fn get_migration_policy(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.read().await;
    let api = migration_policy_api(s.client().client());
    let obj = api.get(&name).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let v = serde_json::to_value(&obj).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(v))
}

/// Validate the payload against the typed spec before the untyped passthrough:
/// a policy whose spec doesn't parse, or that selects nothing, silently
/// applies to nothing — better to reject up front. Unknown extra fields are
/// tolerated (future KubeVirt versions may add knobs).
#[cfg(feature = "web")]
fn validate_policy_spec(obj: &DynamicObject) -> Result<(), String> {
    let spec = obj
        .data
        .get("spec")
        .ok_or_else(|| "missing spec".to_string())?;
    let typed: crate::kube::types::MigrationPolicySpec = serde_json::from_value(spec.clone())
        .map_err(|e| format!("spec does not match MigrationPolicy schema: {e}"))?;
    if typed.selectors.namespace_selector.is_none() && typed.selectors.vmi_selector.is_none() {
        return Err(
            "spec.selectors must set namespaceSelector and/or virtualMachineInstanceSelector"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(feature = "web")]
async fn create_migration_policy(
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let obj: DynamicObject = serde_json::from_value(body).map_err(|e| {
        log::warn!("MigrationPolicy create parse: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    validate_policy_spec(&obj).map_err(|e| {
        log::warn!("MigrationPolicy create rejected: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    let s = state.read().await;
    let api = migration_policy_api(s.client().client());
    let created = api
        .create(&PostParams::default(), &obj)
        .await
        .map_err(|e| {
            log::warn!("create MigrationPolicy: {}", e);
            if let kube::Error::Api(ae) = &e {
                if ae.code == 409 {
                    return StatusCode::CONFLICT;
                }
            }
            StatusCode::BAD_REQUEST
        })?;
    let v = serde_json::to_value(&created).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(v)))
}

#[cfg(feature = "web")]
async fn replace_migration_policy(
    State(state): State<SharedState>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut obj: DynamicObject = serde_json::from_value(body).map_err(|e| {
        log::warn!("MigrationPolicy replace parse: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    if obj.metadata.name.is_none() {
        obj.metadata.name = Some(name.clone());
    }
    validate_policy_spec(&obj).map_err(|e| {
        log::warn!("MigrationPolicy replace rejected: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    let s = state.read().await;
    let api = migration_policy_api(s.client().client());
    let replaced = api
        .replace(&name, &Default::default(), &obj)
        .await
        .map_err(|e| {
            log::warn!("replace MigrationPolicy: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let v = serde_json::to_value(&replaced).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(v))
}

#[cfg(feature = "web")]
async fn delete_migration_policy(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.read().await;
    let api = migration_policy_api(s.client().client());
    api.delete(&name, &DeleteParams::default())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(serde_json::json!({ "deleted": name })))
}
