#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Autoscaler policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoscalerPolicy {
    pub name: String,
    pub namespace: String,
    pub target_kind: String,
    pub target_name: String,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub current_replicas: u32,
    pub cpu_threshold: Option<u8>,
    pub memory_threshold: Option<u8>,
    pub enabled: bool,
}

/// Create autoscaler policy request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAutoscalerPolicyRequest {
    pub name: String,
    pub target_name: String,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub cpu_threshold: Option<u8>,
    pub memory_threshold: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutoscalerQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route(
            "/autoscaler/policies",
            get(list_policies).post(create_policy),
        )
        .with_state(state)
}

/// List autoscaler policies by reading HPAs in the namespace.
#[cfg(feature = "web")]
async fn list_policies(
    State(state): State<SharedState>,
    Query(query): Query<AutoscalerQuery>,
) -> Json<Vec<AutoscalerPolicy>> {
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

    let results: Vec<AutoscalerPolicy> = hpas
        .items
        .iter()
        .map(|hpa| {
            let spec = hpa.spec.as_ref();
            let status = hpa.status.as_ref();

            let cpu_threshold = spec
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
                .map(|v| v as u8);

            let memory_threshold = spec
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
                .map(|v| v as u8);

            AutoscalerPolicy {
                name: hpa.metadata.name.clone().unwrap_or_default(),
                namespace: hpa.metadata.namespace.clone().unwrap_or_default(),
                target_kind: spec
                    .map(|s| s.scale_target_ref.kind.clone())
                    .unwrap_or_default(),
                target_name: spec
                    .map(|s| s.scale_target_ref.name.clone())
                    .unwrap_or_default(),
                min_replicas: spec.and_then(|s| s.min_replicas).unwrap_or(1) as u32,
                max_replicas: spec.map(|s| s.max_replicas).unwrap_or(1) as u32,
                current_replicas: status.and_then(|s| s.current_replicas).unwrap_or(0) as u32,
                cpu_threshold,
                memory_threshold,
                enabled: true,
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn create_policy(
    State(state): State<SharedState>,
    Query(query): Query<AutoscalerQuery>,
    Json(req): Json<CreateAutoscalerPolicyRequest>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    use k8s_openapi::api::autoscaling::v2::{
        CrossVersionObjectReference, HorizontalPodAutoscaler, HorizontalPodAutoscalerSpec,
        MetricSpec, MetricTarget, ResourceMetricSource,
    };

    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let api: kube::api::Api<HorizontalPodAutoscaler> =
        kube::api::Api::namespaced(s.client().client(), &namespace);

    let mut metrics = Vec::new();
    if let Some(cpu_pct) = req.cpu_threshold {
        metrics.push(MetricSpec {
            type_: "Resource".to_string(),
            resource: Some(ResourceMetricSource {
                name: "cpu".to_string(),
                target: MetricTarget {
                    type_: "Utilization".to_string(),
                    average_utilization: Some(cpu_pct as i32),
                    ..Default::default()
                },
            }),
            ..Default::default()
        });
    }
    if let Some(mem_pct) = req.memory_threshold {
        metrics.push(MetricSpec {
            type_: "Resource".to_string(),
            resource: Some(ResourceMetricSource {
                name: "memory".to_string(),
                target: MetricTarget {
                    type_: "Utilization".to_string(),
                    average_utilization: Some(mem_pct as i32),
                    ..Default::default()
                },
            }),
            ..Default::default()
        });
    }

    let hpa = HorizontalPodAutoscaler {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(req.name.clone()),
            namespace: Some(namespace.clone()),
            labels: Some(
                [("vmrogue.io/managed".to_string(), "true".to_string())]
                    .into_iter()
                    .collect(),
            ),
            ..Default::default()
        },
        spec: Some(HorizontalPodAutoscalerSpec {
            scale_target_ref: CrossVersionObjectReference {
                api_version: Some("apps/v1".to_string()),
                kind: "Deployment".to_string(),
                name: req.target_name.clone(),
            },
            min_replicas: Some(req.min_replicas as i32),
            max_replicas: req.max_replicas as i32,
            metrics: if metrics.is_empty() {
                None
            } else {
                Some(metrics)
            },
            ..Default::default()
        }),
        ..Default::default()
    };

    match api.create(&kube::api::PostParams::default(), &hpa).await {
        Ok(_) => (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({
                "status": "created",
                "name": req.name,
                "namespace": namespace,
                "target": req.target_name,
                "min_replicas": req.min_replicas,
                "max_replicas": req.max_replicas,
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "CREATE_FAILED",
                "message": format!("Failed to create HPA: {}", e),
            })),
        ),
    }
}
