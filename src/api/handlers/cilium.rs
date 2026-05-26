// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use kube::{
    Api,
    api::{ApiResource, DynamicObject, GroupVersionKind, ListParams},
};

/// Cilium status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiliumStatus {
    pub version: String,
    pub agent_count: u32,
    pub healthy_agents: u32,
    pub cluster_mesh_enabled: bool,
    pub hubble_enabled: bool,
    pub encryption_enabled: bool,
    /// Namespace where the main Cilium agent DaemonSet was detected (empty if none).
    pub installation_namespace: String,
}

/// Cilium or Kubernetes network policy row for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiliumPolicy {
    pub name: String,
    pub namespace: String,
    pub enforcement: String,
    pub endpoint_selector: HashMap<String, String>,
    pub ingress_rules: u32,
    pub egress_rules: u32,
    /// `CNP`, `CCNP`, or `NetworkPolicy`.
    pub policy_kind: String,
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
    /// `kubernetes_network_policies` (derived from NP rules) or `none`.
    pub flow_source: String,
}

#[derive(Debug, Deserialize)]
pub struct CiliumQuery {
    /// `all` or a specific namespace (defaults to API server namespace).
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/cilium/status", get(get_cilium_status))
        .route("/cilium/policies", get(list_cilium_policies))
        .route("/cilium/flows", get(list_network_flows))
        .with_state(state)
}

#[cfg(feature = "web")]
fn cilium_namespace_priority(ns: Option<&str>) -> u8 {
    match ns.unwrap_or("") {
        "cilium-system" => 3,
        "kube-system" => 2,
        _ => 1,
    }
}

#[cfg(feature = "web")]
fn is_main_cilium_agent_ds(name: &str) -> bool {
    let n = name.to_lowercase();
    if !n.contains("cilium") {
        return false;
    }
    if n.contains("operator")
        || n.contains("node-init")
        || n.contains("preflight")
        || n.contains("envoy")
    {
        return false;
    }
    true
}

/// Detect Cilium across common install locations (kube-system, cilium-system, cluster-wide).
#[cfg(feature = "web")]
async fn get_cilium_status(State(state): State<SharedState>) -> Json<CiliumStatus> {
    use k8s_openapi::api::apps::v1::{DaemonSet, Deployment};
    use k8s_openapi::api::core::v1::ConfigMap;

    let s = state.read().await;
    let client = s.client().client();

    let mut best_ds: Option<DaemonSet> = None;
    let mut best_pri: u8 = 0;
    let mut best_desired: i32 = -1;

    let ds_all: Api<DaemonSet> = Api::all(client.clone());
    if let Ok(list) = ds_all.list(&ListParams::default()).await {
        for ds in list.items {
            let name = ds.metadata.name.as_deref().unwrap_or("");
            if !is_main_cilium_agent_ds(name) {
                continue;
            }
            let pri = cilium_namespace_priority(ds.metadata.namespace.as_deref());
            let desired = ds
                .status
                .as_ref()
                .map(|st| st.desired_number_scheduled)
                .unwrap_or(0);
            if pri > best_pri || (pri == best_pri && desired > best_desired) {
                best_pri = pri;
                best_desired = desired;
                best_ds = Some(ds);
            }
        }
    }

    let mut agent_count = 0u32;
    let mut healthy_agents = 0u32;
    let mut version = String::new();
    let mut installation_namespace = String::new();

    if let Some(ref ds) = best_ds {
        installation_namespace = ds.metadata.namespace.clone().unwrap_or_default();
        if let Some(ref status) = ds.status {
            agent_count = status.desired_number_scheduled.max(0) as u32;
            healthy_agents = status.number_ready.max(0) as u32;
        }
        version = ds
            .metadata
            .labels
            .as_ref()
            .and_then(|l| l.get("app.kubernetes.io/version").cloned())
            .unwrap_or_default();
    }

    let dep_all: Api<Deployment> = Api::all(client.clone());
    let mut hubble_enabled = false;
    let mut cluster_mesh_enabled = false;
    if let Ok(deps) = dep_all.list(&ListParams::default()).await {
        for dep in &deps.items {
            let name = dep.metadata.name.as_deref().unwrap_or("").to_lowercase();
            if name.contains("hubble-relay") || name.contains("hubble-ui") {
                hubble_enabled = true;
            }
            if name.contains("clustermesh-apiserver") || name.contains("clustermesh") {
                cluster_mesh_enabled = true;
            }
        }
    }

    let mut encryption_enabled = false;
    for ns in ["cilium-system", "kube-system"] {
        let cm_api: Api<ConfigMap> = Api::namespaced(client.clone(), ns);
        if let Ok(cms) = cm_api.list(&ListParams::default()).await {
            for cm in &cms.items {
                if cm.metadata.name.as_deref() != Some("cilium-config") {
                    continue;
                }
                if let Some(data) = cm.data.as_ref() {
                    if cm_encryption_enabled(data) {
                        encryption_enabled = true;
                        break;
                    }
                }
            }
        }
        if encryption_enabled {
            break;
        }
    }

    Json(CiliumStatus {
        version,
        agent_count,
        healthy_agents,
        cluster_mesh_enabled,
        hubble_enabled,
        encryption_enabled,
        installation_namespace,
    })
}

