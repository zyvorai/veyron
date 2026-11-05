// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Plan → confirm → execute for agent-proposed VM/node operations.
//!
//! The LLM loop can only *propose* a [`Plan`]; nothing mutates the cluster until a
//! caller explicitly executes it. Execution re-derives the required role of every
//! step from `auth_context::min_role_for_route` (so the agent can never exceed the
//! caller), requires typed confirmation for destructive steps, and plans are
//! single-use, expiring ConfigMaps (`veyron.io/type=copilot-plan`).

use std::collections::BTreeMap;

use axum::http::Method;
use serde::{Deserialize, Serialize};

use crate::api::auth_context::{AuthContext, min_role_for_route};
use crate::api::http_server::web::ApiRole;
use crate::kube::KubeClient;

pub const MAX_STEPS: usize = 25;
pub const DEFAULT_TTL_SECS: i64 = 900;
/// Per-target loop guard: after a step succeeds (or is still running) on a VM/node, further
/// plan steps on the same target are refused for this long. `0` disables it.
pub const DEFAULT_COOLDOWN_SECS: i64 = 60;
/// Finished or expired plans stay readable this long past `expires_at`, then are pruned.
pub const PRUNE_GRACE_SECS: i64 = 3600;
pub const PLAN_LABEL_KEY: &str = "veyron.io/type";
pub const PLAN_LABEL_VALUE: &str = "copilot-plan";

