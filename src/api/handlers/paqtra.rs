// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Paqtra integration (`../paqtra`): Cilium-native network observability over Hubble.
//!
//! Paqtra's API issues a 24h HS256 JWT from `POST /api/v1/auth/login`; the admin account
//! accepts either `ADMIN_PASSWORD` or the install's `API_KEY` as its password. Both are
//! random per install, so unless `VEYRON_PAQTRA_PASSWORD` is set Veyron reads them from
//! Paqtra's own `paqtra-secret`. `/health` is unauthenticated.

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

/// Paqtra's default Helm release (`paqtra install`): Service `<release>-api` on port 9191.
pub const PAQTRA_NAMESPACE: &str = "paqtra";
pub const PAQTRA_SERVICE: &str = "paqtra-api";
pub const PAQTRA_UI_SERVICE: &str = "paqtra-ui";
pub const PAQTRA_PORT: u16 = 9191;
pub const PAQTRA_SECRET: &str = "paqtra-secret";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaqtraStatusResponse {
    pub configured: bool,
    pub reachable: bool,
    pub api_authorized: bool,
    pub base_url: Option<String>,
    /// `env` when VEYRON_PAQTRA_URL is set, `discovered` when found in-cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// `env` or `secret` (read from `paqtra/paqtra-secret`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_source: Option<String>,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Paqtra's own `/health` status (`healthy`, `degraded`, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hubble_connected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flows_per_second: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_flows: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cilium_agents_ready: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cilium_agents_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_url: Option<String>,
}

impl PaqtraStatusResponse {
    fn empty() -> Self {
        Self {
            configured: false,
            reachable: false,
            api_authorized: false,
            base_url: None,
            source: None,
            credentials_source: None,
            message: None,
            version: None,
            health: None,
            hubble_connected: None,
            flows_per_second: None,
            indexed_flows: None,
            retention_days: None,
            cilium_agents_ready: None,
            cilium_agents_total: None,
            health_score: None,
            external_url: paqtra_external_url(),
        }
    }
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

pub fn paqtra_env_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_PAQTRA_URL")
}

pub fn paqtra_external_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_PAQTRA_EXTERNAL_URL")
}

pub fn in_cluster_url() -> String {
    format!("http://{PAQTRA_SERVICE}.{PAQTRA_NAMESPACE}.svc:{PAQTRA_PORT}")
}

