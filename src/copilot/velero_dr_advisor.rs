// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use kube::api::{Api, ApiResource, DynamicObject, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// Velero backup/restore health and DR readiness (distinct from snapshot Backup Advisor).
pub async fn velero_dr_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Velero DR Advisor",
        "velero_dr_advisor",
        "Disaster recovery posture",
    );

    let k8s = client.client();
    let backup_ar = ApiResource {
        group: "velero.io".into(),
        version: "v1".into(),
        api_version: "velero.io/v1".into(),
        kind: "Backup".into(),
        plural: "backups".into(),
    };
    let restore_ar = ApiResource {
        group: "velero.io".into(),
        version: "v1".into(),
        api_version: "velero.io/v1".into(),
        kind: "Restore".into(),
        plural: "restores".into(),
    };

    let backup_api: Api<DynamicObject> = Api::all_with(k8s.clone(), &backup_ar);
    let restore_api: Api<DynamicObject> = Api::all_with(k8s.clone(), &restore_ar);

    let backups = backup_api
        .list(&ListParams::default().limit(50))
        .await
        .map(|l| l.items)
        .unwrap_or_default();
    let restores = restore_api
        .list(&ListParams::default().limit(30))
        .await
        .map(|l| l.items)
        .unwrap_or_default();

    let velero_installed = !backups.is_empty()
        || !restores.is_empty()
        || backup_api
            .list(&ListParams::default().limit(1))
            .await
            .is_ok();

    if !velero_installed {
        r.summary =
            "Velero not detected — use KubeVirt snapshots + Backup Advisor for tier-1 coverage."
                .into();
        r.evidence
            .push("No Velero Backup/Restore CRs found in cluster.".into());
        r.recommendations
            .push("Install Velero for namespace-level DR and off-cluster backup storage.".into());
        r.actions.push(CopilotAction {
            label: "Open Backups".into(),
            action: "navigate".into(),
            page: Some("backups".into()),
        });
        return r;
    }

    let mut failed_backups = 0usize;
    let mut recent_ok = 0usize;
    for b in &backups {
        let name = b.metadata.name.as_deref().unwrap_or("?");
        let phase = b
            .data
            .get("status")
            .and_then(|s| s.get("phase"))
            .and_then(|p| p.as_str())
            .unwrap_or("Unknown");
        if phase == "Failed" || phase == "PartiallyFailed" {
            failed_backups += 1;
            r.evidence.push(format!("Failed backup: {name} ({phase})"));
        } else if phase == "Completed" {
            recent_ok += 1;
        }
    }

    let mut failed_restores = 0usize;
    for rest in &restores {
        let name = rest.metadata.name.as_deref().unwrap_or("?");
        let phase = rest
            .data
            .get("status")
            .and_then(|s| s.get("phase"))
            .and_then(|p| p.as_str())
            .unwrap_or("Unknown");
        if phase == "Failed" {
            failed_restores += 1;
            r.evidence.push(format!("Failed restore: {name}"));
        }
    }

    let vms = client.list_vms_for_scope(scope).await.len();
    r.evidence
        .push(format!("Velero backups listed: {}", backups.len()));
    r.evidence.push(format!(
        "Completed backups: {recent_ok}, failed: {failed_backups}"
    ));
    r.evidence.push(format!(
        "Restore CRs: {} (failed: {failed_restores})",
        restores.len()
    ));
    r.evidence.push(format!("VMs in scope: {vms}"));

    r.summary = if failed_backups > 0 || failed_restores > 0 {
        format!(
            "Velero DR issues — {failed_backups} failed backup(s), {failed_restores} failed restore(s)."
        )
    } else if recent_ok == 0 {
        "Velero installed but no completed backups — schedule regular cluster backups.".into()
    } else {
        format!("Velero healthy — {recent_ok} completed backup(s), DR pipeline active.")
    };

    if recent_ok == 0 {
        r.recommendations
            .push("Create a Velero BackupSchedule for VM namespaces.".into());
    }
    if failed_backups > 0 {
        r.recommendations
            .push("Inspect failed Backup CR status and storage location connectivity.".into());
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Backups".into(),
            action: "navigate".into(),
            page: Some("backups".into()),
        },
        CopilotAction {
            label: "Open Disaster Recovery".into(),
            action: "navigate".into(),
            page: Some("dr".into()),
        },
    ];
    r
}