#[cfg(feature = "web")]
fn cm_encryption_enabled(data: &std::collections::BTreeMap<String, String>) -> bool {
    let keys = [
        "enable-wireguard",
        "enable-ipsec",
        "encryption-enabled",
        "encryption.enabled",
    ];
    keys.iter().any(|k| {
        data.get(*k)
            .map(|v| v.eq_ignore_ascii_case("true") || v == "enabled")
            .unwrap_or(false)
    })
}

#[cfg(feature = "web")]
fn resolve_cilium_namespace(query: &CiliumQuery, default_ns: &str) -> String {
    query
        .namespace
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| default_ns.to_string())
}

#[cfg(feature = "web")]
fn cilium_network_policy_api_resource() -> ApiResource {
    let gvk = GroupVersionKind::gvk("cilium.io", "v2", "CiliumNetworkPolicy");
    let mut resource = ApiResource::from_gvk(&gvk);
    resource.plural = "ciliumnetworkpolicies".to_string();
    resource.api_version = "cilium.io/v2".to_string();
    resource
}

#[cfg(feature = "web")]
fn cilium_clusterwide_policy_api_resource() -> ApiResource {
    let gvk = GroupVersionKind::gvk("cilium.io", "v2", "CiliumClusterwideNetworkPolicy");
    let mut resource = ApiResource::from_gvk(&gvk);
    resource.plural = "ciliumclusterwidenetworkpolicies".to_string();
    resource.api_version = "cilium.io/v2".to_string();
    resource
}

#[cfg(feature = "web")]
fn dynamic_policy_to_row(obj: &DynamicObject, kind_label: &str) -> CiliumPolicy {
    let name = obj.metadata.name.clone().unwrap_or_default();
    let namespace = obj
        .metadata
        .namespace
        .clone()
        .unwrap_or_else(|| "cluster".to_string());
    let spec = obj.data.get("spec").cloned().unwrap_or(Value::Null);
    let endpoint_selector = spec
        .get("endpointSelector")
        .and_then(|es| es.get("matchLabels"))
        .and_then(|m| m.as_object())
        .map(|o| {
            o.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let ingress_rules = spec
        .get("ingress")
        .and_then(|v| v.as_array())
        .map(|a| a.len() as u32)
        .unwrap_or(0);
    let egress_rules = spec
        .get("egress")
        .and_then(|v| v.as_array())
        .map(|a| a.len() as u32)
        .unwrap_or(0);
    CiliumPolicy {
        name,
        namespace,
        enforcement: "Enabled".to_string(),
        endpoint_selector,
        ingress_rules,
        egress_rules,
        policy_kind: kind_label.to_string(),
    }
}

/// List CiliumNetworkPolicy / CCNP when installed, plus Kubernetes NetworkPolicies (same namespace scope as the dashboard).
#[cfg(feature = "web")]
async fn list_cilium_policies(
    State(state): State<SharedState>,
    Query(query): Query<CiliumQuery>,
) -> Json<Vec<CiliumPolicy>> {
    use k8s_openapi::api::networking::v1::NetworkPolicy;

    let s = state.read().await;
    let client = s.client().client();
    let scope = resolve_cilium_namespace(&query, &s.namespace);

    let mut results: Vec<CiliumPolicy> = Vec::new();

    let cnp_res = cilium_network_policy_api_resource();
    let list_cnps = if scope == "all" {
        let api: Api<DynamicObject> = Api::all_with(client.clone(), &cnp_res);
        api.list(&ListParams::default()).await
    } else {
        let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), &scope, &cnp_res);
        api.list(&ListParams::default()).await
    };

    if let Ok(list) = list_cnps {
        for item in list.items {
            results.push(dynamic_policy_to_row(&item, "CNP"));
        }
    }

    // Cluster-wide Cilium policies (always relevant; small list).
    let ccnp_res = cilium_clusterwide_policy_api_resource();
    let api_ccnp: Api<DynamicObject> = Api::all_with(client.clone(), &ccnp_res);
    if let Ok(list) = api_ccnp.list(&ListParams::default()).await {
        for item in list.items {
            results.push(dynamic_policy_to_row(&item, "CCNP"));
        }
    }

    let nps: Vec<NetworkPolicy> = if scope == "all" {
        let api: Api<NetworkPolicy> = Api::all(client.clone());
        api.list(&ListParams::default())
            .await
            .map(|l| l.items)
            .unwrap_or_default()
    } else {
        let api: Api<NetworkPolicy> = Api::namespaced(client.clone(), &scope);
        api.list(&ListParams::default())
            .await
            .map(|l| l.items)
            .unwrap_or_default()
    };

    for np in nps {
        let spec = match np.spec.as_ref() {
            Some(sp) => sp,
            None => continue,
        };
        results.push(CiliumPolicy {
            name: np.metadata.name.clone().unwrap_or_default(),
            namespace: np.metadata.namespace.clone().unwrap_or_default(),
            enforcement: "Enabled".to_string(),
            endpoint_selector: spec
                .pod_selector
                .match_labels
                .clone()
                .unwrap_or_default()
                .into_iter()
                .collect(),
            ingress_rules: spec.ingress.as_ref().map(|r| r.len() as u32).unwrap_or(0),
            egress_rules: spec.egress.as_ref().map(|r| r.len() as u32).unwrap_or(0),
            policy_kind: "NetworkPolicy".to_string(),
        });
    }

    Json(results)
}

