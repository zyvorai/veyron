// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Fork Lab: turn one Running Kairon Machine into N live copies.
//!
//! `POST /vms/:ns/:name/fork` creates ordinary Machines carrying the
//! `kairon.zyvor.dev/fork-from` annotation; kairon-node then asks FluxVM to fork
//! the parent's runtime instead of booting a new VM, so each child starts with
//! the parent's memory and CPU state on a copy-on-write disk.
//! `GET /vms/:ns/:name/forks` lists the children. Children are deleted with the
//! normal VM delete.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use kube::api::{Api, ListParams, PostParams};
use serde::Deserialize;
use serde_json::{Value, json};

use super::machine_images::{Reply, fail, ok, require_kairon};
use crate::api::http_server::web::SharedState;
use crate::kairon::{
    ANNOTATION_FORK_FROM, LABEL_FORKED_FROM, LABEL_MANAGED_BY, Machine, POWER_RUNNING,
};

/// Kairon refuses more than 32 children per call; Veyron's own cap is lower.
const HARD_MAX: u32 = 32;
const DEFAULT_MAX: u32 = 10;

#[derive(Debug, Deserialize)]
struct ForkRequest {
    #[serde(default = "one")]
    count: u32,
    /// Child name prefix; defaults to the parent's name. Children are `<prefix>-<n>`.
    #[serde(default)]
    name_prefix: Option<String>,
}

fn one() -> u32 {
    1
}

fn max_children() -> u32 {
    std::env::var("VEYRON_FORK_MAX")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| *n > 0)
        .map(|n| n.min(HARD_MAX))
        .unwrap_or(DEFAULT_MAX)
}

/// Why `parent` cannot be forked, mirroring kairon's documented limits.
fn fork_blocker(parent: &Machine) -> Option<String> {
    if !parent.is_running() {
        return Some(format!(
            "Machine '{}' is not Running (phase {}); only a running VM can be forked",
            parent.name(),
            parent.phase()
        ));
    }
    if !parent.spec.volumes.is_empty()
        || !parent.spec.disks.is_empty()
        || !parent.spec.device_claims.is_empty()
    {
        return Some(format!(
            "Machine '{}' has volumes, disks or device claims; only the boot disk is copied, so it cannot be forked",
            parent.name()
        ));
    }
    if let Some(b) = parent
        .spec
        .runtime
        .as_ref()
        .and_then(|r| r.backend.as_deref())
        .filter(|b| matches!(*b, "qemu" | "cloud-hypervisor"))
    {
        return Some(format!(
            "Machine '{}' runs on the {b} backend; only flux-vm (or sandbox) Machines can be forked",
            parent.name()
        ));
    }
    if parent.node().is_none() {
        return Some(format!(
            "Machine '{}' has no node yet; children are pinned to the parent's node",
            parent.name()
        ));
    }
    None
}

/// Child Machine for `parent`: copy of the spec pinned to the parent's node, with
/// the fork annotation/label. The parent's own labels are deliberately not copied.
fn child_machine(parent: &Machine, child: &str) -> Machine {
    let mut spec = parent.spec.clone();
    spec.node_name = parent.node().map(str::to_string);
    spec.power_state = Some(POWER_RUNNING.to_string());
    let mut m = Machine::new(child, spec);
    m.metadata.namespace = Some(parent.namespace().to_string());
    m.metadata.annotations =
        Some([(ANNOTATION_FORK_FROM.to_string(), parent.name().to_string())].into());
    m.metadata.labels = Some(
        [
            (LABEL_FORKED_FROM.to_string(), parent.name().to_string()),
            (LABEL_MANAGED_BY.to_string(), "veyron".to_string()),
        ]
        .into(),
    );
    m
}

fn child_json(m: &Machine) -> Value {
    json!({
        "name": m.name(),
        "namespace": m.namespace(),
        "phase": m.phase(),
        "node": m.node(),
        "ips": m.guest_ips(),
        "message": m.message(),
    })
}

fn existing_children(parent: &str, items: &[Machine]) -> usize {
    items
        .iter()
        .filter(|m| {
            m.metadata
                .labels
                .as_ref()
                .and_then(|l| l.get(LABEL_FORKED_FROM))
                .is_some_and(|p| p == parent)
        })
        .count()
}

