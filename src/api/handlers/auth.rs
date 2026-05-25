//! Public auth configuration (OIDC discovery for dashboard).

#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new().route("/api/v1/auth/oidc/config", get(oidc_config))
}

#[cfg(feature = "web")]
async fn oidc_config() -> Json<serde_json::Value> {
    Json(crate::api::oidc::oidc_public_config())
}
