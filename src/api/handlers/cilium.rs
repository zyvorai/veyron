#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Cilium status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiliumStatus {
    pub version: String,
    pub agent_count: u32,
    pub healthy_agents: u32,
    pub cluster_mesh_enabled: bool,
    pub hubble_enabled: bool,
    pub encryption_enabled: bool,
}

/// Cilium network policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiliumPolicy {
    pub name: String,
    pub namespace: String,
    pub enforcement: String,
    pub endpoint_selector: std::collections::HashMap<String, String>,
    pub ingress_rules: u32,
    pub egress_rules: u32,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/cilium/status", get(get_cilium_status))
        .route("/cilium/policies", get(list_cilium_policies))
        .with_state(state)
}

/// Detect Cilium by looking for cilium-agent DaemonSet.
#[cfg(feature = "web")]
async fn get_cilium_status(State(state): State<SharedState>) -> Json<CiliumStatus> {
    use k8s_openapi::api::apps::v1::DaemonSet;

    let s = state.read().await;
    let client = s.client().client();

    let mut agent_count = 0u32;
    let mut healthy_agents = 0u32;
    let mut version = String::new();

    let cilium_ns = "kube-system";
    let ds_api: kube::api::Api<DaemonSet> = kube::api::Api::namespaced(client.clone(), cilium_ns);
    if let Ok(dss) = ds_api.list(&kube::api::ListParams::default()).await {
        for ds in &dss.items {
            let name = ds.metadata.name.as_deref().unwrap_or("");
            if name.contains("cilium") && !name.contains("operator") {
                if let Some(ref status) = ds.status {
                    agent_count = status.desired_number_scheduled as u32;
                    healthy_agents = status.number_ready as u32;
                }
                version = ds
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("app.kubernetes.io/version").cloned())
                    .unwrap_or_default();
            }
        }
    }

    Json(CiliumStatus {
        version,
        agent_count,
        healthy_agents,
        cluster_mesh_enabled: false,
        hubble_enabled: false,
        encryption_enabled: false,
    })
}

#[cfg(feature = "web")]
async fn list_cilium_policies(State(state): State<SharedState>) -> Json<Vec<CiliumPolicy>> {
    // Cilium policies are CRDs — return K8s NetworkPolicies as a fallback
    use k8s_openapi::api::networking::v1::NetworkPolicy;

    let s = state.read().await;
    let api: kube::api::Api<NetworkPolicy> =
        kube::api::Api::namespaced(s.client().client(), &s.namespace);

    let policies = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<CiliumPolicy> = policies
        .items
        .iter()
        .map(|np| {
            let spec = np.spec.as_ref();
            CiliumPolicy {
                name: np.metadata.name.clone().unwrap_or_default(),
                namespace: np.metadata.namespace.clone().unwrap_or_default(),
                enforcement: "Enabled".to_string(),
                endpoint_selector: spec
                    .and_then(|s| s.pod_selector.match_labels.clone())
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
                ingress_rules: spec
                    .and_then(|s| s.ingress.as_ref())
                    .map(|r| r.len() as u32)
                    .unwrap_or(0),
                egress_rules: spec
                    .and_then(|s| s.egress.as_ref())
                    .map(|r| r.len() as u32)
                    .unwrap_or(0),
            }
        })
        .collect();

    Json(results)
}
