// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! In-guest Day-2 operations: OS package patching and thin-provisioning reclaim
//! (fstrim), executed through the existing GuestKit `guest-exec` path.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use serde::Deserialize;

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Deserialize)]
pub struct GuestPatchRequest {
    /// Take a snapshot before patching (best-effort; failure aborts the patch).
    #[serde(default)]
    pub snapshot_first: bool,
    /// Override the update command; defaults to a distro-detecting one-shot.
    #[serde(default)]
    pub command: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/vms/:ns/:name/guest/patch", post(guest_patch))
        .route("/vms/:ns/:name/disks/reclaim", post(disk_reclaim))
        .with_state(state)
}

/// Run a `/bin/sh -c "<script>"` in the guest via GuestKit.
#[cfg(feature = "web")]
async fn guest_sh(
    kube: &crate::kube::KubeClient,
    ns: &str,
    name: &str,
    script: &str,
) -> anyhow::Result<serde_json::Value> {
    let params = serde_json::json!({ "path": "/bin/sh", "arg": ["-c", script] });
    kube.guest_exec_via_guestkit(ns, name, params).await
}

#[cfg(feature = "web")]
async fn guest_patch(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<GuestPatchRequest>,
) -> impl IntoResponse {
    let kube = { state.read().await.kube_client.clone() };

    let mut snapshot: Option<String> = None;
    if req.snapshot_first {
        let snap_name = format!(
            "{name}-prepatch-{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        );
        match crate::snapshots::SnapshotManager::new(&ns).await {
            Ok(mgr) => {
                let cfg = crate::snapshots::SnapshotConfig::new(&name, &snap_name)
                    .with_description("pre-patch snapshot");
                if let Err(e) = mgr.create_snapshot(&cfg).await {
                    return (
                        StatusCode::BAD_GATEWAY,
                        format!("pre-patch snapshot failed (patch aborted): {e}"),
                    )
                        .into_response();
                }
                snapshot = Some(snap_name);
            }
            Err(e) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    format!("pre-patch snapshot manager init failed: {e}"),
                )
                    .into_response();
            }
        }
    }

    // An explicit `command` override is a deliberate escape hatch into raw
    // shell exec (`guestkit.exec`) — kept as-is (it's the caller's own
    // opt-in, and may still be blocked by GuestKit policy on some agents).
    // The DEFAULT distro-detecting path previously ran a hardcoded shell
    // script the same way, which made it permanently non-functional
    // wherever GuestKit's default policy disables shell exec ("capability
    // denied: shell exec disabled by policy") — switched to GuestKit's
    // structured packages.updates + packages.install RPCs instead.
    let result = if let Some(script) = req.command.as_deref() {
        guest_sh(&kube, &ns, &name, script).await
    } else {
        match kube.guest_packages_updates(&ns, &name).await {
            Ok(updates) => {
                let packages: Vec<String> = updates
                    .get("packages")
                    .and_then(|p| p.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                if packages.is_empty() {
                    Ok(serde_json::json!({ "message": "no updates available", "updates": updates }))
                } else {
                    kube.guest_packages_install(
                        &ns,
                        &name,
                        serde_json::json!({ "packages": packages }),
                    )
                    .await
                }
            }
            Err(e) => Err(e),
        }
    };
    match result {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true, "vm": name, "namespace": ns,
                "snapshot": snapshot, "result": result
            })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("guest patch failed: {e}")).into_response(),
    }
}

/// Thin-provisioning reclaim: `fstrim -av` in the guest returns freed space to
/// the underlying (thin) storage. Uses GuestKit's structured `storageTrim` RPC
/// — NOT shell exec (`guest_sh`/`guestkit.exec`), which GuestKit's default
/// policy disables ("capability denied: shell exec disabled by policy"),
/// making the previous implementation permanently non-functional.
#[cfg(feature = "web")]
async fn disk_reclaim(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let kube = { state.read().await.kube_client.clone() };
    match kube
        .guest_storage_trim(&ns, &name, serde_json::json!({}))
        .await
    {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "vm": name, "namespace": ns, "result": result })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("fstrim failed: {e}")).into_response(),
    }
}
