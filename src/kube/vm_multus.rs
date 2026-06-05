// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Attach Multus NetworkAttachmentDefinitions to running VM specs.

use super::KubeClient;
use super::types::{Devices, Interface, MultusNetwork, Network, VirtualMachine};
use anyhow::{Context, Result};
use kube::api::{Patch, PatchParams};
use std::collections::BTreeMap;

impl KubeClient {
    /// Append a Multus secondary interface + network to an existing VirtualMachine (VM should be stopped for safest apply).
    pub async fn attach_multus_nad(
        &self,
        namespace: &str,
        vm_name: &str,
        nad_namespace: &str,
        nad_name: &str,
        interface_name: Option<&str>,
    ) -> Result<(VirtualMachine, String)> {
        let mut vm = self.get_vm(namespace, vm_name).await?;
        let spec = &mut vm.spec.template.spec;

        let networks = spec.networks.get_or_insert_with(Vec::new);
        if spec.domain.devices.is_none() {
            spec.domain.devices = Some(Devices {
                disks: None,
                interfaces: Some(Vec::new()),
                tpm: None,
                rng: None,
                inputs: None,
                host_devices: None,
                watchdog: None,
                autoattach_graphics_device: None,
                autoattach_mem_balloon: None,
                network_interface_multiqueue: None,
                video: None,
                ..Default::default()
            });
        }
        let devices = spec.domain.devices.as_mut().unwrap();
        let interfaces = devices.interfaces.get_or_insert_with(Vec::new);

        let multus_network_name = if nad_namespace == namespace || nad_namespace.is_empty() {
            nad_name.to_string()
        } else {
            format!("{nad_namespace}/{nad_name}")
        };

        let iface_name = interface_name
            .map(str::to_string)
            .unwrap_or_else(|| format!("net{}", interfaces.len().max(1)));

        if networks.iter().any(|n| n.name == iface_name) {
            anyhow::bail!(
                "interface '{}' already exists on VM {}/{}",
                iface_name,
                namespace,
                vm_name
            );
        }

        let iface_name_for_return = iface_name.clone();
        interfaces.push(Interface {
            name: iface_name,
            model: Some("virtio".to_string()),
            mac_address: None,
            masquerade: None,
            bridge: Some(BTreeMap::new()),
            sriov: None,
            ports: None,
            boot_order: None,
        });

        networks.push(Network {
            name: iface_name_for_return.clone(),
            pod: None,
            multus: Some(MultusNetwork {
                network_name: multus_network_name,
            }),
        });

        let patch_body = serde_json::json!({
            "spec": {
                "template": {
                    "spec": {
                        "networks": networks,
                        "domain": {
                            "devices": {
                                "interfaces": interfaces
                            }
                        }
                    }
                }
            }
        });

        let vms = self.vm_api(namespace);
        let pp = PatchParams::default();
        let patched = vms
            .patch(vm_name, &pp, &Patch::Merge(patch_body))
            .await
            .with_context(|| {
                format!("patch VM {namespace}/{vm_name} with Multus NAD {nad_namespace}/{nad_name}")
            })?;
        Ok((patched, iface_name_for_return))
    }
}
