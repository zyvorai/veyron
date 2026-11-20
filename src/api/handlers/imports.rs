// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Land migrated VMs (h2kvm output: a fixed qcow2 or a raw OVA/VMDK/VHD) on Kairon.
//!
//! The disk is served over HTTP(S); kairon-node downloads it into its digest-keyed cache,
//! converts non-qcow2/raw formats and optionally repairs the guest, then boots a per-Machine
//! overlay. Veyron only builds and tracks the `Machine`.

use std::collections::BTreeMap;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use kube::api::{Api, ListParams, PostParams};
use serde::Deserialize;
use serde_json::{Value, json};

use super::import_events::{
    AuditRecord, ClusterHooks, ImportHooks, delete_with_hooks, initial_annotations, observe,
    retry_with_hooks,
};
use super::import_lifecycle::{
    ApiErr, IMPORT_ID_ANNOTATION, ImportPhase, KubeStore, import_id, new_import_id, phase_of,
    retries,
};
use super::import_mappings::{
    KubeMappingStore, MAPPING_ANNOTATION, MappingStore, SOURCE_DATASTORE_ANNOTATION,
    SOURCE_NETWORK_ANNOTATION, STORAGE_POOL_ANNOTATION, apply_mapping, resolve as resolve_mapping,
};
use super::import_validate::{ClusterFacts, HttpProbe, ValidateRequest, run_validate_with_mapping};
use crate::api::auth_context::AuthContext;
use crate::api::http_server::web::SharedState;
use crate::kairon::{
    GuestAgentSpec, ImageSource, Machine, MachineImage, MachineResources, MachineSpec, PortForward,
    SecuritySpec,
};

pub const IMPORTED_BY_LABEL: &str = "veyron.io/imported-by";
pub(crate) const FORMATS: [&str; 6] = ["qcow2", "raw", "ova", "vmdk", "vhd", "vhdx"];

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
    /// Source network name (vSphere port group, Hyper-V switch); resolved by `mapping`.
    #[serde(default)]
    pub network: Option<String>,
    /// Source datastore name; resolved by `mapping`.
    #[serde(default)]
    pub datastore: Option<String>,
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
    /// Name of an import mapping profile (`/import-mappings`) that turns `source.network` and
    /// `source.datastore` into a Kairon network attachment and storage pool.
    #[serde(default)]
    pub mapping: Option<String>,
}

fn default_true() -> bool {
    true
}

pub(crate) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

pub(crate) fn normalize_digest(raw: &str) -> Option<String> {
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
    let mut ann = BTreeMap::from([
        ("veyron.io/import-image-url".to_string(), url.to_string()),
        (IMPORT_ID_ANNOTATION.to_string(), new_import_id()),
    ]);
    if let Some(h) = &req.source.hypervisor {
        ann.insert("veyron.io/import-source-hypervisor".into(), h.clone());
    }
    if let Some(v) = &req.source.vm {
        ann.insert("veyron.io/import-source-vm".into(), v.clone());
    }
    if let Some(n) = &req.source.network {
        ann.insert(SOURCE_NETWORK_ANNOTATION.into(), n.clone());
    }
    if let Some(d) = &req.source.datastore {
        ann.insert(SOURCE_DATASTORE_ANNOTATION.into(), d.clone());
    }
    ann.extend(initial_annotations());
    m.metadata.annotations = Some(ann);
    Ok(m)
}

/// `machine_for_import` plus the named mapping profile, when the request has one.
pub async fn build_import<M: MappingStore>(
    mappings: &M,
    req: &ImportRequest,
    namespace: &str,
) -> Result<Machine, ApiErr> {
    let mut m = machine_for_import(req, namespace).map_err(|e| ApiErr {
        status: StatusCode::BAD_REQUEST,
        code: "INVALID_REQUEST",
        message: e,
    })?;
    if let Some(name) = req.mapping.as_deref() {
        let mapping = mappings
            .get(name)
            .await
            .map_err(|e| ApiErr {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "STORE_FAILED",
                message: e,
            })?
            .ok_or_else(|| ApiErr {
                status: StatusCode::BAD_REQUEST,
                code: "MAPPING_NOT_FOUND",
                message: format!("mapping {name:?} does not exist"),
            })?;
        let resolved = resolve_mapping(
            &mapping,
            req.source.network.as_deref(),
            req.source.datastore.as_deref(),
        )
        .map_err(|e| ApiErr {
            status: StatusCode::BAD_REQUEST,
            code: "UNMAPPED_SOURCE",
            message: e,
        })?;
        apply_mapping(&mut m, &resolved);
    }
    Ok(m)
}

