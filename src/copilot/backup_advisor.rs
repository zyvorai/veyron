// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use std::collections::{HashMap, HashSet};

use kube::api::{Api, ApiResource, DynamicObject, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;
use crate::snapshots::vm_snapshot_schedule;

/// Fleet backup coverage: snapshots, schedules, and Velero presence.
pub async fn backup_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Backup Advisor",
        "backup_advisor",
        "Backup coverage in workspace",
    );

    let k8s = client.client();
    let vms = client.list_vms_for_scope(scope).await;
    let running_or_stopped: Vec<_> = vms
        .iter()
        .filter(|vm| {
            let status = vm
                .status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                .unwrap_or("Unknown");
            !matches!(status, "Failed" | "Error" | "Unknown")
        })
        .collect();

    let snapshots = crate::snapshots::SnapshotManager::list_snapshots_in_scope(k8s.clone(), scope)
        .await
        .unwrap_or_default();

    let mut snap_by_vm: HashMap<String, u32> = HashMap::new();
    for snap in &snapshots {
        let key = format!("{}/{}", snap.namespace, snap.vm_name);
        *snap_by_vm.entry(key).or_insert(0) += 1;
    }

    let schedules = vm_snapshot_schedule::list_schedule_configmaps(k8s.clone())
        .await
        .unwrap_or_default();
    let mut scheduled_vms: HashSet<String> = HashSet::new();
    for cm in schedules {
        let ns = cm
            .metadata
            .namespace
            .unwrap_or_else(|| "default".to_string());
        if let Some(data) = cm.data {
            if let Ok(rec) = serde_json::from_str::<vm_snapshot_schedule::SnapshotScheduleRecord>(
                data.get(vm_snapshot_schedule::SCHEDULE_CM_DATA_KEY)
                    .map(String::as_str)
                    .unwrap_or("{}"),
            ) {
                if rec.enabled {
                    scheduled_vms.insert(format!("{ns}/{}", rec.vm_name));
                }
            }
        }
    }

    let velero_available = velero_installed(k8s.clone()).await;
    let velero_backups = if velero_available {
        list_velero_backup_count(k8s).await
    } else {
        0
    };

    let mut unprotected = Vec::new();
    for vm in &running_or_stopped {
        let name = vm.metadata.name.as_deref().unwrap_or("");
        let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
        let key = format!("{ns}/{name}");
        let snaps = snap_by_vm.get(&key).copied().unwrap_or(0);
        let scheduled = scheduled_vms.contains(&key);
        if snaps == 0 && !scheduled {
            unprotected.push(key);
        }
    }

    r.summary = if running_or_stopped.is_empty() {
        "No VMs in scope to evaluate.".into()
    } else if unprotected.is_empty() {
        format!(
            "All {} VM(s) have snapshot coverage or an active schedule.",
            running_or_stopped.len()
        )
    } else {
        format!(
            "{} of {} VM(s) lack snapshots and have no snapshot schedule.",
            unprotected.len(),
            running_or_stopped.len()
        )
    };

    r.evidence.push(format!(
        "KubeVirt snapshots in scope: {} across {} VM(s)",
        snapshots.len(),
        snap_by_vm.len()
    ));
    r.evidence.push(format!(
        "Snapshot schedules (enabled): {}",
        scheduled_vms.len()
    ));
    if velero_available {
        r.evidence.push(format!(
            "Velero installed — {velero_backups} Backup CR(s) listed"
        ));
    } else {
        r.evidence
            .push("Velero not detected — tier-1 backup is KubeVirt snapshots + schedules".into());
    }

    for key in unprotected.iter().take(12) {
        r.evidence.push(format!("Unprotected: {key}"));
    }
    if unprotected.len() > 12 {
        r.evidence
            .push(format!("… and {} more", unprotected.len() - 12));
    }

    if !unprotected.is_empty() {
        r.recommendations
            .push("Create a snapshot or enable a cron schedule before risky changes.".into());
        r.recommendations
            .push("For DR off-cluster, install Velero and label production VMs.".into());
    }
    if velero_available && velero_backups == 0 {
        r.recommendations
            .push("Velero is installed but no Backup objects found — define a schedule.".into());
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Backups".into(),
            action: "navigate".into(),
            page: Some("backups".into()),
        },
        CopilotAction {
            label: "Open Snapshots".into(),
            action: "navigate".into(),
            page: Some("snapshots".into()),
        },
    ];

    let score = if running_or_stopped.is_empty() {
        100
    } else {
        let protected = running_or_stopped.len().saturating_sub(unprotected.len());
        ((protected as f32 / running_or_stopped.len() as f32) * 100.0).round() as u8
    };
    r.health_score = Some(score);
    r
}

async fn velero_installed(client: kube::Client) -> bool {
    let ar = ApiResource {
        group: "velero.io".to_string(),
        version: "v1".to_string(),
        api_version: "velero.io/v1".to_string(),
        kind: "Backup".to_string(),
        plural: "backups".to_string(),
    };
    let api: Api<DynamicObject> = Api::all_with(client, &ar);
    api.list(&ListParams::default().limit(1)).await.is_ok()
}

async fn list_velero_backup_count(client: kube::Client) -> u32 {
    let ar = ApiResource {
        group: "velero.io".to_string(),
        version: "v1".to_string(),
        api_version: "velero.io/v1".to_string(),
        kind: "Backup".to_string(),
        plural: "backups".to_string(),
    };
    let api: Api<DynamicObject> = Api::all_with(client, &ar);
    api.list(&ListParams::default().limit(50))
        .await
        .ok()
        .map(|l| l.items.len() as u32)
        .unwrap_or(0)
}
