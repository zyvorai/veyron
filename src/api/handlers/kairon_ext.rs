// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Kairon-only VM controls that live on CRDs rather than the relay: migration
//! recovery, idle/lifetime lifecycle, placement, per-VM network policy, and
//! read-only views of claims, quotas, disruption budgets and migration policies.

// The Err side is a ready-to-send HTTP response, returned straight to the client.
#![allow(clippy::result_large_err)]

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kube::api::{
    Api, ApiResource, DeleteParams, DynamicObject, ListParams, Patch, PatchParams, PostParams,
};
use kube::core::GroupVersionKind;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::http_server::web::SharedState;
use crate::kairon::{
    LABEL_MANAGED_BY, LifecycleSpec, Machine, MachineMigration, MigrationRecovery, PlacementSpec,
};

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route(
            "/vms/:ns/:name/migrations/:mig/recovery",
            post(migration_recovery),
        )
        .route(
            "/vms/:ns/:name/lifecycle",
            get(get_lifecycle).put(put_lifecycle),
        )
        .route(
            "/vms/:ns/:name/placement",
            get(get_placement).put(put_placement),
        )
        .route(
            "/vms/:ns/:name/network-policy",
            get(get_policy).put(put_policy).delete(delete_policy),
        )
        .route("/kairon/:kind", get(list_kind))
        .with_state(state)
}

fn err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "error": msg.into() }))).into_response()
}

/// Kubernetes API errors keep their status (admission-webhook denials are 4xx).
fn kube_err(e: kube::Error) -> Response {
    match &e {
        kube::Error::Api(ae) => {
            let code = match ae.code {
                404 => StatusCode::NOT_FOUND,
                409 => StatusCode::CONFLICT,
                403 | 400 | 422 => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::BAD_GATEWAY,
            };
            err(code, ae.message.clone())
        }
        _ => err(StatusCode::BAD_GATEWAY, e.to_string()),
    }
}

async fn client(state: &SharedState) -> Result<kube::Client, Response> {
    if !crate::api::vm_backend::is_kairon() {
        return Err(err(
            StatusCode::NOT_IMPLEMENTED,
            "this operation requires the Kairon backend",
        ));
    }
    Ok(state.read().await.kube_client.client())
}

fn machines(c: &kube::Client, ns: &str) -> Api<Machine> {
    Api::namespaced(c.clone(), ns)
}

// ── Migration recovery ─────────────────────────────────────────────

const RECOVERY_ACTIONS: &[&str] = &[
    "ConfirmDestinationCommitted",
    "ConfirmDestinationNotCommitted",
    "ForceAbort",
];
const RECOVERY_DIAGNOSES: &[&str] = &["DestinationCommitted", "DestinationNotCommitted", "Unknown"];

fn validate_recovery(r: &MigrationRecovery) -> Result<(), String> {
    if !RECOVERY_ACTIONS.contains(&r.action.as_str()) {
        return Err(format!("action must be one of {RECOVERY_ACTIONS:?}"));
    }
    if !RECOVERY_DIAGNOSES.contains(&r.acknowledged_diagnosis.as_str()) {
        return Err(format!(
            "acknowledged_diagnosis must be one of {RECOVERY_DIAGNOSES:?}"
        ));
    }
    if r.reason.trim().is_empty() {
        return Err("reason is required".into());
    }
    Ok(())
}

#[derive(Deserialize)]
struct RecoveryBody {
    action: String,
    acknowledged_diagnosis: String,
    reason: String,
}

async fn migration_recovery(
    State(state): State<SharedState>,
    Path((ns, name, mig)): Path<(String, String, String)>,
    Json(body): Json<RecoveryBody>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let rec = MigrationRecovery {
        action: body.action,
        acknowledged_diagnosis: body.acknowledged_diagnosis,
        reason: body.reason,
    };
    if let Err(m) = validate_recovery(&rec) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, m);
    }
    let api: Api<MachineMigration> = Api::namespaced(c, &ns);
    let existing = match api.get(&mig).await {
        Ok(m) => m,
        Err(e) => return kube_err(e),
    };
    if existing.spec.machine_name != name {
        return err(
            StatusCode::NOT_FOUND,
            format!("migration {mig} does not belong to VM {name}"),
        );
    }
    let patch = json!({ "spec": { "recovery": rec } });
    match api
        .patch(&mig, &PatchParams::default(), &Patch::Merge(&patch))
        .await
    {
        Ok(_) => Json(json!({ "migration": mig, "recovery": rec })).into_response(),
        Err(e) => kube_err(e),
    }
}

