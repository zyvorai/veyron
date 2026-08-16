// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

use super::feature_context::VmrogueFeatureContext;

/// GitOps status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsStatus {
    pub veyron_context: VmrogueFeatureContext,
    pub repo_url: String,
    pub branch: String,
    pub last_commit: String,
    pub sync_status: String,
    pub last_synced: Option<String>,
    pub drift_detected: bool,
    pub note: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argo_applications: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flux_kustomizations: Vec<String>,
}

/// GitOps sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsSyncRequest {
    pub force: bool,
    pub dry_run: bool,
    #[serde(default)]
    pub argo_app: Option<String>,
    /// Flux Kustomization name to annotate for reconcile (namespace from query).
    #[serde(default)]
    pub flux_kustomization: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitOpsQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/gitops/status", get(get_gitops_status))
        .route("/gitops/sync", post(trigger_sync))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_dynamic_crs(
    client: kube::Client,
    namespace: &str,
    group: &str,
    version: &str,
    plural: &str,
) -> Vec<String> {
    use kube::api::{ApiResource, DynamicObject, ListParams};

    let ar = ApiResource {
        group: group.to_string(),
        version: version.to_string(),
        api_version: format!("{group}/{version}"),
        kind: plural.to_string(),
        plural: plural.to_string(),
    };
    let api: kube::Api<DynamicObject> = if namespace == "all" {
        kube::Api::all_with(client, &ar)
    } else {
        kube::Api::namespaced_with(client, namespace, &ar)
    };
    match api.list(&ListParams::default().limit(50)).await {
        Ok(list) => list
            .items
            .iter()
            .filter_map(|o| o.metadata.name.clone())
            .collect(),
        Err(_) => vec![],
    }
}

#[cfg(feature = "web")]
async fn argo_app_status(
    client: kube::Client,
    namespace: &str,
) -> (Vec<String>, Option<String>, Option<String>) {
    use kube::api::{ApiResource, DynamicObject, ListParams};

    let ar = ApiResource {
        group: "argoproj.io".into(),
        version: "v1alpha1".into(),
        api_version: "argoproj.io/v1alpha1".into(),
        kind: "Application".into(),
        plural: "applications".into(),
    };
    let api: kube::Api<DynamicObject> = if namespace == "all" {
        kube::Api::all_with(client, &ar)
    } else {
        kube::Api::namespaced_with(client, namespace, &ar)
    };
    let Ok(list) = api.list(&ListParams::default().limit(50)).await else {
        return (vec![], None, None);
    };
    let names: Vec<String> = list
        .items
        .iter()
        .filter_map(|o| o.metadata.name.clone())
        .collect();
    let first = list.items.first();
    let sync = first.and_then(|o| {
        o.data
            .get("status")
            .and_then(|s| s.get("sync"))
            .and_then(|s| s.get("status"))
            .and_then(|v| v.as_str())
            .map(String::from)
    });
    let health = first.and_then(|o| {
        o.data
            .get("status")
            .and_then(|s| s.get("health"))
            .and_then(|h| h.get("status"))
            .and_then(|v| v.as_str())
            .map(String::from)
    });
    (names, sync, health)
}

#[cfg(feature = "web")]
async fn get_gitops_status(
    State(state): State<SharedState>,
    Query(query): Query<GitOpsQuery>,
) -> Json<GitOpsStatus> {
    use k8s_openapi::api::core::v1::ConfigMap;

    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let client = s.client().client();

    let api: kube::api::Api<ConfigMap> = kube::api::Api::namespaced(client.clone(), &namespace);
    let params = kube::api::ListParams::default().labels("veyron.io/type=gitops");

    let (repo_url, branch, last_commit, last_synced, stored_vm_count) =
        if let Ok(cms) = api.list(&params).await {
            cms.items
                .first()
                .and_then(|cm| {
                    let data = cm.data.as_ref()?;
                    Some((
                        data.get("repo_url").cloned().unwrap_or_default(),
                        data.get("branch")
                            .cloned()
                            .unwrap_or_else(|| "main".to_string()),
                        data.get("last_commit").cloned().unwrap_or_default(),
                        data.get("last_synced").cloned(),
                        data.get("vm_count").and_then(|v| v.parse::<usize>().ok()),
                    ))
                })
                .unwrap_or_default()
        } else {
            Default::default()
        };

    let vms_list_result = if namespace == "all" {
        s.client().list_all_vms().await
    } else {
        s.client().list_vms(&namespace).await
    };

    // A failed list call must not be treated as "zero VMs" for drift purposes —
    // that would flip drift_detected on an API hiccup, a false "out_of_sync"
    // alarm unrelated to real GitOps drift. Only compare when the list actually
    // succeeded; an unreadable count (list failed, or nothing stored yet) means
    // "can't tell," not "drifted."
    let drift_detected = match (&vms_list_result, stored_vm_count) {
        (Ok(vms), Some(count)) => count != vms.len(),
        _ => false,
    };

    let (argo_apps, argo_sync, _argo_health) = argo_app_status(client.clone(), &namespace).await;
    let flux_kusts = list_dynamic_crs(
        client.clone(),
        &namespace,
        "kustomize.toolkit.fluxcd.io",
        "v1",
        "kustomizations",
    )
    .await;

    let has_controllers = !argo_apps.is_empty() || !flux_kusts.is_empty();
    let ctx = if has_controllers {
        VmrogueFeatureContext::gitops_controllers()
    } else {
        VmrogueFeatureContext::gitops_status()
    };

    let sync_status = if let Some(ref st) = argo_sync {
        st.to_lowercase()
    } else if repo_url.is_empty() && !has_controllers {
        "not_configured".to_string()
    } else if drift_detected {
        "out_of_sync".to_string()
    } else if last_synced.is_some() {
        "synced".to_string()
    } else {
        "pending".to_string()
    };

    let note = if has_controllers {
        format!(
            "Argo CD apps: {}. Flux kustomizations: {}. POST /gitops/sync can trigger Argo CD when VEYRON_ARGOCD_URL is set.",
            if argo_apps.is_empty() {
                "none".into()
            } else {
                argo_apps.join(", ")
            },
            if flux_kusts.is_empty() {
                "none".into()
            } else {
                flux_kusts.join(", ")
            }
        )
    } else {
        "Derived from Veyron GitOps ConfigMaps and live VM counts.".to_string()
    };

    Json(GitOpsStatus {
        veyron_context: ctx,
        repo_url,
        branch,
        last_commit,
        sync_status,
        last_synced,
        drift_detected,
        note,
        argo_applications: argo_apps,
        flux_kustomizations: flux_kusts,
    })
}

