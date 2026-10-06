// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! HTTP routes for Veyron AI.
//!
//! Chat and MCP live on the long-lived router (no 30 s request timeout) because a
//! multi-step agent run or a sandbox command can take minutes.

use std::convert::Infallible;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::agent::{self, AgentEvent, AgentOutcome, AgentRequest};
use super::llm::OpenAiBackend;
use super::proposals::{self, ProposalStatus, role_str};
use super::registry;
use crate::api::auth_context::{AuthContext, role_rank};
use crate::api::http_server::web::SharedState;

/// Short routes, mounted under `/api/v1` with the normal request timeout.
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/ai/status", get(status))
        .route("/ai/tools", get(list_tools))
        .route("/ai/proposals", get(list_proposals))
        .route("/ai/proposals/:id", get(get_proposal))
        .route("/ai/proposals/:id/approve", post(approve))
        .route("/ai/proposals/:id/reject", post(reject))
        .route("/ai/investigations", get(super::incidents::list_handler))
        .route("/ai/forecast", get(super::predictive::forecast_handler))
        .route(
            "/ai/models",
            get(super::models::list_handler).post(super::models::create_handler),
        )
        .route(
            "/ai/models/:ns/:name",
            axum::routing::delete(super::models::delete_handler),
        )
        .route(
            "/ai/settings/llm",
            get(super::models::get_llm_setting).put(super::models::put_llm_setting),
        )
        .route(
            "/ai/mcp-servers",
            get(crate::mcp::client::list_servers_handler)
                .put(crate::mcp::client::put_servers_handler),
        )
        .with_state(state)
}

/// Long-running routes, mounted at full paths on the router without a timeout.
pub fn long_lived_router(state: SharedState) -> Router {
    Router::new()
        .route("/api/v1/ai/chat/stream", post(chat_stream))
        .route("/api/v1/ai/chat", post(chat_once))
        .route("/api/v1/ai/search", post(super::search::search_handler))
        .route("/api/v1/ai/intent/vm", post(super::intent::intent_handler))
        .route(
            "/api/v1/ai/policies/draft",
            post(super::policy::draft_handler),
        )
        .route(
            "/api/v1/ai/policies/preview",
            post(super::policy::preview_handler),
        )
        .route(
            "/api/v1/ai/investigations/run",
            post(super::incidents::run_handler),
        )
        .route(
            "/api/v1/ai/mcp-servers/:server/call",
            post(crate::mcp::client::call_handler),
        )
        .with_state(state)
}

pub fn llm_status() -> Value {
    let cfg = crate::copilot::llm_config();
    json!({
        "llm_configured": cfg.is_some(),
        "model": cfg.as_ref().map(|c| c.model.clone()),
        "endpoint_host": cfg.as_ref().and_then(|c| url_host(&c.base_url)),
        "in_cluster_model": crate::copilot::runtime_override().is_some(),
        "mode": if cfg.is_some() { "agent" } else { "advisor" },
    })
}

fn url_host(u: &str) -> Option<String> {
    let rest = u.split("://").nth(1)?;
    Some(rest.split(['/', '?']).next()?.to_string())
}

async fn status(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
) -> Json<Value> {
    let ctx = super::tool_ctx(&state, &auth, "assistant").await;
    let tools = registry::all_tools(&ctx).await;
    let mut s = llm_status();
    s["tools"] = json!(tools.len());
    s["change_tools"] = json!(
        tools
            .iter()
            .filter(|t| t.kind == registry::ToolKind::Change)
            .count()
    );
    s["mcp_endpoint"] = json!("/mcp");
    s["you"] = json!({"subject": auth.subject, "role": role_str(&auth.role)});
    s["sandboxes"] = crate::sandbox::backend_summary();
    Json(json!({"success": true, "data": s}))
}

async fn list_tools(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
) -> Json<Value> {
    let ctx = super::tool_ctx(&state, &auth, "assistant").await;
    Json(json!({"success": true, "data": registry::all_tools(&ctx).await}))
}

