// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Capture a Kairon Machine's root disk as a golden image.
//!
//! `POST /vms/:ns/:name/capture` starts a background job: a `MachineBackup`
//! (guest filesystems frozen when the agent answers), then the backup's root
//! qcow2 streamed from the node relay into the image store, then a
//! `MachineImage` of kind `disk`, then the backup is deleted. Job state lives
//! in a ConfigMap labeled `veyron.io/type=image-capture` in the API namespace.

use std::collections::BTreeMap;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::machine_images::{
    KIND_DISK, PublishRequest, Reply, StoreLocation, catalog_image_for, catalog_json, fail,
    get_catalog_image, ok, read_stored, require_kairon, store_dir, store_stream, valid_image_name,
};
use crate::api::http_server::web::SharedState;
use crate::kairon::{CatalogImage, LABEL_MANAGED_BY, Machine, MachineBackup, MachineBackupSpec};

const TYPE_LABEL: &str = "veyron.io/type";
const TYPE_VALUE: &str = "image-capture";
const JOB_KEY: &str = "job.json";
const BACKUP_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);
const POLL: Duration = Duration::from_secs(5);

pub const PHASE_BACKING_UP: &str = "BackingUp";
pub const PHASE_DOWNLOADING: &str = "Downloading";
pub const PHASE_PUBLISHING: &str = "Publishing";
pub const PHASE_SUCCEEDED: &str = "Succeeded";
pub const PHASE_FAILED: &str = "Failed";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CaptureRequest {
    /// Catalog (and upload) name of the new image.
    pub name: String,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub cpu: Option<String>,
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(default)]
    pub disk_size: Option<String>,
    /// `auto` (default), `required` or `never`.
    #[serde(default)]
    pub quiesce: Option<String>,
    /// Keep the MachineBackup afterwards (default: delete it).
    #[serde(default)]
    pub keep_backup: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CaptureJob {
    pub image: String,
    pub namespace: String,
    pub vm: String,
    pub phase: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub backup: String,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    pub started_at: String,
    #[serde(default)]
    pub finished_at: Option<String>,
}

impl CaptureJob {
    fn terminal(&self) -> bool {
        self.phase == PHASE_SUCCEEDED || self.phase == PHASE_FAILED
    }
}

pub fn job_configmap_name(image: &str) -> String {
    format!("image-capture-{image}")
}

/// MachineBackup name for a capture, kept within 63 characters.
pub fn backup_name(vm: &str, stamp: &str) -> String {
    let suffix = format!("-capture-{stamp}");
    let keep = 63usize.saturating_sub(suffix.len());
    let prefix: String = vm.chars().take(keep).collect();
    format!("{}{suffix}", prefix.trim_end_matches(['-', '.']))
}

fn validate(req: &CaptureRequest) -> Result<(), String> {
    if !valid_image_name(&req.name) {
        return Err(format!(
            "name {:?} must be lowercase letters, digits, '-' or '.', up to 63 characters",
            req.name
        ));
    }
    if job_configmap_name(&req.name).len() > 253 {
        return Err("name is too long".into());
    }
    match req.quiesce.as_deref() {
        None | Some("" | "auto" | "required" | "never") => Ok(()),
        Some(q) => Err(format!(
            "quiesce must be auto, required or never, not {q:?}"
        )),
    }
}

fn publish_request(req: &CaptureRequest) -> PublishRequest {
    PublishRequest {
        name: req.name.clone(),
        from_store: Some(req.name.clone()),
        kind: Some(KIND_DISK.into()),
        os: req.os.clone(),
        family: req.family.clone(),
        version: req.version.clone(),
        display_name: req.display_name.clone(),
        cpu: req.cpu.clone(),
        memory: req.memory.clone(),
        disk_size: req.disk_size.clone(),
        ..Default::default()
    }
}

fn job_configmap(job: &CaptureJob) -> ConfigMap {
    let mut cm = ConfigMap::default();
    cm.metadata.name = Some(job_configmap_name(&job.image));
    cm.metadata.labels = Some(BTreeMap::from([
        (TYPE_LABEL.to_string(), TYPE_VALUE.to_string()),
        (LABEL_MANAGED_BY.to_string(), "veyron".to_string()),
    ]));
    cm.data = Some(BTreeMap::from([(
        JOB_KEY.to_string(),
        serde_json::to_string(job).unwrap_or_default(),
    )]));
    cm
}

