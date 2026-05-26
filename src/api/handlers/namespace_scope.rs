// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Shared `?namespace=` resolution for dashboard and API handlers.
//!
//! Use `namespace=all` for cluster-wide aggregation where supported.

use serde::Deserialize;

/// Standard query parameter used across many GET handlers.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DashboardNamespaceQuery {
    pub namespace: Option<String>,
}

#[inline]
pub fn resolve_opt(namespace: Option<String>, default_ns: &str) -> String {
    namespace
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| default_ns.to_string())
}

#[inline]
pub fn is_all_namespaces(ns: &str) -> bool {
    ns == "all"
}

/// Namespace names suitable for fan-out listing (active namespaces only).
#[cfg(feature = "web")]
pub async fn kubernetes_namespace_names(client: &kube::Client) -> Vec<String> {
    use k8s_openapi::api::core::v1::Namespace;
    use kube::api::{Api, ListParams};
    let ns_api: Api<Namespace> = Api::all(client.clone());
    ns_api
        .list(&ListParams::default())
        .await
        .map(|l| {
            l.items
                .into_iter()
                .filter_map(|n| n.metadata.name)
                .collect()
        })
        .unwrap_or_default()
}

/// List a namespaced resource cluster-wide or in one namespace (`scope` is already resolved).
#[cfg(feature = "web")]
pub async fn list_namespaced_resource<T>(
    client: &kube::Client,
    scope: &str,
) -> Result<Vec<T>, kube::Error>
where
    T: kube::Resource<Scope = kube::core::NamespaceResourceScope, DynamicType = ()>
        + Clone
        + serde::de::DeserializeOwned
        + std::fmt::Debug
        + Send,
{
    use kube::api::{Api, ListParams};
    if is_all_namespaces(scope) {
        let mut out = Vec::new();
        for ns in kubernetes_namespace_names(client).await {
            let api: Api<T> = Api::namespaced(client.clone(), &ns);
            if let Ok(list) = api.list(&ListParams::default()).await {
                out.extend(list.items);
            }
        }
        Ok(out)
    } else {
        let api: Api<T> = Api::namespaced(client.clone(), scope);
        Ok(api.list(&ListParams::default()).await?.items)
    }
}
