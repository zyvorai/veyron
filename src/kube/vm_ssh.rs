// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Inject SSH public keys into KubeVirt VirtualMachine accessCredentials.

use super::KubeClient;
use anyhow::{Context, Result};
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{Patch, PatchParams, PostParams};
use std::collections::BTreeMap;

/// How the guest receives the SSH public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshPropagation {
    QemuGuestAgent,
    ConfigDrive,
}

impl SshPropagation {
    pub fn propagation_json(self, user: &str) -> serde_json::Value {
        match self {
            Self::QemuGuestAgent => serde_json::json!({
                "qemuGuestAgent": {
                    "users": [user]
                }
            }),
            Self::ConfigDrive => serde_json::json!({
                "configDrive": {}
            }),
        }
    }
}

impl KubeClient {
    /// Create or update a Secret and merge an `accessCredentials` SSH entry on the VM.
    pub async fn inject_ssh_public_key(
        &self,
        namespace: &str,
        vm_name: &str,
        public_key: &str,
        secret_name: &str,
        guest_user: &str,
        propagation: SshPropagation,
    ) -> Result<()> {
        let key = public_key.trim();
        if key.is_empty() {
            anyhow::bail!("public key is empty");
        }
        if !key.starts_with("ssh-") {
            anyhow::bail!("public key must start with ssh- (e.g. ssh-ed25519)");
        }

        let mut data = BTreeMap::new();
        data.insert("ssh-publickey".to_string(), key.to_string());

        let secrets: kube::Api<Secret> = kube::Api::namespaced(self.client.clone(), namespace);
        let secret = Secret {
            metadata: ObjectMeta {
                name: Some(secret_name.to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some(BTreeMap::from([(
                    "vmrogue.io/managed-by".to_string(),
                    "vmrogue".to_string(),
                )])),
                ..Default::default()
            },
            string_data: Some(data),
            ..Default::default()
        };

        if secrets.get(secret_name).await.is_ok() {
            let patch = serde_json::json!({
                "stringData": {
                    "ssh-publickey": key
                }
            });
            secrets
                .patch(secret_name, &PatchParams::default(), &Patch::Merge(patch))
                .await
                .context("update SSH secret")?;
        } else {
            secrets
                .create(&PostParams::default(), &secret)
                .await
                .context("create SSH secret")?;
        }

        let credential = serde_json::json!({
            "sshPublicKey": {
                "propagationMethod": propagation.propagation_json(guest_user),
                "source": {
                    "secret": {
                        "secretName": secret_name
                    }
                }
            }
        });

        let vm = self.get_vm(namespace, vm_name).await?;
        let mut existing: Vec<serde_json::Value> = vm
            .spec
            .template
            .spec
            .access_credentials
            .clone()
            .unwrap_or_default();

        existing.retain(|entry| {
            entry
                .pointer("/sshPublicKey/source/secret/secretName")
                .and_then(|v| v.as_str())
                != Some(secret_name)
        });
        existing.push(credential);

        let patch_body = serde_json::json!({
            "spec": {
                "template": {
                    "spec": {
                        "accessCredentials": existing
                    }
                }
            }
        });

        let vms = self.vm_api(namespace);
        vms.patch(vm_name, &PatchParams::default(), &Patch::Merge(patch_body))
            .await
            .context("patch VM accessCredentials")?;

        Ok(())
    }
}
