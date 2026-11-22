// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Import lifecycle: phase derivation, delete and retry.
//!
//! The decisions and builders are pure. The orchestration (`delete_import`, `retry_import`)
//! is generic over [`MachineStore`], so tests drive it with an in-memory fake and the
//! handlers drive it with [`KubeStore`].
//!
//! Kairon cannot re-run a failed image download in place, so a retry deletes the Machine,
//! waits for it to disappear and recreates it from its own spec with the same import id.

use std::collections::BTreeMap;
use std::future::Future;
use std::time::Duration;

use axum::http::StatusCode;
use kube::api::{Api, DeleteParams, PostParams};
use serde::Serialize;

use super::imports::IMPORTED_BY_LABEL;
use crate::kairon::Machine;

pub const IMPORT_ID_ANNOTATION: &str = "veyron.io/import-id";
pub const RETRIES_ANNOTATION: &str = "veyron.io/import-retries";

/// Coarse import progress derived from the Machine status.
///
/// Kairon reports `Running`, `Halted` and friends; it has no dedicated download phase, so
/// `downloading` and `converting` are best-effort guesses from the status message and only
/// ever refine `pending`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhase {
    Pending,
    Downloading,
    Converting,
    Running,
    Stopped,
    Failed,
    Terminating,
}

impl ImportPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Downloading => "downloading",
            Self::Converting => "converting",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
            Self::Terminating => "terminating",
        }
    }

    /// No further transition happens without an operator action.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Running | Self::Stopped | Self::Failed)
    }
}

fn mentions(text: &str, needles: &[&str]) -> bool {
    let t = text.to_ascii_lowercase();
    needles.iter().any(|n| t.contains(n))
}

