// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Velero Backup and Restore discovery.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Velero install namespace (CRs live here). `VELERO_NAMESPACE` overrides.
#[cfg(feature = "web")]
fn velero_namespace() -> String {
    crate::api::integrations::env_var("VELERO_NAMESPACE").unwrap_or_else(|| "velero".to_string())
}

#[cfg(feature = "web")]
fn velero_ar(kind: &str) -> kube::api::ApiResource {
    kube::api::ApiResource::from_gvk(&kube::core::GroupVersionKind::gvk("velero.io", "v1", kind))
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateVeleroBackupRequest {
    /// Namespaces to back up (defaults to all).
    #[serde(default)]
    pub included_namespaces: Vec<String>,
    /// Backup name (defaults to `veyron-<ts>`).
    #[serde(default)]
    pub name: Option<String>,
    /// TTL, e.g. `"720h0m0s"` (Velero default when omitted).
    #[serde(default)]
    pub ttl: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateVeleroRestoreRequest {
    /// Source backup name (required).
    pub backup_name: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeleroBackupRecord {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub storage_location: String,
    pub completion_timestamp: Option<String>,
    pub items_backed_up: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeleroRestoreRecord {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub backup_name: String,
    pub completion_timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeleroStatusResponse {
    pub veyron_context: super::feature_context::VmrogueFeatureContext,
    /// The Velero API type is servable (CRDs installed) — does NOT mean
    /// backups will actually complete, see `backups_functional`.
    pub velero_available: bool,
    /// A real, `Available`-phase BackupStorageLocation exists. Reproduced
    /// live: velero_available can be true (CRDs installed) while Velero has
    /// no controller and no storage location at all — every Backup object
    /// created then sits with an empty status forever, no error, no
    /// progress. This is the signal that actually predicts whether
    /// `POST /velero/backups` will do anything.
    pub backups_functional: bool,
    pub backups: Vec<VeleroBackupRecord>,
    pub restores: Vec<VeleroRestoreRecord>,
}

/// A real, ready-to-use BackupStorageLocation exists — the signal that
/// distinguishes "Velero CRDs are installed" from "Velero backups will
/// actually work here" (see `VeleroStatusResponse::backups_functional`).
#[cfg(feature = "web")]
pub(crate) async fn velero_backups_functional(client: kube::Client) -> bool {
    use kube::api::{Api, ApiResource, DynamicObject};
    let ar = ApiResource::from_gvk(&kube::core::GroupVersionKind::gvk(
        "velero.io",
        "v1",
        "BackupStorageLocation",
    ));
    let api: Api<DynamicObject> = Api::all_with(client, &ar);
    let Ok(list) = api.list(&kube::api::ListParams::default()).await else {
        return false;
    };
    list.items.iter().any(|o| {
        o.data
            .get("status")
            .and_then(|s| s.get("phase"))
            .and_then(|p| p.as_str())
            == Some("Available")
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct VeleroQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/velero/status", get(velero_status))
        .route("/velero/backups", post(create_velero_backup))
        .route("/velero/restores", post(create_velero_restore))
        .with_state(state)
}

#[cfg(feature = "web")]
fn ts_suffix() -> String {
    chrono::Utc::now().format("%Y%m%d%H%M%S").to_string()
}

/// Trigger a Velero backup by creating a `velero.io/v1 Backup` CR.
#[cfg(feature = "web")]
async fn create_velero_backup(
    State(state): State<SharedState>,
    Json(req): Json<CreateVeleroBackupRequest>,
) -> impl IntoResponse {
    use kube::api::{Api, DynamicObject, ObjectMeta, PostParams, TypeMeta};

    let client = { state.read().await.kube_client.client().clone() };
    let ns = velero_namespace();
    let name = req
        .name
        .unwrap_or_else(|| format!("veyron-{}", ts_suffix()));
    let mut spec = serde_json::Map::new();
    if !req.included_namespaces.is_empty() {
        spec.insert("includedNamespaces".into(), req.included_namespaces.into());
    }
    if let Some(ttl) = req.ttl.filter(|t| !t.trim().is_empty()) {
        spec.insert("ttl".into(), ttl.into());
    }
    let obj = DynamicObject {
        types: Some(TypeMeta {
            api_version: "velero.io/v1".into(),
            kind: "Backup".into(),
        }),
        metadata: ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            ..Default::default()
        },
        data: serde_json::json!({ "spec": serde_json::Value::Object(spec) }),
    };
    let api: Api<DynamicObject> = Api::namespaced_with(client, &ns, &velero_ar("Backup"));
    match api.create(&PostParams::default(), &obj).await {
        Ok(_) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "ok": true, "backup": name, "namespace": ns })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("velero backup create failed: {e}"),
        )
            .into_response(),
    }
}

/// Trigger a Velero restore from a backup by creating a `velero.io/v1 Restore` CR.
#[cfg(feature = "web")]
async fn create_velero_restore(
    State(state): State<SharedState>,
    Json(req): Json<CreateVeleroRestoreRequest>,
) -> impl IntoResponse {
    use kube::api::{Api, DynamicObject, ObjectMeta, PostParams, TypeMeta};

    if req.backup_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "backup_name is required".to_string(),
        )
            .into_response();
    }
    let client = { state.read().await.kube_client.client().clone() };
    let ns = velero_namespace();
    let name = req
        .name
        .unwrap_or_else(|| format!("{}-restore-{}", req.backup_name, ts_suffix()));
    let obj = DynamicObject {
        types: Some(TypeMeta {
            api_version: "velero.io/v1".into(),
            kind: "Restore".into(),
        }),
        metadata: ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            ..Default::default()
        },
        data: serde_json::json!({ "spec": { "backupName": req.backup_name } }),
    };
    let api: Api<DynamicObject> = Api::namespaced_with(client, &ns, &velero_ar("Restore"));
    match api.create(&PostParams::default(), &obj).await {
        Ok(_) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "ok": true, "restore": name, "namespace": ns })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("velero restore create failed: {e}"),
        )
            .into_response(),
    }
}

