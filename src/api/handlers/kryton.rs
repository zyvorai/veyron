// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Kryton machine control-plane integration handlers.
//!
//! Proxies the Kryton machine API (`../kryton`) under `/api/v1/kryton/*` so the console
//! and Veyron API keys can drive lab and edge machines (dockur Windows, libvirt Linux)
//! next to cluster VMs, and bootstrap Kryton golden images into the CDI DataSources that
//! Veyron templates clone from. When `VEYRON_KRYTON_URL` is unset, `/kryton/status`
//! reports `configured: false` and every other route returns `503`.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
#[cfg(feature = "web")]
use std::collections::BTreeMap;

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::api::kryton::{KRYTON_IMAGE_NAMESPACE, KrytonClient, KrytonError};

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/kryton/status", get(get_status))
        .route("/kryton/images", get(list_images))
        .route("/kryton/machines", get(list_machines).post(create_machine))
        .route(
            "/kryton/machines/:id",
            get(get_machine).delete(delete_machine),
        )
        .route("/kryton/machines/:id/start", post(start_machine))
        .route("/kryton/machines/:id/stop", post(stop_machine))
        .route(
            "/kryton/machines/:id/snapshots",
            get(list_snapshots).post(create_snapshot),
        )
        .route("/kryton/golden", get(list_golden))
        .route("/kryton/golden/:id/bootstrap", post(bootstrap_golden))
        .route("/kryton/jobs/:id", get(get_job))
        .with_state(state)
}

#[cfg(feature = "web")]
type Q = Query<BTreeMap<String, String>>;

#[cfg(feature = "web")]
fn unconfigured() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "Kryton not configured — set VEYRON_KRYTON_URL"
        })),
    )
        .into_response()
}

#[cfg(feature = "web")]
fn upstream_error(e: KrytonError) -> Response {
    let status = StatusCode::from_u16(e.status).unwrap_or(StatusCode::BAD_GATEWAY);
    (
        status,
        Json(serde_json::json!({ "error": e.to_string(), "upstream_status": e.status })),
    )
        .into_response()
}

#[cfg(feature = "web")]
fn query_pairs(q: &BTreeMap<String, String>, allowed: &[&str]) -> Vec<(String, String)> {
    q.iter()
        .filter(|(k, _)| allowed.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Kryton machine IDs are UUIDs; golden/job IDs are short slugs. Refuse anything that
/// could rewrite the upstream path.
#[cfg(feature = "web")]
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && id != "."
        && id != ".."
}

#[cfg(feature = "web")]
async fn forward(
    method: reqwest::Method,
    path: String,
    query: Vec<(String, String)>,
    body: Option<serde_json::Value>,
    ok: StatusCode,
) -> Response {
    let Some(client) = KrytonClient::from_env() else {
        return unconfigured();
    };
    match client.request(method, &path, &query, body.as_ref()).await {
        Ok(v) => (ok, Json(v)).into_response(),
        Err(e) => upstream_error(e),
    }
}

#[cfg(feature = "web")]
macro_rules! with_id {
    ($id:expr, $body:expr) => {
        if !safe_id(&$id) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid id" })),
            )
                .into_response();
        } else {
            $body
        }
    };
}

#[cfg(feature = "web")]
async fn get_status() -> Response {
    let Some(client) = KrytonClient::from_env() else {
        return Json(serde_json::json!({
            "configured": false,
            "reachable": false,
            "base_url": null,
            "message": "Set VEYRON_KRYTON_URL (and VEYRON_KRYTON_TOKEN) to manage Kryton machines",
            "image_namespace": KRYTON_IMAGE_NAMESPACE,
        }))
        .into_response();
    };
    let reachable = client.healthy().await;
    let (summary, capabilities, message) = if reachable {
        let s = client
            .request(reqwest::Method::GET, "/summary", &[], None)
            .await;
        let c = client
            .request(reqwest::Method::GET, "/capabilities", &[], None)
            .await;
        let msg = s.as_ref().err().map(|e| e.to_string());
        (s.ok(), c.ok(), msg)
    } else {
        (None, None, Some("Kryton /healthz unreachable".to_string()))
    };
    Json(serde_json::json!({
        "configured": true,
        "reachable": reachable,
        "base_url": client.base(),
        "message": message,
        "summary": summary,
        "capabilities": capabilities,
        "image_namespace": KRYTON_IMAGE_NAMESPACE,
    }))
    .into_response()
}

