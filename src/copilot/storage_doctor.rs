// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use std::collections::{HashMap, HashSet};

use k8s_openapi::api::core::v1::PersistentVolumeClaim;

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;
use crate::kube::guest_filesystem::format_mount_evidence;
use crate::kube::types::VirtualMachine;
use crate::kube::windows_rdp::vmi_guest_agent_connected;
use crate::snapshots::SnapshotManager;

/// Fleet storage posture: PVC pressure, snapshot sprawl, and unbound volumes.
pub async fn storage_doctor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Storage Doctor",
        "storage_doctor",
        "Storage health in workspace",
    );

    let k8s = client.client();
    let pvcs = client.list_pvcs_for_scope(scope).await;
    let vms = client.list_vms_for_scope(scope).await;
    let snapshots = SnapshotManager::list_snapshots_in_scope(k8s.clone(), scope)
        .await
        .unwrap_or_default();

    let vm_pvc_names = vm_bound_pvc_names(&vms);
    let mut snap_by_vm: HashMap<String, u32> = HashMap::new();
    for snap in &snapshots {
        let key = format!("{}/{}", snap.namespace, snap.vm_name);
        *snap_by_vm.entry(key).or_insert(0) += 1;
    }

    let prom_usage = prometheus_pvc_usage().await;

    let mut high_usage = Vec::new();
    let mut pending = Vec::new();
    let mut orphan = Vec::new();

    for pvc in &pvcs {
        let ns = pvc.metadata.namespace.as_deref().unwrap_or("default");
        let name = pvc.metadata.name.as_deref().unwrap_or("");
        let key = format!("{ns}/{name}");
        let phase = pvc
            .status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .unwrap_or("?");
        if phase == "Pending" {
            pending.push(key.clone());
        }
        let cap_bytes = pvc_capacity_bytes(pvc);
        let used_bytes = prom_usage
            .get(&(ns.to_string(), name.to_string()))
            .copied()
            .unwrap_or(0);
        if cap_bytes > 0 && used_bytes > 0 {
            let pct = (used_bytes as f64 / cap_bytes as f64) * 100.0;
            if pct >= 85.0 {
                high_usage.push(format!("{key} — {pct:.0}% used"));
            }
        } else if cap_bytes > 0 && !vm_pvc_names.contains(&key) {
            orphan.push(key);
        }
    }

    let mut snap_sprawl = Vec::new();
    for (vm_key, count) in &snap_by_vm {
        if *count >= 5 {
            snap_sprawl.push(format!("{vm_key} — {count} snapshots"));
        }
    }

    r.evidence.push(format!("PVCs in scope: {}", pvcs.len()));
    r.evidence
        .push(format!("KubeVirt snapshots: {} total", snapshots.len()));
    if prom_usage.is_empty() {
        r.evidence.push(
            "PVC usage %: set VEYRON_PROMETHEUS_URL for kubelet volume stats (optional).".into(),
        );
    } else {
        r.evidence.push(format!(
            "Prometheus enriched {} PVC(s) with used bytes",
            prom_usage.len()
        ));
    }

    for row in high_usage.iter().take(8) {
        r.evidence.push(format!("High usage: {row}"));
    }
    for row in pending.iter().take(6) {
        r.evidence.push(format!("Pending PVC: {row}"));
    }
    for row in orphan.iter().take(6) {
        r.evidence
            .push(format!("Orphan PVC (no VM volume ref): {row}"));
    }
    for row in snap_sprawl.iter().take(6) {
        r.evidence.push(format!("Snapshot sprawl: {row}"));
    }

    let issues = high_usage.len() + pending.len() + snap_sprawl.len();
    r.summary = if pvcs.is_empty() {
        "No PVCs in the active workspace scope.".into()
    } else if issues == 0 {
        format!(
            "{} PVC(s) look healthy; no critical storage pressure.",
            pvcs.len()
        )
    } else {
        format!(
            "{issues} storage signal(s) need attention across {} PVC(s).",
            pvcs.len()
        )
    };

    if !high_usage.is_empty() {
        r.recommendations.push(
            "Expand PVCs or prune guest data before disks fill — snapshot before resize.".into(),
        );
    }
    if !pending.is_empty() {
        r.recommendations
            .push("Pending PVCs block VM start — verify StorageClass and CSI drivers.".into());
    }
    if !snap_sprawl.is_empty() {
        r.recommendations
            .push("Apply snapshot retention schedules to control backup sprawl.".into());
    }
    if !orphan.is_empty() {
        r.recommendations
            .push("Review orphan PVCs — delete only after confirming they are unused.".into());
    }

    if prom_usage.is_empty() {
        r.evidence.push(
            "In-guest filesystem: GET /api/v1/vms/:ns/:name/guest-filesystem (guest agent required).".into(),
        );
        sample_guest_filesystem(client, &vms, &mut r).await;
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Storage".into(),
            action: "navigate".into(),
            page: Some("storage".into()),
        },
        CopilotAction {
            label: "Open Snapshots".into(),
            action: "navigate".into(),
            page: Some("snapshots".into()),
        },
        CopilotAction {
            label: "Backup Advisor".into(),
            action: "navigate".into(),
            page: Some("backups".into()),
        },
    ];
    r
}

