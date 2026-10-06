// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Runs API calls in-process on behalf of a caller.
//!
//! AI tools and approved proposals go through the real router (auth, RBAC, audit,
//! validation) instead of calling handlers directly. Each role gets an ephemeral
//! internal key minted at startup; the original caller travels in a header so the
//! audit trail shows "alice (via Veyron AI)" rather than a service identity.

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request};
use once_cell::sync::{Lazy, OnceCell};
use serde_json::Value;
use tower::ServiceExt;

use crate::api::http_server::web::ApiRole;

pub const INTERNAL_KEY_PREFIX: &str = "ai-internal-";
pub const ON_BEHALF_HEADER: &str = "x-veyron-on-behalf-of";

struct InternalKeys {
    admin: String,
    write: String,
    readonly: String,
}

static KEYS: Lazy<InternalKeys> = Lazy::new(|| InternalKeys {
    admin: random_key(),
    write: random_key(),
    readonly: random_key(),
});

static ROUTER: OnceCell<Router> = OnceCell::new();

fn random_key() -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

/// `(name, key, role)` entries the server registers alongside user API keys.
pub fn internal_keys() -> Vec<(String, String, ApiRole)> {
    vec![
        (
            format!("{INTERNAL_KEY_PREFIX}admin"),
            KEYS.admin.clone(),
            ApiRole::Admin,
        ),
        (
            format!("{INTERNAL_KEY_PREFIX}write"),
            KEYS.write.clone(),
            ApiRole::Write,
        ),
        (
            format!("{INTERNAL_KEY_PREFIX}readonly"),
            KEYS.readonly.clone(),
            ApiRole::ReadOnly,
        ),
    ]
}

pub fn install_router(router: Router) {
    let _ = ROUTER.set(router);
}

fn key_for(role: &ApiRole) -> &'static str {
    match role {
        ApiRole::Admin => &KEYS.admin,
        ApiRole::Write => &KEYS.write,
        ApiRole::ReadOnly => &KEYS.readonly,
    }
}

#[derive(Debug, Clone)]
pub struct CallResult {
    pub status: u16,
    pub body: Value,
}

impl CallResult {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Unwrap the `{success, data}` envelope when present.
    pub fn data(&self) -> &Value {
        self.body.get("data").unwrap_or(&self.body)
    }

    pub fn error_message(&self) -> String {
        self.body
            .pointer("/error/message")
            .or_else(|| self.body.get("message"))
            .or_else(|| self.body.get("error"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {}", self.status))
    }
}

/// Call an API route as `role`, attributing it to `subject`.
pub async fn call(
    role: &ApiRole,
    subject: &str,
    method: &str,
    path: &str,
    body: Option<&Value>,
) -> anyhow::Result<CallResult> {
    let router = ROUTER
        .get()
        .ok_or_else(|| anyhow::anyhow!("API router not ready"))?
        .clone();
    let method: Method = method.parse()?;
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("x-api-key", key_for(role))
        .header(ON_BEHALF_HEADER, sanitize_subject(subject));
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(v)?)
        }
        None => Body::empty(),
    };
    let response = router.oneshot(builder.body(body)?).await?;
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024).await?;
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    Ok(CallResult { status, body })
}

fn sanitize_subject(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_keys_are_distinct_and_long() {
        let keys = internal_keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.iter().all(|(_, k, _)| k.len() == 48));
        assert_ne!(keys[0].1, keys[1].1);
        assert!(
            keys.iter()
                .all(|(n, _, _)| n.starts_with(INTERNAL_KEY_PREFIX))
        );
    }

    #[test]
    fn subject_header_strips_control_chars() {
        assert_eq!(sanitize_subject("alice\r\nX-Evil: 1"), "aliceX-Evil: 1");
    }

    #[test]
    fn call_result_unwraps_envelope_and_errors() {
        let r = CallResult {
            status: 404,
            body: serde_json::json!({"success": false, "error": {"message": "VM not found"}}),
        };
        assert!(!r.ok());
        assert_eq!(r.error_message(), "VM not found");
        let r = CallResult {
            status: 200,
            body: serde_json::json!({"success": true, "data": {"name": "x"}}),
        };
        assert_eq!(r.data()["name"], "x");
    }
}
