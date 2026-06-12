// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! OpenID Connect JWKS bearer validation for enterprise SSO.

#[cfg(feature = "web")]
use anyhow::{Context, Result};
#[cfg(feature = "web")]
use serde::Deserialize;
#[cfg(feature = "web")]
use std::sync::OnceLock;

#[cfg(feature = "web")]
#[derive(Debug, Clone, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[cfg(feature = "web")]
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // n/e/alg reserved for full RS256 verify
struct Jwk {
    kid: Option<String>,
    kty: Option<String>,
    #[serde(rename = "use")]
    key_use: Option<String>,
    n: Option<String>,
    e: Option<String>,
    alg: Option<String>,
}

#[cfg(feature = "web")]
static JWKS_CACHE: OnceLock<Jwks> = OnceLock::new();

#[cfg(feature = "web")]
pub fn oidc_configured() -> bool {
    crate::api::integrations::env_var("VMROGUE_OIDC_CLIENT_ID").is_some()
        && (crate::api::integrations::env_var("VMROGUE_OIDC_AUTHORIZATION_URL").is_some()
            || crate::api::integrations::env_var("VMROGUE_OIDC_ISSUER").is_some())
}

#[cfg(feature = "web")]
#[derive(Debug, Clone, Deserialize)]
pub struct OidcTokenExchangeRequest {
    pub code: String,
    pub redirect_uri: String,
    pub code_verifier: String,
}

#[cfg(feature = "web")]
pub async fn exchange_oidc_authorization_code(
    body: OidcTokenExchangeRequest,
) -> Result<serde_json::Value, String> {
    if !oidc_configured() {
        return Err("OIDC is not configured on this API".to_string());
    }
    let code = body.code.trim();
    let redirect_uri = body.redirect_uri.trim();
    let code_verifier = body.code_verifier.trim();
    if code.is_empty() || redirect_uri.is_empty() || code_verifier.is_empty() {
        return Err("code, redirect_uri, and code_verifier are required".to_string());
    }

    let token_url =
        crate::api::integrations::env_var("VMROGUE_OIDC_TOKEN_URL").unwrap_or_else(|| {
            crate::api::integrations::env_var("VMROGUE_OIDC_ISSUER")
                .map(|i| format!("{i}/protocol/openid-connect/token"))
                .unwrap_or_default()
        });
    if token_url.is_empty() {
        return Err("VMROGUE_OIDC_TOKEN_URL or VMROGUE_OIDC_ISSUER must be set".to_string());
    }
    let client_id = crate::api::integrations::env_var("VMROGUE_OIDC_CLIENT_ID")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "VMROGUE_OIDC_CLIENT_ID not set".to_string())?;

    let client = crate::api::integrations::http_client()
        .await
        .map_err(|e| format!("HTTP client: {e}"))?;

    let resp = client
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id.as_str()),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(|e| format!("token request failed: {e}"))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("read token response: {e}"))?;
    if !status.is_success() {
        return Err(format!(
            "IdP token endpoint returned {}: {}",
            status.as_u16(),
            text.chars().take(240).collect::<String>()
        ));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("invalid token JSON: {e}"))?;
    let access_token = parsed
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "token response missing access_token".to_string())?;

    Ok(serde_json::json!({
        "access_token": access_token,
        "token_type": parsed.get("token_type").and_then(|v| v.as_str()).unwrap_or("Bearer"),
        "expires_in": parsed.get("expires_in"),
        "scope": parsed.get("scope"),
    }))
}

#[cfg(feature = "web")]
pub fn oidc_public_config() -> serde_json::Value {
    let issuer = crate::api::integrations::env_var("VMROGUE_OIDC_ISSUER").unwrap_or_default();
    let client_id = crate::api::integrations::env_var("VMROGUE_OIDC_CLIENT_ID").unwrap_or_default();
    let auth_url = crate::api::integrations::env_var("VMROGUE_OIDC_AUTHORIZATION_URL")
        .unwrap_or_else(|| {
            if issuer.is_empty() {
                String::new()
            } else {
                format!("{issuer}/protocol/openid-connect/auth")
            }
        });
    let token_url =
        crate::api::integrations::env_var("VMROGUE_OIDC_TOKEN_URL").unwrap_or_else(|| {
            if issuer.is_empty() {
                String::new()
            } else {
                format!("{issuer}/protocol/openid-connect/token")
            }
        });
    let redirect = crate::api::integrations::env_var("VMROGUE_OIDC_REDIRECT_URI")
        .unwrap_or_else(|| "/dashboard".to_string());
    serde_json::json!({
        "enabled": oidc_configured(),
        "issuer": issuer,
        "client_id": client_id,
        "authorization_url": auth_url,
        "token_url": token_url,
        "redirect_uri": redirect,
        "scope": "openid profile email",
    })
}

