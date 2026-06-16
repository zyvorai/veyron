// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::SocExportStatus;
use crate::soc::event::{to_ecs_doc, SecurityEvent};

static LAST_ERROR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static EVENTS_PUSHED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct ElasticConfig {
    pub url: String,
    pub api_key: String,
    pub index: String,
    pub pipeline: Option<String>,
}

pub fn config() -> Option<ElasticConfig> {
    let url = std::env::var("VEYRON_ELASTIC_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let api_key = std::env::var("VEYRON_ELASTIC_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let index = std::env::var("VEYRON_ELASTIC_INDEX")
        .unwrap_or_else(|_| "logs-vmrogue.security-default".into());
    let pipeline = std::env::var("VEYRON_ELASTIC_PIPELINE")
        .ok()
        .filter(|s| !s.trim().is_empty());
    Some(ElasticConfig {
        url: url.trim_end_matches('/').to_string(),
        api_key,
        index,
        pipeline,
    })
}

pub fn hunt_enabled() -> bool {
    std::env::var("VEYRON_ELASTIC_HUNT_ENABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn status() -> SocExportStatus {
    let configured = config().is_some();
    SocExportStatus {
        backend: "elastic".to_string(),
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
    let mut bulk = String::new();
    for ev in events {
        bulk.push_str(&serde_json::to_string(&serde_json::json!({
            "index": { "_index": cfg.index }
        }))?);
        bulk.push('\n');
        bulk.push_str(&serde_json::to_string(&to_ecs_doc(ev))?);
        bulk.push('\n');
    }
    let url = format!("{}/_bulk", cfg.url);
    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .header("Authorization", format!("ApiKey {}", cfg.api_key))
        .header("Content-Type", "application/x-ndjson");
    if let Some(ref pipe) = cfg.pipeline {
        req = req.query(&[("pipeline", pipe.as_str())]);
    }
    let resp = req.body(bulk).send().await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        if let Ok(mut g) = LAST_ERROR.lock() {
            *g = Some(body.clone());
        }
        anyhow::bail!("elastic bulk: {body}");
    }
    if let Ok(mut g) = LAST_ERROR.lock() {
        *g = None;
    }
    let n = events.len() as u32;
    EVENTS_PUSHED.fetch_add(n as u64, std::sync::atomic::Ordering::Relaxed);
    Ok(n)
}

#[cfg(feature = "web")]
pub async fn run_hunt(query: &str, time_range: &str) -> anyhow::Result<serde_json::Value> {
    let cfg = config().ok_or_else(|| anyhow::anyhow!("Elastic not configured"))?;
    if !hunt_enabled() {
        anyhow::bail!("Elastic hunts disabled — set VEYRON_ELASTIC_HUNT_ENABLED=true");
    }
    let range = parse_time_range(time_range);
    let body = serde_json::json!({
        "size": 100,
        "query": {
            "bool": {
                "must": [
                    { "query_string": { "query": query } },
                    { "range": { "@timestamp": { "gte": range } } }
                ]
            }
        },
        "sort": [{ "@timestamp": "desc" }]
    });
    let url = format!("{}/{}/_search", cfg.url, cfg.index);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("ApiKey {}", cfg.api_key))
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("elastic search HTTP {}", resp.status());
    }
    Ok(resp.json().await?)
}

fn parse_time_range(tr: &str) -> String {
    let hours = tr
        .trim_end_matches('h')
        .parse::<i64>()
        .unwrap_or(24);
    (chrono::Utc::now() - chrono::Duration::hours(hours))
        .to_rfc3339()
}

#[cfg(feature = "web")]
pub async fn probe() -> bool {
    let Some(cfg) = config() else {
        return false;
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build();
    let Ok(client) = client else {
        return false;
    };
    let url = format!("{}/_cluster/health", cfg.url);
    client
        .get(&url)
        .header("Authorization", format!("ApiKey {}", cfg.api_key))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
