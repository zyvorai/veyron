#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub service: String,
    pub uptime_seconds: u64,
}

/// Readiness/liveness probe response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResponse {
    pub ready: bool,
    pub checks: Vec<HealthCheck>,
}

/// Individual health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
        .route("/health/live", get(liveness_check))
}

#[cfg(feature = "web")]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "vmrogue-api".to_string(),
        uptime_seconds: 0,
    })
}

#[cfg(feature = "web")]
async fn readiness_check() -> Json<ProbeResponse> {
    Json(ProbeResponse {
        ready: true,
        checks: vec![],
    })
}

#[cfg(feature = "web")]
async fn liveness_check() -> Json<ProbeResponse> {
    Json(ProbeResponse {
        ready: true,
        checks: vec![],
    })
}