fn job_from(cm: &ConfigMap) -> Option<CaptureJob> {
    serde_json::from_str(cm.data.as_ref()?.get(JOB_KEY)?).ok()
}

async fn save(cms: &Api<ConfigMap>, job: &CaptureJob) {
    let patch = json!({"data": {JOB_KEY: serde_json::to_string(job).unwrap_or_default()}});
    if let Err(e) = cms
        .patch(
            &job_configmap_name(&job.image),
            &PatchParams::default(),
            &Patch::Merge(&patch),
        )
        .await
    {
        log::warn!("image capture {}: saving job state: {e}", job.image);
    }
}

/// `POST /api/v1/vms/:ns/:name/capture`
async fn start_capture(
    State(state): State<SharedState>,
    Path((ns, vm)): Path<(String, String)>,
    Json(req): Json<CaptureRequest>,
) -> Reply {
    require_kairon()?;
    validate(&req).map_err(|e| fail(StatusCode::BAD_REQUEST, "INVALID_REQUEST", e))?;
    let (client, api_ns) = {
        let st = state.read().await;
        (st.client().client(), st.namespace.clone())
    };
    let internal = |e: kube::Error| {
        fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CAPTURE_FAILED",
            e.to_string(),
        )
    };

    match Api::<Machine>::namespaced(client.clone(), &ns)
        .get_opt(&vm)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err(fail(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                format!("Machine {ns}/{vm} not found"),
            ));
        }
        Err(e) => return Err(internal(e)),
    }
    if get_catalog_image(&client, &req.name)
        .await
        .map_err(internal)?
        .is_some()
    {
        return Err(fail(
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
            format!(
                "MachineImage {} already exists; pick a new version name",
                req.name
            ),
        ));
    }
    if read_stored(&store_dir(), &req.name).await.is_some() {
        return Err(fail(
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
            format!("upload {} already exists in the image store", req.name),
        ));
    }

    let cms = Api::<ConfigMap>::namespaced(client.clone(), &api_ns);
    let cm_name = job_configmap_name(&req.name);
    if let Some(cm) = cms.get_opt(&cm_name).await.map_err(internal)? {
        if job_from(&cm).is_some_and(|j| !j.terminal()) {
            return Err(fail(
                StatusCode::CONFLICT,
                "IN_PROGRESS",
                format!("a capture to {} is already running", req.name),
            ));
        }
        cms.delete(&cm_name, &DeleteParams::default())
            .await
            .map_err(internal)?;
    }

    let now = chrono::Utc::now();
    let job = CaptureJob {
        image: req.name.clone(),
        namespace: ns.clone(),
        vm: vm.clone(),
        phase: PHASE_BACKING_UP.into(),
        backup: backup_name(&vm, &now.format("%Y%m%d%H%M%S").to_string()),
        started_at: now.to_rfc3339(),
        ..Default::default()
    };
    cms.create(&PostParams::default(), &job_configmap(&job))
        .await
        .map_err(internal)?;

    let reply = serde_json::to_value(&job).unwrap_or_default();
    tokio::spawn(run_capture(client, cms, job, req));
    ok(StatusCode::ACCEPTED, reply)
}

async fn run_capture(
    client: kube::Client,
    cms: Api<ConfigMap>,
    mut job: CaptureJob,
    req: CaptureRequest,
) {
    let backups = Api::<MachineBackup>::namespaced(client.clone(), &job.namespace);
    let result = capture_steps(&client, &cms, &backups, &mut job, &req).await;
    let backup_exists = !matches!(&result, Err((false, _)));
    match result {
        Ok(()) => {
            job.phase = PHASE_SUCCEEDED.into();
            job.message = format!("published MachineImage {}", job.image);
        }
        Err((_, msg)) => {
            job.phase = PHASE_FAILED.into();
            job.message = msg;
        }
    }
    if backup_exists && !req.keep_backup {
        if let Err(e) = backups.delete(&job.backup, &DeleteParams::default()).await {
            log::warn!("image capture: deleting MachineBackup {}: {e}", job.backup);
        }
    }
    job.finished_at = Some(chrono::Utc::now().to_rfc3339());
    log::info!("image capture {} {}: {}", job.image, job.phase, job.message);
    save(&cms, &job).await;
}

