// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;
use crate::kube::guest_filesystem::{collect_guest_filesystem, format_mount_evidence};
use crate::kube::guest_os::detect_guest_os_family;
use crate::kube::types::VirtualMachineInstance;
use crate::kube::windows_rdp::vmi_guest_agent_connected;
use serde_json::to_value;

/// QEMU guest-agent posture and in-guest signals for a running VM.
pub async fn guest_inspector(client: &KubeClient, namespace: &str, name: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Guest Inspector",
        "guest_inspector",
        format!("Guest view: {namespace}/{name}"),
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
        r.summary = format!("{name} is {status} — guest inspector needs a running VMI.");
        r.recommendations
            .push("Start the VM and wait for AgentConnected before guest actions.".into());
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
    let family = detect_guest_os_family(vm_json.as_ref(), vmi_json.as_ref());
    r.evidence
        .push(format!("Guest OS family (heuristic): {}", family.as_str()));

    append_guest_evidence(&mut r, &vmi);

    let agent_up = vmi_json
        .as_ref()
        .is_some_and(|j| vmi_guest_agent_connected(j));
    if agent_up {
        r.evidence
            .push("QEMU guest agent: connected (AgentConnected=True)".into());
        r.recommendations.push(
            "Guest agent is up — freeze/unfreeze, guest-exec RDP, and consistent backups are available.".into(),
        );
        if let Ok(vm_json) = to_value(&vm) {
            if let Ok(resp) = collect_guest_filesystem(
                client.client().clone(),
                namespace,
                name,
                vmi.metadata.name.as_deref().unwrap_or(name),
                &vm_json,
                vmi_json.as_ref().unwrap_or(&serde_json::Value::Null),
            )
            .await
            {
                for m in resp.mounts.iter().take(6) {
                    r.evidence
                        .push(format!("Guest FS: {}", format_mount_evidence(m)));
                }
                if resp.mounts.iter().any(|m| m.use_percent >= 85.0) {
                    r.recommendations.push(
                        "A guest mount is above 85% — expand PVC or prune data; snapshot first."
                            .into(),
                    );
                }
            }
        }
    } else {
        r.evidence.push("QEMU guest agent: not connected".into());
        r.recommendations.push(
            "Install and enable qemu-guest-agent (Linux) or QEMU Guest Agent + Cloudbase-Init (Windows).".into(),
        );
        r.recommendations.push(
            "Restart the VM after installing the agent so KubeVirt reports AgentConnected.".into(),
        );
    }

    if family == crate::kube::guest_os::GuestOsFamily::Windows {
        r.evidence
            .push("Windows guest — prefer RDP over browser VNC for daily use.".into());
        if agent_up {
            r.recommendations.push(
                "Use guest-agent enable-rdp or golden-image sysprep for Remote Desktop.".into(),
            );
        }
    }

    r.summary = if agent_up {
        format!(
            "Guest agent connected for {name}; OS family is {}.",
            family.as_str()
        )
    } else {
        format!("Guest agent not connected for {name} — limited in-guest visibility.")
    };

    r.actions = vec![
        CopilotAction {
            label: "Open VM detail".into(),
            action: "open_vm".into(),
            page: Some(format!("{namespace}:{name}")),
        },
        CopilotAction {
            label: "VMRogue Doctor".into(),
            action: "copilot_doctor".into(),
            page: Some(format!("{namespace}:{name}")),
        },
    ];
    r
}

/// Merge guest-inspector signals into an existing Doctor report.
pub fn enrich_doctor_with_guest(r: &mut CopilotResponse, vmi: &VirtualMachineInstance) {
    append_guest_evidence(r, vmi);
    let vmi_json = to_value(vmi).ok();
    let agent_up = vmi_json
        .as_ref()
        .is_some_and(|j| vmi_guest_agent_connected(j));
    if !agent_up {
        r.recommendations.push(
            "Guest agent not connected — install qemu-guest-agent for metrics and guest-exec."
                .into(),
        );
        if let Some(score) = r.health_score {
            r.health_score = Some(score.saturating_sub(8));
        }
    }
}

fn append_guest_evidence(r: &mut CopilotResponse, vmi: &VirtualMachineInstance) {
    if let Some(os) = vmi.status.as_ref().and_then(|s| s.guest_os_info.as_ref()) {
        if let Some(name) = os.name.as_deref() {
            r.evidence.push(format!("Guest OS name: {name}"));
        }
        if let Some(version) = os.version.as_deref() {
            r.evidence.push(format!("Guest OS version: {version}"));
        }
        if let Some(kernel) = os.kernel_release.as_deref() {
            r.evidence.push(format!("Kernel: {kernel}"));
        }
    }

    if let Some(conds) = vmi.status.as_ref().map(|s| &s.conditions) {
        for c in conds {
            let t = c.type_.as_deref().unwrap_or("?");
            let st = c.status.as_deref().unwrap_or("?");
            if t == "AgentConnected" || t == "Ready" || t == "LiveMigratable" {
                r.evidence.push(format!("Condition {t}: {st}"));
            }
        }
    }

    if let Some(ifaces) = vmi.status.as_ref().map(|s| &s.interfaces) {
        for iface in ifaces.iter().take(4) {
            let name = iface
                .interface_name
                .as_deref()
                .or(iface.name.as_deref())
                .unwrap_or("default");
            let ip = iface
                .ip_address
                .as_deref()
                .or_else(|| iface.ip_addresses.first().map(String::as_str))
                .unwrap_or("—");
            r.evidence.push(format!("Interface {name}: {ip}"));
        }
    }
}
