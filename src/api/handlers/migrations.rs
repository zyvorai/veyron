#[cfg(feature = "web")]
use axum::{extract::Path, Json, Router, routing::{delete, get}};
use serde::{Deserialize, Serialize};

/// Migration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResponse {
    pub id: String,
    pub vm_name: String,
    pub source_node: String,
    pub target_node: String,
    pub status: String,
    pub migration_type: String,
    pub progress_percent: u8,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Create migration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMigrationRequest {
    pub vm_name: String,
    pub target_node: Option<String>,
    pub migration_type: Option<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/migrations", get(list_migrations).post(create_migration))
        .route("/migrations/{id}", delete(cancel_migration))
}

#[cfg(feature = "web")]
async fn list_migrations() -> Json<Vec<MigrationResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn create_migration(Json(req): Json<CreateMigrationRequest>) -> Json<Option<MigrationResponse>> {
    let _ = req;
    Json(None)
}

#[cfg(feature = "web")]
async fn cancel_migration(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"cancelled": id}))
}
