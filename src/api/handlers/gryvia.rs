// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Gryvia integration (`../gravia`, `zyvorai/gryvia`): the Kubernetes GPU platform for
//! containerized training and inference. Veyron owns GPUs attached to VMs; Gryvia owns
//! GPU job admission, tenants, quotas and GPU-hour metering.
//!
//! Gryvia's api-gateway serves HTTPS with a self-signed certificate and accepts one shared
//! bearer (`GRYVIA_API_KEY`, stored in `gryvia-system/gryvia-api-key`). `/health` is
//! unauthenticated. Veyron probes it, reads `/api/cluster/stats`, and proxies the GPU node,
//! tenant, quota and usage lists read-only.

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

/// Gryvia's in-cluster Services (`helm/gryvia` in the Gryvia repo).
pub const GRYVIA_NAMESPACE: &str = "gryvia-system";
pub const GRYVIA_SERVICE: &str = "gryvia-api-gateway";
pub const GRYVIA_UI_SERVICE: &str = "gryvia-ui";
pub const GRYVIA_PORT: u16 = 8080;
pub const GRYVIA_SECRET: &str = "gryvia-api-key";
pub const GRYVIA_SECRET_KEY: &str = "GRYVIA_API_KEY";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GryviaStats {
    pub total_gpus: Option<u64>,
    pub available_gpus: Option<u64>,
    pub allocated_gpus: Option<u64>,
    pub utilization_percent: Option<f64>,
    pub total_nodes: Option<u64>,
    pub running_jobs: Option<u64>,
    pub pending_jobs: Option<u64>,
    pub failed_jobs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GryviaStatusResponse {
    pub configured: bool,
    pub reachable: bool,
    pub api_authorized: bool,
    pub base_url: Option<String>,
    /// `env` when VEYRON_GRYVIA_URL is set, `discovered` when found in-cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// `env` (VEYRON_GRYVIA_API_KEY) or `secret` (gryvia-system/gryvia-api-key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_source: Option<String>,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<GryviaStats>,
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

pub fn gryvia_env_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_GRYVIA_URL")
}

pub fn gryvia_external_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_GRYVIA_EXTERNAL_URL")
}

fn tls_verify() -> bool {
    crate::api::integrations::env_var("VEYRON_GRYVIA_TLS_VERIFY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn in_cluster_url() -> String {
    format!("https://{GRYVIA_SERVICE}.{GRYVIA_NAMESPACE}.svc:{GRYVIA_PORT}")
}

#[cfg(feature = "web")]
fn http_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .danger_accept_invalid_certs(!tls_verify())
        .build()
        .ok()
}

/// `VEYRON_GRYVIA_URL`, else Gryvia's api-gateway Service when it exists in this cluster.
#[cfg(feature = "web")]
pub async fn gryvia_base_url(kube: &crate::kube::KubeClient) -> Option<(String, &'static str)> {
    if let Some(url) = gryvia_env_url() {
        return Some((url, "env"));
    }
    use k8s_openapi::api::core::v1::Service;
    let services: kube::Api<Service> = kube::Api::namespaced(kube.client(), GRYVIA_NAMESPACE);
    match services.get_opt(GRYVIA_SERVICE).await {
        Ok(Some(_)) => Some((in_cluster_url(), "discovered")),
        _ => None,
    }
}

