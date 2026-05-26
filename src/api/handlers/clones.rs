// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Clone request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneRequest {
    pub source_vm: String,
    pub target_name: String,
    pub namespace: Option<String>,
    pub start_after_clone: bool,
}

/// Clone response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneResponse {
    pub name: String,
    pub source_vm: String,
    pub namespace: String,
    pub status: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/clones", post(create_clone))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn create_clone(
    State(state): State<SharedState>,
    Json(req): Json<CloneRequest>,
) -> Result<Json<CloneResponse>, StatusCode> {
    let s = state.read().await;
    let ns = req.namespace.as_deref().unwrap_or(&s.namespace);

    match s
        .client()
        .clone_vm(ns, &req.source_vm, &req.target_name)
        .await
    {
        Ok(_) => Ok(Json(CloneResponse {
            name: req.target_name,
            source_vm: req.source_vm,
            namespace: ns.to_string(),
            status: "Created".to_string(),
        })),
        Err(e) => {
            log::error!("Failed to clone VM: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