fn import_json(m: &Machine) -> Value {
    let ann = m.metadata.annotations.clone().unwrap_or_default();
    let label = |k: &str| ann.get(k).cloned();
    let phase = phase_of(m);
    json!({
        "import_id": import_id(m),
        "phase": phase,
        "terminal": phase.is_terminal(),
        "retryable": phase == ImportPhase::Failed,
        "retries": retries(m),
        "message": m.message(),
        "name": m.metadata.name,
        "namespace": m.metadata.namespace,
        "status": m.printable_status(),
        "imported_by": m.metadata.labels.as_ref().and_then(|l| l.get(IMPORTED_BY_LABEL)),
        "image_url": label("veyron.io/import-image-url"),
        "source_hypervisor": label("veyron.io/import-source-hypervisor"),
        "source_vm": label("veyron.io/import-source-vm"),
        "format": m.spec.image.source.as_ref().and_then(|s| s.format.clone()),
        "digest": m.spec.image.digest,
        "mapping": label(MAPPING_ANNOTATION),
        "source_network": label(SOURCE_NETWORK_ANNOTATION),
        "source_datastore": label(SOURCE_DATASTORE_ANNOTATION),
        "storage_pool": label(STORAGE_POOL_ANNOTATION),
        "network": m.spec.network.as_ref().map(|n| json!({"mode": n.mode, "bridge": n.bridge})),
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

async fn create_import(
    State(state): State<SharedState>,
    auth: Option<Extension<AuthContext>>,
    Json(req): Json<ImportRequest>,
) -> Reply {
    require_kairon()?;
    let (client, default_ns) = {
        let s = state.read().await;
        (s.client().client(), s.namespace.clone())
    };
    let ns = req.namespace.clone().unwrap_or_else(|| default_ns.clone());
    let hooks = ClusterHooks {
        client: client.clone(),
        namespace: default_ns.clone(),
    };
    let audit = |outcome: &'static str, id: Option<String>, detail: Option<String>| AuditRecord {
        actor: actor(&auth),
        action: "create",
        namespace: ns.clone(),
        name: req.name.clone(),
        import_id: id,
        outcome,
        detail,
    };
    let mappings = KubeMappingStore {
        client: client.clone(),
        namespace: default_ns.clone(),
    };
    let m = match build_import(&mappings, &req, &ns).await {
        Ok(m) => m,
        Err(e) => {
            hooks
                .audit(audit("failure", None, Some(e.code.into())))
                .await;
            return Err(api_fail(e));
        }
    };
    let id = import_id(&m);
    match Api::<Machine>::namespaced(client, &ns)
        .create(&PostParams::default(), &m)
        .await
    {
        Ok(created) => {
            hooks.audit(audit("success", id, None)).await;
            Ok((
                StatusCode::CREATED,
                Json(json!({"success": true, "data": import_json(&created)})),
            ))
        }
        Err(kube::Error::Api(e)) if e.code == 409 => {
            hooks
                .audit(audit("failure", None, Some("ALREADY_EXISTS".into())))
                .await;
            Err(fail(
                StatusCode::CONFLICT,
                "ALREADY_EXISTS",
                format!("VM {ns}/{} already exists", req.name),
            ))
        }
        Err(e) => {
            hooks
                .audit(audit("failure", id, Some("IMPORT_FAILED".into())))
                .await;
            Err(fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "IMPORT_FAILED",
                format!("create Machine {ns}/{}: {e}", req.name),
            ))
        }
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
    let client_for_observe = client.clone();
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
    let store = KubeStore {
        client: client_for_observe,
    };
    let hooks = ClusterHooks {
        client: store.client.clone(),
        namespace: default_ns.clone(),
    };
    for m in &list.items {
        observe(&store, &hooks, m).await;
    }
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
    let (store, hooks) = runtime(&state).await;
    match Api::<Machine>::namespaced(store.client.clone(), &ns)
        .get_opt(&name)
        .await
    {
        Ok(Some(m)) => {
            // A phase change since the last read fires `import.phase_changed`.
            observe(&store, &hooks, &m).await;
            Ok((
                StatusCode::OK,
                Json(json!({"success": true, "data": import_json(&m)})),
            ))
        }
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

fn actor(auth: &Option<Extension<AuthContext>>) -> String {
    auth.as_ref()
        .map(|Extension(a)| a.subject.clone())
        .unwrap_or_else(|| "unknown".into())
}

/// Store and hooks sharing one client; hooks use Veyron's own namespace for webhooks and audit.
async fn runtime(state: &SharedState) -> (KubeStore, ClusterHooks) {
    let s = state.read().await;
    let client = s.client().client();
    (
        KubeStore {
            client: client.clone(),
        },
        ClusterHooks {
            client,
            namespace: s.namespace.clone(),
        },
    )
}

fn api_fail(e: ApiErr) -> (StatusCode, Json<Value>) {
    fail(e.status, e.code, e.message)
}

#[derive(Debug, Deserialize)]
struct RetryQuery {
    #[serde(default)]
    force: bool,
}

/// `DELETE /imports/:ns/:name`: delete an imported VM (404 `NOT_AN_IMPORT` for any other VM).
async fn delete_import_route(
    State(state): State<SharedState>,
    auth: Option<Extension<AuthContext>>,
    Path((ns, name)): Path<(String, String)>,
) -> Reply {
    require_kairon()?;
    let (store, hooks) = runtime(&state).await;
    let out = delete_with_hooks(&store, &hooks, &actor(&auth), &ns, &name)
        .await
        .map_err(api_fail)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"success": true, "data": {
            "name": name, "namespace": ns,
            "import_id": out.import_id, "phase": out.phase,
        }})),
    ))
}