#[cfg(feature = "web")]
async fn list_images(Query(q): Q) -> Response {
    forward(
        reqwest::Method::GET,
        "/images".into(),
        query_pairs(&q, &["project", "os"]),
        None,
        StatusCode::OK,
    )
    .await
}

#[cfg(feature = "web")]
async fn list_machines(Query(q): Q) -> Response {
    forward(
        reqwest::Method::GET,
        "/machines".into(),
        query_pairs(&q, &["project", "limit", "cursor"]),
        None,
        StatusCode::OK,
    )
    .await
}

#[cfg(feature = "web")]
async fn create_machine(Query(q): Q, Json(body): Json<serde_json::Value>) -> Response {
    forward(
        reqwest::Method::POST,
        "/machines".into(),
        query_pairs(&q, &["project"]),
        Some(body),
        StatusCode::ACCEPTED,
    )
    .await
}

#[cfg(feature = "web")]
async fn get_machine(Path(id): Path<String>, Query(q): Q) -> Response {
    with_id!(
        id,
        forward(
            reqwest::Method::GET,
            format!("/machines/{id}"),
            query_pairs(&q, &["project"]),
            None,
            StatusCode::OK,
        )
        .await
    )
}

#[cfg(feature = "web")]
async fn delete_machine(Path(id): Path<String>, Query(q): Q) -> Response {
    with_id!(
        id,
        forward(
            reqwest::Method::DELETE,
            format!("/machines/{id}"),
            query_pairs(&q, &["project"]),
            None,
            StatusCode::ACCEPTED,
        )
        .await
    )
}

#[cfg(feature = "web")]
async fn start_machine(Path(id): Path<String>, Query(q): Q) -> Response {
    with_id!(
        id,
        forward(
            reqwest::Method::POST,
            format!("/machines/{id}/start"),
            query_pairs(&q, &["project"]),
            None,
            StatusCode::ACCEPTED,
        )
        .await
    )
}

#[cfg(feature = "web")]
async fn stop_machine(Path(id): Path<String>, Query(q): Q) -> Response {
    with_id!(
        id,
        forward(
            reqwest::Method::POST,
            format!("/machines/{id}/stop"),
            query_pairs(&q, &["project"]),
            None,
            StatusCode::ACCEPTED,
        )
        .await
    )
}

#[cfg(feature = "web")]
async fn list_snapshots(Path(id): Path<String>, Query(q): Q) -> Response {
    with_id!(
        id,
        forward(
            reqwest::Method::GET,
            format!("/machines/{id}/snapshots"),
            query_pairs(&q, &["project"]),
            None,
            StatusCode::OK,
        )
        .await
    )
}

#[cfg(feature = "web")]
async fn create_snapshot(
    Path(id): Path<String>,
    Query(q): Q,
    body: Option<Json<serde_json::Value>>,
) -> Response {
    with_id!(
        id,
        forward(
            reqwest::Method::POST,
            format!("/machines/{id}/snapshot"),
            query_pairs(&q, &["project"]),
            Some(
                body.map(|Json(b)| b)
                    .unwrap_or_else(|| serde_json::json!({}))
            ),
            StatusCode::ACCEPTED,
        )
        .await
    )
}

#[cfg(feature = "web")]
async fn list_golden() -> Response {
    forward(
        reqwest::Method::GET,
        "/golden".into(),
        vec![],
        None,
        StatusCode::OK,
    )
    .await
}

/// Publish a finished Kryton golden build as the CDI DataSource
/// `kryton-images/<image-id>` (what Kryton-backed templates such as `rocky-10` clone).
#[cfg(feature = "web")]
async fn bootstrap_golden(Path(id): Path<String>) -> Response {
    with_id!(
        id,
        forward(
            reqwest::Method::POST,
            format!("/golden/{id}/bootstrap"),
            vec![],
            Some(serde_json::json!({})),
            StatusCode::ACCEPTED,
        )
        .await
    )
}

#[cfg(feature = "web")]
async fn get_job(Path(id): Path<String>) -> Response {
    with_id!(
        id,
        forward(
            reqwest::Method::GET,
            format!("/jobs/{id}"),
            vec![],
            None,
            StatusCode::OK,
        )
        .await
    )
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    #[test]
    fn safe_id_rejects_path_tricks() {
        assert!(safe_id("6f1c2a8e-1b2c-4d5e-8f90-123456789abc"));
        assert!(safe_id("rocky-10"));
        assert!(!safe_id(""));
        assert!(!safe_id(".."));
        assert!(!safe_id("a/b"));
        assert!(!safe_id("a?b=c"));
    }
}
