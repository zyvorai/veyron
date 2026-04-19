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

/// A network flow observed or inferred from NetworkPolicies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFlow {
    pub source: String,
    pub destination: String,
    pub namespace: String,
    pub protocol: String,
    pub port: u16,
    pub verdict: String,
}

/// Summary of network flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFlowSummary {
    pub total_flows: u32,
    pub allowed: u32,
    pub denied: u32,
    pub flows: Vec<NetworkFlow>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/cilium/status", get(get_cilium_status))
        .route("/cilium/policies", get(list_cilium_policies))
        .route("/cilium/flows", get(list_network_flows))
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

    // Detect Hubble via hubble-relay Deployment
    let mut hubble_enabled = false;
    // Detect ClusterMesh via clustermesh-apiserver Deployment
    let mut cluster_mesh_enabled = false;
    {
        use k8s_openapi::api::apps::v1::Deployment;
        let dep_api: kube::api::Api<Deployment> =
            kube::api::Api::namespaced(client.clone(), cilium_ns);
        if let Ok(deps) = dep_api.list(&kube::api::ListParams::default()).await {
            for dep in &deps.items {
                let name = dep.metadata.name.as_deref().unwrap_or("");
                if name.contains("hubble-relay") || name.contains("hubble-ui") {
                    hubble_enabled = true;
                }
                if name.contains("clustermesh-apiserver") || name.contains("clustermesh") {
                    cluster_mesh_enabled = true;
                }
            }
        }
    }

    // Detect encryption via Cilium ConfigMap key or WireGuard DaemonSet
    let mut encryption_enabled = false;
    {
        use k8s_openapi::api::core::v1::ConfigMap;
        let cm_api: kube::api::Api<ConfigMap> =
            kube::api::Api::namespaced(client.clone(), cilium_ns);
        if let Ok(cms) = cm_api.list(&kube::api::ListParams::default()).await {
            for cm in &cms.items {
                let name = cm.metadata.name.as_deref().unwrap_or("");
                if name == "cilium-config" {
                    if let Some(data) = cm.data.as_ref() {
                        let ipsec = data.get("enable-ipsec").map(|v| v == "true").unwrap_or(false);
                        let wg = data
                            .get("enable-wireguard")
                            .map(|v| v == "true")
                            .unwrap_or(false);
                        if ipsec || wg {
                            encryption_enabled = true;
                        }
                    }
                }
            }
        }
    }

    Json(CiliumStatus {
        version,
        agent_count,
        healthy_agents,
        cluster_mesh_enabled,
        hubble_enabled,
        encryption_enabled,
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

/// Derive network flows from K8s NetworkPolicies.
/// Shows allowed/denied flows based on policy ingress/egress rules.
#[cfg(feature = "web")]
async fn list_network_flows(State(state): State<SharedState>) -> Json<NetworkFlowSummary> {
    use k8s_openapi::api::networking::v1::NetworkPolicy;

    let s = state.read().await;
    let ns = &s.namespace;
    let api: kube::api::Api<NetworkPolicy> =
        kube::api::Api::namespaced(s.client().client(), ns);

    let policies = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(NetworkFlowSummary { total_flows: 0, allowed: 0, denied: 0, flows: vec![] }),
    };

    let mut flows = Vec::new();
    for np in &policies.items {
        let np_name = np.metadata.name.as_deref().unwrap_or("unknown");
        let np_ns = np.metadata.namespace.as_deref().unwrap_or(ns.as_str());
        let spec = match np.spec.as_ref() { Some(s) => s, None => continue };

        // Each ingress rule with ports → allowed ingress flow
        if let Some(ingress_rules) = spec.ingress.as_ref() {
            for rule in ingress_rules {
                let ports = rule.ports.as_deref().unwrap_or(&[]);
                if ports.is_empty() {
                    flows.push(NetworkFlow {
                        source: "any".to_string(),
                        destination: np_name.to_string(),
                        namespace: np_ns.to_string(),
                        protocol: "TCP".to_string(),
                        port: 0,
                        verdict: "allowed".to_string(),
                    });
                }
                for port_spec in ports {
                    let port_num = port_spec.port.as_ref()
                        .and_then(|p| if let k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(n) = p { Some(*n as u16) } else { None })
                        .unwrap_or(0);
                    let proto = port_spec.protocol.as_deref().unwrap_or("TCP").to_string();
                    flows.push(NetworkFlow {
                        source: "policy-allowed".to_string(),
                        destination: np_name.to_string(),
                        namespace: np_ns.to_string(),
                        protocol: proto,
                        port: port_num,
                        verdict: "allowed".to_string(),
                    });
                }
            }
        }

        // Any egress rule → allowed egress flow
        if let Some(egress_rules) = spec.egress.as_ref() {
            for rule in egress_rules {
                let ports = rule.ports.as_deref().unwrap_or(&[]);
                for port_spec in ports {
                    let port_num = port_spec.port.as_ref()
                        .and_then(|p| if let k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(n) = p { Some(*n as u16) } else { None })
                        .unwrap_or(0);
                    let proto = port_spec.protocol.as_deref().unwrap_or("TCP").to_string();
                    flows.push(NetworkFlow {
                        source: np_name.to_string(),
                        destination: "policy-allowed".to_string(),
                        namespace: np_ns.to_string(),
                        protocol: proto,
                        port: port_num,
                        verdict: "allowed".to_string(),
                    });
                }
            }
        }
    }

    let allowed = flows.iter().filter(|f| f.verdict == "allowed").count() as u32;
    let denied = flows.iter().filter(|f| f.verdict == "denied").count() as u32;
    let total = flows.len() as u32;

    Json(NetworkFlowSummary { total_flows: total, allowed, denied, flows })
}
