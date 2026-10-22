// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! "Network access": what a Kairon VM's QEMU process may reach.
//!
//! A user-mode (slirp) guest has no host-visible interface, so FluxVM governs it with an eBPF
//! hook on the VM's cgroup whose policy is `spec.network.egress` on the Machine. The Machine
//! spec is the source of truth: kairon-node reconciles it over FluxVM, so a change made only
//! on the node would be overwritten. This module therefore *writes* by patching the spec
//! (pinned to the `resourceVersion` it read) and *reads* live state, observed flows and
//! suggestions through the node relay. The policy model, presets, explain and diagnostics are
//! in [`crate::kairon::egress`].
//!
//! Routes (Kairon backend only):
//! * `GET/PUT/DELETE /vms/:ns/:name/egress` (+ `DELETE …/egress/grant` to end a timed change)
//! * `GET …/egress/observed`, `POST …/egress/suggest`, `POST …/egress/explain`
//! * `GET /egress/presets`, `GET /egress/overview`, `POST /egress/apply`
//! * `GET/PUT /nodes/:name/egress-defaults`

// The Err side is a ready-to-send HTTP response, returned straight to the client.
#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::http_server::web::SharedState;
use crate::kairon::egress::{
    ANN_TEMPLATE, EgressMode, EgressPolicy, EgressProto, EgressRule, NetworkAccessRequest,
    PlatformEndpoints, Problem, ProblemInput, Selector, detect_problems, explain,
    merge_suggestions, parse_duration, presets, validate,
};
use crate::kairon::relay::RelayTarget;
use crate::kairon::{LABEL_MANAGED_BY, Machine};

const GRANT_LABEL: &str = "veyron.io/type";
const GRANT_VALUE: &str = "egress-grant";
const GRANT_KEY: &str = "grant.json";
const FLEET_MAX: usize = 500;

type Reply = Result<Response, Response>;

fn err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "error": msg.into() }))).into_response()
}

fn require_kairon() -> Result<(), Response> {
    if crate::api::vm_backend::is_kairon() {
        Ok(())
    } else {
        Err(err(
            StatusCode::NOT_IMPLEMENTED,
            "network access policy requires the Kairon backend",
        ))
    }
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route(
            "/vms/:ns/:name/egress",
            get(get_egress).put(put_egress).delete(delete_egress),
        )
        .route(
            "/vms/:ns/:name/egress/grant",
            axum::routing::delete(delete_grant),
        )
        .route("/vms/:ns/:name/egress/observed", get(get_observed))
        .route("/vms/:ns/:name/egress/suggest", post(post_suggest))
        .route(
            "/vms/:ns/:name/egress/explain",
            post(post_explain).get(get_explain),
        )
        .route("/egress/presets", get(get_presets))
        .route("/egress/overview", get(get_overview))
        .route("/egress/apply", post(post_apply))
        .route(
            "/nodes/:name/egress-defaults",
            get(get_defaults).put(put_defaults),
        )
        .with_state(state)
}

// ---- shared helpers ----------------------------------------------------------------------

/// Veyron's own reachable endpoints from the process environment.
pub fn platform_endpoints() -> PlatformEndpoints {
    PlatformEndpoints::from_env(&|k| std::env::var(k).ok())
}

fn kube_err(e: kube::Error) -> Response {
    match &e {
        kube::Error::Api(ae) => {
            let code = match ae.code {
                404 => StatusCode::NOT_FOUND,
                409 => StatusCode::CONFLICT,
                400 | 422 => StatusCode::UNPROCESSABLE_ENTITY,
                403 if ae.message.contains("admission webhook") => StatusCode::UNPROCESSABLE_ENTITY,
                403 => StatusCode::FORBIDDEN,
                _ => StatusCode::BAD_GATEWAY,
            };
            err(code, ae.message.clone())
        }
        _ => err(StatusCode::BAD_GATEWAY, e.to_string()),
    }
}

struct Ctx {
    kube: crate::kube::KubeClient,
    api_ns: String,
}

async fn ctx(state: &SharedState) -> Ctx {
    let st = state.read().await;
    Ctx {
        kube: st.client().clone(),
        api_ns: st.namespace.clone(),
    }
}

async fn get_machine(c: &Ctx, ns: &str, name: &str) -> Result<Machine, Response> {
    Api::<Machine>::namespaced(c.kube.client(), ns)
        .get_opt(name)
        .await
        .map_err(kube_err)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, format!("VM {ns}/{name} not found")))
}

fn template_of(m: &Machine) -> Option<String> {
    m.metadata
        .annotations
        .as_ref()
        .and_then(|a| a.get(ANN_TEMPLATE))
        .cloned()
}

