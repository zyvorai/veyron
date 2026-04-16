#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Network interface response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceResponse {
    pub name: String,
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
) -> Json<Vec<NetworkInterfaceResponse>> {
    let s = state.read().await;
    let vmis = s.client().list_vmis(&s.namespace).await.unwrap_or_default();

    let mut results: Vec<NetworkInterfaceResponse> = Vec::new();

    for vmi in &vmis {
        let vm_name = vmi.metadata.name.clone().unwrap_or_default();
        let status = vmi.status.as_ref();
        let phase = status
            .and_then(|s| s.phase.as_deref())
            .unwrap_or("Unknown");

        if let Some(st) = status {
            for iface in &st.interfaces {
                results.push(NetworkInterfaceResponse {
                    name: iface
                        .interface_name
                        .clone()
                        .or_else(|| iface.name.clone())
                        .unwrap_or_default(),
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
) -> Json<Vec<BandwidthResponse>> {
    use crate::kube::types::VirtualMachineInstance;

    let s = state.read().await;
    let vmis_api: kube::api::Api<VirtualMachineInstance> =
        kube::api::Api::namespaced(s.client().client(), &s.namespace);
    let vmis = match vmis_api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list.items,
        Err(_) => return Json(vec![]),
    };

    let now = chrono::Utc::now().to_rfc3339();
    let results: Vec<BandwidthResponse> = vmis
        .iter()
        .filter_map(|vmi| {
            let status = vmi.status.as_ref()?;
            Some(BandwidthResponse {
                interface: status
                    .interfaces
                    .first()
                    .and_then(|i| i.interface_name.clone())
                    .unwrap_or_else(|| "eth0".to_string()),
                rx_bytes_per_sec: 0,
                tx_bytes_per_sec: 0,
                rx_packets_per_sec: 0,
                tx_packets_per_sec: 0,
                timestamp: now.clone(),
            })
        })
        .collect();

    Json(results)
}
