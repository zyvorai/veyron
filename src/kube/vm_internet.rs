// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Per-VM internet egress policies (CiliumNetworkPolicy preferred, Kubernetes NetworkPolicy fallback).
//!
//! Selects virt-launcher pods via label `kubevirt.io/vm=<VirtualMachine.metadata.name>`.

use anyhow::{Context, Result};
use kube::{
    Client,
    api::{Api, ApiResource, DeleteParams, DynamicObject, GroupVersionKind, PostParams},
};
use serde::Serialize;
use serde_json::json;

const PREFIX: &str = "veyron-net-";
const MANAGED_BY: &str = "veyron";
const LABEL_MANAGED: &str = "veyron.io/managed-by";
const LABEL_VM_NAME: &str = "veyron.io/vm-name";
const LABEL_EGRESS: &str = "veyron.io/vm-egress";

/// Build a stable, DNS-safe policy name (CNP + NP share this name in the same namespace).
pub fn internet_policy_name(vm_name: &str) -> String {
    let slug: String = vm_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "vm" } else { slug };
    let max_body = 63usize.saturating_sub(PREFIX.len());
    let mut s = slug.to_string();
    if s.len() > max_body {
        s.truncate(max_body);
        while !s.is_empty() && !s.is_char_boundary(s.len()) {
            s.pop();
        }
        while s.ends_with('-') {
            s.pop();
        }
        if s.is_empty() {
            s = "vm".into();
        }
    }
    format!("{PREFIX}{s}")
}

fn cilium_cnp_resource() -> ApiResource {
    let gvk = GroupVersionKind::gvk("cilium.io", "v2", "CiliumNetworkPolicy");
    let mut ar = ApiResource::from_gvk(&gvk);
    ar.plural = "ciliumnetworkpolicies".to_string();
    ar.api_version = "cilium.io/v2".to_string();
    ar
}

fn policy_labels(vm_name: &str) -> serde_json::Value {
    json!({
        LABEL_MANAGED: MANAGED_BY,
        LABEL_VM_NAME: vm_name,
        LABEL_EGRESS: "internet",
    })
}

fn cilium_body(namespace: &str, vm_name: &str, pol_name: &str) -> serde_json::Value {
    json!({
        "apiVersion": "cilium.io/v2",
        "kind": "CiliumNetworkPolicy",
        "metadata": {
            "name": pol_name,
            "namespace": namespace,
            "labels": policy_labels(vm_name),
        },
        "spec": {
            "endpointSelector": {
                "matchLabels": {
                    "kubevirt.io/vm": vm_name
                }
            },
            "egress": [
                { "toEntities": ["all"] }
            ]
        }
    })
}

fn network_policy_body(namespace: &str, vm_name: &str, pol_name: &str) -> serde_json::Value {
    json!({
        "apiVersion": "networking.k8s.io/v1",
        "kind": "NetworkPolicy",
        "metadata": {
            "name": pol_name,
            "namespace": namespace,
            "labels": policy_labels(vm_name),
        },
        "spec": {
            "podSelector": {
                "matchLabels": {
                    "kubevirt.io/vm": vm_name
                }
            },
            "policyTypes": ["Egress"],
            "egress": [ {} ]
        }
    })
}

async fn create_dynamic_np(
    client: &Client,
    ar: &ApiResource,
    namespace: &str,
    body: serde_json::Value,
) -> std::result::Result<(), kube::Error> {
    let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace, ar);
    let obj: DynamicObject = serde_json::from_value(body).map_err(kube::Error::SerdeError)?;
    match api.create(&PostParams::default(), &obj).await {
        Ok(_) => Ok(()),
        Err(kube::Error::Api(ref ae)) if ae.code == 409 => Ok(()),
        Err(e) => Err(e),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VmInternetStatus {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_name: Option<String>,
}

/// Returns whether a Veyron-managed internet policy exists and which backend.
pub async fn vm_internet_status(
    client: &Client,
    namespace: &str,
    vm_name: &str,
) -> VmInternetStatus {
    let pol_name = internet_policy_name(vm_name);
    let cnp_res = cilium_cnp_resource();
    let cnp_api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace, &cnp_res);
    if cnp_api.get(&pol_name).await.is_ok() {
        return VmInternetStatus {
            enabled: true,
            backend: Some("cilium".into()),
            policy_name: Some(pol_name),
        };
    }
    use k8s_openapi::api::networking::v1::NetworkPolicy;
    let np_api: Api<NetworkPolicy> = Api::namespaced(client.clone(), namespace);
    if np_api.get(&pol_name).await.is_ok() {
        return VmInternetStatus {
            enabled: true,
            backend: Some("kubernetes".into()),
            policy_name: Some(pol_name),
        };
    }
    VmInternetStatus {
        enabled: false,
        backend: None,
        policy_name: None,
    }
}

