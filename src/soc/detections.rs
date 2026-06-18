// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use chrono::Utc;
use k8s_openapi::api::networking::v1::NetworkPolicy;
use kube::api::{Api, ListParams};
use serde::{Deserialize, Serialize};

use crate::kube::KubeClient;
use crate::kube::vm_rdp;
use crate::kube::windows_rdp::veyron_rdp_service_name;
use crate::operator_crds::VeyronVM;

use super::event::SecurityEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocDetection {
    pub id: String,
    pub rule_id: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub resource: String,
    pub namespace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vm_name: Option<String>,
    pub status: String,
    pub first_seen: String,
    pub last_seen: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acked_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acked_at: Option<String>,
}

impl SocDetection {
    pub fn open(
        rule_id: &str,
        severity: &str,
        title: &str,
        message: &str,
        resource: &str,
        ns: &str,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        let id = format!("det-{}-{}", rule_id, resource.replace('/', "-"));
        Self {
            id,
            rule_id: rule_id.to_string(),
            severity: severity.to_string(),
            title: title.to_string(),
            message: message.to_string(),
            resource: resource.to_string(),
            namespace: ns.to_string(),
            vm_name: None,
            status: "open".to_string(),
            first_seen: now.clone(),
            last_seen: now,
            acked_by: None,
            acked_at: None,
        }
    }

    pub fn with_vm(mut self, vm: &str) -> Self {
        self.vm_name = Some(vm.to_string());
        self
    }
}

/// Evaluate detection rules against live cluster state.
#[cfg(feature = "web")]
pub async fn evaluate_detections(client: &KubeClient, scope: &str) -> Vec<SocDetection> {
    let mut out = Vec::new();
    let k8s = client.client();
    let vms = client.list_vms_for_scope(scope).await;
    let namespaces: Vec<String> = if scope == "all" {
        vms.iter()
            .filter_map(|v| v.metadata.namespace.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    } else {
        vec![scope.to_string()]
    };

    for ns in &namespaces {
        let np_api: Api<NetworkPolicy> = Api::namespaced(k8s.clone(), ns);
        let np_count = np_api
            .list(&ListParams::default())
            .await
            .map(|l| l.items.len())
            .unwrap_or(0);
        if np_count == 0 {
            out.push(SocDetection::open(
                "no-network-policy-ns",
                "medium",
                "No NetworkPolicies in namespace",
                &format!("Namespace {ns} has zero Kubernetes NetworkPolicies"),
                ns,
                ns,
            ));
        }
    }

    for vm in &vms {
        let name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let ns = vm.metadata.namespace.as_deref().unwrap_or(scope);

        let vmi_spec = &vm.spec.template.spec;
        let uses_host_network = vmi_spec
            .networks
            .as_ref()
            .map(|nets| nets.iter().any(|n| n.name == "host"))
            .unwrap_or(false);
        if uses_host_network {
            out.push(
                SocDetection::open(
                    "privileged-vm-spec",
                    "critical",
                    "Host network enabled on VM",
                    &format!("VM {ns}/{name} uses host network — bypasses pod network isolation"),
                    &format!("{ns}/{name}"),
                    ns,
                )
                .with_vm(name),
            );
        }

        if let Ok(Some(_)) = vm_rdp::get_rdp_expose_service(k8s.clone(), ns, name).await {
            out.push(
                SocDetection::open(
                    "rdp-public-expose",
                    "high",
                    "RDP exposed via NodePort",
                    &format!(
                        "KubeVirt RDP NodePort Service `{}` exposes desktop remotely",
                        veyron_rdp_service_name(name)
                    ),
                    &format!("{ns}/{name}"),
                    ns,
                )
                .with_vm(name),
            );
        }

        if let Ok(Some(svc)) = client.get_vm_expose_service(ns, name).await {
            let st = svc
                .spec
                .as_ref()
                .and_then(|s| s.type_.as_deref())
                .unwrap_or("ClusterIP");
            if st == "NodePort" || st == "LoadBalancer" {
                out.push(
                    SocDetection::open(
                        "ssh-nodeport-expose",
                        "high",
                        "SSH exposed externally",
                        &format!("SSH expose Service for {ns}/{name} is type {st}"),
                        &format!("{ns}/{name}"),
                        ns,
                    )
                    .with_vm(name),
                );
            }
        }

        let vrvm_api: Api<VeyronVM> = Api::namespaced(k8s.clone(), ns);
        if let Ok(vrvm) = vrvm_api.get(name).await {
            if vrvm.status.as_ref().is_some_and(|s| s.drift_detected) {
                out.push(
                    SocDetection::open(
                        "veyron-drift",
                        "medium",
                        "Operator drift detected",
                        &format!("VeyronVM {ns}/{name} reports driftDetected=true"),
                        &format!("{ns}/{name}"),
                        ns,
                    )
                    .with_vm(name),
                );
            }
        }
    }

    let events = client.list_events_for_scope(scope).await;
    let failed_sched: usize = events
        .iter()
        .filter(|e| e.reason.as_deref() == Some("FailedScheduling"))
        .count();
    if failed_sched >= 3 {
        out.push(SocDetection::open(
            "burst-failed-scheduling",
            "medium",
            "Multiple scheduling failures",
            &format!("{failed_sched} FailedScheduling events in scope — check node capacity"),
            scope,
            if scope == "all" { "cluster" } else { scope },
        ));
    }

    out
}

/// Merge freshly evaluated detections with stored ack state.
#[cfg(feature = "web")]
pub fn merge_detections(fresh: Vec<SocDetection>, stored: &[SocDetection]) -> Vec<SocDetection> {
    let now = Utc::now().to_rfc3339();
    let mut out = Vec::new();
    for mut d in fresh {
        if let Some(old) = stored.iter().find(|s| s.id == d.id) {
            d.first_seen = old.first_seen.clone();
            if old.status == "acknowledged" {
                d.status = "acknowledged".to_string();
                d.acked_by = old.acked_by.clone();
                d.acked_at = old.acked_at.clone();
            }
        }
        d.last_seen = now.clone();
        out.push(d);
    }
    out
}

#[cfg(feature = "web")]
pub fn detection_to_event(d: &SocDetection) -> SecurityEvent {
    super::event::SecurityEvent::new("detection", &d.severity, "veyron", &d.message)
        .with_namespace(&d.namespace)
        .with_action(&d.rule_id)
        .with_outcome(&d.status)
        .with_target(&d.resource)
        .label("rule.id", &d.rule_id)
        .label("detection.title", &d.title)
}