pub const VM_ACTIONS: [&str; 8] = [
    "start", "stop", "restart", "pause", "unpause", "migrate", "delete", "snapshot",
];
pub const NODE_ACTIONS: [&str; 2] = ["cordon", "uncordon"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepResult {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanStep {
    pub action: String,
    /// `vm` or `node`.
    pub kind: String,
    #[serde(default)]
    pub namespace: String,
    pub name: String,
    pub destructive: bool,
    /// Informational; execution re-derives the role from the route.
    pub min_role: String,
    pub preview: String,
    /// Why this step cannot run (checked at proposal time and again at execution).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<StepResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plan {
    pub id: String,
    pub prompt: String,
    pub created_by: String,
    pub created_at: String,
    pub expires_at: String,
    /// pending | executing | executed | failed | cancelled
    pub status: String,
    pub min_role: String,
    pub steps: Vec<PlanStep>,
    /// When execution was claimed; drives the per-target cooldown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
}

pub fn cooldown_secs() -> i64 {
    std::env::var("VEYRON_AI_PLAN_COOLDOWN_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|v: &i64| *v >= 0)
        .unwrap_or(DEFAULT_COOLDOWN_SECS)
}

pub fn ttl_secs() -> i64 {
    std::env::var("VEYRON_AI_PLAN_TTL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|v: &i64| *v > 0)
        .unwrap_or(DEFAULT_TTL_SECS)
}

fn role_str(r: &ApiRole) -> &'static str {
    match r {
        ApiRole::ReadOnly => "readonly",
        ApiRole::Write => "write",
        ApiRole::Admin => "admin",
    }
}

fn role_from_str(s: &str) -> ApiRole {
    match s {
        "admin" => ApiRole::Admin,
        "write" => ApiRole::Write,
        _ => ApiRole::ReadOnly,
    }
}

/// HTTP route equivalent to a step; used for role derivation only.
pub fn route_for(action: &str, kind: &str, ns: &str, name: &str) -> (Method, String) {
    match (kind, action) {
        ("node", a) => (Method::POST, format!("/api/v1/nodes/{name}/{a}")),
        ("vm", "delete") => (Method::DELETE, format!("/api/v1/vms/{ns}/{name}")),
        ("vm", "snapshot") => (
            Method::POST,
            format!("/api/v1/snapshots/{ns}/{name}/create"),
        ),
        (_, a) => (Method::POST, format!("/api/v1/vms/{ns}/{name}/{a}")),
    }
}

pub fn required_role(step: &PlanStep) -> ApiRole {
    let (method, path) = route_for(&step.action, &step.kind, &step.namespace, &step.name);
    min_role_for_route(&method, &path)
}

fn rank(r: &ApiRole) -> u8 {
    match r {
        ApiRole::ReadOnly => 0,
        ApiRole::Write => 1,
        ApiRole::Admin => 2,
    }
}

fn max_role(steps: &[PlanStep]) -> ApiRole {
    steps
        .iter()
        .map(required_role)
        .max_by_key(rank)
        .unwrap_or(ApiRole::Write)
}

fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 253
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
}

/// Build plan steps without touching the cluster (validation + role derivation).
pub fn build_steps(
    kind: &str,
    action: &str,
    namespace: &str,
    names: &[String],
) -> Result<Vec<PlanStep>, String> {
    let action = action.trim().to_lowercase();
    let allowed: &[&str] = match kind {
        "vm" => &VM_ACTIONS,
        "node" => &NODE_ACTIONS,
        _ => return Err(format!("unknown kind '{kind}'")),
    };
    if !allowed.contains(&action.as_str()) {
        return Err(format!(
            "unsupported {kind} action '{action}' (allowed: {})",
            allowed.join(", ")
        ));
    }
    if names.is_empty() {
        return Err("at least one target name is required".into());
    }
    if names.len() > MAX_STEPS {
        return Err(format!("too many targets ({} > {MAX_STEPS})", names.len()));
    }
    if kind == "vm" && !valid_name(namespace) {
        return Err(format!("invalid namespace '{namespace}'"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut steps = Vec::new();
    for n in names {
        let n = n.trim();
        if !valid_name(n) {
            return Err(format!("invalid name '{n}'"));
        }
        if !seen.insert(n.to_string()) {
            continue;
        }
        let mut step = PlanStep {
            action: action.clone(),
            kind: kind.to_string(),
            namespace: if kind == "vm" {
                namespace.to_string()
            } else {
                String::new()
            },
            name: n.to_string(),
            destructive: kind == "vm" && action == "delete",
            min_role: String::new(),
            preview: String::new(),
            blocked: None,
            result: None,
        };
        step.min_role = role_str(&required_role(&step)).to_string();
        step.preview = if kind == "vm" {
            format!("{action} VM {namespace}/{n}")
        } else {
            format!("{action} node {n}")
        };
        steps.push(step);
    }
    Ok(steps)
}

pub fn new_plan(prompt: &str, created_by: &str, steps: Vec<PlanStep>, ttl: i64) -> Plan {
    let now = chrono::Utc::now();
    let id = format!(
        "p-{:x}{:04x}",
        now.timestamp_millis(),
        rand::random::<u16>()
    );
    let min_role = role_str(&max_role(&steps)).to_string();
    Plan {
        id,
        prompt: prompt.chars().take(500).collect(),
        created_by: created_by.to_string(),
        created_at: now.to_rfc3339(),
        expires_at: (now + chrono::Duration::seconds(ttl)).to_rfc3339(),
        status: "pending".into(),
        min_role,
        steps,
        started_at: None,
    }
}

pub fn is_expired(plan: &Plan, now: chrono::DateTime<chrono::Utc>) -> bool {
    chrono::DateTime::parse_from_rfc3339(&plan.expires_at)
        .map(|t| t.with_timezone(&chrono::Utc) <= now)
        .unwrap_or(true)
}

/// Authorization + state checks before executing/cancelling. `Err((http_status, msg))`.
pub fn check_can_act(
    plan: &Plan,
    caller: &AuthContext,
    actor: &str,
    execute: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), (u16, String)> {
    if plan.status != "pending" {
        return Err((409, format!("plan is already {}", plan.status)));
    }
    if is_expired(plan, now) {
        return Err((
            410,
            "plan expired — ask the assistant to propose it again".into(),
        ));
    }
    if !caller.allows(&ApiRole::Write) {
        return Err((403, "Write role required".into()));
    }
    if plan.created_by != actor && !caller.allows(&ApiRole::Admin) {
        return Err((
            403,
            "only the plan's creator or an admin may act on it".into(),
        ));
    }
    if execute {
        for s in &plan.steps {
            let need = required_role(s);
            if !caller.allows(&need) {
                return Err((
                    403,
                    format!(
                        "{} role required for step '{}' — the agent cannot exceed your permissions",
                        role_str(&need),
                        s.preview
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Every destructive step's target name must be typed back by the caller.
pub fn check_confirmation(plan: &Plan, confirm_names: &[String]) -> Result<(), String> {
    let missing: Vec<&str> = plan
        .steps
        .iter()
        .filter(|s| s.destructive)
        .filter(|s| !confirm_names.iter().any(|c| c == &s.name))
        .map(|s| s.name.as_str())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "destructive steps need typed confirmation of: {}",
            missing.join(", ")
        ))
    }
}

/// Probe the cluster for blockers and annotate steps (best-effort, never fails the plan).
pub async fn annotate_blockers(client: &KubeClient, steps: &mut [PlanStep]) {
    for s in steps.iter_mut() {
        if let Some(msg) = blocker_for(client, s).await {
            s.blocked = Some(msg);
        }
    }
}

async fn blocker_for(client: &KubeClient, s: &PlanStep) -> Option<String> {
    use crate::api::vm_backend::{BackendKind, VmBackend};
    if s.kind != "vm" {
        return None;
    }
    let backend = VmBackend::for_client(client);
    if backend.get(&s.namespace, &s.name).await.is_err() {
        return Some("VM not found".into());
    }
    // Only KubeVirt needs the pre-flight gate; Kairon's migrate refuses VFIO devices itself.
    if s.action == "migrate" && backend.kind() == BackendKind::KubeVirt {
        if let Ok(el) = client
            .migration_eligibility(
                &s.namespace,
                &s.name,
                crate::kube::migration_guard::ClusterMigrationCaps::from_env(),
            )
            .await
        {
            if !el.eligible {
                return Some(format!(
                    "migration blocked: {}",
                    el.blockers
                        .iter()
                        .map(|b| b.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }
    }
    None
}

/// Take a snapshot named `<vm>-snap-<timestamp>` through the same code the REST route uses.
async fn snapshot_vm(client: &KubeClient, ns: &str, name: &str) -> anyhow::Result<()> {
    let snap = format!("{name}-snap-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::is_kairon() {
        return crate::api::kairon_ops::create_snapshot(client, ns, name, &snap)
            .await
            .map(|_| ());
    }
    let manager = crate::snapshots::SnapshotManager::from_client(client.client(), ns);
    manager
        .create_snapshot(&crate::snapshots::SnapshotConfig::new(name, &snap))
        .await
        .map(|_| ())
}

async fn run_step(
    client: &KubeClient,
    s: &PlanStep,
    cooldown: Option<(&[Plan], &str)>,
) -> anyhow::Result<()> {
    use crate::api::vm_backend::{BackendKind, MigrateRequest, VmBackend};
    if let Some(b) = blocker_for(client, s).await {
        anyhow::bail!(b);
    }
    if let Some((recent, self_id)) = cooldown {
        if let Some(b) = cooldown_blocker(recent, s, chrono::Utc::now(), cooldown_secs(), self_id) {
            anyhow::bail!(b);
        }
    }
    let backend = VmBackend::for_client(client);
    let (ns, name) = (s.namespace.as_str(), s.name.as_str());
    match (s.kind.as_str(), s.action.as_str()) {
        ("vm", "start") => backend.start(ns, name).await,
        ("vm", "stop") => backend.stop(ns, name).await,
        ("vm", "restart") => backend.restart(ns, name).await,
        ("vm", "pause") => backend.pause(ns, name).await,
        ("vm", "unpause") => backend.unpause(ns, name).await,
        ("vm", "migrate") if backend.kind() == BackendKind::KubeVirt => {
            client.migrate_vm(ns, name, None).await
        }
        ("vm", "migrate") => backend
            .migrate(ns, name, &MigrateRequest::default())
            .await
            .map(|_| ()),
        ("vm", "delete") => backend.delete(ns, name).await,
        ("vm", "snapshot") => snapshot_vm(client, ns, name).await,
        ("node", "cordon") => client.set_node_schedulable(&s.name, false).await,
        ("node", "uncordon") => client.set_node_schedulable(&s.name, true).await,
        _ => anyhow::bail!("unsupported step {} {}", s.kind, s.action),
    }
}

/// Run all steps in order. Steps are re-checked for blockers right before running.
/// On a failure with `stop_on_error`, remaining steps are marked skipped.
pub async fn execute_steps(
    client: &KubeClient,
    steps: &mut [PlanStep],
    stop_on_error: bool,
    cooldown: Option<(&[Plan], &str)>,
) {
    let mut aborted = false;
    for s in steps.iter_mut() {
        if aborted {
            s.result = Some(StepResult {
                ok: false,
                error: Some("skipped after earlier failure".into()),
            });
            continue;
        }
        match run_step(client, s, cooldown).await {
            Ok(()) => {
                s.result = Some(StepResult {
                    ok: true,
                    error: None,
                })
            }
            Err(e) => {
                s.result = Some(StepResult {
                    ok: false,
                    error: Some(e.to_string()),
                });
                if stop_on_error {
                    aborted = true;
                }
            }
        }
    }
}

pub fn final_status(steps: &[PlanStep]) -> &'static str {
    if steps
        .iter()
        .all(|s| s.result.as_ref().is_some_and(|r| r.ok))
    {
        "executed"
    } else {
        "failed"
    }
}

// ── Storage (ConfigMap per plan) ────────────────────────────────────

fn cm_name(id: &str) -> String {
    format!("veyron-plan-{id}")
}

fn valid_plan_id(id: &str) -> bool {
    id.starts_with("p-") && id.len() <= 32 && id[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn plan_to_cm(plan: &Plan, ns: &str) -> k8s_openapi::api::core::v1::ConfigMap {
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::ObjectMeta;
    ConfigMap {
        metadata: ObjectMeta {
            name: Some(cm_name(&plan.id)),
            namespace: Some(ns.to_string()),
            labels: Some(BTreeMap::from([(
                PLAN_LABEL_KEY.to_string(),
                PLAN_LABEL_VALUE.to_string(),
            )])),
            ..Default::default()
        },
        data: Some(BTreeMap::from([(
            "plan".to_string(),
            serde_json::to_string(plan).unwrap_or_default(),
        )])),
        ..Default::default()
    }
}

pub async fn save(client: &kube::Client, ns: &str, plan: &Plan) -> anyhow::Result<()> {
    use k8s_openapi::api::core::v1::ConfigMap;
    let api: kube::Api<ConfigMap> = kube::Api::namespaced(client.clone(), ns);
    api.create(&Default::default(), &plan_to_cm(plan, ns))
        .await?;
    Ok(())
}

/// Load a plan plus its ConfigMap resourceVersion (for atomic claims).
pub async fn load(
    client: &kube::Client,
    ns: &str,
    id: &str,
) -> anyhow::Result<Option<(Plan, Option<String>)>> {
    use k8s_openapi::api::core::v1::ConfigMap;
    if !valid_plan_id(id) {
        return Ok(None);
    }
    let api: kube::Api<ConfigMap> = kube::Api::namespaced(client.clone(), ns);
    let Some(cm) = api.get_opt(&cm_name(id)).await? else {
        return Ok(None);
    };
    let rv = cm.metadata.resource_version.clone();
    let raw = cm
        .data
        .and_then(|d| d.get("plan").cloned())
        .unwrap_or_default();
    Ok(serde_json::from_str::<Plan>(&raw).ok().map(|p| (p, rv)))
}

/// Replace the plan, optionally conditional on `resource_version` (409 on a lost race).
pub async fn update(
    client: &kube::Client,
    ns: &str,
    plan: &Plan,
    resource_version: Option<String>,
) -> anyhow::Result<()> {
    use k8s_openapi::api::core::v1::ConfigMap;
    let api: kube::Api<ConfigMap> = kube::Api::namespaced(client.clone(), ns);
    let mut cm = plan_to_cm(plan, ns);
    cm.metadata.resource_version = resource_version;
    api.replace(&cm_name(&plan.id), &Default::default(), &cm)
        .await?;
    Ok(())
}

/// One-line description of what a plan does, for audit targets (`start VM ns/a; stop VM ns/b (+2 more)`).
pub fn summary(plan: &Plan) -> String {
    const SHOWN: usize = 3;
    let mut out = plan
        .steps
        .iter()
        .take(SHOWN)
        .map(|s| s.preview.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    if plan.steps.len() > SHOWN {
        out.push_str(&format!(" (+{} more)", plan.steps.len() - SHOWN));
    }
    out
}

/// A plan is prunable once it has been expired for longer than `grace_secs`.
/// Every status qualifies: the plan is single-use and its audit trail lives in the SOC store.
pub fn is_prunable(plan: &Plan, now: chrono::DateTime<chrono::Utc>, grace_secs: i64) -> bool {
    chrono::DateTime::parse_from_rfc3339(&plan.expires_at)
        .map(|e| now.signed_duration_since(e).num_seconds() > grace_secs)
        .unwrap_or(false)
}

// ── Per-target cooldown ─────────────────────────────────────────────

/// Why `step` must wait: another plan acted on the same target within `cooldown` seconds.
/// A step counts if it succeeded or is still running; failed attempts never lock a target.
/// `self_id` is skipped so a plan never blocks itself.
pub fn cooldown_blocker(
    recent: &[Plan],
    step: &PlanStep,
    now: chrono::DateTime<chrono::Utc>,
    cooldown: i64,
    self_id: &str,
) -> Option<String> {
    // Snapshots are non-disruptive: they neither wait on nor lock a target, so
    // "snapshot, then change" works across plans.
    if cooldown <= 0 || step.action == "snapshot" {
        return None;
    }
    for p in recent.iter().filter(|p| p.id != self_id) {
        let Some(t) = p
            .started_at
            .as_deref()
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        else {
            continue;
        };
        let age = now.signed_duration_since(t).num_seconds();
        if !(0..cooldown).contains(&age) {
            continue;
        }
        for s in &p.steps {
            let same = s.kind == step.kind && s.namespace == step.namespace && s.name == step.name;
            let active = s.result.as_ref().is_none_or(|r| r.ok);
            if same && active && s.action != "snapshot" {
                return Some(format!(
                    "cooling down: {} ran on this target {age}s ago (plan {}); retry in {}s",
                    s.action,
                    p.id,
                    cooldown - age
                ));
            }
        }
    }
    None
}

/// Mark steps that are in cooldown as blocked (keeps an existing blocker).
pub fn apply_cooldown(
    steps: &mut [PlanStep],
    recent: &[Plan],
    now: chrono::DateTime<chrono::Utc>,
    cooldown: i64,
    self_id: &str,
) {
    for s in steps.iter_mut() {
        if s.blocked.is_none() {
            s.blocked = cooldown_blocker(recent, s, now, cooldown, self_id);
        }
    }
}

/// Plans whose execution started within the cooldown window (best effort: `[]` on error).
pub async fn recent_started(
    client: &kube::Client,
    ns: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<Plan> {
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::ListParams;
    let cooldown = cooldown_secs();
    if cooldown <= 0 {
        return Vec::new();
    }
    let api: kube::Api<ConfigMap> = kube::Api::namespaced(client.clone(), ns);
    let lp = ListParams::default().labels(&format!("{PLAN_LABEL_KEY}={PLAN_LABEL_VALUE}"));
    let Ok(list) = api.list(&lp).await else {
        return Vec::new();
    };
    list.items
        .into_iter()
        .filter_map(|cm| cm.data?.get("plan").cloned())
        .filter_map(|raw| serde_json::from_str::<Plan>(&raw).ok())
        .filter(|p| {
            p.started_at
                .as_deref()
                .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                .is_some_and(|t| {
                    let age = now.signed_duration_since(t).num_seconds();
                    (0..cooldown).contains(&age)
                })
        })
        .collect()
}

/// Delete plan ConfigMaps that expired more than `PRUNE_GRACE_SECS` ago. Best effort:
/// returns how many were deleted and never fails the caller.
pub async fn prune(client: &kube::Client, ns: &str, now: chrono::DateTime<chrono::Utc>) -> usize {
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::{DeleteParams, ListParams};
    let api: kube::Api<ConfigMap> = kube::Api::namespaced(client.clone(), ns);
    let lp = ListParams::default().labels(&format!("{PLAN_LABEL_KEY}={PLAN_LABEL_VALUE}"));
    let Ok(list) = api.list(&lp).await else {
        return 0;
    };
    let mut deleted = 0;
    for cm in list.items {
        let Some(name) = cm.metadata.name.clone() else {
            continue;
        };
        let plan = cm
            .data
            .as_ref()
            .and_then(|d| d.get("plan"))
            .and_then(|raw| serde_json::from_str::<Plan>(raw).ok());
        let stale = match plan {
            Some(p) => is_prunable(&p, now, PRUNE_GRACE_SECS),
            // Unreadable plan data can never be executed; drop it once it is old enough.
            None => cm.metadata.creation_timestamp.as_ref().is_some_and(|t| {
                now.signed_duration_since(t.0).num_seconds() > ttl_secs() + PRUNE_GRACE_SECS
            }),
        };
        if stale && api.delete(&name, &DeleteParams::default()).await.is_ok() {
            deleted += 1;
        }
    }
    deleted
}

pub fn role_label(r: &ApiRole) -> &'static str {
    role_str(r)
}

pub fn parse_role(s: &str) -> ApiRole {
    role_from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn ctx(role: ApiRole) -> AuthContext {
        AuthContext {
            role,
            subject: "tester".into(),
        }
    }

    #[test]
    fn builds_steps_and_derives_roles() {
        let s = build_steps("vm", "restart", "prod", &names(&["a", "b", "a"])).unwrap();
        assert_eq!(s.len(), 2, "duplicates collapse");
        assert!(s.iter().all(|x| x.min_role == "write" && !x.destructive));
        let d = build_steps("vm", "delete", "prod", &names(&["a"])).unwrap();
        assert!(d[0].destructive);
        let n = build_steps("node", "cordon", "", &names(&["n1"])).unwrap();
        assert_eq!(n[0].min_role, "admin");
        assert_eq!(new_plan("p", "u", n, 60).min_role, "admin");
    }

    #[test]
    fn rejects_bad_input() {
        assert!(build_steps("vm", "format", "ns", &names(&["a"])).is_err());
        assert!(build_steps("vm", "start", "ns", &[]).is_err());
        assert!(build_steps("vm", "start", "Bad NS", &names(&["a"])).is_err());
        assert!(build_steps("vm", "start", "ns", &names(&["a;rm"])).is_err());
        let many: Vec<String> = (0..26).map(|i| format!("vm{i}")).collect();
        assert!(build_steps("vm", "stop", "ns", &many).is_err());
        assert!(build_steps("pod", "stop", "ns", &names(&["a"])).is_err());
    }

    #[test]
    fn caller_cannot_exceed_own_role() {
        let now = chrono::Utc::now();
        let plan = new_plan(
            "cordon",
            "alice",
            build_steps("node", "cordon", "", &names(&["n1"])).unwrap(),
            60,
        );
        let e = check_can_act(&plan, &ctx(ApiRole::Write), "alice", true, now).unwrap_err();
        assert_eq!(e.0, 403);
        assert!(check_can_act(&plan, &ctx(ApiRole::Admin), "alice", true, now).is_ok());
        assert_eq!(
            check_can_act(&plan, &ctx(ApiRole::ReadOnly), "alice", true, now)
                .unwrap_err()
                .0,
            403
        );
    }

    #[test]
    fn only_creator_or_admin_and_single_use_and_expiry() {
        let now = chrono::Utc::now();
        let mut plan = new_plan(
            "x",
            "alice",
            build_steps("vm", "start", "ns", &names(&["a"])).unwrap(),
            60,
        );
        assert_eq!(
            check_can_act(&plan, &ctx(ApiRole::Write), "bob", true, now)
                .unwrap_err()
                .0,
            403
        );
        assert!(check_can_act(&plan, &ctx(ApiRole::Admin), "bob", true, now).is_ok());
        let later = now + chrono::Duration::seconds(120);
        assert_eq!(
            check_can_act(&plan, &ctx(ApiRole::Write), "alice", true, later)
                .unwrap_err()
                .0,
            410
        );
        plan.status = "executed".into();
        assert_eq!(
            check_can_act(&plan, &ctx(ApiRole::Write), "alice", true, now)
                .unwrap_err()
                .0,
            409
        );
    }

    #[test]
    fn destructive_steps_need_typed_names() {
        let plan = new_plan(
            "x",
            "u",
            build_steps("vm", "delete", "ns", &names(&["a", "b"])).unwrap(),
            60,
        );
        assert!(
            check_confirmation(&plan, &names(&["a"]))
                .unwrap_err()
                .contains('b')
        );
        assert!(check_confirmation(&plan, &names(&["a", "b"])).is_ok());
        let safe = new_plan(
            "x",
            "u",
            build_steps("vm", "stop", "ns", &names(&["a"])).unwrap(),
            60,
        );
        assert!(check_confirmation(&safe, &[]).is_ok());
    }

    #[test]
    fn plan_ids_are_validated_and_final_status() {
        assert!(valid_plan_id("p-18c3f0a1b2c30001"));
        assert!(!valid_plan_id("../etc"));
        assert!(!valid_plan_id("p-zz"));
        let mut s = build_steps("vm", "stop", "ns", &names(&["a"])).unwrap();
        s[0].result = Some(StepResult {
            ok: true,
            error: None,
        });
        assert_eq!(final_status(&s), "executed");
        s[0].result = Some(StepResult {
            ok: false,
            error: Some("x".into()),
        });
        assert_eq!(final_status(&s), "failed");
        s[0].result = None;
        assert_eq!(final_status(&s), "failed");
    }

    #[test]
    fn summary_lists_first_steps() {
        let steps = build_steps("vm", "stop", "ns", &names(&["a", "b", "c", "d", "e"])).unwrap();
        let mut p = new_plan("x", "u", steps, 60);
        assert!(summary(&p).ends_with("(+2 more)"));
        assert_eq!(summary(&p).matches("; ").count(), 2);
        p.steps.truncate(1);
        assert_eq!(summary(&p), p.steps[0].preview);
    }

    #[test]
    fn prunable_only_after_expiry_plus_grace() {
        let now = chrono::Utc::now();
        let mut p = new_plan(
            "x",
            "u",
            build_steps("vm", "stop", "ns", &names(&["a"])).unwrap(),
            60,
        );
        p.expires_at = (now - chrono::Duration::seconds(10)).to_rfc3339();
        assert!(!is_prunable(&p, now, 3600), "just expired stays readable");
        p.expires_at = (now - chrono::Duration::seconds(3601)).to_rfc3339();
        assert!(is_prunable(&p, now, 3600));
        p.expires_at = (now + chrono::Duration::seconds(60)).to_rfc3339();
        assert!(!is_prunable(&p, now, 0), "live plans are never pruned");
        p.expires_at = "garbage".into();
        assert!(!is_prunable(&p, now, 0), "unparseable expiry is kept");
    }

    fn started_plan(id: &str, name: &str, ago_secs: i64, ok: Option<bool>) -> Plan {
        let mut p = new_plan(
            "x",
            "u",
            build_steps("vm", "restart", "ns", &names(&[name])).unwrap(),
            60,
        );
        p.id = id.into();
        p.started_at =
            Some((chrono::Utc::now() - chrono::Duration::seconds(ago_secs)).to_rfc3339());
        p.steps[0].result = ok.map(|ok| StepResult { ok, error: None });
        p
    }

    #[test]
    fn cooldown_blocks_the_same_target_only_inside_the_window() {
        let now = chrono::Utc::now();
        let step = build_steps("vm", "stop", "ns", &names(&["a"]))
            .unwrap()
            .remove(0);
        let recent = vec![started_plan("p-1", "a", 10, Some(true))];
        let msg = cooldown_blocker(&recent, &step, now, 60, "p-9").expect("blocked");
        assert!(msg.contains("restart ran on this target"), "{msg}");
        assert!(msg.contains("p-1"), "{msg}");
        // other VM, other namespace, expired window, disabled, and self are all fine
        let other = build_steps("vm", "stop", "ns", &names(&["b"]))
            .unwrap()
            .remove(0);
        assert!(cooldown_blocker(&recent, &other, now, 60, "p-9").is_none());
        let other_ns = build_steps("vm", "stop", "other", &names(&["a"]))
            .unwrap()
            .remove(0);
        assert!(cooldown_blocker(&recent, &other_ns, now, 60, "p-9").is_none());
        let old = vec![started_plan("p-1", "a", 120, Some(true))];
        assert!(cooldown_blocker(&old, &step, now, 60, "p-9").is_none());
        assert!(cooldown_blocker(&recent, &step, now, 0, "p-9").is_none());
        assert!(cooldown_blocker(&recent, &step, now, 60, "p-1").is_none());
    }

    #[test]
    fn snapshot_is_a_write_step_and_never_cooldown_locked() {
        let snap = build_steps("vm", "snapshot", "ns", &names(&["a"]))
            .unwrap()
            .remove(0);
        assert_eq!(snap.min_role, "write");
        assert!(!snap.destructive);
        assert_eq!(snap.preview, "snapshot VM ns/a");
        let now = chrono::Utc::now();
        // a recent restart does not delay a snapshot ...
        let recent = vec![started_plan("p-1", "a", 5, Some(true))];
        assert!(cooldown_blocker(&recent, &snap, now, 60, "p-9").is_none());
        // ... and a recent snapshot does not delay a restart.
        let mut p = started_plan("p-2", "a", 5, Some(true));
        p.steps = vec![snap.clone()];
        p.steps[0].result = Some(StepResult {
            ok: true,
            error: None,
        });
        let restart = build_steps("vm", "restart", "ns", &names(&["a"]))
            .unwrap()
            .remove(0);
        assert!(cooldown_blocker(&[p], &restart, now, 60, "p-9").is_none());
    }

    #[test]
    fn failed_attempts_do_not_lock_but_running_ones_do() {
        let now = chrono::Utc::now();
        let step = build_steps("vm", "stop", "ns", &names(&["a"]))
            .unwrap()
            .remove(0);
        let failed = vec![started_plan("p-1", "a", 5, Some(false))];
        assert!(cooldown_blocker(&failed, &step, now, 60, "p-9").is_none());
        let running = vec![started_plan("p-1", "a", 5, None)];
        assert!(cooldown_blocker(&running, &step, now, 60, "p-9").is_some());
    }

    #[test]
    fn apply_cooldown_keeps_an_existing_blocker() {
        let now = chrono::Utc::now();
        let mut steps = build_steps("vm", "stop", "ns", &names(&["a", "b"])).unwrap();
        steps[1].blocked = Some("VM not found".into());
        steps[0].name = "a".into();
        let recent = vec![
            started_plan("p-1", "a", 5, Some(true)),
            started_plan("p-2", "b", 5, Some(true)),
        ];
        apply_cooldown(&mut steps, &recent, now, 60, "p-9");
        assert!(
            steps[0]
                .blocked
                .as_deref()
                .unwrap()
                .starts_with("cooling down")
        );
        assert_eq!(steps[1].blocked.as_deref(), Some("VM not found"));
    }
}
