// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Kairon `AgentSession` CRUD (experimental).
//!
//! Sessions need FluxVM agent sessions, which today run on Mac nodes only; on a
//! cluster without the CRD or without such a node the create is accepted by the API
//! server but never leaves `Pending`. Veyron therefore probes the CRD and answers
//! `501 AGENT_SESSION_UNAVAILABLE` when it is absent. Session verbs (rewind, branch,
//! bisect) go through kairon-ui and are not proxied here.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use kube::api::{Api, ApiResource, DeleteParams, DynamicObject, ListParams, PostParams};
use kube::core::GroupVersionKind;
use serde::Deserialize;
use serde_json::{Value, json};

use super::machine_images::{Reply, fail, ok, require_kairon};
use crate::api::http_server::web::SharedState;

fn resource() -> ApiResource {
    ApiResource::from_gvk_with_plural(
        &GroupVersionKind::gvk("kairon.zyvor.dev", "v1beta1", "AgentSession"),
        "agentsessions",
    )
}

fn view(o: &DynamicObject) -> Value {
    let st = o.data.get("status");
    json!({
        "name": o.metadata.name,
        "namespace": o.metadata.namespace,
        "agent": o.data.get("spec").and_then(|s| s.get("agent")),
        "phase": st.and_then(|s| s.get("phase")),
        "state": st.and_then(|s| s.get("state")),
        "node": st.and_then(|s| s.get("nodeName")),
        "message": st.and_then(|s| s.get("message")),
        "expires_at": st.and_then(|s| s.get("expiresAt")),
        "parent": st.and_then(|s| s.get("parent")),
        "children": st.and_then(|s| s.get("children")),
        "usage": st.and_then(|s| s.get("usage")),
    })
}

async fn api_for(
    state: &SharedState,
    ns: Option<&str>,
) -> Result<Api<DynamicObject>, (StatusCode, Json<Value>)> {
    require_kairon()?;
    let client = { state.read().await.client().client() };
    let api = match ns {
        Some(ns) => Api::namespaced_with(client, ns, &resource()),
        None => Api::all_with(client, &resource()),
    };
    // Probe: a 404 on list means the CRD is not installed.
    match api.list(&ListParams::default().limit(1)).await {
        Ok(_) => Ok(api),
        Err(kube::Error::Api(ae)) if ae.code == 404 => Err(fail(
            StatusCode::NOT_IMPLEMENTED,
            "AGENT_SESSION_UNAVAILABLE",
            "the AgentSession CRD is not installed (needs Kairon v0.10 and a FluxVM agent-session node)",
        )),
        Err(e) => Err(fail(
            StatusCode::BAD_GATEWAY,
            "UPSTREAM_QUERY_FAILED",
            e.to_string(),
        )),
    }
}

async fn list_all(State(state): State<SharedState>) -> Reply {
    let api = api_for(&state, None).await?;
    let items = api.list(&ListParams::default()).await.map_err(|e| {
        fail(
            StatusCode::BAD_GATEWAY,
            "UPSTREAM_QUERY_FAILED",
            e.to_string(),
        )
    })?;
    ok(
        StatusCode::OK,
        json!({"items": items.items.iter().map(view).collect::<Vec<_>>()}),
    )
}

#[derive(Debug, Deserialize)]
struct CreateReq {
    name: String,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    ttl_seconds: Option<u64>,
    #[serde(default)]
    cpu: Option<String>,
    #[serde(default)]
    memory: Option<String>,
}

fn spec_from(req: &CreateReq) -> Value {
    let mut spec = serde_json::Map::new();
    if let Some(a) = &req.agent {
        spec.insert("agent".into(), json!(a));
    }
    if let Some(i) = &req.image {
        spec.insert("image".into(), json!(i));
    }
    if let Some(t) = req.ttl_seconds {
        spec.insert("ttlSeconds".into(), json!(t));
    }
    let mut res = serde_json::Map::new();
    if let Some(c) = &req.cpu {
        res.insert("cpu".into(), json!(c));
    }
    if let Some(m) = &req.memory {
        res.insert("memory".into(), json!(m));
    }
    if !res.is_empty() {
        spec.insert("resources".into(), Value::Object(res));
    }
    Value::Object(spec)
}

