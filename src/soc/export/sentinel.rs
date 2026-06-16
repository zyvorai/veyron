// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::SocExportStatus;
use crate::soc::event::to_ecs_doc;
use crate::soc::event::SecurityEvent;

static LAST_ERROR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static EVENTS_PUSHED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct SentinelConfig {
    pub dce_url: String,
    pub dcr_rule: String,
    pub stream: String,
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
}

pub fn config() -> Option<SentinelConfig> {
    let dce_url = std::env::var("VEYRON_SENTINEL_DCE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let dcr_rule = std::env::var("VEYRON_SENTINEL_DCR_RULE")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let stream = std::env::var("VEYRON_SENTINEL_STREAM")
        .unwrap_or_else(|_| "Custom-VMRogueSecurity".into());
    let tenant_id = std::env::var("VEYRON_SENTINEL_TENANT_ID").ok()?;
    let client_id = std::env::var("VEYRON_SENTINEL_CLIENT_ID").ok()?;
    let client_secret = std::env::var("VEYRON_SENTINEL_CLIENT_SECRET").ok()?;
    Some(SentinelConfig {
        dce_url: dce_url.trim_end_matches('/').to_string(),
        dcr_rule,
        stream,
        tenant_id,
        client_id,
        client_secret,
    })
}

pub fn status() -> SocExportStatus {
    let configured = config().is_some();
    SocExportStatus {
        backend: "sentinel".to_string(),
        configured,
        last_push_ok: configured && LAST_ERROR.lock().ok().and_then(|g| g.clone()).is_none(),
        last_error: LAST_ERROR.lock().ok().and_then(|g| g.clone()),
        events_pushed: EVENTS_PUSHED.load(std::sync::atomic::Ordering::Relaxed),
    }
}

#[cfg(feature = "web")]
pub async fn push_events(events: &[SecurityEvent]) -> anyhow::Result<u32> {
    let cfg = match config() {
        Some(c) => c,
        None => return Ok(0),
    };
    if events.is_empty() {
        return Ok(0);
    }
    let token = acquire_token(&cfg).await?;
    let mut body = Vec::new();
    for ev in events {
        body.push(to_ecs_doc(ev));
    }
    let url = format!(
        "{}/dataCollectionRules/{}/streams/{}?api-version=2023-01-01",
        cfg.dce_url, cfg.dcr_rule, cfg.stream
    );
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let t = resp.text().await.unwrap_or_default();
        if let Ok(mut g) = LAST_ERROR.lock() {
            *g = Some(t);
        }
        anyhow::bail!("sentinel push failed");
    }
    if let Ok(mut g) = LAST_ERROR.lock() {
        *g = None;
    }
    let n = events.len() as u32;
    EVENTS_PUSHED.fetch_add(n as u64, std::sync::atomic::Ordering::Relaxed);
    Ok(n)
}

#[cfg(feature = "web")]
async fn acquire_token(cfg: &SentinelConfig) -> anyhow::Result<String> {
    let url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        cfg.tenant_id
    );
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .form(&[
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
            ("scope", "https://monitor.azure.com/.default"),
            ("grant_type", "client_credentials"),
        ])
        .send()
        .await?;
    let json: serde_json::Value = resp.json().await?;
    json.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no access_token in sentinel oauth response"))
}

#[cfg(feature = "web")]
pub async fn probe() -> bool {
    config().is_some()
}
