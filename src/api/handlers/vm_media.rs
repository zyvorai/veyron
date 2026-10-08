// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Eject install media from a Kairon Machine.
//!
//! `POST /vms/:ns/:name/cdroms/:cdrom/eject` removes the entry from the
//! Machine's `spec.cdroms`. kairon-node then ejects that drive's medium through
//! FluxVM (live when the VM runs) and keeps the empty drive, so an installed VM
//! no longer depends on a host-local ISO and can live-migrate.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use kube::api::{Api, Patch, PatchParams};
use serde_json::{Value, json};

use super::machine_images::{Reply, fail, ok};
use crate::api::http_server::web::SharedState;
use crate::kairon::{Machine, MachineCdrom};

/// Merge patch setting `spec.cdroms` to `remaining`, pinned to the
/// `resourceVersion` it was computed from so a concurrent edit fails with 409
/// instead of being overwritten.
fn eject_patch(resource_version: Option<&str>, remaining: &[MachineCdrom]) -> Value {
    json!({
        "metadata": {"resourceVersion": resource_version},
        "spec": {"cdroms": remaining},
    })
}

async fn eject_cdrom(
    State(state): State<SharedState>,
    Path((ns, vm, cdrom)): Path<(String, String, String)>,
) -> Reply {
    if !crate::api::vm_backend::is_kairon() {
        return Err(fail(
            StatusCode::NOT_IMPLEMENTED,
            "KAIRON_REQUIRED",
            "install media is a Kairon Machine feature; KubeVirt VMs keep their cdrom disks in spec",
        ));
    }
    let client = { state.read().await.client().client() };
    let api = Api::<Machine>::namespaced(client, &ns);
    let internal = |e: kube::Error| fail(StatusCode::BAD_GATEWAY, "EJECT_FAILED", e.to_string());

    let machine = api.get_opt(&vm).await.map_err(internal)?.ok_or_else(|| {
        fail(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("Machine {ns}/{vm} not found"),
        )
    })?;
    if !machine.spec.cdroms.iter().any(|c| c.name == cdrom) {
        return Err(fail(
            StatusCode::NOT_FOUND,
            "CDROM_NOT_FOUND",
            format!(
                "Machine {ns}/{vm} has no cdrom {cdrom:?} in spec.cdroms (it may already be ejected)"
            ),
        ));
    }
    let remaining: Vec<MachineCdrom> = machine
        .spec
        .cdroms
        .iter()
        .filter(|c| c.name != cdrom)
        .cloned()
        .collect();
    let patch = eject_patch(machine.metadata.resource_version.as_deref(), &remaining);
    let updated = api
        .patch(&vm, &PatchParams::default(), &Patch::Merge(&patch))
        .await
        .map_err(|e| match &e {
            kube::Error::Api(ae) if ae.code == 409 => fail(
                StatusCode::CONFLICT,
                "CONFLICT",
                format!("spec.cdroms changed while ejecting; retry: {}", ae.message),
            ),
            kube::Error::Api(ae) if matches!(ae.code, 400 | 403 | 422) => fail(
                StatusCode::UNPROCESSABLE_ENTITY,
                "EJECT_REJECTED",
                ae.message.clone(),
            ),
            _ => internal(e),
        })?;

    let remaining: Vec<&str> = updated
        .spec
        .cdroms
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    ok(
        StatusCode::ACCEPTED,
        json!({
            "namespace": ns,
            "vm": vm,
            "ejected": cdrom,
            "cdroms": remaining,
            "note": "kairon-node ejects the medium on its next reconcile (live when running); the empty drive stays",
        }),
    )
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/vms/:ns/:name/cdroms/:cdrom/eject", post(eject_cdrom))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eject_patch_pins_resource_version_and_keeps_other_entries() {
        let virtio = MachineCdrom {
            name: "virtio".into(),
            image_ref: Some("virtio-win".into()),
            ..Default::default()
        };
        let p = eject_patch(Some("42"), std::slice::from_ref(&virtio));
        assert_eq!(p["metadata"]["resourceVersion"], "42");
        assert_eq!(
            p["spec"]["cdroms"],
            json!([{"name": "virtio", "imageRef": "virtio-win"}])
        );
        assert_eq!(eject_patch(Some("7"), &[])["spec"]["cdroms"], json!([]));
    }
}
