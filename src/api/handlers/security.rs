#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
async fn get_security_posture() -> Json<SecurityPosture> {
    Json(SecurityPosture {
        overall_score: 0,
        risk_level: "unknown".to_string(),
        total_findings: 0,
        critical_findings: 0,
        high_findings: 0,
        medium_findings: 0,
        low_findings: 0,
    })
}

#[cfg(feature = "web")]
async fn list_security_findings() -> Json<Vec<SecurityFinding>> {
    Json(vec![])
}
