// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Kryton machine control-plane client.
//!
//! Kryton (`../kryton`, the Zyvor machine API) puts one REST + CloudEvents contract in
//! front of Windows and Linux machines on four providers: in-memory `demo`, `dockur`
//! (real Windows in containers on a lab host), `libvirt` (Linux cloud images on a KVM
//! host) and `kubevirt`. Veyron drives Kryton for lab and edge machines that live outside
//! the cluster, and reuses its golden-image pipeline: `POST /golden/{id}/bootstrap`
//! publishes a checksum-pinned image as the CDI DataSource `kryton-images/<image-id>`,
//! which Veyron templates clone from.
//!
//! Configuration mirrors the Atlas integration:
//! - `VEYRON_KRYTON_URL`     — Kryton base URL, e.g. `http://kryton.kryton-system:8080`
//! - `VEYRON_KRYTON_TOKEN`   — bearer API key (Kryton stores only its SHA-256 digest)
//! - `VEYRON_KRYTON_PROJECT` — optional project sent as `?project=` (Kryton default otherwise)
//!
//! The wire contract is Kryton REST v1 (`/api/v1`); see `../kryton/openapi.yaml`.

/// Namespace where Kryton publishes golden-image DataSources (`KRYTON_IMAGE_NAMESPACE` default).
pub const KRYTON_IMAGE_NAMESPACE: &str = "kryton-images";

#[cfg(feature = "web")]
pub fn kryton_base_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_KRYTON_URL")
}

#[cfg(feature = "web")]
pub fn kryton_token() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_KRYTON_TOKEN")
}

#[cfg(feature = "web")]
pub fn kryton_project() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_KRYTON_PROJECT")
}

/// Kryton is an admin-configured backend, so loopback is allowed; non-http(s) schemes and
/// the cloud metadata endpoint are refused.
#[cfg(feature = "web")]
pub fn validate_kryton_url(url: &str) -> Result<(), String> {
    let lower = url.trim().to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(format!("unsafe Kryton URL scheme: {url}"));
    }
    let host = lower
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .split('@')
        .next_back()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    if host == "169.254.169.254" || host.starts_with("metadata.") {
        return Err("Kryton URL points at a cloud metadata endpoint".to_string());
    }
    Ok(())
}

/// Error from a Kryton call: the upstream HTTP status (or 502 when unreachable) plus body.
#[cfg(feature = "web")]
#[derive(Debug)]
pub struct KrytonError {
    pub status: u16,
    pub message: String,
}

#[cfg(feature = "web")]
impl std::fmt::Display for KrytonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Kryton returned {}: {}", self.status, self.message)
    }
}

/// Thin client over Kryton REST v1. Responses are passed through as JSON so new Kryton
/// fields reach the console without a Veyron release.
#[cfg(feature = "web")]
#[derive(Clone)]
pub struct KrytonClient {
    base: String,
    token: Option<String>,
    project: Option<String>,
}

#[cfg(feature = "web")]
impl KrytonClient {
    /// Build a client from env, or `None` when `VEYRON_KRYTON_URL` is unset/invalid.
    pub fn from_env() -> Option<Self> {
        let base = kryton_base_url()?;
        validate_kryton_url(&base).ok()?;
        Some(Self {
            base: base.trim_end_matches('/').to_string(),
            token: kryton_token(),
            project: kryton_project(),
        })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    /// `GET /healthz` — true when Kryton answers. Unauthenticated in Kryton.
    pub async fn healthy(&self) -> bool {
        let Ok(client) = crate::api::integrations::http_client().await else {
            return false;
        };
        matches!(
            client.get(format!("{}/healthz", self.base)).send().await,
            Ok(r) if r.status().is_success()
        )
    }

    /// Call `{base}/api/v1{path}` and return the JSON body. Query pairs are forwarded
    /// as-is; `project` is added from `VEYRON_KRYTON_PROJECT` unless the caller set one.
    pub async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, KrytonError> {
        let client = crate::api::integrations::http_client()
            .await
            .map_err(|e| KrytonError {
                status: 502,
                message: format!("HTTP client unavailable: {e}"),
            })?;
        let mut q: Vec<(String, String)> = query.to_vec();
        if let Some(p) = &self.project {
            if !q.iter().any(|(k, _)| k == "project") {
                q.push(("project".to_string(), p.clone()));
            }
        }
        let mut req = client.request(method, format!("{}/api/v1{}", self.base, path));
        if !q.is_empty() {
            req = req.query(&q);
        }
        if let Some(t) = self.token.as_ref().filter(|t| !t.trim().is_empty()) {
            req = req.bearer_auth(t);
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.map_err(|e| KrytonError {
            status: 502,
            message: format!("Kryton request failed: {e}"),
        })?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(KrytonError {
                status: status.as_u16(),
                message: text.trim().chars().take(300).collect(),
            });
        }
        if text.trim().is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| KrytonError {
            status: 502,
            message: format!("invalid Kryton JSON: {e}"),
        })
    }
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_bad_schemes_and_metadata() {
        assert!(validate_kryton_url("http://127.0.0.1:8080").is_ok());
        assert!(validate_kryton_url("https://kryton.example.com").is_ok());
        assert!(validate_kryton_url("file:///etc/passwd").is_err());
        assert!(validate_kryton_url("http://169.254.169.254/latest").is_err());
    }
}
