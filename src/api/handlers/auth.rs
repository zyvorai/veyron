// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Username+password dashboard login and user-account management.
//!
//! `POST /auth/login` is public (see `dashboard_paths::is_auth_exempt_path`);
//! everything under `/auth/users` requires Admin (see
//! `auth_context::min_role_for_route`). Accounts are stored via
//! `crate::kube::user_store` (a single `veyron-users` Secret).

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::kube::user_store;
use crate::kube::user_store::UserAccount;

/// Session token lifetime for username+password logins.
const LOGIN_TOKEN_TTL_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub role: String,
    pub display_name: String,
    pub expires_at: String,
}

/// A user account without its password hash — safe to return from the API.
#[derive(Debug, Clone, Serialize)]
pub struct UserSummary {
    pub username: String,
    pub role: String,
    pub display_name: String,
    pub created_at: String,
}

impl From<UserAccount> for UserSummary {
    fn from(u: UserAccount) -> Self {
        Self {
            username: u.username,
            role: u.role,
            display_name: u.display_name,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetPasswordRequest {
    pub password: String,
}

#[cfg(feature = "web")]
fn err_json(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": code, "message": message.into() })),
    )
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/users", get(list_users).post(create_user))
        .route("/auth/users/:username", delete(delete_user))
        .route("/auth/users/:username/password", post(set_password))
        .with_state(state)
}

/// `POST /api/v1/auth/login` — public. Verifies username+password against the
/// `veyron-users` Secret and, on success, mints a session JWT that every
/// existing auth path (X-API-Key / Bearer / ?token=) already accepts.
#[cfg(feature = "web")]
async fn login(
    State(state): State<SharedState>,
    Json(req): Json<LoginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let s = state.read().await;
    if !s.check_login_rate_limit() {
        return err_json(
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            "Too many login attempts, try again shortly",
        );
    }
    let kube_client = s.kube_client.clone();
    let namespace = s.namespace.clone();
    drop(s);

    let username = req.username.trim();
    if username.is_empty() || req.password.is_empty() {
        return err_json(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "Invalid username or password",
        );
    }

    let user = match kube_client.get_user(&namespace, username).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return err_json(
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Invalid username or password",
            );
        }
        Err(e) => {
            log::error!("login: failed to read veyron-users secret: {e}");
            return err_json(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Failed to verify credentials",
            );
        }
    };

    if !user_store::verify_password(&req.password, &user.password_hash) {
        return err_json(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "Invalid username or password",
        );
    }

    let token = match state
        .read()
        .await
        .issue_jwt(&user.username, &user.role, LOGIN_TOKEN_TTL_SECS)
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("login: failed to issue session token: {e}");
            return err_json(
                StatusCode::SERVICE_UNAVAILABLE,
                "AUTH_NOT_CONFIGURED",
                format!("Username/password login is not fully configured on the server: {e}"),
            );
        }
    };

    let expires_at =
        (chrono::Utc::now() + chrono::Duration::seconds(LOGIN_TOKEN_TTL_SECS)).to_rfc3339();
    (
        StatusCode::OK,
        Json(serde_json::json!(LoginResponse {
            token,
            role: user.role,
            display_name: user.display_name,
            expires_at,
        })),
    )
}

/// `GET /api/v1/auth/users` — Admin only. Never includes password hashes.
#[cfg(feature = "web")]
async fn list_users(State(state): State<SharedState>) -> (StatusCode, Json<serde_json::Value>) {
    let (kube_client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    match kube_client.list_users(&namespace).await {
        Ok(users) => {
            let summaries: Vec<UserSummary> = users.into_iter().map(Into::into).collect();
            (StatusCode::OK, Json(serde_json::json!(summaries)))
        }
        Err(e) => err_json(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            format!("Failed to list users: {e}"),
        ),
    }
}

/// `POST /api/v1/auth/users` — Admin only.
#[cfg(feature = "web")]
async fn create_user(
    State(state): State<SharedState>,
    Json(req): Json<CreateUserRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let (kube_client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    match kube_client
        .create_user(
            &namespace,
            req.username.trim(),
            &req.password,
            req.role.trim(),
            req.display_name.trim(),
        )
        .await
    {
        Ok(()) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "status": "created", "username": req.username })),
        ),
        Err(e) => err_json(StatusCode::BAD_REQUEST, "CREATE_FAILED", e.to_string()),
    }
}

/// `DELETE /api/v1/auth/users/:username` — Admin only.
#[cfg(feature = "web")]
async fn delete_user(
    State(state): State<SharedState>,
    Path(username): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let (kube_client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    match kube_client.delete_user(&namespace, &username).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "deleted", "username": username })),
        ),
        Err(e) => err_json(StatusCode::BAD_REQUEST, "DELETE_FAILED", e.to_string()),
    }
}

/// `POST /api/v1/auth/users/:username/password` — Admin only. Resets a user's password.
#[cfg(feature = "web")]
async fn set_password(
    State(state): State<SharedState>,
    Path(username): Path<String>,
    Json(req): Json<SetPasswordRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let (kube_client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    match kube_client
        .set_user_password(&namespace, &username, &req.password)
        .await
    {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "password_updated", "username": username })),
        ),
        Err(e) => err_json(StatusCode::BAD_REQUEST, "UPDATE_FAILED", e.to_string()),
    }
}
