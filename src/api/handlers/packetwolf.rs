// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! PacketWolf Network Brain integration — status probe and optional API proxy base.

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
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

pub fn packetwolf_base_url() -> Option<String> {
    crate::api::integrations::env_var("VMROGUE_PACKETWOLF_URL")
}

pub fn packetwolf_external_url() -> Option<String> {
    crate::api::integrations::env_var("VMROGUE_PACKETWOLF_EXTERNAL_URL")
}

pub fn packetwolf_api_key() -> Option<String> {
    crate::api::integrations::env_var("VMROGUE_PACKETWOLF_API_KEY")
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

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/packetwolf/status", get(get_packetwolf_status))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_packetwolf_status(State(_state): State<SharedState>) -> Json<PacketWolfStatusResponse> {
    let base = packetwolf_base_url();
    let external = packetwolf_external_url();
    if base.is_none() {
        return Json(PacketWolfStatusResponse {
            configured: false,
            reachable: false,
            api_authorized: false,
            base_url: None,
            message: Some(
                "PacketWolf not configured — set VMROGUE_PACKETWOLF_URL or run integrations bootstrap"
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