pub fn phase_of(m: &Machine) -> ImportPhase {
    if m.metadata.deletion_timestamp.is_some() {
        return ImportPhase::Terminating;
    }
    let raw = m.phase().to_ascii_lowercase();
    let failed_condition = m
        .status
        .as_ref()
        .map(|s| {
            s.conditions.iter().any(|c| {
                c.status == "False"
                    && c.reason
                        .as_deref()
                        .map(|r| mentions(r, &["fail", "error"]))
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    if raw.contains("fail") || raw.contains("error") || failed_condition {
        return ImportPhase::Failed;
    }
    match raw.as_str() {
        "running" => ImportPhase::Running,
        "halted" | "stopped" | "paused" => ImportPhase::Stopped,
        _ => {
            if m.power_state() != crate::kairon::POWER_RUNNING && raw == "unknown" {
                return ImportPhase::Stopped;
            }
            match m.message() {
                Some(msg) if mentions(msg, &["convert"]) => ImportPhase::Converting,
                Some(msg) if mentions(msg, &["download", "fetch", "pull"]) => {
                    ImportPhase::Downloading
                }
                _ => ImportPhase::Pending,
            }
        }
    }
}

pub fn is_import(m: &Machine) -> bool {
    m.metadata
        .labels
        .as_ref()
        .is_some_and(|l| l.contains_key(IMPORTED_BY_LABEL))
}

pub fn import_id(m: &Machine) -> Option<String> {
    m.metadata
        .annotations
        .as_ref()
        .and_then(|a| a.get(IMPORT_ID_ANNOTATION))
        .cloned()
}

pub fn retries(m: &Machine) -> u32 {
    m.metadata
        .annotations
        .as_ref()
        .and_then(|a| a.get(RETRIES_ANNOTATION))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// `imp-` plus 8 hex characters.
pub fn new_import_id() -> String {
    use rand::Rng;
    format!("imp-{:08x}", rand::thread_rng().r#gen::<u32>())
}

/// API-shaped error, converted to the handlers' JSON envelope by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiErr {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

fn api_err(status: StatusCode, code: &'static str, message: impl Into<String>) -> ApiErr {
    ApiErr {
        status,
        code,
        message: message.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteDecision {
    Proceed,
    /// Already being deleted: answer success without a second delete call.
    AlreadyTerminating,
    NotAnImport,
}

pub fn delete_decision(m: &Machine) -> DeleteDecision {
    if !is_import(m) {
        DeleteDecision::NotAnImport
    } else if m.metadata.deletion_timestamp.is_some() {
        DeleteDecision::AlreadyTerminating
    } else {
        DeleteDecision::Proceed
    }
}

/// Why a retry is refused.
pub fn retry_refusal(m: &Machine, force: bool) -> Option<ApiErr> {
    let phase = phase_of(m);
    if !is_import(m) {
        Some(api_err(
            StatusCode::NOT_FOUND,
            "NOT_AN_IMPORT",
            "VM was not created by an import",
        ))
    } else if phase == ImportPhase::Terminating {
        Some(api_err(
            StatusCode::CONFLICT,
            "TERMINATING",
            "import is being deleted",
        ))
    } else if phase != ImportPhase::Failed && !force {
        Some(api_err(
            StatusCode::CONFLICT,
            "NOT_RETRYABLE",
            format!(
                "import is {}; only failed imports are retried (pass ?force=true to recreate anyway)",
                phase.as_str()
            ),
        ))
    } else {
        None
    }
}

/// A fresh Machine with the same spec, labels and annotations, import id preserved and the
/// retry counter incremented. Server-assigned metadata and status are dropped.
pub fn retry_machine(old: &Machine) -> Machine {
    let mut m = Machine::new(
        &old.metadata.name.clone().unwrap_or_default(),
        old.spec.clone(),
    );
    m.metadata.namespace = old.metadata.namespace.clone();
    m.metadata.labels = old.metadata.labels.clone();
    let mut ann: BTreeMap<String, String> = old.metadata.annotations.clone().unwrap_or_default();
    ann.entry(IMPORT_ID_ANNOTATION.into())
        .or_insert_with(new_import_id);
    ann.insert(RETRIES_ANNOTATION.into(), (retries(old) + 1).to_string());
    m.metadata.annotations = Some(ann);
    m
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Conflict,
    Other(String),
}

/// The slice of the Kubernetes API the import handlers need, so they run without a cluster.
pub trait MachineStore: Send + Sync {
    fn get(
        &self,
        ns: &str,
        name: &str,
    ) -> impl Future<Output = Result<Option<Machine>, StoreError>> + Send;
    fn create(&self, m: &Machine) -> impl Future<Output = Result<Machine, StoreError>> + Send;
    /// Deleting a Machine that is already gone is `Ok`.
    fn delete(&self, ns: &str, name: &str) -> impl Future<Output = Result<(), StoreError>> + Send;
    /// Set one annotation (merge patch). The default does nothing, for stores that do not
    /// track annotations.
    #[allow(clippy::manual_async_fn)]
    fn annotate(
        &self,
        _ns: &str,
        _name: &str,
        _key: &str,
        _value: &str,
    ) -> impl Future<Output = Result<(), StoreError>> + Send {
        async { Ok(()) }
    }
}

pub struct KubeStore {
    pub client: kube::Client,
}

fn store_err(e: kube::Error) -> StoreError {
    match e {
        kube::Error::Api(a) if a.code == 409 => StoreError::Conflict,
        other => StoreError::Other(other.to_string()),
    }
}

impl MachineStore for KubeStore {
    async fn get(&self, ns: &str, name: &str) -> Result<Option<Machine>, StoreError> {
        Api::<Machine>::namespaced(self.client.clone(), ns)
            .get_opt(name)
            .await
            .map_err(store_err)
    }

    async fn create(&self, m: &Machine) -> Result<Machine, StoreError> {
        let ns = m.metadata.namespace.clone().unwrap_or_default();
        Api::<Machine>::namespaced(self.client.clone(), &ns)
            .create(&PostParams::default(), m)
            .await
            .map_err(store_err)
    }

    async fn delete(&self, ns: &str, name: &str) -> Result<(), StoreError> {
        match Api::<Machine>::namespaced(self.client.clone(), ns)
            .delete(name, &DeleteParams::default())
            .await
        {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(a)) if a.code == 404 => Ok(()),
            Err(e) => Err(store_err(e)),
        }
    }

    async fn annotate(
        &self,
        ns: &str,
        name: &str,
        key: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        let patch = serde_json::json!({"metadata": {"annotations": {key: value}}});
        Api::<Machine>::namespaced(self.client.clone(), ns)
            .patch(
                name,
                &kube::api::PatchParams::default(),
                &kube::api::Patch::Merge(&patch),
            )
            .await
            .map(|_| ())
            .map_err(store_err)
    }
}

fn other(e: StoreError) -> ApiErr {
    match e {
        StoreError::Conflict => api_err(StatusCode::CONFLICT, "CONFLICT", "resource conflict"),
        StoreError::Other(m) => api_err(StatusCode::INTERNAL_SERVER_ERROR, "STORE_FAILED", m),
    }
}

fn not_found(ns: &str, name: &str) -> ApiErr {
    api_err(
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        format!("VM {ns}/{name} not found"),
    )
}

/// What a lifecycle action did.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub import_id: Option<String>,
    pub phase: ImportPhase,
    pub machine: Option<Machine>,
}

pub async fn delete_import<S: MachineStore>(
    store: &S,
    ns: &str,
    name: &str,
) -> Result<Outcome, ApiErr> {
    let m = store
        .get(ns, name)
        .await
        .map_err(other)?
        .ok_or_else(|| not_found(ns, name))?;
    match delete_decision(&m) {
        DeleteDecision::NotAnImport => Err(api_err(
            StatusCode::NOT_FOUND,
            "NOT_AN_IMPORT",
            format!("VM {ns}/{name} was not created by an import; delete it through /vms"),
        )),
        DeleteDecision::AlreadyTerminating => Ok(Outcome {
            import_id: import_id(&m),
            phase: ImportPhase::Terminating,
            machine: None,
        }),
        DeleteDecision::Proceed => {
            store.delete(ns, name).await.map_err(other)?;
            Ok(Outcome {
                import_id: import_id(&m),
                phase: ImportPhase::Terminating,
                machine: None,
            })
        }
    }
}

/// Delete a failed (or, with `force`, any) import and recreate it. Polls every `interval`
/// up to `attempts` times for the old Machine to disappear.
pub async fn retry_import<S: MachineStore>(
    store: &S,
    ns: &str,
    name: &str,
    force: bool,
    interval: Duration,
    attempts: u32,
) -> Result<Outcome, ApiErr> {
    let old = store
        .get(ns, name)
        .await
        .map_err(other)?
        .ok_or_else(|| not_found(ns, name))?;
    if let Some(refusal) = retry_refusal(&old, force) {
        return Err(refusal);
    }
    let fresh = retry_machine(&old);
    store.delete(ns, name).await.map_err(other)?;
    let mut gone = false;
    for _ in 0..attempts {
        if store.get(ns, name).await.map_err(other)?.is_none() {
            gone = true;
            break;
        }
        tokio::time::sleep(interval).await;
    }
    if !gone {
        return Err(api_err(
            StatusCode::CONFLICT,
            "STILL_TERMINATING",
            format!(
                "VM {ns}/{name} is still terminating; the delete was issued, call retry again shortly"
            ),
        ));
    }
    let created = store.create(&fresh).await.map_err(|e| match e {
        StoreError::Conflict => api_err(
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
            format!("VM {ns}/{name} was recreated by someone else"),
        ),
        e => other(e),
    })?;
    Ok(Outcome {
        import_id: import_id(&created),
        phase: phase_of(&created),
        machine: Some(created),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::kairon::{MachineSpec, MachineStatus};
    use std::sync::Mutex;

    pub fn import_machine(name: &str) -> Machine {
        let mut m = Machine::new(name, MachineSpec::default());
        m.metadata.namespace = Some("prod".into());
        m.metadata.labels = Some(BTreeMap::from([(IMPORTED_BY_LABEL.into(), "h2kvm".into())]));
        m.metadata.annotations = Some(BTreeMap::from([
            (IMPORT_ID_ANNOTATION.into(), "imp-00000001".into()),
            (
                "veyron.io/import-image-url".into(),
                "http://x/y.qcow2".into(),
            ),
        ]));
        m
    }

    fn with_phase(mut m: Machine, phase: &str, msg: Option<&str>) -> Machine {
        m.status = Some(MachineStatus {
            phase: Some(phase.into()),
            message: msg.map(Into::into),
            ..Default::default()
        });
        m
    }

    /// In-memory store. `sticky` makes a deleted Machine linger for N `get`s (finalizers).
    #[derive(Default)]
    pub struct FakeStore {
        pub machines: Mutex<BTreeMap<(String, String), Machine>>,
        pub sticky: Mutex<u32>,
        pub deletes: Mutex<u32>,
    }

    impl FakeStore {
        pub fn with(ms: Vec<Machine>) -> Self {
            let s = Self::default();
            for m in ms {
                s.machines.lock().unwrap().insert(
                    (
                        m.metadata.namespace.clone().unwrap(),
                        m.metadata.name.clone().unwrap(),
                    ),
                    m,
                );
            }
            s
        }
    }

    impl MachineStore for FakeStore {
        async fn get(&self, ns: &str, name: &str) -> Result<Option<Machine>, StoreError> {
            let key = (ns.to_string(), name.to_string());
            let mut sticky = self.sticky.lock().unwrap();
            let mut map = self.machines.lock().unwrap();
            if *sticky > 0 && *self.deletes.lock().unwrap() > 0 && map.contains_key(&key) {
                *sticky -= 1;
                if *sticky == 0 {
                    map.remove(&key);
                    return Ok(None);
                }
            }
            Ok(map.get(&key).cloned())
        }
        async fn create(&self, m: &Machine) -> Result<Machine, StoreError> {
            let key = (
                m.metadata.namespace.clone().unwrap(),
                m.metadata.name.clone().unwrap(),
            );
            let mut map = self.machines.lock().unwrap();
            if map.contains_key(&key) {
                return Err(StoreError::Conflict);
            }
            map.insert(key, m.clone());
            Ok(m.clone())
        }
        async fn annotate(
            &self,
            ns: &str,
            name: &str,
            key: &str,
            value: &str,
        ) -> Result<(), StoreError> {
            let mut map = self.machines.lock().unwrap();
            match map.get_mut(&(ns.to_string(), name.to_string())) {
                Some(m) => {
                    m.metadata
                        .annotations
                        .get_or_insert_with(Default::default)
                        .insert(key.to_string(), value.to_string());
                    Ok(())
                }
                None => Err(StoreError::Other("not found".into())),
            }
        }
        async fn delete(&self, ns: &str, name: &str) -> Result<(), StoreError> {
            *self.deletes.lock().unwrap() += 1;
            if *self.sticky.lock().unwrap() == 0 {
                self.machines
                    .lock()
                    .unwrap()
                    .remove(&(ns.to_string(), name.to_string()));
            }
            Ok(())
        }
    }

    #[test]
    fn phase_derivation() {
        let m = import_machine("a");
        assert_eq!(phase_of(&m), ImportPhase::Pending);
        assert_eq!(
            phase_of(&with_phase(import_machine("a"), "Running", None)),
            ImportPhase::Running
        );
        assert_eq!(
            phase_of(&with_phase(import_machine("a"), "Halted", None)),
            ImportPhase::Stopped
        );
        assert_eq!(
            phase_of(&with_phase(import_machine("a"), "Failed", Some("boom"))),
            ImportPhase::Failed
        );
        assert_eq!(
            phase_of(&with_phase(
                import_machine("a"),
                "Pending",
                Some("Downloading image 40%")
            )),
            ImportPhase::Downloading
        );
        assert_eq!(
            phase_of(&with_phase(
                import_machine("a"),
                "Pending",
                Some("converting vmdk")
            )),
            ImportPhase::Converting
        );
        let mut stopped = import_machine("a");
        stopped.spec.power_state = Some("Stopped".into());
        assert_eq!(phase_of(&stopped), ImportPhase::Stopped);
        let mut term = with_phase(import_machine("a"), "Running", None);
        term.metadata.deletion_timestamp = Some(
            k8s_openapi::apimachinery::pkg::apis::meta::v1::Time(chrono::Utc::now()),
        );
        assert_eq!(phase_of(&term), ImportPhase::Terminating);
        assert!(ImportPhase::Failed.is_terminal() && !ImportPhase::Pending.is_terminal());
    }

    #[test]
    fn failed_condition_means_failed() {
        let mut m = with_phase(import_machine("a"), "Pending", None);
        m.status.as_mut().unwrap().conditions = vec![crate::kairon::MachineCondition {
            type_: "ImageReady".into(),
            status: "False".into(),
            reason: Some("DigestMismatch Error".into()),
            message: None,
            last_transition_time: None,
        }];
        assert_eq!(phase_of(&m), ImportPhase::Failed);
    }

    #[test]
    fn delete_decisions() {
        assert_eq!(
            delete_decision(&import_machine("a")),
            DeleteDecision::Proceed
        );
        let plain = Machine::new("a", MachineSpec::default());
        assert_eq!(delete_decision(&plain), DeleteDecision::NotAnImport);
    }

    #[test]
    fn retry_machine_keeps_spec_and_bumps_counter() {
        let mut old = with_phase(import_machine("web01"), "Failed", None);
        old.metadata.resource_version = Some("77".into());
        old.metadata.uid = Some("u".into());
        old.spec.resources.memory = "8Gi".into();
        let fresh = retry_machine(&old);
        assert_eq!(fresh.metadata.name.as_deref(), Some("web01"));
        assert_eq!(fresh.metadata.namespace.as_deref(), Some("prod"));
        assert!(fresh.metadata.resource_version.is_none() && fresh.metadata.uid.is_none());
        assert!(fresh.status.is_none());
        assert_eq!(fresh.spec.resources.memory, "8Gi");
        assert_eq!(import_id(&fresh).as_deref(), Some("imp-00000001"));
        assert_eq!(retries(&fresh), 1);
        assert_eq!(retries(&retry_machine(&fresh)), 2);
        assert_eq!(
            fresh.metadata.annotations.unwrap()["veyron.io/import-image-url"],
            "http://x/y.qcow2"
        );
    }

    #[test]
    fn retry_refusals() {
        let running = with_phase(import_machine("a"), "Running", None);
        assert_eq!(
            retry_refusal(&running, false).unwrap().code,
            "NOT_RETRYABLE"
        );
        assert!(retry_refusal(&running, true).is_none());
        let failed = with_phase(import_machine("a"), "Failed", None);
        assert!(retry_refusal(&failed, false).is_none());
        let plain = Machine::new("a", MachineSpec::default());
        assert_eq!(retry_refusal(&plain, true).unwrap().code, "NOT_AN_IMPORT");
    }

    #[tokio::test]
    async fn delete_flow() {
        let store = FakeStore::with(vec![import_machine("web01")]);
        let out = delete_import(&store, "prod", "web01").await.unwrap();
        assert_eq!(out.phase, ImportPhase::Terminating);
        assert_eq!(out.import_id.as_deref(), Some("imp-00000001"));
        assert!(store.get("prod", "web01").await.unwrap().is_none());
        assert_eq!(
            delete_import(&store, "prod", "web01")
                .await
                .unwrap_err()
                .code,
            "NOT_FOUND"
        );
        let mut plain = Machine::new("vm", MachineSpec::default());
        plain.metadata.namespace = Some("prod".into());
        let store = FakeStore::with(vec![plain]);
        let err = delete_import(&store, "prod", "vm").await.unwrap_err();
        assert_eq!(err.code, "NOT_AN_IMPORT");
        assert_eq!(
            *store.deletes.lock().unwrap(),
            0,
            "never deletes a non-import"
        );
    }

    #[tokio::test]
    async fn retry_flow_recreates_failed_import() {
        let store = FakeStore::with(vec![with_phase(import_machine("web01"), "Failed", None)]);
        let out = retry_import(&store, "prod", "web01", false, Duration::from_millis(1), 5)
            .await
            .unwrap();
        assert_eq!(out.phase, ImportPhase::Pending);
        let now = store.get("prod", "web01").await.unwrap().unwrap();
        assert!(now.status.is_none());
        assert_eq!(retries(&now), 1);
    }

    #[tokio::test]
    async fn retry_waits_for_finalizers_then_gives_up() {
        let failed = with_phase(import_machine("web01"), "Failed", None);
        let store = FakeStore::with(vec![failed.clone()]);
        *store.sticky.lock().unwrap() = 3;
        retry_import(&store, "prod", "web01", false, Duration::from_millis(1), 10)
            .await
            .unwrap();
        let store = FakeStore::with(vec![failed]);
        *store.sticky.lock().unwrap() = 100;
        let err = retry_import(&store, "prod", "web01", false, Duration::from_millis(1), 3)
            .await
            .unwrap_err();
        assert_eq!(err.code, "STILL_TERMINATING");
    }

    #[tokio::test]
    async fn retry_refuses_healthy_import_without_deleting() {
        let store = FakeStore::with(vec![with_phase(import_machine("web01"), "Running", None)]);
        let err = retry_import(&store, "prod", "web01", false, Duration::from_millis(1), 3)
            .await
            .unwrap_err();
        assert_eq!(err.code, "NOT_RETRYABLE");
        assert_eq!(*store.deletes.lock().unwrap(), 0);
    }
}
