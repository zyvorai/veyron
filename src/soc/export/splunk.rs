// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::SocExportStatus;
use crate::soc::event::SecurityEvent;

static LAST_ERROR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static EVENTS_PUSHED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct SplunkConfig {
    pub hec_url: String,
    pub token: String,
    pub index: Option<String>,
    pub sourcetype: String,
}

pub fn config() -> Option<SplunkConfig> {
    let hec_url = std::env::var("VEYRON_SPLUNK_HEC_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let token = std::env::var("VEYRON_SPLUNK_HEC_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let index = std::env::var("VEYRON_SPLUNK_INDEX").ok().filter(|s| !s.is_empty());
    let sourcetype = std::env::var("VEYRON_SPLUNK_SOURCETYPE")
        .unwrap_or_else(|_| "veyron:security".into());
    Some(SplunkConfig {
        hec_url: hec_url.trim_end_matches('/').to_string(),
        token,
        index,
        sourcetype,
    })
}

pub fn status() -> SocExportStatus {
    let configured = config().is_some();
    SocExportStatus {
        backend: "splunk".to_string(),
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
    let client = reqwest::Client::new();
    let url = format!("{}/services/collector/event", cfg.hec_url);
    let mut pushed = 0u32;
    for ev in events {
        let payload = serde_json::json!({
            "event": ev,
            "sourcetype": cfg.sourcetype,
            "index": cfg.index,
        });
        let resp = client
            .post(&url)
            .header("Authorization", format!("Splunk {}", cfg.token))
            .json(&payload)
            .send()
            .await?;
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            if let Ok(mut g) = LAST_ERROR.lock() {
                *g = Some(body);
            }
            anyhow::bail!("splunk hec failed");
        }
        pushed += 1;
    }
    if let Ok(mut g) = LAST_ERROR.lock() {
        *g = None;
    }
    EVENTS_PUSHED.fetch_add(pushed as u64, std::sync::atomic::Ordering::Relaxed);
    Ok(pushed)
}

#[cfg(feature = "web")]
pub async fn run_hunt(query: &str, time_range: &str) -> anyhow::Result<serde_json::Value> {
    let base = std::env::var("VEYRON_SPLUNK_REST_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("VEYRON_SPLUNK_REST_URL not set"))?;
    let token = std::env::var("VEYRON_SPLUNK_HEC_TOKEN")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("VEYRON_SPLUNK_HEC_TOKEN not set"))?;
    let earliest = parse_earliest(time_range);
    let spl = format!("search {query} earliest={earliest} | head 100");
    let url = format!(
        "{}/services/search/jobs/export?output_mode=json",
        base.trim_end_matches('/')
    );
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .form(&[("search", spl.as_str())])
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("splunk search HTTP {}", resp.status());
    }
    let text = resp.text().await?;
    Ok(serde_json::json!({ "raw": text, "backend": "splunk" }))
}

fn parse_earliest(tr: &str) -> String {
    let h = tr.trim_end_matches('h').parse::<i64>().unwrap_or(24);
    format!("-{}h", h)
}

#[cfg(feature = "web")]
pub async fn probe() -> bool {
    let Some(cfg) = config() else {
        return false;
    };
    let client = reqwest::Client::new();
    client
        .get(&cfg.hec_url)
        .header("Authorization", format!("Splunk {}", cfg.token))
        .send()
        .await
        .map(|r| r.status().is_success() || r.status().as_u16() == 405)
        .unwrap_or(false)
}
