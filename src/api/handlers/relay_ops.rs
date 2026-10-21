// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Per-VM routes backed by kairon-node's relay (FluxVM): incident diagnostics
//! bundles, filesystem quiesce, pressure/cpuset, network observability and
//! packet capture, egress checks and runtime capabilities. Kairon backend only.

// The Err side is a ready-to-send HTTP response, returned straight to the client.
#![allow(clippy::result_large_err)]

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use reqwest::Method;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::http_server::web::SharedState;
use crate::kairon::relay::RelayTarget;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/vms/:ns/:name/diagnostics", post(diag_start))
        .route(
            "/vms/:ns/:name/diagnostics/:token",
            get(diag_get).delete(diag_delete),
        )
        .route("/vms/:ns/:name/diagnostics/:token/:file", get(diag_file))
        .route("/vms/:ns/:name/freeze", post(freeze))
        .route("/vms/:ns/:name/thaw", post(thaw))
        .route("/vms/:ns/:name/frozen", get(frozen))
        .route("/vms/:ns/:name/quiesced-snapshot", post(quiesced_snapshot))
        .route("/vms/:ns/:name/pressure", get(pressure))
        .route("/vms/:ns/:name/cpuset", get(cpuset))
        .route("/vms/:ns/:name/network/effective", get(net_effective))
        .route("/vms/:ns/:name/network/stats", get(net_stats))
        .route("/vms/:ns/:name/network/flows", get(net_flows))
        .route("/vms/:ns/:name/network/drop-reasons", get(net_drop_reasons))
        .route("/vms/:ns/:name/network/drops", get(net_drops))
        .route(
            "/vms/:ns/:name/network/capture",
            post(capture_start).get(capture_list),
        )
        .route("/vms/:ns/:name/network/capture/:token", get(capture_file))
        .route("/vms/:ns/:name/egress-check", post(egress_check))
        .route("/vms/:ns/:name/runtime-capabilities", get(capabilities))
        .with_state(state)
}

type Reply = Result<Response, Response>;

fn err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "error": msg.into() }))).into_response()
}

/// Resolve the relay for a running Machine; 501 when the backend is not Kairon.
async fn target(state: &SharedState, ns: &str, name: &str) -> Result<RelayTarget, Response> {
    if !crate::api::vm_backend::is_kairon() {
        return Err(err(
            StatusCode::NOT_IMPLEMENTED,
            "this operation requires the Kairon backend",
        ));
    }
    let kube = { state.read().await.kube_client.clone() };
    let m = kube::Api::<crate::kairon::Machine>::namespaced(kube.client(), ns)
        .get(name)
        .await
        .map_err(|e| err(StatusCode::NOT_FOUND, format!("VM {ns}/{name}: {e}")))?;
    RelayTarget::for_machine(&kube.client(), &m)
        .await
        .map_err(|e| err(StatusCode::SERVICE_UNAVAILABLE, format!("{e:#}")))
}

async fn json_call(
    t: &RelayTarget,
    method: Method,
    path: &str,
    segments: &[&str],
    query: &str,
    body: Option<&Value>,
) -> Reply {
    t.call_json(method, path, segments, query, body)
        .await
        .map(|v| Json(v).into_response())
        .map_err(|e| err(StatusCode::BAD_GATEWAY, format!("{e:#}")))
}

#[derive(Debug, Deserialize)]
struct LimitQuery {
    limit: Option<u32>,
}

impl LimitQuery {
    fn query(&self) -> String {
        self.limit
            .map(|l| format!("limit={}", l.min(5000)))
            .unwrap_or_default()
    }
}

// --- diagnostics bundles ----------------------------------------------------

async fn diag_start(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    body: Option<Json<Value>>,
) -> Reply {
    let t = target(&state, &ns, &name).await?;
    let body = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    json_call(&t, Method::POST, "diagnostics", &[], "", Some(&body)).await
}

async fn diag_get(
    State(state): State<SharedState>,
    Path((ns, name, token)): Path<(String, String, String)>,
) -> Reply {
    let t = target(&state, &ns, &name).await?;
    json_call(&t, Method::GET, "diagnostics", &[&token], "", None).await
}

async fn diag_delete(
    State(state): State<SharedState>,
    Path((ns, name, token)): Path<(String, String, String)>,
) -> Reply {
    let t = target(&state, &ns, &name).await?;
    json_call(&t, Method::DELETE, "diagnostics", &[&token], "", None).await
}

