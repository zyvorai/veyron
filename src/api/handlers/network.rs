#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

/// Network interface response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceResponse {
    pub name: String,
    pub vm_namespace: String,
    pub vm_name: String,
    pub mac_address: String,
    pub ip_address: Option<String>,
    pub interface_type: String,
    pub network_name: String,
    pub status: String,
}

/// Bandwidth usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthResponse {
    pub interface: String,
    pub vm_namespace: String,
    pub vm_name: String,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub rx_packets_per_sec: u64,
    pub tx_packets_per_sec: u64,
    pub timestamp: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/network/interfaces", get(list_interfaces))
        .route("/network/bandwidth", get(get_bandwidth))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_interfaces(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<NetworkInterfaceResponse>> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let vmis = s.client().list_vmis_for_scope(&scope).await;

    let mut results: Vec<NetworkInterfaceResponse> = Vec::new();

    for vmi in &vmis {
        let vm_name = vmi.metadata.name.clone().unwrap_or_default();
        let vm_namespace = vmi
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| scope.clone());
        let status = vmi.status.as_ref();
        let phase = status.and_then(|s| s.phase.as_deref()).unwrap_or("Unknown");

        if let Some(st) = status {
            for iface in &st.interfaces {
                results.push(NetworkInterfaceResponse {
                    name: iface
                        .interface_name
                        .clone()
                        .or_else(|| iface.name.clone())
                        .unwrap_or_default(),
                    vm_namespace: vm_namespace.clone(),
                    vm_name: vm_name.clone(),
                    mac_address: iface.mac.clone().unwrap_or_default(),
                    ip_address: iface.ip_address.clone(),
                    interface_type: "virtio".to_string(),
                    network_name: iface.name.clone().unwrap_or_else(|| "default".to_string()),
                    status: phase.to_string(),
                });
            }
        }
    }

    Json(results)
}

#[cfg(feature = "web")]
async fn get_bandwidth(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<BandwidthResponse>> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let vmis = s.client().list_vmis_for_scope(&scope).await;

    let now = chrono::Utc::now().to_rfc3339();
    let mut results: Vec<BandwidthResponse> = Vec::new();

    for vmi in &vmis {
        let status = match vmi.status.as_ref() {
            Some(s) => s,
            None => continue,
        };
        let vm_name = vmi.metadata.name.as_deref().unwrap_or("unknown");
        let vm_namespace = vmi.metadata.namespace.as_deref().unwrap_or(scope.as_str());
        let iface_name = status
            .interfaces
            .first()
            .and_then(|i| i.interface_name.clone())
            .unwrap_or_else(|| "eth0".to_string());

        let collector = crate::monitoring::metrics::MetricsCollector::new(vm_namespace.to_string());
        let (rx, tx, rx_pkt, tx_pkt) = if let Ok(m) = collector.collect(vm_name).await {
            (
                m.network.rx_bytes_per_sec,
                m.network.tx_bytes_per_sec,
                m.network.rx_packets_per_sec,
                m.network.tx_packets_per_sec,
            )
        } else {
            (0, 0, 0, 0)
        };

        results.push(BandwidthResponse {
            interface: iface_name,
            vm_namespace: vm_namespace.to_string(),
            vm_name: vm_name.to_string(),
            rx_bytes_per_sec: rx,
            tx_bytes_per_sec: tx,
            rx_packets_per_sec: rx_pkt,
            tx_packets_per_sec: tx_pkt,
            timestamp: now.clone(),
        });
    }

    Json(results)
}