fn desired_of(m: &Machine) -> Option<EgressPolicy> {
    m.spec.network.as_ref().and_then(|n| n.egress.clone())
}

/// Merge patch for `spec.network.egress`, pinned to the `resourceVersion` it was computed from
/// so a concurrent edit fails with 409 instead of being overwritten. `None` clears the field.
fn egress_patch(resource_version: Option<&str>, policy: Option<&EgressPolicy>) -> Value {
    json!({
        "metadata": {"resourceVersion": resource_version},
        "spec": {"network": {"egress": policy}},
    })
}

async fn patch_egress(
    c: &Ctx,
    m: &Machine,
    policy: Option<&EgressPolicy>,
) -> Result<Machine, Response> {
    let name = m.metadata.name.clone().unwrap_or_default();
    let ns = m.metadata.namespace.clone().unwrap_or_default();
    let patch = egress_patch(m.metadata.resource_version.as_deref(), policy);
    Api::<Machine>::namespaced(c.kube.client(), &ns)
        .patch(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
        .map_err(|e| match &e {
            kube::Error::Api(ae) if ae.code == 409 => err(
                StatusCode::CONFLICT,
                format!("the VM changed while saving; retry: {}", ae.message),
            ),
            _ => kube_err(e),
        })
}

async fn relay_for(c: &Ctx, m: &Machine) -> Option<RelayTarget> {
    if !m.is_running() {
        return None;
    }
    RelayTarget::for_machine(&c.kube.client(), m).await.ok()
}

/// What the node reports for a running VM, never failing the caller: a node that predates the
/// feature simply yields `None` plus the reason.
struct Live {
    reply: Option<Value>,
    error: Option<String>,
}

async fn live_policy(relay: Option<&RelayTarget>) -> Live {
    let Some(t) = relay else {
        return Live {
            reply: None,
            error: None,
        };
    };
    match t.call_json(Method::GET, "vm-egress", &[], "", None).await {
        Ok(v) => Live {
            reply: Some(v),
            error: None,
        },
        Err(e) => Live {
            reply: None,
            error: Some(short_err(&e)),
        },
    }
}

fn short_err(e: &anyhow::Error) -> String {
    let s = format!("{e:#}");
    if s.contains("404") {
        "the node does not support network access policy yet (update FluxVM and kairon-node)".into()
    } else {
        s.chars().take(300).collect()
    }
}

fn policy_from(v: &Value) -> Option<EgressPolicy> {
    serde_json::from_value(v.clone()).ok()
}

/// Blocked / would-be-blocked flow counts from the node's observed list.
fn flow_counts(flows: &[Value]) -> (usize, usize) {
    let audit = flows
        .iter()
        .filter(|f| f.get("audit").and_then(Value::as_bool).unwrap_or(false))
        .count();
    (flows.len() - audit, audit)
}

async fn observed_flows(relay: Option<&RelayTarget>) -> Option<Vec<Value>> {
    let t = relay?;
    let v = t
        .call_json(Method::GET, "vm-egress-observed", &[], "", None)
        .await
        .ok()?;
    Some(
        v.get("flows")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    )
}

/// The runtime id is appended by the relay; per-VM relay paths are `vm-egress/<runtime id>`.
fn ok_json(v: Value) -> Reply {
    Ok(Json(v).into_response())
}

// ---- grants (timed changes) -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Grant {
    namespace: String,
    name: String,
    /// RFC 3339
    expires_at: String,
    created_at: String,
    /// What to restore: the spec before the change (`None` = no policy of its own).
    revert_to: Option<EgressPolicy>,
}

fn grant_cm_name(ns: &str, name: &str) -> String {
    format!("egress-grant-{ns}.{name}")
}

fn grant_cm(g: &Grant) -> ConfigMap {
    let mut cm = ConfigMap::default();
    cm.metadata.name = Some(grant_cm_name(&g.namespace, &g.name));
    cm.metadata.labels = Some(BTreeMap::from([
        (GRANT_LABEL.to_string(), GRANT_VALUE.to_string()),
        (LABEL_MANAGED_BY.to_string(), "veyron".to_string()),
    ]));
    cm.data = Some(BTreeMap::from([(
        GRANT_KEY.to_string(),
        serde_json::to_string(g).unwrap_or_default(),
    )]));
    cm
}

fn grant_from(cm: &ConfigMap) -> Option<Grant> {
    serde_json::from_str(cm.data.as_ref()?.get(GRANT_KEY)?).ok()
}

async fn read_grant(c: &Ctx, ns: &str, name: &str) -> Option<Grant> {
    Api::<ConfigMap>::namespaced(c.kube.client(), &c.api_ns)
        .get_opt(&grant_cm_name(ns, name))
        .await
        .ok()
        .flatten()
        .as_ref()
        .and_then(grant_from)
}

