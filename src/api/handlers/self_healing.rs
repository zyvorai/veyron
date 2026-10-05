// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Self-healing: evaluate VM health and remediate unhealthy VMs (restart a VMI
//! that has entered a Failed phase). Callable on demand or on a schedule via the
//! automation/cron surface; dry-run by default-safe (explicit `heal` required).

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::post,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use super::namespace_scope;
#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Deserialize)]
pub struct HealQuery {
    #[serde(default)]
    pub namespace: Option<String>,
    /// Actually restart unhealthy VMs. Omitted/false = dry run (report only).
    #[serde(default)]
    pub heal: bool,
}

#[derive(Debug, Serialize)]
pub struct HealAction {
    pub namespace: String,
    pub vm: String,
    pub phase: String,
    pub action: String,
    pub healed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/self-healing/run", post(run_self_healing))
        .route(
            "/self-healing/policy",
            axum::routing::get(get_policy).post(set_policy),
        )
        .with_state(state)
}

// ── Policy (ConfigMap-persisted; drives the background reconciler) ──────────

const POLICY_NS: &str = "veyron-system";
const POLICY_NAME: &str = "veyron-self-healing-policy";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelfHealingPolicy {
    /// When true, the background reconciler restarts unhealthy VMs each tick.
    #[serde(default)]
    pub enabled: bool,
    /// Comma-separated namespaces, or `all` / empty for cluster-wide.
    #[serde(default)]
    pub namespaces: Option<String>,
    /// Minimum seconds between heals of the same VM (loop guard). Default 600.
    #[serde(default)]
    pub cooldown_secs: u64,
}

#[cfg(feature = "web")]
pub async fn read_policy(client: &kube::Client) -> SelfHealingPolicy {
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::Api;
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), POLICY_NS);
    match api.get_opt(POLICY_NAME).await {
        Ok(Some(cm)) => {
            let d = cm.data.unwrap_or_default();
            SelfHealingPolicy {
                enabled: d.get("enabled").map(|v| v == "true").unwrap_or(false),
                namespaces: d.get("namespaces").cloned(),
                cooldown_secs: d
                    .get("cooldown_secs")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(600),
            }
        }
        _ => SelfHealingPolicy::default(),
    }
}

#[cfg(feature = "web")]
async fn write_policy(client: &kube::Client, p: &SelfHealingPolicy) -> anyhow::Result<()> {
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::{Api, ObjectMeta, Patch, PatchParams};
    use std::collections::BTreeMap;

    let mut data = BTreeMap::new();
    data.insert("enabled".to_string(), p.enabled.to_string());
    if let Some(ns) = p.namespaces.as_ref().filter(|s| !s.trim().is_empty()) {
        data.insert("namespaces".to_string(), ns.clone());
    }
    data.insert(
        "cooldown_secs".to_string(),
        if p.cooldown_secs == 0 {
            600
        } else {
            p.cooldown_secs
        }
        .to_string(),
    );
    let cm = ConfigMap {
        metadata: ObjectMeta {
            name: Some(POLICY_NAME.to_string()),
            namespace: Some(POLICY_NS.to_string()),
            labels: Some(BTreeMap::from([(
                "veyron.io/type".to_string(),
                "self-healing".to_string(),
            )])),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), POLICY_NS);
    api.patch(
        POLICY_NAME,
        &PatchParams::apply("veyron-selfheal").force(),
        &Patch::Apply(&cm),
    )
    .await?;
    Ok(())
}

#[cfg(feature = "web")]
async fn get_policy(State(state): State<SharedState>) -> Json<SelfHealingPolicy> {
    let client = { state.read().await.kube_client.client().clone() };
    Json(read_policy(&client).await)
}

