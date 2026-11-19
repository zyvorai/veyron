// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Batch and wave imports: `POST /imports/batch`.
//!
//! A plan is a list of import requests, each with an optional `wave`. Waves run in ascending
//! order; inside a wave at most `concurrency` Machines are created at once. Every VM gets its
//! own result, so one bad row never hides the others. By default a failed wave stops the later
//! waves (their VMs are reported as `skipped`), which is what a cutover plan wants: do not
//! start the app tier when the database tier failed to land.
//!
//! "Created" means the Machine was accepted by Kubernetes, not that the guest is `Running`;
//! the gate between waves is creation, and operators follow progress on `GET /imports`.
//!
//! Everything is generic over [`MachineStore`] and [`MappingStore`], so tests need no cluster.

use std::collections::{BTreeMap, BTreeSet};

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::import_events::{ClusterHooks, audit_batch};
use super::import_lifecycle::{ApiErr, KubeStore, MachineStore, StoreError, import_id};
use super::import_mappings::{KubeMappingStore, MappingStore};
use super::imports::{ImportRequest, build_import};
use crate::api::http_server::web::SharedState;

pub const MAX_VMS: usize = 200;
pub const DEFAULT_CONCURRENCY: usize = 4;
pub const MAX_CONCURRENCY: usize = 16;
pub const DEFAULT_WAVE: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
pub struct BatchItem {
    #[serde(flatten)]
    pub req: ImportRequest,
    /// Wave number, ascending; default 1.
    #[serde(default)]
    pub wave: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BatchRequest {
    #[serde(default)]
    pub vms: Vec<BatchItem>,
    /// CSV alternative to `vms` (see [`csv_to_items`]).
    #[serde(default)]
    pub csv: Option<String>,
    /// Default namespace for rows without one.
    #[serde(default)]
    pub namespace: Option<String>,
    /// Default mapping profile for rows without one.
    #[serde(default)]
    pub mapping: Option<String>,
    /// Machines created at once within a wave (default 4, max 16).
    #[serde(default)]
    pub concurrency: Option<usize>,
    /// Skip later waves when a wave has a failure (default true).
    #[serde(default = "default_true")]
    pub stop_on_failure: bool,
    /// Validate and build every Machine, create nothing.
    #[serde(default)]
    pub dry_run: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ItemError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ItemResult {
    /// Position in the submitted list.
    pub index: usize,
    pub name: String,
    pub namespace: String,
    pub wave: u32,
    /// `created`, `valid` (dry run), `failed` or `skipped`.
    pub status: &'static str,
    pub import_id: Option<String>,
    pub error: Option<ItemError>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub struct WaveReport {
    pub wave: u32,
    pub created: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchReport {
    pub dry_run: bool,
    pub total: usize,
    /// Machines created (or, in a dry run, requests that would be).
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub concurrency: usize,
    pub waves: Vec<WaveReport>,
    /// Ordered by wave, then submission order.
    pub results: Vec<ItemResult>,
}

fn err(status: StatusCode, code: &'static str, message: impl Into<String>) -> ApiErr {
    ApiErr {
        status,
        code,
        message: message.into(),
    }
}

/// Parse CSV with a header row into batch items.
///
/// Columns (any order, unknown columns are an error): `name`, `image_url`, `sha256`,
/// `namespace`, `format`, `cpus`, `memory`, `wave`, `mapping`, `repair`, `secure_boot`,
/// `tpm`, `start`, `forwards` (`hostPort:guestPort` pairs separated by `;`), and the source
/// fields `hypervisor`, `vm`, `tool`, `network`, `datastore`. Empty cells are omitted.
/// Fields may be double-quoted (`""` escapes a quote).
pub fn csv_to_items(text: &str) -> Result<Vec<BatchItem>, String> {
    let rows = parse_csv(text)?;
    let mut it = rows.into_iter();
    let header = it.next().ok_or("CSV is empty")?;
    let header: Vec<String> = header
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .collect();
    const KNOWN: [&str; 19] = [
        "name",
        "image_url",
        "sha256",
        "namespace",
        "format",
        "cpus",
        "memory",
        "wave",
        "mapping",
        "repair",
        "secure_boot",
        "tpm",
        "start",
        "forwards",
        "hypervisor",
        "vm",
        "tool",
        "network",
        "datastore",
    ];
    for h in &header {
        if !KNOWN.contains(&h.as_str()) {
            return Err(format!("unknown CSV column {h:?}"));
        }
    }
    let mut seen = BTreeSet::new();
    for h in &header {
        if !seen.insert(h) {
            return Err(format!("duplicate CSV column {h:?}"));
        }
    }
    let mut items = Vec::new();
    for (i, row) in it.enumerate() {
        let line = i + 2;
        if row.len() > header.len() {
            return Err(format!(
                "line {line}: {} cells for {} columns",
                row.len(),
                header.len()
            ));
        }
        let mut obj = serde_json::Map::new();
        let mut source = serde_json::Map::new();
        for (col, cell) in header.iter().zip(row.iter()) {
            let cell = cell.trim();
            if cell.is_empty() {
                continue;
            }
            let bad = |what: &str| format!("line {line}: column {col}: {what}");
            match col.as_str() {
                "cpus" | "wave" => {
                    let n: u32 = cell.parse().map_err(|_| bad("not a number"))?;
                    obj.insert(col.clone(), json!(n));
                }
                "repair" | "secure_boot" | "tpm" | "start" => {
                    let b = match cell.to_ascii_lowercase().as_str() {
                        "true" | "yes" | "1" => true,
                        "false" | "no" | "0" => false,
                        _ => return Err(bad("expected true or false")),
                    };
                    obj.insert(col.clone(), json!(b));
                }
                "forwards" => {
                    let mut fw = Vec::new();
                    for pair in cell.split(';').map(str::trim).filter(|p| !p.is_empty()) {
                        let (h, g) = pair.split_once(':').ok_or_else(|| bad("use host:guest"))?;
                        let f = crate::kairon::PortForward {
                            host_port: h.trim().parse().map_err(|_| bad("bad host port"))?,
                            guest_port: g.trim().parse().map_err(|_| bad("bad guest port"))?,
                            protocol: None,
                        };
                        fw.push(serde_json::to_value(f).map_err(|e| e.to_string())?);
                    }
                    obj.insert("forwards".into(), Value::Array(fw));
                }
                "hypervisor" | "vm" | "tool" | "network" | "datastore" => {
                    source.insert(col.clone(), json!(cell));
                }
                _ => {
                    obj.insert(col.clone(), json!(cell));
                }
            }
        }
        if !source.is_empty() {
            obj.insert("source".into(), Value::Object(source));
        }
        let item: BatchItem =
            serde_json::from_value(Value::Object(obj)).map_err(|e| format!("line {line}: {e}"))?;
        items.push(item);
    }
    Ok(items)
}

/// Minimal RFC 4180 reader: quoted fields, `""` escapes, CRLF or LF. Blank lines are skipped.
fn parse_csv(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut in_quotes = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    let mut any = false;
    while let Some(c) = chars.next() {
        any = true;
        if in_quotes {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    cell.push('"');
                    chars.next();
                }
                '"' => in_quotes = false,
                _ => cell.push(c),
            }
            continue;
        }
        match c {
            '"' if cell.is_empty() => in_quotes = true,
            ',' => row.push(std::mem::take(&mut cell)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut cell));
                if !(row.len() == 1 && row[0].trim().is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            _ => cell.push(c),
        }
    }
    if in_quotes {
        return Err("unterminated quote in CSV".into());
    }
    if any && (!cell.is_empty() || !row.is_empty()) {
        row.push(cell);
        if !(row.len() == 1 && row[0].trim().is_empty()) {
            rows.push(row);
        }
    }
    Ok(rows)
}

fn clamp_concurrency(c: Option<usize>) -> usize {
    c.unwrap_or(DEFAULT_CONCURRENCY).clamp(1, MAX_CONCURRENCY)
}

/// Run a plan. `Err` only for a malformed plan as a whole (empty, too big, bad CSV); per-VM
/// problems are results.
pub async fn run_batch<S: MachineStore, M: MappingStore>(
    store: &S,
    mappings: &M,
    default_ns: &str,
    batch: BatchRequest,
) -> Result<BatchReport, ApiErr> {
    let mut items = batch.vms;
    if let Some(csv) = &batch.csv {
        if !items.is_empty() {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "INVALID_REQUEST",
                "give either vms or csv, not both",
            ));
        }
        items = csv_to_items(csv).map_err(|e| err(StatusCode::BAD_REQUEST, "INVALID_CSV", e))?;
    }
    if items.is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "no VMs in the plan",
        ));
    }
    if items.len() > MAX_VMS {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "TOO_MANY_VMS",
            format!("a plan holds at most {MAX_VMS} VMs"),
        ));
    }
    let concurrency = clamp_concurrency(batch.concurrency);
    let batch_ns = batch
        .namespace
        .clone()
        .unwrap_or_else(|| default_ns.to_string());

    // Resolve namespace, mapping and wave per row; group by wave.
    struct Row {
        index: usize,
        wave: u32,
        ns: String,
        req: ImportRequest,
    }
    let mut waves: BTreeMap<u32, Vec<Row>> = BTreeMap::new();
    for (index, it) in items.into_iter().enumerate() {
        let mut req = it.req;
        let ns = req.namespace.clone().unwrap_or_else(|| batch_ns.clone());
        if req.mapping.is_none() {
            req.mapping = batch.mapping.clone();
        }
        let wave = it.wave.unwrap_or(DEFAULT_WAVE);
        waves.entry(wave).or_default().push(Row {
            index,
            wave,
            ns,
            req,
        });
    }

    let mut results: Vec<ItemResult> = Vec::new();
    let mut reports: Vec<WaveReport> = Vec::new();
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    let mut blocked = false;

    for (wave, rows) in waves {
        let mut report = WaveReport {
            wave,
            ..Default::default()
        };
        let mut wave_results: Vec<ItemResult> = Vec::new();
        let mut runnable = Vec::new();
        for row in rows {
            let base = |status, error: Option<ItemError>| ItemResult {
                index: row.index,
                name: row.req.name.clone(),
                namespace: row.ns.clone(),
                wave: row.wave,
                status,
                import_id: None,
                error,
            };
            if blocked {
                wave_results.push(base(
                    "skipped",
                    Some(ItemError {
                        code: "WAVE_BLOCKED".into(),
                        message: "an earlier wave failed and stop_on_failure is set".into(),
                    }),
                ));
            } else if !seen.insert((row.ns.clone(), row.req.name.clone())) {
                wave_results.push(base(
                    "failed",
                    Some(ItemError {
                        code: "DUPLICATE".into(),
                        message: format!(
                            "{}/{} appears more than once in the plan",
                            row.ns, row.req.name
                        ),
                    }),
                ));
            } else {
                runnable.push(row);
            }
        }
        let dry_run = batch.dry_run;
        let done: Vec<ItemResult> = futures_util::stream::iter(runnable)
            .map(|row| async move {
                let mk = |status, import_id, error| ItemResult {
                    index: row.index,
                    name: row.req.name.clone(),
                    namespace: row.ns.clone(),
                    wave: row.wave,
                    status,
                    import_id,
                    error,
                };
                let machine = match build_import(mappings, &row.req, &row.ns).await {
                    Ok(m) => m,
                    Err(e) => {
                        return mk(
                            "failed",
                            None,
                            Some(ItemError {
                                code: e.code.into(),
                                message: e.message,
                            }),
                        );
                    }
                };
                let id = import_id(&machine);
                if dry_run {
                    return mk("valid", id, None);
                }
                match store.create(&machine).await {
                    Ok(_) => mk("created", id, None),
                    Err(StoreError::Conflict) => mk(
                        "failed",
                        None,
                        Some(ItemError {
                            code: "ALREADY_EXISTS".into(),
                            message: format!("VM {}/{} already exists", row.ns, row.req.name),
                        }),
                    ),
                    Err(StoreError::Other(m)) => mk(
                        "failed",
                        None,
                        Some(ItemError {
                            code: "IMPORT_FAILED".into(),
                            message: m,
                        }),
                    ),
                }
            })
            .buffered(concurrency)
            .collect()
            .await;
        wave_results.extend(done);
        wave_results.sort_by_key(|r| r.index);
        for r in &wave_results {
            match r.status {
                "created" | "valid" => report.created += 1,
                "failed" => report.failed += 1,
                _ => report.skipped += 1,
            }
        }
        if report.failed > 0 && batch.stop_on_failure && !dry_run {
            blocked = true;
        }
        results.extend(wave_results);
        reports.push(report);
    }

    let count = |s: &str| results.iter().filter(|r| r.status == s).count();
    Ok(BatchReport {
        dry_run: batch.dry_run,
        total: results.len(),
        succeeded: count("created") + count("valid"),
        failed: count("failed"),
        skipped: count("skipped"),
        concurrency,
        waves: reports,
        results,
    })
}

