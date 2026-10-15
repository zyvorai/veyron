// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Agent Lab (Kairon backend): branch a sandbox into N live copies and keep one,
//! checkpoint before every exec with undo/pin, wake an idle-suspended sandbox on
//! connect, and keep a per-sandbox run ledger.
//!
//! Everything here is best-effort around the exec path: a failed checkpoint or ledger
//! write is logged and never blocks the command the agent asked for.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use chrono::Utc;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    ANN_CREATED, ANN_EXPIRES, ANN_INTERNET, ANN_OWNER, LABEL_SANDBOX, LABEL_STATE, STATE_CLAIMED,
    Sandbox, env_u64, namespace,
};
use crate::api::kairon_ops;
use crate::kairon::{LABEL_FORKED_FROM, LABEL_MANAGED_BY, Machine, POWER_RUNNING};
use crate::kube::KubeClient;

/// Comma-separated checkpoint names exempt from ring-buffer pruning.
pub const ANN_PINNED: &str = "veyron.io/sandbox-pinned";
const LEDGER_TYPE: &str = "sandbox-run";
const LABEL_TYPE: &str = "veyron.io/type";
const LABEL_SBX: &str = "veyron.io/sandbox-id";
const MAX_DETAIL: usize = 500;

fn machines(client: &kube::Client) -> Api<Machine> {
    Api::namespaced(client.clone(), &namespace())
}

fn kc(client: &kube::Client) -> KubeClient {
    KubeClient::from_client(client.clone())
}

// ── Checkpoints / undo / pin ───────────────────────────────────────

pub fn auto_checkpoint_enabled() -> bool {
    !matches!(
        std::env::var("VEYRON_SANDBOX_AUTO_CHECKPOINT")
            .ok()
            .as_deref(),
        Some("0" | "false" | "off")
    )
}

fn ckpt_keep() -> usize {
    env_u64("VEYRON_SANDBOX_CKPT_KEEP", 5).max(1) as usize
}

fn ckpt_prefix(id: &str) -> String {
    format!("{id}-ckpt-")
}

#[derive(Debug, Clone, Serialize)]
pub struct Checkpoint {
    pub name: String,
    pub pinned: bool,
    pub status: String,
}

