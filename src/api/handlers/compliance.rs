// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

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
use super::namespace_scope::{self, DashboardNamespaceQuery};

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
    /// True if a control check below couldn't reach the Kubernetes API (as opposed
    /// to genuinely finding zero matching resources) — when true, `score` may be
    /// artificially low because a failed check is conservatively scored as failing.
    #[serde(default)]
    pub data_incomplete: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
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
async fn compute_compliance_statuses(
    client: &crate::kube::KubeClient,
    scope: &str,
) -> Vec<ComplianceStatus> {
    use k8s_openapi::api::networking::v1::NetworkPolicy;
    use k8s_openapi::api::rbac::v1::RoleBinding;

    let kube = client.client();
    let vms = client.list_vms_for_scope(scope).await;
    let now = chrono::Utc::now().to_rfc3339();
    let total_vms = vms.len() as u32;

    // Per-VM compliance checks
    let mut resource_limits = 0u32;
    let mut eviction_strategy = 0u32;
    let mut rng_device = 0u32;
    let mut no_host_network = 0u32;
    let mut tpm_device = 0u32;

    for vm in &vms {
        let vmi_spec = &vm.spec.template.spec;
        let domain = &vmi_spec.domain;

        if domain
            .resources
            .limits
            .as_ref()
            .map(|m| !m.is_empty())
            .unwrap_or(false)
        {
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
        if domain
            .devices
            .as_ref()
            .and_then(|d| d.tpm.as_ref())
            .is_some()
        {
            tpm_device += 1;
        }
        let uses_host_net = vmi_spec
            .networks
            .as_ref()
            .map(|nets| nets.iter().any(|n| n.name == "host"))
            .unwrap_or(false);
        if !uses_host_net {
            no_host_network += 1;
        }
    }

    // Namespace-level checks (count as pass/fail against total_vms for scoring).
    // A list failure here (permissions, API hiccup) is NOT the same as "genuinely
    // zero policies/bindings" — track it separately so the score doesn't silently
    // masquerade a broken check as a real compliance finding.
    let mut warnings: Vec<String> = Vec::new();

    let np_result = if namespace_scope::is_all_namespaces(scope) {
        let np_api: kube::api::Api<NetworkPolicy> = kube::api::Api::all(kube.clone());
        np_api.list(&kube::api::ListParams::default()).await
    } else {
        let np_api: kube::api::Api<NetworkPolicy> = kube::api::Api::namespaced(kube.clone(), scope);
        np_api.list(&kube::api::ListParams::default()).await
    };
    let np_count = match np_result {
        Ok(l) => l.items.len() as u32,
        Err(e) => {
            warnings.push(format!("network-policies check could not reach the API: {e}"));
            0
        }
    };
    let has_network_policies = if np_count > 0 { total_vms } else { 0 };

    let rb_result = if namespace_scope::is_all_namespaces(scope) {
        let rb_api: kube::api::Api<RoleBinding> = kube::api::Api::all(kube.clone());
        rb_api.list(&kube::api::ListParams::default()).await
    } else {
        let rb_api: kube::api::Api<RoleBinding> = kube::api::Api::namespaced(kube.clone(), scope);
        rb_api.list(&kube::api::ListParams::default()).await
    };
    let rb_count = match rb_result {
        Ok(l) => l.items.len() as u32,
        Err(e) => {
            warnings.push(format!("rbac-configured check could not reach the API: {e}"));
            0
        }
    };
    let has_rbac = if rb_count > 0 { total_vms } else { 0 };
    let data_incomplete = !warnings.is_empty();

    // CIS KubeVirt Benchmark: 7 controls
    let cis_controls: &[(&str, u32)] = &[
        ("resource-limits", resource_limits),
        ("eviction-strategy", eviction_strategy),
        ("rng-device", rng_device),
        ("tpm-device", tpm_device),
        ("no-host-network", no_host_network),
        ("network-policies", has_network_policies),
        ("rbac-configured", has_rbac),
    ];
    let cis_total = cis_controls.len() as u32 * total_vms.max(1);
    let cis_passing: u32 = cis_controls.iter().map(|(_, p)| p).sum();
    let cis_failing = cis_total.saturating_sub(cis_passing);
    let cis_score = if cis_total == 0 {
        100u8
    } else {
        ((cis_passing as f64 / cis_total as f64) * 100.0) as u8
    };

    // NIST SP 800-190 subset: resource isolation + network segmentation
    let nist_controls: &[(&str, u32)] = &[
        ("resource-limits", resource_limits),
        ("no-host-network", no_host_network),
        ("network-policies", has_network_policies),
        ("rbac-configured", has_rbac),
    ];
    let nist_total = nist_controls.len() as u32 * total_vms.max(1);
    let nist_passing: u32 = nist_controls.iter().map(|(_, p)| p).sum();
    let nist_failing = nist_total.saturating_sub(nist_passing);
    let nist_score = if nist_total == 0 {
        100u8
    } else {
        ((nist_passing as f64 / nist_total as f64) * 100.0) as u8
    };

    vec![
        ComplianceStatus {
            framework: "CIS Benchmark".to_string(),
            compliant: cis_failing == 0,
            score: cis_score,
            total_controls: cis_total,
            passing_controls: cis_passing,
            failing_controls: cis_failing,
            last_checked: now.clone(),
            data_incomplete,
            warnings: warnings.clone(),
        },
        ComplianceStatus {
            framework: "NIST SP 800-190".to_string(),
            compliant: nist_failing == 0,
            score: nist_score,
            total_controls: nist_total,
            passing_controls: nist_passing,
            failing_controls: nist_failing,
            last_checked: now.clone(),
            data_incomplete,
            warnings,
        },
    ]
}

#[cfg(feature = "web")]
async fn get_compliance_status(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<ComplianceStatus>> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    Json(compute_compliance_statuses(s.client(), &scope).await)
}

#[cfg(feature = "web")]
async fn list_compliance_reports(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<ComplianceReport>> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let statuses = compute_compliance_statuses(s.client(), &scope).await;
    let now = chrono::Utc::now().to_rfc3339();

    let reports: Vec<ComplianceReport> = statuses
        .into_iter()
        .map(|status| ComplianceReport {
            id: format!(
                "report-{}",
                status.framework.to_lowercase().replace(' ', "-")
            ),
            framework: status.framework.clone(),
            generated_at: now.clone(),
            summary: status,
            findings: Vec::new(),
        })
        .collect();

    Json(reports)
}
