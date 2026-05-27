// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Loki log query integration for the dashboard Logs page.

#[cfg(feature = "web")]
use anyhow::{Context, Result};
#[cfg(feature = "web")]
use chrono::Utc;
#[cfg(feature = "web")]
use serde::Deserialize;

#[cfg(feature = "web")]
use super::integrations;

#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
struct LokiQueryResponse {
    data: Option<LokiData>,
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
struct LokiData {
    result: Option<Vec<LokiStream>>,
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
struct LokiStream {
    stream: Option<std::collections::HashMap<String, String>>,
    values: Option<Vec<(String, String)>>,
}

#[cfg(feature = "web")]
pub struct LokiLogLine {
    pub timestamp: String,
    pub message: String,
    pub namespace: String,
    pub pod: String,
}

#[cfg(feature = "web")]
pub async fn query_range(
    namespace_scope: &str,
    search: Option<&str>,
    limit: u32,
) -> Result<Vec<LokiLogLine>> {
    let base = integrations::env_var("VMROGUE_LOKI_URL").context("VMROGUE_LOKI_URL not set")?;
    let now = Utc::now();
    let start = (now - chrono::TimeDelta::hours(1))
        .timestamp_nanos_opt()
        .unwrap_or(0);
    let end = now.timestamp_nanos_opt().unwrap_or(0);

    let mut query = if namespace_scope == "all" {
        "{namespace=~\".+\"}".to_string()
    } else {
        format!("{{namespace=\"{namespace_scope}\"}}")
    };
    if let Some(q) = search.filter(|s| !s.is_empty()) {
        query.push_str(&format!(" |= \"{}\"", q.replace('"', "\\\"")));
    }

    let url = format!(
        "{}/loki/api/v1/query_range?query={}&limit={}&start={}&end={}",
        base.trim_end_matches('/'),
        urlencoding_encode(&query),
        limit.max(1).min(5000),
        start,
        end
    );

    let client = integrations::http_client().await?;
    let mut req = client.get(&url);
    if let Some(t) = integrations::env_var("VMROGUE_LOKI_TOKEN") {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await.context("loki query")?;
    if !resp.status().is_success() {
        anyhow::bail!("loki HTTP {}", resp.status());
    }
    let body: LokiQueryResponse = resp.json().await.context("loki json")?;

    let mut lines = Vec::new();
    for stream in body.data.and_then(|d| d.result).unwrap_or_default() {
        let ns = stream
            .stream
            .as_ref()
            .and_then(|s| s.get("namespace"))
            .cloned()
            .unwrap_or_default();
        let pod = stream
            .stream
            .as_ref()
            .and_then(|s| s.get("pod"))
            .cloned()
            .unwrap_or_default();
        for (ts, msg) in stream.values.unwrap_or_default() {
            lines.push(LokiLogLine {
                timestamp: ts,
                message: msg,
                namespace: ns.clone(),
                pod: pod.clone(),
            });
        }
    }
    Ok(lines)
}

#[cfg(feature = "web")]
fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
