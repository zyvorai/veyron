// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! VMTemplate / VMProfile catalog status and cluster sync.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/catalog/status", get(catalog_status_handler))
        .route("/catalog/sync", post(catalog_sync_handler))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn catalog_status_handler(
    State(state): State<SharedState>,
) -> Json<crate::catalog::CatalogStatus> {
    let s = state.read().await;
    let client = s.client().client();
    drop(s);
    Json(crate::catalog::catalog_status(&client).await)
}

#[cfg(feature = "web")]
async fn catalog_sync_handler(
    State(state): State<SharedState>,
) -> Result<Json<crate::catalog::CatalogSyncResult>, (StatusCode, String)> {
    let s = state.read().await;
    let client = s.client().client();
    drop(s);

    match crate::catalog::sync_embedded_catalog(&client).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("catalog sync failed: {e}"),
        )),
    }
}