#[cfg(feature = "web")]
fn http_client() -> Option<reqwest::Client> {
    let verify = crate::api::integrations::env_var("VEYRON_PAQTRA_TLS_VERIFY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .danger_accept_invalid_certs(!verify)
        .build()
        .ok()
}

/// `VEYRON_PAQTRA_URL`, else Paqtra's API Service when it exists in this cluster.
#[cfg(feature = "web")]
pub async fn paqtra_base_url(kube: &crate::kube::KubeClient) -> Option<(String, &'static str)> {
    if let Some(url) = paqtra_env_url() {
        return Some((url, "env"));
    }
    use k8s_openapi::api::core::v1::Service;
    let services: kube::Api<Service> = kube::Api::namespaced(kube.client(), PAQTRA_NAMESPACE);
    match services.get_opt(PAQTRA_SERVICE).await {
        Ok(Some(_)) => Some((in_cluster_url(), "discovered")),
        _ => None,
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Credentials {
    username: String,
    password: String,
    source: &'static str,
}

/// `VEYRON_PAQTRA_USERNAME`/`VEYRON_PAQTRA_PASSWORD`, else Paqtra's install Secret
/// (`ADMIN_USERNAME` + `API_KEY`, falling back to `ADMIN_PASSWORD`).
#[cfg(feature = "web")]
async fn credentials(kube: &crate::kube::KubeClient) -> Option<Credentials> {
    use crate::api::integrations::env_var;
    if let Some(password) = env_var("VEYRON_PAQTRA_PASSWORD") {
        return Some(Credentials {
            username: env_var("VEYRON_PAQTRA_USERNAME").unwrap_or_else(|| "admin".into()),
            password,
            source: "env",
        });
    }
    use k8s_openapi::api::core::v1::Secret;
    let secrets: kube::Api<Secret> = kube::Api::namespaced(kube.client(), PAQTRA_NAMESPACE);
    let secret = secrets.get_opt(PAQTRA_SECRET).await.ok()??;
    credentials_from_secret(&secret.data.unwrap_or_default())
}

fn credentials_from_secret(
    data: &std::collections::BTreeMap<String, k8s_openapi::ByteString>,
) -> Option<Credentials> {
    let field = |k: &str| {
        data.get(k)
            .and_then(|v| String::from_utf8(v.0.clone()).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    Some(Credentials {
        username: field("ADMIN_USERNAME").unwrap_or_else(|| "admin".into()),
        password: field("API_KEY").or_else(|| field("ADMIN_PASSWORD"))?,
        source: "secret",
    })
}

#[cfg(feature = "web")]
struct CachedToken {
    base: String,
    username: String,
    token: String,
    expires_at: std::time::Instant,
}

#[cfg(feature = "web")]
static TOKEN: tokio::sync::Mutex<Option<CachedToken>> = tokio::sync::Mutex::const_new(None);

/// Logs in (or reuses the cached token for this base URL and user) and returns a bearer.
#[cfg(feature = "web")]
async fn bearer(
    client: &reqwest::Client,
    base: &str,
    creds: &Credentials,
    force: bool,
) -> Result<String, String> {
    let mut cached = TOKEN.lock().await;
    if !force
        && let Some(t) = cached.as_ref()
        && t.base == base
        && t.username == creds.username
        && t.expires_at > std::time::Instant::now()
    {
        return Ok(t.token.clone());
    }
    let resp = client
        .post(format!("{}/api/v1/auth/login", base.trim_end_matches('/')))
        .json(&serde_json::json!({"username": creds.username, "password": creds.password}))
        .send()
        .await
        .map_err(|e| format!("Paqtra login request failed: {e}"))?;
    if !resp.status().is_success() {
        *cached = None;
        return Err(format!("Paqtra login returned {}", resp.status()));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid Paqtra login JSON: {e}"))?;
    let token = body
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or("Paqtra login response has no token")?
        .to_string();
    let ttl = body
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);
    *cached = Some(CachedToken {
        base: base.to_string(),
        username: creds.username.clone(),
        token: token.clone(),
        expires_at: std::time::Instant::now()
            + std::time::Duration::from_secs(ttl.saturating_sub(60).max(1)),
    });
    Ok(token)
}

#[derive(Debug)]
enum PaqtraError {
    NotFound,
    NoCredentials,
    Login(String),
    Upstream(String),
}

impl PaqtraError {
    fn message(&self) -> String {
        match self {
            Self::NotFound => {
                "Paqtra not found in the paqtra namespace — install Paqtra or set VEYRON_PAQTRA_URL"
                    .into()
            }
            Self::NoCredentials => format!(
                "No Paqtra credentials — set VEYRON_PAQTRA_PASSWORD or grant Veyron read access to {PAQTRA_NAMESPACE}/{PAQTRA_SECRET}"
            ),
            Self::Login(e) => format!("{e} — check VEYRON_PAQTRA_USERNAME/VEYRON_PAQTRA_PASSWORD"),
            Self::Upstream(e) => e.clone(),
        }
    }
}

/// Authenticated GET against Paqtra; re-logs in once if the cached token was rejected.
#[cfg(feature = "web")]
async fn paqtra_get_json(
    kube: &crate::kube::KubeClient,
    base: &str,
    path: &str,
    query: &[(&str, String)],
) -> Result<serde_json::Value, PaqtraError> {
    validate_proxy_url(base).map_err(PaqtraError::Upstream)?;
    let client = http_client().ok_or(PaqtraError::Upstream("HTTP client unavailable".into()))?;
    let creds = credentials(kube).await.ok_or(PaqtraError::NoCredentials)?;
    let url = format!("{}{}", base.trim_end_matches('/'), path);
    for attempt in 0..2 {
        let token = bearer(&client, base, &creds, attempt > 0)
            .await
            .map_err(PaqtraError::Login)?;
        let resp = client
            .get(&url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .query(query)
            .send()
            .await
            .map_err(|e| PaqtraError::Upstream(format!("Paqtra request failed: {e}")))?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
            continue;
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(PaqtraError::Upstream(format!(
                "Paqtra returned {status}: {}",
                body.trim().chars().take(200).collect::<String>()
            )));
        }
        return resp
            .json()
            .await
            .map_err(|e| PaqtraError::Upstream(format!("Invalid Paqtra JSON: {e}")));
    }
    Err(PaqtraError::Login(
        "Paqtra rejected the session token".into(),
    ))
}

/// Status probes give up quickly so an upstream restart reads as "unreachable", not a hang.
#[cfg(feature = "web")]
const STATUS_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Paqtra's authenticated status routes query Cilium and the cluster on a cold cache and
/// regularly take 1-5 s, so they get more room than the `/health` liveness probe.
#[cfg(feature = "web")]
const AUTH_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(12);

/// Unauthenticated `/health`; `None` when unreachable.
#[cfg(feature = "web")]
pub async fn probe_health(base: &str) -> Option<serde_json::Value> {
    validate_proxy_url(base).ok()?;
    let resp = http_client()?
        .get(format!("{}/health", base.trim_end_matches('/')))
        .timeout(STATUS_PROBE_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    Some(resp.json().await.unwrap_or(serde_json::Value::Null))
}

#[cfg(feature = "web")]
async fn proxy(
    state: &SharedState,
    path: &str,
    query: &[(&str, String)],
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kube = state.read().await.kube_client.clone();
    let (base, _) = paqtra_base_url(&kube).await.ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            PaqtraError::NotFound.message(),
        )
    })?;
    paqtra_get_json(&kube, &base, path, query)
        .await
        .map(Json)
        .map_err(|e| {
            let code = match e {
                PaqtraError::NoCredentials => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::BAD_GATEWAY,
            };
            (code, e.message())
        })
}

#[derive(Debug, Deserialize)]
struct PaqtraFlowsQuery {
    namespace: Option<String>,
    verdict: Option<String>,
    limit: Option<u32>,
}

fn ns_pair(ns: Option<String>) -> Option<(&'static str, String)> {
    ns.filter(|s| !s.trim().is_empty() && s != "all")
        .map(|ns| ("namespace", ns))
}

const VERDICTS: &[&str] = &[
    "FORWARDED",
    "DROPPED",
    "AUDIT",
    "REDIRECTED",
    "ERROR",
    "TRACED",
];

fn flow_query(q: PaqtraFlowsQuery) -> Vec<(&'static str, String)> {
    let mut pairs = vec![("limit", q.limit.unwrap_or(100).clamp(1, 500).to_string())];
    pairs.extend(ns_pair(q.namespace));
    if let Some(v) = q
        .verdict
        .map(|v| v.trim().to_uppercase())
        .filter(|v| VERDICTS.contains(&v.as_str()))
    {
        pairs.push(("verdict", v));
    }
    pairs
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/paqtra/status", get(get_paqtra_status))
        .route("/paqtra/flows", get(get_paqtra_flows))
        .route("/paqtra/drops", get(get_paqtra_drops))
        .route("/paqtra/posture", get(get_paqtra_posture))
        .with_state(state)
}

/// Fills a status response from Paqtra's unauthenticated `/health` body.
fn apply_health(out: &mut PaqtraStatusResponse, body: &serde_json::Value) {
    let ingest = body.pointer("/subsystems/flow_ingest");
    out.version = body.get("version").and_then(|v| v.as_str()).map(Into::into);
    out.health = body.get("status").and_then(|v| v.as_str()).map(Into::into);
    out.hubble_connected = ingest
        .and_then(|i| i.get("connected"))
        .and_then(|v| v.as_bool());
    out.flows_per_second = ingest
        .and_then(|i| i.get("events_per_sec"))
        .and_then(|v| v.as_f64());
    out.indexed_flows = ingest
        .and_then(|i| i.get("indexed"))
        .and_then(|v| v.as_u64());
    out.retention_days = ingest
        .and_then(|i| i.get("retention_days"))
        .and_then(|v| v.as_u64());
}

#[cfg(feature = "web")]
async fn bounded(
    fut: impl std::future::Future<Output = Result<serde_json::Value, PaqtraError>>,
) -> Result<serde_json::Value, PaqtraError> {
    tokio::time::timeout(AUTH_PROBE_TIMEOUT, fut)
        .await
        .unwrap_or_else(|_| {
            Err(PaqtraError::Upstream(format!(
                "Paqtra API did not answer within {} s",
                AUTH_PROBE_TIMEOUT.as_secs()
            )))
        })
}

#[cfg(feature = "web")]
async fn get_paqtra_status(State(state): State<SharedState>) -> Json<PaqtraStatusResponse> {
    let kube = state.read().await.kube_client.clone();
    let mut out = PaqtraStatusResponse::empty();
    let Some((base, source)) = paqtra_base_url(&kube).await else {
        out.message = Some(PaqtraError::NotFound.message());
        return Json(out);
    };
    out.configured = true;
    out.base_url = Some(base.clone());
    out.source = Some(source.to_string());
    let discover_ui = async {
        if out.external_url.is_some() {
            return None;
        }
        super::integrations::discover_nodeport_url(
            &kube.client(),
            PAQTRA_NAMESPACE,
            PAQTRA_UI_SERVICE,
        )
        .await
        .map(|u| u.replacen("http://", "https://", 1))
    };

    let (ui, health, creds, cilium, cluster) = tokio::join!(
        discover_ui,
        probe_health(&base),
        credentials(&kube),
        bounded(paqtra_get_json(&kube, &base, "/api/v1/cilium/status", &[])),
        bounded(paqtra_get_json(&kube, &base, "/api/v1/cluster/health", &[])),
    );
    if ui.is_some() {
        out.external_url = ui;
    }
    let Some(health) = health else {
        out.message = Some(format!("Paqtra health probe failed at {base}/health"));
        return Json(out);
    };
    out.reachable = true;
    apply_health(&mut out, &health);
    out.credentials_source = creds.map(|c| c.source.to_string());
    let cilium = match cilium {
        Ok(c) => c,
        Err(e) => {
            out.message = Some(e.message());
            return Json(out);
        }
    };
    out.api_authorized = true;
    out.cilium_agents_ready = cilium.get("healthy").and_then(|v| v.as_u64());
    out.cilium_agents_total = cilium.get("total").and_then(|v| v.as_u64());
    if let Ok(cluster) = cluster {
        out.health_score = cluster.get("score").and_then(|v| v.as_f64());
    }
    Json(out)
}

#[cfg(feature = "web")]
async fn get_paqtra_flows(
    State(state): State<SharedState>,
    Query(q): Query<PaqtraFlowsQuery>,
) -> impl IntoResponse {
    proxy(&state, "/api/v1/flows", &flow_query(q)).await
}

#[cfg(feature = "web")]
async fn get_paqtra_drops(State(state): State<SharedState>) -> impl IntoResponse {
    proxy(&state, "/api/v1/modules/rootcause/drops", &[]).await
}

#[cfg(feature = "web")]
async fn get_paqtra_posture(State(state): State<SharedState>) -> impl IntoResponse {
    proxy(&state, "/api/v1/security/posture", &[]).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::ByteString;
    use std::collections::BTreeMap;

    #[test]
    fn blocks_unsafe_proxy_targets() {
        assert!(validate_proxy_url("http://paqtra-api.paqtra.svc:9191").is_ok());
        assert!(validate_proxy_url("http://127.0.0.1:9191").is_err());
        assert!(validate_proxy_url("http://169.254.169.254/").is_err());
        assert!(validate_proxy_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn secret_prefers_api_key_over_admin_password() {
        let mut data = BTreeMap::new();
        data.insert("ADMIN_USERNAME".into(), ByteString(b"ops".to_vec()));
        data.insert("ADMIN_PASSWORD".into(), ByteString(b"pw-123456".to_vec()));
        data.insert("API_KEY".into(), ByteString(b"key-abc".to_vec()));
        let c = credentials_from_secret(&data).unwrap();
        assert_eq!(c.username, "ops");
        assert_eq!(c.password, "key-abc");
        assert_eq!(c.source, "secret");

        data.remove("API_KEY");
        data.remove("ADMIN_USERNAME");
        let c = credentials_from_secret(&data).unwrap();
        assert_eq!(c.username, "admin");
        assert_eq!(c.password, "pw-123456");

        data.remove("ADMIN_PASSWORD");
        assert!(credentials_from_secret(&data).is_none());
    }

    #[test]
    fn health_body_maps_paqtra_fields() {
        let body = serde_json::json!({
            "status": "healthy",
            "version": "2.2.2",
            "subsystems": {"flow_ingest": {
                "connected": true, "events_per_sec": 31.2, "indexed": 13278622, "retention_days": 7
            }}
        });
        let mut out = PaqtraStatusResponse::empty();
        apply_health(&mut out, &body);
        assert_eq!(out.version.as_deref(), Some("2.2.2"));
        assert_eq!(out.health.as_deref(), Some("healthy"));
        assert_eq!(out.hubble_connected, Some(true));
        assert_eq!(out.flows_per_second, Some(31.2));
        assert_eq!(out.indexed_flows, Some(13278622));
        assert_eq!(out.retention_days, Some(7));
    }

    #[test]
    fn flow_query_clamps_and_filters() {
        let q = flow_query(PaqtraFlowsQuery {
            namespace: Some("all".into()),
            verdict: Some("dropped".into()),
            limit: Some(10_000),
        });
        assert_eq!(
            q,
            vec![
                ("limit", "500".to_string()),
                ("verdict", "DROPPED".to_string())
            ]
        );
        let q = flow_query(PaqtraFlowsQuery {
            namespace: Some("prod".into()),
            verdict: Some("bogus".into()),
            limit: None,
        });
        assert_eq!(
            q,
            vec![
                ("limit", "100".to_string()),
                ("namespace", "prod".to_string())
            ]
        );
    }
}
