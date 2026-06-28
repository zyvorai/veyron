// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! PacketWolf Network Brain integration — status probe and optional API proxy base.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketWolfStatusResponse {
    pub configured: bool,
    pub reachable: bool,
    #[serde(default)]
    pub api_authorized: bool,
    pub base_url: Option<String>,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_url: Option<String>,
}

/// Returns an error string if the URL is unsafe to proxy to (non-http/https scheme,
/// loopback, link-local, or cloud metadata endpoints).
fn validate_proxy_url(url: &str) -> Result<(), String> {
    let lower = url.trim().to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(format!("unsafe URL scheme in proxy target: {url}"));
    }
    let host_part = lower
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .split('@')
        .next_back()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    let blocked = [
        "localhost",
        "127.",
        "169.254.",
        "::1",
        "0.0.0.0",
        "metadata.",
    ];
    for prefix in &blocked {
        if host_part == *prefix || host_part.starts_with(prefix) {
            return Err(format!("proxy target host is blocked: {host_part}"));
        }
    }
    Ok(())
}

pub fn packetwolf_base_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_PACKETWOLF_URL")
}

pub fn packetwolf_external_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_PACKETWOLF_EXTERNAL_URL")
}

pub fn packetwolf_api_key() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_PACKETWOLF_API_KEY")
}

#[cfg(feature = "web")]
fn health_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/health") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/health")
    }
}

#[cfg(feature = "web")]
pub async fn probe_packetwolf_health(base: &str) -> Option<serde_json::Value> {
    validate_proxy_url(base).ok()?;
    let url = health_url(base);
    let Ok(client) = crate::api::integrations::http_client().await else {
        return None;
    };
    let mut req = client.get(&url);
    if let Some(key) = packetwolf_api_key().filter(|k| !k.trim().is_empty()) {
        req = req.header("X-API-Key", key);
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => resp.json().await.ok(),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct PacketWolfNsQuery {
    namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PacketWolfFlowsQuery {
    namespace: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

#[cfg(feature = "web")]
async fn proxy_packetwolf_json(
    path: &str,
    query_pairs: &[(&str, String)],
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let base = packetwolf_base_url().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "PacketWolf not configured — set VEYRON_PACKETWOLF_URL".to_string(),
    ))?;
    validate_proxy_url(&base).map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
    let url = format!("{}{}", base.trim_end_matches('/'), path);
    let Ok(client) = crate::api::integrations::http_client().await else {
        return Err((
            StatusCode::BAD_GATEWAY,
            "HTTP client unavailable".to_string(),
        ));
    };
    let mut req = client.get(&url);
    if !query_pairs.is_empty() {
        req = req.query(query_pairs);
    }
    if let Some(key) = packetwolf_api_key().filter(|k| !k.trim().is_empty()) {
        req = req.header("X-API-Key", key);
    }
    let resp = req.send().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("PacketWolf request failed: {e}"),
        )
    })?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!(
                "PacketWolf returned {status}: {}",
                body.trim().chars().take(200).collect::<String>()
            ),
        ));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Invalid PacketWolf JSON: {e}"),
        )
    })?;
    Ok(Json(json))
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/packetwolf/status", get(get_packetwolf_status))
        .route(
            "/packetwolf/network/overview",
            get(get_packetwolf_network_overview),
        )
        .route("/packetwolf/flows", get(get_packetwolf_flows))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_packetwolf_status(
    State(_state): State<SharedState>,
) -> Json<PacketWolfStatusResponse> {
    let base = packetwolf_base_url();
    let external = packetwolf_external_url();
    if base.is_none() {
        return Json(PacketWolfStatusResponse {
            configured: false,
            reachable: false,
            api_authorized: false,
            base_url: None,
            message: Some(
                "PacketWolf not configured — set VEYRON_PACKETWOLF_URL or run integrations bootstrap"
                    .to_string(),
            ),
            health_status: None,
            health_mode: None,
            external_url: external,
        });
    }

    let base = base.unwrap();
    let health = probe_packetwolf_health(&base).await;
    let reachable = health.is_some();
    let health_status = health
        .as_ref()
        .and_then(|h| h.get("status"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let health_mode = health
        .as_ref()
        .and_then(|h| h.get("health_mode"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Json(PacketWolfStatusResponse {
        configured: true,
        reachable,
        api_authorized: reachable,
        base_url: Some(base),
        message: if reachable {
            None
        } else {
            Some("PacketWolf API health probe failed".to_string())
        },
        health_status,
        health_mode,
        external_url: external,
    })
}

#[cfg(feature = "web")]
async fn get_packetwolf_network_overview(
    State(_state): State<SharedState>,
    Query(q): Query<PacketWolfNsQuery>,
) -> impl IntoResponse {
    let ns = q
        .namespace
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "all".to_string());
    proxy_packetwolf_json("/api/v1/network/overview", &[("namespace", ns)]).await
}

#[cfg(feature = "web")]
async fn get_packetwolf_flows(
    State(_state): State<SharedState>,
    Query(q): Query<PacketWolfFlowsQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let mut pairs = vec![("limit", limit.to_string())];
    if let Some(ns) = q.namespace.filter(|s| !s.trim().is_empty() && s != "all") {
        pairs.push(("namespace", ns));
    }
    proxy_packetwolf_json("/api/v1/flows", &pairs).await
}