/// Answer without a model: the deterministic advisors, phrased as a chat reply.
async fn advisor_reply(ctx: &registry::ToolCtx, req: &AgentRequest) -> String {
    let query = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let creq = crate::copilot::CopilotChatRequest {
        messages: vec![],
        query: Some(query),
        namespace: req.context.namespace.clone(),
        vm_name: req.context.vm_name.clone(),
    };
    let r = crate::copilot::copilot_chat(&ctx.kube, &ctx.scope, &creq).await;
    let mut text = r.copilot.title.clone();
    if !r.copilot.summary.is_empty() && r.copilot.summary != r.copilot.title {
        text.push_str("\n\n");
        text.push_str(&r.copilot.summary);
    }
    if !r.copilot.recommendations.is_empty() {
        text.push_str("\n\n");
        for rec in r.copilot.recommendations.iter().take(6) {
            text.push_str(&format!("- {rec}\n"));
        }
    }
    text.push_str(
        "\n\n_No language model is configured, so this answer comes from Veyron's built-in advisors. \
         Set VEYRON_AI_URL / VEYRON_AI_API_KEY, or serve a model from AI → Models, for full agent answers._",
    );
    text
}

async fn run_agent(
    ctx: registry::ToolCtx,
    req: AgentRequest,
    emit: &(dyn Fn(AgentEvent) + Send + Sync),
) -> AgentOutcome {
    match OpenAiBackend::from_env() {
        Some(backend) => {
            emit(AgentEvent::Start {
                mode: "agent".into(),
                model: Some(backend.cfg.model.clone()),
            });
            agent::run(&backend, &ctx, &req, emit).await
        }
        None => {
            emit(AgentEvent::Start {
                mode: "advisor".into(),
                model: None,
            });
            let content = advisor_reply(&ctx, &req).await;
            emit(AgentEvent::Message {
                content: content.clone(),
            });
            AgentOutcome {
                content,
                ..Default::default()
            }
        }
    }
}

async fn chat_stream(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<AgentRequest>,
) -> Response {
    let ctx = super::tool_ctx(&state, &auth, "assistant").await;
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AgentEvent>();
    tokio::spawn(async move {
        let tx2 = tx.clone();
        let emit = move |e: AgentEvent| {
            let _ = tx2.send(e);
        };
        run_agent(ctx, req, &emit).await;
        let _ = tx.send(AgentEvent::Done);
    });
    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        let ev = rx.recv().await?;
        let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
        Some((Ok::<Event, Infallible>(Event::default().data(data)), rx))
    });
    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

async fn chat_once(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<AgentRequest>,
) -> Json<Value> {
    let ctx = super::tool_ctx(&state, &auth, "assistant").await;
    let events = std::sync::Mutex::new(Vec::new());
    let emit = |e: AgentEvent| {
        if !matches!(e, AgentEvent::Token { .. }) {
            if let Ok(mut v) = events.lock() {
                v.push(e);
            }
        }
    };
    let out = run_agent(ctx, req, &emit).await;
    let events = events.into_inner().unwrap_or_default();
    Json(json!({"success": true, "data": {"outcome": out, "events": events}}))
}

#[derive(Deserialize, Default)]
pub struct ProposalQuery {
    #[serde(default)]
    pub status: Option<String>,
}

async fn list_proposals(
    State(state): State<SharedState>,
    Query(q): Query<ProposalQuery>,
) -> Response {
    let (client, ns) = {
        let s = state.read().await;
        (s.kube_client.client(), super::ai_namespace(&s.namespace))
    };
    match proposals::list(&client, &ns).await {
        Ok(mut all) => {
            if let Some(st) = q.status.filter(|s| !s.is_empty()) {
                all.retain(|p| {
                    serde_json::to_value(&p.status)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s == st))
                        .unwrap_or(false)
                });
            }
            Json(json!({"success": true, "data": all})).into_response()
        }
        Err(e) => err(500, &e.to_string()),
    }
}

async fn get_proposal(State(state): State<SharedState>, Path(id): Path<String>) -> Response {
    let (client, ns) = {
        let s = state.read().await;
        (s.kube_client.client(), super::ai_namespace(&s.namespace))
    };
    match proposals::load(&client, &ns, &id).await {
        Ok(Some(p)) => Json(json!({"success": true, "data": p})).into_response(),
        Ok(None) => err(404, "Proposal not found"),
        Err(e) => err(500, &e.to_string()),
    }
}

