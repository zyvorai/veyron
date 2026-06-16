// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
use super::feature_context::VmrogueFeatureContext;
#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

/// Security posture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPosture {
    pub veyron_context: VmrogueFeatureContext,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFindingsResponse {
    pub veyron_context: VmrogueFeatureContext,
    pub findings: Vec<SecurityFinding>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/security/posture", get(get_security_posture))
        .route("/security/findings", get(list_security_findings))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_security_posture(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<SecurityPosture> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let vms = s.client().list_vms_for_scope(&scope).await;

    let mut critical = 0u32;
    let mut high = 0u32;
    let mut medium = 0u32;
    let mut low = 0u32;

    for vm in &vms {
        let vmi_spec = &vm.spec.template.spec;
        let domain = &vmi_spec.domain;
        let devices = domain.devices.as_ref();

        // Critical: privileged access via host networking, hostPID, or hostIPC
        let uses_host_network = vmi_spec
            .networks
            .as_ref()
            .map(|nets| nets.iter().any(|n| n.name == "host"))
            .unwrap_or(false);
        if uses_host_network {
            critical += 1;
        }

        // High: no resource limits (resource exhaustion risk)
        let has_resources = domain
            .resources
            .requests
            .as_ref()
            .map(|m| !m.is_empty())
            .unwrap_or(false)
            || domain
                .resources
                .limits
                .as_ref()
                .map(|m| !m.is_empty())
                .unwrap_or(false);
        if !has_resources {
            high += 1;
        }

        // Medium: no TPM device
        let has_tpm = devices.and_then(|d| d.tpm.as_ref()).is_some();
        if !has_tpm {
            medium += 1;
        }

        // Low: no RNG device
        let has_rng = devices.and_then(|d| d.rng.as_ref()).is_some();
        if !has_rng {
            low += 1;
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

    let trivy_enabled = std::env::var("VMROGUE_TRIVY_URL")
        .map(|u| !u.trim().is_empty())
        .unwrap_or(false);

    Json(SecurityPosture {
        veyron_context: if trivy_enabled {
            VmrogueFeatureContext::security_trivy()
        } else {
            VmrogueFeatureContext::security_config()
        },
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
async fn list_security_findings(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<SecurityFindingsResponse> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let vms = s.client().list_vms_for_scope(&scope).await;
    let now = chrono::Utc::now().to_rfc3339();
    let mut findings = Vec::new();
    let mut id_counter = 0u32;

    for vm in &vms {
        let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let vm_ns = vm.metadata.namespace.as_deref().unwrap_or(scope.as_str());
        let vmi_spec = &vm.spec.template.spec;
        let domain = &vmi_spec.domain;
        let devices = domain.devices.as_ref();

        // Critical: host network access
        let uses_host_network = vmi_spec
            .networks
            .as_ref()
            .map(|nets| nets.iter().any(|n| n.name == "host"))
            .unwrap_or(false);
        if uses_host_network {
            id_counter += 1;
            findings.push(SecurityFinding {
                id: format!("SEC-{:04}", id_counter),
                severity: "Critical".to_string(),
                category: "Network".to_string(),
                title: "Host network access enabled".to_string(),
                description: format!(
                    "VM '{}' uses the host network, bypassing network isolation",
                    vm_name
                ),
                resource: format!("{}/{}", vm_ns, vm_name),
                recommendation:
                    "Remove host network binding and use a dedicated VM network interface"
                        .to_string(),
                detected_at: now.clone(),
            });
        }

        if devices.and_then(|d| d.rng.as_ref()).is_none() {
            id_counter += 1;
            findings.push(SecurityFinding {
                id: format!("SEC-{:04}", id_counter),
                severity: "Low".to_string(),
                category: "Configuration".to_string(),
                title: "No RNG device configured".to_string(),
                description: format!("VM '{}' does not have a virtio-rng device", vm_name),
                resource: format!("{}/{}", vm_ns, vm_name),
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
                resource: format!("{}/{}", vm_ns, vm_name),
                recommendation: "Add a TPM device for enhanced security".to_string(),
                detected_at: now.clone(),
            });
        }

        let has_limits = domain
            .resources
            .limits
            .as_ref()
            .map(|m| !m.is_empty())
            .unwrap_or(false);
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
                resource: format!("{}/{}", vm_ns, vm_name),
                recommendation: "Set CPU and memory limits to prevent resource exhaustion"
                    .to_string(),
                detected_at: now.clone(),
            });
        }
    }

    let trivy_url = std::env::var("VMROGUE_TRIVY_URL")
        .ok()
        .filter(|u| !u.trim().is_empty());
    let mut used_trivy = false;
    if let Some(ref base) = trivy_url {
        for vm in &vms {
            let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
            let vm_ns = vm.metadata.namespace.as_deref().unwrap_or(scope.as_str());
            let scan = crate::security::scan::VulnerabilityScanner::scan(
                &crate::security::scan::ScanConfig::new(
                    vm_name,
                    crate::security::scan::ScanType::Quick,
                ),
            );
            for vuln in &scan.vulnerabilities {
                id_counter += 1;
                used_trivy = true;
                findings.push(SecurityFinding {
                    id: format!("SEC-{:04}", id_counter),
                    severity: match vuln.severity {
                        crate::security::Severity::Critical => "Critical",
                        crate::security::Severity::High => "High",
                        crate::security::Severity::Medium => "Medium",
                        crate::security::Severity::Low => "Low",
                        crate::security::Severity::Info => "Info",
                    }
                    .to_string(),
                    category: "Vulnerability".to_string(),
                    title: vuln.title.clone(),
                    description: vuln.description.clone(),
                    resource: format!("{}/{}", vm_ns, vm_name),
                    recommendation: vuln
                        .fixed_version
                        .as_ref()
                        .map(|v| format!("Upgrade to fixed version: {v}"))
                        .unwrap_or_else(|| {
                            "Apply vendor patches or rebuild the golden image".to_string()
                        }),
                    detected_at: now.clone(),
                });
            }
            let _ = base;
        }
    }

    Json(SecurityFindingsResponse {
        veyron_context: if used_trivy {
            VmrogueFeatureContext::security_trivy()
        } else {
            VmrogueFeatureContext::security_config()
        },
        findings,
    })
}
