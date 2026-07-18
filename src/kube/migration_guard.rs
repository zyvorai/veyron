// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Per-VM live-migration eligibility. Every migrate entry point (single, bulk,
//! REST) consults this before creating a `VirtualMachineInstanceMigration` —
//! previously they fired blindly and a passthrough-GPU VM would sit in a
//! failed migration loop. The physical-device rule is absolute: a VM holding
//! a passthrough GPU or host device can never live-migrate; the device state
//! lives on the physical card. vGPU (mediated) devices can migrate only when
//! the cluster is verified for it (`ClusterMigrationCaps`, Phase 2).

use serde::{Deserialize, Serialize};

use crate::kube::gpu_inventory::{GpuResourceKind, classify_gpu_resource};
use crate::kube::types::{VirtualMachine, VirtualMachineInstance};

/// Cluster-level facts that can relax per-VM rules (probed, never assumed).
#[derive(Debug, Clone, Copy, Default)]
pub struct ClusterMigrationCaps {
    /// True only when the cluster is verified to live-migrate vGPU
    /// (mediated-device) VMIs — licensed NVIDIA vGPU + KubeVirt support.
    pub vgpu_live_migration: bool,
}

impl ClusterMigrationCaps {
    /// Resolve the caps for this cluster. Upstream KubeVirt (≤1.8) does not
    /// live-migrate mdev-attached VMIs, so there is nothing safe to
    /// auto-detect: vGPU migration is enabled only by explicit operator
    /// attestation via `VEYRON_VGPU_LIVE_MIGRATION=1`, set after verifying the
    /// vGPU host-driver + KubeVirt combination actually migrates (Phase 2).
    pub fn from_env() -> Self {
        Self {
            vgpu_live_migration: matches!(
                std::env::var("VEYRON_VGPU_LIVE_MIGRATION").as_deref(),
                Ok("1") | Ok("true") | Ok("yes")
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationBlocker {
    /// Stable machine-readable code, e.g. `GPU_PASSTHROUGH_NOT_MIGRATABLE`.
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationEligibility {
    pub eligible: bool,
    /// Hard blockers — the migration will fail or is impossible.
    pub blockers: Vec<MigrationBlocker>,
    /// Soft signals — migration may work but deserves attention.
    pub warnings: Vec<MigrationBlocker>,
}

/// Cold-move guidance offered whenever live migration is impossible.
const COLD_MOVE_SUGGESTION: &str = "stop the VM, then start it with node placement \
     (scheduling.node_selector or a cordon on the source node) — a passthrough \
     device can only move with a stop/start 'cold move'";

/// Pure eligibility evaluation over the VM spec and (if running) its VMI.
pub fn evaluate_eligibility(
    vm: &VirtualMachine,
    vmi: Option<&VirtualMachineInstance>,
    caps: ClusterMigrationCaps,
) -> MigrationEligibility {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    let devices = vm.spec.template.spec.domain.devices.as_ref();

    // Physical passthrough devices pin the VM to its node — always blocking.
    if let Some(dev) = devices {
        if let Some(hds) = dev.host_devices.as_ref().filter(|h| !h.is_empty()) {
            blockers.push(MigrationBlocker {
                code: "HOST_DEVICE_NOT_MIGRATABLE".into(),
                message: format!(
                    "VM holds {} passthrough host device(s) ({}); live migration is impossible",
                    hds.len(),
                    hds.iter()
                        .map(|h| h.resource_name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                suggestion: Some(COLD_MOVE_SUGGESTION.into()),
            });
        }
        if let Some(gpus) = dev.gpus.as_ref().filter(|g| !g.is_empty()) {
            let all_vgpu = gpus
                .iter()
                .all(|g| classify_gpu_resource(&g.device_name) == GpuResourceKind::Vgpu);
            if all_vgpu && caps.vgpu_live_migration {
                warnings.push(MigrationBlocker {
                    code: "VGPU_MIGRATION_BEST_EFFORT".into(),
                    message: "vGPU live migration is enabled for this cluster; ensure source \
                              and target hosts run identical vGPU host-driver versions"
                        .into(),
                    suggestion: None,
                });
            } else {
                blockers.push(MigrationBlocker {
                    code: "GPU_PASSTHROUGH_NOT_MIGRATABLE".into(),
                    message: format!(
                        "VM holds {} GPU device(s) ({}); a passthrough GPU's state lives on \
                         the physical card and can never live-migrate",
                        gpus.len(),
                        gpus.iter()
                            .map(|g| g.device_name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    suggestion: Some(COLD_MOVE_SUGGESTION.into()),
                });
            }
        }
    }

    match vmi {
        None => blockers.push(MigrationBlocker {
            code: "VM_NOT_RUNNING".into(),
            message: "VM has no running instance to migrate".into(),
            suggestion: Some("start the VM first".into()),
        }),
        Some(vmi) => {
            if let Some(status) = vmi.status.as_ref() {
                if status.phase.as_deref() != Some("Running") {
                    blockers.push(MigrationBlocker {
                        code: "VMI_NOT_RUNNING".into(),
                        message: format!(
                            "VM instance is in phase {} — only Running VMIs migrate",
                            status.phase.as_deref().unwrap_or("Unknown")
                        ),
                        suggestion: None,
                    });
                }
                // KubeVirt's own verdict is authoritative — surface it verbatim.
                for c in &status.conditions {
                    if c.type_.as_deref() == Some("LiveMigratable")
                        && c.status.as_deref() == Some("False")
                    {
                        let already_flagged = blockers.iter().any(|b| {
                            b.code == "GPU_PASSTHROUGH_NOT_MIGRATABLE"
                                || b.code == "HOST_DEVICE_NOT_MIGRATABLE"
                        }) && c.reason.as_deref()
                            == Some("HostDeviceNotLiveMigratable");
                        if !already_flagged {
                            blockers.push(MigrationBlocker {
                                code: format!(
                                    "KUBEVIRT_{}",
                                    c.reason.as_deref().unwrap_or("NOT_LIVE_MIGRATABLE")
                                ),
                                message: c.message.clone().unwrap_or_else(|| {
                                    "KubeVirt reports this VMI is not live-migratable".into()
                                }),
                                suggestion: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Ephemeral-only storage survives a migration but the disk contents are
    // node-local scratch — worth a heads-up, not a hard block.
    let volumes = vm.spec.template.spec.volumes.as_deref().unwrap_or(&[]);
    let has_persistent = volumes
        .iter()
        .any(|v| v.persistent_volume_claim.is_some() || v.data_volume.is_some());
    if !volumes.is_empty() && !has_persistent {
        warnings.push(MigrationBlocker {
            code: "EPHEMERAL_STORAGE_ONLY".into(),
            message: "VM uses only container/ephemeral disks; nothing persists across a \
                      migration beyond memory state"
                .into(),
            suggestion: None,
        });
    }

    MigrationEligibility {
        eligible: blockers.is_empty(),
        blockers,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VMConfigBuilder;
    use crate::kube::converter::vm_config_to_kubevirt;
    use crate::kube::types::{VirtualMachineInstanceStatus, VmiCondition, VmiSpec};
    use kube::core::ObjectMeta;

    fn running_vmi() -> VirtualMachineInstance {
        VirtualMachineInstance {
            metadata: ObjectMeta::default(),
            spec: VmiSpec {
                extra: Default::default(),
            },
            status: Some(VirtualMachineInstanceStatus {
                phase: Some("Running".into()),
                ..Default::default()
            }),
        }
    }

    fn base_vm(gpu: Option<&str>) -> VirtualMachine {
        let mut b = VMConfigBuilder::new("vm")
            .namespace("default")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_blank_disk("rootdisk", "10Gi", 1)
            .add_pod_network("default");
        if let Some(resource) = gpu {
            b = b.add_gpu("gpu0", resource);
        }
        vm_config_to_kubevirt(&b.build()).unwrap()
    }

    #[test]
    fn clean_running_vm_is_eligible() {
        let vm = base_vm(None);
        let vmi = running_vmi();
        let e = evaluate_eligibility(&vm, Some(&vmi), ClusterMigrationCaps::default());
        assert!(e.eligible, "blockers: {:?}", e.blockers);
    }

    #[test]
    fn passthrough_gpu_blocks_migration() {
        let vm = base_vm(Some("nvidia.com/gpu"));
        let vmi = running_vmi();
        let e = evaluate_eligibility(&vm, Some(&vmi), ClusterMigrationCaps::default());
        assert!(!e.eligible);
        assert_eq!(e.blockers[0].code, "GPU_PASSTHROUGH_NOT_MIGRATABLE");
        assert!(
            e.blockers[0]
                .suggestion
                .as_deref()
                .unwrap()
                .contains("cold move")
        );
    }

    #[test]
    fn vgpu_migrates_only_when_cluster_verified() {
        let vm = base_vm(Some("nvidia.com/GRID_T4-2Q"));
        let vmi = running_vmi();
        let denied = evaluate_eligibility(&vm, Some(&vmi), ClusterMigrationCaps::default());
        assert!(!denied.eligible);
        let allowed = evaluate_eligibility(
            &vm,
            Some(&vmi),
            ClusterMigrationCaps {
                vgpu_live_migration: true,
            },
        );
        assert!(allowed.eligible, "blockers: {:?}", allowed.blockers);
        assert_eq!(allowed.warnings[0].code, "VGPU_MIGRATION_BEST_EFFORT");
    }

    #[test]
    fn stopped_vm_blocks_migration() {
        let vm = base_vm(None);
        let e = evaluate_eligibility(&vm, None, ClusterMigrationCaps::default());
        assert!(!e.eligible);
        assert_eq!(e.blockers[0].code, "VM_NOT_RUNNING");
    }

    #[test]
    fn kubevirt_condition_reason_is_surfaced() {
        let vm = base_vm(None);
        let mut vmi = running_vmi();
        vmi.status.as_mut().unwrap().conditions = vec![VmiCondition {
            type_: Some("LiveMigratable".into()),
            status: Some("False".into()),
            reason: Some("DisksNotLiveMigratable".into()),
            message: Some("cannot migrate VMI: PVC rootdisk is not shared".into()),
        }];
        let e = evaluate_eligibility(&vm, Some(&vmi), ClusterMigrationCaps::default());
        assert!(!e.eligible);
        assert_eq!(e.blockers[0].code, "KUBEVIRT_DisksNotLiveMigratable");
        assert!(e.blockers[0].message.contains("not shared"));
    }

    #[test]
    fn gpu_blocker_not_duplicated_by_kubevirt_condition() {
        let vm = base_vm(Some("nvidia.com/gpu"));
        let mut vmi = running_vmi();
        vmi.status.as_mut().unwrap().conditions = vec![VmiCondition {
            type_: Some("LiveMigratable".into()),
            status: Some("False".into()),
            reason: Some("HostDeviceNotLiveMigratable".into()),
            message: Some("cannot migrate VMI which has host devices".into()),
        }];
        let e = evaluate_eligibility(&vm, Some(&vmi), ClusterMigrationCaps::default());
        assert_eq!(e.blockers.len(), 1);
        assert_eq!(e.blockers[0].code, "GPU_PASSTHROUGH_NOT_MIGRATABLE");
    }
}