fn envelope(r: Result<BatchReport, ApiErr>) -> (StatusCode, Json<Value>) {
    match r {
        Ok(rep) => {
            // 200 when everything landed, 207 when the plan ran but some VMs did not.
            let status = if rep.failed == 0 && rep.skipped == 0 {
                StatusCode::OK
            } else {
                StatusCode::MULTI_STATUS
            };
            (
                status,
                Json(json!({"success": rep.failed == 0 && rep.skipped == 0, "data": rep})),
            )
        }
        Err(e) => (
            e.status,
            Json(json!({"success": false, "error": {"code": e.code, "message": e.message}})),
        ),
    }
}

async fn batch_route(
    State(state): State<SharedState>,
    auth: Option<axum::Extension<crate::api::auth_context::AuthContext>>,
    Json(batch): Json<BatchRequest>,
) -> (StatusCode, Json<Value>) {
    if !crate::api::vm_backend::is_kairon() {
        return envelope(Err(err(
            StatusCode::NOT_IMPLEMENTED,
            "KAIRON_REQUIRED",
            "VM imports land on Kairon Machines; unset VEYRON_VM_BACKEND or set it to kairon",
        )));
    }
    let (client, ns) = {
        let s = state.read().await;
        (s.client().client(), s.namespace.clone())
    };
    let store = KubeStore {
        client: client.clone(),
    };
    let mappings = KubeMappingStore {
        client,
        namespace: ns.clone(),
    };
    let hooks = ClusterHooks {
        client: store.client.clone(),
        namespace: ns.clone(),
    };
    let actor = auth
        .map(|axum::Extension(a)| a.subject)
        .unwrap_or_else(|| "unknown".into());
    let report = run_batch(&store, &mappings, &ns, batch).await;
    if let Ok(rep) = &report {
        audit_batch(&hooks, &actor, rep).await;
    }
    envelope(report)
}

