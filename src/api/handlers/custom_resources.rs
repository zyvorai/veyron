// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use kube::{
    Api,
    api::{ApiResource, DynamicObject, GroupVersionKind, ListParams},
};

/// Custom resource response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomResourceResponse {
    pub name: String,
    pub group: String,
    pub version: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub scope: String,
    pub instance_count: u32,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/custom-resources", get(list_custom_resources))
        .with_state(state)
}

/// Discover CRDs installed in the cluster via the apiextensions API.
#[cfg(feature = "web")]
async fn list_custom_resources(
    State(state): State<SharedState>,
) -> Json<Vec<CustomResourceResponse>> {
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;

    let s = state.read().await;
    let client = s.client().client();
    let api: Api<CustomResourceDefinition> = Api::all(client.clone());

    let crds = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let mut results = Vec::with_capacity(crds.items.len());
    for crd in &crds.items {
        let spec = &crd.spec;
        let version = preferred_crd_version(&spec.versions);
        let instance_count = count_crd_instances(
            client.clone(),
            &spec.group,
            &version,
            &spec.names.kind,
            &spec.names.plural,
        )
        .await;

        results.push(CustomResourceResponse {
            name: crd.metadata.name.clone().unwrap_or_default(),
            group: spec.group.clone(),
            version,
            kind: spec.names.kind.clone(),
            namespace: if spec.scope == "Namespaced" {
                Some("*".to_string())
            } else {
                None
            },
            scope: spec.scope.clone(),
            instance_count,
        });
    }

    Json(results)
}

#[cfg(feature = "web")]
fn preferred_crd_version(
    versions: &[k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinitionVersion],
) -> String {
    versions
        .iter()
        .find(|v| v.storage)
        .or_else(|| versions.iter().find(|v| v.served))
        .or_else(|| versions.first())
        .map(|v| v.name.clone())
        .unwrap_or_default()
}

#[cfg(feature = "web")]
async fn count_crd_instances(
    client: kube::Client,
    group: &str,
    version: &str,
    kind: &str,
    plural: &str,
) -> u32 {
    let gvk = GroupVersionKind::gvk(group, version, kind);
    let mut resource = ApiResource::from_gvk(&gvk);
    resource.plural = plural.to_string();
    resource.api_version = format!("{}/{}", group, version);

    let api: Api<DynamicObject> = Api::all_with(client, &resource);
    match api.list(&ListParams::default()).await {
        Ok(list) => list.items.len() as u32,
        Err(_) => 0,
    }
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinitionVersion;

    #[test]
    fn preferred_crd_version_prefers_storage_then_served() {
        let versions = vec![
            CustomResourceDefinitionVersion {
                name: "v1beta1".to_string(),
                served: true,
                storage: false,
                ..Default::default()
            },
            CustomResourceDefinitionVersion {
                name: "v1".to_string(),
                served: true,
                storage: true,
                ..Default::default()
            },
        ];

        assert_eq!(preferred_crd_version(&versions), "v1");
    }
}
