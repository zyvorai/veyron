// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Proposals: every change an AI wants to make, waiting for a human.
//!
//! A proposal is a list of API calls plus what they will do (diff, blast radius),
//! how to tell they worked (verify checks), and how to undo them (inverse steps,
//! or a pre-change snapshot). Approving runs the steps through the real router as
//! the approver, so RBAC and audit apply exactly as if they had clicked the buttons.

use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, ListParams, ObjectMeta, PostParams};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::exec;
use crate::api::auth_context::role_rank;
use crate::api::http_server::web::ApiRole;

pub const PROPOSAL_TYPE: &str = "ai-proposal";
const CM_PREFIX: &str = "veyron-ai-proposal-";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalStep {
    pub method: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    pub summary: String,
    /// `readonly` | `write` | `admin`
    pub min_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifyCheck {
    pub description: String,
    pub path: String,
    /// JSON pointer into the response `data` (envelope already unwrapped).
    pub pointer: String,
    /// Passes when the value at `pointer` equals any of these (case-insensitive).
    /// Empty means "the GET succeeds"; `"<absent>"` passes on 404.
    #[serde(default)]
    pub expect: Vec<String>,
    #[serde(default = "default_verify_timeout")]
    pub timeout_secs: u64,
}

fn default_verify_timeout() -> u64 {
    120
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BlastRadius {
    #[serde(default)]
    pub vms: Vec<String>,
    #[serde(default)]
    pub namespaces: Vec<String>,
    #[serde(default)]
    pub nodes: Vec<String>,
    #[serde(default)]
    pub downtime: bool,
    #[serde(default)]
    pub reversible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_cost_delta: Option<f64>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffLine {
    pub target: String,
    pub field: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotTarget {
    pub namespace: String,
    pub vm_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepResult {
    pub step: String,
    pub status: u16,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    RolledBack,
    Rejected,
    /// Nobody decided within `VEYRON_AI_PROPOSAL_TTL_HOURS`; the cluster may have moved on.
    Expired,
}

/// How long a proposal may wait for a decision (default 72 h).
pub fn ttl_hours() -> i64 {
    std::env::var("VEYRON_AI_PROPOSAL_TTL_HOURS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|h: &i64| *h > 0)
        .unwrap_or(72)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub rationale: String,
    /// `assistant` | `mcp` | `incident` | `predictive` | `policy` | `intent`
    pub source: String,
    pub status: ProposalStatus,
    pub steps: Vec<ProposalStep>,
    #[serde(default)]
    pub diff: Vec<DiffLine>,
    #[serde(default)]
    pub blast_radius: BlastRadius,
    /// VMs that can be snapshotted before running, for rollback.
    #[serde(default)]
    pub snapshot_targets: Vec<SnapshotTarget>,
    #[serde(default)]
    pub verify: Vec<VerifyCheck>,
    /// Steps that undo this change if verification fails.
    #[serde(default)]
    pub rollback: Vec<ProposalStep>,
    pub proposed_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub results: Vec<StepResult>,
    #[serde(default)]
    pub log: Vec<String>,
}

pub fn role_from_str(s: &str) -> ApiRole {
    match s {
        "admin" => ApiRole::Admin,
        "write" => ApiRole::Write,
        _ => ApiRole::ReadOnly,
    }
}

pub fn role_str(r: &ApiRole) -> &'static str {
    match r {
        ApiRole::Admin => "admin",
        ApiRole::Write => "write",
        ApiRole::ReadOnly => "readonly",
    }
}

impl Proposal {
    pub fn new(title: impl Into<String>, source: &str, proposed_by: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: new_id(),
            title: title.into(),
            rationale: String::new(),
            source: source.to_string(),
            status: ProposalStatus::Pending,
            steps: Vec::new(),
            diff: Vec::new(),
            blast_radius: BlastRadius::default(),
            snapshot_targets: Vec::new(),
            verify: Vec::new(),
            rollback: Vec::new(),
            proposed_by: proposed_by.to_string(),
            decided_by: None,
            created_at: now.clone(),
            updated_at: now,
            results: Vec::new(),
            log: Vec::new(),
        }
    }

    /// The approver needs the highest role any step requires.
    pub fn required_role(&self) -> ApiRole {
        self.steps
            .iter()
            .map(|s| role_from_str(&s.min_role))
            .max_by_key(role_rank)
            .unwrap_or(ApiRole::Write)
            .max_by_rank(ApiRole::Write)
    }

    /// Pending for longer than the TTL.
    pub fn is_stale(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        self.status == ProposalStatus::Pending
            && chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .is_ok_and(|t| now.signed_duration_since(t) > chrono::Duration::hours(ttl_hours()))
    }

    fn note(&mut self, msg: impl Into<String>) {
        let line = format!("{} {}", chrono::Utc::now().format("%H:%M:%S"), msg.into());
        self.log.push(line);
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

trait MaxByRank {
    fn max_by_rank(self, other: ApiRole) -> ApiRole;
}

impl MaxByRank for ApiRole {
    fn max_by_rank(self, other: ApiRole) -> ApiRole {
        if role_rank(&self) >= role_rank(&other) {
            self
        } else {
            other
        }
    }
}

pub fn new_id() -> String {
    use rand::Rng;
    let suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(6)
        .map(char::from)
        .collect::<String>()
        .to_lowercase();
    format!("{}-{suffix}", chrono::Utc::now().format("%y%m%d%H%M%S"))
}

/// Does `value` satisfy the check's expectations?
pub fn verify_matches(check: &VerifyCheck, status: u16, data: &Value) -> bool {
    if check.expect.iter().any(|e| e == "<absent>") {
        return status == 404;
    }
    if !(200..300).contains(&status) {
        return false;
    }
    if check.expect.is_empty() {
        return true;
    }
    let got = match data.pointer(&check.pointer) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => return false,
        Some(v) => v.to_string(),
    };
    check.expect.iter().any(|e| e.eq_ignore_ascii_case(&got))
}

// ── Storage (one ConfigMap per proposal) ─────────────────────────

fn to_cm(p: &Proposal, namespace: &str) -> anyhow::Result<ConfigMap> {
    let mut labels = BTreeMap::new();
    labels.insert("veyron.io/type".to_string(), PROPOSAL_TYPE.to_string());
    labels.insert(
        "veyron.io/proposal-status".to_string(),
        serde_json::to_value(&p.status)?
            .as_str()
            .unwrap_or("pending")
            .to_string(),
    );
    let mut data = BTreeMap::new();
    data.insert("proposal.json".to_string(), serde_json::to_string(p)?);
    Ok(ConfigMap {
        metadata: ObjectMeta {
            name: Some(format!("{CM_PREFIX}{}", p.id)),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    })
}

fn from_cm(cm: &ConfigMap) -> Option<Proposal> {
    serde_json::from_str(cm.data.as_ref()?.get("proposal.json")?).ok()
}

pub async fn save(client: &kube::Client, namespace: &str, p: &Proposal) -> anyhow::Result<()> {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    let cm = to_cm(p, namespace)?;
    let name = cm.metadata.name.clone().unwrap_or_default();
    match api.get_opt(&name).await? {
        Some(existing) => {
            let mut cm = cm;
            cm.metadata.resource_version = existing.metadata.resource_version;
            api.replace(&name, &PostParams::default(), &cm).await?;
        }
        None => {
            api.create(&PostParams::default(), &cm).await?;
        }
    }
    Ok(())
}

pub async fn load(
    client: &kube::Client,
    namespace: &str,
    id: &str,
) -> anyhow::Result<Option<Proposal>> {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    Ok(api
        .get_opt(&format!("{CM_PREFIX}{id}"))
        .await?
        .as_ref()
        .and_then(from_cm))
}

pub async fn list(client: &kube::Client, namespace: &str) -> anyhow::Result<Vec<Proposal>> {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    let lp = ListParams::default().labels(&format!("veyron.io/type={PROPOSAL_TYPE}"));
    let mut out: Vec<Proposal> = api
        .list(&lp)
        .await?
        .items
        .iter()
        .filter_map(from_cm)
        .collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

/// Mark proposals that waited longer than the TTL as expired.
pub async fn expire_stale(client: &kube::Client, namespace: &str) {
    let Ok(all) = list(client, namespace).await else {
        return;
    };
    let now = chrono::Utc::now();
    for mut p in all.into_iter().filter(|p| p.is_stale(now)) {
        p.status = ProposalStatus::Expired;
        p.note(format!(
            "Expired after {} h without a decision",
            ttl_hours()
        ));
        if let Err(e) = save(client, namespace, &p).await {
            log::warn!("expiring proposal {}: {e}", p.id);
        }
    }
}

/// Keep the newest `keep` finished proposals; pending ones are never pruned.
pub async fn prune(client: &kube::Client, namespace: &str, keep: usize) {
    let Ok(all) = list(client, namespace).await else {
        return;
    };
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    for p in all
        .iter()
        .filter(|p| p.status != ProposalStatus::Pending && p.status != ProposalStatus::Running)
        .skip(keep)
    {
        let _ = api
            .delete(&format!("{CM_PREFIX}{}", p.id), &Default::default())
            .await;
    }
}

// ── Execution ─────────────────────────────────────────────────────

/// Run an approved proposal: optional pre-snapshots, steps, verification, rollback.
/// Pure with respect to storage so it can be tested against a fake router.
pub async fn execute(mut p: Proposal, role: &ApiRole, subject: &str, snapshot: bool) -> Proposal {
    p.status = ProposalStatus::Running;
    p.decided_by = Some(subject.to_string());
    p.note(format!("Approved by {subject}"));

    if snapshot {
        let short = p.id.rsplit('-').next().unwrap_or("x").to_string();
        for t in p.snapshot_targets.iter_mut() {
            let name = format!("ai-pre-{}-{short}", t.vm_name)
                .chars()
                .take(63)
                .collect::<String>();
            let path = format!("/api/v1/snapshots/{}/{}/create", t.namespace, t.vm_name);
            let body = serde_json::json!({ "snapshot_name": name });
            match exec::call(role, subject, "POST", &path, Some(&body)).await {
                Ok(r) if r.ok() => t.snapshot_name = Some(name),
                Ok(r) => p.log.push(format!(
                    "Pre-snapshot of {}/{} failed: {}",
                    t.namespace,
                    t.vm_name,
                    r.error_message()
                )),
                Err(e) => p.log.push(format!("Pre-snapshot error: {e}")),
            }
        }
        let taken = p
            .snapshot_targets
            .iter()
            .filter(|t| t.snapshot_name.is_some())
            .count();
        p.note(format!("Took {taken} pre-change snapshot(s)"));
    }

    let steps = p.steps.clone();
    for step in &steps {
        let res = exec::call(role, subject, &step.method, &step.path, step.body.as_ref()).await;
        let (status, ok, message) = match res {
            Ok(r) => (
                r.status,
                r.ok(),
                if r.ok() {
                    "ok".into()
                } else {
                    r.error_message()
                },
            ),
            Err(e) => (0, false, e.to_string()),
        };
        p.results.push(StepResult {
            step: step.summary.clone(),
            status,
            ok,
            message: message.clone(),
        });
        if !ok {
            p.note(format!("Step failed: {} ({message})", step.summary));
            return rollback(p, role, subject).await;
        }
        p.note(format!("Done: {}", step.summary));
    }

    for check in p.verify.clone() {
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(check.timeout_secs);
        let mut passed = false;
        loop {
            if let Ok(r) = exec::call(role, subject, "GET", &check.path, None).await {
                if verify_matches(&check, r.status, r.data()) {
                    passed = true;
                    break;
                }
            }
            if std::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
        if !passed {
            p.note(format!("Verification failed: {}", check.description));
            return rollback(p, role, subject).await;
        }
        p.note(format!("Verified: {}", check.description));
    }

    p.status = ProposalStatus::Succeeded;
    p.note("Change applied and verified");
    p
}

async fn rollback(mut p: Proposal, role: &ApiRole, subject: &str) -> Proposal {
    if !p.rollback.is_empty() {
        let mut all_ok = true;
        for step in p.rollback.clone() {
            let ok = matches!(
                exec::call(role, subject, &step.method, &step.path, step.body.as_ref()).await,
                Ok(ref r) if r.ok()
            );
            all_ok &= ok;
            p.note(format!(
                "Rollback {}: {}",
                if ok { "done" } else { "failed" },
                step.summary
            ));
        }
        p.status = if all_ok {
            ProposalStatus::RolledBack
        } else {
            ProposalStatus::Failed
        };
        return p;
    }
    let snaps: Vec<SnapshotTarget> = p
        .snapshot_targets
        .iter()
        .filter(|t| t.snapshot_name.is_some())
        .cloned()
        .collect();
    if snaps.is_empty() {
        p.status = ProposalStatus::Failed;
        p.note("No rollback available (no inverse steps, no pre-change snapshot)");
        return p;
    }
    // Restore is an admin route; it only returns the VM to the state captured
    // moments before this approved change, so it runs with the admin key.
    let mut all_ok = true;
    for t in snaps {
        let snap = t.snapshot_name.unwrap_or_default();
        let path = format!("/api/v1/snapshots/{}/{}/restore", t.namespace, snap);
        let ok = matches!(
            exec::call(&ApiRole::Admin, subject, "POST", &path, Some(&serde_json::json!({}))).await,
            Ok(ref r) if r.ok()
        );
        all_ok &= ok;
        p.note(format!(
            "Restore {}/{} from {snap}: {}",
            t.namespace,
            t.vm_name,
            if ok { "started" } else { "failed" }
        ));
    }
    p.status = if all_ok {
        ProposalStatus::RolledBack
    } else {
        ProposalStatus::Failed
    };
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(role: &str) -> ProposalStep {
        ProposalStep {
            method: "POST".into(),
            path: "/api/v1/x".into(),
            body: None,
            summary: "x".into(),
            min_role: role.into(),
        }
    }

    #[test]
    fn required_role_is_highest_step_and_at_least_write() {
        let mut p = Proposal::new("t", "assistant", "alice");
        p.steps = vec![step("readonly")];
        assert_eq!(p.required_role(), ApiRole::Write);
        p.steps.push(step("admin"));
        assert_eq!(p.required_role(), ApiRole::Admin);
    }

    #[test]
    fn pending_proposals_go_stale_after_ttl() {
        let now = chrono::Utc::now();
        let mut p = Proposal::new("t", "assistant", "me");
        assert!(!p.is_stale(now));
        p.created_at = (now - chrono::Duration::hours(ttl_hours() + 1)).to_rfc3339();
        assert!(p.is_stale(now));
        p.status = ProposalStatus::Rejected;
        assert!(!p.is_stale(now));
    }

    #[test]
    fn verify_matching_rules() {
        let c = VerifyCheck {
            description: "stopped".into(),
            path: "/x".into(),
            pointer: "/status".into(),
            expect: vec!["Stopped".into()],
            timeout_secs: 1,
        };
        assert!(verify_matches(
            &c,
            200,
            &serde_json::json!({"status": "stopped"})
        ));
        assert!(!verify_matches(
            &c,
            200,
            &serde_json::json!({"status": "Running"})
        ));
        assert!(!verify_matches(
            &c,
            500,
            &serde_json::json!({"status": "Stopped"})
        ));
        let gone = VerifyCheck {
            expect: vec!["<absent>".into()],
            ..c.clone()
        };
        assert!(verify_matches(&gone, 404, &Value::Null));
        let any = VerifyCheck {
            expect: vec![],
            ..c
        };
        assert!(verify_matches(&any, 200, &Value::Null));
    }

    #[test]
    fn configmap_roundtrip_keeps_everything() {
        let mut p = Proposal::new("Stop web", "assistant", "alice");
        p.steps = vec![step("write")];
        p.blast_radius.vms = vec!["default/web".into()];
        let cm = to_cm(&p, "veyron-system").unwrap();
        assert_eq!(
            cm.metadata.labels.as_ref().unwrap()["veyron.io/proposal-status"],
            "pending"
        );
        assert_eq!(from_cm(&cm).unwrap(), p);
    }

    #[tokio::test]
    async fn execute_runs_steps_verifies_and_rolls_back() {
        super::super::test_router::install();
        // Happy path: step succeeds, verify reads the fake VM status.
        let mut p = Proposal::new("Stop", "assistant", "alice");
        p.steps = vec![ProposalStep {
            method: "POST".into(),
            path: "/api/v1/vms/default/web/stop".into(),
            body: None,
            summary: "Stop default/web".into(),
            min_role: "write".into(),
        }];
        p.verify = vec![VerifyCheck {
            description: "web is stopped".into(),
            path: "/api/v1/vms/default/web".into(),
            pointer: "/status".into(),
            expect: vec!["Stopped".into()],
            timeout_secs: 1,
        }];
        let done = execute(p.clone(), &ApiRole::Write, "alice", false).await;
        assert_eq!(done.status, ProposalStatus::Succeeded, "{:?}", done.log);
        assert_eq!(done.decided_by.as_deref(), Some("alice"));

        // Failing step triggers the inverse rollback step.
        let mut bad = p;
        bad.steps[0].path = "/api/v1/vms/default/web/explode".into();
        bad.rollback = vec![ProposalStep {
            method: "POST".into(),
            path: "/api/v1/vms/default/web/start".into(),
            body: None,
            summary: "Start default/web".into(),
            min_role: "write".into(),
        }];
        let rolled = execute(bad, &ApiRole::Write, "alice", false).await;
        assert_eq!(
            rolled.status,
            ProposalStatus::RolledBack,
            "{:?}",
            rolled.log
        );
        assert!(!rolled.results[0].ok);
    }
}