async fn create(
    State(state): State<SharedState>,
    Path(ns): Path<String>,
    Json(req): Json<CreateReq>,
) -> Reply {
    if req.name.is_empty() || req.name.len() > 63 {
        return Err(fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            "INVALID_NAME",
            "name is required (max 63 characters)",
        ));
    }
    let api = api_for(&state, Some(&ns)).await?;
    let obj: DynamicObject = serde_json::from_value(json!({
        "apiVersion": "kairon.zyvor.dev/v1beta1",
        "kind": "AgentSession",
        "metadata": {"name": req.name, "namespace": ns,
                     "labels": {"app.kubernetes.io/managed-by": "veyron"}},
        "spec": spec_from(&req),
    }))
    .map_err(|e| {
        fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            "INVALID_SESSION",
            e.to_string(),
        )
    })?;
    let created = api
        .create(&PostParams::default(), &obj)
        .await
        .map_err(|e| match &e {
            kube::Error::Api(ae) if ae.code == 409 => {
                fail(StatusCode::CONFLICT, "ALREADY_EXISTS", ae.message.clone())
            }
            kube::Error::Api(ae) if matches!(ae.code, 400 | 403 | 422) => fail(
                StatusCode::UNPROCESSABLE_ENTITY,
                "SESSION_REJECTED",
                ae.message.clone(),
            ),
            _ => fail(StatusCode::BAD_GATEWAY, "CREATE_FAILED", e.to_string()),
        })?;
    ok(StatusCode::CREATED, view(&created))
}

