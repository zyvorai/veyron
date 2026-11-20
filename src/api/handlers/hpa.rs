// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// HPA response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpaResponse {
    pub name: String,
    pub namespace: String,
    pub target_kind: String,
    pub target_name: String,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub current_replicas: u32,
    pub desired_replicas: u32,
    pub cpu_utilization_target: Option<u32>,
    pub cpu_utilization_current: Option<u32>,
    pub memory_utilization_target: Option<u32>,
    pub memory_utilization_current: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HpaQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new().route("/hpa", get(list_hpa)).with_state(state)
}

#[cfg(feature = "web")]
async fn list_hpa(
    State(state): State<SharedState>,
    Query(query): Query<HpaQuery>,
) -> Json<Vec<HpaResponse>> {
    use k8s_openapi::api::autoscaling::v2::HorizontalPodAutoscaler;

    let s = state.read().await;
    let client = s.client().client();
    let api: kube::api::Api<HorizontalPodAutoscaler> = match query.namespace.as_deref() {
        Some("all") => kube::api::Api::all(client),
        Some(ns) => kube::api::Api::namespaced(client, ns),
        None => kube::api::Api::namespaced(client, &s.namespace),
    };
    let hpas = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<HpaResponse> = hpas
        .items
        .iter()
        .map(|hpa| {
            let spec = hpa.spec.as_ref();
            let status = hpa.status.as_ref();

            let target_ref = spec.map(|s| &s.scale_target_ref);
            let target_kind = target_ref.map(|t| t.kind.clone()).unwrap_or_default();
            let target_name = target_ref.map(|t| t.name.clone()).unwrap_or_default();

            // Extract resource metric targets from spec
            let cpu_utilization_target = spec
                .and_then(|s| s.metrics.as_ref())
                .and_then(|metrics| {
                    metrics.iter().find(|m| {
                        m.type_ == "Resource"
                            && m.resource
                                .as_ref()
                                .map(|r| r.name == "cpu")
                                .unwrap_or(false)
                    })
                })
                .and_then(|m| m.resource.as_ref())
                .and_then(|r| r.target.average_utilization)
                .map(|v| v as u32);

            let memory_utilization_target = spec
                .and_then(|s| s.metrics.as_ref())
                .and_then(|metrics| {
                    metrics.iter().find(|m| {
                        m.type_ == "Resource"
                            && m.resource
                                .as_ref()
                                .map(|r| r.name == "memory")
                                .unwrap_or(false)
                    })
                })
                .and_then(|m| m.resource.as_ref())
                .and_then(|r| r.target.average_utilization)
                .map(|v| v as u32);

            // Extract current utilization from status
            let cpu_utilization_current = status
                .and_then(|s| s.current_metrics.as_ref())
                .and_then(|metrics| {
                    metrics.iter().find(|m| {
                        m.type_ == "Resource"
                            && m.resource
                                .as_ref()
                                .map(|r| r.name == "cpu")
                                .unwrap_or(false)
                    })
                })
                .and_then(|m| m.resource.as_ref())
                .and_then(|r| r.current.average_utilization)
                .map(|v| v as u32);

            let memory_utilization_current = status
                .and_then(|s| s.current_metrics.as_ref())
                .and_then(|metrics| {
                    metrics.iter().find(|m| {
                        m.type_ == "Resource"
                            && m.resource
                                .as_ref()
                                .map(|r| r.name == "memory")
                                .unwrap_or(false)
                    })
                })
                .and_then(|m| m.resource.as_ref())
                .and_then(|r| r.current.average_utilization)
                .map(|v| v as u32);

            HpaResponse {
                name: hpa.metadata.name.clone().unwrap_or_default(),
                namespace: hpa.metadata.namespace.clone().unwrap_or_default(),
                target_kind,
                target_name,
                min_replicas: spec.and_then(|s| s.min_replicas).unwrap_or(1) as u32,
                max_replicas: spec.map(|s| s.max_replicas).unwrap_or(1) as u32,
                current_replicas: status.and_then(|s| s.current_replicas).unwrap_or(0) as u32,
                desired_replicas: status.map(|s| s.desired_replicas).unwrap_or(0) as u32,
                cpu_utilization_target,
                cpu_utilization_current,
                memory_utilization_target,
                memory_utilization_current,
            }
        })
        .collect();

    Json(results)
}
