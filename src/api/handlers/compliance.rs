#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Compliance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub framework: String,
    pub compliant: bool,
    pub score: u8,
    pub total_controls: u32,
    pub passing_controls: u32,
    pub failing_controls: u32,
    pub last_checked: String,
}

/// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub id: String,
    pub framework: String,
    pub generated_at: String,
    pub summary: ComplianceStatus,
    pub findings: Vec<ComplianceFinding>,
}

/// Compliance finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub control_id: String,
    pub title: String,
    pub status: String,
    pub severity: String,
    pub description: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/compliance/status", get(get_compliance_status))
        .route("/compliance/reports", get(list_compliance_reports))
}

#[cfg(feature = "web")]
async fn get_compliance_status() -> Json<Vec<ComplianceStatus>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn list_compliance_reports() -> Json<Vec<ComplianceReport>> {
    Json(vec![])
}
