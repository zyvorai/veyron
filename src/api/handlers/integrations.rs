// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Optional backend wiring status for dashboard-next (env + lightweight probe).

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::State,
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStatusItem {
    pub id: String,
    pub name: String,
    pub env_var: String,
    pub configured: bool,
    /// Host (and port) only — no credentials or query strings.
    pub endpoint: Option<String>,
    /// `not_configured` | `ok` | `failed` | `skipped`
    pub probe: String,
    pub feeds: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationsStatusResponse {
    pub vmrogue_context: super::feature_context::VmrogueFeatureContext,
    pub integrations: Vec<IntegrationStatusItem>,
    pub configured_count: u32,
}

struct IntegrationDef {
    id: &'static str,
    name: &'static str,
    env_var: &'static str,
    feeds: &'static str,
}

const DEFINITIONS: &[IntegrationDef] = &[
    IntegrationDef {
        id: "prometheus",
        name: "Prometheus",
        env_var: "VMROGUE_PROMETHEUS_URL",
        feeds: "Storage usage (PVC), metrics timeline, forecasting",
    },
    IntegrationDef {
        id: "alertmanager",
        name: "Alertmanager",
        env_var: "VMROGUE_ALERTMANAGER_URL",
        feeds: "Incidents timeline (active alerts)",
    },
    IntegrationDef {
        id: "loki",
        name: "Loki",
        env_var: "VMROGUE_LOKI_URL",
        feeds: "Logs dashboard (LogQL)",
    },
    IntegrationDef {
        id: "opencost",
        name: "OpenCost",
        env_var: "VMROGUE_OPENCOST_URL",
        feeds: "Costs summary, allocation, budgets context",
    },
    IntegrationDef {
        id: "trivy",
        name: "Trivy",
        env_var: "VMROGUE_TRIVY_URL",
        feeds: "Security findings (CVE merge)",
    },
    IntegrationDef {
        id: "jaeger",
        name: "Jaeger",
        env_var: "VMROGUE_JAEGER_QUERY_URL",
        feeds: "Traces API",
    },
    IntegrationDef {
        id: "argocd",
        name: "Argo CD",
        env_var: "VMROGUE_ARGOCD_URL",
        feeds: "GitOps sync (POST /gitops/sync)",
    },
    IntegrationDef {
        id: "argocd_token",
        name: "Argo CD token",
        env_var: "VMROGUE_ARGOCD_TOKEN",
        feeds: "Bearer auth for Argo CD API",
    },
];

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/integrations/status", get(get_integrations_status))
        .with_state(state)
}

#[cfg(feature = "web")]
fn integrations_context() -> super::feature_context::VmrogueFeatureContext {
    super::feature_context::VmrogueFeatureContext {
        data_source: "api_env_and_probe".to_string(),
        scope: "Optional observability, FinOps, security, tracing, and GitOps backends wired via VMROGUE_* env vars.".to_string(),
        limitations: "Probes are best-effort HTTP checks from the API pod (cluster DNS). OIDC is listed separately when VMROGUE_OIDC_* is set. Deploy bootstrap: scripts/lib/bootstrap-integrations.sh.".to_string(),
    }
}

#[cfg(feature = "web")]
fn redact_endpoint(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let host_port = without_scheme.split('/').next().unwrap_or(without_scheme);
    if host_port.is_empty() {
        None
    } else {
        Some(host_port.to_string())
    }
}

#[cfg(feature = "web")]
async fn probe_url(url: &str) -> bool {
    let Ok(client) = crate::api::integrations::http_client().await else {
        return false;
    };
    match client.get(url).send().await {
        Ok(resp) => resp.status().as_u16() < 500,
        Err(_) => false,
    }
}

#[cfg(feature = "web")]
async fn probe_url_bearer(url: &str, token: &str) -> bool {
    let Ok(client) = crate::api::integrations::http_client().await else {
        return false;
    };
    match client.get(url).bearer_auth(token).send().await {
        Ok(resp) => resp.status().as_u16() < 500,
        Err(_) => false,
    }
}

#[cfg(feature = "web")]
async fn get_integrations_status(
    State(_state): State<SharedState>,
) -> Json<IntegrationsStatusResponse> {
    let mut integrations = Vec::with_capacity(DEFINITIONS.len() + 1);
    let mut configured_count = 0u32;
    let mut probe_futures = Vec::new();

    for def in DEFINITIONS {
        let raw = crate::api::integrations::env_var(def.env_var);
        let configured = raw.is_some();
        if configured {
            configured_count += 1;
        }
        let endpoint = if def.id == "argocd_token" {
            None
        } else {
            raw.as_deref().and_then(redact_endpoint)
        };
        let probe = if def.id == "argocd_token" {
            if configured {
                "ok".to_string()
            } else {
                "not_configured".to_string()
            }
        } else if !configured {
            "not_configured".to_string()
        } else {
            "skipped".to_string()
        };
        if let Some(url) = raw.filter(|_| def.id != "argocd_token") {
            if def.id == "argocd" {
                if let Some(token) = crate::api::integrations::env_var("VMROGUE_ARGOCD_TOKEN") {
                    let apps_url = format!("{}/api/v1/applications", url.trim_end_matches('/'));
                    probe_futures.push((def.id, apps_url, Some(token)));
                } else {
                    probe_futures.push((def.id, url, None));
                }
            } else {
                probe_futures.push((def.id, url, None));
            }
        }
        integrations.push(IntegrationStatusItem {
            id: def.id.to_string(),
            name: def.name.to_string(),
            env_var: def.env_var.to_string(),
            configured,
            endpoint,
            probe,
            feeds: def.feeds.to_string(),
        });
    }

    let oidc_on = crate::api::oidc::oidc_configured();
    if oidc_on {
        configured_count += 1;
    }
    integrations.push(IntegrationStatusItem {
        id: "oidc".to_string(),
        name: "OIDC / SSO".to_string(),
        env_var: "VMROGUE_OIDC_ISSUER or VMROGUE_OIDC_AUTHORIZATION_URL".to_string(),
        configured: oidc_on,
        endpoint: crate::api::integrations::env_var("VMROGUE_OIDC_ISSUER")
            .or_else(|| crate::api::integrations::env_var("VMROGUE_OIDC_AUTHORIZATION_URL"))
            .and_then(|u| redact_endpoint(&u)),
        probe: if oidc_on {
            "ok".to_string()
        } else {
            "not_configured".to_string()
        },
        feeds: "Dashboard login (Bearer / OIDC callback)".to_string(),
    });

    let probe_results: Vec<(String, bool)> = futures_util::future::join_all(probe_futures.into_iter().map(
        |(id, url, token)| async move {
            let ok = if let Some(t) = token {
                probe_url_bearer(&url, &t).await
            } else {
                probe_url(&url).await
            };
            (id.to_string(), ok)
        },
    ))
    .await;

    for (id, ok) in probe_results {
        if let Some(item) = integrations.iter_mut().find(|i| i.id == id) {
            if item.configured {
                item.probe = if ok { "ok".to_string() } else { "failed".to_string() };
            }
        }
    }

    Json(IntegrationsStatusResponse {
        vmrogue_context: integrations_context(),
        integrations,
        configured_count,
    })
}