/// Create CiliumNetworkPolicy if the API is available; otherwise Kubernetes NetworkPolicy.
pub async fn ensure_vm_internet_egress(
    client: &Client,
    namespace: &str,
    vm_name: &str,
) -> Result<VmInternetStatus> {
    let pol_name = internet_policy_name(vm_name);
    let cnp_res = cilium_cnp_resource();
    let cnp_body = cilium_body(namespace, vm_name, &pol_name);
    match create_dynamic_np(client, &cnp_res, namespace, cnp_body).await {
        Ok(()) => {
            return Ok(VmInternetStatus {
                enabled: true,
                backend: Some("cilium".into()),
                policy_name: Some(pol_name),
            });
        }
        Err(kube::Error::Api(ref ae)) if ae.code == 409 => {
            return Ok(VmInternetStatus {
                enabled: true,
                backend: Some("cilium".into()),
                policy_name: Some(pol_name),
            });
        }
        Err(e) => {
            log::warn!(
                "CiliumNetworkPolicy not created for {namespace}/{vm_name} (falling back to Kubernetes NetworkPolicy if possible): {e}"
            );
        }
    }

    let gvk = GroupVersionKind::gvk("networking.k8s.io", "v1", "NetworkPolicy");
    let mut np_ar = ApiResource::from_gvk(&gvk);
    np_ar.plural = "networkpolicies".to_string();
    np_ar.api_version = "networking.k8s.io/v1".to_string();
    let np_body = network_policy_body(namespace, vm_name, &pol_name);
    create_dynamic_np(client, &np_ar, namespace, np_body)
        .await
        .with_context(|| {
            format!(
                "failed to create NetworkPolicy {}/{} for VM internet egress",
                namespace, pol_name
            )
        })?;

    Ok(VmInternetStatus {
        enabled: true,
        backend: Some("kubernetes".into()),
        policy_name: Some(pol_name),
    })
}

/// Deletes Veyron-managed CiliumNetworkPolicy and/or NetworkPolicy with the expected name.
pub async fn remove_vm_internet_egress(
    client: &Client,
    namespace: &str,
    vm_name: &str,
) -> Result<()> {
    let pol_name = internet_policy_name(vm_name);

    let cnp_res = cilium_cnp_resource();
    let cnp_api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace, &cnp_res);
    if let Err(e) = cnp_api.delete(&pol_name, &DeleteParams::default()).await {
        if !matches!(&e, kube::Error::Api(ae) if ae.code == 404) {
            return Err(e).with_context(|| {
                format!("failed to delete CiliumNetworkPolicy {namespace}/{pol_name}")
            });
        }
    }

    use k8s_openapi::api::networking::v1::NetworkPolicy;
    let np_api: Api<NetworkPolicy> = Api::namespaced(client.clone(), namespace);
    if let Err(e) = np_api.delete(&pol_name, &DeleteParams::default()).await {
        if !matches!(&e, kube::Error::Api(ae) if ae.code == 404) {
            return Err(e)
                .with_context(|| format!("failed to delete NetworkPolicy {namespace}/{pol_name}"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_name_sanitizes_and_fits() {
        assert_eq!(internet_policy_name("web-01"), "veyron-net-web-01");
        assert!(internet_policy_name("vm").starts_with("veyron-net-"));
        let long = "a".repeat(80);
        let n = internet_policy_name(&long);
        assert!(n.len() <= 63);
    }
}
