// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use k8s_openapi::api::core::v1::Service;
use kube::api::{Api, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// Observability stack discovery: metrics, logs, traces.
pub async fn observability_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Observability Advisor",
        "observability_advisor",
        "Observability stack in cluster",
    );

    let k8s = client.client();
    let mut metrics = false;
    let mut logs = false;
    let mut traces = false;
    let mut hits: Vec<String> = Vec::new();

    let probe_ns = [
        "monitoring",
        "logging",
        "observability",
        "kube-system",
        "cilium-system",
        "veyron-system",
        scope,
    ];

    for ns in probe_ns {
        if ns.is_empty() {
            continue;
        }
        let svc_api: Api<Service> = Api::namespaced(k8s.clone(), ns);
        let Ok(svcs) = svc_api.list(&ListParams::default()).await else {
            continue;
        };
        for svc in &svcs.items {
            let name = svc.metadata.name.as_deref().unwrap_or("").to_lowercase();
            if name.contains("prometheus")
                || name.contains("metrics-server")
                || name.contains("thanos")
                || name.contains("victoria")
                || name.contains("kube-state-metrics")
            {
                metrics = true;
                hits.push(format!("metrics: {ns}/{name}"));
            }
            if name.contains("loki")
                || name.contains("elasticsearch")
                || name.contains("fluent")
                || name.contains("vector")
                || name.contains("promtail")
            {
                logs = true;
                hits.push(format!("logs: {ns}/{name}"));
            }
            if name.contains("jaeger")
                || name.contains("tempo")
                || name.contains("otel")
                || name.contains("zipkin")
            {
                traces = true;
                hits.push(format!("traces: {ns}/{name}"));
            }
        }
    }

    let prom_env = std::env::var("VEYRON_PROMETHEUS_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .is_some();
    let loki_env = std::env::var("VEYRON_LOKI_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .is_some();
    let jaeger_env = std::env::var("VEYRON_JAEGER_QUERY_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .is_some();

    r.evidence.push(format!(
        "Metrics: {} (cluster {})",
        if metrics || prom_env {
            "available"
        } else {
            "not detected"
        },
        if prom_env {
            "+ VEYRON_PROMETHEUS_URL"
        } else {
            "services only"
        }
    ));
    r.evidence.push(format!(
        "Logs: {} (cluster {})",
        if logs || loki_env {
            "available"
        } else {
            "not detected"
        },
        if loki_env {
            "+ VEYRON_LOKI_URL"
        } else {
            "services only"
        }
    ));
    r.evidence.push(format!(
        "Traces: {} (cluster {})",
        if traces || jaeger_env {
            "available"
        } else {
            "not detected"
        },
        if jaeger_env {
            "+ VEYRON_JAEGER_QUERY_URL"
        } else {
            "services only"
        }
    ));

    for h in hits.iter().take(8) {
        r.evidence.push(h.clone());
    }

    let pillars = [metrics || prom_env, logs || loki_env, traces || jaeger_env];
    let active = pillars.iter().filter(|&&x| x).count();

    r.summary = match active {
        3 => "Full observability pillars detected — metrics, logs, and traces.".into(),
        0 => {
            "No observability stack detected — install kube-prometheus-stack or set VEYRON_* URLs."
                .into()
        }
        n => format!("{n}/3 observability pillars active in cluster or via env vars."),
    };

    if !metrics && !prom_env {
        r.recommendations.push(
            "Install Prometheus and set VEYRON_PROMETHEUS_URL for storage usage and performance percentiles.".into(),
        );
    }
    if !logs && !loki_env {
        r.recommendations.push(
            "Add Loki or Elasticsearch and set VEYRON_LOKI_URL for LogQL in the dashboard.".into(),
        );
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Observability".into(),
            action: "navigate".into(),
            page: Some("observability".into()),
        },
        CopilotAction {
            label: "Open Monitoring".into(),
            action: "navigate".into(),
            page: Some("monitoring".into()),
        },
    ];
    r
}
