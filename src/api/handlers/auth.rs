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
    Extension, Json, Router,
    extract::{ConnectInfo, Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::auth_context::AuthContext;
#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::api::key_store::{CreateSpec, KeyError, KeyStore, Pepper};
#[cfg(feature = "web")]
use crate::kube::api_key_store::KubeKeyBackend;
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
        .route("/auth/api-keys", get(list_api_keys).post(create_api_key))
        .route("/auth/api-keys/:id", delete(revoke_api_key))
        .with_state(state)
}

/// `POST /api/v1/auth/login` — public. Verifies username+password against the
/// `veyron-users` Secret and, on success, mints a session JWT that every
/// existing auth path (X-API-Key / Bearer / ?token=) already accepts.
#[cfg(feature = "web")]
async fn login(
    State(state): State<SharedState>,
    peer: Option<ConnectInfo<std::net::SocketAddr>>,
    Json(req): Json<LoginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let s = state.read().await;
    // Peer address only: X-Forwarded-For is client-controlled and not trusted.
    let client = peer
        .map(|ConnectInfo(a)| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    if !s.check_login_rate_limit(&client, &req.username) {
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

    let token = match state.read().await.issue_jwt(
        &user.username,
        &user.role,
        LOGIN_TOKEN_TTL_SECS,
        user.token_version,
    ) {
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

#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default = "default_key_role")]
    pub role: String,
    /// Namespaces the key may act in; empty or `["*"]` means no restriction.
    #[serde(default)]
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

fn default_key_role() -> String {
    "readonly".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListApiKeysQuery {
    #[serde(default)]
    pub include_inactive: bool,
}

#[cfg(feature = "web")]
fn key_err(e: KeyError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, code) = match &e {
        KeyError::Invalid(_) => (StatusCode::BAD_REQUEST, "INVALID_REQUEST"),
        KeyError::Duplicate(_) | KeyError::Conflict => (StatusCode::CONFLICT, "CONFLICT"),
        KeyError::Full => (StatusCode::CONFLICT, "KEY_STORE_FULL"),
        KeyError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
        KeyError::NoPepper => (StatusCode::SERVICE_UNAVAILABLE, "KEY_STORE_NOT_CONFIGURED"),
        KeyError::Backend(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
    };
    // Backend errors can carry cluster detail; log it and return a generic message.
    let msg = if let KeyError::Backend(d) = &e {
        log::error!("api key store: {d}");
        "API key store unavailable".to_string()
    } else {
        e.to_string()
    };
    err_json(status, code, msg)
}

/// Audit a key lifecycle event: the key's id and name only, never the token or its hash.
#[cfg(feature = "web")]
async fn audit_key_event(
    client: &crate::kube::KubeClient,
    namespace: &str,
    actor: &str,
    action: &str,
    key_id: &str,
    key_name: &str,
) {
    log::info!("audit: {actor} {action} api key id={key_id} name={key_name}");
    let mut ev = crate::soc::event::SecurityEvent::new(
        "audit",
        "medium",
        "veyron",
        format!("{action} api key {key_name} ({key_id})"),
    )
    .with_actor(actor)
    .with_action(action)
    .with_target(format!("api-key/{key_id}"))
    .with_outcome("success");
    ev.labels.insert("key_name".into(), key_name.into());
    crate::soc::store::append_events(&client.client(), namespace, vec![ev]).await;
}

#[cfg(feature = "web")]
fn key_store_for(client: &crate::kube::KubeClient, namespace: &str) -> KeyStore<KubeKeyBackend> {
    KeyStore::new(KubeKeyBackend::new(client, namespace), Pepper::from_env())
}

/// `GET /api/v1/auth/api-keys` — Admin only. Metadata only; never a token, salt or hash.
#[cfg(feature = "web")]
async fn list_api_keys(
    State(state): State<SharedState>,
    Query(q): Query<ListApiKeysQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    let (client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    match key_store_for(&client, &namespace)
        .list(q.include_inactive, chrono::Utc::now())
        .await
    {
        Ok(keys) => (StatusCode::OK, Json(serde_json::json!(keys))),
        Err(e) => key_err(e),
    }
}

/// `POST /api/v1/auth/api-keys` — Admin only. The response carries the plaintext key once;
/// it cannot be retrieved again.
#[cfg(feature = "web")]
async fn create_api_key(
    State(state): State<SharedState>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CreateApiKeyRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let (client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let spec = CreateSpec {
        name: req.name,
        role: req.role.trim().to_lowercase(),
        scopes: req.scopes,
        expires_in_days: req.expires_in_days,
    };
    match key_store_for(&client, &namespace)
        .create(spec, &ctx.subject, chrono::Utc::now())
        .await
    {
        Ok((summary, token)) => {
            audit_key_event(
                &client,
                &namespace,
                &ctx.subject,
                "create",
                &summary.id,
                &summary.name,
            )
            .await;
            let mut body = serde_json::json!(summary);
            body["key"] = serde_json::Value::String(token);
            body["note"] = "Store this key now; it is not shown again.".into();
            (StatusCode::CREATED, Json(body))
        }
        Err(e) => key_err(e),
    }
}

/// `DELETE /api/v1/auth/api-keys/:id` — Admin only. `:id` may also be the key's name.
#[cfg(feature = "web")]
async fn revoke_api_key(
    State(state): State<SharedState>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let (client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    match key_store_for(&client, &namespace)
        .revoke(&id, &ctx.subject, chrono::Utc::now())
        .await
    {
        Ok(summary) => {
            audit_key_event(
                &client,
                &namespace,
                &ctx.subject,
                "revoke",
                &summary.id,
                &summary.name,
            )
            .await;
            (StatusCode::OK, Json(serde_json::json!(summary)))
        }
        Err(e) => key_err(e),
    }
}
