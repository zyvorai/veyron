// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! In-guest Day-2 operations: OS package patching and thin-provisioning reclaim
//! (fstrim), executed through the existing GuestKit `guest-exec` path.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
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
        .route("/vms/:ns/:name/logs", get(vm_logs))
        .with_state(state)
}

/// Distro-detecting one-shot OS update for guests reached over the QEMU guest agent.
const QGA_PATCH_SCRIPT: &str = "if command -v apt-get >/dev/null; then DEBIAN_FRONTEND=noninteractive apt-get update -q && DEBIAN_FRONTEND=noninteractive apt-get -y -q upgrade; elif command -v dnf >/dev/null; then dnf -y -q upgrade; elif command -v yum >/dev/null; then yum -y -q update; elif command -v zypper >/dev/null; then zypper -n -q update; else echo 'no supported package manager' >&2; exit 2; fi";

#[derive(Debug, Clone, Deserialize)]
pub struct LogsQuery {
    #[serde(default)]
    pub tail: Option<u32>,
}

/// Hypervisor-side logs: the FluxVM runtime log on Kairon, the virt-launcher pod's
/// log on KubeVirt.
#[cfg(feature = "web")]
async fn vm_logs(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Query(q): Query<LogsQuery>,
) -> impl IntoResponse {
    let tail = q.tail.unwrap_or(500).min(10_000);
    let kube = { state.read().await.kube_client.clone() };
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::is_kairon() {
        return match crate::api::kairon_ops::runtime_logs(&kube, &ns, &name, tail).await {
            Ok(text) => (StatusCode::OK, text).into_response(),
            Err(e) => (StatusCode::BAD_GATEWAY, format!("runtime logs: {e:#}")).into_response(),
        };
    }
    use k8s_openapi::api::core::v1::Pod;
    use kube::api::{Api, ListParams, LogParams};
    let pods: Api<Pod> = Api::namespaced(kube.client(), &ns);
    let lp = ListParams::default().labels(&format!("{}={name}", crate::kube::VM_NAME_LABEL));
    let pod = match pods.list(&lp).await {
        Ok(list) => list.items.into_iter().find_map(|p| p.metadata.name),
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("list pods: {e}")).into_response(),
    };
    let Some(pod) = pod else {
        return (
            StatusCode::NOT_FOUND,
            format!("VM '{name}' has no virt-launcher pod (is it running?)"),
        )
            .into_response();
    };
    let params = LogParams {
        container: Some("compute".to_string()),
        tail_lines: Some(i64::from(tail)),
        ..Default::default()
    };
    match pods.logs(&pod, &params).await {
        Ok(text) => (StatusCode::OK, text).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("pod logs: {e}")).into_response(),
    }
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
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::is_kairon() {
        return crate::api::kairon_ops::guest_exec(kube, ns, name, &params).await;
    }
    kube.guest_exec_via_guestkit(ns, name, params).await
}

#[cfg(feature = "web")]
async fn pre_patch_snapshot(
    kube: &crate::kube::KubeClient,
    ns: &str,
    name: &str,
    snap_name: &str,
) -> anyhow::Result<()> {
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::is_kairon() {
        return crate::api::kairon_ops::create_snapshot(kube, ns, name, snap_name)
            .await
            .map(|_| ());
    }
    let _ = kube;
    let mgr = crate::snapshots::SnapshotManager::new(ns).await?;
    let cfg = crate::snapshots::SnapshotConfig::new(name, snap_name)
        .with_description("pre-patch snapshot");
    mgr.create_snapshot(&cfg).await.map(|_| ())
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
        if let Err(e) = pre_patch_snapshot(&kube, &ns, &name, &snap_name).await {
            return (
                StatusCode::BAD_GATEWAY,
                format!("pre-patch snapshot failed (patch aborted): {e:#}"),
            )
                .into_response();
        }
        snapshot = Some(snap_name);
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
    } else if crate::api::vm_backend::is_kairon() {
        guest_sh(&kube, &ns, &name, QGA_PATCH_SCRIPT).await
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
    let result = if crate::api::vm_backend::is_kairon() {
        guest_sh(&kube, &ns, &name, "fstrim -av").await
    } else {
        kube.guest_storage_trim(&ns, &name, serde_json::json!({}))
            .await
    };
    match result {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "vm": name, "namespace": ns, "result": result })),
        )
            .into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("fstrim failed: {e}")).into_response(),
    }
}
