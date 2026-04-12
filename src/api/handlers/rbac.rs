#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
async fn list_roles() -> Json<Vec<RbacRole>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn list_bindings() -> Json<Vec<RbacBinding>> {
    Json(vec![])
}
