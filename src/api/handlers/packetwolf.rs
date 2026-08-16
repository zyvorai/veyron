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

/// PacketWolf's actual auth model (confirmed against a live deployment): `/api/v1/auth/login`
/// with a username/password returns a short-lived (~24h) JWT that must be sent as
/// `Authorization: Bearer <token>` — a static `X-API-Key` header (what packetwolf_api_key()
/// sends) is simply not an auth mechanism this API accepts, so a deployment without these
/// two set can never succeed no matter what VEYRON_PACKETWOLF_API_KEY contains.
pub fn packetwolf_username() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_PACKETWOLF_USERNAME")
}

pub fn packetwolf_password() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_PACKETWOLF_PASSWORD")
}

#[cfg(feature = "web")]
#[derive(Debug, Clone)]
struct PacketWolfToken {
    token: String,
    expires_at: std::time::Instant,
}

#[cfg(feature = "web")]
static PACKETWOLF_TOKEN: std::sync::OnceLock<tokio::sync::RwLock<Option<PacketWolfToken>>> =
    std::sync::OnceLock::new();

#[cfg(feature = "web")]
fn packetwolf_token_cache() -> &'static tokio::sync::RwLock<Option<PacketWolfToken>> {
    PACKETWOLF_TOKEN.get_or_init(|| tokio::sync::RwLock::new(None))
}

/// Logs in to PacketWolf (if VEYRON_PACKETWOLF_USERNAME/PASSWORD are set) and returns a
/// cached bearer token, transparently re-logging in ~60s before the cached token expires.
/// Returns None if username/password aren't configured (caller falls back to X-API-Key).
#[cfg(feature = "web")]
async fn packetwolf_bearer_token(client: &reqwest::Client, base: &str) -> Option<String> {
    let username = packetwolf_username()?;
    let password = packetwolf_password()?;

    {
        let cached = packetwolf_token_cache().read().await;
        if let Some(t) = cached.as_ref() {
            if t.expires_at > std::time::Instant::now() {
                return Some(t.token.clone());
            }
        }
    }

    let login_url = format!("{}/api/v1/auth/login", base.trim_end_matches('/'));
    let resp = client
        .post(&login_url)
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        log::warn!(
            "PacketWolf login failed: HTTP {}",
            resp.status().as_u16()
        );
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    let token = body.get("token")?.as_str()?.to_string();
    let expires_in = body
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);
    let expires_at = std::time::Instant::now()
        + std::time::Duration::from_secs(expires_in.saturating_sub(60).max(1));

    let mut cached = packetwolf_token_cache().write().await;
    *cached = Some(PacketWolfToken {
        token: token.clone(),
        expires_at,
    });
    Some(token)
}

/// Applies whichever PacketWolf auth mechanism is actually configured. JWT bearer-token
/// login takes priority (it's the only mechanism this API genuinely accepts); a static
/// X-API-Key header is sent as a fallback for PacketWolf deployments/versions that use
/// that model instead.
#[cfg(feature = "web")]
async fn apply_packetwolf_auth(
    req: reqwest::RequestBuilder,
    client: &reqwest::Client,
    base: &str,
) -> reqwest::RequestBuilder {
    if let Some(token) = packetwolf_bearer_token(client, base).await {
        return req.bearer_auth(token);
    }
    if let Some(key) = packetwolf_api_key().filter(|k| !k.trim().is_empty()) {
        return req.header("X-API-Key", key);
    }
    req
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
    let req = apply_packetwolf_auth(client.get(&url), &client, base).await;
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
    req = apply_packetwolf_auth(req, &client, &base).await;
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