/// `POST /imports/:ns/:name/retry`: recreate a failed import from its own spec.
async fn retry_import_route(
    State(state): State<SharedState>,
    auth: Option<Extension<AuthContext>>,
    Path((ns, name)): Path<(String, String)>,
    Query(q): Query<RetryQuery>,
) -> Reply {
    require_kairon()?;
    let (store, hooks) = runtime(&state).await;
    let out = retry_with_hooks(
        &store,
        &hooks,
        &actor(&auth),
        &ns,
        &name,
        q.force,
        std::time::Duration::from_millis(500),
        30,
    )
    .await
    .map_err(api_fail)?;
    let data = out.machine.as_ref().map(import_json);
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"success": true, "data": data})),
    ))
}

/// `POST /imports/validate`: dry-run preflight; always 200 with a report (see `import_validate`).
async fn validate_import_route(
    State(state): State<SharedState>,
    Json(req): Json<ValidateRequest>,
) -> Reply {
    require_kairon()?;
    let mappings = {
        let s = state.read().await;
        KubeMappingStore {
            client: s.client().client(),
            namespace: s.namespace.clone(),
        }
    };
    let report =
        run_validate_with_mapping(&req, &ClusterFacts::from_env(), &HttpProbe, &mappings).await;
    Ok((
        StatusCode::OK,
        Json(json!({"success": true, "data": report})),
    ))
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/imports", get(list_imports).post(create_import))
        .route("/imports/validate", post(validate_import_route))
        .route(
            "/imports/:ns/:name",
            get(get_import).delete(delete_import_route),
        )
        .route("/imports/:ns/:name/retry", post(retry_import_route))
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
        assert!(ann[IMPORT_ID_ANNOTATION].starts_with("imp-"));
        assert_eq!(ann["veyron.io/import-last-phase"], "pending");
    }

    #[test]
    fn import_json_carries_phase_fields() {
        let m = machine_for_import(&req(), "prod").unwrap();
        let v = import_json(&m);
        assert_eq!(v["phase"], "pending");
        assert_eq!(v["terminal"], false);
        assert_eq!(v["retryable"], false);
        assert_eq!(v["retries"], 0);
        assert!(v["import_id"].as_str().unwrap().starts_with("imp-"));
    }

    #[tokio::test]
    async fn mapping_sets_network_and_records_pool() {
        use crate::api::handlers::import_mappings::tests::{FakeMappings, dc1};
        let maps = FakeMappings::default();
        maps.0.lock().unwrap().insert("dc1".into(), dc1());
        let mut r = req();
        r.mapping = Some("dc1".into());
        r.source.network = Some("DMZ".into());
        r.source.datastore = Some("ds-ssd".into());
        r.forwards = vec![PortForward {
            host_port: 30022,
            guest_port: 22,
            protocol: None,
        }];
        let m = build_import(&maps, &r, "prod").await.unwrap();
        let net = m.spec.network.as_ref().unwrap();
        assert_eq!(net.mode.as_deref(), Some("tap"));
        assert_eq!(net.bridge.as_deref(), Some("br-dmz"));
        assert_eq!(net.forwards.len(), 1, "forwards survive the mapping");
        let v = import_json(&m);
        assert_eq!(v["mapping"], "dc1");
        assert_eq!(v["storage_pool"], "fast");
        assert_eq!(v["source_network"], "DMZ");
        assert_eq!(v["network"]["bridge"], "br-dmz");

        // no mapping: source names are only recorded
        let mut plain = req();
        plain.source.network = Some("DMZ".into());
        let m = build_import(&maps, &plain, "prod").await.unwrap();
        assert!(m.spec.network.is_none());
        assert_eq!(import_json(&m)["source_network"], "DMZ");
    }

    #[tokio::test]
    async fn mapping_errors_are_400() {
        use crate::api::handlers::import_mappings::tests::{FakeMappings, dc1};
        let maps = FakeMappings::default();
        let mut r = req();
        r.mapping = Some("nope".into());
        let e = build_import(&maps, &r, "prod").await.unwrap_err();
        assert_eq!(
            (e.status, e.code),
            (StatusCode::BAD_REQUEST, "MAPPING_NOT_FOUND")
        );
        maps.0.lock().unwrap().insert("dc1".into(), dc1());
        r.mapping = Some("dc1".into());
        r.source.network = Some("Lab".into());
        let e = build_import(&maps, &r, "prod").await.unwrap_err();
        assert_eq!(e.code, "UNMAPPED_SOURCE");
        let mut bad = req();
        bad.name = "Bad".into();
        assert_eq!(
            build_import(&maps, &bad, "prod").await.unwrap_err().code,
            "INVALID_REQUEST"
        );
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