/// Errors carry whether the MachineBackup was created (so it gets cleaned up).
async fn capture_steps(
    client: &kube::Client,
    cms: &Api<ConfigMap>,
    backups: &Api<MachineBackup>,
    job: &mut CaptureJob,
    req: &CaptureRequest,
) -> Result<(), (bool, String)> {
    let mut b = MachineBackup::new(
        &job.backup,
        MachineBackupSpec {
            machine_name: job.vm.clone(),
            quiesce: req.quiesce.clone().filter(|q| !q.is_empty()),
            atlas: None,
        },
    );
    b.metadata.labels = Some(BTreeMap::from([
        (LABEL_MANAGED_BY.to_string(), "veyron".to_string()),
        (TYPE_LABEL.to_string(), TYPE_VALUE.to_string()),
    ]));
    backups
        .create(&PostParams::default(), &b)
        .await
        .map_err(|e| (false, format!("create MachineBackup {}: {e}", job.backup)))?;

    let deadline = tokio::time::Instant::now() + BACKUP_TIMEOUT;
    let (disk_name, node) = loop {
        tokio::time::sleep(POLL).await;
        let b = backups
            .get(&job.backup)
            .await
            .map_err(|e| (true, format!("read MachineBackup {}: {e}", job.backup)))?;
        let st = b.status.unwrap_or_default();
        match st.phase.as_deref() {
            Some("Succeeded") => {
                let disk = st
                    .extra
                    .get("disk")
                    .and_then(|d| d.get("name"))
                    .and_then(Value::as_str);
                let node = st.extra.get("nodeName").and_then(Value::as_str);
                match (disk, node) {
                    (Some(d), Some(n)) if !d.is_empty() && !n.is_empty() => {
                        break (d.to_string(), n.to_string());
                    }
                    _ => {
                        return Err((
                            true,
                            format!(
                                "MachineBackup {} succeeded without a FluxVM disk backup",
                                job.backup
                            ),
                        ));
                    }
                }
            }
            Some("Failed") => {
                return Err((
                    true,
                    format!(
                        "MachineBackup {} failed: {}",
                        job.backup,
                        st.message.unwrap_or_default()
                    ),
                ));
            }
            _ if tokio::time::Instant::now() > deadline => {
                return Err((
                    true,
                    format!(
                        "MachineBackup {} did not finish in {}m",
                        job.backup,
                        BACKUP_TIMEOUT.as_secs() / 60
                    ),
                ));
            }
            _ => {}
        }
    };

    job.phase = PHASE_DOWNLOADING.into();
    job.message = format!("streaming backup {disk_name} from node {node}");
    save(cms, job).await;
    let relay = crate::kairon::relay::RelayTarget::for_node(client, &node)
        .await
        .map_err(|e| (true, format!("{e:#}")))?;
    let resp = relay
        .backup_root(&disk_name)
        .await
        .map_err(|e| (true, format!("{e:#}")))?;
    let stored = store_stream(
        &store_dir(),
        &job.image,
        "qcow2".into(),
        resp.bytes_stream(),
        None,
        false,
    )
    .await
    .map_err(|e| (true, format!("storing {}: {e}", job.image)))?;
    job.digest = Some(stored.digest.clone());
    job.size = Some(stored.size);

    job.phase = PHASE_PUBLISHING.into();
    job.message = format!("publishing MachineImage {}", job.image);
    save(cms, job).await;
    let img = catalog_image_for(
        &publish_request(req),
        Some(&stored),
        &StoreLocation::from_env(),
    )
    .map_err(|e| {
        (
            true,
            format!("upload {} is stored but can't be published: {e}", job.image),
        )
    })?;
    Api::<CatalogImage>::all(client.clone())
        .create(&PostParams::default(), &img)
        .await
        .map_err(|e| {
            (
                true,
                format!("upload {} is stored but publishing failed: {e}", job.image),
            )
        })?;
    Ok(())
}