#[cfg(feature = "web")]
async fn trigger_argocd_sync(app: &str) -> bool {
    let base = match crate::api::integrations::env_var("VEYRON_ARGOCD_URL") {
        Some(u) => u,
        None => return false,
    };
    let token = match crate::api::integrations::env_var("VEYRON_ARGOCD_TOKEN") {
        Some(t) => t,
        None => return false,
    };
    let url = format!(
        "{}/api/v1/applications/{}/sync",
        base.trim_end_matches('/'),
        app
    );
    let body = serde_json::json!({ "prune": false });
    crate::api::integrations::post_json(&url, &body, Some(&token))
        .await
        .unwrap_or(false)
}

#[cfg(feature = "web")]
async fn trigger_flux_reconcile(
    client: kube::Client,
    namespace: &str,
    kustomization: &str,
) -> bool {
    use kube::api::{ApiResource, DynamicObject, Patch, PatchParams};

    let ar = ApiResource {
        group: "kustomize.toolkit.fluxcd.io".into(),
        version: "v1".into(),
        api_version: "kustomize.toolkit.fluxcd.io/v1".into(),
        kind: "Kustomization".into(),
        plural: "kustomizations".into(),
    };
    let api: kube::Api<DynamicObject> = kube::Api::namespaced_with(client, namespace, &ar);
    let now = chrono::Utc::now().to_rfc3339();
    let patch = serde_json::json!({
        "metadata": {
            "annotations": {
                "reconcile.fluxcd.io/requestedAt": now,
                "reconcile.fluxcd.io/forceAt": now
            }
        }
    });
    api.patch(kustomization, &PatchParams::default(), &Patch::Merge(patch))
        .await
        .is_ok()
}

#[cfg(feature = "web")]
async fn trigger_sync(
    State(state): State<SharedState>,
    Query(query): Query<GitOpsQuery>,
    Json(req): Json<GitOpsSyncRequest>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    use k8s_openapi::api::core::v1::ConfigMap;

    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };
    let now = chrono::Utc::now().to_rfc3339();

    let mut argo_triggered = false;
    let mut flux_triggered = false;
    if !req.dry_run {
        if let Some(ref app) = req.argo_app {
            argo_triggered = trigger_argocd_sync(app).await;
        } else if let Some(app) = crate::api::integrations::env_var("VEYRON_ARGOCD_DEFAULT_APP") {
            argo_triggered = trigger_argocd_sync(&app).await;
        }

        if let Some(ref kust) = req.flux_kustomization {
            flux_triggered = trigger_flux_reconcile(s.client().client(), &namespace, kust).await;
        } else if let Some(kust) =
            crate::api::integrations::env_var("VEYRON_FLUX_DEFAULT_KUSTOMIZATION")
        {
            flux_triggered = trigger_flux_reconcile(s.client().client(), &namespace, &kust).await;
        }

        let api: kube::api::Api<ConfigMap> =
            kube::api::Api::namespaced(s.client().client(), &namespace);
        let params = kube::api::ListParams::default().labels("veyron.io/type=gitops");
        if let Ok(cms) = api.list(&params).await {
            if let Some(cm) = cms.items.first() {
                if let Some(name) = cm.metadata.name.as_deref() {
                    let patch = serde_json::json!({
                        "data": {
                            "last_synced": now,
                            "vm_count": vms.len().to_string(),
                        }
                    });
                    let _ = api
                        .patch(
                            name,
                            &kube::api::PatchParams::default(),
                            &kube::api::Patch::Merge(patch),
                        )
                        .await;
                }
            }
        }
    }

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "status": if req.dry_run { "dry_run" } else { "synced" },
            "namespace": namespace,
            "vms_in_cluster": vms.len(),
            "synced_at": if req.dry_run { serde_json::Value::Null } else { serde_json::Value::String(now) },
            "dry_run": req.dry_run,
            "force": req.force,
            "argo_sync_triggered": argo_triggered,
            "flux_reconcile_triggered": flux_triggered,
            "veyron_context": serde_json::to_value(VmrogueFeatureContext::gitops_controllers()).unwrap_or(serde_json::Value::Null),
        })),
    )
}
