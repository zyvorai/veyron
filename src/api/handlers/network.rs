#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
pub fn router() -> Router {
    Router::new()
        .route("/network/interfaces", get(list_interfaces))
        .route("/network/bandwidth", get(get_bandwidth))
}

#[cfg(feature = "web")]
async fn list_interfaces() -> Json<Vec<NetworkInterfaceResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn get_bandwidth() -> Json<Vec<BandwidthResponse>> {
    Json(vec![])
}
