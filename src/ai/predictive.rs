// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Predictive ops: sample guest disks, VM memory, launcher restarts and node
//! health every five minutes, fit robust trends, and warn before things break.
//! Each forecast becomes a `VeyronInsight` and, where there is a clear fix, a
//! proposal (grow the disk, restart the guest, or cordon and drain the node).
//!
//! Samples live in a ConfigMap ring buffer (`veyron.io/type=ai-samples`, ~48 h).
//! Guest filesystem readings come from Prometheus when `VEYRON_PROMETHEUS_URL`
//! is set and from the guest agent otherwise.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use axum::{Json, extract::State, response::IntoResponse, response::Response};
use futures_util::{StreamExt, stream};
use k8s_openapi::api::core::v1::{ConfigMap, Node, Pod};
use kube::Api;
use kube::api::{ListParams, ObjectMeta, Patch, PatchParams, PostParams};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::api::err;
use super::registry::{self, ToolCtx};
use crate::api::http_server::web::SharedState;

/// Run every Nth leader tick (ticks are one minute apart).
pub const TICK_EVERY: u64 = 5;

const CM_NAME: &str = "veyron-ai-samples";
const TYPE_LABEL: &str = "ai-samples";
const KEEP_HOURS: i64 = 48;
const FULL_RES_HOURS: i64 = 6;
const BUCKET_SECS: i64 = 1800;
const MAX_SERIES: usize = 300;
const FS_PER_TICK: usize = 20;
const MEM_PER_TICK: usize = 100;
const REALERT_HOURS: i64 = 12;

type Series = BTreeMap<String, Vec<(i64, f64)>>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Store {
    #[serde(default)]
    pub fs: Series,
    #[serde(default)]
    pub mem: Series,
    #[serde(default)]
    pub restarts: Series,
    #[serde(default)]
    pub node_ready: Series,
    #[serde(default)]
    pub node_pressure: Series,
    /// Latest filesystem size per fs key, for disk-grow proposals.
    #[serde(default)]
    pub fs_size: BTreeMap<String, u64>,
    /// Forecast key → (RFC3339 time first raised, proposal id).
    #[serde(default)]
    pub alerted: BTreeMap<String, (String, String)>,
    #[serde(default)]
    pub fs_offset: usize,
    #[serde(default)]
    pub sampled_at: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub forecast: Vec<ForecastItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ForecastItem {
    pub id: String,
    /// disk_full | memory_leak | crash_loop | node_degrading
    pub kind: String,
    pub target: String,
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_hours: Option<f64>,
    /// critical | warning
    pub severity: String,
    pub current: f64,
    pub trend_per_hour: f64,
    pub unit: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub fix: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub proposal_id: String,
}

// ── Storage ───────────────────────────────────────────────────────

async fn load(client: &kube::Client, ns: &str) -> Store {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), ns);
    api.get_opt(CM_NAME)
        .await
        .ok()
        .flatten()
        .and_then(|cm| cm.data?.get("store.json").cloned())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

async fn save(client: &kube::Client, ns: &str, store: &Store) -> anyhow::Result<()> {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), ns);
    let cm = ConfigMap {
        metadata: ObjectMeta {
            name: Some(CM_NAME.into()),
            namespace: Some(ns.into()),
            labels: Some(BTreeMap::from([
                ("veyron.io/type".to_string(), TYPE_LABEL.to_string()),
                (
                    "app.kubernetes.io/managed-by".to_string(),
                    "veyron".to_string(),
                ),
            ])),
            ..Default::default()
        },
        data: Some(BTreeMap::from([(
            "store.json".to_string(),
            serde_json::to_string(store)?,
        )])),
        ..Default::default()
    };
    match api.get_opt(CM_NAME).await? {
        Some(_) => {
            api.patch(
                CM_NAME,
                &PatchParams::apply("veyron-ai").force(),
                &Patch::Apply(&cm),
            )
            .await?;
        }
        None => {
            api.create(&PostParams::default(), &cm).await?;
        }
    }
    Ok(())
}

