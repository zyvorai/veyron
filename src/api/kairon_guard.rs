// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Refuse KubeVirt/CDI/operator-only routes with `501` when Kairon runs the VMs, so a
//! Kairon cluster never quietly creates KubeVirt objects or reports KubeVirt state.

use axum::{
    Json,
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// The feature a route belongs to when it only exists on KubeVirt, else `None`.
pub fn kubevirt_only(method: &Method, path: &str) -> Option<&'static str> {
    let rest = path
        .strip_prefix("/api/v1/veyron")
        .or_else(|| path.strip_prefix("/api/v1"))?;
    let seg: Vec<&str> = rest.trim_matches('/').split('/').collect();
    let get = method == Method::GET;
    match seg.as_slice() {
        ["images", "import" | "upload" | "publish"] if !get => Some("CDI image import"),
        ["images", "datasources"] => Some("CDI DataSources"),
        ["images", _, _] if method == Method::DELETE => Some("CDI DataVolumes"),
        ["kubevirt", ..] => Some("KubeVirt configuration"),
        ["vmis", ..] => Some("KubeVirt VirtualMachineInstances"),
        ["dr", ..] => Some("KubeVirt DR export/failover"),
        ["disks", "expand"] => Some("DataVolume expansion"),
        ["vms", _, _, "disks", _, "migrate"] => Some("KubeVirt volume migration"),
        ["vms", _, _, "drift", ..] => Some("operator drift"),
        ["migrations"] | ["migrations", _] => Some("cluster-wide KubeVirt migrations"),
        ["crds", "veyronvms" | "blueprints", ..] => Some("Veyron operator CRDs"),
        ["clones"] => Some("KubeVirt VirtualMachineClone"),
        ["snapshots", _, "restore"] => Some("KubeVirt snapshot restore"),
        ["snapshots", _] if method == Method::DELETE => Some("KubeVirt snapshots"),
        ["platform", "upgrade"] => Some("KubeVirt/CDI upgrade"),
        _ => None,
    }
}

pub async fn middleware(req: Request, next: Next) -> Response {
    if crate::api::vm_backend::is_kairon() {
        if let Some(feature) = kubevirt_only(req.method(), req.uri().path()) {
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(serde_json::json!({
                    "success": false,
                    "error": {
                        "code": "KUBEVIRT_ONLY",
                        "message": format!(
                            "{feature} is KubeVirt-only; this Veyron runs VMs on Kairon. \
                             Use the per-VM /api/v1/vms routes instead."
                        ),
                    },
                })),
            )
                .into_response();
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_kubevirt_only_routes() {
        for (m, p) in [
            (Method::POST, "/api/v1/images/upload"),
            (Method::GET, "/api/v1/images/datasources"),
            (Method::DELETE, "/api/v1/images/default/dv"),
            (Method::GET, "/api/v1/vmis"),
            (Method::GET, "/api/v1/kubevirt/migration-policies"),
            (Method::POST, "/api/v1/dr/failover"),
            (Method::POST, "/api/v1/vms/default/web/disks/root/migrate"),
            (Method::GET, "/api/v1/vms/default/web/drift"),
            (Method::GET, "/api/v1/migrations"),
            (Method::POST, "/api/v1/crds/veyronvms"),
            (Method::POST, "/api/v1/clones"),
            (Method::POST, "/api/v1/snapshots/abc/restore"),
            (Method::POST, "/api/v1/veyron/platform/upgrade"),
        ] {
            assert!(kubevirt_only(&m, p).is_some(), "{m} {p}");
        }
    }

    #[test]
    fn keeps_backend_neutral_routes() {
        for (m, p) in [
            (Method::GET, "/api/v1/vms"),
            (Method::POST, "/api/v1/vms/default/web/start"),
            (Method::GET, "/api/v1/vms/default/web/migrations"),
            (Method::POST, "/api/v1/vms/default/web/migrate"),
            (Method::POST, "/api/v1/snapshots/default/web/create"),
            (Method::POST, "/api/v1/snapshots/default/snap/restore"),
            (Method::GET, "/api/v1/images/catalog"),
            (Method::GET, "/api/v1/snapshots/default"),
            (Method::POST, "/api/v1/imports"),
            (Method::GET, "/console"),
        ] {
            assert!(kubevirt_only(&m, p).is_none(), "{m} {p}");
        }
    }
}