// ── Lifecycle / placement ──────────────────────────────────────────

async fn get_machine(c: &kube::Client, ns: &str, name: &str) -> Result<Machine, Response> {
    machines(c, ns).get(name).await.map_err(kube_err)
}

async fn get_lifecycle(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    match get_machine(&c, &ns, &name).await {
        Ok(m) => Json(json!({ "lifecycle": m.spec.lifecycle })).into_response(),
        Err(r) => r,
    }
}

const IDLE_ACTIONS: &[&str] = &["Pause", "Hibernate", "Halt", "Stop", "Delete"];

fn validate_lifecycle(l: &LifecycleSpec) -> Result<(), String> {
    if let Some(a) = &l.idle_action
        && !IDLE_ACTIONS.contains(&a.as_str())
    {
        return Err(format!("idle_action must be one of {IDLE_ACTIONS:?}"));
    }
    if l.idle_timeout_seconds == Some(0) || l.max_lifetime_seconds == Some(0) {
        return Err("timeouts must be positive; omit a field to leave it unset".into());
    }
    Ok(())
}

/// `PUT` replaces the whole block; an empty object clears it.
async fn put_lifecycle(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(body): Json<LifecycleSpec>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    if let Err(m) = validate_lifecycle(&body) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, m);
    }
    let value = if body == LifecycleSpec::default() {
        Value::Null
    } else {
        json!(body)
    };
    let patch = json!({ "spec": { "lifecycle": value } });
    match machines(&c, &ns)
        .patch(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
    {
        Ok(m) => Json(json!({ "lifecycle": m.spec.lifecycle })).into_response(),
        Err(e) => kube_err(e),
    }
}

async fn get_placement(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    match get_machine(&c, &ns, &name).await {
        Ok(m) => Json(json!({
            "placement": m.spec.placement,
            "node": m.status.as_ref().and_then(|s| s.node_name.clone()),
        }))
        .into_response(),
        Err(r) => r,
    }
}

fn validate_placement(p: &PlacementSpec) -> Result<(), String> {
    for t in p
        .affinity
        .iter()
        .chain(&p.anti_affinity)
        .chain(&p.preferred_affinity)
        .chain(&p.preferred_anti_affinity)
    {
        if t.topology_key.as_deref().unwrap_or("").is_empty() {
            return Err("every (anti-)affinity term needs a topology_key".into());
        }
    }
    for t in p
        .preferred_affinity
        .iter()
        .chain(&p.preferred_anti_affinity)
    {
        if !(1..=100).contains(&t.weight.unwrap_or(1)) {
            return Err("preferred term weight must be 1-100".into());
        }
    }
    Ok(())
}

/// Replaces `spec.placement`. Kairon's admission webhook can still refuse it
/// (e.g. a `maxSkew` the fleet cannot meet); that denial comes back as 422.
async fn put_placement(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(body): Json<PlacementSpec>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    if let Err(m) = validate_placement(&body) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, m);
    }
    let value = if body == PlacementSpec::default() {
        Value::Null
    } else {
        json!(body)
    };
    let patch = json!({ "spec": { "placement": value } });
    match machines(&c, &ns)
        .patch(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
    {
        Ok(m) => Json(json!({ "placement": m.spec.placement })).into_response(),
        Err(e) => kube_err(e),
    }
}

// ── Network policy ─────────────────────────────────────────────────

const POLICY_KEYS: &[&str] = &[
    "defaultAllow",
    "allowCidrs",
    "denyCidrs",
    "allowPorts",
    "maxEgressMbps",
    "maxEgressPps",
    "allowFqdns",
    "auditMode",
    "allowIcmp",
    "sampleRate",
    "allowSNI",
    "allowDNS",
    "maxIngressMbps",
    "maxIngressPps",
];

fn dyn_api(c: &kube::Client, ns: &str, kind: &str, plural: &str) -> Api<DynamicObject> {
    let gvk = GroupVersionKind::gvk("kairon.zyvor.dev", "v1beta1", kind);
    Api::namespaced_with(
        c.clone(),
        ns,
        &ApiResource::from_gvk_with_plural(&gvk, plural),
    )
}