async fn save_grant(c: &Ctx, g: &Grant) -> Result<(), Response> {
    let cms = Api::<ConfigMap>::namespaced(c.kube.client(), &c.api_ns);
    let cm = grant_cm(g);
    match cms.create(&PostParams::default(), &cm).await {
        Ok(_) => Ok(()),
        Err(kube::Error::Api(ae)) if ae.code == 409 => {
            // A newer timed change replaces the old one but keeps the ORIGINAL revert target:
            // reverting to the previous temporary state would leave the VM open for good.
            let existing = cms
                .get(&grant_cm_name(&g.namespace, &g.name))
                .await
                .map_err(kube_err)?;
            let mut merged = g.clone();
            if let Some(old) = grant_from(&existing) {
                merged.revert_to = old.revert_to;
            }
            let patch =
                json!({"data": {GRANT_KEY: serde_json::to_string(&merged).unwrap_or_default()}});
            cms.patch(
                &grant_cm_name(&g.namespace, &g.name),
                &PatchParams::default(),
                &Patch::Merge(&patch),
            )
            .await
            .map(|_| ())
            .map_err(kube_err)
        }
        Err(e) => Err(kube_err(e)),
    }
}

async fn forget_grant(c: &Ctx, ns: &str, name: &str) {
    let _ = Api::<ConfigMap>::namespaced(c.kube.client(), &c.api_ns)
        .delete(&grant_cm_name(ns, name), &DeleteParams::default())
        .await;
}

