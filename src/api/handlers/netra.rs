// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Netra integration (`../netra`): eBPF network observability for VM and pod traffic.
//!
//! Netra's controller authenticates with one shared bearer (`NETRA_API_KEY`) and serves
//! HTTPS with a self-signed certificate by default. Veyron probes `/healthz`, reads
//! `/api/v1/status`, and proxies the flow summary and per-VM network view.

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

/// Netra's in-cluster Service (`deploy/controller.yaml` in the Netra repo).
pub const NETRA_NAMESPACE: &str = "netra-system";
pub const NETRA_SERVICE: &str = "netra";
pub const NETRA_PORT: u16 = 30870;
/// Netra's lab default bearer (`../netra/docs/dashboard-login.md`).
pub const NETRA_DEFAULT_API_KEY: &str = "Admin@321";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetraStatusResponse {
    pub configured: bool,
    pub reachable: bool,
    pub api_authorized: bool,
    pub base_url: Option<String>,
    /// `env` when VEYRON_NETRA_URL is set, `discovered` when found in-cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datapath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_agents: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flows_per_second: Option<f64>,
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

pub fn netra_env_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_NETRA_URL")
}

pub fn netra_external_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_NETRA_EXTERNAL_URL")
}

pub fn netra_api_key() -> String {
    crate::api::integrations::env_var("VEYRON_NETRA_API_KEY")
        .unwrap_or_else(|| NETRA_DEFAULT_API_KEY.to_string())
}

fn tls_verify() -> bool {
    crate::api::integrations::env_var("VEYRON_NETRA_TLS_VERIFY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn in_cluster_url() -> String {
    format!("https://{NETRA_SERVICE}.{NETRA_NAMESPACE}.svc:{NETRA_PORT}")
}

#[cfg(feature = "web")]
fn http_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .danger_accept_invalid_certs(!tls_verify())
        .build()
        .ok()
}

/// `VEYRON_NETRA_URL`, else Netra's Service when it exists in this cluster.
#[cfg(feature = "web")]
pub async fn netra_base_url(kube: &crate::kube::KubeClient) -> Option<(String, &'static str)> {
    if let Some(url) = netra_env_url() {
        return Some((url, "env"));
    }
    use k8s_openapi::api::core::v1::Service;
    let services: kube::Api<Service> = kube::Api::namespaced(kube.client(), NETRA_NAMESPACE);
    match services.get_opt(NETRA_SERVICE).await {
        Ok(Some(_)) => Some((in_cluster_url(), "discovered")),
        _ => None,
    }
}

#[cfg(feature = "web")]
async fn netra_get(
    base: &str,
    path: &str,
    query: &[(&str, String)],
) -> Result<reqwest::Response, String> {
    validate_proxy_url(base)?;
    let client = http_client().ok_or("HTTP client unavailable")?;
    let mut req = client
        .get(format!("{}{}", base.trim_end_matches('/'), path))
        .bearer_auth(netra_api_key())
        .header("Accept", "application/json");
    if !query.is_empty() {
        req = req.query(query);
    }
    req.send()
        .await
        .map_err(|e| format!("Netra request failed: {e}"))
}

/// Status probes give up quickly so an upstream restart reads as "unreachable", not a hang.
#[cfg(feature = "web")]
const STATUS_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[cfg(feature = "web")]
pub async fn probe_healthz(base: &str) -> bool {
    matches!(
        tokio::time::timeout(STATUS_PROBE_TIMEOUT, netra_get(base, "/healthz", &[])).await,
        Ok(Ok(r)) if r.status().is_success()
    )
}

#[cfg(feature = "web")]
async fn proxy_netra_json(
    state: &SharedState,
    path: &str,
    query: &[(&str, String)],
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kube = state.read().await.kube_client.clone();
    let (base, _) = netra_base_url(&kube).await.ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Netra not found — install it in netra-system or set VEYRON_NETRA_URL".to_string(),
    ))?;
    let resp = netra_get(&base, path, query)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!(
                "Netra returned {status}: {}",
                body.trim().chars().take(200).collect::<String>()
            ),
        ));
    }
    resp.json()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Invalid Netra JSON: {e}")))
}