async fn sample_guest_filesystem(
    client: &KubeClient,
    vms: &[VirtualMachine],
    r: &mut CopilotResponse,
) {
    use serde_json::to_value;

    for vm in vms
        .iter()
        .filter(|v| {
            v.status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                == Some("Running")
        })
        .take(2)
    {
        let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
        let name = vm.metadata.name.as_deref().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let Ok(vmi) = client.get_vmi(ns, name).await else {
            continue;
        };
        let vmi_json = to_value(&vmi).ok();
        if !vmi_json.as_ref().is_some_and(vmi_guest_agent_connected) {
            continue;
        }
        let vm_json = to_value(vm).ok();
        if let Ok(resp) = crate::kube::guest_filesystem::collect_guest_filesystem(
            client.client().clone(),
            ns,
            name,
            vmi.metadata.name.as_deref().unwrap_or(name),
            vm_json.as_ref().unwrap_or(&serde_json::Value::Null),
            vmi_json.as_ref().unwrap_or(&serde_json::Value::Null),
        )
        .await
        {
            for m in resp.mounts.iter().take(4) {
                r.evidence.push(format!(
                    "Guest FS {ns}/{name}: {}",
                    format_mount_evidence(m)
                ));
            }
            return;
        }
    }
}

fn vm_bound_pvc_names(vms: &[VirtualMachine]) -> HashSet<String> {
    let mut names = HashSet::new();
    for vm in vms {
        let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
        if let Some(vols) = vm.spec.template.spec.volumes.as_ref() {
            for vol in vols {
                if let Some(pvc) = vol.persistent_volume_claim.as_ref() {
                    names.insert(format!("{ns}/{}", pvc.claim_name));
                }
                if let Some(dv) = vol.data_volume.as_ref() {
                    names.insert(format!("{ns}/{}", dv.name));
                }
            }
        }
    }
    names
}

fn pvc_capacity_bytes(pvc: &PersistentVolumeClaim) -> u64 {
    pvc.status
        .as_ref()
        .and_then(|s| s.capacity.as_ref())
        .and_then(|c| c.get("storage"))
        .map(|q| crate::utils::parse_memory_bytes(&q.0))
        .unwrap_or(0)
}

#[cfg(feature = "web")]
async fn prometheus_pvc_usage() -> HashMap<(String, String), u64> {
    let url = match std::env::var("VEYRON_PROMETHEUS_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => return HashMap::new(),
    };
    let rows =
        crate::api::prometheus::instant_query_vector(&url, "kubelet_volume_stats_used_bytes")
            .await
            .unwrap_or_default();
    let mut out = HashMap::new();
    for (metric, val) in rows {
        let ns = metric.get("namespace").cloned().unwrap_or_default();
        let pvc = metric
            .get("persistentvolumeclaim")
            .cloned()
            .unwrap_or_default();
        if !ns.is_empty() && !pvc.is_empty() {
            out.insert((ns, pvc), val as u64);
        }
    }
    out
}

#[cfg(not(feature = "web"))]
async fn prometheus_pvc_usage() -> HashMap<(String, String), u64> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pvc_capacity_parses_quantity() {
        use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
        let mut cap = std::collections::BTreeMap::new();
        cap.insert("storage".to_string(), Quantity("10Gi".to_string()));
        let pvc = PersistentVolumeClaim {
            status: Some(k8s_openapi::api::core::v1::PersistentVolumeClaimStatus {
                phase: Some("Bound".into()),
                capacity: Some(cap),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(pvc_capacity_bytes(&pvc) > 0);
    }
}