/// Put a VM back to what it was before a timed change.
async fn revert(c: &Ctx, g: &Grant) -> Result<(), String> {
    let api = Api::<Machine>::namespaced(c.kube.client(), &g.namespace);
    match api.get_opt(&g.name).await.map_err(|e| e.to_string())? {
        None => {} // the VM is gone; nothing to restore
        Some(m) => {
            let patch = json!({"spec": {"network": {"egress": g.revert_to}}});
            let _ = m;
            api.patch(&g.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    forget_grant(c, &g.namespace, &g.name).await;
    Ok(())
}

/// Scheduler tick: end timed changes whose time is up. Leader-only like the other ticks.
pub async fn egress_grant_tick(
    kube: crate::kube::KubeClient,
    api_ns: String,
) -> anyhow::Result<()> {
    if !crate::api::vm_backend::is_kairon() {
        return Ok(());
    }
    let c = Ctx { kube, api_ns };
    let list = Api::<ConfigMap>::namespaced(c.kube.client(), &c.api_ns)
        .list(&ListParams::default().labels(&format!("{GRANT_LABEL}={GRANT_VALUE}")))
        .await?;
    let now = chrono::Utc::now();
    for cm in &list.items {
        let Some(g) = grant_from(cm) else { continue };
        let due = chrono::DateTime::parse_from_rfc3339(&g.expires_at)
            .map(|t| t.with_timezone(&chrono::Utc) <= now)
            .unwrap_or(true);
        if due {
            match revert(&c, &g).await {
                Ok(()) => log::info!(
                    "network access: timed change on {}/{} ended; restored the previous policy",
                    g.namespace,
                    g.name
                ),
                Err(e) => log::warn!(
                    "network access: ending timed change on {}/{}: {e}",
                    g.namespace,
                    g.name
                ),
            }
        }
    }
    Ok(())
}

// ---- GET /vms/:ns/:name/egress ------------------------------------------------------------

async fn get_egress(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Reply {
    require_kairon()?;
    let c = ctx(&state).await;
    let m = get_machine(&c, &ns, &name).await?;
    let relay = relay_for(&c, &m).await;
    let live = live_policy(relay.as_ref()).await;
    let flows = observed_flows(relay.as_ref()).await.unwrap_or_default();
    let (blocked, would_block) = flow_counts(&flows);

    let configured = desired_of(&m);
    // A VM with no policy of its own follows the node default; use what the node reports.
    let live_policy_v = live.reply.as_ref().and_then(|v| policy_from(&v["policy"]));
    let effective = configured
        .clone()
        .or_else(|| live_policy_v.clone())
        .unwrap_or_default();
    let programmed = live
        .reply
        .as_ref()
        .and_then(|v| v.get("programmed"))
        .filter(|v| !v.is_null())
        .and_then(policy_from);
    let agent = if m.is_running() {
        crate::api::kairon_ops::guest_status(&c.kube, &ns, &name)
            .await
            .ok()
            .and_then(|v| v["connected"].as_bool())
    } else {
        None
    };
    let ep = platform_endpoints();
    let template = template_of(&m);
    let problems: Vec<Problem> = detect_problems(
        &ProblemInput {
            desired: &effective,
            programmed: programmed.as_ref(),
            running: m.is_running(),
            live_known: live.reply.is_some(),
            agent_connected: agent,
            blocked_flows: blocked,
        },
        &ep,
        template.as_deref(),
    );
    let grant = read_grant(&c, &ns, &name).await;
    ok_json(json!({
        "namespace": ns,
        "name": name,
        "running": m.is_running(),
        "configured": configured.is_some(),
        "source": if configured.is_some() { "vm" } else { "node-default" },
        "policy": effective,
        "programmed": programmed,
        "live_error": live.error,
        "agent_connected": agent,
        "observed": {"blocked": blocked, "would_block": would_block, "flows": flows},
        "problems": problems,
        "grant": grant,
        "template": template,
        "endpoints": ep,
    }))
}

// ---- PUT / DELETE ----------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SetBody {
    preset: Option<String>,
    mode: Option<EgressMode>,
    rules: Vec<EgressRule>,
    /// Revert after this long (`30m`, `2h`, `1d`).
    expires_in: Option<String>,
}

impl SetBody {
    fn access(&self) -> NetworkAccessRequest {
        NetworkAccessRequest {
            preset: self.preset.clone(),
            mode: self.mode,
            rules: self.rules.clone(),
        }
    }
}

fn unprocessable(msg: String) -> Response {
    err(StatusCode::UNPROCESSABLE_ENTITY, msg)
}

async fn put_egress(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(body): Json<SetBody>,
) -> Reply {
    require_kairon()?;
    let c = ctx(&state).await;
    let m = get_machine(&c, &ns, &name).await?;
    let template = template_of(&m);
    let policy = body
        .access()
        .resolve(&platform_endpoints(), template.as_deref())
        .map_err(unprocessable)?;
    let expires = body
        .expires_in
        .as_deref()
        .map(parse_duration)
        .transpose()
        .map_err(unprocessable)?;

    let previous = desired_of(&m);
    let updated = patch_egress(&c, &m, Some(&policy)).await?;
    let grant = match expires {
        Some(secs) => {
            let now = chrono::Utc::now();
            let g = Grant {
                namespace: ns.clone(),
                name: name.clone(),
                expires_at: (now + chrono::Duration::seconds(secs as i64)).to_rfc3339(),
                created_at: now.to_rfc3339(),
                revert_to: previous,
            };
            if let Err(e) = save_grant(&c, &g).await {
                // Do not leave a "temporary" change that will never end.
                let _ = patch_egress(&c, &updated, g.revert_to.as_ref()).await;
                return Err(e);
            }
            Some(g)
        }
        None => {
            // A permanent change replaces any pending timed one.
            forget_grant(&c, &ns, &name).await;
            None
        }
    };
    log::info!(
        "network access: {ns}/{name} set to {:?} with {} rule(s){}",
        policy.mode,
        policy.rules.len(),
        grant
            .as_ref()
            .map(|g| format!(" until {}", g.expires_at))
            .unwrap_or_default()
    );
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "namespace": ns,
            "name": name,
            "policy": policy,
            "grant": grant,
            "note": "saved on the Machine; kairon-node applies it to the running VM on its next reconcile",
        })),
    )
        .into_response())
}

async fn delete_egress(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Reply {
    require_kairon()?;
    let c = ctx(&state).await;
    let m = get_machine(&c, &ns, &name).await?;
    if desired_of(&m).is_some() {
        patch_egress(&c, &m, None).await?;
    }
    forget_grant(&c, &ns, &name).await;
    ok_json(json!({
        "namespace": ns,
        "name": name,
        "policy": null,
        "note": "the VM follows the node default again",
    }))
}

/// End a timed change now and restore what the VM had before it.
async fn delete_grant(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Reply {
    require_kairon()?;
    let c = ctx(&state).await;
    let g = read_grant(&c, &ns, &name).await.ok_or_else(|| {
        err(
            StatusCode::NOT_FOUND,
            "no timed change is pending for this VM",
        )
    })?;
    revert(&c, &g)
        .await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, e))?;
    ok_json(json!({"namespace": ns, "name": name, "restored": g.revert_to}))
}

// ---- observed, suggest, explain ---------------------------------------------------------

async fn running_relay(state: &SharedState, ns: &str, name: &str) -> Result<RelayTarget, Response> {
    let c = ctx(state).await;
    let m = get_machine(&c, ns, name).await?;
    relay_for(&c, &m).await.ok_or_else(|| {
        err(
            StatusCode::CONFLICT,
            "the VM is not running, so it has no live network activity to report",
        )
    })
}

