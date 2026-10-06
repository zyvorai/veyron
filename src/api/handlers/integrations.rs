// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Optional backend wiring status for the web dashboard (env + lightweight probe).

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationOpenLink {
    /// `in_app` (Insights section id) or `external` (browser URL).
    pub kind: String,
    pub label: String,
    pub href: String,
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<IntegrationOpenLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationsStatusResponse {
    pub veyron_context: super::feature_context::VmrogueFeatureContext,
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
        id: "grafana",
        name: "Grafana",
        env_var: "VEYRON_GRAFANA_URL",
        feeds: "Dashboards (external UI when NodePort/URL is reachable)",
    },
    IntegrationDef {
        id: "prometheus",
        name: "Prometheus",
        env_var: "VEYRON_PROMETHEUS_URL",
        feeds: "Storage usage (PVC), metrics timeline, forecasting",
    },
    IntegrationDef {
        id: "alertmanager",
        name: "Alertmanager",
        env_var: "VEYRON_ALERTMANAGER_URL",
        feeds: "Incidents timeline (active alerts)",
    },
    IntegrationDef {
        id: "loki",
        name: "Loki",
        env_var: "VEYRON_LOKI_URL",
        feeds: "Logs dashboard (LogQL)",
    },
    IntegrationDef {
        id: "opencost",
        name: "OpenCost",
        env_var: "VEYRON_OPENCOST_URL",
        feeds: "Costs summary, allocation, budgets context",
    },
    IntegrationDef {
        id: "trivy",
        name: "Trivy",
        env_var: "VEYRON_TRIVY_URL",
        feeds: "Security findings (CVE merge)",
    },
    IntegrationDef {
        id: "jaeger",
        name: "Jaeger",
        env_var: "VEYRON_JAEGER_QUERY_URL",
        feeds: "Traces API",
    },
    IntegrationDef {
        id: "argocd",
        name: "Argo CD",
        env_var: "VEYRON_ARGOCD_URL",
        feeds: "GitOps sync (POST /gitops/sync)",
    },
    IntegrationDef {
        id: "argocd_token",
        name: "Argo CD token",
        env_var: "VEYRON_ARGOCD_TOKEN",
        feeds: "Bearer auth for Argo CD API",
    },
    IntegrationDef {
        id: "elastic",
        name: "Elastic Security",
        env_var: "VEYRON_ELASTIC_URL",
        feeds: "SOC events export (ECS bulk) + optional hunts",
    },
    IntegrationDef {
        id: "splunk",
        name: "Splunk",
        env_var: "VEYRON_SPLUNK_HEC_URL",
        feeds: "SOC events via HEC + optional SPL hunts",
    },
    IntegrationDef {
        id: "sentinel",
        name: "Microsoft Sentinel",
        env_var: "VEYRON_SENTINEL_DCE_URL",
        feeds: "SOC events via Log Analytics DCE",
    },
    IntegrationDef {
        id: "qradar",
        name: "IBM QRadar",
        env_var: "VEYRON_QRADAR_SYSLOG_HOST",
        feeds: "SOC events via LEEF syslog",
    },
    IntegrationDef {
        id: "soar",
        name: "SOAR webhook",
        env_var: "VEYRON_SOAR_WEBHOOK_URL",
        feeds: "Detection fired / playbook triggers",
    },
    IntegrationDef {
        id: "netra",
        name: "Netra",
        env_var: "VEYRON_NETRA_URL",
        feeds: "eBPF network observability: flows, drops, VM traffic (GET /api/v1/netra/status)",
    },
    IntegrationDef {
        id: "paqtra",
        name: "Paqtra",
        env_var: "VEYRON_PAQTRA_URL",
        feeds: "Cilium/Hubble flow history, drops, policy posture (GET /api/v1/paqtra/status)",
    },
    IntegrationDef {
        id: "atlas",
        name: "Atlas",
        env_var: "VEYRON_ATLAS_URL",
        feeds: "Ceph-backed VM disk snapshot/backup/restore (GET /api/v1/atlas/status)",
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
        scope: "Optional observability, FinOps, security, tracing, and GitOps backends wired via VEYRON_* env vars.".to_string(),
        limitations: "Probes are best-effort HTTP checks from the API pod (cluster DNS). OIDC is listed separately when VEYRON_OIDC_* is set. Deploy bootstrap: scripts/lib/bootstrap-integrations.sh.".to_string(),
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
fn in_app_open(id: &str) -> Option<IntegrationOpenLink> {
    let (href, label) = match id {
        "prometheus" => ("monitoring", "Open monitoring"),
        "alertmanager" => ("incidents", "Open incidents"),
        "loki" => ("logs", "Open logs"),
        "opencost" => ("costs", "Open costs"),
        "trivy" => ("security", "Open security"),
        "jaeger" => ("traces", "Open traces"),
        "elastic" | "splunk" | "sentinel" | "qradar" | "soar" => ("soc", "Open SOC"),
        "netra" => ("netra", "Open Netra"),
        "paqtra" => ("paqtra", "Open Paqtra"),
        "atlas" => ("snapshots", "Open snapshots"),
        _ => return None,
    };
    Some(IntegrationOpenLink {
        kind: "in_app".to_string(),
        label: label.to_string(),
        href: href.to_string(),
    })
}

#[cfg(feature = "web")]
fn external_open(label: &str, href: String) -> IntegrationOpenLink {
    IntegrationOpenLink {
        kind: "external".to_string(),
        label: label.to_string(),
        href,
    }
}

#[cfg(feature = "web")]
fn base_url_from_env(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let scheme_end = trimmed.find("://").map(|i| i + 3).unwrap_or(0);
    let rest = &trimmed[scheme_end..];
    let authority = rest.split('/').next().unwrap_or(rest);
    if authority.is_empty() {
        return None;
    }
    let scheme = if trimmed.starts_with("https://") {
        "https"
    } else {
        "http"
    };
    Some(format!("{scheme}://{authority}"))
}

#[cfg(feature = "web")]
async fn first_ready_node_ip(client: &kube::Client) -> Option<String> {
    use k8s_openapi::api::core::v1::Node;

    let api: kube::api::Api<Node> = kube::api::Api::all(client.clone());
    let nodes = api.list(&kube::api::ListParams::default()).await.ok()?;
    for node in nodes.items {
        let ready = node
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .and_then(|conds| {
                conds
                    .iter()
                    .find(|c| c.type_ == "Ready")
                    .map(|c| c.status == "True")
            })
            .unwrap_or(false);
        if !ready {
            continue;
        }
        let addrs = node.status.as_ref().and_then(|s| s.addresses.as_ref())?;
        for prefer in ["ExternalIP", "InternalIP"] {
            if let Some(a) = addrs.iter().find(|a| a.type_ == prefer) {
                let ip = a.address.trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

#[cfg(feature = "web")]
pub(crate) async fn discover_nodeport_url(
    client: &kube::Client,
    namespace: &str,
    name_contains: &str,
) -> Option<String> {
    use k8s_openapi::api::core::v1::Service;

    let node_ip = first_ready_node_ip(client).await?;
    let api: kube::api::Api<Service> = kube::api::Api::namespaced(client.clone(), namespace);
    let svcs = api.list(&kube::api::ListParams::default()).await.ok()?;
    let needle = name_contains.to_lowercase();
    for svc in svcs.items {
        let name = svc.metadata.name.as_deref().unwrap_or("").to_lowercase();
        if !name.contains(&needle) {
            continue;
        }
        if name.contains("grafana") && needle == "prometheus" {
            continue;
        }
        if name.contains("node-exporter") || name.contains("operator") {
            continue;
        }
        let ports = svc.spec.as_ref().and_then(|s| s.ports.as_ref())?;
        let node_port = ports
            .iter()
            .find_map(|p| p.node_port.filter(|np| *np > 0))?;
        let scheme = "http";
        return Some(format!("{scheme}://{node_ip}:{node_port}"));
    }
    None
}

#[cfg(feature = "web")]
async fn resolve_external_open(
    client: &kube::Client,
    id: &str,
    raw_url: Option<&str>,
) -> Option<IntegrationOpenLink> {
    let external_env = match id {
        "grafana" => "VEYRON_GRAFANA_EXTERNAL_URL",
        "prometheus" => "VEYRON_PROMETHEUS_EXTERNAL_URL",
        "alertmanager" => "VEYRON_ALERTMANAGER_EXTERNAL_URL",
        "argocd" => "VEYRON_ARGOCD_EXTERNAL_URL",
        "jaeger" => "VEYRON_JAEGER_EXTERNAL_URL",
        "netra" => "VEYRON_NETRA_EXTERNAL_URL",
        "paqtra" => "VEYRON_PAQTRA_EXTERNAL_URL",
        _ => return None,
    };
    if let Some(url) = crate::api::integrations::env_var(external_env) {
        return Some(external_open("Open console", url));
    }
    if let Some(raw) = raw_url.and_then(base_url_from_env) {
        if id == "argocd" {
            return Some(external_open("Open Argo CD", raw));
        }
    }
    let (ns, hint) = match id {
        "grafana" => ("monitoring", "grafana"),
        "prometheus" => ("monitoring", "prometheus"),
        "alertmanager" => ("monitoring", "alertmanager"),
        "argocd" => {
            for ns in ["argocd", "argo-cd"] {
                if let Some(u) = discover_nodeport_url(client, ns, "argocd-server").await {
                    return Some(external_open("Open Argo CD", u));
                }
                if let Some(u) = discover_nodeport_url(client, ns, "server").await {
                    return Some(external_open("Open Argo CD", u));
                }
            }
            return raw_url
                .and_then(base_url_from_env)
                .map(|u| external_open("Open Argo CD", u));
        }
        "jaeger" => ("monitoring", "jaeger"),
        "netra" => {
            let ns = crate::api::handlers::netra::NETRA_NAMESPACE;
            let svc = crate::api::handlers::netra::NETRA_SERVICE;
            if let Some(u) = discover_nodeport_url(client, ns, svc).await {
                return Some(external_open(
                    "Open Netra",
                    u.replacen("http://", "https://", 1),
                ));
            }
            return raw_url
                .and_then(base_url_from_env)
                .map(|u| external_open("Open Netra", u));
        }
        "paqtra" => {
            use crate::api::handlers::paqtra::{PAQTRA_NAMESPACE, PAQTRA_UI_SERVICE};
            return discover_nodeport_url(client, PAQTRA_NAMESPACE, PAQTRA_UI_SERVICE)
                .await
                .map(|u| external_open("Open Paqtra", u.replacen("http://", "https://", 1)));
        }
        _ => return None,
    };
    discover_nodeport_url(client, ns, hint)
        .await
        .map(|u| external_open("Open console", u))
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
    State(state): State<SharedState>,
) -> Json<IntegrationsStatusResponse> {
    let kube_client = state.read().await.client().client().clone();
    let netra_base = {
        let kube = state.read().await.kube_client.clone();
        crate::api::handlers::netra::netra_base_url(&kube)
            .await
            .map(|(url, _)| url)
    };
    let paqtra_base = {
        let kube = state.read().await.kube_client.clone();
        crate::api::handlers::paqtra::paqtra_base_url(&kube)
            .await
            .map(|(url, _)| url)
    };
    let mut integrations = Vec::with_capacity(DEFINITIONS.len() + 1);
    let mut configured_count = 0u32;
    let mut probe_futures = Vec::new();

    for def in DEFINITIONS {
        let raw = if def.id == "netra" {
            netra_base.clone()
        } else if def.id == "paqtra" {
            paqtra_base.clone()
        } else {
            crate::api::integrations::env_var(def.env_var)
        };
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
        if let Some(url) = raw.as_deref().filter(|_| def.id != "argocd_token") {
            if def.id == "argocd" {
                if let Some(token) = crate::api::integrations::env_var("VEYRON_ARGOCD_TOKEN") {
                    let apps_url = format!("{}/api/v1/applications", url.trim_end_matches('/'));
                    probe_futures.push((def.id, apps_url.to_string(), Some(token)));
                } else {
                    probe_futures.push((def.id, url.to_string(), None));
                }
            } else if def.id == "netra" || def.id == "paqtra" {
                // Probed below with the integration's own client.
            } else if def.id == "atlas" {
                let health = if url.trim_end_matches('/').ends_with("/health") {
                    url.to_string()
                } else {
                    format!("{}/health", url.trim_end_matches('/'))
                };
                probe_futures.push((def.id, health, None));
            } else {
                probe_futures.push((def.id, url.to_string(), None));
            }
        }
        let mut open = if def.id == "argocd_token" {
            None
        } else if matches!(
            def.id,
            "grafana" | "argocd" | "prometheus" | "alertmanager" | "jaeger" | "netra" | "paqtra"
        ) {
            resolve_external_open(&kube_client, def.id, raw.as_deref())
                .await
                .or_else(|| in_app_open(def.id))
        } else if configured {
            in_app_open(def.id)
        } else {
            None
        };
        if open.is_none() && def.id == "argocd" && configured {
            open = Some(IntegrationOpenLink {
                kind: "in_app_view".to_string(),
                label: "Open Platform".to_string(),
                href: "platform".to_string(),
            });
        }

        integrations.push(IntegrationStatusItem {
            id: def.id.to_string(),
            name: def.name.to_string(),
            env_var: def.env_var.to_string(),
            configured,
            endpoint,
            probe,
            feeds: def.feeds.to_string(),
            open,
        });
    }

    let oidc_on = crate::api::oidc::oidc_configured();
    if oidc_on {
        configured_count += 1;
    }
    integrations.push(IntegrationStatusItem {
        id: "oidc".to_string(),
        name: "OIDC / SSO".to_string(),
        env_var: "VEYRON_OIDC_ISSUER or VEYRON_OIDC_AUTHORIZATION_URL".to_string(),
        configured: oidc_on,
        endpoint: crate::api::integrations::env_var("VEYRON_OIDC_ISSUER")
            .or_else(|| crate::api::integrations::env_var("VEYRON_OIDC_AUTHORIZATION_URL"))
            .and_then(|u| redact_endpoint(&u)),
        probe: if oidc_on {
            "ok".to_string()
        } else {
            "not_configured".to_string()
        },
        feeds: "Dashboard login (Bearer / OIDC callback)".to_string(),
        open: None,
    });

    let probe_results: Vec<(String, bool)> =
        futures_util::future::join_all(probe_futures.into_iter().map(
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
                item.probe = if ok {
                    "ok".to_string()
                } else {
                    "failed".to_string()
                };
            }
        }
    }

    for item in integrations.iter_mut() {
        if !item.configured {
            continue;
        }
        let ok = match item.id.as_str() {
            "elastic" => crate::soc::export::elastic::probe().await,
            "splunk" => crate::soc::export::splunk::probe().await,
            "sentinel" => crate::soc::export::sentinel::probe().await,
            "qradar" => crate::soc::export::qradar::probe().await,
            "netra" => match netra_base.as_deref() {
                Some(base) => crate::api::handlers::netra::probe_healthz(base).await,
                None => false,
            },
            "paqtra" => match paqtra_base.as_deref() {
                Some(base) => crate::api::handlers::paqtra::probe_health(base)
                    .await
                    .is_some(),
                None => false,
            },
            _ => continue,
        };
        item.probe = if ok {
            "ok".to_string()
        } else {
            "failed".to_string()
        };
    }

    Json(IntegrationsStatusResponse {
        veyron_context: integrations_context(),
        integrations,
        configured_count,
    })
}
