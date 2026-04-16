#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Security posture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPosture {
    pub overall_score: u8,
    pub risk_level: String,
    pub total_findings: u32,
    pub critical_findings: u32,
    pub high_findings: u32,
    pub medium_findings: u32,
    pub low_findings: u32,
}

/// Security finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub severity: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub resource: String,
    pub recommendation: String,
    pub detected_at: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/security/posture", get(get_security_posture))
        .route("/security/findings", get(list_security_findings))
}

#[cfg(feature = "web")]
async fn get_security_posture() -> impl IntoResponse {
    not_implemented("Security posture")
}

#[cfg(feature = "web")]
async fn list_security_findings() -> impl IntoResponse {
    not_implemented("Security findings")
}