async fn fork_vm(
    State(state): State<SharedState>,
    Path((ns, vm)): Path<(String, String)>,
    Json(req): Json<ForkRequest>,
) -> Reply {
    require_kairon()?;
    let cap = max_children();
    if req.count == 0 || req.count > cap {
        return Err(fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            "INVALID_COUNT",
            format!("count must be between 1 and {cap}"),
        ));
    }
    let prefix = req.name_prefix.clone().unwrap_or_else(|| vm.clone());
    if prefix.is_empty()
        || prefix.len() > 40
        || !prefix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            "INVALID_NAME_PREFIX",
            "name_prefix must be lowercase letters, digits and '-', at most 40 characters",
        ));
    }
    let client = { state.read().await.client().client() };
    let api = Api::<Machine>::namespaced(client, &ns);
    let upstream = |e: kube::Error| fail(StatusCode::BAD_GATEWAY, "FORK_FAILED", e.to_string());

    let parent = api.get_opt(&vm).await.map_err(upstream)?.ok_or_else(|| {
        fail(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("Machine {ns}/{vm} not found"),
        )
    })?;
    if let Some(why) = fork_blocker(&parent) {
        return Err(fail(StatusCode::CONFLICT, "FORK_BLOCKED", why));
    }
    let all = api.list(&ListParams::default()).await.map_err(upstream)?;
    let have = existing_children(&vm, &all.items) as u32;
    if have + req.count > cap {
        return Err(fail(
            StatusCode::CONFLICT,
            "FORK_LIMIT",
            format!("{vm} already has {have} fork(s); the limit is {cap} per VM (VEYRON_FORK_MAX)"),
        ));
    }
    let taken: std::collections::HashSet<&str> = all.items.iter().map(|m| m.name()).collect();
    let mut created = Vec::new();
    let mut n = 1u32;
    while (created.len() as u32) < req.count {
        let name = format!("{prefix}-{n}");
        n += 1;
        if taken.contains(name.as_str()) {
            continue;
        }
        let child = child_machine(&parent, &name);
        match api.create(&PostParams::default(), &child).await {
            Ok(m) => created.push(m),
            Err(kube::Error::Api(ae)) if ae.code == 409 => continue,
            Err(e) => {
                return Err(fail(
                    StatusCode::BAD_GATEWAY,
                    "FORK_FAILED",
                    format!("created {} of {}: {e}", created.len(), req.count),
                ));
            }
        }
    }
    ok(
        StatusCode::ACCEPTED,
        json!({
            "parent": vm,
            "namespace": ns,
            "children": created.iter().map(child_json).collect::<Vec<_>>(),
            "note": "kairon-node forks the parent's runtime for each child on its next reconcile",
        }),
    )
}

async fn list_forks(
    State(state): State<SharedState>,
    Path((ns, vm)): Path<(String, String)>,
) -> Reply {
    require_kairon()?;
    let client = { state.read().await.client().client() };
    let api = Api::<Machine>::namespaced(client, &ns);
    let lp = ListParams::default().labels(&format!("{LABEL_FORKED_FROM}={vm}"));
    let items = api
        .list(&lp)
        .await
        .map_err(|e| fail(StatusCode::BAD_GATEWAY, "FORK_LIST_FAILED", e.to_string()))?;
    ok(
        StatusCode::OK,
        json!({
            "parent": vm,
            "namespace": ns,
            "forks": items.items.iter().map(child_json).collect::<Vec<_>>(),
        }),
    )
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/vms/:ns/:name/fork", post(fork_vm))
        .route("/vms/:ns/:name/forks", get(list_forks))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kairon::{MachineSpec, MachineStatus};

    fn running(name: &str) -> Machine {
        let mut m = Machine::new(name, MachineSpec::default());
        m.metadata.namespace = Some("default".into());
        m.metadata.labels = Some([("team".to_string(), "a".to_string())].into());
        m.status = Some(MachineStatus {
            phase: Some("Running".into()),
            node_name: Some("node-1".into()),
            ..Default::default()
        });
        m
    }

    #[test]
    fn running_plain_machine_is_forkable() {
        assert!(fork_blocker(&running("base")).is_none());
    }

    #[test]
    fn stopped_machine_is_blocked() {
        let mut m = running("base");
        m.status.as_mut().unwrap().phase = Some("Stopped".into());
        assert!(fork_blocker(&m).unwrap().contains("not Running"));
    }

    #[test]
    fn child_is_pinned_labelled_and_drops_parent_labels() {
        let c = child_machine(&running("base"), "trial-1");
        assert_eq!(c.spec.node_name.as_deref(), Some("node-1"));
        let labels = c.metadata.labels.unwrap();
        assert_eq!(labels.get(LABEL_FORKED_FROM).unwrap(), "base");
        assert!(!labels.contains_key("team"));
        assert_eq!(
            c.metadata
                .annotations
                .unwrap()
                .get(ANNOTATION_FORK_FROM)
                .unwrap(),
            "base"
        );
    }

    #[test]
    fn qemu_backend_is_blocked() {
        let mut m = running("base");
        m.spec.runtime = Some(crate::kairon::RuntimeSpec {
            backend: Some("qemu".into()),
            kernel: None,
        });
        assert!(fork_blocker(&m).unwrap().contains("qemu"));
    }

    #[test]
    fn counts_only_this_parents_children() {
        let a = child_machine(&running("base"), "base-1");
        let b = child_machine(&running("other"), "other-1");
        assert_eq!(existing_children("base", &[a, b]), 1);
    }
}