pub(crate) fn err(status: u16, msg: &str) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(json!({"success": false, "status": status, "error": {"message": msg}})),
    )
        .into_response()
}

#[derive(Deserialize, Default)]
pub struct ApproveBody {
    /// Snapshot affected VMs first so a failed change can be restored.
    #[serde(default)]
    pub pre_snapshot: bool,
}

async fn approve(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    body: Option<Json<ApproveBody>>,
) -> Response {
    let body = body.map(|b| b.0).unwrap_or_default();
    let (client, ns) = {
        let s = state.read().await;
        (s.kube_client.client(), super::ai_namespace(&s.namespace))
    };
    match approve_and_run(client, ns, &auth, &id, body.pre_snapshot).await {
        Ok(running) => (
            StatusCode::ACCEPTED,
            Json(json!({"success": true, "data": running})),
        )
            .into_response(),
        Err((code, msg)) => err(code, &msg),
    }
}

/// Check the approver, mark the proposal running and execute it in the background.
pub async fn approve_and_run(
    client: kube::Client,
    ns: String,
    auth: &AuthContext,
    id: &str,
    pre_snapshot: bool,
) -> Result<proposals::Proposal, (u16, String)> {
    let p = match proposals::load(&client, &ns, id).await {
        Ok(Some(p)) => p,
        Ok(None) => return Err((404, "Proposal not found".into())),
        Err(e) => return Err((500, e.to_string())),
    };
    if p.status != ProposalStatus::Pending {
        return Err((409, "This proposal was already decided".into()));
    }
    if p.is_stale(chrono::Utc::now()) {
        return Err((
            409,
            "This proposal expired; ask Veyron AI to draft it again against the current cluster"
                .into(),
        ));
    }
    if auth.subject.contains("(via Veyron AI)") {
        return Err((403, "AI tools cannot approve proposals".into()));
    }
    let need = p.required_role();
    if role_rank(&auth.role) < role_rank(&need) {
        return Err((
            403,
            format!("Approving this proposal needs the {} role", role_str(&need)),
        ));
    }
    let mut running = p.clone();
    running.status = ProposalStatus::Running;
    running.decided_by = Some(auth.subject.clone());
    running.log.push(format!("Approved by {}", auth.subject));
    proposals::save(&client, &ns, &running)
        .await
        .map_err(|e| (500, e.to_string()))?;
    let role = auth.role.clone();
    let subject = auth.subject.clone();
    tokio::spawn(async move {
        let done = proposals::execute(p, &role, &subject, pre_snapshot).await;
        if let Err(e) = proposals::save(&client, &ns, &done).await {
            log::warn!("saving proposal {} result: {e}", done.id);
        }
    });
    Ok(running)
}

/// Mark a pending proposal rejected.
pub async fn reject_proposal(
    client: &kube::Client,
    ns: &str,
    auth: &AuthContext,
    id: &str,
) -> Result<proposals::Proposal, (u16, String)> {
    let mut p = match proposals::load(client, ns, id).await {
        Ok(Some(p)) => p,
        Ok(None) => return Err((404, "Proposal not found".into())),
        Err(e) => return Err((500, e.to_string())),
    };
    if p.status != ProposalStatus::Pending {
        return Err((409, "This proposal was already decided".into()));
    }
    p.status = ProposalStatus::Rejected;
    p.decided_by = Some(auth.subject.clone());
    p.updated_at = chrono::Utc::now().to_rfc3339();
    p.log.push(format!("Rejected by {}", auth.subject));
    proposals::save(client, ns, &p)
        .await
        .map_err(|e| (500, e.to_string()))?;
    Ok(p)
}

async fn reject(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Response {
    let (client, ns) = {
        let s = state.read().await;
        (s.kube_client.client(), super::ai_namespace(&s.namespace))
    };
    match reject_proposal(&client, &ns, &auth, &id).await {
        Ok(p) => Json(json!({"success": true, "data": p})).into_response(),
        Err((code, msg)) => err(code, &msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_host_extracts_host() {
        assert_eq!(
            url_host("https://openrouter.ai/api/v1").as_deref(),
            Some("openrouter.ai")
        );
        assert_eq!(
            url_host("http://vllm.models.svc:8000/v1").as_deref(),
            Some("vllm.models.svc:8000")
        );
    }
}
