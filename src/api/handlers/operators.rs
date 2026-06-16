// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};
#[cfg(feature = "web")]
use k8s_openapi::{
    api::apps::v1::Deployment,
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};

/// Operator response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorResponse {
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub status: String,
    pub managed_resources: Vec<String>,
    pub installed_at: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/operators", get(list_operators))
        .with_state(state)
}

/// Discover operators by listing Deployments with common operator name patterns.
#[cfg(feature = "web")]
async fn list_operators(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<OperatorResponse>> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let client = s.client().client();
    let mut results = Vec::new();
    let crd_api: kube::api::Api<CustomResourceDefinition> = kube::api::Api::all(client.clone());
    let crds = crd_api
        .list(&kube::api::ListParams::default())
        .await
        .map(|l| l.items)
        .unwrap_or_default();

    let namespaces: Vec<String> = if namespace_scope::is_all_namespaces(&scope) {
        let mut ns = vec![
            "kubevirt".to_string(),
            "olm".to_string(),
            "operators".to_string(),
            "veyron-system".to_string(),
        ];
        if let Ok(all) = s.client().list_namespaces().await {
            for n in all.into_iter().filter_map(|n| n.metadata.name) {
                if !ns.contains(&n) {
                    ns.push(n);
                }
            }
        }
        ns
    } else {
        vec![
            "kubevirt".to_string(),
            "olm".to_string(),
            "operators".to_string(),
            "veyron-system".to_string(),
            scope.clone(),
        ]
    };

    for ns in &namespaces {
        let api: kube::api::Api<Deployment> =
            kube::api::Api::namespaced(client.clone(), ns.as_str());
        if let Ok(deploys) = api.list(&kube::api::ListParams::default()).await {
            for d in &deploys.items {
                let name = d.metadata.name.as_deref().unwrap_or("");
                // Heuristic: operators typically have "operator", "controller", or "manager" in name
                if name.contains("operator")
                    || name.contains("controller")
                    || name.contains("manager")
                {
                    let ready = d
                        .status
                        .as_ref()
                        .and_then(|s| s.ready_replicas)
                        .unwrap_or(0);
                    let desired = d.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1);

                    results.push(OperatorResponse {
                        name: name.to_string(),
                        namespace: ns.clone(),
                        version: infer_operator_version(d),
                        status: if ready >= desired {
                            "Running".to_string()
                        } else {
                            "Degraded".to_string()
                        },
                        managed_resources: infer_managed_resources(name, &crds),
                        installed_at: d
                            .metadata
                            .creation_timestamp
                            .as_ref()
                            .map(|t| t.0.to_rfc3339())
                            .unwrap_or_default(),
                    });
                }
            }
        }
    }

    Json(results)
}

#[cfg(feature = "web")]
fn infer_operator_version(deployment: &Deployment) -> String {
    if let Some(labels) = deployment.metadata.labels.as_ref() {
        for key in ["app.kubernetes.io/version", "version"] {
            if let Some(version) = labels.get(key) {
                return version.clone();
            }
        }
    }

    deployment
        .spec
        .as_ref()
        .and_then(|s| s.template.spec.as_ref())
        .and_then(|spec| spec.containers.first())
        .and_then(|container| container.image.as_deref())
        .and_then(extract_image_tag)
        .unwrap_or_default()
}

#[cfg(feature = "web")]
fn extract_image_tag(image: &str) -> Option<String> {
    let without_digest = image.split('@').next().unwrap_or(image);
    let last_segment = without_digest.rsplit('/').next().unwrap_or(without_digest);
    let colon_count = last_segment.matches(':').count();
    if colon_count == 1 {
        return last_segment
            .rsplit_once(':')
            .map(|(_, tag)| tag.to_string())
            .filter(|tag| !tag.is_empty());
    }
    None
}

#[cfg(feature = "web")]
fn infer_managed_resources(operator_name: &str, crds: &[CustomResourceDefinition]) -> Vec<String> {
    let tokens = operator_tokens(operator_name);
    let mut managed = Vec::new();

    for crd in crds {
        let spec = &crd.spec;
        let haystacks = [
            crd.metadata.name.as_deref().unwrap_or(""),
            spec.group.as_str(),
            spec.names.kind.as_str(),
            spec.names.plural.as_str(),
        ];

        if tokens
            .iter()
            .any(|token| haystacks.iter().any(|h| h.to_lowercase().contains(token)))
        {
            managed.push(spec.names.kind.clone());
        }
    }

    managed.sort();
    managed.dedup();
    managed
}

#[cfg(feature = "web")]
fn operator_tokens(name: &str) -> Vec<String> {
    let ignored = [
        "operator",
        "controller",
        "manager",
        "deployment",
        "system",
        "app",
        "cluster",
    ];

    name.split(|c: char| !c.is_ascii_alphanumeric())
        .map(|part| part.to_lowercase())
        .filter(|part| part.len() > 2 && !ignored.contains(&part.as_str()))
        .collect()
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    #[test]
    fn extract_image_tag_reads_container_tag() {
        assert_eq!(
            extract_image_tag("quay.io/example/veyron-operator:v0.2.0"),
            Some("v0.2.0".to_string())
        );
        assert_eq!(
            extract_image_tag("quay.io/example/image@sha256:deadbeef"),
            None
        );
    }

    #[test]
    fn operator_tokens_drops_generic_words() {
        assert_eq!(operator_tokens("veyron-operator-manager"), vec!["veyron"]);
    }
}