const MAX_POLICY_DERIVED_FLOWS: usize = 400;

/// Derive illustrative flows from Kubernetes NetworkPolicies (same scope as policies API).
#[cfg(feature = "web")]
async fn list_network_flows(
    State(state): State<SharedState>,
    Query(query): Query<CiliumQuery>,
) -> Json<NetworkFlowSummary> {
    use k8s_openapi::api::networking::v1::NetworkPolicy;

    let s = state.read().await;
    let client = s.client().client();
    let scope = resolve_cilium_namespace(&query, &s.namespace);

    let policies: Vec<NetworkPolicy> = if scope == "all" {
        let api: Api<NetworkPolicy> = Api::all(client.clone());
        api.list(&ListParams::default())
            .await
            .map(|l| l.items)
            .unwrap_or_default()
    } else {
        let api: Api<NetworkPolicy> = Api::namespaced(client.clone(), &scope);
        api.list(&ListParams::default())
            .await
            .map(|l| l.items)
            .unwrap_or_default()
    };

    let mut flows = Vec::new();
    for np in &policies {
        let np_name = np.metadata.name.as_deref().unwrap_or("unknown");
        let np_ns = np.metadata.namespace.as_deref().unwrap_or("default");
        let spec = match np.spec.as_ref() {
            Some(sp) => sp,
            None => continue,
        };

        if let Some(ingress_rules) = spec.ingress.as_ref() {
            for rule in ingress_rules {
                let ports = rule.ports.as_deref().unwrap_or(&[]);
                if ports.is_empty() {
                    if flows.len() >= MAX_POLICY_DERIVED_FLOWS {
                        break;
                    }
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
                    if flows.len() >= MAX_POLICY_DERIVED_FLOWS {
                        break;
                    }
                    let port_num = port_spec
                        .port
                        .as_ref()
                        .and_then(|p| {
                            if let k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(
                                n,
                            ) = p
                            {
                                Some(*n as u16)
                            } else {
                                None
                            }
                        })
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

        if let Some(egress_rules) = spec.egress.as_ref() {
            for rule in egress_rules {
                let ports = rule.ports.as_deref().unwrap_or(&[]);
                for port_spec in ports {
                    if flows.len() >= MAX_POLICY_DERIVED_FLOWS {
                        break;
                    }
                    let port_num = port_spec
                        .port
                        .as_ref()
                        .and_then(|p| {
                            if let k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(
                                n,
                            ) = p
                            {
                                Some(*n as u16)
                            } else {
                                None
                            }
                        })
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

    let had_policies = !policies.is_empty();
    let allowed = flows.iter().filter(|f| f.verdict == "allowed").count() as u32;
    let denied = flows.iter().filter(|f| f.verdict == "denied").count() as u32;
    let total = flows.len() as u32;
    let flow_source = if total > 0 || had_policies {
        "kubernetes_network_policies"
    } else {
        "none"
    }
    .to_string();

    Json(NetworkFlowSummary {
        total_flows: total,
        allowed,
        denied,
        flows,
        flow_source,
    })
}
