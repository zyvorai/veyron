// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Optional Prometheus instant-query helper for PVC volume usage (kubelet cAdvisor metrics).

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct PromEnvelope {
    status: Option<String>,
    data: Option<PromData>,
}

#[derive(Debug, Deserialize)]
struct PromData {
    result: Option<Vec<PromResult>>,
}

#[derive(Debug, Deserialize)]
struct PromResult {
    metric: Option<HashMap<String, String>>,
    value: Option<(f64, String)>,
}

/// Normalize a Prometheus base URL to the instant-query endpoint.
pub fn instant_query_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.contains("/query") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/api/v1/query")
    }
}

/// Run an instant Prometheus query. `base` is the Prometheus server root or `/api/v1/query` URL.
pub async fn instant_query_vector(
    base: &str,
    query: &str,
) -> Result<Vec<(HashMap<String, String>, f64)>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("reqwest client")?;

    let resp = client
        .get(instant_query_url(base))
        .query(&[("query", query)])
        .send()
        .await
        .context("prometheus GET")?;
    if !resp.status().is_success() {
        anyhow::bail!("prometheus HTTP {}", resp.status());
    }

    let env: PromEnvelope = resp.json().await.context("prometheus json")?;
    if env.status.as_deref() != Some("success") {
        anyhow::bail!("prometheus status {:?}", env.status);
    }

    let mut out = Vec::new();
    for r in env.data.and_then(|d| d.result).unwrap_or_default() {
        if let Some((_, val)) = r.value {
            if let Ok(v) = val.parse::<f64>() {
                out.push((r.metric.unwrap_or_default(), v));
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
struct PromRangeData {
    result: Option<Vec<PromRangeResult>>,
}

#[derive(Debug, Deserialize)]
struct PromRangeResult {
    metric: Option<HashMap<String, String>>,
    values: Option<Vec<(f64, String)>>,
}

#[derive(Debug, Deserialize)]
struct PromRangeEnvelope {
    status: Option<String>,
    data: Option<PromRangeData>,
}

/// Run a Prometheus range query. `base` is the query-API prefix ending in `/query` or `/query_range`.
pub async fn range_query_series(
    base: &str,
    query: &str,
    start_unix: i64,
    end_unix: i64,
    step_secs: u64,
) -> Result<Vec<(HashMap<String, String>, Vec<(i64, f64)>)>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("reqwest client")?;

    let range_url = if base.contains("/query_range") {
        base.trim_end_matches('/').to_string()
    } else if base.ends_with("/query") {
        format!("{}_range", base.trim_end_matches('/'))
    } else {
        format!("{}/api/v1/query_range", base.trim_end_matches('/'))
    };

    let resp = client
        .get(&range_url)
        .query(&[
            ("query", query),
            ("start", &start_unix.to_string()),
            ("end", &end_unix.to_string()),
            ("step", &step_secs.to_string()),
        ])
        .send()
        .await
        .context("prometheus range GET")?;
    if !resp.status().is_success() {
        anyhow::bail!("prometheus range HTTP {}", resp.status());
    }

    let env: PromRangeEnvelope = resp.json().await.context("prometheus range json")?;
    if env.status.as_deref() != Some("success") {
        anyhow::bail!("prometheus range status {:?}", env.status);
    }

    let mut out = Vec::new();
    for r in env.data.and_then(|d| d.result).unwrap_or_default() {
        let points: Vec<(i64, f64)> = r
            .values
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(ts, val)| val.parse::<f64>().ok().map(|v| (ts as i64, v)))
            .collect();
        out.push((r.metric.unwrap_or_default(), points));
    }
    Ok(out)
}

/// Map `(namespace, pvc_name) -> used_bytes` from `kubelet_volume_stats_used_bytes` samples.
pub fn index_pvc_used_bytes(
    samples: Vec<(HashMap<String, String>, f64)>,
) -> HashMap<(String, String), u64> {
    let mut m = HashMap::new();
    for (metric, v) in samples {
        let ns = metric
            .get("namespace")
            .cloned()
            .or_else(|| metric.get("exported_namespace").cloned());
        let pvc = metric
            .get("persistentvolumeclaim")
            .cloned()
            .or_else(|| metric.get("persistentvolume_claim").cloned());
        if let (Some(ns), Some(pvc)) = (ns, pvc) {
            let b = v.max(0.0) as u64;
            m.entry((ns, pvc))
                .and_modify(|e: &mut u64| *e = (*e).max(b))
                .or_insert(b);
        }
    }
    m
}

/// Compute a percentile (0.0–1.0) from a sorted slice of samples.
pub fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p.clamp(0.0, 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn index_pvc_used_bytes_parses_namespace_and_pvc() {
        let mut m1 = HashMap::new();
        m1.insert("namespace".to_string(), "ns1".to_string());
        m1.insert("persistentvolumeclaim".to_string(), "data-disk".to_string());
        let samples = vec![(m1, 1024.0)];
        let m = index_pvc_used_bytes(samples);
        assert_eq!(
            m.get(&("ns1".to_string(), "data-disk".to_string())),
            Some(&1024u64)
        );
    }

    #[test]
    fn index_pvc_used_bytes_takes_max_per_key() {
        let mut a = HashMap::new();
        a.insert("namespace".to_string(), "n".to_string());
        a.insert("persistentvolumeclaim".to_string(), "p".to_string());
        let mut b = HashMap::new();
        b.insert("namespace".to_string(), "n".to_string());
        b.insert("persistentvolumeclaim".to_string(), "p".to_string());
        let m = index_pvc_used_bytes(vec![(a, 100.0), (b, 500.0)]);
        assert_eq!(m.get(&("n".to_string(), "p".to_string())), Some(&500u64));
    }
}