fn policy_name(vm: &str) -> String {
    format!("{vm}-netpol")
}

fn validate_policy(p: &Value) -> Result<(), String> {
    let obj = p.as_object().ok_or("policy must be an object")?;
    if let Some(bad) = obj.keys().find(|k| !POLICY_KEYS.contains(&k.as_str())) {
        return Err(format!("unknown policy field '{bad}'"));
    }
    for k in ["maxIngressMbps", "maxIngressPps"] {
        if obj.get(k).and_then(Value::as_u64) == Some(0) {
            return Err(format!("{k} must be at least 1; omit it for no limit"));
        }
    }
    Ok(())
}

async fn get_policy(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    match dyn_api(&c, &ns, "MachineNetworkPolicy", "machinenetworkpolicies")
        .get_opt(&policy_name(&name))
        .await
    {
        Ok(Some(o)) => Json(json!({
            "policy": o.data.get("spec").and_then(|s| s.get("policy")),
            "status": o.data.get("status"),
        }))
        .into_response(),
        Ok(None) => err(StatusCode::NOT_FOUND, "no network policy for this VM"),
        Err(e) => kube_err(e),
    }
}

/// Body is the `policy` block (allowCidrs, allowFqdns, allowSNI, allowDNS,
/// maxEgressMbps, maxIngressMbps, ...); unknown fields are refused so typos
/// don't silently leave a VM open.
async fn put_policy(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(policy): Json<Value>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    if let Err(m) = validate_policy(&policy) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, m);
    }
    if let Err(r) = get_machine(&c, &ns, &name).await {
        return r;
    }
    let api = dyn_api(&c, &ns, "MachineNetworkPolicy", "machinenetworkpolicies");
    let pname = policy_name(&name);
    let spec = json!({ "machineName": name, "policy": policy });
    let res = match api.get_opt(&pname).await {
        // Merge patch would keep stale list entries; replace the whole policy.
        Ok(Some(_)) => {
            api.patch(
                &pname,
                &PatchParams::default(),
                &Patch::Merge(&json!({ "spec": spec })),
            )
            .await
        }
        Ok(None) => {
            let obj: DynamicObject = match serde_json::from_value(json!({
                "apiVersion": "kairon.zyvor.dev/v1beta1",
                "kind": "MachineNetworkPolicy",
                "metadata": {
                    "name": pname,
                    "namespace": ns,
                    "labels": {LABEL_MANAGED_BY: "veyron"},
                },
                "spec": spec,
            })) {
                Ok(o) => o,
                Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            };
            api.create(&PostParams::default(), &obj).await
        }
        Err(e) => Err(e),
    };
    match res {
        Ok(o) => Json(json!({
            "name": o.metadata.name,
            "policy": o.data.get("spec").and_then(|s| s.get("policy")),
        }))
        .into_response(),
        Err(e) => kube_err(e),
    }
}

async fn delete_policy(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    match dyn_api(&c, &ns, "MachineNetworkPolicy", "machinenetworkpolicies")
        .delete(&policy_name(&name), &DeleteParams::default())
        .await
    {
        Ok(_) => Json(json!({ "deleted": policy_name(&name) })).into_response(),
        Err(e) => kube_err(e),
    }
}

// ── Claims, quotas, budgets, migration policies (read-only) ───────

/// URL slug -> (kind, plural) for the read-only views.
const KINDS: &[(&str, &str, &str)] = &[
    ("claims", "MachineClaim", "machineclaims"),
    ("quotas", "MachineQuota", "machinequotas"),
    (
        "disruption-budgets",
        "MachineDisruptionBudget",
        "machinedisruptionbudgets",
    ),
    ("migration-policies", "MigrationPolicy", "migrationpolicies"),
    (
        "backup-policies",
        "MachineBackupPolicy",
        "machinebackuppolicies",
    ),
    (
        "protection-plans",
        "MachineProtectionPlan",
        "machineprotectionplans",
    ),
    (
        "virtual-networks",
        "MachineVirtualNetwork",
        "machinevirtualnetworks",
    ),
    (
        "accelerator-profiles",
        "MachineAcceleratorProfile",
        "machineacceleratorprofiles",
    ),
    (
        "balance-policies",
        "MachineBalancePolicy",
        "machinebalancepolicies",
    ),
    ("autoscalers", "MachineAutoscaler", "machineautoscalers"),
    (
        "recovery-plans",
        "MachineRecoveryPlan",
        "machinerecoveryplans",
    ),
    ("ha-profiles", "MachineHAProfile", "machinehaprofiles"),
    ("usage-ledgers", "MachineUsageLedger", "machineusageledgers"),
    ("node-topologies", "NodeTopology", "nodetopologies"),
];

