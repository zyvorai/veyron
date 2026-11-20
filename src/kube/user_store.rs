// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! User accounts for username+password dashboard login.
//!
//! Accounts live as a single JSON blob inside one Secret (`veyron-users`,
//! key `users.json`) rather than one Secret per user, so list/create/delete
//! stay atomic. Passwords are Argon2id-hashed, never stored or returned
//! in plaintext.

use super::KubeClient;
use anyhow::{Context, Result};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{Patch, PatchParams, PostParams};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const USERS_SECRET_NAME: &str = "veyron-users";
const USERS_SECRET_KEY: &str = "users.json";
/// Lab default console password for the bootstrap admin (matches Netra's admin / Admin@321).
pub const DEFAULT_ADMIN_PASSWORD: &str = "Admin@321";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub username: String,
    pub password_hash: String,
    pub role: String, // "admin" | "write" | "readonly"
    pub display_name: String,
    pub created_at: String,
    /// Bumped on password reset; session JWTs carry the value they were minted
    /// with and are rejected once it no longer matches (see [`session_valid`]).
    #[serde(default)]
    pub token_version: u64,
}

/// Whether a login-minted session token (`sub`, `tv` claims) is still valid:
/// the user must still exist and the token version must be current.
pub fn session_valid(users: &[UserAccount], sub: &str, token_version: u64) -> bool {
    users
        .iter()
        .any(|u| u.username == sub && u.token_version == token_version)
}

/// Short-lived cache of the user list so per-request session checks do not
/// read the Secret every time. Cleared on every write; stale by at most the TTL
/// across API replicas.
static USER_CACHE: std::sync::Mutex<Option<(std::time::Instant, Vec<UserAccount>)>> =
    std::sync::Mutex::new(None);
const USER_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(5);

fn clear_user_cache() {
    *USER_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Hash a plaintext password with Argon2id (library default, OWASP-recommended params).
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("failed to hash password: {e}"))
}

/// Verify a plaintext password against a stored Argon2 hash. Never panics on a malformed hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Username rules shared with PAM-style validation elsewhere in the codebase.
pub fn validate_username(username: &str) -> bool {
    crate::api::pam_auth::validate_username(username)
}

/// Recognized dashboard roles for a user account.
pub fn validate_role(role: &str) -> bool {
    matches!(role, "admin" | "write" | "readonly")
}

impl KubeClient {
    /// List all user accounts (namespace = the API's operating namespace).
    pub async fn list_users(&self, namespace: &str) -> Result<Vec<UserAccount>> {
        let secrets: kube::Api<Secret> = kube::Api::namespaced(self.client.clone(), namespace);
        match secrets.get(USERS_SECRET_NAME).await {
            Ok(secret) => Ok(decode_users(&secret)),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(Vec::new()),
            Err(e) => Err(e).context("read veyron-users secret"),
        }
    }

    /// Find one user account by username (case-sensitive).
    pub async fn get_user(&self, namespace: &str, username: &str) -> Result<Option<UserAccount>> {
        Ok(self
            .list_users(namespace)
            .await?
            .into_iter()
            .find(|u| u.username == username))
    }

    /// Create a new user account. Fails if the username already exists.
    pub async fn create_user(
        &self,
        namespace: &str,
        username: &str,
        password: &str,
        role: &str,
        display_name: &str,
    ) -> Result<()> {
        if !validate_username(username) {
            anyhow::bail!("invalid username '{username}'");
        }
        if !validate_role(role) {
            anyhow::bail!("invalid role '{role}' (expected admin, write, or readonly)");
        }
        if password.len() < 8 {
            anyhow::bail!("password must be at least 8 characters");
        }

        let mut users = self.list_users(namespace).await?;
        if users.iter().any(|u| u.username == username) {
            anyhow::bail!("user '{username}' already exists");
        }
        users.push(UserAccount {
            username: username.to_string(),
            password_hash: hash_password(password)?,
            role: role.to_string(),
            display_name: if display_name.is_empty() {
                username.to_string()
            } else {
                display_name.to_string()
            },
            created_at: chrono::Utc::now().to_rfc3339(),
            token_version: 0,
        });
        self.write_users(namespace, &users).await
    }