async fn get_observed(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Reply {
    require_kairon()?;
    let t = running_relay(&state, &ns, &name).await?;
    t.call_json(Method::GET, "vm-egress-observed", &[], "", None)
        .await
        .map(|v| Json(v).into_response())
        .map_err(|e| err(StatusCode::BAD_GATEWAY, short_err(&e)))
}

/// Rules that would allow what the guest tried to reach, with a preview of the merged policy.
/// Read-only: applying is a `PUT …/egress` with the rules you accept.
async fn post_suggest(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Reply {
    require_kairon()?;
    let c = ctx(&state).await;
    let m = get_machine(&c, &ns, &name).await?;
    let relay = relay_for(&c, &m).await.ok_or_else(|| {
        err(
            StatusCode::CONFLICT,
            "the VM is not running, so there is nothing to learn from",
        )
    })?;
    let v = relay
        .call_json(Method::POST, "vm-egress-suggest", &[], "", None)
        .await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, short_err(&e)))?;
    let rules: Vec<EgressRule> =
        serde_json::from_value(v.get("rules").cloned().unwrap_or(json!([]))).map_err(|e| {
            err(
                StatusCode::BAD_GATEWAY,
                format!("unexpected node reply: {e}"),
            )
        })?;
    let current = desired_of(&m).unwrap_or_default();
    let merged = merge_suggestions(&current, &rules);
    let (valid, problem) = match validate(&merged) {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e)),
    };
    ok_json(json!({
        "rules": rules,
        "merged": merged,
        "valid": valid,
        "problem": problem,
    }))
}

#[derive(Debug, Deserialize)]
struct ExplainBody {
    host: String,
    port: u16,
    #[serde(default)]
    proto: Option<EgressProto>,
}

fn host_ok(h: &str) -> bool {
    !h.is_empty()
        && h.len() <= 253
        && h.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

async fn resolve_v4(host: &str) -> Vec<Ipv4Addr> {
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return vec![ip];
    }
    match tokio::time::timeout(Duration::from_secs(4), tokio::net::lookup_host((host, 0))).await {
        Ok(Ok(addrs)) => {
            let mut v: Vec<Ipv4Addr> = addrs
                .filter_map(|a| match a.ip() {
                    IpAddr::V4(i) => Some(i),
                    IpAddr::V6(_) => None,
                })
                .collect();
            v.sort();
            v.dedup();
            v
        }
        _ => vec![],
    }
}

/// Would this VM's policy let it reach `host:port`? Names are resolved by Veyron (the guest's
/// resolver may differ), each address gets its own verdict.
async fn post_explain(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(body): Json<ExplainBody>,
) -> Reply {
    explain_reply(&state, &ns, &name, body).await
}

/// Same as the POST, for read-only callers (the AI tools and plain links):
/// `GET …/egress/explain?host=archive.ubuntu.com&port=443&proto=tcp`.
async fn get_explain(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Query(body): Query<ExplainBody>,
) -> Reply {
    explain_reply(&state, &ns, &name, body).await
}

async fn explain_reply(state: &SharedState, ns: &str, name: &str, body: ExplainBody) -> Reply {
    let (state, ns, name) = (state, ns.to_string(), name.to_string());
    require_kairon()?;
    if !host_ok(body.host.trim()) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "host is required (name or IPv4 address)",
        ));
    }
    if body.port == 0 {
        return Err(err(StatusCode::BAD_REQUEST, "port must be 1-65535"));
    }
    let c = ctx(state).await;
    let m = get_machine(&c, &ns, &name).await?;
    let policy = match desired_of(&m) {
        Some(p) => p,
        None => {
            let relay = relay_for(&c, &m).await;
            live_policy(relay.as_ref())
                .await
                .reply
                .and_then(|v| policy_from(&v["policy"]))
                .unwrap_or_default()
        }
    };
    let proto = body.proto.unwrap_or(EgressProto::Tcp);
    let host = body.host.trim().to_string();
    let addrs = resolve_v4(&host).await;
    if addrs.is_empty() {
        return Err(err(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("{host} did not resolve to an IPv4 address"),
        ));
    }
    let verdicts: Vec<_> = addrs
        .iter()
        .map(|ip| explain(&policy, *ip, proto, body.port))
        .collect();
    let allowed = verdicts.iter().all(|v| v.allowed);
    ok_json(json!({
        "host": host,
        "port": body.port,
        "proto": proto,
        "allowed": allowed,
        "mode": policy.mode,
        "verdicts": verdicts,
    }))
}

// ---- presets ------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct PresetQuery {
    template: Option<String>,
}

async fn get_presets(Query(q): Query<PresetQuery>) -> Reply {
    let ep = platform_endpoints();
    let list = presets(&ep, q.template.as_deref());
    ok_json(json!({
        "presets": list,
        "endpoints": ep,
        "default": crate::kairon::egress::BUILTIN_DEFAULT_PRESET,
    }))
}

