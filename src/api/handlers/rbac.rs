#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

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
pub fn router() -> Router {
    Router::new()
        .route("/rbac/roles", get(list_roles))
        .route("/rbac/bindings", get(list_bindings))
}

#[cfg(feature = "web")]
async fn list_roles() -> impl IntoResponse {
    not_implemented("RBAC roles")
}

#[cfg(feature = "web")]
async fn list_bindings() -> impl IntoResponse {
    not_implemented("RBAC bindings")
}