    /// Reset an existing user's password.
    pub async fn set_user_password(
        &self,
        namespace: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        if password.len() < 8 {
            anyhow::bail!("password must be at least 8 characters");
        }
        let mut users = self.list_users(namespace).await?;
        let user = users
            .iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| anyhow::anyhow!("user '{username}' not found"))?;
        user.password_hash = hash_password(password)?;
        // Invalidate every session token issued before this reset.
        user.token_version = user.token_version.wrapping_add(1);
        self.write_users(namespace, &users).await
    }

    /// Delete a user account. No-op (Ok) if the username doesn't exist.
    pub async fn delete_user(&self, namespace: &str, username: &str) -> Result<()> {
        let mut users = self.list_users(namespace).await?;
        let before = users.len();
        users.retain(|u| u.username != username);
        if users.len() == before {
            return Ok(());
        }
        self.write_users(namespace, &users).await
    }

    /// Whether a login-minted session token is still valid (user exists, token
    /// version current). Fails closed when the user list cannot be read and no
    /// cached copy exists.
    pub async fn session_token_valid(&self, namespace: &str, sub: &str, tv: u64) -> bool {
        {
            let cache = USER_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((at, users)) = cache.as_ref() {
                if at.elapsed() < USER_CACHE_TTL {
                    return session_valid(users, sub, tv);
                }
            }
        }
        match self.list_users(namespace).await {
            Ok(users) => {
                let ok = session_valid(&users, sub, tv);
                *USER_CACHE.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some((std::time::Instant::now(), users));
                ok
            }
            Err(e) => {
                log::warn!("session check: could not read users: {e}");
                let cache = USER_CACHE.lock().unwrap_or_else(|e| e.into_inner());
                cache
                    .as_ref()
                    .is_some_and(|(_, users)| session_valid(users, sub, tv))
            }
        }
    }

    async fn write_users(&self, namespace: &str, users: &[UserAccount]) -> Result<()> {
        clear_user_cache();
        let secrets: kube::Api<Secret> = kube::Api::namespaced(self.client.clone(), namespace);
        let body = serde_json::to_string(users).context("serialize veyron-users")?;
        let mut data = BTreeMap::new();
        data.insert(USERS_SECRET_KEY.to_string(), body.clone());

        if secrets.get(USERS_SECRET_NAME).await.is_ok() {
            let patch = serde_json::json!({ "stringData": { USERS_SECRET_KEY: body } });
            secrets
                .patch(
                    USERS_SECRET_NAME,
                    &PatchParams::default(),
                    &Patch::Merge(patch),
                )
                .await
                .context("update veyron-users secret")?;
        } else {
            let secret = Secret {
                metadata: ObjectMeta {
                    name: Some(USERS_SECRET_NAME.to_string()),
                    namespace: Some(namespace.to_string()),
                    labels: Some(BTreeMap::from([(
                        "veyron.io/managed-by".to_string(),
                        "veyron".to_string(),
                    )])),
                    ..Default::default()
                },
                string_data: Some(data),
                ..Default::default()
            };
            secrets
                .create(&PostParams::default(), &secret)
                .await
                .context("create veyron-users secret")?;
        }
        Ok(())
    }
}