fn key_from_secret_data(
    data: &std::collections::BTreeMap<String, k8s_openapi::ByteString>,
) -> Option<String> {
    data.get(GRYVIA_SECRET_KEY)
        .and_then(|b| String::from_utf8(b.0.clone()).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// `VEYRON_GRYVIA_API_KEY`, else the key from Gryvia's own install Secret.
#[cfg(feature = "web")]
pub async fn gryvia_api_key(kube: &crate::kube::KubeClient) -> Option<(String, &'static str)> {
    if let Some(key) = crate::api::integrations::env_var("VEYRON_GRYVIA_API_KEY") {
        return Some((key, "env"));
    }
    use k8s_openapi::api::core::v1::Secret;
    let secrets: kube::Api<Secret> = kube::Api::namespaced(kube.client(), GRYVIA_NAMESPACE);
    let secret = secrets.get_opt(GRYVIA_SECRET).await.ok()??;
    key_from_secret_data(&secret.data.unwrap_or_default()).map(|k| (k, "secret"))
}

#[cfg(feature = "web")]
async fn gryvia_get(
    base: &str,
    path: &str,
    key: Option<&str>,
    query: &[(&str, String)],
) -> Result<reqwest::Response, String> {
    validate_proxy_url(base)?;
    let client = http_client().ok_or("HTTP client unavailable")?;
    let mut req = client
        .get(format!("{}{}", base.trim_end_matches('/'), path))
        .header("Accept", "application/json");
    if let Some(key) = key {
        req = req.bearer_auth(key);
    }
    if !query.is_empty() {
        req = req.query(query);
    }
    req.send()
        .await
        .map_err(|e| format!("Gryvia request failed: {e}"))
}

/// Status probes give up quickly so an upstream restart reads as "unreachable", not a hang.
#[cfg(feature = "web")]
const STATUS_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[cfg(feature = "web")]
pub async fn probe_health(base: &str) -> bool {
    matches!(
        tokio::time::timeout(STATUS_PROBE_TIMEOUT, gryvia_get(base, "/health", None, &[])).await,
        Ok(Ok(r)) if r.status().is_success()
    )
}

#[cfg(feature = "web")]
async fn proxy_gryvia_json(
    state: &SharedState,
    path: &str,
    query: &[(&str, String)],
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kube = state.read().await.kube_client.clone();
    let (base, _) = gryvia_base_url(&kube).await.ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Gryvia not found — install it in gryvia-system or set VEYRON_GRYVIA_URL".to_string(),
    ))?;
    let (key, _) = gryvia_api_key(&kube).await.ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "No Gryvia API key — set VEYRON_GRYVIA_API_KEY or create gryvia-system/gryvia-api-key"
            .to_string(),
    ))?;
    let resp = gryvia_get(&base, path, Some(&key), query)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!(
                "Gryvia returned {status}: {}",
                body.trim().chars().take(200).collect::<String>()
            ),
        ));
    }
    resp.json()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Invalid Gryvia JSON: {e}")))
}

#[derive(Debug, Deserialize)]
struct GryviaUsageQuery {
    tenant: Option<String>,
    from: Option<String>,
    to: Option<String>,
    group_by: Option<String>,
}

/// Forwards only the usage filters Gryvia accepts; `group_by` outside tenant|sku|day is dropped.
fn usage_pairs(q: GryviaUsageQuery) -> Vec<(&'static str, String)> {
    let mut pairs = Vec::new();
    let keep = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(t) = keep(q.tenant) {
        pairs.push(("tenant", t));
    }
    if let Some(f) = keep(q.from) {
        pairs.push(("from", f));
    }
    if let Some(t) = keep(q.to) {
        pairs.push(("to", t));
    }
    if let Some(g) = keep(q.group_by).filter(|g| matches!(g.as_str(), "tenant" | "sku" | "day")) {
        pairs.push(("groupBy", g));
    }
    pairs
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/gryvia/status", get(get_gryvia_status))
        .route("/gryvia/nodes", get(get_gryvia_nodes))
        .route("/gryvia/tenants", get(get_gryvia_tenants))
        .route("/gryvia/quotas", get(get_gryvia_quotas))
        .route("/gryvia/usage", get(get_gryvia_usage))
        .with_state(state)
}

/// Maps Gryvia's camelCase `/api/cluster/stats` body.
fn stats_from_body(body: &serde_json::Value) -> GryviaStats {
    let u = |k: &str| body.get(k).and_then(|v| v.as_u64());
    GryviaStats {
        total_gpus: u("totalGPUs"),
        available_gpus: u("availableGPUs"),
        allocated_gpus: u("allocatedGPUs"),
        utilization_percent: body.get("utilizationPercent").and_then(|v| v.as_f64()),
        total_nodes: u("totalNodes"),
        running_jobs: u("runningJobs"),
        pending_jobs: u("pendingJobs"),
        failed_jobs: u("failedJobs"),
    }
}

