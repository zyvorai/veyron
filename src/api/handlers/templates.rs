#[cfg(feature = "web")]
use axum::{extract::Path, Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Template response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateResponse {
    pub name: String,
    pub description: String,
    pub os_type: String,
    pub default_cpus: u32,
    pub default_memory: String,
    pub default_disk_size: String,
    pub tags: Vec<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/templates", get(list_templates))
        .route("/templates/{name}", get(get_template))
}

#[cfg(feature = "web")]
async fn list_templates() -> Json<Vec<TemplateResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn get_template(Path(name): Path<String>) -> Json<Option<TemplateResponse>> {
    let _ = name;
    Json(None)
}