#[cfg(feature = "web")]
fn velero_context(available: bool) -> super::feature_context::VmrogueFeatureContext {
    if available {
        super::feature_context::VmrogueFeatureContext {
            data_source: "velero_backup_restore_crds".to_string(),
            scope: "Velero Backup and Restore objects cluster-wide.".to_string(),
            limitations: "Policy creation and scheduled backups require Velero CLI or Helm; Veyron lists CR status only.".to_string(),
        }
    } else {
        super::feature_context::VmrogueFeatureContext {
            data_source: "velero_not_installed".to_string(),
            scope: "Velero CRDs not detected.".to_string(),
            limitations:
                "Install Velero in the cluster or use KubeVirt snapshot schedules as tier-1 backup."
                    .to_string(),
        }
    }
}

#[cfg(feature = "web")]
async fn velero_status(
    State(state): State<SharedState>,
    Query(_q): Query<VeleroQuery>,
) -> Json<VeleroStatusResponse> {
    use kube::api::{Api, ApiResource, DynamicObject, ListParams};

    let client = {
        let s = state.read().await;
        s.client().client()
    };

    let backup_ar = ApiResource {
        group: "velero.io".to_string(),
        version: "v1".to_string(),
        api_version: "velero.io/v1".to_string(),
        kind: "Backup".to_string(),
        plural: "backups".to_string(),
    };
    let restore_ar = ApiResource {
        group: "velero.io".to_string(),
        version: "v1".to_string(),
        api_version: "velero.io/v1".to_string(),
        kind: "Restore".to_string(),
        plural: "restores".to_string(),
    };

    let client_for_bsl_check = client.clone();
    let backup_api: Api<DynamicObject> = Api::all_with(client.clone(), &backup_ar);
    let restore_api: Api<DynamicObject> = Api::all_with(client, &restore_ar);

    let backup_list = backup_api.list(&ListParams::default().limit(50)).await;
    let velero_available = backup_list.is_ok()
        || restore_api
            .list(&ListParams::default().limit(1))
            .await
            .is_ok();

    let backups: Vec<VeleroBackupRecord> = backup_list
        .ok()
        .map(|l| {
            l.items
                .into_iter()
                .filter_map(|o| {
                    let name = o.metadata.name?;
                    let ns = o.metadata.namespace.unwrap_or_else(|| "velero".to_string());
                    let phase = o
                        .data
                        .get("status")
                        .and_then(|st| st.get("phase"))
                        .and_then(|p| p.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let storage_location = o
                        .data
                        .get("spec")
                        .and_then(|sp| sp.get("storageLocation"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let completion = o
                        .data
                        .get("status")
                        .and_then(|st| st.get("completionTimestamp"))
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let items = o
                        .data
                        .get("status")
                        .and_then(|st| st.get("progress"))
                        .and_then(|p| p.get("itemsBackedUp"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    Some(VeleroBackupRecord {
                        name,
                        namespace: ns,
                        phase,
                        storage_location,
                        completion_timestamp: completion,
                        items_backed_up: items,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let restores: Vec<VeleroRestoreRecord> = restore_api
        .list(&ListParams::default().limit(50))
        .await
        .ok()
        .map(|l| {
            l.items
                .into_iter()
                .filter_map(|o| {
                    let name = o.metadata.name?;
                    let ns = o.metadata.namespace.unwrap_or_else(|| "velero".to_string());
                    let phase = o
                        .data
                        .get("status")
                        .and_then(|st| st.get("phase"))
                        .and_then(|p| p.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let backup_name = o
                        .data
                        .get("spec")
                        .and_then(|sp| sp.get("backupName"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let completion = o
                        .data
                        .get("status")
                        .and_then(|st| st.get("completionTimestamp"))
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    Some(VeleroRestoreRecord {
                        name,
                        namespace: ns,
                        phase,
                        backup_name,
                        completion_timestamp: completion,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let backups_functional = velero_backups_functional(client_for_bsl_check).await;

    Json(VeleroStatusResponse {
        veyron_context: velero_context(velero_available),
        velero_available,
        backups_functional,
        backups,
        restores,
    })
}