fn pinned_set(m: &Machine) -> Vec<String> {
    m.metadata
        .annotations
        .as_ref()
        .and_then(|a| a.get(ANN_PINNED))
        .map(|v| {
            v.split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Newest last (names embed a millisecond timestamp).
pub async fn checkpoints(client: &kube::Client, sb: &Sandbox) -> Result<Vec<Checkpoint>> {
    let pinned = pinned_set(&machines(client).get(&sb.vm).await?);
    let prefix = ckpt_prefix(&sb.id);
    let mut out: Vec<Checkpoint> =
        kairon_ops::list_snapshots(&kc(client), &sb.namespace, Some(&sb.vm))
            .await?
            .into_iter()
            .filter(|r| r.name.starts_with(&prefix))
            .map(|r| Checkpoint {
                pinned: pinned.contains(&r.name),
                name: r.name,
                status: r.status,
            })
            .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Checkpoint before an exec; returns the checkpoint name. Never fails the caller.
pub async fn checkpoint_before_exec(client: &kube::Client, sb: &Sandbox) -> Option<String> {
    if !auto_checkpoint_enabled() || sb.status != "ready" {
        return None;
    }
    let name = format!("{}{}", ckpt_prefix(&sb.id), Utc::now().timestamp_millis());
    if let Err(e) = kairon_ops::create_snapshot(&kc(client), &sb.namespace, &sb.vm, &name).await {
        log::warn!("sandbox {}: pre-exec checkpoint failed: {e:#}", sb.id);
        return None;
    }
    // Ring buffer: drop the oldest unpinned records beyond the keep limit.
    if let Ok(all) = checkpoints(client, sb).await {
        let unpinned: Vec<&Checkpoint> = all.iter().filter(|c| !c.pinned).collect();
        if unpinned.len() > ckpt_keep() {
            for old in &unpinned[..unpinned.len() - ckpt_keep()] {
                let _ = kairon_ops::delete_snapshot(&kc(client), &sb.namespace, &old.name).await;
            }
        }
    }
    Some(name)
}

/// Restore `name`, or the newest checkpoint, in place.
pub async fn undo(client: &kube::Client, sb: &Sandbox, name: Option<&str>) -> Result<String> {
    let all = checkpoints(client, sb).await?;
    let target = match name {
        Some(n) => all
            .iter()
            .find(|c| c.name == n)
            .map(|c| c.name.clone())
            .ok_or_else(|| anyhow::anyhow!("no checkpoint {n:?} for sandbox {}", sb.id))?,
        None => all
            .last()
            .map(|c| c.name.clone())
            .ok_or_else(|| anyhow::anyhow!("sandbox {} has no checkpoints yet", sb.id))?,
    };
    kairon_ops::restore_snapshot(&kc(client), &sb.namespace, &target).await?;
    Ok(target)
}

pub async fn pin(client: &kube::Client, sb: &Sandbox, name: &str) -> Result<()> {
    if !checkpoints(client, sb)
        .await?
        .iter()
        .any(|c| c.name == name)
    {
        bail!("no checkpoint {name:?} for sandbox {}", sb.id);
    }
    let api = machines(client);
    let mut pinned = pinned_set(&api.get(&sb.vm).await?);
    if !pinned.iter().any(|p| p == name) {
        pinned.push(name.to_string());
    }
    let patch = json!({"metadata": {"annotations": {ANN_PINNED: pinned.join(",")}}});
    api.patch(&sb.vm, &PatchParams::default(), &Patch::Merge(&patch))
        .await?;
    Ok(())
}

// ── Branch / pick ──────────────────────────────────────────────────

/// Fork `sb` into `count` children that keep its running processes. Children
/// inherit the owner, expiry and internet flag.
pub async fn fork(client: &kube::Client, sb: &Sandbox, count: u32) -> Result<Vec<Sandbox>> {
    let api = machines(client);
    if sb.status != "ready" {
        bail!(
            "sandbox {} is {}; only a ready sandbox can be forked",
            sb.id,
            sb.status
        );
    }
    let parent = api.get(&sb.vm).await?;
    let existing: Vec<String> = api
        .list(&ListParams::default())
        .await?
        .items
        .iter()
        .map(|m| m.name().to_string())
        .collect();
    let mut made = Vec::new();
    let mut n = 1u32;
    while (made.len() as u32) < count {
        let name = format!("{}-b{n}", sb.id);
        n += 1;
        if existing.contains(&name) {
            continue;
        }
        let mut child = crate::api::handlers::vm_fork::child_machine(&parent, &name);
        child.metadata.namespace = Some(namespace());
        let anns = child
            .metadata
            .annotations
            .get_or_insert_with(Default::default);
        for key in [ANN_OWNER, ANN_EXPIRES, ANN_INTERNET] {
            if let Some(v) = parent
                .metadata
                .annotations
                .as_ref()
                .and_then(|a| a.get(key))
            {
                anns.insert(key.to_string(), v.clone());
            }
        }
        anns.insert(ANN_CREATED.to_string(), Utc::now().to_rfc3339());
        let labels = child.metadata.labels.get_or_insert_with(Default::default);
        labels.insert(LABEL_SANDBOX.to_string(), "true".to_string());
        labels.insert(LABEL_STATE.to_string(), STATE_CLAIMED.to_string());
        labels.insert(LABEL_MANAGED_BY.to_string(), "veyron".to_string());
        labels.insert(LABEL_FORKED_FROM.to_string(), sb.vm.clone());
        let created = api.create(&PostParams::default(), &child).await?;
        let mut view = sb.clone();
        view.id = created.name().to_string();
        view.vm = created.name().to_string();
        view.status = "starting".into();
        view.parent = sb.id.clone();
        made.push(view);
    }
    Ok(made)
}

/// Keep `keep`; delete the rest of its branch group (the parent and its other forks).
/// The survivor is detached from the group so it becomes an ordinary sandbox.
pub async fn pick(client: &kube::Client, any_member: &Sandbox, keep: &str) -> Result<Vec<String>> {
    let api = machines(client);
    let root = if any_member.parent.is_empty() {
        any_member.id.clone()
    } else {
        any_member.parent.clone()
    };
    let mut group: Vec<String> = api
        .list(&ListParams::default().labels(&format!("{LABEL_FORKED_FROM}={root}")))
        .await?
        .items
        .iter()
        .map(|m| m.name().to_string())
        .collect();
    group.push(root.clone());
    if !group.iter().any(|g| g == keep) {
        bail!("{keep} is not part of the branch group of {root}");
    }
    let mut deleted = Vec::new();
    for g in group.iter().filter(|g| g.as_str() != keep) {
        match api.delete(g, &DeleteParams::background()).await {
            Ok(_) => deleted.push(g.clone()),
            Err(kube::Error::Api(ae)) if ae.code == 404 => {}
            Err(e) => return Err(e.into()),
        }
    }
    // JSON merge patch: null removes the label so the survivor stands alone.
    let patch = json!({"metadata": {"labels": {LABEL_FORKED_FROM: null}}});
    let _ = api
        .patch(keep, &PatchParams::default(), &Patch::Merge(&patch))
        .await;
    Ok(deleted)
}

// ── Wake on connect ────────────────────────────────────────────────

/// Resume an idle-suspended sandbox and wait until it is Running.
pub async fn wake(client: &kube::Client, sb: &Sandbox) -> Result<()> {
    let api = machines(client);
    let patch = json!({
        "metadata": {"annotations": {"kairon.zyvor.dev/idle-suspended": null}},
        "spec": {"powerState": POWER_RUNNING},
    });
    api.patch(&sb.vm, &PatchParams::default(), &Patch::Merge(&patch))
        .await?;
    let deadline = Instant::now() + Duration::from_secs(env_u64("VEYRON_SANDBOX_WAKE_SECS", 60));
    loop {
        let m = api.get(&sb.vm).await?;
        if m.is_running() && m.runtime_id().is_some() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("sandbox {} did not resume within the wake timeout", sb.id);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

// ── Egress policy ──────────────────────────────────────────────────

/// Per-sandbox egress allowlist, enforced by kairon through a default-deny
/// `MachineNetworkPolicy` it owns for the Machine.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EgressPolicy {
    #[serde(default)]
    pub allow_cidrs: Vec<String>,
    #[serde(default)]
    pub allow_ports: Vec<String>,
    #[serde(default)]
    pub allow_fqdns: Vec<String>,
    #[serde(default)]
    pub allow_dns: Vec<String>,
    #[serde(default)]
    pub allow_sni: Vec<String>,
    #[serde(default)]
    pub allow_icmp: bool,
}

fn policy_api(client: &kube::Client) -> Api<kube::api::DynamicObject> {
    let gvk =
        kube::core::GroupVersionKind::gvk("kairon.zyvor.dev", "v1beta1", "MachineNetworkPolicy");
    Api::namespaced_with(
        client.clone(),
        &namespace(),
        &kube::api::ApiResource::from_gvk_with_plural(&gvk, "machinenetworkpolicies"),
    )
}

fn policy_name(id: &str) -> String {
    format!("{id}-egress")
}

fn policy_object(sb: &Sandbox, p: &EgressPolicy) -> serde_json::Value {
    json!({
        "apiVersion": "kairon.zyvor.dev/v1beta1",
        "kind": "MachineNetworkPolicy",
        "metadata": {
            "name": policy_name(&sb.id),
            "namespace": namespace(),
            "labels": {LABEL_MANAGED_BY: "veyron", LABEL_SBX: sb.id},
        },
        "spec": {
            "machineName": sb.vm,
            "policy": {
                "defaultAllow": false,
                "allowCidrs": p.allow_cidrs,
                "allowPorts": p.allow_ports,
                "allowFqdns": p.allow_fqdns,
                "allowDNS": p.allow_dns,
                "allowSNI": p.allow_sni,
                "allowIcmp": p.allow_icmp,
            },
        },
    })
}

/// Apply (create or replace) the sandbox's egress allowlist. Returns kairon's own
/// view of it; `enforced` is true only once kairon reports the Machine as applied.
pub async fn set_egress(
    client: &kube::Client,
    sb: &Sandbox,
    p: &EgressPolicy,
) -> Result<serde_json::Value> {
    let api = policy_api(client);
    let obj: kube::api::DynamicObject = serde_json::from_value(policy_object(sb, p))?;
    let name = policy_name(&sb.id);
    if api.get_opt(&name).await?.is_some() {
        api.patch(
            &name,
            &PatchParams::default(),
            &Patch::Merge(&json!({"spec": obj.data["spec"]})),
        )
        .await?;
    } else {
        api.create(&PostParams::default(), &obj).await?;
    }
    get_egress(client, sb)
        .await?
        .ok_or_else(|| anyhow::anyhow!("egress policy vanished after apply"))
}

pub async fn get_egress(client: &kube::Client, sb: &Sandbox) -> Result<Option<serde_json::Value>> {
    let Some(obj) = policy_api(client).get_opt(&policy_name(&sb.id)).await? else {
        return Ok(None);
    };
    Ok(Some(egress_view(sb, &obj.data)))
}

fn egress_view(sb: &Sandbox, data: &serde_json::Value) -> serde_json::Value {
    let st = data.get("status");
    let applied = st
        .and_then(|s| s.get("appliedMachines"))
        .and_then(|v| v.as_array())
        .is_some_and(|a| a.iter().any(|m| m.as_str() == Some(sb.vm.as_str())));
    json!({
        "sandbox": sb.id,
        "policy": data.get("spec").and_then(|s| s.get("policy")),
        "enforced": applied,
        "phase": st.and_then(|s| s.get("phase")),
        "message": st.and_then(|s| s.get("message")),
    })
}

pub async fn clear_egress(client: &kube::Client, sb: &Sandbox) -> Result<bool> {
    match policy_api(client)
        .delete(&policy_name(&sb.id), &DeleteParams::default())
        .await
    {
        Ok(_) => Ok(true),
        Err(kube::Error::Api(ae)) if ae.code == 404 => Ok(false),
        Err(e) => Err(e.into()),
    }
}

// ── Run ledger ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LedgerEntry {
    pub at: String,
    pub sandbox: String,
    pub actor: String,
    /// `exec` | `write_file` | `fork` | `pick` | `undo` | `pin`
    pub action: String,
    /// Command line / path / branch info, truncated; never command output.
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<String>,
}

fn ledger_keep() -> usize {
    env_u64("VEYRON_SANDBOX_LEDGER_KEEP", 200).max(10) as usize
}

fn truncate_detail(s: &str) -> String {
    if s.chars().count() <= MAX_DETAIL {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(MAX_DETAIL).collect();
        t.push('…');
        t
    }
}

fn configmaps(client: &kube::Client) -> Api<ConfigMap> {
    Api::namespaced(client.clone(), &namespace())
}

/// Append a ledger record; failures are logged only.
pub async fn record(client: &kube::Client, mut e: LedgerEntry) {
    e.at = Utc::now().to_rfc3339();
    e.detail = truncate_detail(&e.detail);
    let name = format!(
        "veyron-sbxrun-{}-{}",
        e.sandbox,
        Utc::now().timestamp_millis()
    );
    let mut cm = ConfigMap::default();
    cm.metadata.name = Some(name);
    cm.metadata.namespace = Some(namespace());
    cm.metadata.labels = Some(BTreeMap::from([
        (LABEL_TYPE.to_string(), LEDGER_TYPE.to_string()),
        (LABEL_SBX.to_string(), e.sandbox.clone()),
    ]));
    cm.data = Some(BTreeMap::from([(
        "entry".to_string(),
        serde_json::to_string(&e).unwrap_or_default(),
    )]));
    if let Err(err) = configmaps(client).create(&PostParams::default(), &cm).await {
        log::warn!("sandbox {}: ledger write failed: {err}", e.sandbox);
        return;
    }
    prune_ledger(client, &e.sandbox).await;
}

async fn prune_ledger(client: &kube::Client, sandbox: &str) {
    let Ok(mut all) = entries(client, sandbox).await else {
        return;
    };
    if all.len() <= ledger_keep() {
        return;
    }
    all.sort_by(|a, b| a.0.cmp(&b.0));
    let api = configmaps(client);
    for (name, _) in &all[..all.len() - ledger_keep()] {
        let _ = api.delete(name, &DeleteParams::default()).await;
    }
}

async fn entries(client: &kube::Client, sandbox: &str) -> Result<Vec<(String, LedgerEntry)>> {
    let lp =
        ListParams::default().labels(&format!("{LABEL_TYPE}={LEDGER_TYPE},{LABEL_SBX}={sandbox}"));
    Ok(configmaps(client)
        .list(&lp)
        .await?
        .items
        .into_iter()
        .filter_map(|cm| {
            let name = cm.metadata.name.clone()?;
            let raw = cm.data.as_ref()?.get("entry")?.clone();
            Some((name, serde_json::from_str::<LedgerEntry>(&raw).ok()?))
        })
        .collect())
}

pub async fn ledger(client: &kube::Client, sandbox: &str) -> Result<Vec<LedgerEntry>> {
    let mut v = entries(client, sandbox).await?;
    v.sort_by(|a, b| a.1.at.cmp(&b.1.at));
    Ok(v.into_iter().map(|(_, e)| e).collect())
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct LedgerSummary {
    pub runs: usize,
    pub failures: usize,
    pub exec_seconds: f64,
    pub forks: usize,
    pub undos: usize,
}

pub fn summarize(entries: &[LedgerEntry]) -> LedgerSummary {
    let mut s = LedgerSummary::default();
    for e in entries {
        match e.action.as_str() {
            "exec" => {
                s.runs += 1;
                if e.exit_code.is_some_and(|c| c != 0) {
                    s.failures += 1;
                }
                s.exec_seconds += e.duration_ms as f64 / 1000.0;
            }
            "fork" => s.forks += 1,
            "undo" => s.undos += 1,
            _ => {}
        }
    }
    s.exec_seconds = (s.exec_seconds * 100.0).round() / 100.0;
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_is_truncated_on_char_boundaries() {
        let long = "é".repeat(MAX_DETAIL + 50);
        let t = truncate_detail(&long);
        assert_eq!(t.chars().count(), MAX_DETAIL + 1);
        assert!(t.ends_with('…'));
        assert_eq!(truncate_detail("ls"), "ls");
    }

    #[test]
    fn summary_counts_runs_failures_and_seconds() {
        let e = |action: &str, code: Option<i64>, ms: u64| LedgerEntry {
            action: action.into(),
            exit_code: code,
            duration_ms: ms,
            ..Default::default()
        };
        let s = summarize(&[
            e("exec", Some(0), 1500),
            e("exec", Some(2), 500),
            e("fork", None, 0),
            e("undo", None, 0),
        ]);
        assert_eq!(
            s,
            LedgerSummary {
                runs: 2,
                failures: 1,
                exec_seconds: 2.0,
                forks: 1,
                undos: 1
            }
        );
    }

    #[test]
    fn pinned_annotation_round_trips() {
        let mut m = Machine::new("s", Default::default());
        m.metadata.annotations = Some(BTreeMap::from([(
            ANN_PINNED.to_string(),
            "a,,b".to_string(),
        )]));
        assert_eq!(pinned_set(&m), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn egress_is_default_deny_and_bound_to_the_machine() {
        let sb = Sandbox {
            id: "sbx-1".into(),
            vm: "sbx-1".into(),
            ..Default::default()
        };
        let p = EgressPolicy {
            allow_fqdns: vec!["pypi.org".into()],
            allow_ports: vec!["443".into()],
            ..Default::default()
        };
        let o = policy_object(&sb, &p);
        assert_eq!(o["spec"]["machineName"], "sbx-1");
        assert_eq!(o["spec"]["policy"]["defaultAllow"], false);
        assert_eq!(o["spec"]["policy"]["allowFqdns"], json!(["pypi.org"]));
        assert_eq!(o["metadata"]["name"], "sbx-1-egress");
    }

    #[test]
    fn egress_enforced_only_when_kairon_reports_machine_applied() {
        let sb = Sandbox {
            id: "s".into(),
            vm: "s".into(),
            ..Default::default()
        };
        let pending = json!({"spec": {"policy": {}}, "status": {"phase": "Pending"}});
        assert_eq!(egress_view(&sb, &pending)["enforced"], false);
        let applied = json!({"status": {"phase": "Active", "appliedMachines": ["s"]}});
        assert_eq!(egress_view(&sb, &applied)["enforced"], true);
    }

    #[test]
    fn checkpoint_names_are_prefixed_by_sandbox() {
        assert_eq!(ckpt_prefix("sbx-1"), "sbx-1-ckpt-");
    }
}
