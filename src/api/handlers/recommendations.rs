#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Dashboard passes `?namespace=all` or a specific namespace.
#[derive(Debug, Deserialize)]
pub struct RecommendationQuery {
    pub namespace: Option<String>,
}

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

/// Monthly USD estimate from the same reference rates as `handlers/costs.rs` (CPU + memory only).
#[cfg(feature = "web")]
fn vm_monthly_cost_estimate(vm: &crate::kube::VirtualMachine) -> f64 {
    const CPU_RATE: f64 = 0.048;
    const MEMORY_RATE: f64 = 0.006;
    const HOURS_PER_MONTH: f64 = 730.0;
    let cpu_cores = vm
        .spec
        .template
        .spec
        .domain
        .cpu
        .as_ref()
        .map(|c| c.cores.unwrap_or(1) as f64)
        .unwrap_or(1.0);
    let memory_str = vm
        .spec
        .template
        .spec
        .domain
        .resources
        .requests
        .as_ref()
        .and_then(|r| r.get("memory"))
        .map(|v| v.as_str())
        .unwrap_or("0");
    let memory_gib = crate::utils::parse_memory_gib(memory_str);
    let cpu_cost = cpu_cores * CPU_RATE * HOURS_PER_MONTH;
    let memory_cost = memory_gib * MEMORY_RATE * HOURS_PER_MONTH;
    round2(cpu_cost + memory_cost)
}

#[cfg(feature = "web")]
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Generate recommendations by analyzing live VM configurations.
#[cfg(feature = "web")]
async fn list_recommendations(
    State(state): State<SharedState>,
    Query(query): Query<RecommendationQuery>,
) -> Json<Vec<Recommendation>> {
    let s = state.read().await;
    let scope = query
        .namespace
        .clone()
        .unwrap_or_else(|| s.namespace.clone());
    let vms = if scope == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&scope).await.unwrap_or_default()
    };
    let mut recs = Vec::new();
    let mut id_counter = 0u32;

    for vm in &vms {
        let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let vm_ns = vm
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| scope.clone());
        let domain = &vm.spec.template.spec.domain;
        let devices = domain.devices.as_ref();

        // Check for missing resource limits
        let monthly = vm_monthly_cost_estimate(vm);

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
                resource: format!("{}/{}", vm_ns, vm_name),
                estimated_savings: Some(round2(monthly * 0.12)),
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
                resource: format!("{}/{}", vm_ns, vm_name),
                estimated_savings: Some(round2((monthly * 0.02).max(4.0))),
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
                resource: format!("{}/{}", vm_ns, vm_name),
                estimated_savings: Some(round2(monthly * 0.08)),
                impact: "High".to_string(),
                effort: "Low".to_string(),
            });
        }
    }

    Json(recs)
}
