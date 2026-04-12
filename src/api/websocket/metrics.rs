use serde::{Deserialize, Serialize};

/// Metrics stream subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSubscription {
    pub vm_names: Vec<String>,
    pub namespace: String,
    pub metrics: Vec<String>,
    pub interval_seconds: u32,
}

/// Metrics stream message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsStreamMessage {
    pub vm_name: String,
    pub timestamp: String,
    pub cpu_usage: Option<f64>,
    pub memory_usage: Option<f64>,
    pub disk_read_bytes: Option<u64>,
    pub disk_write_bytes: Option<u64>,
    pub network_rx_bytes: Option<u64>,
    pub network_tx_bytes: Option<u64>,
}