#[cfg(feature = "web")]
async fn fetch_jwks() -> Result<Jwks> {
    if let Some(cached) = JWKS_CACHE.get() {
        return Ok(Jwks {
            keys: cached.keys.clone(),
        });
    }
    let url = crate::api::integrations::env_var("VMROGUE_OIDC_JWKS_URL")
        .context("VMROGUE_OIDC_JWKS_URL")?;
    let client = crate::api::integrations::http_client().await?;
    let jwks: Jwks = client
        .get(&url)
        .send()
        .await?
        .json()
        .await
        .context("jwks json")?;
    let _ = JWKS_CACHE.set(Jwks {
        keys: jwks.keys.clone(),
    });
    Ok(jwks)
}

#[cfg(feature = "web")]
fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let padded = match input.len() % 4 {
        2 => format!("{input}=="),
        3 => format!("{input}="),
        _ => input.to_string(),
    };
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(padded.trim_end_matches('='))
        .ok()
}

#[cfg(feature = "web")]
fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let bytes = base64url_decode(parts[1])?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(feature = "web")]
fn decode_jwt_header(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let bytes = base64url_decode(parts[0])?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(feature = "web")]
fn map_role_claim(value: &serde_json::Value) -> String {
    let claim = crate::api::integrations::env_var("VMROGUE_OIDC_ROLE_CLAIM")
        .unwrap_or_else(|| "groups".to_string());
    if let Some(s) = value.get(&claim).and_then(|v| v.as_str()) {
        return normalize_role(s);
    }
    if let Some(arr) = value.get(&claim).and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                let r = normalize_role(s);
                if r != "readonly" {
                    return r;
                }
            }
        }
    }
    value
        .get("role")
        .and_then(|v| v.as_str())
        .map(normalize_role)
        .unwrap_or_else(|| "readonly".to_string())
}

#[cfg(feature = "web")]
fn normalize_role(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.contains("admin") || lower == "vmrogue-admin" {
        "admin".to_string()
    } else if lower.contains("write") || lower == "vmrogue-write" {
        "write".to_string()
    } else {
        "readonly".to_string()
    }
}

#[cfg(feature = "web")]
fn issuer_matches(payload: &serde_json::Value) -> bool {
    let expected = match crate::api::integrations::env_var("VMROGUE_OIDC_ISSUER") {
        Some(i) if !i.is_empty() => i,
        _ => return true,
    };
    payload
        .get("iss")
        .and_then(|v| v.as_str())
        .map(|iss| iss == expected)
        .unwrap_or(false)
}

#[cfg(feature = "web")]
fn token_not_expired(payload: &serde_json::Value) -> bool {
    let Some(exp) = payload.get("exp").and_then(|v| v.as_i64()) else {
        return true;
    };
    chrono::Utc::now().timestamp() <= exp
}

/// Validate OIDC bearer via JWKS signature verification or userinfo fallback.
/// Returns role string: admin | write | readonly.
#[cfg(feature = "web")]
pub async fn oidc_role_from_bearer(token: &str) -> Option<String> {
    let payload = decode_jwt_payload(token)?;
    if !issuer_matches(&payload) || !token_not_expired(&payload) {
        return None;
    }

    if crate::api::integrations::env_var("VMROGUE_OIDC_JWKS_URL").is_some() {
        if let Ok(jwks) = fetch_jwks().await {
            let header = decode_jwt_header(token)?;
            let kid = header.get("kid").and_then(|v| v.as_str());
            if let Some(key) = jwks.keys.iter().find(|k| {
                k.kty.as_deref() == Some("RSA")
                    && kid.map(|id| k.kid.as_deref() == Some(id)).unwrap_or(true)
            }) {
                if let (Some(n), Some(e)) = (key.n.as_deref(), key.e.as_deref()) {
                    if let Ok(decoding_key) = jsonwebtoken::DecodingKey::from_rsa_components(n, e) {
                        let mut validation =
                            jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
                        if let Some(ref iss) =
                            crate::api::integrations::env_var("VMROGUE_OIDC_ISSUER")
                        {
                            if !iss.is_empty() {
                                validation.set_issuer(&[iss.as_str()]);
                            }
                        }
                        validation.validate_exp = true;
                        if let Ok(data) = jsonwebtoken::decode::<serde_json::Value>(
                            token,
                            &decoding_key,
                            &validation,
                        ) {
                            return Some(map_role_claim(&data.claims));
                        }
                    }
                }
            }
        }
    }

    if let Ok(Some(role)) = crate::api::integrations::oidc_userinfo_role(token).await {
        return Some(role);
    }

    None
}

#[cfg(not(feature = "web"))]
pub fn oidc_configured() -> bool {
    false
}
