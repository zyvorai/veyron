// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Import phase-change webhooks and SOC audit entries tagged with the import id.
//!
//! Veyron has no controller loop watching Machines, so a phase change is noticed when
//! something reads the import (`GET /imports`, `GET /imports/:ns/:name`, retry, delete). The last
//! phase an observer saw is kept in the `veyron.io/import-last-phase` annotation; a different
//! current phase emits one `import.phase_changed` webhook and updates the annotation. Delivery is
//! best effort and runs in the background so a slow receiver never delays an API response; two
//! readers racing on the same change can both emit it.
//!
//! Webhooks are the existing ConfigMaps (`veyron.io/type=webhook`, see `POST /webhooks`); a
//! webhook receives these events when its `events` list contains `import.phase_changed`
//! (or `import.*`, or `*`).

use std::collections::BTreeMap;
use std::future::Future;

use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::api::{Api, ListParams};

use super::import_lifecycle::{
    ApiErr, ImportPhase, MachineStore, Outcome, delete_import, import_id, is_import, phase_of,
    retry_import,
};
use super::import_plans::BatchReport;
use crate::api::webhooks::{WebhookConfig, WebhookEvent, WebhookPayload};
use crate::kairon::Machine;
use crate::soc::event::SecurityEvent;

pub const PHASE_ANNOTATION: &str = "veyron.io/import-last-phase";
pub const EVENT_NAME: &str = "import.phase_changed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseChange {
    pub import_id: Option<String>,
    pub name: String,
    pub namespace: String,
    pub from: Option<String>,
    pub to: ImportPhase,
    pub message: Option<String>,
}

/// The change between the last observed phase and the current one, if any. Machines that are
/// not imports, and imports never observed (no annotation yet), report nothing until the
/// annotation exists, which creation sets, so the first real transition is never lost.
pub fn detect_change(m: &Machine) -> Option<PhaseChange> {
    if !is_import(m) {
        return None;
    }
    let now = phase_of(m);
    let prev = m
        .metadata
        .annotations
        .as_ref()
        .and_then(|a| a.get(PHASE_ANNOTATION))
        .cloned();
    if prev.as_deref() == Some(now.as_str()) {
        return None;
    }
    Some(PhaseChange {
        import_id: import_id(m),
        name: m.metadata.name.clone().unwrap_or_default(),
        namespace: m.metadata.namespace.clone().unwrap_or_default(),
        from: prev,
        to: now,
        message: m.message().map(str::to_string),
    })
}

pub fn payload_for(c: &PhaseChange) -> WebhookPayload {
    let mut p = WebhookPayload::new(&WebhookEvent::ImportPhaseChanged);
    p.add_data("name", &c.name);
    p.add_data("namespace", &c.namespace);
    p.add_data("phase", c.to.as_str());
    p.add_data("previous_phase", c.from.clone().unwrap_or_default());
    if let Some(id) = &c.import_id {
        p.add_data("import_id", id);
    }
    if let Some(m) = &c.message {
        p.add_data("message", m);
    }
    p
}

/// Does a webhook ConfigMap's `events` list subscribe to import phase changes?
pub fn subscribes(events_csv: &str) -> bool {
    events_csv
        .split(',')
        .map(str::trim)
        .any(|e| e == EVENT_NAME || e == "import.*" || e == "*")
}