/// `POST /imports/csv-to-json`: CSV in the body, the equivalent JSON plan out. Creates nothing.
async fn csv_route(body: String) -> (StatusCode, Json<Value>) {
    match csv_to_items(&body) {
        Ok(items) => {
            let vms: Vec<Value> = items
                .iter()
                .map(|i| {
                    let mut v = json!({
                        "name": i.req.name, "namespace": i.req.namespace,
                        "image_url": i.req.image_url, "sha256": i.req.sha256,
                        "format": i.req.format, "cpus": i.req.cpus, "memory": i.req.memory,
                        "mapping": i.req.mapping, "wave": i.wave,
                        "repair": i.req.repair, "secure_boot": i.req.secure_boot,
                        "tpm": i.req.tpm, "start": i.req.start,
                        "forwards": i.req.forwards,
                        "source": {
                            "hypervisor": i.req.source.hypervisor, "vm": i.req.source.vm,
                            "tool": i.req.source.tool, "network": i.req.source.network,
                            "datastore": i.req.source.datastore,
                        },
                    });
                    prune_nulls(&mut v);
                    v
                })
                .collect();
            (
                StatusCode::OK,
                Json(json!({"success": true, "data": {"vms": vms}})),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "error": {"code": "INVALID_CSV", "message": e}})),
        ),
    }
}

