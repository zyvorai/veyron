// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Land migrated VMs (h2kvm output: a fixed qcow2 or a raw OVA/VMDK/VHD) on Kairon.
//!
//! The disk is served over HTTP(S); kairon-node downloads it into its digest-keyed cache,
//! converts non-qcow2/raw formats and optionally repairs the guest, then boots a per-Machine
//! overlay. Veyron only builds and tracks the `Machine`.

use std::collections::BTreeMap;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use kube::api::{Api, ListParams, PostParams};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::http_server::web::SharedState;
use crate::kairon::{
    GuestAgentSpec, ImageSource, Machine, MachineImage, MachineResources, MachineSpec, PortForward,
    SecuritySpec,
};

pub const IMPORTED_BY_LABEL: &str = "veyron.io/imported-by";
const FORMATS: [&str; 6] = ["qcow2", "raw", "ova", "vmdk", "vhd", "vhdx"];

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ImportSource {
    /// `vmware`, `hyperv`, `azure`, `kvm`, ...
    #[serde(default)]
    pub hypervisor: Option<String>,
    /// VM name or path on the source platform.
    #[serde(default)]
    pub vm: Option<String>,
    /// Tool that converted the disk (default `h2kvm`).
    #[serde(default)]
    pub tool: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportRequest {
    pub name: String,
    #[serde(default)]
    pub namespace: Option<String>,
    /// `http(s)://` URL kairon-node can reach.
    pub image_url: String,
    /// Hex digest of the file at `image_url` (`sha256:` prefix optional).
    pub sha256: String,
    /// `qcow2` (default), `raw`, `ova`, `vmdk`, `vhd` or `vhdx`.
    #[serde(default)]
    pub format: Option<String>,
    /// Ask FluxVM to repair the guest offline (skip when h2kvm already fixed it).
    #[serde(default)]
    pub repair: bool,
    #[serde(default)]
    pub cpus: Option<u32>,
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(default)]
    pub secure_boot: bool,
    #[serde(default)]
    pub tpm: bool,
    /// Node port forwards (`host_port` → `guest_port`); only settable at creation.
    #[serde(default)]
    pub forwards: Vec<PortForward>,
    #[serde(default = "default_true")]
    pub start: bool,
    #[serde(default)]
    pub source: ImportSource,
}

fn default_true() -> bool {
    true
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

fn normalize_digest(raw: &str) -> Option<String> {
    let hex = raw.trim().strip_prefix("sha256:").unwrap_or(raw.trim());
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| format!("sha256:{}", hex.to_ascii_lowercase()))
}

/// Validate an import request and build its Machine.
pub fn machine_for_import(req: &ImportRequest, namespace: &str) -> Result<Machine, String> {
    if !valid_name(&req.name) {
        return Err(format!("name {:?} must be a DNS-1123 label", req.name));
    }
    let url = req.image_url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("image_url must be an http:// or https:// URL".into());
    }
    let digest = normalize_digest(&req.sha256)
        .ok_or("sha256 must be 64 hex characters, optionally prefixed with sha256:")?;
    let format = req
        .format
        .as_deref()
        .unwrap_or("qcow2")
        .to_ascii_lowercase();
    if !FORMATS.contains(&format.as_str()) {
        return Err(format!("format must be one of {}", FORMATS.join(", ")));
    }
    let cpus = req.cpus.unwrap_or(2);
    if !(1..=256).contains(&cpus) {
        return Err("cpus must be between 1 and 256".into());
    }
    let memory = req.memory.clone().unwrap_or_else(|| "4Gi".into());
    if memory.parse::<u64>().is_ok() || !memory.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(format!("memory {memory:?} must be a quantity such as 4Gi"));
    }

    let mut spec = MachineSpec {
        image: MachineImage {
            digest: Some(digest),
            source: Some(ImageSource {
                http_url: Some(url.to_string()),
                format: Some(format),
                repair: req.repair,
                ..Default::default()
            }),
            ..Default::default()
        },
        resources: MachineResources {
            cpu: cpus.to_string(),
            memory,
            ..Default::default()
        },
        guest_agent: Some(GuestAgentSpec {
            enabled: true,
            console: false,
        }),
        power_state: Some(
            if req.start {
                crate::kairon::POWER_RUNNING
            } else {
                crate::kairon::POWER_STOPPED
            }
            .to_string(),
        ),
        ..Default::default()
    };
    if req.secure_boot || req.tpm {
        spec.security = Some(SecuritySpec {
            secure_boot: req.secure_boot,
            tpm: req.tpm,
        });
    }
    if !req.forwards.is_empty() {
        spec.network = Some(crate::kairon::NetworkSpec {
            forwards: req.forwards.clone(),
            ..Default::default()
        });
    }

    let mut m = Machine::new(&req.name, spec);
    m.metadata.namespace = Some(namespace.to_string());
    let tool = req.source.tool.clone().unwrap_or_else(|| "h2kvm".into());
    m.metadata.labels = Some(BTreeMap::from([(IMPORTED_BY_LABEL.to_string(), tool)]));
    let mut ann = BTreeMap::from([("veyron.io/import-image-url".to_string(), url.to_string())]);
    if let Some(h) = &req.source.hypervisor {
        ann.insert("veyron.io/import-source-hypervisor".into(), h.clone());
    }
    if let Some(v) = &req.source.vm {
        ann.insert("veyron.io/import-source-vm".into(), v.clone());
    }
    m.metadata.annotations = Some(ann);
    Ok(m)
}

