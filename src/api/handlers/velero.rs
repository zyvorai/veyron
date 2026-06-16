// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Velero Backup and Restore discovery.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
    pub velero_available: bool,
    pub backups: Vec<VeleroBackupRecord>,
    pub restores: Vec<VeleroRestoreRecord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VeleroQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/velero/status", get(velero_status))
        .with_state(state)
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

    Json(VeleroStatusResponse {
        veyron_context: velero_context(velero_available),
        velero_available,
        backups,
        restores,
    })
}
