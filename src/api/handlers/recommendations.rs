#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub category: String,
    pub priority: String,
    pub title: String,
    pub description: String,
    pub resource: String,
    pub estimated_savings: Option<f64>,
    pub impact: String,
    pub effort: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/recommendations", get(list_recommendations))
        .with_state(state)
}

/// Generate recommendations by analyzing live VM configurations.
#[cfg(feature = "web")]
async fn list_recommendations(State(state): State<SharedState>) -> Json<Vec<Recommendation>> {
    let s = state.read().await;
    let vms = s.client().list_vms(&s.namespace).await.unwrap_or_default();
    let mut recs = Vec::new();
    let mut id_counter = 0u32;

    for vm in &vms {
        let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let domain = &vm.spec.template.spec.domain;
        let devices = domain.devices.as_ref();

        // Check for missing resource limits
        if domain.resources.limits.is_none() {
            id_counter += 1;
            recs.push(Recommendation {
                id: format!("REC-{:04}", id_counter),
                category: "ResourceOptimization".to_string(),
                priority: "High".to_string(),
                title: "Set resource limits".to_string(),
                description: format!(
                    "VM '{}' has no CPU/memory limits set, risking resource contention",
                    vm_name
                ),
                resource: format!("{}/{}", s.namespace, vm_name),
                estimated_savings: None,
                impact: "High".to_string(),
                effort: "Low".to_string(),
            });
        }

        // Check for missing RNG device
        if devices.and_then(|d| d.rng.as_ref()).is_none() {
            id_counter += 1;
            recs.push(Recommendation {
                id: format!("REC-{:04}", id_counter),
                category: "SecurityHardening".to_string(),
                priority: "Medium".to_string(),
                title: "Add RNG device".to_string(),
                description: format!(
                    "VM '{}' lacks a virtio-rng device for secure random number generation",
                    vm_name
                ),
                resource: format!("{}/{}", s.namespace, vm_name),
                estimated_savings: None,
                impact: "Medium".to_string(),
                effort: "Low".to_string(),
            });
        }

        // Check for missing eviction strategy
        if vm.spec.template.spec.eviction_strategy.is_none() {
            id_counter += 1;
            recs.push(Recommendation {
                id: format!("REC-{:04}", id_counter),
                category: "HighAvailability".to_string(),
                priority: "Medium".to_string(),
                title: "Set eviction strategy".to_string(),
                description: format!(
                    "VM '{}' has no eviction strategy; it may be killed during node maintenance",
                    vm_name
                ),
                resource: format!("{}/{}", s.namespace, vm_name),
                estimated_savings: None,
                impact: "High".to_string(),
                effort: "Low".to_string(),
            });
        }
    }

    Json(recs)
}