/// Full resolution for the last few hours, 30-minute buckets after that, nothing past 48 h.
pub fn compact_series(points: &mut Vec<(i64, f64)>, now: i64) {
    let full_from = now - FULL_RES_HOURS * 3600;
    let keep_from = now - KEEP_HOURS * 3600;
    let mut out: Vec<(i64, f64)> = Vec::with_capacity(points.len());
    let mut last_bucket = i64::MIN;
    for &(t, v) in points.iter() {
        if t < keep_from {
            continue;
        }
        if t >= full_from {
            out.push((t, v));
            continue;
        }
        let b = t / BUCKET_SECS;
        if b != last_bucket {
            out.push((t, v));
            last_bucket = b;
        }
    }
    *points = out;
}

fn push(series: &mut Series, key: String, t: i64, v: f64) {
    let s = series.entry(key).or_default();
    if s.last().is_some_and(|(lt, _)| *lt == t) {
        return;
    }
    s.push((t, v));
    compact_series(s, t);
}

fn prune_series(series: &mut Series, now: i64) {
    series.retain(|_, v| v.last().is_some_and(|(t, _)| now - t < KEEP_HOURS * 3600));
    if series.len() > MAX_SERIES {
        let mut by_age: Vec<(String, i64)> = series
            .iter()
            .map(|(k, v)| (k.clone(), v.last().map(|p| p.0).unwrap_or(0)))
            .collect();
        by_age.sort_by_key(|(_, t)| *t);
        for (k, _) in by_age.into_iter().take(series.len() - MAX_SERIES) {
            series.remove(&k);
        }
    }
}

// ── Trends ────────────────────────────────────────────────────────

/// Theil–Sen slope (median of pairwise slopes), per hour. Robust to spikes.
pub fn theil_sen_per_hour(points: &[(i64, f64)]) -> Option<f64> {
    let pts = &points[points.len().saturating_sub(60)..];
    let mut slopes = Vec::new();
    for i in 0..pts.len() {
        for j in (i + 1)..pts.len() {
            let dt = (pts[j].0 - pts[i].0) as f64 / 3600.0;
            if dt > 0.0 {
                slopes.push((pts[j].1 - pts[i].1) / dt);
            }
        }
    }
    if slopes.is_empty() {
        return None;
    }
    slopes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = slopes.len() / 2;
    Some(if slopes.len() % 2 == 0 {
        (slopes[m - 1] + slopes[m]) / 2.0
    } else {
        slopes[m]
    })
}

fn span_hours(points: &[(i64, f64)]) -> f64 {
    match (points.first(), points.last()) {
        (Some(a), Some(b)) => (b.0 - a.0) as f64 / 3600.0,
        _ => 0.0,
    }
}

/// Share of consecutive steps that go up (ignoring flat steps).
fn rising_share(points: &[(i64, f64)]) -> f64 {
    let (mut up, mut moved) = (0, 0);
    for w in points.windows(2) {
        let d = w[1].1 - w[0].1;
        if d.abs() > f64::EPSILON {
            moved += 1;
            if d > 0.0 {
                up += 1;
            }
        }
    }
    if moved == 0 {
        0.0
    } else {
        up as f64 / moved as f64
    }
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

fn key_id(key: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:012x}")[..12].to_string()
}

pub fn forecast_disk(key: &str, pts: &[(i64, f64)]) -> Option<ForecastItem> {
    if pts.len() < 6 || span_hours(pts) < 1.0 {
        return None;
    }
    let (target, mount) = key.rsplit_once('|')?;
    if ["/boot", "/snap", "/run", "/dev", "/sys", "/proc"]
        .iter()
        .any(|p| mount.starts_with(p))
    {
        return None;
    }
    let cur = pts.last()?.1;
    let slope = theil_sen_per_hour(pts)?;
    if slope <= 0.0005 {
        return None;
    }
    let eta = ((0.97 - cur) / slope).max(0.0);
    if eta > 72.0 && cur < 0.9 {
        return None;
    }
    let sev = if eta <= 24.0 || cur >= 0.95 {
        "critical"
    } else {
        "warning"
    };
    Some(ForecastItem {
        id: key_id(&format!("disk|{key}")),
        kind: "disk_full".into(),
        target: target.into(),
        title: format!("{mount} on {target} fills up in about {}", hours_words(eta)),
        detail: format!(
            "{mount} is {:.0}% full and growing {:.1} points an hour.",
            cur * 100.0,
            slope * 100.0
        ),
        eta_hours: Some(round1(eta)),
        severity: sev.into(),
        current: round1(cur * 100.0),
        trend_per_hour: round1(slope * 100.0),
        unit: "%".into(),
        ..Default::default()
    })
}

