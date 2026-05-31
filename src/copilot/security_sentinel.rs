// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use k8s_openapi::api::networking::v1::NetworkPolicy;
use kube::api::{Api, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;
use crate::kube::vm_rdp;
use crate::kube::windows_rdp::vmrogue_rdp_service_name;
use crate::operator_crds::VMRogueVM;

/// Per-VM security posture: exposure, policies, and operator drift.
pub async fn security_sentinel(client: &KubeClient, namespace: &str, name: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Security Sentinel",
        "security_sentinel",
        format!("Security view: {namespace}/{name}"),
    );

    let mut findings = 0u32;

    let k8s = client.client();
    let np_api: Api<NetworkPolicy> = Api::namespaced(k8s.clone(), namespace);
    let policies = np_api.list(&ListParams::default()).await;
    let policy_count = policies.as_ref().map(|l| l.items.len()).unwrap_or(0);
    r.evidence
        .push(format!("Namespace NetworkPolicies: {policy_count}"));
    if policy_count == 0 {
        findings += 1;
        r.evidence
            .push("No Kubernetes NetworkPolicies in namespace".into());
    }

    match vm_rdp::get_rdp_expose_service(k8s.clone(), namespace, name).await {
        Ok(Some(svc)) => {
            let np = vm_rdp::rdp_node_port_from_service(&svc);
            findings += 1;
            r.evidence.push(format!(
                "RDP exposed via NodePort Service `{}`{}",
                vmrogue_rdp_service_name(name),
                np.map(|p| format!(" on port {p}"))
                    .unwrap_or_default()
            ));
            r.recommendations.push(
                "Restrict RDP NodePort with firewall, VPN, or Gateway API — avoid public exposure.".into(),
            );
        }
        Ok(None) => r.evidence.push("No RDP NodePort Service".into()),
        Err(e) => r.evidence.push(format!("RDP lookup: {e}")),
    }

    match client.get_vm_expose_service(namespace, name).await {
        Ok(Some(svc)) => {
            let st = svc
                .spec
                .as_ref()
                .and_then(|s| s.type_.as_deref())
                .unwrap_or("ClusterIP");
            if st == "NodePort" || st == "LoadBalancer" {
                findings += 1;
                r.evidence
                    .push(format!("SSH expose Service is {st} — guest reachable from outside cluster"));
            } else {
                r.evidence.push(format!("SSH expose Service — {st}"));
            }
        }
        Ok(None) => r.evidence.push("No SSH expose Service".into()),
        Err(e) => r.evidence.push(format!("SSH expose lookup: {e}")),
    }

    let vrvm_api: Api<VMRogueVM> = Api::namespaced(k8s, namespace);
    match vrvm_api.get(name).await {
        Ok(vrvm) => {
            if vrvm.status.as_ref().is_some_and(|s| s.drift_detected) {
                findings += 1;
                let msg = vrvm
                    .status
                    .as_ref()
                    .and_then(|s| s.drift_message.clone())
                    .unwrap_or_else(|| "spec drift from template/profile".into());
                r.evidence.push(format!("Operator drift detected: {msg}"));
                r.recommendations
                    .push("Reconcile VMRogueVM or update template/profile to clear drift.".into());
            } else {
                r.evidence.push("VMRogueVM operator: in sync (no drift)".into());
            }
        }
        Err(_) => r.evidence.push("Not operator-managed (no VMRogueVM CR)".into()),
    }

    let allow_inet = client
        .get_vm(namespace, name)
        .await
        .ok()
        .and_then(|vm| {
            vm.metadata
                .labels
                .as_ref()
                .and_then(|l| l.get("vmrogue.io/allow-internet"))
                .map(|v| v == "true")
        })
        .unwrap_or(true);
    if allow_inet {
        r.evidence.push("Internet egress: allowed (default)".into());
    } else {
        r.evidence.push("Internet egress: restricted by label".into());
    }

    append_trivy_evidence(&mut r, name, &mut findings).await;

    r.summary = if findings == 0 {
        format!("No high-risk exposure signals for {name}.")
    } else {
        format!("{findings} security signal(s) for {name} — review exposure and drift.")
    };

    if policy_count == 0 {
        r.recommendations.push(
            "Add default-deny NetworkPolicies with explicit egress for production VMs.".into(),
        );
    }
    if findings == 0 && r.recommendations.is_empty() {
        r.recommendations
            .push("Continue periodic snapshot and Velero backup checks.".into());
    }

    r.actions = vec![
        CopilotAction {
            label: "Security page".into(),
            action: "navigate".into(),
            page: Some("security".into()),
        },
        CopilotAction {
            label: "Network policies".into(),
            action: "navigate".into(),
            page: Some("network-policies".into()),
        },
        CopilotAction {
            label: "Open VM detail".into(),
            action: "open_vm".into(),
            page: Some(format!("{namespace}:{name}")),
        },
    ];
    r
}