/// Streams a finished bundle or its memory dump (the latter is Admin-only, see
/// `auth_context::min_role_for_route`).
async fn diag_file(
    State(state): State<SharedState>,
    Path((ns, name, token, file)): Path<(String, String, String, String)>,
) -> Reply {
    if file != "bundle" && file != "memory-dump" {
        return Err(err(
            StatusCode::NOT_FOUND,
            "file must be bundle or memory-dump",
        ));
    }
    let t = target(&state, &ns, &name).await?;
    let resp = t
        .call(Method::GET, "diagnostics", &[&token, &file], "", None, None)
        .await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, format!("{e:#}")))?;
    Ok(stream_response(resp))
}

/// Pass a relay body through, keeping its content type/disposition.
fn stream_response(resp: reqwest::Response) -> Response {
    let mut out = Response::builder().status(StatusCode::OK);
    for h in [header::CONTENT_TYPE, header::CONTENT_DISPOSITION] {
        if let Some(v) = resp.headers().get(&h).and_then(|v| v.to_str().ok())
            && let Ok(v) = HeaderValue::from_str(v)
        {
            out = out.header(h, v);
        }
    }
    out.body(Body::from_stream(resp.bytes_stream()))
        .unwrap_or_else(|_| err(StatusCode::BAD_GATEWAY, "relay stream"))
}

// --- quiesce ----------------------------------------------------------------

async fn freeze(State(s): State<SharedState>, Path((ns, n)): Path<(String, String)>) -> Reply {
    let t = target(&s, &ns, &n).await?;
    json_call(&t, Method::POST, "freeze", &[], "", None).await
}

async fn thaw(State(s): State<SharedState>, Path((ns, n)): Path<(String, String)>) -> Reply {
    let t = target(&s, &ns, &n).await?;
    json_call(&t, Method::POST, "thaw", &[], "", None).await
}

async fn frozen(State(s): State<SharedState>, Path((ns, n)): Path<(String, String)>) -> Reply {
    let t = target(&s, &ns, &n).await?;
    json_call(&t, Method::GET, "frozen", &[], "", None).await
}

#[derive(Debug, Default, Deserialize)]
struct QuiescedSnapshot {
    name: Option<String>,
}

/// freeze -> snapshot -> thaw. The thaw always runs, including after a failed snapshot.
async fn quiesced_snapshot(
    State(state): State<SharedState>,
    Path((ns, vm)): Path<(String, String)>,
    body: Option<Json<QuiescedSnapshot>>,
) -> Reply {
    let t = target(&state, &ns, &vm).await?;
    let name = body.and_then(|Json(b)| b.name).unwrap_or_else(|| {
        format!(
            "quiesced-{vm}-{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        )
    });
    json_call(&t, Method::POST, "freeze", &[], "", None).await?;
    let kube = { state.read().await.kube_client.clone() };
    let snap = crate::api::kairon_ops::create_snapshot(&kube, &ns, &vm, &name).await;
    let thawed = json_call(&t, Method::POST, "thaw", &[], "", None).await;
    if thawed.is_err() {
        log::error!("quiesced snapshot {ns}/{vm}: thaw failed; guest filesystems may stay frozen");
    }
    match (snap, thawed) {
        (Ok(rec), Ok(_)) => {
            Ok(Json(json!({"snapshot": rec, "consistency": "quiesced"})).into_response())
        }
        (Ok(rec), Err(_)) => Ok((
            StatusCode::MULTI_STATUS,
            Json(json!({"snapshot": rec, "warning": "snapshot taken but thaw failed; POST /thaw"})),
        )
            .into_response()),
        (Err(e), _) => Err(err(StatusCode::BAD_GATEWAY, format!("snapshot: {e:#}"))),
    }
}

async fn pressure(State(s): State<SharedState>, Path((ns, n)): Path<(String, String)>) -> Reply {
    let t = target(&s, &ns, &n).await?;
    json_call(&t, Method::GET, "pressure", &[], "", None).await
}

async fn cpuset(State(s): State<SharedState>, Path((ns, n)): Path<(String, String)>) -> Reply {
    let t = target(&s, &ns, &n).await?;
    json_call(&t, Method::GET, "cpuset", &[], "", None).await
}

// --- network observability ----------------------------------------------------

macro_rules! net_get {
    ($fn:ident, $path:literal, limited) => {
        async fn $fn(
            State(s): State<SharedState>,
            Path((ns, n)): Path<(String, String)>,
            Query(q): Query<LimitQuery>,
        ) -> Reply {
            let t = target(&s, &ns, &n).await?;
            json_call(&t, Method::GET, $path, &[], &q.query(), None).await
        }
    };
    ($fn:ident, $path:literal) => {
        async fn $fn(State(s): State<SharedState>, Path((ns, n)): Path<(String, String)>) -> Reply {
            let t = target(&s, &ns, &n).await?;
            json_call(&t, Method::GET, $path, &[], "", None).await
        }
    };
}

net_get!(net_effective, "network-effective");
net_get!(net_stats, "network-stats");
net_get!(net_flows, "network-flows", limited);
net_get!(net_drop_reasons, "network-drop-reasons", limited);
net_get!(net_drops, "network-drops", limited);
net_get!(capture_list, "network-capture");

async fn capture_start(
    State(s): State<SharedState>,
    Path((ns, n)): Path<(String, String)>,
    Json(session): Json<Value>,
) -> Reply {
    let t = target(&s, &ns, &n).await?;
    json_call(&t, Method::POST, "network-capture", &[], "", Some(&session)).await
}

async fn capture_file(
    State(s): State<SharedState>,
    Path((ns, n, token)): Path<(String, String, String)>,
) -> Reply {
    let t = target(&s, &ns, &n).await?;
    let resp = t
        .call(Method::GET, "network-capture", &[&token], "", None, None)
        .await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, format!("{e:#}")))?;
    Ok(stream_response(resp))
}