async fn get_one(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Reply {
    let api = api_for(&state, Some(&ns)).await?;
    match api.get_opt(&name).await {
        Ok(Some(o)) => ok(StatusCode::OK, view(&o)),
        Ok(None) => Err(fail(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("AgentSession {ns}/{name} not found"),
        )),
        Err(e) => Err(fail(
            StatusCode::BAD_GATEWAY,
            "UPSTREAM_QUERY_FAILED",
            e.to_string(),
        )),
    }
}

async fn delete_one(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Reply {
    let api = api_for(&state, Some(&ns)).await?;
    match api.delete(&name, &DeleteParams::background()).await {
        Ok(_) => ok(StatusCode::OK, json!({"name": name, "deleted": true})),
        Err(kube::Error::Api(ae)) if ae.code == 404 => Err(fail(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("AgentSession {ns}/{name} not found"),
        )),
        Err(e) => Err(fail(
            StatusCode::BAD_GATEWAY,
            "DELETE_FAILED",
            e.to_string(),
        )),
    }
}

/// FluxVM agent-session verbs reachable through `/agent-sessions/:ns/:name/ops/*rest`, by first
/// path segment. JSON-only: binary or streaming verbs (screenshot, pty, input, uploads, files,
/// event stream) and the ones that hand out credentials (tokens, ssh) are left out on purpose,
/// mirroring kairon-node's own relay allow-list.
const OPS_VERBS: &[&str] = &[
    "approvals",
    "grants",
    "probe",
    "exec",
    "checkpoint",
    "restore",
    "rewind",
    "fork",
    "promote",
    "events",
    "artifacts",
    "trajectory",
    "preview",
    "previews",
    "jobs",
    "git",
];

/// Validate the caller-supplied tail before it is spliced into a relay path.
fn ops_path(rest: &str) -> Result<String, String> {
    let rest = rest.trim_matches('/');
    let verb = rest.split('/').next().unwrap_or("");
    if !OPS_VERBS.contains(&verb) {
        return Err(format!("'{verb}' is not an available agent-session verb"));
    }
    if rest.len() > 200
        || rest
            .split('/')
            .any(|seg| seg.is_empty() || seg == "." || seg == "..")
        || !rest
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/'))
    {
        return Err("invalid agent-session path".into());
    }
    Ok(rest.to_string())
}

async fn session_ops(
    State(state): State<SharedState>,
    method: axum::http::Method,
    Path((ns, name, rest)): Path<(String, String, String)>,
    axum::extract::RawQuery(query): axum::extract::RawQuery,
    body: Option<Json<Value>>,
) -> Reply {
    let tail = ops_path(&rest)
        .map_err(|m| fail(StatusCode::UNPROCESSABLE_ENTITY, "INVALID_SESSION_PATH", m))?;
    let api = api_for(&state, Some(&ns)).await?;
    let o = api.get_opt(&name).await.map_err(|e| {
        fail(
            StatusCode::BAD_GATEWAY,
            "UPSTREAM_QUERY_FAILED",
            e.to_string(),
        )
    })?;
    let Some(o) = o else {
        return Err(fail(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("AgentSession {ns}/{name} not found"),
        ));
    };
    let st = o.data.get("status");
    let field = |k: &str| {
        st.and_then(|s| s.get(k))
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
    };
    let (Some(node), Some(sid)) = (field("nodeName"), field("sessionID")) else {
        return Err(fail(
            StatusCode::CONFLICT,
            "SESSION_NOT_READY",
            "the session has no node or FluxVM session id yet",
        ));
    };
    if !sid.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err(fail(
            StatusCode::BAD_GATEWAY,
            "BAD_SESSION_ID",
            "unexpected session id format",
        ));
    }
    let client = { state.read().await.client().client() };
    let target = crate::kairon::relay::RelayTarget::for_node(&client, node)
        .await
        .map_err(|e| {
            fail(
                StatusCode::SERVICE_UNAVAILABLE,
                "RELAY_UNAVAILABLE",
                format!("{e:#}"),
            )
        })?;
    let body = body.map(|Json(v)| v);
    let out = target
        .call_node(
            method,
            &format!("agent-sessions/{sid}/{tail}"),
            query.as_deref().unwrap_or(""),
            body.as_ref(),
        )
        .await
        .map_err(|e| fail(StatusCode::BAD_GATEWAY, "RELAY_FAILED", format!("{e:#}")))?;
    ok(StatusCode::OK, out)
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route(
            "/agent-sessions/:ns/:name/ops/*rest",
            get(session_ops).post(session_ops),
        )
        .route("/agent-sessions", get(list_all))
        .route("/agent-sessions/:ns", axum::routing::post(create))
        .route("/agent-sessions/:ns/:name", get(get_one).delete(delete_one))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_carries_only_given_fields() {
        let req = CreateReq {
            name: "s".into(),
            agent: Some("claude".into()),
            image: None,
            ttl_seconds: Some(600),
            cpu: Some("2".into()),
            memory: None,
        };
        assert_eq!(
            spec_from(&req),
            json!({"agent": "claude", "ttlSeconds": 600, "resources": {"cpu": "2"}})
        );
    }

    #[test]
    fn ops_path_allows_listed_verbs_only() {
        assert_eq!(ops_path("exec").unwrap(), "exec");
        assert_eq!(
            ops_path("/approvals/abc-1/approve/").unwrap(),
            "approvals/abc-1/approve"
        );
        assert!(ops_path("tokens").is_err());
        assert!(ops_path("ssh").is_err());
        assert!(ops_path("pty").is_err());
        assert!(ops_path("exec/../tokens").is_err());
        assert!(ops_path("exec//x").is_err());
        assert!(ops_path("exec/a b").is_err());
        assert!(ops_path("").is_err());
    }

    #[test]
    fn view_reads_status_fields() {
        let o: DynamicObject = serde_json::from_value(json!({
            "apiVersion": "kairon.zyvor.dev/v1beta1", "kind": "AgentSession",
            "metadata": {"name": "a", "namespace": "n"},
            "spec": {"agent": "x"},
            "status": {"phase": "Running", "nodeName": "mac-1"}
        }))
        .unwrap();
        let v = view(&o);
        assert_eq!(v["phase"], "Running");
        assert_eq!(v["node"], "mac-1");
        assert_eq!(v["agent"], "x");
    }
}
