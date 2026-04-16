#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/security/posture", get(get_security_posture))
        .route("/security/findings", get(list_security_findings))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_security_posture(State(state): State<SharedState>) -> Json<SecurityPosture> {
    let s = state.read().await;
    let vms = s.client().list_vms(&s.namespace).await.unwrap_or_default();

    let critical = 0u32;
    let mut high = 0u32;
    let mut medium = 0u32;
    let mut low = 0u32;

    for vm in &vms {
        let domain = &vm.spec.template.spec.domain;
        let devices = domain.devices.as_ref();

        // Check for common security issues
        let has_rng = devices.and_then(|d| d.rng.as_ref()).is_some();
        let has_tpm = devices.and_then(|d| d.tpm.as_ref()).is_some();
        let has_resources = domain.resources.requests.as_ref().map(|m| !m.is_empty()).unwrap_or(false)
            || domain.resources.limits.as_ref().map(|m| !m.is_empty()).unwrap_or(false);

        if !has_rng {
            low += 1;
        }
        if !has_tpm {
            medium += 1;
        }
        if !has_resources {
            high += 1;
        }
    }

    let total = critical + high + medium + low;
    let score = if total == 0 {
        100
    } else {
        let deductions = critical * 20 + high * 10 + medium * 5 + low * 2;
        100u8.saturating_sub(deductions.min(100) as u8)
    };

    let risk_level = match score {
        90..=100 => "Low",
        70..=89 => "Medium",
        50..=69 => "High",
        _ => "Critical",
    }
    .to_string();

    Json(SecurityPosture {
        overall_score: score,
        risk_level,
        total_findings: total,
        critical_findings: critical,
        high_findings: high,
        medium_findings: medium,
        low_findings: low,
    })
}

#[cfg(feature = "web")]
async fn list_security_findings(State(state): State<SharedState>) -> Json<Vec<SecurityFinding>> {
    let s = state.read().await;
    let vms = s.client().list_vms(&s.namespace).await.unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();
    let mut findings = Vec::new();
    let mut id_counter = 0u32;

    for vm in &vms {
        let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let domain = &vm.spec.template.spec.domain;
        let devices = domain.devices.as_ref();

        if devices.and_then(|d| d.rng.as_ref()).is_none() {
            id_counter += 1;
            findings.push(SecurityFinding {
                id: format!("SEC-{:04}", id_counter),
                severity: "Low".to_string(),
                category: "Configuration".to_string(),
                title: "No RNG device configured".to_string(),
                description: format!("VM '{}' does not have a virtio-rng device", vm_name),
                resource: format!("{}/{}", s.namespace, vm_name),
                recommendation: "Add an RNG device for secure random number generation".to_string(),
                detected_at: now.clone(),
            });
        }

        if devices.and_then(|d| d.tpm.as_ref()).is_none() {
            id_counter += 1;
            findings.push(SecurityFinding {
                id: format!("SEC-{:04}", id_counter),
                severity: "Medium".to_string(),
                category: "Configuration".to_string(),
                title: "No TPM device configured".to_string(),
                description: format!("VM '{}' does not have a TPM device", vm_name),
                resource: format!("{}/{}", s.namespace, vm_name),
                recommendation: "Add a TPM device for enhanced security".to_string(),
                detected_at: now.clone(),
            });
        }

        let has_limits = domain.resources.limits.as_ref().map(|m| !m.is_empty()).unwrap_or(false);
        if !has_limits {
            id_counter += 1;
            findings.push(SecurityFinding {
                id: format!("SEC-{:04}", id_counter),
                severity: "High".to_string(),
                category: "Resource".to_string(),
                title: "No resource limits set".to_string(),
                description: format!(
                    "VM '{}' has no resource limits, risking resource exhaustion",
                    vm_name
                ),
                resource: format!("{}/{}", s.namespace, vm_name),
                recommendation: "Set CPU and memory limits to prevent resource exhaustion"
                    .to_string(),
                detected_at: now.clone(),
            });
        }
    }

    Json(findings)
}