fn kind_info(slug: &str) -> Option<(&'static str, &'static str)> {
    KINDS
        .iter()
        .find(|(s, _, _)| *s == slug)
        .map(|(_, k, p)| (*k, *p))
}

#[derive(Deserialize)]
struct NsQuery {
    namespace: Option<String>,
}

async fn list_kind(
    State(state): State<SharedState>,
    Path(kind): Path<String>,
    Query(q): Query<NsQuery>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let Some((k, plural)) = kind_info(&kind) else {
        let known: Vec<&str> = KINDS.iter().map(|(s, _, _)| *s).collect();
        return err(
            StatusCode::NOT_FOUND,
            format!("kind must be one of {}", known.join(", ")),
        );
    };
    let gvk = GroupVersionKind::gvk("kairon.zyvor.dev", "v1beta1", k);
    let ar = ApiResource::from_gvk_with_plural(&gvk, plural);
    let api: Api<DynamicObject> = match q.namespace.as_deref() {
        None | Some("all") | Some("") => Api::all_with(c, &ar),
        Some(ns) => Api::namespaced_with(c, ns, &ar),
    };
    match api.list(&ListParams::default()).await {
        Ok(l) => Json(json!({
            "kind": k,
            "items": l.items.iter().map(|o| json!({
                "name": o.metadata.name,
                "namespace": o.metadata.namespace,
                "spec": o.data.get("spec"),
                "status": o.data.get("status"),
            })).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(e) => kube_err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kairon::AffinityTerm;

    #[test]
    fn recovery_requires_known_action_and_reason() {
        let ok = MigrationRecovery {
            action: "ForceAbort".into(),
            acknowledged_diagnosis: "Unknown".into(),
            reason: "node lost".into(),
        };
        assert!(validate_recovery(&ok).is_ok());
        let mut bad = ok.clone();
        bad.action = "Nuke".into();
        assert!(validate_recovery(&bad).is_err());
        let mut bad = ok;
        bad.reason = "  ".into();
        assert!(validate_recovery(&bad).is_err());
    }

    #[test]
    fn lifecycle_rejects_unknown_idle_action_and_zero() {
        let l = LifecycleSpec {
            idle_action: Some("Explode".into()),
            ..Default::default()
        };
        assert!(validate_lifecycle(&l).is_err());
        let l = LifecycleSpec {
            idle_timeout_seconds: Some(0),
            ..Default::default()
        };
        assert!(validate_lifecycle(&l).is_err());
        let l = LifecycleSpec {
            idle_action: Some("Pause".into()),
            idle_timeout_seconds: Some(300),
            ..Default::default()
        };
        assert!(validate_lifecycle(&l).is_ok());
    }

    #[test]
    fn placement_terms_need_topology_key_and_sane_weight() {
        let mut p = PlacementSpec::default();
        p.anti_affinity.push(AffinityTerm::default());
        assert!(validate_placement(&p).is_err());
        let mut p = PlacementSpec::default();
        p.preferred_affinity.push(AffinityTerm {
            weight: Some(500),
            topology_key: Some("kubernetes.io/hostname".into()),
            ..Default::default()
        });
        assert!(validate_placement(&p).is_err());
        p.preferred_affinity[0].weight = Some(50);
        assert!(validate_placement(&p).is_ok());
    }

    #[test]
    fn every_listed_kind_is_a_known_plural() {
        for (_, _, plural) in KINDS {
            assert!(crate::kairon::ALL_PLURALS.contains(plural), "{plural}");
        }
        assert!(kind_info("nope").is_none());
    }

    #[test]
    fn policy_refuses_unknown_fields() {
        assert!(validate_policy(&json!({"allowFqdns": ["a.com"]})).is_ok());
        assert!(validate_policy(&json!({"allowFqdn": ["a.com"]})).is_err());
        assert!(validate_policy(&json!({"maxIngressMbps": 0})).is_err());
        assert!(validate_policy(&json!([])).is_err());
    }
}
