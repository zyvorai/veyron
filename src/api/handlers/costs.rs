#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Cost entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub vm_name: String,
    pub namespace: String,
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub storage_cost: f64,
    pub network_cost: f64,
    pub total_cost: f64,
    pub currency: String,
    pub period: String,
}

/// Cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_cost: f64,
    pub currency: String,
    pub period: String,
    pub by_namespace: std::collections::HashMap<String, f64>,
    pub by_resource_type: std::collections::HashMap<String, f64>,
}

/// Cost forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostForecast {
    pub current_monthly: f64,
    pub projected_monthly: f64,
    pub trend: String,
    pub currency: String,
}

/// Budget entry for cost anomaly detection & budget alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub name: String,
    pub namespace: String,
    pub monthly_limit: f64,
    pub current_spend: f64,
    pub alert_threshold_percent: f64,
    pub status: String,
}

/// Request body for creating a budget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBudgetRequest {
    pub name: String,
    pub namespace: String,
    pub monthly_limit: f64,
    pub alert_threshold_percent: Option<f64>,
}

const CPU_RATE: f64 = 0.048;
const MEMORY_RATE: f64 = 0.006;
const STORAGE_RATE: f64 = 0.0001;
/// Flat network egress cost per VM per month (GiB assumed egress × $/GiB)
const NETWORK_RATE_PER_VM: f64 = 0.50;
const HOURS_PER_MONTH: f64 = 730.0;

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/costs", get(list_costs))
        .route("/costs/summary", get(get_cost_summary))
        .route("/costs/forecast", get(get_cost_forecast))
        .route("/costs/budgets", get(list_budgets).post(create_budget))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_costs(State(state): State<SharedState>) -> Json<Vec<CostEntry>> {
    let s = state.read().await;
    let vms = s.client().list_all_vms().await.unwrap_or_default();

    let results: Vec<CostEntry> = vms
        .iter()
        .map(|vm| {
            let meta = &vm.metadata;
            let name = meta.name.clone().unwrap_or_default();
            let namespace = meta.namespace.clone().unwrap_or_default();

            let cpu_cores = vm.spec.template.spec.domain.cpu.as_ref()
                .map(|c| c.cores.unwrap_or(1) as f64)
                .unwrap_or(1.0);

            let memory_str = vm.spec.template.spec.domain.resources.requests.as_ref()
                .and_then(|r| r.get("memory"))
                .map(|v| v.as_str())
                .unwrap_or("0");

            let memory_gib = parse_memory_gib(memory_str);

            let cpu_cost = cpu_cores * CPU_RATE * HOURS_PER_MONTH;
            let memory_cost = memory_gib * MEMORY_RATE * HOURS_PER_MONTH;
            // Sum actual disk sizes from VM spec
            let total_disk_gib: f64 = vm.spec.template.spec.domain.resources.requests.as_ref()
                .and_then(|r| r.get("storage"))
                .map(|v| parse_memory_gib(v.as_str()))
                .unwrap_or(20.0); // default 20Gi if no storage in requests
            let storage_cost = total_disk_gib * STORAGE_RATE * HOURS_PER_MONTH;
            // Flat per-VM network egress estimate; refined when live metrics are available
            let network_cost = NETWORK_RATE_PER_VM;
            let total_cost = cpu_cost + memory_cost + storage_cost + network_cost;

            CostEntry {
                vm_name: name,
                namespace,
                cpu_cost: round2(cpu_cost),
                memory_cost: round2(memory_cost),
                storage_cost: round2(storage_cost),
                network_cost: round2(network_cost),
                total_cost: round2(total_cost),
                currency: "USD".to_string(),
                period: "monthly".to_string(),
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn get_cost_summary(State(state): State<SharedState>) -> Json<CostSummary> {
    let costs = list_costs(State(state)).await.0;

    let total: f64 = costs.iter().map(|c| c.total_cost).sum();
    let mut by_namespace: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut cpu_total = 0.0;
    let mut mem_total = 0.0;
    let mut storage_total = 0.0;
    let mut network_total = 0.0;

    for c in &costs {
        *by_namespace.entry(c.namespace.clone()).or_default() += c.total_cost;
        cpu_total += c.cpu_cost;
        mem_total += c.memory_cost;
        storage_total += c.storage_cost;
        network_total += c.network_cost;
    }

    let mut by_resource_type = std::collections::HashMap::new();
    by_resource_type.insert("cpu".to_string(), round2(cpu_total));
    by_resource_type.insert("memory".to_string(), round2(mem_total));
    by_resource_type.insert("storage".to_string(), round2(storage_total));
    by_resource_type.insert("network".to_string(), round2(network_total));

    Json(CostSummary {
        total_cost: round2(total),
        currency: "USD".to_string(),
        period: "monthly".to_string(),
        by_namespace,
        by_resource_type,
    })
}

#[cfg(feature = "web")]
async fn get_cost_forecast(State(state): State<SharedState>) -> Json<CostForecast> {
    let costs = list_costs(State(state)).await.0;
    let current: f64 = costs.iter().map(|c| c.total_cost).sum();

    Json(CostForecast {
        current_monthly: round2(current),
        projected_monthly: round2(current * 1.05),
        trend: if current > 0.0 { "growing" } else { "stable" }.to_string(),
        currency: "USD".to_string(),
    })
}

#[cfg(feature = "web")]
async fn list_budgets(State(state): State<SharedState>) -> Json<Vec<Budget>> {
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::{Api, ListParams};

    let s = state.read().await;
    let client = s.client().client();

    // List ConfigMaps with label vmrogue.io/type=budget in vmrogue-system namespace
    let cms: Api<ConfigMap> = Api::namespaced(client.clone(), "vmrogue-system");
    let lp = ListParams::default().labels("vmrogue.io/type=budget");
    let cm_list = match cms.list(&lp).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    // Get current costs for comparison
    let vms = s.client().list_all_vms().await.unwrap_or_default();

    // Build namespace -> total cost mapping
    let mut namespace_costs: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();
    for vm in &vms {
        let ns = vm
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());
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
        let memory_gib = parse_memory_gib(memory_str);
        let cpu_cost = cpu_cores * CPU_RATE * HOURS_PER_MONTH;
        let memory_cost = memory_gib * MEMORY_RATE * HOURS_PER_MONTH;
        let storage_cost = 20.0 * STORAGE_RATE * HOURS_PER_MONTH;
        let total = cpu_cost + memory_cost + storage_cost;
        *namespace_costs.entry(ns).or_default() += total;
    }

    let budgets: Vec<Budget> = cm_list
        .items
        .iter()
        .filter_map(|cm| {
            let data = cm.data.as_ref()?;
            let name = data.get("name")?.clone();
            let namespace = data.get("namespace")?.clone();
            let monthly_limit: f64 = data.get("monthly_limit")?.parse().ok()?;
            let alert_threshold_percent: f64 = data
                .get("alert_threshold_percent")
                .and_then(|v| v.parse().ok())
                .unwrap_or(80.0);

            let current_spend = namespace_costs.get(&namespace).copied().unwrap_or(0.0);
            let utilization = if monthly_limit > 0.0 {
                (current_spend / monthly_limit) * 100.0
            } else {
                0.0
            };

            let status = if utilization >= 100.0 {
                "exceeded".to_string()
            } else if utilization >= alert_threshold_percent {
                "warning".to_string()
            } else {
                "healthy".to_string()
            };

            Some(Budget {
                name,
                namespace,
                monthly_limit: round2(monthly_limit),
                current_spend: round2(current_spend),
                alert_threshold_percent,
                status,
            })
        })
        .collect();

    Json(budgets)
}

#[cfg(feature = "web")]
async fn create_budget(
    State(state): State<SharedState>,
    Json(req): Json<CreateBudgetRequest>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::{Api, PostParams};
    use std::collections::BTreeMap;

    let s = state.read().await;
    let client = s.client().client();

    let alert_threshold = req.alert_threshold_percent.unwrap_or(80.0);

    // Ensure vmrogue-system namespace exists
    {
        use k8s_openapi::api::core::v1::Namespace;
        let ns_api: kube::Api<Namespace> = kube::Api::all(client.clone());
        if ns_api.get("vmrogue-system").await.is_err() {
            let ns = Namespace {
                metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                    name: Some("vmrogue-system".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            };
            let _ = ns_api.create(&kube::api::PostParams::default(), &ns).await;
        }
    }

    // Build ConfigMap data
    let mut data = BTreeMap::new();
    data.insert("name".to_string(), req.name.clone());
    data.insert("namespace".to_string(), req.namespace.clone());
    data.insert("monthly_limit".to_string(), req.monthly_limit.to_string());
    data.insert(
        "alert_threshold_percent".to_string(),
        alert_threshold.to_string(),
    );

    // Build labels
    let mut labels = BTreeMap::new();
    labels.insert("vmrogue.io/type".to_string(), "budget".to_string());
    labels.insert("vmrogue.io/budget-name".to_string(), req.name.clone());
    labels.insert(
        "vmrogue.io/budget-namespace".to_string(),
        req.namespace.clone(),
    );

    let cm_name = format!("vmrogue-budget-{}", req.name);

    let cm = ConfigMap {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(cm_name.clone()),
            namespace: Some("vmrogue-system".to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };

    let cms: Api<ConfigMap> = Api::namespaced(client, "vmrogue-system");
    match cms.create(&PostParams::default(), &cm).await {
        Ok(_) => (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({
                "status": "created",
                "name": req.name,
                "namespace": req.namespace,
                "monthly_limit": req.monthly_limit,
                "alert_threshold_percent": alert_threshold,
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "BUDGET_CREATE_FAILED",
                "message": format!("Failed to create budget ConfigMap: {}", e),
            })),
        ),
    }
}

fn parse_memory_gib(s: &str) -> f64 {
    crate::utils::parse_memory_gib(s)
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
