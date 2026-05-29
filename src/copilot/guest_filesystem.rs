// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};
use crate::kube::guest_filesystem::{collect_guest_filesystem, format_mount_evidence};
use crate::kube::KubeClient;
use serde_json::to_value;

/// Copilot report for in-guest filesystem usage (guest-exec).
pub async fn guest_filesystem_report(
    client: &KubeClient,
    namespace: &str,
    name: &str,
) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Guest Filesystem",
        "guest_filesystem",
        format!("Guest disk usage: {namespace}/{name}"),
    );

    let vm = match client.get_vm(namespace, name).await {
        Ok(v) => v,
        Err(e) => {
            r.summary = format!("Could not load VirtualMachine: {e}");
            return r;
        }
    };

    let status = vm
        .status
        .as_ref()
        .and_then(|s| s.printable_status.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    if status != "Running" {
        r.summary = format!("{name} is {status} — guest filesystem probe needs a running VMI.");
        r.recommendations
            .push("Start the VM and wait for AgentConnected.".into());
        return r;
    }

    let vmi = match client.get_vmi(namespace, name).await {
        Ok(v) => v,
        Err(e) => {
            r.summary = format!("VM is Running but VMI is unavailable: {e}");
            return r;
        }
    };

    let vm_json = to_value(&vm).ok();
    let vmi_json = to_value(&vmi).ok();
    let vmi_name = vmi.metadata.name.clone().unwrap_or_else(|| name.to_string());

    match collect_guest_filesystem(
        client.client().clone(),
        namespace,
        name,
        &vmi_name,
        vm_json.as_ref().unwrap_or(&serde_json::Value::Null),
        vmi_json.as_ref().unwrap_or(&serde_json::Value::Null),
    )
    .await
    {
        Ok(resp) => {
            if resp.mounts.is_empty() {
                r.summary = format!(
                    "Guest agent ran but no mounts were parsed for {name}."
                );
                if let Some(msg) = resp.message {
                    r.evidence.push(msg);
                }
            } else {
                let hot: Vec<_> = resp
                    .mounts
                    .iter()
                    .filter(|m| m.use_percent >= 85.0)
                    .collect();
                r.summary = if hot.is_empty() {
                    format!(
                        "{} mount(s) from guest; no mount above 85% usage.",
                        resp.mounts.len()
                    )
                } else {
                    format!(
                        "{} mount(s) above 85% usage — expand disk or prune data.",
                        hot.len()
                    )
                };
                for m in resp.mounts.iter().take(12) {
                    r.evidence.push(format_mount_evidence(m));
                }
                if !hot.is_empty() {
                    r.recommendations.push(
                        "Snapshot the VM before expanding PVCs or resizing guest partitions.".into(),
                    );
                }
            }
        }
        Err(e) => {
            r.summary = format!("Guest filesystem probe failed: {e}");
            r.recommendations.push(
                "Install qemu-guest-agent (Linux) or QEMU Guest Agent (Windows) and restart the VM.".into(),
            );
        }
    }

    r.actions = vec![
        CopilotAction {
            label: "Open VM detail".into(),
            action: "open_vm".into(),
            page: Some(format!("{namespace}:{name}")),
        },
        CopilotAction {
            label: "Storage Doctor".into(),
            action: "navigate".into(),
            page: Some("storage".into()),
        },
    ];
    r
}