pub fn forecast_memory(key: &str, pts: &[(i64, f64)]) -> Option<ForecastItem> {
    if pts.len() < 8 || span_hours(pts) < 3.0 {
        return None;
    }
    let cur = pts.last()?.1;
    let slope = theil_sen_per_hour(pts)?;
    if slope < 0.5 || cur < 50.0 || rising_share(pts) < 0.6 {
        return None;
    }
    let eta = ((95.0 - cur) / slope).max(0.0);
    if eta > 96.0 {
        return None;
    }
    Some(ForecastItem {
        id: key_id(&format!("mem|{key}")),
        kind: "memory_leak".into(),
        target: key.into(),
        title: format!("Memory leak suspected on {key}"),
        detail: format!(
            "Memory use climbed steadily to {cur:.0}% ({slope:.1} points an hour over {:.0} h); it reaches 95% in about {}.",
            span_hours(pts),
            hours_words(eta)
        ),
        eta_hours: Some(round1(eta)),
        severity: if eta <= 12.0 { "critical" } else { "warning" }.into(),
        current: round1(cur),
        trend_per_hour: round1(slope),
        unit: "%".into(),
        ..Default::default()
    })
}

pub fn forecast_restarts(key: &str, pts: &[(i64, f64)], now: i64) -> Option<ForecastItem> {
    let cur = pts.last()?.1;
    let base = pts.iter().find(|(t, _)| now - t <= 6 * 3600)?.1;
    let delta = cur - base;
    if delta < 3.0 {
        return None;
    }
    Some(ForecastItem {
        id: key_id(&format!("restarts|{key}")),
        kind: "crash_loop".into(),
        target: key.into(),
        title: format!("{key} keeps restarting"),
        detail: format!("Its launcher restarted {delta:.0} times in the last 6 hours."),
        severity: if delta >= 10.0 { "critical" } else { "warning" }.into(),
        current: cur,
        trend_per_hour: round1(delta / 6.0),
        unit: "restarts".into(),
        ..Default::default()
    })
}