fn import_json(m: &Machine) -> Value {
    let ann = m.metadata.annotations.clone().unwrap_or_default();
    let label = |k: &str| ann.get(k).cloned();
    json!({
        "name": m.metadata.name,
        "namespace": m.metadata.namespace,
        "status": m.printable_status(),
        "imported_by": m.metadata.labels.as_ref().and_then(|l| l.get(IMPORTED_BY_LABEL)),
        "image_url": label("veyron.io/import-image-url"),
        "source_hypervisor": label("veyron.io/import-source-hypervisor"),
        "source_vm": label("veyron.io/import-source-vm"),
        "format": m.spec.image.source.as_ref().and_then(|s| s.format.clone()),
        "digest": m.spec.image.digest,
        "created": m.metadata.creation_timestamp.as_ref().map(|t| t.0.to_rfc3339()),
        "conditions": m.status.as_ref().map(|s| &s.conditions),
    })
}

fn fail(status: StatusCode, code: &str, message: impl Into<String>) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({"success": false, "error": {"code": code, "message": message.into()}})),
    )
}

type Reply = Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)>;

fn require_kairon() -> Result<(), (StatusCode, Json<Value>)> {
    if crate::api::vm_backend::is_kairon() {
        Ok(())
    } else {
        Err(fail(
            StatusCode::NOT_IMPLEMENTED,
            "KAIRON_REQUIRED",
            "VM imports land on Kairon Machines; unset VEYRON_VM_BACKEND or set it to kairon",
        ))
    }
}

async fn create_import(State(state): State<SharedState>, Json(req): Json<ImportRequest>) -> Reply {
    require_kairon()?;
    let (client, default_ns) = {
        let s = state.read().await;
        (s.client().client(), s.namespace.clone())
    };
    let ns = req.namespace.clone().unwrap_or(default_ns);
    let m = machine_for_import(&req, &ns)
        .map_err(|e| fail(StatusCode::BAD_REQUEST, "INVALID_REQUEST", e))?;
    match Api::<Machine>::namespaced(client, &ns)
        .create(&PostParams::default(), &m)
        .await
    {
        Ok(created) => Ok((
            StatusCode::CREATED,
            Json(json!({"success": true, "data": import_json(&created)})),
        )),
        Err(kube::Error::Api(e)) if e.code == 409 => Err(fail(
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
            format!("VM {ns}/{} already exists", req.name),
        )),
        Err(e) => Err(fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "IMPORT_FAILED",
            format!("create Machine {ns}/{}: {e}", req.name),
        )),
    }
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default)]
    namespace: Option<String>,
}

