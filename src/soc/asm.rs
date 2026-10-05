// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use serde::{Deserialize, Serialize};

use crate::kube::KubeClient;
use crate::kube::vm_rdp;
use crate::kube::windows_rdp::veyron_rdp_service_name;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackSurfaceAsset {
    pub vm_namespace: String,
    pub vm_name: String,
    pub exposure_type: String,
    pub detail: String,
    pub risk_score: u8,
    pub internet_facing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackSurfaceReport {
    pub scope: String,
    pub assets: Vec<AttackSurfaceAsset>,
    pub total_internet_facing: u32,
    pub average_risk_score: u8,
    pub scanned_at: String,
}

#[cfg(feature = "web")]
pub async fn scan_attack_surface(client: &KubeClient, scope: &str) -> AttackSurfaceReport {
    let mut assets = Vec::new();
    let k8s = client.client();
    let vms = client.list_vms_for_scope(scope).await;

    for vm in &vms {
        let name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let ns = vm.metadata.namespace.as_deref().unwrap_or(scope);

        if let Ok(Some(_)) = vm_rdp::get_rdp_expose_service(k8s.clone(), ns, name).await {
            assets.push(AttackSurfaceAsset {
                vm_namespace: ns.to_string(),
                vm_name: name.to_string(),
                exposure_type: "rdp_nodeport".to_string(),
                detail: format!("RDP service {}", veyron_rdp_service_name(name)),
                risk_score: 85,
                internet_facing: true,
            });
        }

        if let Ok(Some(svc)) = client.get_vm_expose_service(ns, name).await {
            let st = svc
                .spec
                .as_ref()
                .and_then(|s| s.type_.as_deref())
                .unwrap_or("ClusterIP");
            if st == "NodePort" || st == "LoadBalancer" {
                assets.push(AttackSurfaceAsset {
                    vm_namespace: ns.to_string(),
                    vm_name: name.to_string(),
                    exposure_type: "ssh_expose".to_string(),
                    detail: format!("SSH expose Service type {st}"),
                    risk_score: 80,
                    internet_facing: true,
                });
            }
        }

        let vmi_spec = &vm.spec.template.spec;
        if vmi_spec
            .networks
            .as_ref()
            .map(|nets| nets.iter().any(|n| n.name == "host"))
            .unwrap_or(false)
        {
            assets.push(AttackSurfaceAsset {
                vm_namespace: ns.to_string(),
                vm_name: name.to_string(),
                exposure_type: "host_network".to_string(),
                detail: "VM uses host network stack".to_string(),
                risk_score: 90,
                internet_facing: true,
            });
        }
    }

    let total_if = assets.iter().filter(|a| a.internet_facing).count() as u32;
    let avg = if assets.is_empty() {
        0
    } else {
        (assets.iter().map(|a| a.risk_score as u32).sum::<u32>() / assets.len() as u32) as u8
    };

    AttackSurfaceReport {
        scope: scope.to_string(),
        assets,
        total_internet_facing: total_if,
        average_risk_score: avg,
        scanned_at: chrono::Utc::now().to_rfc3339(),
    }
}