#[cfg(feature = "web")]
async fn get_gryvia_status(State(state): State<SharedState>) -> Json<GryviaStatusResponse> {
    let kube = state.read().await.kube_client.clone();
    let mut out = GryviaStatusResponse {
        configured: false,
        reachable: false,
        api_authorized: false,
        base_url: None,
        source: None,
        key_source: None,
        message: None,
        stats: None,
        external_url: gryvia_external_url(),
    };
    let Some((base, source)) = gryvia_base_url(&kube).await else {
        out.message = Some(
            "Gryvia not found in gryvia-system — install Gryvia or set VEYRON_GRYVIA_URL"
                .to_string(),
        );
        return Json(out);
    };
    out.configured = true;
    out.base_url = Some(base.clone());
    out.source = Some(source.to_string());
    if out.external_url.is_none() {
        out.external_url = super::integrations::discover_nodeport_url(
            &kube.client(),
            GRYVIA_NAMESPACE,
            GRYVIA_UI_SERVICE,
        )
        .await
        .map(|u| u.replacen("http://", "https://", 1));
    }

    out.reachable = probe_health(&base).await;
    if !out.reachable {
        out.message = Some(format!("Gryvia health probe failed at {base}/health"));
        return Json(out);
    }
    let Some((key, key_source)) = gryvia_api_key(&kube).await else {
        out.message = Some(
            "No Gryvia API key — set VEYRON_GRYVIA_API_KEY or create gryvia-system/gryvia-api-key"
                .to_string(),
        );
        return Json(out);
    };
    out.key_source = Some(key_source.to_string());
    let stats = tokio::time::timeout(
        STATUS_PROBE_TIMEOUT,
        gryvia_get(&base, "/api/cluster/stats", Some(&key), &[]),
    )
    .await
    .unwrap_or_else(|_| Err("Gryvia /api/cluster/stats timed out".to_string()));
    match stats {
        Ok(r) if r.status().is_success() => {
            out.api_authorized = true;
            if let Ok(body) = r.json::<serde_json::Value>().await {
                out.stats = Some(stats_from_body(&body));
            }
        }
        Ok(r)
            if r.status() == reqwest::StatusCode::UNAUTHORIZED
                || r.status() == reqwest::StatusCode::FORBIDDEN =>
        {
            out.message = Some(
                "Gryvia rejected the API key — set VEYRON_GRYVIA_API_KEY to Gryvia's GRYVIA_API_KEY"
                    .to_string(),
            );
        }
        Ok(r) => out.message = Some(format!("Gryvia /api/cluster/stats returned {}", r.status())),
        Err(e) => out.message = Some(e),
    }
    Json(out)
}

#[cfg(feature = "web")]
async fn get_gryvia_nodes(State(state): State<SharedState>) -> impl IntoResponse {
    proxy_gryvia_json(&state, "/api/nodes", &[]).await
}

#[cfg(feature = "web")]
async fn get_gryvia_tenants(State(state): State<SharedState>) -> impl IntoResponse {
    proxy_gryvia_json(&state, "/api/tenants", &[]).await
}

#[cfg(feature = "web")]
async fn get_gryvia_quotas(State(state): State<SharedState>) -> impl IntoResponse {
    proxy_gryvia_json(&state, "/api/quotas", &[]).await
}

#[cfg(feature = "web")]
async fn get_gryvia_usage(
    State(state): State<SharedState>,
    Query(q): Query<GryviaUsageQuery>,
) -> impl IntoResponse {
    proxy_gryvia_json(&state, "/api/usage", &usage_pairs(q)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_unsafe_proxy_targets() {
        assert!(validate_proxy_url(&in_cluster_url()).is_ok());
        assert!(validate_proxy_url("http://127.0.0.1:8080").is_err());
        assert!(validate_proxy_url("http://169.254.169.254/").is_err());
        assert!(validate_proxy_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn stats_map_gryvia_fields() {
        let body = serde_json::json!({
            "totalGPUs": 8, "availableGPUs": 3, "allocatedGPUs": 5,
            "utilizationPercent": 61.5, "totalNodes": 2,
            "runningJobs": 4, "pendingJobs": 1, "failedJobs": 0
        });
        let s = stats_from_body(&body);
        assert_eq!(s.total_gpus, Some(8));
        assert_eq!(s.available_gpus, Some(3));
        assert_eq!(s.allocated_gpus, Some(5));
        assert_eq!(s.utilization_percent, Some(61.5));
        assert_eq!(s.total_nodes, Some(2));
        assert_eq!(s.running_jobs, Some(4));
    }

    #[test]
    fn secret_key_is_trimmed_and_required() {
        use k8s_openapi::ByteString;
        let mut data = std::collections::BTreeMap::new();
        data.insert(
            GRYVIA_SECRET_KEY.to_string(),
            ByteString(b" abc123\n".to_vec()),
        );
        assert_eq!(key_from_secret_data(&data).as_deref(), Some("abc123"));
        data.insert(GRYVIA_SECRET_KEY.to_string(), ByteString(b"  ".to_vec()));
        assert!(key_from_secret_data(&data).is_none());
        assert!(key_from_secret_data(&Default::default()).is_none());
    }

    #[test]
    fn usage_filters_only_forward_supported_values() {
        let pairs = usage_pairs(GryviaUsageQuery {
            tenant: Some("acme".into()),
            from: Some(" ".into()),
            to: None,
            group_by: Some("sku".into()),
        });
        assert_eq!(
            pairs,
            vec![
                ("tenant", "acme".to_string()),
                ("groupBy", "sku".to_string())
            ]
        );
        let bad = usage_pairs(GryviaUsageQuery {
            tenant: None,
            from: None,
            to: None,
            group_by: Some("node; drop".into()),
        });
        assert!(bad.is_empty());
    }
}