/// Ensure a dashboard login account exists so `POST /api/v1/auth/login` works
/// after a fresh deploy.
///
/// Controlled by env (all optional):
/// - `VEYRON_BOOTSTRAP_ADMIN` — set to `0`/`false` to skip (default: on)
/// - `VEYRON_BOOTSTRAP_ADMIN_USER` — default `admin`
/// - `VEYRON_BOOTSTRAP_ADMIN_PASSWORD` — default `VEYRON_API_KEY`, else a random password
///   logged once at `warn` (there is no fixed fallback)
/// - `VEYRON_BOOTSTRAP_ADMIN_SYNC` — set to `1` to reset password when user exists
///
/// Requires `VEYRON_JWT_SECRET` + `VEYRON_JWT_ISSUER` (login mints a JWT).
pub async fn bootstrap_dashboard_admin(client: &KubeClient, namespace: &str) {
    let disabled = std::env::var("VEYRON_BOOTSTRAP_ADMIN")
        .ok()
        .map(|v| v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("no"))
        .unwrap_or(false);
    if disabled {
        return;
    }

    let jwt_secret = std::env::var("VEYRON_JWT_SECRET").unwrap_or_default();
    let jwt_issuer = std::env::var("VEYRON_JWT_ISSUER").unwrap_or_default();
    if jwt_secret.trim().is_empty() || jwt_issuer.trim().is_empty() {
        log::warn!(
            "dashboard login bootstrap skipped: set VEYRON_JWT_SECRET and VEYRON_JWT_ISSUER (deploy scripts create these on veyron-api-key Secret)"
        );
        return;
    }

    let username =
        std::env::var("VEYRON_BOOTSTRAP_ADMIN_USER").unwrap_or_else(|_| "admin".to_string());
    let configured = std::env::var("VEYRON_BOOTSTRAP_ADMIN_PASSWORD")
        .or_else(|_| std::env::var("VEYRON_API_KEY"))
        .ok()
        .filter(|p| !p.trim().is_empty());
    let defaulted = configured.is_none();
    let password = configured.unwrap_or_else(|| DEFAULT_ADMIN_PASSWORD.to_string());
    let sync = std::env::var("VEYRON_BOOTSTRAP_ADMIN_SYNC")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    match client.get_user(namespace, &username).await {
        Ok(None) => match client
            .create_user(namespace, &username, &password, "admin", "Administrator")
            .await
        {
            Ok(()) if defaulted => log::warn!(
                "bootstrapped dashboard login user '{username}' with the default password — set VEYRON_BOOTSTRAP_ADMIN_PASSWORD for production"
            ),
            Ok(()) => log::info!(
                "bootstrapped dashboard login user '{username}' (password from VEYRON_BOOTSTRAP_ADMIN_PASSWORD / VEYRON_API_KEY)"
            ),
            Err(e) => log::error!("dashboard login bootstrap failed creating '{username}': {e}"),
        },
        Ok(Some(_)) if sync && !defaulted => match client
            .set_user_password(namespace, &username, &password)
            .await
        {
            Ok(()) => log::info!(
                "synced dashboard login password for '{username}' (VEYRON_BOOTSTRAP_ADMIN_SYNC=1)"
            ),
            Err(e) => log::error!("dashboard login bootstrap failed syncing '{username}': {e}"),
        },
        Ok(Some(_)) => {
            log::debug!("dashboard login user '{username}' already present — bootstrap skipped");
        }
        Err(e) => log::warn!("dashboard login bootstrap: could not read users: {e}"),
    }
}

fn decode_users(secret: &Secret) -> Vec<UserAccount> {
    let raw = secret
        .data
        .as_ref()
        .and_then(|d| d.get(USERS_SECRET_KEY))
        .map(|b| b.0.clone())
        .and_then(|bytes| String::from_utf8(bytes).ok());
    // `data` (base64) is what the API server returns on read even though we
    // write via `string_data`; fall back to `string_data` for a freshly
    // constructed (not-yet-round-tripped) object, e.g. in tests.
    let raw = raw.or_else(|| {
        secret
            .string_data
            .as_ref()
            .and_then(|d| d.get(USERS_SECRET_KEY))
            .cloned()
    });
    raw.and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acct(name: &str, tv: u64) -> UserAccount {
        UserAccount {
            username: name.into(),
            password_hash: String::new(),
            role: "admin".into(),
            display_name: name.into(),
            created_at: String::new(),
            token_version: tv,
        }
    }

    #[test]
    fn session_revoked_on_delete_and_password_reset() {
        let users = vec![acct("alice", 2)];
        assert!(session_valid(&users, "alice", 2));
        assert!(!session_valid(&users, "alice", 1), "reset bumps version");
        assert!(!session_valid(&users, "bob", 0), "deleted/unknown user");
        assert!(!session_valid(&[], "alice", 2));
    }

    #[test]
    fn legacy_user_records_default_token_version_zero() {
        let u: UserAccount = serde_json::from_str(
            r#"{"username":"a","password_hash":"x","role":"admin","display_name":"a","created_at":"t"}"#,
        )
        .unwrap();
        assert_eq!(u.token_version, 0);
    }

    #[test]
    fn hash_and_verify_round_trip() {
        let hash = hash_password("correct-horse-battery-staple").unwrap();
        assert!(verify_password("correct-horse-battery-staple", &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn verify_rejects_malformed_hash() {
        assert!(!verify_password("anything", "not-a-real-hash"));
    }

    #[test]
    fn role_validation() {
        assert!(validate_role("admin"));
        assert!(validate_role("write"));
        assert!(validate_role("readonly"));
        assert!(!validate_role("superuser"));
    }
}