// ---- node defaults -------------------------------------------------------------------------

async fn node_target(state: &SharedState, node: &str) -> Result<RelayTarget, Response> {
    require_kairon()?;
    let c = ctx(state).await;
    RelayTarget::for_node(&c.kube.client(), node)
        .await
        .map_err(|e| err(StatusCode::SERVICE_UNAVAILABLE, format!("{e:#}")))
}

async fn get_defaults(State(state): State<SharedState>, Path(node): Path<String>) -> Reply {
    let t = node_target(&state, &node).await?;
    t.call_node(Method::GET, "egress-defaults", "", None)
        .await
        .map(|v| Json(v).into_response())
        .map_err(|e| err(StatusCode::BAD_GATEWAY, short_err(&e)))
}

#[derive(Debug, Default, Deserialize)]
struct DefaultsQuery {
    #[serde(default)]
    apply_running: bool,
}

/// Replace a node's default policy (what VMs without a policy of their own get).
async fn put_defaults(
    State(state): State<SharedState>,
    Path(node): Path<String>,
    Query(q): Query<DefaultsQuery>,
    Json(body): Json<SetBody>,
) -> Reply {
    let policy = body
        .access()
        .resolve(&platform_endpoints(), None)
        .map_err(unprocessable)?;
    let t = node_target(&state, &node).await?;
    let query = if q.apply_running {
        "apply_running=true"
    } else {
        ""
    };
    t.call_node(
        Method::PUT,
        "egress-defaults",
        query,
        Some(&serde_json::to_value(&policy).unwrap_or_default()),
    )
    .await
    .map(|v| Json(v).into_response())
    .map_err(|e| err(StatusCode::BAD_GATEWAY, short_err(&e)))
}

// ---- fleet --------------------------------------------------------------------------------

fn mode_name(p: Option<&EgressPolicy>) -> &'static str {
    match p.map(|p| p.mode) {
        None => "inherit",
        Some(EgressMode::Loopback) => "loopback",
        Some(EgressMode::Allowlist) => "allowlist",
        Some(EgressMode::Audit) => "audit",
        Some(EgressMode::Open) => "open",
    }
}

#[derive(Debug, Default, Deserialize)]
struct OverviewQuery {
    namespace: Option<String>,
    /// Also ask each running VM's node for blocked flows and agent state (slower).
    #[serde(default)]
    live: bool,
}

async fn list_machines(c: &Ctx, ns: Option<&str>) -> Result<Vec<Machine>, Response> {
    let api = match ns.filter(|n| *n != "all") {
        Some(n) => Api::<Machine>::namespaced(c.kube.client(), n),
        None => Api::<Machine>::all(c.kube.client()),
    };
    api.list(&ListParams::default())
        .await
        .map(|l| l.items)
        .map_err(kube_err)
}

/// Blocked-flow count and agent state for one running VM; `None`s when the node cannot say
/// (old FluxVM, relay down, or slow: each probe gets six seconds).
type Probe = (Option<usize>, Option<bool>);

async fn probe_machine(c: Ctx, m: Machine) -> Probe {
    let (c, m) = (&c, &m);
    let probe = async {
        let relay = relay_for(c, m).await?;
        let flows = observed_flows(Some(&relay)).await?;
        let ns = m.metadata.namespace.clone().unwrap_or_default();
        let name = m.metadata.name.clone().unwrap_or_default();
        let agent = crate::api::kairon_ops::guest_status(&c.kube, &ns, &name)
            .await
            .ok()
            .and_then(|v| v["connected"].as_bool());
        Some((flow_counts(&flows).0, agent))
    };
    match tokio::time::timeout(Duration::from_secs(6), probe).await {
        Ok(Some((b, a))) => (Some(b), a),
        _ => (None, None),
    }
}