/// `GET /api/v1/image-captures`
async fn list_captures(State(state): State<SharedState>) -> Reply {
    require_kairon()?;
    let (client, api_ns) = {
        let st = state.read().await;
        (st.client().client(), st.namespace.clone())
    };
    let list = Api::<ConfigMap>::namespaced(client, &api_ns)
        .list(&ListParams::default().labels(&format!("{TYPE_LABEL}={TYPE_VALUE}")))
        .await
        .map_err(|e| {
            fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_FAILED",
                e.to_string(),
            )
        })?;
    let mut jobs: Vec<CaptureJob> = list.items.iter().filter_map(job_from).collect();
    jobs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    ok(StatusCode::OK, json!(jobs))
}

/// `DELETE /api/v1/image-captures/:image` — forget a finished job.
async fn delete_capture(State(state): State<SharedState>, Path(image): Path<String>) -> Reply {
    require_kairon()?;
    let (client, api_ns) = {
        let st = state.read().await;
        (st.client().client(), st.namespace.clone())
    };
    let cms = Api::<ConfigMap>::namespaced(client.clone(), &api_ns);
    let name = job_configmap_name(&image);
    let cm = cms
        .get_opt(&name)
        .await
        .map_err(|e| {
            fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DELETE_FAILED",
                e.to_string(),
            )
        })?
        .ok_or_else(|| {
            fail(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                format!("no capture job for {image}"),
            )
        })?;
    if job_from(&cm).is_some_and(|j| !j.terminal()) {
        return Err(fail(
            StatusCode::CONFLICT,
            "IN_PROGRESS",
            format!("capture to {image} is still running"),
        ));
    }
    cms.delete(&name, &DeleteParams::default())
        .await
        .map_err(|e| {
            fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DELETE_FAILED",
                e.to_string(),
            )
        })?;
    let image_json = get_catalog_image(&client, &image)
        .await
        .ok()
        .flatten()
        .map(|m| catalog_json(&m));
    ok(
        StatusCode::OK,
        json!({"image": image, "machine_image": image_json}),
    )
}

/// Nested under `/api/v1`.
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/vms/:ns/:name/capture", post(start_capture))
        .route("/image-captures", get(list_captures))
        .route("/image-captures/:image", delete(delete_capture))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_names_fit_kubernetes_limits() {
        assert_eq!(
            backup_name("win-gold", "20261008010203"),
            "win-gold-capture-20261008010203"
        );
        let long = backup_name(&"a".repeat(80), "20261008010203");
        assert_eq!(long.len(), 63);
        assert!(long.ends_with("-capture-20261008010203"));
        let dash = backup_name(&format!("{}-x", "b".repeat(39)), "20261008010203");
        assert!(!dash.contains("--"), "{dash}");
    }

    #[test]
    fn validates_name_and_quiesce() {
        let mut req = CaptureRequest {
            name: "ubuntu-gold-v1".into(),
            ..Default::default()
        };
        assert!(validate(&req).is_ok());
        req.quiesce = Some("required".into());
        assert!(validate(&req).is_ok());
        req.quiesce = Some("always".into());
        assert!(validate(&req).is_err());
        req.quiesce = None;
        req.name = "Bad_Name".into();
        assert!(validate(&req).is_err());
    }

    #[test]
    fn publishes_the_stored_upload_as_a_disk() {
        let req = CaptureRequest {
            name: "web-gold-v2".into(),
            os: Some("linux".into()),
            cpu: Some("2".into()),
            ..Default::default()
        };
        let p = publish_request(&req);
        assert_eq!(p.from_store.as_deref(), Some("web-gold-v2"));
        assert_eq!(p.kind.as_deref(), Some(KIND_DISK));
        assert_eq!(p.cpu.as_deref(), Some("2"));
    }

    #[test]
    fn job_round_trips_through_its_configmap() {
        let job = CaptureJob {
            image: "web-gold-v2".into(),
            namespace: "default".into(),
            vm: "web".into(),
            phase: PHASE_DOWNLOADING.into(),
            backup: "web-capture-1".into(),
            started_at: "2026-10-08T00:00:00Z".into(),
            ..Default::default()
        };
        let cm = job_configmap(&job);
        assert_eq!(
            cm.metadata.name.as_deref(),
            Some("image-capture-web-gold-v2")
        );
        assert_eq!(
            cm.metadata
                .labels
                .as_ref()
                .unwrap()
                .get(TYPE_LABEL)
                .map(String::as_str),
            Some(TYPE_VALUE)
        );
        let back = job_from(&cm).unwrap();
        assert_eq!(back, job);
        assert!(!back.terminal());
    }
}