#[cfg(feature = "web")]
async fn append_trivy_evidence(r: &mut CopilotResponse, vm_name: &str, findings: &mut u32) {
    let Some(base) = std::env::var("VMROGUE_TRIVY_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
    else {
        r.evidence
            .push("Trivy not configured — set VMROGUE_TRIVY_URL for CVE scan".into());
        return;
    };
    match trivy_summary(&base, vm_name).await {
        Some((crit, high)) => {
            if crit + high > 0 {
                *findings += 1;
                r.evidence.push(format!(
                    "Trivy: {crit} critical + {high} high CVE(s) on {vm_name}"
                ));
                r.recommendations
                    .push("Patch guest OS packages or rebuild golden image.".into());
            } else {
                r.evidence
                    .push(format!("Trivy: no critical/high CVEs on {vm_name}"));
            }
        }
        None => r.evidence.push(format!("Trivy scan unavailable for {vm_name}")),
    }
}

#[cfg(not(feature = "web"))]
async fn append_trivy_evidence(r: &mut CopilotResponse, _vm_name: &str, _findings: &mut u32) {
    let _ = r;
}

#[cfg(feature = "web")]
async fn trivy_summary(base_url: &str, vm_name: &str) -> Option<(u32, u32)> {
    let url = format!("{}/scan/{}", base_url.trim_end_matches('/'), vm_name);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .ok()?;
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    let vulns = body.get("vulnerabilities")?.as_array()?;
    let mut crit = 0u32;
    let mut high = 0u32;
    for v in vulns {
        match v.get("Severity").and_then(|s| s.as_str()) {
            Some("CRITICAL") => crit += 1,
            Some("HIGH") => high += 1,
            _ => {}
        }
    }
    Some((crit, high))
}

#[cfg(not(feature = "web"))]
async fn trivy_summary(_base_url: &str, _vm_name: &str) -> Option<(u32, u32)> {
    None
}

/// Fleet-wide security sweep for Copilot ask.
pub async fn security_sentinel_fleet(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Security Sentinel",
        "security_fleet",
        "Fleet security signals",
    );

    let vms = client.list_vms_for_scope(scope).await;
    let mut exposed_rdp = Vec::new();
    let mut drift = Vec::new();
    let k8s = client.client();

    for vm in &vms {
        let name = vm.metadata.name.as_deref().unwrap_or("");
        let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
        if vm_rdp::get_rdp_expose_service(k8s.clone(), ns, name)
            .await
            .ok()
            .flatten()
            .is_some()
        {
            exposed_rdp.push(format!("{ns}/{name}"));
        }
        let vrvm_api: Api<VMRogueVM> = Api::namespaced(k8s.clone(), ns);
        if let Ok(vrvm) = vrvm_api.get(name).await {
            if vrvm.status.as_ref().is_some_and(|s| s.drift_detected) {
                drift.push(format!("{ns}/{name}"));
            }
        }
    }

    r.evidence.push(format!("VMs evaluated: {}", vms.len()));
    for key in exposed_rdp.iter().take(8) {
        r.evidence.push(format!("RDP NodePort exposed: {key}"));
    }
    for key in drift.iter().take(8) {
        r.evidence.push(format!("Operator drift: {key}"));
    }

    let trivy_url = std::env::var("VMROGUE_TRIVY_URL")
        .ok()
        .filter(|u| !u.trim().is_empty());
    if let Some(base) = trivy_url {
        let mut cve_vms = 0usize;
        for vm in vms.iter().take(15) {
            let name = vm.metadata.name.as_deref().unwrap_or("");
            if let Some((crit, high)) = trivy_summary(&base, name).await {
                if crit + high > 0 {
                    cve_vms += 1;
                    r.evidence
                        .push(format!("Trivy {name}: {crit} critical, {high} high CVE(s)"));
                }
            }
        }
        if cve_vms == 0 {
            r.evidence
                .push("Trivy: no critical/high CVEs in sampled VMs".into());
        }
    } else {
        r.evidence.push(
            "Trivy not configured — set VMROGUE_TRIVY_URL for CVE merge".into(),
        );
    }

    let issues = exposed_rdp.len() + drift.len();
    r.summary = if issues == 0 {
        "No fleet-wide RDP exposure or operator drift detected in scope.".into()
    } else {
        format!("{issues} fleet security signal(s) in scope.")
    };

    if !exposed_rdp.is_empty() {
        r.recommendations
            .push("Audit RDP NodePort Services — use VPN or bastion access.".into());
    }
    if !drift.is_empty() {
        r.recommendations
            .push("Resolve VMRogueVM drift before production changes.".into());
    }

    r.actions.push(CopilotAction {
        label: "Open Security".into(),
        action: "navigate".into(),
        page: Some("security".into()),
    });
    r
}