fn prune_nulls(v: &mut Value) {
    if let Value::Object(m) = v {
        m.retain(|_, x| !x.is_null());
        for x in m.values_mut() {
            prune_nulls(x);
        }
        m.retain(|_, x| !matches!(x, Value::Object(o) if o.is_empty()));
    }
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/imports/batch", post(batch_route))
        .route("/imports/csv-to-json", post(csv_route))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::handlers::import_lifecycle::tests::FakeStore;
    use crate::api::handlers::import_mappings::tests::{FakeMappings, dc1};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn vm(name: &str, wave: Option<u32>) -> Value {
        let mut v = json!({
            "name": name,
            "image_url": format!("https://files.example/{name}.qcow2"),
            "sha256": "ab".repeat(32),
        });
        if let Some(w) = wave {
            v["wave"] = json!(w);
        }
        v
    }

    fn plan(vms: Vec<Value>) -> BatchRequest {
        serde_json::from_value(json!({"vms": vms, "namespace": "prod"})).unwrap()
    }

    #[tokio::test]
    async fn creates_every_vm_with_its_own_result() {
        let store = FakeStore::default();
        let maps = FakeMappings::default();
        let rep = run_batch(
            &store,
            &maps,
            "default",
            plan(vec![vm("a", None), vm("b", None)]),
        )
        .await
        .unwrap();
        assert_eq!((rep.total, rep.succeeded, rep.failed), (2, 2, 0));
        assert!(
            rep.results
                .iter()
                .all(|r| r.status == "created" && r.namespace == "prod")
        );
        assert!(
            rep.results[0]
                .import_id
                .as_deref()
                .unwrap()
                .starts_with("imp-")
        );
        assert_eq!(store.machines.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn bad_rows_fail_alone_and_duplicates_are_caught() {
        let store = FakeStore::default();
        let maps = FakeMappings::default();
        let mut bad = vm("c", None);
        bad["sha256"] = json!("nope");
        let rep = run_batch(
            &store,
            &maps,
            "default",
            plan(vec![vm("a", None), bad, vm("a", None)]),
        )
        .await
        .unwrap();
        let by: Vec<_> = rep.results.iter().map(|r| r.status).collect();
        assert_eq!(by, ["created", "failed", "failed"]);
        assert_eq!(
            rep.results[1].error.as_ref().unwrap().code,
            "INVALID_REQUEST"
        );
        assert_eq!(rep.results[2].error.as_ref().unwrap().code, "DUPLICATE");
    }

    #[tokio::test]
    async fn existing_machine_is_reported_not_overwritten() {
        let store = FakeStore::with(vec![
            crate::api::handlers::import_lifecycle::tests::import_machine("a"),
        ]);
        let maps = FakeMappings::default();
        let rep = run_batch(&store, &maps, "default", plan(vec![vm("a", None)]))
            .await
            .unwrap();
        assert_eq!(
            rep.results[0].error.as_ref().unwrap().code,
            "ALREADY_EXISTS"
        );
    }

    #[tokio::test]
    async fn failed_wave_blocks_later_waves() {
        let store = FakeStore::default();
        let maps = FakeMappings::default();
        let mut bad = vm("db", Some(1));
        bad["format"] = json!("iso");
        let rep = run_batch(
            &store,
            &maps,
            "default",
            plan(vec![vm("app", Some(2)), bad, vm("web", Some(3))]),
        )
        .await
        .unwrap();
        // ordered by wave
        let order: Vec<_> = rep
            .results
            .iter()
            .map(|r| (r.name.as_str(), r.status))
            .collect();
        assert_eq!(
            order,
            [("db", "failed"), ("app", "skipped"), ("web", "skipped")]
        );
        assert_eq!(
            rep.waves.iter().map(|w| w.wave).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert!(store.machines.lock().unwrap().is_empty());

        // stop_on_failure=false carries on
        let mut p = plan(vec![vm("app", Some(2)), {
            let mut b = vm("db", Some(1));
            b["format"] = json!("iso");
            b
        }]);
        p.stop_on_failure = false;
        let rep = run_batch(&store, &maps, "default", p).await.unwrap();
        assert_eq!((rep.succeeded, rep.failed, rep.skipped), (1, 1, 0));
    }

    #[tokio::test]
    async fn waves_run_in_order() {
        let store = FakeStore::default();
        let maps = FakeMappings::default();
        let rep = run_batch(
            &store,
            &maps,
            "default",
            plan(vec![
                vm("late", Some(5)),
                vm("early", Some(1)),
                vm("mid", Some(2)),
            ]),
        )
        .await
        .unwrap();
        let names: Vec<_> = rep.results.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["early", "mid", "late"]);
    }

    #[tokio::test]
    async fn dry_run_creates_nothing_and_reports_all_errors() {
        let store = FakeStore::default();
        let maps = FakeMappings::default();
        let mut bad = vm("x", Some(1));
        bad["cpus"] = json!(0);
        let mut p = plan(vec![bad, vm("y", Some(2))]);
        p.dry_run = true;
        let rep = run_batch(&store, &maps, "default", p).await.unwrap();
        assert!(rep.dry_run);
        assert_eq!(
            rep.results.iter().map(|r| r.status).collect::<Vec<_>>(),
            ["failed", "valid"]
        );
        assert!(store.machines.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn batch_mapping_default_applies() {
        let store = FakeStore::default();
        let maps = FakeMappings::default();
        maps.0.lock().unwrap().insert("dc1".into(), dc1());
        let mut v = vm("a", None);
        v["source"] = json!({"network": "DMZ"});
        let mut p = plan(vec![v, vm("b", None)]);
        p.mapping = Some("dc1".into());
        let rep = run_batch(&store, &maps, "default", p).await.unwrap();
        // b has no source network and dc1 has no default_network: still fine, nothing applied
        assert_eq!(rep.succeeded, 2, "{:?}", rep.results);
        let m = store.machines.lock().unwrap()[&("prod".to_string(), "a".to_string())].clone();
        assert_eq!(m.spec.network.unwrap().bridge.as_deref(), Some("br-dmz"));
        // unknown mapping fails the row
        let mut p = plan(vec![vm("z", None)]);
        p.mapping = Some("nope".into());
        let rep = run_batch(&store, &maps, "default", p).await.unwrap();
        assert_eq!(
            rep.results[0].error.as_ref().unwrap().code,
            "MAPPING_NOT_FOUND"
        );
    }

    #[tokio::test]
    async fn rejects_malformed_plans() {
        let store = FakeStore::default();
        let maps = FakeMappings::default();
        let e = run_batch(&store, &maps, "d", BatchRequest::default())
            .await
            .unwrap_err();
        assert_eq!(e.code, "INVALID_REQUEST");
        let many = plan((0..=MAX_VMS).map(|i| vm(&format!("v{i}"), None)).collect());
        assert_eq!(
            run_batch(&store, &maps, "d", many).await.unwrap_err().code,
            "TOO_MANY_VMS"
        );
        let mut both = plan(vec![vm("a", None)]);
        both.csv = Some("name\nx".into());
        assert_eq!(
            run_batch(&store, &maps, "d", both).await.unwrap_err().code,
            "INVALID_REQUEST"
        );
    }

    /// Store that records the most creates in flight at once.
    #[derive(Default)]
    struct SlowStore {
        inflight: AtomicUsize,
        peak: AtomicUsize,
    }
    impl MachineStore for SlowStore {
        async fn get(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<crate::kairon::Machine>, StoreError> {
            Ok(None)
        }
        async fn create(
            &self,
            m: &crate::kairon::Machine,
        ) -> Result<crate::kairon::Machine, StoreError> {
            let now = self.inflight.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.inflight.fetch_sub(1, Ordering::SeqCst);
            Ok(m.clone())
        }
        async fn delete(&self, _: &str, _: &str) -> Result<(), StoreError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn concurrency_cap_is_honoured() {
        let store = SlowStore::default();
        let maps = FakeMappings::default();
        let mut p = plan((0..10).map(|i| vm(&format!("v{i}"), None)).collect());
        p.concurrency = Some(3);
        let rep = run_batch(&store, &maps, "d", p).await.unwrap();
        assert_eq!(rep.succeeded, 10);
        assert_eq!(rep.concurrency, 3);
        let peak = store.peak.load(Ordering::SeqCst);
        assert!((2..=3).contains(&peak), "peak {peak}");
        assert_eq!(clamp_concurrency(Some(0)), 1);
        assert_eq!(clamp_concurrency(Some(999)), MAX_CONCURRENCY);
        assert_eq!(clamp_concurrency(None), DEFAULT_CONCURRENCY);
    }

    #[test]
    fn csv_parsing() {
        let csv = "name,image_url,sha256,cpus,wave,secure_boot,forwards,hypervisor,network\r\n\
                   web01,https://f/x.qcow2,abab,4,2,yes,30022:22;30080:80,vmware,\"VM Network\"\r\n\
                   \r\n\
                   \"db,01\",https://f/y.qcow2,cdcd,,,,,,\n";
        let items = csv_to_items(csv).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].req.cpus, Some(4));
        assert_eq!(items[0].wave, Some(2));
        assert!(items[0].req.secure_boot);
        assert_eq!(items[0].req.forwards.len(), 2);
        assert_eq!(items[0].req.source.network.as_deref(), Some("VM Network"));
        assert_eq!(items[0].req.source.hypervisor.as_deref(), Some("vmware"));
        assert_eq!(items[1].req.name, "db,01");
        assert_eq!(items[1].wave, None);
    }

    #[test]
    fn csv_errors_name_the_line() {
        assert!(csv_to_items("").unwrap_err().contains("empty"));
        assert!(
            csv_to_items("name,bogus\na,b")
                .unwrap_err()
                .contains("bogus")
        );
        assert!(
            csv_to_items("name,name\na,b")
                .unwrap_err()
                .contains("duplicate")
        );
        assert!(
            csv_to_items("name,image_url,sha256,cpus\na,b,c,four")
                .unwrap_err()
                .contains("line 2")
        );
        assert!(
            csv_to_items("name,image_url,sha256\n\"a,b,c")
                .unwrap_err()
                .contains("quote")
        );
        assert!(csv_to_items("name,image_url,sha256,start\na,b,c,maybe").is_err());
        // missing required column surfaces as a serde error with the line
        assert!(csv_to_items("name\na").unwrap_err().contains("line 2"));
    }

    #[tokio::test]
    async fn csv_plan_runs() {
        let store = FakeStore::default();
        let maps = FakeMappings::default();
        let h = "ab".repeat(32);
        let p = BatchRequest {
            csv: Some(format!(
                "name,image_url,sha256,wave\na,https://f/a.qcow2,{h},1\nb,https://f/b.qcow2,{h},2\n"
            )),
            namespace: Some("prod".into()),
            stop_on_failure: true,
            ..Default::default()
        };
        let rep = run_batch(&store, &maps, "d", p).await.unwrap();
        assert_eq!(rep.succeeded, 2);
    }
}
