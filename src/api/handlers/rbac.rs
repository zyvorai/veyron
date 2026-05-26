// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// RBAC role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacRole {
    pub name: String,
    pub namespace: Option<String>,
    pub rules: Vec<RbacRule>,
    pub is_cluster_role: bool,
}

/// RBAC rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacRule {
    pub api_groups: Vec<String>,
    pub resources: Vec<String>,
    pub verbs: Vec<String>,
}

/// RBAC binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacBinding {
    pub name: String,
    pub namespace: Option<String>,
    pub role_name: String,
    pub subjects: Vec<RbacSubject>,
}

/// RBAC subject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacSubject {
    pub kind: String,
    pub name: String,
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/rbac/roles", get(list_roles))
        .route("/rbac/bindings", get(list_bindings))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_roles(State(state): State<SharedState>) -> Json<Vec<RbacRole>> {
    use k8s_openapi::api::rbac::v1::ClusterRole;

    let s = state.read().await;
    let api: kube::api::Api<ClusterRole> = kube::api::Api::all(s.client().client());
    let roles = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<RbacRole> = roles
        .items
        .iter()
        .map(|role| {
            let rules = role
                .rules
                .as_ref()
                .map(|rs| {
                    rs.iter()
                        .map(|r| RbacRule {
                            api_groups: r.api_groups.clone().unwrap_or_default(),
                            resources: r.resources.clone().unwrap_or_default(),
                            verbs: r.verbs.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            RbacRole {
                name: role.metadata.name.clone().unwrap_or_default(),
                namespace: None,
                rules,
                is_cluster_role: true,
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn list_bindings(State(state): State<SharedState>) -> Json<Vec<RbacBinding>> {
    use k8s_openapi::api::rbac::v1::ClusterRoleBinding;

    let s = state.read().await;
    let api: kube::api::Api<ClusterRoleBinding> = kube::api::Api::all(s.client().client());
    let bindings = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<RbacBinding> = bindings
        .items
        .iter()
        .map(|binding| {
            let subjects = binding
                .subjects
                .as_ref()
                .map(|ss| {
                    ss.iter()
                        .map(|s| RbacSubject {
                            kind: s.kind.clone(),
                            name: s.name.clone(),
                            namespace: s.namespace.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            RbacBinding {
                name: binding.metadata.name.clone().unwrap_or_default(),
                namespace: None,
                role_name: binding.role_ref.name.clone(),
                subjects,
            }
        })
        .collect();

    Json(results)
}