#[derive(Debug, Deserialize)]
struct NetraNsQuery {
    namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NetraFlowsQuery {
    namespace: Option<String>,
    #[serde(default)]
    number: Option<u32>,
}

fn ns_pair(ns: Option<String>) -> Option<(&'static str, String)> {
    ns.filter(|s| !s.trim().is_empty() && s != "all")
        .map(|ns| ("namespace", ns))
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/netra/status", get(get_netra_status))
        .route("/netra/flows/summary", get(get_netra_flow_summary))
        .route("/netra/vms", get(get_netra_vms))
        .with_state(state)
}

/// Fills a status response from Netra's `/api/v1/status` body.
fn apply_status_body(out: &mut NetraStatusResponse, body: &serde_json::Value) {
    let s = |k: &str| body.get(k).and_then(|v| v.as_str()).map(str::to_string);
    out.version = s("version");
    out.datapath = s("datapath");
    out.agents = body.get("agents").and_then(|v| v.as_u64());
    out.stale_agents = body.get("staleAgents").and_then(|v| v.as_u64());
    out.mode = body
        .pointer("/fastPath/mode")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    out.flows_per_second = body.pointer("/hubble/flowsRate").and_then(|v| v.as_f64());
}

#[cfg(feature = "web")]
async fn get_netra_status(State(state): State<SharedState>) -> Json<NetraStatusResponse> {
    let kube = state.read().await.kube_client.clone();
    let mut out = NetraStatusResponse {
        configured: false,
        reachable: false,
        api_authorized: false,
        base_url: None,
        source: None,
        message: None,
        version: None,
        datapath: None,
        agents: None,
        stale_agents: None,
        mode: None,
        flows_per_second: None,
        external_url: netra_external_url(),
    };
    let Some((base, source)) = netra_base_url(&kube).await else {
        out.message = Some(
            "Netra not found in netra-system — install Netra or set VEYRON_NETRA_URL".to_string(),
        );
        return Json(out);
    };
    out.configured = true;
    out.base_url = Some(base.clone());
    out.source = Some(source.to_string());
    if out.external_url.is_none() {
        out.external_url = super::integrations::discover_nodeport_url(
            &kube.client(),
            NETRA_NAMESPACE,
            NETRA_SERVICE,
        )
        .await
        .map(|u| u.replacen("http://", "https://", 1));
    }

    let (healthy, status) = tokio::join!(probe_healthz(&base), async {
        tokio::time::timeout(
            STATUS_PROBE_TIMEOUT,
            netra_get(&base, "/api/v1/status", &[]),
        )
        .await
        .unwrap_or_else(|_| Err("Netra /api/v1/status timed out".to_string()))
    });
    out.reachable = healthy;
    if !out.reachable {
        out.message = Some(format!("Netra health probe failed at {base}/healthz"));
        return Json(out);
    }
    match status {
        Ok(r) if r.status().is_success() => {
            out.api_authorized = true;
            if let Ok(body) = r.json::<serde_json::Value>().await {
                apply_status_body(&mut out, &body);
            }
        }
        Ok(r) if r.status() == reqwest::StatusCode::UNAUTHORIZED => {
            out.message = Some(
                "Netra rejected the API key — set VEYRON_NETRA_API_KEY to Netra's NETRA_API_KEY"
                    .to_string(),
            );
        }
        Ok(r) => out.message = Some(format!("Netra /api/v1/status returned {}", r.status())),
        Err(e) => out.message = Some(e),
    }
    Json(out)
}

#[cfg(feature = "web")]
async fn get_netra_flow_summary(
    State(state): State<SharedState>,
    Query(q): Query<NetraFlowsQuery>,
) -> impl IntoResponse {
    let number = q.number.unwrap_or(200).clamp(1, 5000);
    let mut pairs = vec![("number", number.to_string())];
    pairs.extend(ns_pair(q.namespace));
    proxy_netra_json(&state, "/api/v1/flows/summary", &pairs).await
}

#[cfg(feature = "web")]
async fn get_netra_vms(
    State(state): State<SharedState>,
    Query(q): Query<NetraNsQuery>,
) -> impl IntoResponse {
    let pairs: Vec<_> = ns_pair(q.namespace).into_iter().collect();
    proxy_netra_json(&state, "/api/v1/vms", &pairs).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_unsafe_proxy_targets() {
        assert!(validate_proxy_url("https://netra.netra-system.svc:30870").is_ok());
        assert!(validate_proxy_url("http://127.0.0.1:30870").is_err());
        assert!(validate_proxy_url("http://169.254.169.254/").is_err());
        assert!(validate_proxy_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn status_body_maps_netra_fields() {
        let body = serde_json::json!({
            "version": "0.29.0",
            "datapath": "standalone-ebpf",
            "agents": 3,
            "staleAgents": 1,
            "fastPath": {"mode": "observe"},
            "hubble": {"flowsRate": 186.5}
        });
        let mut out = NetraStatusResponse {
            configured: true,
            reachable: true,
            api_authorized: true,
            base_url: None,
            source: None,
            message: None,
            version: None,
            datapath: None,
            agents: None,
            stale_agents: None,
            mode: None,
            flows_per_second: None,
            external_url: None,
        };
        apply_status_body(&mut out, &body);
        assert_eq!(out.version.as_deref(), Some("0.29.0"));
        assert_eq!(out.agents, Some(3));
        assert_eq!(out.stale_agents, Some(1));
        assert_eq!(out.mode.as_deref(), Some("observe"));
        assert_eq!(out.flows_per_second, Some(186.5));
    }

    #[test]
    fn namespace_all_is_not_forwarded() {
        assert!(ns_pair(Some("all".into())).is_none());
        assert!(ns_pair(Some(" ".into())).is_none());
        assert_eq!(
            ns_pair(Some("prod".into())),
            Some(("namespace", "prod".into()))
        );
    }
}
