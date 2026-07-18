// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Day-2 compute operations: live CPU/memory hotplug, run-strategy control, and
//! bulk/fleet power+migrate actions over the existing single-VM KubeClient methods.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{post, put},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Deserialize)]
pub struct HotplugRequest {
    /// New CPU socket count (live-hotplugged; also raises `maxSockets`).
    #[serde(default)]
    pub sockets: Option<u32>,
    /// New guest memory, e.g. `"4Gi"` (live-hotplugged; also raises `maxGuest`).
    #[serde(default)]
    pub memory: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunStrategyRequest {
    /// Always | Manual | Halted | RerunOnFailure.
    pub strategy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BulkActionRequest {
    /// start | stop | restart | migrate | delete
    pub action: String,
    /// Namespace of the target VMs (all names must share it).
    pub namespace: String,
    /// Explicit VM names to act on.
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkResult {
    pub name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/vms/:ns/:name/hotplug", post(hotplug_vm))
        .route("/vms/:ns/:name/run-strategy", put(set_run_strategy))
        .route("/vms/:ns/:name/drift/remediate", post(remediate_drift))
        .route("/vms/bulk", post(bulk_action))
        .with_state(state)
}

/// Force the operator to re-reconcile a VeyronVM (desired → actual), which
/// re-applies the declared spec and clears drift. Bumping a well-known
/// annotation is the standard "kick the reconciler" pattern; the operator's
/// converter then repairs the KubeVirt VM. Complements the detect-only
/// `GET /api/v1/vms/:ns/:name/drift`.
#[cfg(feature = "web")]
async fn remediate_drift(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    use kube::Api;
    use kube::api::{Patch, PatchParams};

    let client = { state.read().await.kube_client.client().clone() };
    let api: Api<crate::operator_crds::VeyronVM> = Api::namespaced(client, &ns);
    if api.get(&name).await.is_err() {
        return (
            StatusCode::NOT_FOUND,
            format!("VeyronVM {ns}/{name} not found"),
        )
            .into_response();
    }
    let ts = chrono::Utc::now().to_rfc3339();
    let patch = serde_json::json!({
        "metadata": { "annotations": { "veyron.io/reconcile-requested": ts } }
    });
    match api
        .patch(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true, "vm": name, "namespace": ns,
                "requested_at": ts,
                "note": "operator will re-reconcile desired→actual and clear drift"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("drift remediation failed: {e}"),
        )
            .into_response(),
    }
}

const VALID_RUN_STRATEGIES: [&str; 4] = ["Always", "Manual", "Halted", "RerunOnFailure"];

#[cfg(feature = "web")]
async fn hotplug_vm(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<HotplugRequest>,
) -> impl IntoResponse {
    if req.sockets.is_none() && req.memory.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            "at least one of sockets/memory is required".to_string(),
        )
            .into_response();
    }
    let kube = { state.read().await.kube_client.clone() };
    match kube
        .hotplug_vm_resources(&ns, &name, req.sockets, req.memory.as_deref())
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true, "vm": name, "namespace": ns,
                "sockets": req.sockets, "memory": req.memory,
                "note": "applied live when the cluster runs KubeVirt with the LiveUpdate rollout strategy; otherwise on next restart"
            })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("hotplug failed: {e}")).into_response(),
    }
}

#[cfg(feature = "web")]
async fn set_run_strategy(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<RunStrategyRequest>,
) -> impl IntoResponse {
    if !VALID_RUN_STRATEGIES.contains(&req.strategy.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "invalid run strategy '{}' — must be one of {:?}",
                req.strategy, VALID_RUN_STRATEGIES
            ),
        )
            .into_response();
    }
    let kube = { state.read().await.kube_client.clone() };
    match kube.set_run_strategy(&ns, &name, &req.strategy).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "vm": name, "runStrategy": req.strategy })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("set run strategy failed: {e}"),
        )
            .into_response(),
    }
}

#[cfg(feature = "web")]
async fn bulk_action(
    State(state): State<SharedState>,
    Json(req): Json<BulkActionRequest>,
) -> impl IntoResponse {
    let action = req.action.to_lowercase();
    if !["start", "stop", "restart", "migrate", "delete"].contains(&action.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            format!("invalid action '{}'", req.action),
        )
            .into_response();
    }
    if req.names.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "names must not be empty".to_string(),
        )
            .into_response();
    }
    let kube = { state.read().await.kube_client.clone() };
    let mut results = Vec::with_capacity(req.names.len());
    for name in &req.names {
        let r = match action.as_str() {
            "start" => kube.start_vm(&req.namespace, name).await.map(|_| ()),
            "stop" => kube.stop_vm(&req.namespace, name).await.map(|_| ()),
            "restart" => kube.restart_vm(&req.namespace, name).await.map(|_| ()),
            "delete" => kube.delete_vm(&req.namespace, name).await,
            "migrate" => {
                // Per-VM eligibility: a passthrough-GPU VM in the batch fails
                // with the blocker text instead of silently "succeeding" into
                // a doomed migration. Probe failure never blocks.
                let blocked = match kube
                    .migration_eligibility(
                        &req.namespace,
                        name,
                        crate::kube::migration_guard::ClusterMigrationCaps::from_env(),
                    )
                    .await
                {
                    Ok(elig) if !elig.eligible => Some(
                        elig.blockers
                            .iter()
                            .map(|b| b.message.as_str())
                            .collect::<Vec<_>>()
                            .join("; "),
                    ),
                    _ => None,
                };
                match blocked {
                    Some(msg) => Err(anyhow::anyhow!("migration blocked: {msg}")),
                    None => kube.migrate_vm(&req.namespace, name, None).await,
                }
            }
            _ => unreachable!(),
        };
        results.push(match r {
            Ok(()) => BulkResult {
                name: name.clone(),
                ok: true,
                error: None,
            },
            Err(e) => BulkResult {
                name: name.clone(),
                ok: false,
                error: Some(e.to_string()),
            },
        });
    }
    let ok = results.iter().filter(|r| r.ok).count();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "action": action, "total": results.len(), "succeeded": ok,
            "failed": results.len() - ok, "results": results
        })),
    )
        .into_response()
}
