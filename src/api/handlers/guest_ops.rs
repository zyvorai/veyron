// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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

/// Distro-detecting package update: apt / dnf / yum / zypper / apk.
const DEFAULT_PATCH_CMD: &str = "set -e; \
if command -v apt-get >/dev/null 2>&1; then export DEBIAN_FRONTEND=noninteractive; apt-get update && apt-get -y upgrade; \
elif command -v dnf >/dev/null 2>&1; then dnf -y upgrade; \
elif command -v yum >/dev/null 2>&1; then yum -y update; \
elif command -v zypper >/dev/null 2>&1; then zypper -n update; \
elif command -v apk >/dev/null 2>&1; then apk update && apk upgrade; \
else echo 'no supported package manager found' >&2; exit 3; fi";

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

    let script = req.command.as_deref().unwrap_or(DEFAULT_PATCH_CMD);
    match guest_sh(&kube, &ns, &name, script).await {
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
/// the underlying (thin) storage.
#[cfg(feature = "web")]
async fn disk_reclaim(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let kube = { state.read().await.kube_client.clone() };
    match guest_sh(&kube, &ns, &name, "fstrim -av 2>&1 || /sbin/fstrim -av").await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "vm": name, "namespace": ns, "result": result })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("fstrim failed: {e}")).into_response(),
    }
}