// --- node-level diagnostics (resolved through the VM's node) -------------------

#[derive(Debug, Deserialize)]
struct EgressCheck {
    host: String,
}

/// Drops credential material FluxVM may echo (e.g. `inject_authorization` carries
/// a real vault secret); kairon-node relays it verbatim, so Veyron is the boundary.
/// Replaces such fields with a `<key>_present` boolean.
pub(crate) fn redact_secrets(v: &mut Value) {
    match v {
        Value::Object(map) => {
            let sensitive: Vec<String> = map
                .keys()
                .filter(|k| {
                    let k = k.to_ascii_lowercase();
                    ["authoriz", "secret", "token", "password", "credential"]
                        .iter()
                        .any(|s| k.contains(s))
                })
                .cloned()
                .collect();
            for k in sensitive {
                let present = match map.remove(&k) {
                    Some(Value::String(s)) => !s.is_empty(),
                    Some(Value::Null) | None => false,
                    Some(_) => true,
                };
                map.insert(format!("{k}_present"), Value::Bool(present));
            }
            map.values_mut().for_each(redact_secrets);
        }
        Value::Array(a) => a.iter_mut().for_each(redact_secrets),
        _ => {}
    }
}

async fn egress_check(
    State(s): State<SharedState>,
    Path((ns, n)): Path<(String, String)>,
    Json(req): Json<EgressCheck>,
) -> Reply {
    if req.host.trim().is_empty() || req.host.len() > 253 {
        return Err(err(StatusCode::BAD_REQUEST, "host is required"));
    }
    let t = target(&s, &ns, &n).await?;
    let mut v = t
        .call_json(
            Method::POST,
            "egress-check",
            &[],
            "",
            Some(&json!({ "host": req.host })),
        )
        .await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, format!("{e:#}")))?;
    redact_secrets(&mut v);
    Ok(Json(v).into_response())
}

async fn capabilities(
    State(s): State<SharedState>,
    Path((ns, n)): Path<(String, String)>,
) -> Reply {
    let t = target(&s, &ns, &n).await?;
    json_call(&t, Method::GET, "capabilities", &[], "", None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_nested_credentials() {
        let mut v = json!({
            "allowed": true,
            "inject_authorization": "Bearer abc",
            "rule": {"api_token": "", "host": "x.dev"},
            "list": [{"password": "p"}]
        });
        redact_secrets(&mut v);
        assert_eq!(v["inject_authorization_present"], true);
        assert!(v.get("inject_authorization").is_none());
        assert_eq!(v["rule"]["api_token_present"], false);
        assert_eq!(v["rule"]["host"], "x.dev");
        assert_eq!(v["list"][0]["password_present"], true);
        assert!(!v.to_string().contains("abc"));
    }

    #[test]
    fn limit_is_capped() {
        assert_eq!(LimitQuery { limit: Some(99999) }.query(), "limit=5000");
        assert_eq!(LimitQuery { limit: None }.query(), "");
    }
}