async fn list_imports(State(state): State<SharedState>, Query(q): Query<ListQuery>) -> Reply {
    require_kairon()?;
    let (client, default_ns) = {
        let s = state.read().await;
        (s.client().client(), s.namespace.clone())
    };
    let api = match q.namespace.as_deref() {
        Some("all") => Api::<Machine>::all(client),
        other => Api::<Machine>::namespaced(client, other.unwrap_or(&default_ns)),
    };
    let list = api
        .list(&ListParams::default().labels(IMPORTED_BY_LABEL))
        .await
        .map_err(|e| {
            fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_FAILED",
                e.to_string(),
            )
        })?;
    let items: Vec<Value> = list.items.iter().map(import_json).collect();
    Ok((
        StatusCode::OK,
        Json(json!({"success": true, "data": items})),
    ))
}

async fn get_import(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Reply {
    require_kairon()?;
    let client = state.read().await.client().client();
    match Api::<Machine>::namespaced(client, &ns).get_opt(&name).await {
        Ok(Some(m)) => Ok((
            StatusCode::OK,
            Json(json!({"success": true, "data": import_json(&m)})),
        )),
        Ok(None) => Err(fail(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("VM {ns}/{name} not found"),
        )),
        Err(e) => Err(fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "GET_FAILED",
            e.to_string(),
        )),
    }
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/imports", get(list_imports).post(create_import))
        .route("/imports/:ns/:name", get(get_import))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> ImportRequest {
        serde_json::from_value(json!({
            "name": "web01",
            "image_url": "http://10.0.0.5:8099/abc/web01.qcow2",
            "sha256": "AB".repeat(32),
            "cpus": 4,
            "memory": "8Gi",
            "source": {"hypervisor": "vmware", "vm": "dc1/web01"}
        }))
        .unwrap()
    }

    #[test]
    fn builds_a_running_machine_from_an_h2kvm_image() {
        let m = machine_for_import(&req(), "prod").unwrap();
        let src = m.spec.image.source.as_ref().unwrap();
        assert_eq!(
            src.http_url.as_deref(),
            Some("http://10.0.0.5:8099/abc/web01.qcow2")
        );
        assert_eq!(src.format.as_deref(), Some("qcow2"));
        assert!(!src.repair);
        assert_eq!(
            m.spec.image.digest.as_deref(),
            Some(format!("sha256:{}", "ab".repeat(32)).as_str())
        );
        assert_eq!(m.spec.resources.cpu, "4");
        assert_eq!(m.spec.resources.memory, "8Gi");
        assert_eq!(m.spec.power_state.as_deref(), Some("Running"));
        assert!(m.spec.guest_agent.as_ref().unwrap().enabled);
        assert_eq!(
            m.metadata
                .labels
                .unwrap()
                .get(IMPORTED_BY_LABEL)
                .map(String::as_str),
            Some("h2kvm")
        );
        let ann = m.metadata.annotations.unwrap();
        assert_eq!(ann["veyron.io/import-source-hypervisor"], "vmware");
        assert_eq!(ann["veyron.io/import-source-vm"], "dc1/web01");
    }

    #[test]
    fn rejects_bad_input() {
        let mut r = req();
        r.sha256 = "deadbeef".into();
        assert!(
            machine_for_import(&r, "prod")
                .unwrap_err()
                .contains("sha256")
        );
        let mut r = req();
        r.image_url = "file:///tmp/x.qcow2".into();
        assert!(machine_for_import(&r, "prod").is_err());
        let mut r = req();
        r.format = Some("iso".into());
        assert!(machine_for_import(&r, "prod").is_err());
        let mut r = req();
        r.name = "Web_01".into();
        assert!(machine_for_import(&r, "prod").is_err());
        let mut r = req();
        r.memory = Some("4096".into());
        assert!(machine_for_import(&r, "prod").is_err());
    }

    #[test]
    fn stopped_import_keeps_ova_repair_and_forwards() {
        let mut r = req();
        r.start = false;
        r.format = Some("OVA".into());
        r.repair = true;
        r.forwards = vec![PortForward {
            host_port: 30022,
            guest_port: 22,
            protocol: None,
        }];
        let m = machine_for_import(&r, "prod").unwrap();
        assert_eq!(m.spec.power_state.as_deref(), Some("Stopped"));
        let src = m.spec.image.source.unwrap();
        assert_eq!(src.format.as_deref(), Some("ova"));
        assert!(src.repair);
        assert_eq!(m.spec.network.unwrap().forwards[0].guest_port, 22);
    }
}