pub fn forecast_node(
    node: &str,
    ready: &[(i64, f64)],
    pressure: &[(i64, f64)],
    now: i64,
) -> Option<ForecastItem> {
    let recent: Vec<&(i64, f64)> = ready.iter().filter(|(t, _)| now - t <= 24 * 3600).collect();
    let flaps = recent
        .windows(2)
        .filter(|w| (w[0].1 - w[1].1).abs() > 0.5)
        .count();
    let press_now = pressure
        .iter()
        .rev()
        .take(2)
        .filter(|(_, v)| *v > 0.0)
        .count()
        == 2;
    if flaps < 2 && !press_now {
        return None;
    }
    let mut why = Vec::new();
    if flaps >= 2 {
        why.push(format!("Ready flipped {flaps} times in 24 h"));
    }
    if press_now {
        why.push("it reports memory, disk or PID pressure".to_string());
    }
    Some(ForecastItem {
        id: key_id(&format!("node|{node}")),
        kind: "node_degrading".into(),
        target: node.into(),
        title: format!("Node {node} is degrading"),
        detail: format!("{}.", capitalize(&why.join(" and "))),
        severity: if flaps >= 4 { "critical" } else { "warning" }.into(),
        current: flaps as f64,
        trend_per_hour: 0.0,
        unit: "flaps".into(),
        ..Default::default()
    })
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

pub fn hours_words(h: f64) -> String {
    if h < 1.0 {
        "under an hour".into()
    } else if h < 48.0 {
        format!("{:.0} h", h)
    } else {
        format!("{:.0} days", h / 24.0)
    }
}

pub fn forecast_all(store: &Store, now: i64) -> Vec<ForecastItem> {
    let mut out: Vec<ForecastItem> = Vec::new();
    out.extend(store.fs.iter().filter_map(|(k, p)| forecast_disk(k, p)));
    out.extend(store.mem.iter().filter_map(|(k, p)| forecast_memory(k, p)));
    out.extend(
        store
            .restarts
            .iter()
            .filter_map(|(k, p)| forecast_restarts(k, p, now)),
    );
    let empty = Vec::new();
    out.extend(store.node_ready.iter().filter_map(|(n, p)| {
        forecast_node(n, p, store.node_pressure.get(n).unwrap_or(&empty), now)
    }));
    out.sort_by(|a, b| {
        (a.severity != "critical")
            .cmp(&(b.severity != "critical"))
            .then(
                a.eta_hours
                    .unwrap_or(f64::MAX)
                    .partial_cmp(&b.eta_hours.unwrap_or(f64::MAX))
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });
    out
}

// ── Sampling ──────────────────────────────────────────────────────

async fn sample(state: &SharedState, store: &mut Store) {
    let kube = state.read().await.kube_client.clone();
    let client = kube.client();
    let now = chrono::Utc::now().timestamp();
    let sandbox_ns = crate::sandbox::namespace();

    let vmis: Vec<(String, String)> =
        Api::<crate::kube::types::VirtualMachineInstance>::all(client.clone())
            .list(&ListParams::default())
            .await
            .map(|l| {
                l.items
                    .into_iter()
                    .filter(|v| {
                        v.status.as_ref().and_then(|s| s.phase.as_deref()) == Some("Running")
                    })
                    .filter_map(|v| Some((v.metadata.namespace?, v.metadata.name?)))
                    .filter(|(ns, _)| *ns != sandbox_ns)
                    .collect()
            })
            .unwrap_or_default();

    // Guest filesystems: a rotating slice of VMs per tick (each read is a guest exec).
    if !vmis.is_empty() {
        let start = store.fs_offset % vmis.len();
        let slice: Vec<(String, String)> = vmis
            .iter()
            .cycle()
            .skip(start)
            .take(FS_PER_TICK.min(vmis.len()))
            .cloned()
            .collect();
        store.fs_offset = (start + slice.len()) % vmis.len();
        let readings: Vec<_> = stream::iter(slice)
            .map(|(ns, vm)| {
                let kube = kube.clone();
                async move {
                    let r = tokio::time::timeout(
                        Duration::from_secs(20),
                        kube.guest_filesystem_metrics(&ns, &vm),
                    )
                    .await;
                    (ns, vm, r)
                }
            })
            .buffer_unordered(5)
            .collect()
            .await;
        let mut prom = false;
        for (ns, vm, r) in readings {
            let Ok(Ok(resp)) = r else { continue };
            let mounts = if resp.mounts.is_empty() {
                &resp.prometheus_mounts
            } else {
                &resp.mounts
            };
            prom |= resp.mounts.is_empty() && !resp.prometheus_mounts.is_empty();
            for m in mounts.iter().filter(|m| m.size_bytes > 0) {
                let key = format!("{ns}/{vm}|{}", m.mount);
                push(
                    &mut store.fs,
                    key.clone(),
                    now,
                    m.used_bytes as f64 / m.size_bytes as f64,
                );
                store.fs_size.insert(key, m.size_bytes);
            }
        }
        store.source = if prom {
            "prometheus".into()
        } else {
            "guest-agent".into()
        };
    }

    // VM memory from the metrics pipeline.
    let mems: Vec<_> = stream::iter(vmis.iter().take(MEM_PER_TICK).cloned())
        .map(|(ns, vm)| async move {
            let c = crate::monitoring::metrics::MetricsCollector::new(ns.clone());
            let r = tokio::time::timeout(Duration::from_secs(10), c.collect(&vm)).await;
            (ns, vm, r)
        })
        .buffer_unordered(8)
        .collect()
        .await;
    for (ns, vm, r) in mems {
        if let Ok(Ok(m)) = r {
            let pct = m.memory.usage_percent;
            if pct.is_finite() && pct > 0.0 {
                push(&mut store.mem, format!("{ns}/{vm}"), now, pct);
            }
        }
    }

    // Launcher restarts.
    if let Ok(pods) = Api::<Pod>::all(client.clone())
        .list(&ListParams::default().labels("kubevirt.io=virt-launcher"))
        .await
    {
        let mut per_vm: HashMap<String, i32> = HashMap::new();
        for p in pods.items {
            let ns = p.metadata.namespace.clone().unwrap_or_default();
            if ns == sandbox_ns {
                continue;
            }
            let labels = p.metadata.labels.clone().unwrap_or_default();
            let Some(vm) = labels
                .get(crate::kube::VM_NAME_LABEL)
                .or_else(|| labels.get("kubevirt.io/domain"))
                .cloned()
            else {
                continue;
            };
            let n: i32 = p
                .status
                .as_ref()
                .and_then(|s| s.container_statuses.as_ref())
                .map(|c| c.iter().map(|c| c.restart_count).sum())
                .unwrap_or(0);
            *per_vm.entry(format!("{ns}/{vm}")).or_default() += n;
        }
        for (k, n) in per_vm {
            push(&mut store.restarts, k, now, n as f64);
        }
    }

    // Node readiness and pressure.
    if let Ok(nodes) = Api::<Node>::all(client.clone())
        .list(&ListParams::default())
        .await
    {
        for n in nodes.items {
            let name = n.metadata.name.clone().unwrap_or_default();
            let conds = n
                .status
                .as_ref()
                .and_then(|s| s.conditions.clone())
                .unwrap_or_default();
            let ready = conds
                .iter()
                .any(|c| c.type_ == "Ready" && c.status == "True");
            let pressure = conds
                .iter()
                .filter(|c| c.type_.ends_with("Pressure") && c.status == "True")
                .count();
            push(
                &mut store.node_ready,
                name.clone(),
                now,
                if ready { 1.0 } else { 0.0 },
            );
            push(&mut store.node_pressure, name, now, pressure as f64);
        }
    }

    for s in [
        &mut store.fs,
        &mut store.mem,
        &mut store.restarts,
        &mut store.node_ready,
        &mut store.node_pressure,
    ] {
        prune_series(s, now);
    }
    store.fs_size.retain(|k, _| store.fs.contains_key(k));
    store.sampled_at = chrono::Utc::now().to_rfc3339();
}

// ── Fixes ─────────────────────────────────────────────────────────

fn split_target(t: &str) -> (String, String) {
    match t.split_once('/') {
        Some((a, b)) => (a.to_string(), b.to_string()),
        None => (String::new(), t.to_string()),
    }
}

/// The PVC behind a VM's first persistent disk, if any.
async fn root_pvc(client: &kube::Client, ns: &str, vm: &str) -> Option<String> {
    let v = Api::<crate::kube::types::VirtualMachine>::namespaced(client.clone(), ns)
        .get(vm)
        .await
        .ok()?;
    let j = serde_json::to_value(&v).ok()?;
    j.pointer("/spec/template/spec/volumes")?
        .as_array()?
        .iter()
        .find_map(|vol| {
            vol.pointer("/persistentVolumeClaim/claimName")
                .or_else(|| vol.pointer("/dataVolume/name"))
                .and_then(|n| n.as_str())
                .map(str::to_string)
        })
}

fn grown_size(bytes: u64) -> String {
    let gi = (bytes as f64 * 1.5 / (1u64 << 30) as f64).ceil().max(1.0) as u64;
    format!("{gi}Gi")
}

async fn propose(ctx: &ToolCtx, item: &ForecastItem, store: &Store) -> (String, Option<String>) {
    let reason = json!(format!("Predicted: {} {}", item.title, item.detail));
    match item.kind.as_str() {
        "disk_full" => {
            let (ns, vm) = split_target(&item.target);
            let fs_key = store
                .fs
                .keys()
                .find(|k| {
                    k.starts_with(&format!("{}|", item.target))
                        && key_id(&format!("disk|{k}")) == item.id
                })
                .cloned()
                .unwrap_or_default();
            let size = store.fs_size.get(&fs_key).copied().unwrap_or(0);
            match root_pvc(&ctx.kube.client(), &ns, &vm).await {
                Some(pvc) if size > 0 => {
                    let new_size = grown_size(size);
                    let args = json!({"namespace": ns, "vm_name": vm, "disk": pvc, "new_size": new_size, "reason": reason});
                    let id = submit(ctx, "expand_disk", &args).await;
                    (format!("Grow disk {pvc} to {new_size}"), id)
                }
                _ => ("Free space in the guest, or move data to a larger disk (this disk is not a resizable PVC)".into(), None),
            }
        }
        "memory_leak" => {
            let (ns, vm) = split_target(&item.target);
            let id = submit(
                ctx,
                "restart_vm",
                &json!({"namespace": ns, "vm_name": vm, "reason": reason}),
            )
            .await;
            (
                "Restart the guest to release leaked memory, then find the leaking process".into(),
                id,
            )
        }
        "crash_loop" => (
            "Open an investigation; restarts usually point at the guest or its storage".into(),
            None,
        ),
        "node_degrading" => {
            let id = drain_proposal(ctx, &item.target, &reason).await;
            ("Cordon the node and live-migrate its VMs away".into(), id)
        }
        _ => (String::new(), None),
    }
}

async fn submit(ctx: &ToolCtx, tool: &str, args: &Value) -> Option<String> {
    let p = registry::draft_change(ctx, tool, args).await.ok()?;
    let mut p = p;
    p.source = "predictive".into();
    registry::submit_proposal(ctx, p)
        .await
        .proposal
        .map(|p| p.id)
}

/// Cordon plus a live migration for every VM on the node that can move.
async fn drain_proposal(ctx: &ToolCtx, node: &str, reason: &Value) -> Option<String> {
    let mut p =
        registry::draft_change(ctx, "cordon_node", &json!({"node": node, "reason": reason}))
            .await
            .ok()?;
    let client = ctx.kube.client();
    let vmis = Api::<crate::kube::types::VirtualMachineInstance>::all(client.clone())
        .list(&ListParams::default())
        .await
        .ok()?;
    let caps = crate::kube::migration_guard::ClusterMigrationCaps::from_env();
    let mut skipped = Vec::new();
    for vmi in vmis
        .items
        .iter()
        .filter(|v| v.status.as_ref().and_then(|s| s.node_name.as_deref()) == Some(node))
        .take(10)
    {
        let (Some(ns), Some(name)) = (vmi.metadata.namespace.clone(), vmi.metadata.name.clone())
        else {
            continue;
        };
        let Ok(vm) = Api::<crate::kube::types::VirtualMachine>::namespaced(client.clone(), &ns)
            .get(&name)
            .await
        else {
            continue;
        };
        if !crate::kube::migration_guard::evaluate_eligibility(&vm, Some(vmi), caps).eligible {
            skipped.push(format!("{ns}/{name}"));
            continue;
        }
        if let Ok(m) = registry::draft_change(
            ctx,
            "migrate_vm",
            &json!({"namespace": ns, "vm_name": name}),
        )
        .await
        {
            p.steps.extend(m.steps);
            p.verify.extend(m.verify);
            p.blast_radius.vms.extend(m.blast_radius.vms);
            p.diff.extend(m.diff);
        }
    }
    if !skipped.is_empty() {
        p.blast_radius.notes.push(format!(
            "Not migrated (passthrough devices pin them): {}",
            skipped.join(", ")
        ));
    }
    p.title = format!("Drain degrading node {node}");
    p.source = "predictive".into();
    registry::submit_proposal(ctx, p)
        .await
        .proposal
        .map(|p| p.id)
}

async fn record_insight(client: &kube::Client, ai_ns: &str, item: &ForecastItem, fix: &str) {
    use crate::operator_crds::{VeyronInsight, VeyronInsightSpec};
    let (ns, vm) = split_target(&item.target);
    let ns = if ns.is_empty() { ai_ns.to_string() } else { ns };
    let spec = VeyronInsightSpec {
        insight_type: match item.kind.as_str() {
            "disk_full" => "Capacity",
            "memory_leak" => "Performance",
            _ => "Anomaly",
        }
        .into(),
        severity: if item.severity == "critical" {
            "Critical"
        } else {
            "High"
        }
        .into(),
        vm_ref: (!vm.is_empty() && item.kind != "node_degrading").then_some(vm),
        title: item.title.clone(),
        description: format!("{} {fix}", item.detail).trim().to_string(),
        source: Some("veyron-ai/predictive".into()),
        data: HashMap::from([
            ("forecastId".to_string(), item.id.clone()),
            ("kind".to_string(), item.kind.clone()),
            (
                "etaHours".to_string(),
                item.eta_hours.map(|e| e.to_string()).unwrap_or_default(),
            ),
            ("proposalId".to_string(), item.proposal_id.clone()),
        ]),
    };
    let mut ins = VeyronInsight::new(&format!("ai-forecast-{}", item.id), spec);
    ins.metadata.namespace = Some(ns.clone());
    let api: Api<VeyronInsight> = Api::namespaced(client.clone(), &ns);
    if let Err(e) = api.create(&PostParams::default(), &ins).await {
        if !matches!(&e, kube::Error::Api(ae) if ae.code == 409 || ae.code == 404) {
            log::debug!("forecast insight: {e}");
        }
    }
}

// ── Tick and handler ──────────────────────────────────────────────

pub async fn tick(state: &SharedState) {
    if std::env::var("VEYRON_AI_PREDICTIVE").is_ok_and(|v| v == "0" || v == "false") {
        return;
    }
    let ctx = super::system_ctx(state, "predictive").await;
    let client = ctx.kube.client();
    let ns = ctx.api_namespace.clone();
    let mut store = load(&client, &ns).await;
    sample(state, &mut store).await;

    let now = chrono::Utc::now();
    let mut items = forecast_all(&store, now.timestamp());
    let cutoff = now - chrono::Duration::hours(REALERT_HOURS);
    for item in items.iter_mut() {
        if let Some((when, pid)) = store.alerted.get(&item.id) {
            if chrono::DateTime::parse_from_rfc3339(when).is_ok_and(|t| t > cutoff) {
                item.proposal_id = pid.clone();
                continue;
            }
        }
        let (fix, pid) = propose(&ctx, item, &store).await;
        item.fix = fix.clone();
        item.proposal_id = pid.unwrap_or_default();
        record_insight(&client, &ns, item, &fix).await;
        store.alerted.insert(
            item.id.clone(),
            (now.to_rfc3339(), item.proposal_id.clone()),
        );
    }
    // Keep the fix text for items raised on earlier ticks.
    for item in items.iter_mut().filter(|i| i.fix.is_empty()) {
        if let Some(prev) = store.forecast.iter().find(|p| p.id == item.id) {
            item.fix = prev.fix.clone();
        }
    }
    store.alerted.retain(|_, (when, _)| {
        chrono::DateTime::parse_from_rfc3339(when)
            .is_ok_and(|t| now.signed_duration_since(t).num_hours() < 48)
    });
    store.forecast = items;
    if let Err(e) = save(&client, &ns, &store).await {
        log::warn!("save AI samples: {e}");
    }
}

pub async fn forecast_handler(State(state): State<SharedState>) -> Response {
    let (client, ns) = {
        let s = state.read().await;
        (s.kube_client.client(), super::ai_namespace(&s.namespace))
    };
    let store = load(&client, &ns).await;
    let series = store.fs.len() + store.mem.len() + store.restarts.len() + store.node_ready.len();
    let oldest = [&store.fs, &store.mem, &store.node_ready]
        .iter()
        .flat_map(|s| s.values())
        .filter_map(|v| v.first().map(|p| p.0))
        .min();
    let history_hours =
        oldest.map(|t| round1((chrono::Utc::now().timestamp() - t) as f64 / 3600.0));
    if store.sampled_at.is_empty() {
        return Json(json!({"success": true, "data": {
            "items": [], "series": 0, "sampled_at": null, "source": null, "history_hours": null,
            "note": "No samples yet. The Lease-holding API replica samples every 5 minutes; trends need about an hour of history."
        }}))
        .into_response();
    }
    if store.forecast.iter().any(|i| i.id.is_empty()) {
        return err(500, "Forecast store is corrupt");
    }
    Json(json!({"success": true, "data": {
        "items": store.forecast,
        "series": series,
        "sampled_at": store.sampled_at,
        "source": store.source,
        "history_hours": history_hours,
    }}))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: usize, step_secs: i64, start: f64, per_step: f64) -> Vec<(i64, f64)> {
        (0..n)
            .map(|i| (i as i64 * step_secs, start + per_step * i as f64))
            .collect()
    }

    #[test]
    fn theil_sen_ignores_a_spike() {
        let mut p = line(20, 3600, 0.5, 0.01);
        p[10].1 = 5.0;
        let s = theil_sen_per_hour(&p).unwrap();
        assert!((s - 0.01).abs() < 1e-9, "{s}");
    }

    #[test]
    fn disk_full_forecast() {
        // 70% → +2 points/hour: full (97%) in ~13.5 h → critical.
        let p = line(12, 600, 0.70, 0.02 / 6.0);
        let f = forecast_disk("prod/db|/var", &p).unwrap();
        assert_eq!(f.kind, "disk_full");
        assert_eq!(f.severity, "critical");
        assert!(f.eta_hours.unwrap() < 24.0);
        assert!(
            forecast_disk("prod/db|/var", &line(12, 600, 0.40, 0.0)).is_none(),
            "flat disk is fine"
        );
        assert!(forecast_disk("prod/db|/boot", &p).is_none());
    }

    #[test]
    fn memory_leak_needs_steady_rise() {
        let p = line(20, 900, 60.0, 0.5); // +2 points/hour over ~5 h
        let f = forecast_memory("a/b", &p).unwrap();
        assert_eq!(f.kind, "memory_leak");
        let noisy: Vec<(i64, f64)> = (0..20)
            .map(|i| (i * 900, if i % 2 == 0 { 60.0 } else { 80.0 }))
            .collect();
        assert!(forecast_memory("a/b", &noisy).is_none());
    }

    #[test]
    fn restarts_and_node_flaps() {
        let now = 10 * 3600;
        let r = vec![(now - 5 * 3600, 2.0), (now, 7.0)];
        assert_eq!(forecast_restarts("a/b", &r, now).unwrap().current, 7.0);
        let ready = vec![
            (now - 3000, 1.0),
            (now - 2000, 0.0),
            (now - 1000, 1.0),
            (now, 1.0),
        ];
        let f = forecast_node("n1", &ready, &[], now).unwrap();
        assert_eq!(f.kind, "node_degrading");
        assert!(forecast_node("n1", &[(now, 1.0)], &[(now - 300, 1.0), (now, 1.0)], now).is_some());
        assert!(forecast_node("n1", &[(now, 1.0)], &[(now, 1.0)], now).is_none());
    }

    #[test]
    fn compaction_keeps_recent_points_and_buckets_old_ones() {
        let now = 50 * 3600;
        let mut p: Vec<(i64, f64)> = (0..=600).map(|i| (i * 300, i as f64)).collect(); // 5-min for 50 h
        compact_series(&mut p, now);
        assert!(p.first().unwrap().0 >= now - KEEP_HOURS * 3600);
        let recent = p
            .iter()
            .filter(|(t, _)| *t >= now - FULL_RES_HOURS * 3600)
            .count();
        assert_eq!(recent, 73);
        assert!(p.len() < 160, "{}", p.len());
    }

    #[test]
    fn grows_by_half_rounded_up() {
        assert_eq!(grown_size(20 * (1 << 30)), "30Gi");
        assert_eq!(grown_size(1 << 20), "1Gi");
    }
}
