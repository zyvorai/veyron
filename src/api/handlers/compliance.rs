#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/compliance/status", get(get_compliance_status))
        .route("/compliance/reports", get(list_compliance_reports))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_compliance_status(
    State(state): State<SharedState>,
) -> Json<Vec<ComplianceStatus>> {
    let s = state.read().await;
    let vms = s.client().list_vms(&s.namespace).await.unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();

    // Run basic compliance checks across all VMs
    let mut resource_limits = 0u32;
    let mut eviction_strategy = 0u32;
    let mut rng_device = 0u32;
    let total_vms = vms.len() as u32;

    for vm in &vms {
        let vmi_spec = &vm.spec.template.spec;
        let domain = &vmi_spec.domain;

        if domain.resources.limits.as_ref().map(|m| !m.is_empty()).unwrap_or(false) {
            resource_limits += 1;
        }
        if vmi_spec.eviction_strategy.is_some() {
            eviction_strategy += 1;
        }
        if domain
            .devices
            .as_ref()
            .and_then(|d| d.rng.as_ref())
            .is_some()
        {
            rng_device += 1;
        }
    }

    let total_controls = 3u32;

    let check = |passing: u32| -> ComplianceStatus {
        let failing = total_vms.saturating_sub(passing);
        let score = if total_vms == 0 {
            100
        } else {
            ((passing as f64 / total_vms as f64) * 100.0) as u8
        };
        ComplianceStatus {
            framework: String::new(),
            compliant: failing == 0,
            score,
            total_controls: total_vms,
            passing_controls: passing,
            failing_controls: failing,
            last_checked: now.clone(),
        }
    };

    // Provide a summary per "framework" style grouping
    let mut cis = check(resource_limits.min(eviction_strategy).min(rng_device));
    cis.framework = "CIS Benchmark".to_string();
    let passing_all = [resource_limits, eviction_strategy, rng_device]
        .iter()
        .sum::<u32>();
    cis.total_controls = total_controls * total_vms;
    cis.passing_controls = passing_all;
    cis.failing_controls = cis.total_controls.saturating_sub(passing_all);
    cis.score = if cis.total_controls == 0 {
        100
    } else {
        ((passing_all as f64 / (total_controls * total_vms) as f64) * 100.0) as u8
    };
    cis.compliant = cis.failing_controls == 0;

    Json(vec![cis])
}

#[cfg(feature = "web")]
async fn list_compliance_reports(
    State(state): State<SharedState>,
) -> Json<Vec<ComplianceReport>> {
    let statuses = get_compliance_status(State(state)).await.0;
    let now = chrono::Utc::now().to_rfc3339();

    let reports: Vec<ComplianceReport> = statuses
        .into_iter()
        .map(|status| ComplianceReport {
            id: format!("report-{}", status.framework.to_lowercase().replace(' ', "-")),
            framework: status.framework.clone(),
            generated_at: now.clone(),
            summary: status,
            findings: Vec::new(),
        })
        .collect();

    Json(reports)
}