async fn get_overview(State(state): State<SharedState>, Query(q): Query<OverviewQuery>) -> Reply {
    require_kairon()?;
    let c = ctx(&state).await;
    let machines = list_machines(&c, q.namespace.as_deref()).await?;
    let mut by_mode: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut items = Vec::new();
    for m in &machines {
        let desired = desired_of(m);
        *by_mode.entry(mode_name(desired.as_ref())).or_default() += 1;
        items.push(json!({
            "namespace": m.metadata.namespace,
            "name": m.metadata.name,
            "phase": m.phase(),
            "running": m.is_running(),
            "mode": mode_name(desired.as_ref()),
            "rules": desired.as_ref().map(|p| p.rules.len()),
            "template": template_of(m),
            "blocked": Value::Null,
            "agent_connected": Value::Null,
        }));
    }
    if q.live {
        use futures_util::StreamExt;
        let running: Vec<(usize, Machine)> = machines
            .iter()
            .enumerate()
            .filter(|(_, m)| m.is_running())
            .take(200)
            .map(|(i, m)| (i, m.clone()))
            .collect();
        // Owned values only: borrowed ones inside a stream closure trip higher-ranked lifetime
        // inference when the handler future must be `Send`.
        let probes: Vec<_> = running
            .into_iter()
            .map(|(i, m)| {
                let owned = Ctx {
                    kube: c.kube.clone(),
                    api_ns: c.api_ns.clone(),
                };
                async move { (i, probe_machine(owned, m).await) }
            })
            .collect();
        let results: Vec<(usize, Probe)> = futures_util::stream::iter(probes)
            .buffer_unordered(8)
            .collect()
            .await;
        for (i, (blocked, agent)) in results {
            items[i]["blocked"] = json!(blocked);
            items[i]["agent_connected"] = json!(agent);
        }
    }
    let with_blocked = items
        .iter()
        .filter(|i| i["blocked"].as_u64().unwrap_or(0) > 0)
        .count();
    let no_agent = items
        .iter()
        .filter(|i| i["running"] == true && i["agent_connected"] == false)
        .count();
    ok_json(json!({
        "total": machines.len(),
        "by_mode": by_mode,
        "with_blocked_flows": with_blocked,
        "running_without_agent": no_agent,
        "live": q.live,
        "items": items,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyBody {
    selector: Selector,
    #[serde(flatten)]
    change: SetBody,
    /// Default true: report what would change without changing it.
    #[serde(default = "yes")]
    dry_run: bool,
}

fn yes() -> bool {
    true
}

/// Change the network access of many VMs at once. A dry run (the default) lists what would
/// change; at most [`FLEET_MAX`] VMs per call.
async fn post_apply(State(state): State<SharedState>, Json(body): Json<ApplyBody>) -> Reply {
    require_kairon()?;
    if body.selector.is_empty() {
        return Err(unprocessable(
            "select VMs by namespaces, names or labels, or set \"all\": true".into(),
        ));
    }
    let expires = body
        .change
        .expires_in
        .as_deref()
        .map(parse_duration)
        .transpose()
        .map_err(unprocessable)?;
    let c = ctx(&state).await;
    let ep = platform_endpoints();
    let machines = list_machines(&c, None).await?;
    let targets: Vec<&Machine> = machines
        .iter()
        .filter(|m| {
            body.selector.matches(
                m.metadata.namespace.as_deref().unwrap_or(""),
                m.metadata.name.as_deref().unwrap_or(""),
                &m.metadata.labels.clone().unwrap_or_default(),
            )
        })
        .collect();
    if targets.len() > FLEET_MAX {
        return Err(unprocessable(format!(
            "{} VMs match; narrow the selector to at most {FLEET_MAX}",
            targets.len()
        )));
    }
    let mut results = Vec::new();
    let (mut changed, mut failed) = (0usize, 0usize);
    for m in targets {
        let ns = m.metadata.namespace.clone().unwrap_or_default();
        let name = m.metadata.name.clone().unwrap_or_default();
        let from = desired_of(m);
        let policy = match body.change.access().resolve(&ep, template_of(m).as_deref()) {
            Ok(p) => p,
            Err(e) => {
                failed += 1;
                results.push(json!({"namespace": ns, "name": name, "ok": false, "error": e}));
                continue;
            }
        };
        let unchanged = from.as_ref() == Some(&policy);
        let mut entry = json!({
            "namespace": ns,
            "name": name,
            "from": mode_name(from.as_ref()),
            "to": mode_name(Some(&policy)),
            "rules": policy.rules.len(),
            "unchanged": unchanged,
        });
        if !body.dry_run && !unchanged {
            match patch_egress(&c, m, Some(&policy)).await {
                Ok(_) => {
                    if let Some(secs) = expires {
                        let now = chrono::Utc::now();
                        let g = Grant {
                            namespace: ns.clone(),
                            name: name.clone(),
                            expires_at: (now + chrono::Duration::seconds(secs as i64)).to_rfc3339(),
                            created_at: now.to_rfc3339(),
                            revert_to: from.clone(),
                        };
                        if save_grant(&c, &g).await.is_err() {
                            entry["warning"] = json!("applied, but the timer could not be saved");
                        }
                    }
                    changed += 1;
                    entry["ok"] = json!(true);
                }
                Err(resp) => {
                    failed += 1;
                    entry["ok"] = json!(false);
                    entry["error"] = json!(format!("{}", resp.status()));
                }
            }
        }
        results.push(entry);
    }
    ok_json(json!({
        "dry_run": body.dry_run,
        "matched": results.len(),
        "changed": changed,
        "failed": failed,
        "results": results,
    }))
}

#[cfg(test)]
fn preset_exists(id: &str) -> bool {
    crate::kairon::egress::preset(id, &platform_endpoints(), None).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_pins_resource_version_and_null_clears() {
        let p = EgressPolicy {
            mode: EgressMode::Allowlist,
            rules: vec![EgressRule::new(
                "10.0.0.0/8",
                EgressProto::Tcp,
                Some(5151),
                "",
            )],
        };
        let v = egress_patch(Some("42"), Some(&p));
        assert_eq!(v["metadata"]["resourceVersion"], "42");
        assert_eq!(v["spec"]["network"]["egress"]["mode"], "allowlist");
        assert_eq!(
            v["spec"]["network"]["egress"]["rules"][0]["cidr"],
            "10.0.0.0/8"
        );
        let clear = egress_patch(Some("7"), None);
        assert!(clear["spec"]["network"]["egress"].is_null());
    }

    #[test]
    fn grant_roundtrips_through_a_configmap_and_keeps_its_revert_target() {
        let g = Grant {
            namespace: "default".into(),
            name: "vm1".into(),
            expires_at: "2030-01-01T00:00:00+00:00".into(),
            created_at: "2029-12-31T00:00:00+00:00".into(),
            revert_to: Some(EgressPolicy::default()),
        };
        let cm = grant_cm(&g);
        assert_eq!(
            cm.metadata.name.as_deref(),
            Some("egress-grant-default.vm1")
        );
        let back = grant_from(&cm).unwrap();
        assert_eq!(back.revert_to, Some(EgressPolicy::default()));
        assert_eq!(
            cm.metadata
                .labels
                .unwrap()
                .get(GRANT_LABEL)
                .map(String::as_str),
            Some(GRANT_VALUE)
        );
        let none = Grant {
            revert_to: None,
            ..g
        };
        assert!(grant_from(&grant_cm(&none)).unwrap().revert_to.is_none());
    }

    #[test]
    fn set_body_rejects_unknown_fields_and_resolves() {
        assert!(serde_json::from_value::<SetBody>(json!({"presett": "web"})).is_err());
        let b: SetBody = serde_json::from_value(json!({
            "preset": "platform",
            "expires_in": "1h",
            "rules": [{"cidr": "203.0.113.0/24", "proto": "tcp", "port": 8443}]
        }))
        .unwrap();
        let p = b.access().resolve(&platform_endpoints(), None).unwrap();
        assert!(p.rules.iter().any(|r| r.cidr == "203.0.113.0/24"));
        assert_eq!(
            parse_duration(b.expires_in.as_deref().unwrap()).unwrap(),
            3600
        );
    }

    #[test]
    fn apply_body_defaults_to_a_dry_run() {
        let b: ApplyBody =
            serde_json::from_value(json!({"selector": {"namespaces": ["a"]}, "preset": "web"}))
                .unwrap();
        assert!(b.dry_run);
        assert_eq!(b.change.preset.as_deref(), Some("web"));
        let off: ApplyBody = serde_json::from_value(
            json!({"selector": {"all": true}, "preset": "web", "dry_run": false}),
        )
        .unwrap();
        assert!(!off.dry_run);
    }

    #[test]
    fn host_validation() {
        assert!(host_ok("archive.ubuntu.com"));
        assert!(host_ok("10.0.0.1"));
        assert!(!host_ok(""));
        assert!(!host_ok("a b"));
        assert!(!host_ok("a/b"));
        assert!(!host_ok(&"x".repeat(300)));
    }

    #[test]
    fn flow_counts_split_blocked_from_would_block() {
        let flows = vec![
            json!({"dst": "1.1.1.1", "audit": false}),
            json!({"dst": "2.2.2.2", "audit": true}),
            json!({"dst": "3.3.3.3"}),
        ];
        assert_eq!(flow_counts(&flows), (2, 1));
    }

    #[test]
    fn mode_names() {
        assert_eq!(mode_name(None), "inherit");
        assert_eq!(mode_name(Some(&EgressPolicy::default())), "loopback");
    }

    #[test]
    fn every_builtin_preset_exists() {
        for id in [
            "isolated",
            "platform",
            "platform-packages",
            "web",
            "private",
            "learn",
            "open",
        ] {
            assert!(preset_exists(id), "{id}");
        }
    }

    #[test]
    fn friendly_error_for_an_old_node() {
        let e = anyhow::anyhow!("kairon-node relay /vm-egress returned 404 Not Found: ");
        assert!(short_err(&e).contains("update FluxVM"));
    }
}