/// Webhook ConfigMaps that want import events, as ready-to-deliver configs plus the name of the
/// Secret holding each signing key. Inactive webhooks and URLs that fail the SSRF policy are
/// dropped.
pub fn webhook_targets(cms: &[ConfigMap]) -> Vec<(WebhookConfig, Option<String>)> {
    cms.iter()
        .filter_map(|cm| {
            let d = cm.data.as_ref()?;
            if d.get("active").map(|v| v == "false").unwrap_or(false) {
                return None;
            }
            if !subscribes(d.get("events")?) {
                return None;
            }
            let mut cfg = WebhookConfig::new(d.get("name")?.clone(), d.get("url")?.clone()).ok()?;
            cfg.add_event(WebhookEvent::ImportPhaseChanged);
            Some((cfg, d.get("secret_ref").cloned()))
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub actor: String,
    /// `create`, `delete`, `retry`, `batch-create`.
    pub action: &'static str,
    pub namespace: String,
    pub name: String,
    pub import_id: Option<String>,
    pub outcome: &'static str,
    pub detail: Option<String>,
}

/// The SOC event for an import action. The import id is a label (`import_id`), so
/// `/soc/audit` and `/soc/events` entries can be joined to `GET /imports` rows and webhooks.
pub fn audit_event(a: &AuditRecord) -> SecurityEvent {
    let severity = if a.outcome == "success" {
        "low"
    } else {
        "medium"
    };
    let target = format!("{}/{}", a.namespace, a.name);
    let mut msg = format!("import {} {target} — {}", a.action, a.outcome);
    if let Some(id) = &a.import_id {
        msg.push_str(&format!(" ({id})"));
    }
    let mut ev = SecurityEvent::new("audit", severity, "veyron", msg)
        .with_actor(a.actor.clone())
        .with_action(format!("import.{}", a.action))
        .with_target(target)
        .with_namespace(a.namespace.clone())
        .with_vm(a.name.clone())
        .with_outcome(a.outcome);
    ev.labels.insert("via".into(), "imports".into());
    if let Some(id) = &a.import_id {
        ev.labels.insert("import_id".into(), id.clone());
    }
    if let Some(d) = &a.detail {
        ev.labels
            .insert("detail".into(), d.chars().take(200).collect());
    }
    ev
}

/// Side effects of import activity, injectable so handlers are testable.
pub trait ImportHooks: Send + Sync {
    fn phase_changed(&self, change: PhaseChange) -> impl Future<Output = ()> + Send;
    fn audit(&self, record: AuditRecord) -> impl Future<Output = ()> + Send;
}

/// Emit the change for `m` (if any) and record the new phase on the Machine.
pub async fn observe<S: MachineStore, H: ImportHooks>(store: &S, hooks: &H, m: &Machine) {
    let Some(change) = detect_change(m) else {
        return;
    };
    // Record first: if the annotation write fails we stay quiet rather than re-emit on every read.
    if store
        .annotate(
            &change.namespace,
            &change.name,
            PHASE_ANNOTATION,
            change.to.as_str(),
        )
        .await
        .is_err()
    {
        return;
    }
    hooks.phase_changed(change).await;
}

/// Real hooks: webhooks from ConfigMaps in Veyron's namespace, audit into the SOC event log.
pub struct ClusterHooks {
    pub client: kube::Client,
    pub namespace: String,
}

impl ImportHooks for ClusterHooks {
    async fn phase_changed(&self, change: PhaseChange) {
        let client = self.client.clone();
        let ns = self.namespace.clone();
        // Detached: never hold up the API response on a slow webhook receiver.
        tokio::spawn(async move {
            let cms: Api<ConfigMap> = Api::namespaced(client.clone(), &ns);
            let Ok(list) = cms
                .list(&ListParams::default().labels("veyron.io/type=webhook"))
                .await
            else {
                return;
            };
            let payload = payload_for(&change);
            for (cfg, secret_ref) in webhook_targets(&list.items) {
                let mut cfg = cfg;
                if let Some(sname) = secret_ref {
                    let secrets: Api<Secret> = Api::namespaced(client.clone(), &ns);
                    if let Ok(Some(s)) = secrets.get_opt(&sname).await {
                        if let Some(bytes) = s.data.and_then(|d| d.get("secret").cloned()) {
                            if let Ok(text) = String::from_utf8(bytes.0) {
                                cfg = cfg.with_secret(text);
                            }
                        }
                    }
                }
                if let Err(e) = crate::api::webhooks::deliver_webhook(&cfg, &payload).await {
                    log::warn!("import webhook '{}' failed: {e}", cfg.name);
                }
            }
        });
    }

    async fn audit(&self, record: AuditRecord) {
        let ev = audit_event(&record);
        crate::soc::store::append_events(&self.client, &self.namespace, vec![ev]).await;
    }
}

/// Initial annotation for a freshly built import.
pub fn initial_annotations() -> BTreeMap<String, String> {
    BTreeMap::from([(
        PHASE_ANNOTATION.to_string(),
        ImportPhase::Pending.as_str().to_string(),
    )])
}

fn record(
    actor: &str,
    action: &'static str,
    ns: &str,
    name: &str,
    id: Option<String>,
    result: &Result<(), ApiErr>,
) -> AuditRecord {
    AuditRecord {
        actor: actor.to_string(),
        action,
        namespace: ns.to_string(),
        name: name.to_string(),
        import_id: id,
        outcome: if result.is_ok() { "success" } else { "failure" },
        detail: result.as_ref().err().map(|e| e.code.to_string()),
    }
}

/// `delete_import` plus the audit entry and the `terminating` event.
pub async fn delete_with_hooks<S: MachineStore, H: ImportHooks>(
    store: &S,
    hooks: &H,
    actor: &str,
    ns: &str,
    name: &str,
) -> Result<Outcome, ApiErr> {
    let out = delete_import(store, ns, name).await;
    let id = out.as_ref().ok().and_then(|o| o.import_id.clone());
    hooks
        .audit(record(
            actor,
            "delete",
            ns,
            name,
            id.clone(),
            &out.as_ref().map(|_| ()).map_err(Clone::clone),
        ))
        .await;
    if out.is_ok() {
        hooks
            .phase_changed(PhaseChange {
                import_id: id,
                name: name.to_string(),
                namespace: ns.to_string(),
                from: None,
                to: ImportPhase::Terminating,
                message: None,
            })
            .await;
    }
    out
}

/// `retry_import` plus the audit entry and a `failed -> pending` event.
#[allow(clippy::too_many_arguments)]
pub async fn retry_with_hooks<S: MachineStore, H: ImportHooks>(
    store: &S,
    hooks: &H,
    actor: &str,
    ns: &str,
    name: &str,
    force: bool,
    interval: std::time::Duration,
    attempts: u32,
) -> Result<Outcome, ApiErr> {
    let before = store
        .get(ns, name)
        .await
        .ok()
        .flatten()
        .map(|m| phase_of(&m));
    let out = retry_import(store, ns, name, force, interval, attempts).await;
    let id = out.as_ref().ok().and_then(|o| o.import_id.clone());
    hooks
        .audit(record(
            actor,
            "retry",
            ns,
            name,
            id.clone(),
            &out.as_ref().map(|_| ()).map_err(Clone::clone),
        ))
        .await;
    if out.is_ok() {
        hooks
            .phase_changed(PhaseChange {
                import_id: id,
                name: name.to_string(),
                namespace: ns.to_string(),
                from: before.map(|p| p.as_str().to_string()),
                to: ImportPhase::Pending,
                message: None,
            })
            .await;
    }
    out
}

/// One audit entry per Machine a batch created (or failed to create).
pub async fn audit_batch<H: ImportHooks>(hooks: &H, actor: &str, report: &BatchReport) {
    if report.dry_run {
        return;
    }
    for r in &report.results {
        if r.status == "skipped" {
            continue;
        }
        let ok = r.status == "created";
        hooks
            .audit(AuditRecord {
                actor: actor.to_string(),
                action: "batch-create",
                namespace: r.namespace.clone(),
                name: r.name.clone(),
                import_id: r.import_id.clone(),
                outcome: if ok { "success" } else { "failure" },
                detail: r.error.as_ref().map(|e| e.code.clone()),
            })
            .await;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::api::handlers::import_lifecycle::tests::{FakeStore, import_machine};
    use crate::kairon::MachineStatus;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct RecordingHooks {
        pub changes: Mutex<Vec<PhaseChange>>,
        pub audits: Mutex<Vec<AuditRecord>>,
    }
    impl ImportHooks for RecordingHooks {
        async fn phase_changed(&self, change: PhaseChange) {
            self.changes.lock().unwrap().push(change);
        }
        async fn audit(&self, record: AuditRecord) {
            self.audits.lock().unwrap().push(record);
        }
    }

    fn running(mut m: Machine) -> Machine {
        m.status = Some(MachineStatus {
            phase: Some("Running".into()),
            ..Default::default()
        });
        m
    }

    #[test]
    fn detects_only_real_changes() {
        let mut m = import_machine("web01");
        // no annotation yet: first observation reports from=None
        let c = detect_change(&m).unwrap();
        assert_eq!((c.from.clone(), c.to), (None, ImportPhase::Pending));
        m.metadata
            .annotations
            .as_mut()
            .unwrap()
            .insert(PHASE_ANNOTATION.into(), "pending".into());
        assert!(detect_change(&m).is_none());
        let m = running(m);
        let c = detect_change(&m).unwrap();
        assert_eq!(c.from.as_deref(), Some("pending"));
        assert_eq!(c.to, ImportPhase::Running);
        assert_eq!(c.import_id.as_deref(), Some("imp-00000001"));
        // non-imports never emit
        let plain = Machine::new("x", Default::default());
        assert!(detect_change(&plain).is_none());
    }

    #[test]
    fn payload_carries_import_id_and_phases() {
        let c = PhaseChange {
            import_id: Some("imp-1".into()),
            name: "web01".into(),
            namespace: "prod".into(),
            from: Some("pending".into()),
            to: ImportPhase::Failed,
            message: Some("digest mismatch".into()),
        };
        let p = payload_for(&c);
        assert_eq!(p.event, "import.phase_changed");
        assert_eq!(p.data["import_id"], "imp-1");
        assert_eq!(p.data["phase"], "failed");
        assert_eq!(p.data["previous_phase"], "pending");
        assert_eq!(p.data["message"], "digest mismatch");
    }

    #[test]
    fn subscription_matching() {
        assert!(subscribes("vm.created, import.phase_changed"));
        assert!(subscribes("import.*"));
        assert!(subscribes("*"));
        assert!(!subscribes("vm.created,backup.failed"));
        assert!(!subscribes(""));
    }

    fn cm(name: &str, url: &str, events: &str, active: Option<&str>) -> ConfigMap {
        let mut data = BTreeMap::from([
            ("name".to_string(), name.to_string()),
            ("url".to_string(), url.to_string()),
            ("events".to_string(), events.to_string()),
        ]);
        if let Some(a) = active {
            data.insert("active".into(), a.into());
        }
        ConfigMap {
            data: Some(data),
            ..Default::default()
        }
    }

    #[test]
    fn webhook_targets_filter_inactive_unsubscribed_and_private() {
        let cms = vec![
            cm(
                "ok",
                "https://example.com/hook",
                "import.phase_changed",
                Some("true"),
            ),
            cm(
                "off",
                "https://example.com/hook2",
                "import.phase_changed",
                Some("false"),
            ),
            cm("other", "https://example.com/hook3", "vm.created", None),
            cm(
                "internal",
                "https://127.0.0.1/hook",
                "import.phase_changed",
                None,
            ),
            cm(
                "plain",
                "http://example.com/hook",
                "import.phase_changed",
                None,
            ),
        ];
        let t = webhook_targets(&cms);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].0.name, "ok");
    }

    #[test]
    fn audit_event_is_tagged_with_import_id() {
        let ev = audit_event(&AuditRecord {
            actor: "alice".into(),
            action: "delete",
            namespace: "prod".into(),
            name: "web01".into(),
            import_id: Some("imp-00000001".into()),
            outcome: "success",
            detail: None,
        });
        assert_eq!(ev.labels["import_id"], "imp-00000001");
        assert_eq!(ev.actor, "alice");
        assert_eq!(ev.action, "import.delete");
        assert_eq!(ev.target, "prod/web01");
        assert_eq!(ev.vm_name.as_deref(), Some("web01"));
        assert!(ev.message.contains("imp-00000001"));
    }

    #[tokio::test]
    async fn delete_and_retry_emit_audit_and_events() {
        let failed = {
            let mut m = import_machine("web01");
            m.status = Some(MachineStatus {
                phase: Some("Failed".into()),
                ..Default::default()
            });
            m
        };
        let store = FakeStore::with(vec![failed]);
        let hooks = RecordingHooks::default();
        retry_with_hooks(
            &store,
            &hooks,
            "alice",
            "prod",
            "web01",
            false,
            std::time::Duration::from_millis(1),
            3,
        )
        .await
        .unwrap();
        {
            let ch = hooks.changes.lock().unwrap();
            assert_eq!(ch[0].from.as_deref(), Some("failed"));
            assert_eq!(ch[0].to, ImportPhase::Pending);
            let au = hooks.audits.lock().unwrap();
            assert_eq!(
                (au[0].action, au[0].outcome, au[0].actor.as_str()),
                ("retry", "success", "alice")
            );
            assert_eq!(au[0].import_id.as_deref(), Some("imp-00000001"));
        }
        delete_with_hooks(&store, &hooks, "bob", "prod", "web01")
            .await
            .unwrap();
        assert_eq!(
            hooks.changes.lock().unwrap()[1].to,
            ImportPhase::Terminating
        );
        // a failed delete is audited as a failure and emits no event
        let err = delete_with_hooks(&store, &hooks, "bob", "prod", "web01")
            .await
            .unwrap_err();
        assert_eq!(err.code, "NOT_FOUND");
        assert_eq!(hooks.changes.lock().unwrap().len(), 2);
        let au = hooks.audits.lock().unwrap();
        assert_eq!(au.last().unwrap().outcome, "failure");
        assert_eq!(au.last().unwrap().detail.as_deref(), Some("NOT_FOUND"));
    }

    #[tokio::test]
    async fn batch_audit_skips_dry_runs_and_skipped_rows() {
        use super::super::import_plans::{BatchReport, ItemError, ItemResult};
        let item = |name: &str, status: &'static str| ItemResult {
            index: 0,
            name: name.into(),
            namespace: "prod".into(),
            wave: 1,
            status,
            import_id: Some(format!("imp-{name}")),
            error: (status == "failed").then(|| ItemError {
                code: "ALREADY_EXISTS".into(),
                message: "x".into(),
            }),
        };
        let mut rep = BatchReport {
            dry_run: false,
            total: 3,
            succeeded: 1,
            failed: 1,
            skipped: 1,
            concurrency: 4,
            waves: vec![],
            results: vec![
                item("a", "created"),
                item("b", "failed"),
                item("c", "skipped"),
            ],
        };
        let hooks = RecordingHooks::default();
        audit_batch(&hooks, "carol", &rep).await;
        {
            let au = hooks.audits.lock().unwrap();
            assert_eq!(au.len(), 2);
            assert_eq!(au[0].import_id.as_deref(), Some("imp-a"));
            assert_eq!(au[1].outcome, "failure");
        }
        rep.dry_run = true;
        audit_batch(&hooks, "carol", &rep).await;
        assert_eq!(hooks.audits.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn observe_emits_once_then_goes_quiet() {
        let mut m = import_machine("web01");
        m.metadata
            .annotations
            .as_mut()
            .unwrap()
            .insert(PHASE_ANNOTATION.into(), "pending".into());
        let store = FakeStore::with(vec![m.clone()]);
        let hooks = RecordingHooks::default();
        observe(&store, &hooks, &m).await;
        assert!(hooks.changes.lock().unwrap().is_empty());

        // The controller reports Running: the store holds the new status.
        let live = running(store.get("prod", "web01").await.unwrap().unwrap());
        store
            .machines
            .lock()
            .unwrap()
            .insert(("prod".into(), "web01".into()), live.clone());
        observe(&store, &hooks, &live).await;
        assert_eq!(hooks.changes.lock().unwrap().len(), 1);
        // The annotation was written, so a second read emits nothing.
        let again = store.get("prod", "web01").await.unwrap().unwrap();
        assert_eq!(
            again.metadata.annotations.as_ref().unwrap()[PHASE_ANNOTATION],
            "running"
        );
        observe(&store, &hooks, &again).await;
        assert_eq!(hooks.changes.lock().unwrap().len(), 1);
    }
}
