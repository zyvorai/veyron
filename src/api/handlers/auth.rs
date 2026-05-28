// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Public auth configuration (OIDC discovery for dashboard).

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/api/v1/auth/oidc/config", get(oidc_config))
        .route("/api/v1/auth/oidc/token", post(oidc_token_exchange))
}

#[cfg(feature = "web")]
async fn oidc_config() -> Json<serde_json::Value> {
    Json(crate::api::oidc::oidc_public_config())
}

#[cfg(feature = "web")]
async fn oidc_token_exchange(
    Json(body): Json<crate::api::oidc::OidcTokenExchangeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match crate::api::oidc::exchange_oidc_authorization_code(body).await {
        Ok(json) => Ok(Json(json)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}