#[cfg(feature = "web")]
async fn set_policy(
    State(state): State<SharedState>,
    Json(policy): Json<SelfHealingPolicy>,
) -> Json<serde_json::Value> {
    let client = { state.read().await.kube_client.client().clone() };
    match write_policy(&client, &policy).await {
        Ok(()) => Json(serde_json::json!({ "ok": true, "policy": policy })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

/// Annotation persisting the last self-heal restart time on the VM object
/// itself, in RFC 3339. An in-process cooldown map doesn't survive a leader
/// failover of the lease-guarded scheduler loop — the newly-elected replica
/// would immediately re-heal a VM still inside its cooldown window. The VM
/// object outlives any one replica, so this is the source of truth.
const LAST_HEAL_ANNOTATION: &str = "veyron.io/last-self-heal";

/// Background reconciler tick: when the policy is enabled, restart unhealthy
/// VMs, respecting a per-VM cooldown persisted via [`LAST_HEAL_ANNOTATION`].
/// Called from the lease-guarded scheduler loop so only the leader replica
/// heals.
#[cfg(feature = "web")]
pub async fn self_healing_tick(kube: crate::kube::KubeClient) -> anyhow::Result<()> {
    use chrono::{DateTime, Utc};

    let policy = read_policy(&kube.client()).await;
    if !policy.enabled {
        return Ok(());
    }
    let cooldown = chrono::Duration::seconds(policy.cooldown_secs.max(60) as i64);
    let scope = policy.namespaces.clone().unwrap_or_default();

    let vmis = if scope.is_empty() || scope == "all" {
        kube.list_all_vmis()
            .await
            .inspect_err(|e| {
                log::error!("self-healing tick: failed to list VMIs cluster-wide: {e}")
            })
            .unwrap_or_default()
    } else {
        let mut all = Vec::new();
        for ns in scope.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            all.extend(
                kube.list_vmis(ns)
                    .await
                    .inspect_err(|e| {
                        log::error!("self-healing tick: failed to list VMIs in {ns}: {e}")
                    })
                    .unwrap_or_default(),
            );
        }
        all
    };

    for vmi in &vmis {
        let phase = vmi
            .status
            .as_ref()
            .and_then(|s| s.phase.clone())
            .unwrap_or_default();
        if !is_unhealthy(&phase) {
            continue;
        }
        let ns = vmi.metadata.namespace.clone().unwrap_or_default();
        let name = vmi.metadata.name.clone().unwrap_or_default();
        let key = format!("{ns}/{name}");

        match kube.get_vm(&ns, &name).await {
            Ok(vm) => {
                let last_heal = vm
                    .metadata
                    .annotations
                    .as_ref()
                    .and_then(|a| a.get(LAST_HEAL_ANNOTATION))
                    .and_then(|t| DateTime::parse_from_rfc3339(t).ok());
                if let Some(t) = last_heal {
                    if Utc::now().signed_duration_since(t) < cooldown {
                        continue;
                    }
                }
            }
            Err(e) => {
                log::warn!("self-heal: failed to read VM {key} for cooldown check: {e}");
                continue;
            }
        }

        match kube.restart_vm(&ns, &name).await {
            Ok(_) => {
                log::info!("self-heal restarted {key} (phase {phase})");
                if let Err(e) = kube
                    .annotate_vm(&ns, &name, LAST_HEAL_ANNOTATION, &Utc::now().to_rfc3339())
                    .await
                {
                    log::warn!("self-heal: failed to record cooldown annotation on {key}: {e}");
                }
            }
            Err(e) => log::warn!("self-heal restart {key} failed: {e}"),
        }
    }
    Ok(())
}

/// Phases that indicate an unhealthy, restartable VMI.
#[cfg(feature = "web")]
fn is_unhealthy(phase: &str) -> bool {
    matches!(phase, "Failed" | "Unknown")
}

#[cfg(feature = "web")]
async fn run_self_healing(
    State(state): State<SharedState>,
    Query(q): Query<HealQuery>,
) -> Json<serde_json::Value> {
    let (kube, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);

    let vmis = if namespace_scope::is_all_namespaces(&scope) {
        kube.list_all_vmis()
            .await
            .inspect_err(|e| log::error!("self-healing run: failed to list VMIs cluster-wide: {e}"))
            .unwrap_or_default()
    } else {
        kube.list_vmis(&scope)
            .await
            .inspect_err(|e| log::error!("self-healing run: failed to list VMIs in {scope}: {e}"))
            .unwrap_or_default()
    };

    let mut actions = Vec::new();
    for vmi in &vmis {
        let phase = vmi
            .status
            .as_ref()
            .and_then(|s| s.phase.clone())
            .unwrap_or_default();
        if !is_unhealthy(&phase) {
            continue;
        }
        let ns = vmi.metadata.namespace.clone().unwrap_or_default();
        let name = vmi.metadata.name.clone().unwrap_or_default();
        let mut action = HealAction {
            namespace: ns.clone(),
            vm: name.clone(),
            phase,
            action: "restart".to_string(),
            healed: false,
            error: None,
        };
        if q.heal {
            match kube.restart_vm(&ns, &name).await {
                Ok(_) => action.healed = true,
                Err(e) => action.error = Some(e.to_string()),
            }
        }
        actions.push(action);
    }

    let healed = actions.iter().filter(|a| a.healed).count();
    Json(serde_json::json!({
        "dry_run": !q.heal,
        "unhealthy": actions.len(),
        "healed": healed,
        "actions": actions,
        "note": if q.heal { "restarted unhealthy VMs" } else { "dry run — pass ?heal=true to remediate" }
    }))
}
