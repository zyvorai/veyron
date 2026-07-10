// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
        .with_state(state)
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
        kube.list_all_vmis().await.unwrap_or_default()
    } else {
        kube.list_vmis(&scope).await.unwrap_or_default()
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
