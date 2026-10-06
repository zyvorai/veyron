// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Veyron AI: one tool registry and approval engine behind the console assistant,
//! the MCP server, sandboxes and the background investigators. See `docs/ai.md`.

pub mod agent;
pub mod api;
pub mod cel;
pub mod exec;
pub mod incidents;
pub mod intent;
pub mod llm;
pub mod models;
pub mod policy;
pub mod predictive;
pub mod proposals;
pub mod registry;
pub mod search;

use crate::api::auth_context::AuthContext;
use crate::api::http_server::web::SharedState;

/// Namespace for AI state (proposals, investigations, samples, settings).
pub fn ai_namespace(api_default_ns: &str) -> String {
    std::env::var("VEYRON_AI_NAMESPACE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if api_default_ns.is_empty() || api_default_ns == "all" {
                "default".to_string()
            } else {
                api_default_ns.to_string()
            }
        })
}

pub async fn tool_ctx(state: &SharedState, auth: &AuthContext, source: &str) -> registry::ToolCtx {
    let s = state.read().await;
    registry::ToolCtx {
        role: auth.role.clone(),
        subject: auth.subject.clone(),
        kube: s.kube_client.clone(),
        scope: "all".to_string(),
        api_namespace: ai_namespace(&s.namespace),
        source: source.to_string(),
    }
}

/// Context for background workers (incident investigator, predictive ops).
pub async fn system_ctx(state: &SharedState, source: &str) -> registry::ToolCtx {
    tool_ctx(
        state,
        &AuthContext {
            role: crate::api::http_server::web::ApiRole::ReadOnly,
            subject: format!("veyron-ai/{source}"),
        },
        source,
    )
    .await
}

/// Background AI work, called once a minute by the Lease-holding replica. Each job
/// runs in its own task and is skipped while its previous run is still going.
pub fn leader_tick(state: SharedState, n: u64) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static SANDBOX: AtomicBool = AtomicBool::new(false);
    static INCIDENTS: AtomicBool = AtomicBool::new(false);
    static PREDICT: AtomicBool = AtomicBool::new(false);

    fn spawn_guarded<F>(flag: &'static AtomicBool, fut: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        if flag.swap(true, Ordering::SeqCst) {
            return;
        }
        tokio::spawn(async move {
            fut.await;
            flag.store(false, Ordering::SeqCst);
        });
    }

    let s = state.clone();
    spawn_guarded(&SANDBOX, async move { crate::sandbox::tick(&s).await });
    let s = state.clone();
    spawn_guarded(&INCIDENTS, async move { incidents::tick(&s).await });
    if n % predictive::TICK_EVERY == 1 || predictive::TICK_EVERY == 1 {
        spawn_guarded(&PREDICT, async move { predictive::tick(&state).await });
    }
}

#[cfg(test)]
pub mod test_router {
    //! A fake API behind `exec::call` so tools and proposals can be tested offline.
    use axum::{
        Json, Router,
        extract::Path,
        routing::{get, post},
    };
    use once_cell::sync::Lazy;
    use serde_json::json;
    use std::sync::Mutex;

    static WEB_STATUS: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("Running".into()));

    pub fn install() {
        let app = Router::new()
            .route(
                "/api/v1/vms",
                get(|| async {
                    Json(json!({"success": true, "data": [
                        {"name": "web", "namespace": "default", "status": "Running", "node": "node-a"}
                    ]}))
                }),
            )
            .route(
                "/api/v1/vms/:ns/:name",
                get(|Path((_ns, name)): Path<(String, String)>| async move {
                    if name != "web" {
                        return (
                            axum::http::StatusCode::NOT_FOUND,
                            Json(json!({"success": false, "error": {"message": "VM not found"}})),
                        );
                    }
                    let status = WEB_STATUS.lock().unwrap().clone();
                    (
                        axum::http::StatusCode::OK,
                        Json(json!({"success": true, "data": {
                            "name": "web", "namespace": "default", "status": status,
                            "cpu": "2 cores", "memory": "4Gi", "node": "node-a"
                        }})),
                    )
                }),
            )
            .route(
                "/api/v1/vms/:ns/:name/:verb",
                post(|Path((_ns, _name, verb)): Path<(String, String, String)>| async move {
                    let next = match verb.as_str() {
                        "stop" => "Stopped",
                        "start" | "unpause" | "restart" => "Running",
                        "pause" => "Paused",
                        _ => {
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({"success": false, "error": {"message": "boom"}})),
                            );
                        }
                    };
                    *WEB_STATUS.lock().unwrap() = next.into();
                    (axum::http::StatusCode::OK, Json(json!({"success": true, "data": {}})))
                }),
            );
        super::exec::install_router(app);
    }

    /// Tool context for tests; `None` when no kubeconfig is available locally.
    pub async fn ctx() -> Option<super::registry::ToolCtx> {
        let kube = crate::kube::KubeClient::new().await.ok()?;
        Some(super::registry::ToolCtx {
            role: crate::api::http_server::web::ApiRole::Write,
            subject: "alice".into(),
            kube,
            scope: "all".into(),
            api_namespace: "default".into(),
            source: "assistant".into(),
        })
    }
}
