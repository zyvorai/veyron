// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// HTTP Server - Real web server using axum for serving API and dashboard
//
// This module is only compiled when the "web" feature is enabled.

#[cfg(feature = "web")]
pub mod web {
    use crate::api::auth_context::{self, AuthContext};
    use crate::api::dashboard_paths;
    use crate::api::{ApiResponse, HttpMethod, RequestContext};
    use crate::config::{
        CloudInitDelivery, DiskSource, VMConfigBuilder, VmExposeConfig, VmExposePort,
    };
    use crate::copilot::{self, AiMode};
    use crate::kube::{
        KubeClient, MigrateVmOptions, vm_expose_service_name, vm_internet, vm_rdp, windows_rdp,
    };
    use crate::profiles::PROFILES;
    use crate::templates::TEMPLATES;
    use crate::tui::state::VmInfo;
    use axum::{
        Json, Router,
        body::Bytes,
        extract::{
            DefaultBodyLimit, Path, Query, State, WebSocketUpgrade,
            ws::{Message, WebSocket},
        },
        http::{HeaderMap, StatusCode, header},
        middleware,
        response::IntoResponse,
        routing::{delete, get, post, put},
    };
    use include_dir::{Dir, include_dir};
    use k8s_openapi::api::core::v1::{PersistentVolumeClaim, Service};
    use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
    use serde::{Deserialize, Serialize};
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tower_http::cors::{AllowOrigin, CorsLayer};
    use tower_http::timeout::TimeoutLayer;

    /// First non-empty environment variable from `keys` (Veyron names take precedence).
    mod resources;
    mod tls;
    mod vm;
    mod ws;
    use self::{resources::*, tls::*, vm::*, ws::*};

    fn env_var_first(keys: &[&str]) -> Option<String> {
        keys.iter().find_map(|k| {
            std::env::var(k)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        })
    }

    /// Simple sliding-window rate limiter state.
    struct RateLimiterState {
        inner: std::sync::Mutex<RateLimiterInner>,
        max_requests: u64,
        window_secs: u64,
    }

    struct RateLimiterInner {
        count: u64,
        window_start: u64,
    }

    impl RateLimiterState {
        fn new(max_requests: u64, window_secs: u64) -> Self {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            Self {
                inner: std::sync::Mutex::new(RateLimiterInner {
                    count: 0,
                    window_start: now,
                }),
                max_requests,
                window_secs,
            }
        }

        fn check_rate_limit(&self) -> bool {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());

            // Reset window if expired
            if now - inner.window_start >= self.window_secs {
                inner.window_start = now;
                inner.count = 1;
                return true;
            }

            inner.count += 1;
            inner.count <= self.max_requests
        }
    }

    /// TLS configuration for the API server.
    #[derive(Clone, Debug)]
    pub struct TlsConfig {
        pub cert_path: String,
        pub key_path: String,
    }

    impl TlsConfig {
        /// Validate that the cert and key files exist on disk.
        pub fn validate(&self) -> anyhow::Result<()> {
            if !std::path::Path::new(&self.cert_path).exists() {
                return Err(anyhow::anyhow!(
                    "TLS certificate file not found: {}",
                    self.cert_path
                ));
            }
            if !std::path::Path::new(&self.key_path).exists() {
                return Err(anyhow::anyhow!("TLS key file not found: {}", self.key_path));
            }
            Ok(())
        }
    }

    /// API key role for RBAC enforcement.
    #[derive(Debug, Clone, PartialEq)]
    pub enum ApiRole {
        Admin,    // Full access
        Write,    // Read + mutating operations
        ReadOnly, // GET only
    }

    /// An API key with an associated role.
    #[derive(Debug, Clone)]
    pub struct ApiKeyEntry {
        pub key: String,
        pub role: ApiRole,
        pub name: String,
    }

    pub struct WebState {
        pub namespace: String,
        pub kube_client: KubeClient,
        /// Kubeconfig context name used for API calls (may differ from file default).
        pub active_kube_context: Option<String>,
        pub api_key: Option<String>,
        pub api_keys: Vec<ApiKeyEntry>,
        rate_limiter: RateLimiterState,
        ai_rate_limiter: RateLimiterState,
        login_rate_limiter: RateLimiterState,
    }

    impl WebState {
        pub async fn new(namespace: String, rate_limit_per_minute: u64) -> anyhow::Result<Self> {
            let api_key = env_var_first(&["VEYRON_API_KEY", "VEYRON_API_KEY"]);
            if api_key.is_none() {
                log::warn!(
                    "VEYRON_API_KEY (or VEYRON_API_KEY) is not set - API will reject all requests. \
                     Set VEYRON_API_KEY to enable access."
                );
            }

            // Load additional API keys with roles from VEYRON_API_KEYS
            // Format: "name1:key1:admin,name2:key2:readonly,name3:key3:write"
            let mut api_keys = Vec::new();
            if let Some(ref primary) = api_key {
                api_keys.push(ApiKeyEntry {
                    key: primary.clone(),
                    role: ApiRole::Admin,
                    name: "primary".to_string(),
                });
            }
            if let Some(keys_str) = env_var_first(&["VEYRON_API_KEYS", "VEYRON_API_KEYS"]) {
                for entry in keys_str.split(',') {
                    let parts: Vec<&str> = entry.trim().split(':').collect();
                    if parts.len() >= 2 {
                        let name = parts[0].to_string();
                        let key = parts[1].to_string();
                        let role = match parts.get(2).copied() {
                            Some("admin") => ApiRole::Admin,
                            Some("write") => ApiRole::Write,
                            Some("readonly") | Some("read") => ApiRole::ReadOnly,
                            _ => ApiRole::ReadOnly,
                        };
                        api_keys.push(ApiKeyEntry { key, role, name });
                    }
                }
            }

            let mut active_kube_context = None;
            let bootstrap = KubeClient::new().await?;
            if let Some(ctx) =
                crate::api::handlers::clusters::load_persisted_kube_context(&bootstrap, &namespace)
                    .await
            {
                crate::kube::set_active_kube_context(Some(ctx.clone()));
                active_kube_context = Some(ctx);
            }
            let kube_client = KubeClient::new().await?;
            Ok(Self {
                namespace,
                kube_client,
                active_kube_context,
                api_key,
                api_keys,
                rate_limiter: RateLimiterState::new(rate_limit_per_minute, 60),
                ai_rate_limiter: RateLimiterState::new(copilot::ai_rate_limit_per_min(), 60),
                // Brute-force guard on the public, unauthenticated login endpoint.
                login_rate_limiter: RateLimiterState::new(30, 300),
            })
        }

        /// Switch kube API client to another kubeconfig context (multi-cluster file).
        pub async fn apply_kube_context(&mut self, context: Option<String>) -> anyhow::Result<()> {
            crate::kube::set_active_kube_context(context.clone());
            self.kube_client = KubeClient::new().await?;
            self.active_kube_context = context;
            Ok(())
        }

        pub fn client(&self) -> &KubeClient {
            &self.kube_client
        }

        /// Authenticate an API key and return its role.
        pub fn authenticate(&self, key: &str) -> Option<&ApiRole> {
            self.api_keys.iter().find_map(|entry| {
                if constant_time_eq(key.as_bytes(), entry.key.as_bytes()) {
                    Some(&entry.role)
                } else {
                    None
                }
            })
        }

        /// Authenticate a JWT Bearer token with HMAC-SHA256 signature verification.
        ///
        /// **Required:** Set VEYRON_JWT_SECRET (shared secret for HMAC-SHA256 signature).
        /// **Required:** Set VEYRON_JWT_ISSUER (expected issuer claim).
        /// **Optional:** Set VEYRON_JWT_ROLE_CLAIM (claim containing role, default: "role").
        /// JWT roles: "admin", "write", "readonly" (default: "readonly").
        ///
        /// The token signature is verified using HMAC-SHA256 before claims are trusted.
        pub fn authenticate_jwt(&self, token: &str) -> Option<ApiRole> {
            let secret = std::env::var("VEYRON_JWT_SECRET").ok()?;
            let expected_issuer = std::env::var("VEYRON_JWT_ISSUER").ok()?;
            if secret.is_empty() || expected_issuer.is_empty() {
                return None;
            }

            let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
            validation.set_issuer(&[expected_issuer.as_str()]);
            validation.validate_exp = true;

            let token_data = jsonwebtoken::decode::<serde_json::Value>(
                token,
                &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
                &validation,
            )
            .map_err(|e| {
                log::debug!("JWT validation failed: {e}");
                e
            })
            .ok()?;

            let role_claim =
                std::env::var("VEYRON_JWT_ROLE_CLAIM").unwrap_or_else(|_| "role".to_string());
            let role_str = token_data
                .claims
                .get(&role_claim)
                .and_then(|v| v.as_str())
                .unwrap_or("readonly");

            let role = match role_str {
                "admin" => ApiRole::Admin,
                "write" => ApiRole::Write,
                _ => ApiRole::ReadOnly,
            };

            Some(role)
        }

        /// Whether another login attempt is allowed under the shared brute-force
        /// guard for `POST /api/v1/auth/login`. Global, not per-caller — matches
        /// the existing `rate_limiter`/`ai_rate_limiter` pattern (no per-IP state
        /// exists elsewhere in this codebase).
        pub fn check_login_rate_limit(&self) -> bool {
            self.login_rate_limiter.check_rate_limit()
        }

        /// Mint a signed session token for a successfully authenticated username+
        /// password login. Reuses the same `VEYRON_JWT_SECRET`/`VEYRON_JWT_ISSUER`
        /// that `authenticate_jwt` already validates against, so the returned
        /// token is accepted by every existing auth path with no other changes.
        pub fn issue_jwt(
            &self,
            username: &str,
            role: &str,
            ttl_secs: i64,
        ) -> Result<String, String> {
            let secret = std::env::var("VEYRON_JWT_SECRET")
                .map_err(|_| "VEYRON_JWT_SECRET is not configured".to_string())?;
            let issuer = std::env::var("VEYRON_JWT_ISSUER")
                .map_err(|_| "VEYRON_JWT_ISSUER is not configured".to_string())?;
            if secret.is_empty() || issuer.is_empty() {
                return Err("VEYRON_JWT_SECRET/VEYRON_JWT_ISSUER must not be empty".to_string());
            }
            let role_claim =
                std::env::var("VEYRON_JWT_ROLE_CLAIM").unwrap_or_else(|_| "role".to_string());
            let now = chrono::Utc::now().timestamp();
            let mut claims = serde_json::Map::new();
            claims.insert(
                "sub".to_string(),
                serde_json::Value::String(username.to_string()),
            );
            claims.insert("iss".to_string(), serde_json::Value::String(issuer));
            claims.insert("iat".to_string(), serde_json::Value::from(now));
            claims.insert("exp".to_string(), serde_json::Value::from(now + ttl_secs));
            claims.insert(role_claim, serde_json::Value::String(role.to_string()));

            jsonwebtoken::encode(
                &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
                &serde_json::Value::Object(claims),
                &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
            )
            .map_err(|e| format!("failed to sign session token: {e}"))
        }
    }

    /// URL percent-decode (e.g., %40 → @, %20 → space).
    fn percent_decode(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            } else if c == '+' {
                result.push(' ');
            } else {
                result.push(c);
            }
        }
        result
    }

    pub type SharedState = Arc<RwLock<WebState>>;

    // ── Kubernetes name validation (RFC 1123 DNS label) ──────────

    /// Validate that a string is a valid Kubernetes name (RFC 1123 DNS label).
    /// Must be at most 63 characters, consist of lowercase alphanumeric characters
    /// or '-', and must start and end with an alphanumeric character.
    fn is_valid_k8s_name(s: &str) -> bool {
        if s.is_empty() || s.len() > 63 {
            return false;
        }
        let bytes = s.as_bytes();
        // Must start and end with alphanumeric
        if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
            return false;
        }
        if !bytes[bytes.len() - 1].is_ascii_lowercase() && !bytes[bytes.len() - 1].is_ascii_digit()
        {
            return false;
        }
        // All characters must be lowercase alphanumeric or '-'
        s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    }

    /// Return a 400 error if a path parameter is not a valid Kubernetes name.
    fn validate_k8s_params(
        params: &[(&str, &str)],
    ) -> Option<(StatusCode, Json<serde_json::Value>)> {
        for (label, value) in params {
            if !is_valid_k8s_name(value) {
                return Some(err_json(
                    400,
                    "INVALID_PARAMETER",
                    &format!("'{}' is not a valid Kubernetes name", label),
                ));
            }
        }
        None
    }

    // ── Auth middleware ──────────────────────────────────────────

    /// Constant-time byte comparison to prevent timing attacks.
    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        result == 0
    }

    /// Record mutating API calls into the SOC event buffer (after auth succeeds).
    async fn soc_audit_middleware(
        State(state): State<SharedState>,
        request: axum::extract::Request,
        next: middleware::Next,
    ) -> impl IntoResponse {
        let method = request.method().to_string();
        let path = request.uri().path().to_string();
        let response = next.run(request).await;
        let outcome = if response.status().is_success() {
            "success"
        } else {
            "failure"
        };
        let state_c = state.clone();
        let method_c = method.clone();
        let path_c = path.clone();
        let outcome_c = outcome.to_string();
        tokio::spawn(async move {
            crate::api::handlers::soc::record_api_audit(
                &state_c, "api", &method_c, &path_c, &outcome_c,
            )
            .await;
        });
        response
    }

    /// Resolve the RBAC role for an authenticated credential.
    async fn resolve_api_role(state: &SharedState, key: &str, is_primary: bool) -> Option<ApiRole> {
        if is_primary {
            let s = state.read().await;
            if let Some(role) = s.authenticate(key) {
                return Some(role.clone());
            }
            return Some(ApiRole::Admin);
        }

        let s = state.read().await;
        if let Some(role) = s.authenticate(key) {
            return Some(role.clone());
        }
        if let Some(role) = s.authenticate_jwt(key) {
            return Some(role);
        }
        drop(s);

        if crate::api::oidc::oidc_configured() {
            return crate::api::oidc::oidc_role_from_bearer(key)
                .await
                .map(|rstr| match rstr.as_str() {
                    "admin" => ApiRole::Admin,
                    "write" => ApiRole::Write,
                    _ => ApiRole::ReadOnly,
                });
        }
        if crate::api::integrations::env_var("VEYRON_OIDC_USERINFO_URL").is_some() {
            return match crate::api::integrations::oidc_userinfo_role(key).await {
                Ok(Some(rstr)) => Some(match rstr.as_str() {
                    "admin" => ApiRole::Admin,
                    "write" => ApiRole::Write,
                    _ => ApiRole::ReadOnly,
                }),
                _ => None,
            };
        }
        None
    }

    fn authorize_request(
        role: ApiRole,
        method: &axum::http::Method,
        path: &str,
    ) -> Result<AuthContext, (StatusCode, Json<serde_json::Value>)> {
        let ctx = AuthContext { role };
        let required = auth_context::min_role_for_route(method, path);
        if !ctx.allows(&required) {
            let msg = if required == ApiRole::Admin {
                "Admin role required for this operation"
            } else {
                "Insufficient permissions for this operation"
            };
            let (status, json) = err_json(403, "FORBIDDEN", msg);
            return Err((status, json));
        }
        Ok(ctx)
    }

    /// API key authentication middleware.
    ///
    /// If the `VEYRON_API_KEY` env var was set at startup, every request must present
    /// that key via `X-API-Key` header or `Authorization: Bearer <key>`.
    /// If the env var was unset, all requests are rejected (deny by default).
    async fn auth_middleware(
        State(state): State<SharedState>,
        headers: HeaderMap,
        request: axum::extract::Request,
        next: middleware::Next,
    ) -> impl IntoResponse {
        // Allow health, dashboard, static assets, and dashboard-originated API calls without auth
        let path = request.uri().path();
        if dashboard_paths::is_auth_exempt_path(path) {
            return next.run(request).await.into_response();
        }

        // Dashboard API calls must include the API key like any other client.
        // The dashboard login stores the key in localStorage and sends it
        // via the X-API-Key header on every fetch call.

        // Allow CORS preflight (OPTIONS) requests without auth
        if request.method() == axum::http::Method::OPTIONS {
            return next.run(request).await.into_response();
        }

        let s = state.read().await;
        let expected_key = match &s.api_key {
            Some(k) => k.clone(),
            None => {
                let (status, json) = err_json(
                    503,
                    "AUTH_NOT_CONFIGURED",
                    "API key not configured. Set VEYRON_API_KEY (or VEYRON_API_KEY) environment variable.",
                );
                return (status, json).into_response();
            }
        };
        drop(s);

        // Check X-API-Key header, Authorization: Bearer, ?ticket= (WS consoles), or legacy ?token=
        let ws_ticket = request.uri().query().and_then(|q| {
            q.split('&')
                .find_map(|p| p.strip_prefix("ticket=").map(percent_decode))
        });
        if auth_context::is_ws_console_path(path) {
            if let Some(ticket) = ws_ticket {
                if crate::api::ws_ticket::consume_ws_ticket(&ticket) {
                    let ctx = AuthContext {
                        role: ApiRole::Write,
                    };
                    let mut request = request;
                    request.extensions_mut().insert(ctx);
                    return next.run(request).await.into_response();
                }
                let (status, json) =
                    err_json(401, "UNAUTHORIZED", "Invalid or expired WebSocket ticket");
                return (status, json).into_response();
            }
        }

        let provided_key = headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                headers
                    .get(header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer ").map(|s| s.to_string()))
            })
            .or_else(|| {
                request.uri().query().and_then(|q| {
                    q.split('&')
                        .find_map(|p| p.strip_prefix("token=").map(percent_decode))
                })
            });

        let method = request.method().clone();

        match provided_key {
            Some(key) if constant_time_eq(key.as_bytes(), expected_key.as_bytes()) => {
                let role = resolve_api_role(&state, &key, true)
                    .await
                    .unwrap_or(ApiRole::Admin);
                match authorize_request(role, &method, path) {
                    Ok(ctx) => {
                        let mut request = request;
                        request.extensions_mut().insert(ctx);
                        next.run(request).await.into_response()
                    }
                    Err(resp) => resp.into_response(),
                }
            }
            Some(key) => match resolve_api_role(&state, &key, false).await {
                Some(role) => match authorize_request(role, &method, path) {
                    Ok(ctx) => {
                        let mut request = request;
                        request.extensions_mut().insert(ctx);
                        next.run(request).await.into_response()
                    }
                    Err(resp) => resp.into_response(),
                },
                None => {
                    let (status, json) =
                        err_json(401, "UNAUTHORIZED", "Invalid or missing API key");
                    (status, json).into_response()
                }
            },
            None => {
                let (status, json) = err_json(401, "UNAUTHORIZED", "Invalid or missing API key");
                (status, json).into_response()
            }
        }
    }

    // ── Rate limiting middleware ──────────────────────────────────

    /// Middleware that enforces a global request rate limit.
    async fn rate_limit_middleware(
        State(state): State<SharedState>,
        request: axum::extract::Request,
        next: middleware::Next,
    ) -> impl IntoResponse {
        // Allow health, dashboard, and internal dashboard API calls without rate limiting
        let path = request.uri().path();
        if dashboard_paths::is_rate_limit_exempt_path(path, request.method().as_str()) {
            return next.run(request).await.into_response();
        }

        let s = state.read().await;
        if !s.rate_limiter.check_rate_limit() {
            let (status, json) =
                err_json(429, "RATE_LIMITED", "Too many requests. Please slow down.");
            return (status, json).into_response();
        }

        let method = request.method().as_str();
        if dashboard_paths::is_ai_rate_limited_path(path, method)
            && copilot::ai_mode() != AiMode::Off
            && !s.ai_rate_limiter.check_rate_limit()
        {
            let (status, json) = err_json(
                429,
                "AI_RATE_LIMITED",
                "Too many Copilot LLM requests. Please slow down.",
            );
            return (status, json).into_response();
        }
        drop(s);
        next.run(request).await.into_response()
    }

    // ── WebSocket Metrics Streaming ──────────────────────────────

    /// Middleware that adds security headers to every response.
    async fn security_headers_middleware(
        request: axum::extract::Request,
        next: middleware::Next,
    ) -> impl IntoResponse {
        let mut response = next.run(request).await;
        let headers = response.headers_mut();
        headers.insert("x-content-type-options", "nosniff".parse().unwrap());
        headers.insert("x-frame-options", "DENY".parse().unwrap());
        headers.insert("cache-control", "no-store".parse().unwrap());
        headers.insert("x-xss-protection", "0".parse().unwrap());
        headers.insert("content-security-policy", "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; img-src 'self' data: blob:; font-src 'self' data: https://fonts.gstatic.com; connect-src 'self' ws: wss:".parse().unwrap());
        headers.insert("referrer-policy", "no-referrer".parse().unwrap());
        // HSTS: instruct browsers to only use HTTPS for 1 year
        headers.insert(
            "strict-transport-security",
            "max-age=31536000; includeSubDomains".parse().unwrap(),
        );
        response
    }

    // ── CORS configuration ──────────────────────────────────────

    /// Build a CORS layer. If `VEYRON_CORS_ORIGINS` is set (comma-separated
    /// list of origins), allow those origins. Otherwise default to same-origin
    /// only (no extra origins allowed).
    /// Every verb the API routes use — PUT/PATCH back rdp-expose, network/internet,
    /// run-strategy and friends, so they must pass a cross-origin preflight too.
    const CORS_METHODS: [axum::http::Method; 6] = [
        axum::http::Method::GET,
        axum::http::Method::POST,
        axum::http::Method::PUT,
        axum::http::Method::PATCH,
        axum::http::Method::DELETE,
        axum::http::Method::OPTIONS,
    ];

    fn build_cors_layer() -> CorsLayer {
        let origins = std::env::var("VEYRON_CORS_ORIGINS").ok();

        let allow_origin = match origins {
            Some(ref raw) if !raw.is_empty() => {
                let parsed: Vec<_> = raw
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if parsed.is_empty() {
                    AllowOrigin::default()
                } else {
                    AllowOrigin::list(parsed)
                }
            }
            _ => AllowOrigin::default(), // same-origin: no Access-Control-Allow-Origin header
        };

        CorsLayer::new()
            .allow_origin(allow_origin)
            .allow_methods(CORS_METHODS)
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                "x-api-key".parse().unwrap(),
            ])
    }

    // ── Router ──────────────────────────────────────────────────

    pub fn build_router(state: SharedState) -> Router {
        let request_timeout_secs = std::env::var("VEYRON_HTTP_REQUEST_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(30);

        // Long-lived WebSockets must not sit behind the global HTTP request timeout (often ~30s).
        let long_lived_ws = Router::new()
            .route("/api/v1/vms/:ns/:name/vnc", get(vnc_websocket_handler))
            .route(
                "/api/v1/vms/:ns/:name/serial",
                get(serial_websocket_handler),
            )
            .route("/api/v1/ws/metrics", get(metrics_websocket_handler))
            .with_state(state.clone());

        let timed_rest =
            Router::new()
                .route("/", get(root_handler))
                .route("/console", get(console_handler))
                .route("/console/", get(console_handler))
                .route("/console/assets/*file", get(console_asset_handler))
                .route("/dashboard", get(console_redirect_handler))
                .route("/dashboard/", get(console_redirect_handler))
                // VM endpoints
                .route("/api/v1/vms", get(list_vms_handler))
                .route("/api/v1/vms", post(create_vm_handler))
                // Batch must be before parameterized :ns/:name routes to avoid ambiguity
                .route("/api/v1/vms/batch", post(batch_vm_handler))
                .route("/api/v1/vms/:ns/:name", get(get_vm_handler))
                .route("/api/v1/vms/:ns/:name/drift", get(get_vm_drift_handler))
                .route("/api/v1/vms/:ns/:name", delete(delete_vm_handler))
                // Templates
                .route("/api/v1/templates", get(list_templates_handler))
                .route("/api/v1/vms/:ns/:name/start", post(start_vm_handler))
                .route("/api/v1/vms/:ns/:name/stop", post(stop_vm_handler))
                .route("/api/v1/vms/:ns/:name/restart", post(restart_vm_handler))
                .route("/api/v1/vms/:ns/:name/clone", post(clone_vm_handler))
                .route("/api/v1/vms/:ns/:name/pause", post(pause_vm_handler))
                .route("/api/v1/vms/:ns/:name/unpause", post(unpause_vm_handler))
                .route("/api/v1/vms/:ns/:name/migrate", post(migrate_vm_handler))
                .route(
                    "/api/v1/vms/:ns/:name/migrations",
                    get(list_vm_migrations_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/migrations/:migname",
                    delete(delete_vm_migration_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/freeze",
                    post(guest_freeze_vm_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/unfreeze",
                    post(guest_unfreeze_vm_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/softreboot",
                    post(guest_softreboot_vm_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/volumes/status",
                    get(vm_volume_status_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/volumes/hotplug",
                    post(vm_hotplug_volume_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/storage/data-disk/defaults",
                    get(vm_data_disk_defaults_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/storage/data-disk",
                    post(vm_add_data_disk_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/volumes/hotremove",
                    post(vm_hotremove_volume_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/console/serial",
                    get(vm_serial_console_hint_handler),
                )
                .route("/api/v1/vms/:ns/:name/expose", get(get_vm_expose_handler))
                .route("/api/v1/vms/:ns/:name/expose", put(put_vm_expose_handler))
                .route(
                    "/api/v1/vms/:ns/:name/expose",
                    delete(delete_vm_expose_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/network/internet",
                    get(vm_internet_get_handler)
                        .put(vm_internet_put_handler)
                        .delete(vm_internet_delete_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/rdp-expose",
                    get(get_vm_rdp_expose_handler)
                        .put(put_vm_rdp_expose_handler)
                        .delete(delete_vm_rdp_expose_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest-agent/enable-rdp",
                    post(guest_agent_enable_rdp_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest-agent/disable-rdp",
                    post(guest_agent_disable_rdp_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest-filesystem",
                    get(guest_filesystem_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/status",
                    get(guest_runtime_status_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/evidence",
                    get(guest_evidence_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/doctor",
                    get(guest_doctor_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/fix-plan",
                    post(guest_fix_plan_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/metrics",
                    get(guest_metrics_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/migrate-score",
                    get(guest_migrate_score_handler),
                )
                .route("/api/v1/vms/:ns/:name/guest/exec", post(guest_exec_handler))
                .route(
                    "/api/v1/vms/:ns/:name/guest/agent",
                    get(guest_agent_status_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name/guest/agent/deploy",
                    post(guest_agent_deploy_handler),
                )
                .route(
                    "/api/v1/platform/guestkit/binary",
                    get(guestkit_binary_handler),
                )
                .route(
                    "/api/v1/vms/:ns/:name",
                    axum::routing::put(update_vm_handler),
                )
                .route("/api/v1/vms/:ns/:name/security", get(vm_security_handler))
                .route("/api/v1/vms/:ns/:name/events", get(vm_events_handler))
                // Snapshots
                .route("/api/v1/snapshots", get(list_snapshots_handler))
                // Registered directly here (not via the nested handlers router) because the
                // static `/api/v1/snapshots` node above shadows nested `/snapshots/*` param
                // routes under matchit — see export_snapshots in handlers::snapshots.
                .route(
                    "/api/v1/snapshots/export",
                    get(crate::api::handlers::snapshots::export_snapshots),
                )
                .route("/api/v1/snapshots/:ns/:vm", get(list_vm_snapshots_handler))
                .route(
                    "/api/v1/snapshots/:ns/:name/delete",
                    post(delete_snapshot_handler),
                )
                // Snapshots - create & restore
                .route(
                    "/api/v1/snapshots/:ns/:vm/create",
                    post(create_snapshot_handler),
                )
                .route(
                    "/api/v1/snapshots/:ns/:name/restore",
                    post(restore_snapshot_handler),
                )
                .route(
                    "/api/v1/snapshot-schedules",
                    get(list_snapshot_schedules_handler),
                )
                .route(
                    "/api/v1/snapshot-schedules",
                    post(create_snapshot_schedule_handler),
                )
                .route(
                    "/api/v1/snapshot-schedules/:ns/:cm_name",
                    delete(delete_snapshot_schedule_handler),
                )
                // Events
                .route("/api/v1/events", get(list_events_handler))
                .route("/api/v1/events/recent", get(recent_events_handler))
                // Cluster resources
                .route("/api/v1/nodes", get(list_nodes_handler))
                .route(
                    "/api/v1/nodes/:name",
                    get(crate::api::handlers::nodes::get_node),
                )
                .route(
                    "/api/v1/nodes/:name/cordon",
                    post(crate::api::handlers::nodes::cordon_node),
                )
                .route(
                    "/api/v1/nodes/:name/uncordon",
                    post(crate::api::handlers::nodes::uncordon_node),
                )
                .route("/api/v1/pods", get(list_pods_handler))
                .route("/api/v1/pods/summary", get(pods_summary_handler))
                .route("/api/v1/pods/:name/logs", get(get_pod_logs_handler))
                .route(
                    "/api/v1/pods/:ns/:name",
                    delete(crate::api::handlers::pods::delete_pod),
                )
                .route(
                    "/api/v1/nodes/:name/reboot",
                    post(crate::api::handlers::nodes::reboot_node),
                )
                .route("/api/v1/profiles", get(list_profiles_handler))
                .route("/api/v1/namespaces", get(list_namespaces_handler))
                .route("/api/v1/activity", get(activity_feed_handler))
                // Storage
                .route("/api/v1/storage/pvcs", get(list_pvcs_handler))
                .route("/api/v1/storage/classes", get(list_storage_classes_handler))
                // OpenAPI
                .route("/api/openapi.json", get(openapi_handler))
                // Dashboard overview
                .route(
                    "/api/v1/dashboard/overview",
                    get(dashboard_overview_handler),
                )
                // Health + public OIDC discovery
                .route("/api/v1/health", get(health_handler))
                // K8s-style readiness/liveness with component checks (dashboard
                // Nodes → Platform Readiness card). The handlers::health router that
                // defines these was never merged, so mount them here explicitly.
                .route(
                    "/api/v1/health/ready",
                    get(crate::api::handlers::health::readiness_check),
                )
                .route(
                    "/api/v1/health/live",
                    get(crate::api::handlers::health::liveness_check),
                )
                .route("/api/v1/ws/ticket", post(ws_ticket_handler))
                .route(
                    "/api/v1/auth/oidc/config",
                    get(|| async { axum::Json(crate::api::oidc::oidc_public_config()) }),
                )
                .route(
                    "/api/v1/auth/oidc/token",
                    post(
                        |axum::Json(body): axum::Json<
                            crate::api::oidc::OidcTokenExchangeRequest,
                        >| async move {
                            match crate::api::oidc::exchange_oidc_authorization_code(body).await {
                                Ok(json) => axum::Json(json).into_response(),
                                Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response(),
                            }
                        },
                    ),
                )
                .with_state(state.clone())
                // Handler modules register paths like `/ingress`, `/monitoring/status`; nest under `/api/v1`
                // so the dashboard (`/api/v1/...`) and OpenAPI stay aligned.
                .merge(
                    Router::new().nest("/api/v1", crate::api::handlers::all_routes(state.clone())),
                )
                // Veyron product alias — same handlers as /api/v1/* (VEYRON_* env still supported).
                .merge(Router::new().nest(
                    "/api/v1/veyron",
                    crate::api::handlers::all_routes(state.clone()),
                ))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    std::time::Duration::from_secs(request_timeout_secs),
                ));

        Router::new()
            .merge(long_lived_ws)
            .merge(timed_rest)
            // Layers applied in reverse order (outermost = last .layer() call)
            .layer(middleware::from_fn(security_headers_middleware))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                rate_limit_middleware,
            ))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                soc_audit_middleware,
            ))
            .layer(middleware::from_fn_with_state(state, auth_middleware))
            .layer(build_cors_layer())
            .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10 MiB
    }

    pub async fn start_server(
        host: &str,
        port: u16,
        namespace: String,
        tls_config: Option<TlsConfig>,
        rate_limit_per_minute: u64,
    ) -> anyhow::Result<()> {
        // Validate TLS config early if provided
        if let Some(ref tls) = tls_config {
            tls.validate()?;
        }

        // Snapshot schedule worker: only one replica should run ticks (Kubernetes Lease).
        let scheduler_lease_ns = std::env::var("VEYRON_SCHEDULER_LEASE_NAMESPACE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| namespace.clone());
        let disable_scheduler_lease = std::env::var("VEYRON_SCHEDULER_LEASE_DISABLED")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let scheduler_holder = crate::snapshots::scheduler_lease::scheduler_holder_identity();

        // Initialize kube client at startup instead of lazily per-request
        let state = Arc::new(RwLock::new(
            WebState::new(namespace.clone(), rate_limit_per_minute).await?,
        ));
        {
            let kube = { state.read().await.kube_client.clone() };
            crate::kube::user_store::bootstrap_dashboard_admin(&kube, &namespace).await;
        }
        {
            let ns = namespace.clone();
            let k8s = {
                let s = state.read().await;
                s.kube_client.client()
            };
            crate::soc::export::spawn_export_loop(ns, k8s, 30);
        }
        let schedule_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await;
            loop {
                interval.tick().await;
                let client = {
                    let s = schedule_state.read().await;
                    s.kube_client.client()
                };
                if !disable_scheduler_lease {
                    match crate::snapshots::scheduler_lease::acquire_snapshot_scheduler_leader(
                        client.clone(),
                        &scheduler_lease_ns,
                        &scheduler_holder,
                    )
                    .await
                    {
                        Ok(true) => {}
                        Ok(false) => continue,
                        Err(e) => {
                            log::warn!(
                                "snapshot scheduler lease (namespace={}): {}",
                                scheduler_lease_ns,
                                e
                            );
                            continue;
                        }
                    }
                }
                if let Err(e) =
                    crate::snapshots::vm_snapshot_schedule::snapshot_schedule_tick(client).await
                {
                    log::warn!("snapshot schedule tick: {}", e);
                }
                // Self-healing reconciler (leader-only, gated by its policy CM).
                let kube_for_heal = { schedule_state.read().await.kube_client.clone() };
                if let Err(e) =
                    crate::api::handlers::self_healing::self_healing_tick(kube_for_heal).await
                {
                    log::warn!("self-healing tick: {}", e);
                }
            }
        });
        let app = build_router(state);
        let addr = format!("{}:{}", host, port);

        if let Some(tls) = tls_config {
            log::info!("Starting HTTPS server on {}", addr);

            // Install the ring crypto provider (already used by kube-client)
            let _ = rustls::crypto::ring::default_provider().install_default();

            let rustls_config =
                axum_server::tls_rustls::RustlsConfig::from_pem_file(&tls.cert_path, &tls.key_path)
                    .await?;
            let tls_addr: std::net::SocketAddr = addr.parse()?;

            // Start HTTP→HTTPS redirect on port 80 (or port-1 if port != 443)
            let redirect_port = if port == 443 {
                80
            } else {
                port.saturating_sub(1)
            };
            let redirect_host = host.to_string();
            let https_port = port;
            tokio::spawn(async move {
                let redirect_addr = format!("{}:{}", redirect_host, redirect_port);
                log::info!(
                    "Starting HTTP→HTTPS redirect on {} → port {}",
                    redirect_addr,
                    https_port
                );
                let redirect_app =
                    Router::new().fallback(move |req: axum::extract::Request| async move {
                        let host = req
                            .headers()
                            .get("host")
                            .and_then(|h| h.to_str().ok())
                            .unwrap_or("localhost");
                        // Strip port from host header if present
                        let hostname = host.split(':').next().unwrap_or(host);
                        let path = req
                            .uri()
                            .path_and_query()
                            .map(|pq| pq.as_str())
                            .unwrap_or("/");
                        let https_url = if https_port == 443 {
                            format!("https://{}{}", hostname, path)
                        } else {
                            format!("https://{}:{}{}", hostname, https_port, path)
                        };
                        axum::response::Redirect::temporary(&https_url).into_response()
                    });
                match tokio::net::TcpListener::bind(&redirect_addr).await {
                    Ok(listener) => {
                        let _ = axum::serve(listener, redirect_app).await;
                    }
                    Err(e) => {
                        log::warn!(
                            "Could not start HTTP redirect server on {}: {}",
                            redirect_addr,
                            e
                        );
                    }
                }
            });

            axum_server::bind_rustls(tls_addr, rustls_config)
                .serve(app.into_make_service())
                .await?;
        } else {
            log::info!("Starting HTTP server on {}", addr);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }

        Ok(())
    }

    async fn root_handler() -> impl IntoResponse {
        Json(serde_json::json!({
            "service": "veyron",
            "api": "/api/v1",
            "health": "/api/v1/health",
            "console": "/console",
        }))
    }

    // React console (frontend/) — Vite build embedded via include_dir!, served at /console.
    // Local cargo build needs `cd frontend && npm run build` first; Docker runs that stage.
    static CONSOLE_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

    fn console_mime(path: &str) -> &'static str {
        if path.ends_with(".js") {
            "application/javascript"
        } else if path.ends_with(".css") {
            "text/css"
        } else if path.ends_with(".svg") {
            "image/svg+xml"
        } else if path.ends_with(".json") {
            "application/json"
        } else if path.ends_with(".html") {
            "text/html; charset=utf-8"
        } else {
            "application/octet-stream"
        }
    }

    async fn console_handler() -> impl IntoResponse {
        match CONSOLE_DIST.get_file("index.html") {
            Some(f) => (
                [
                    (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                    (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
                ],
                f.contents(),
            )
                .into_response(),
            None => (
                StatusCode::NOT_FOUND,
                "Console build not found — run `npm run build` in frontend/ before `cargo build`.",
            )
                .into_response(),
        }
    }

    async fn console_redirect_handler() -> impl IntoResponse {
        axum::response::Redirect::temporary("/console")
    }

    async fn console_asset_handler(Path(file): Path<String>) -> impl IntoResponse {
        let rel = format!("assets/{file}");
        match CONSOLE_DIST.get_file(&rel) {
            Some(f) => (
                [
                    (header::CONTENT_TYPE, console_mime(&rel)),
                    (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
                ],
                f.contents(),
            )
                .into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }

    fn req_ctx(method: HttpMethod, path: &str) -> RequestContext {
        RequestContext::new(method, path)
    }

    // ── Policy Enforcement ──────────────────────────────────────
    /// Internal error sanitize helper context (policy violation payload).
    struct PolicyViolation {
        policy_name: String,
        enforcement: String,
        message: String,
    }

    async fn check_policies(
        client: &KubeClient,
        config: &crate::config::VMConfig,
    ) -> anyhow::Result<Vec<PolicyViolation>> {
        use crate::operator_crds::VeyronPolicy;

        let api: kube::Api<VeyronPolicy> =
            kube::Api::namespaced(client.client(), &config.namespace);

        let policies = match api.list(&kube::api::ListParams::default()).await {
            Ok(list) => list.items,
            Err(e) => {
                let msg = e.to_string();
                // CRDs not installed or no permission — skip policy check
                if msg.contains("not found")
                    || msg.contains("NotFound")
                    || msg.contains("the server could not find")
                {
                    return Ok(vec![]);
                }
                log::warn!("Policy check failed: {}", msg);
                return Ok(vec![]); // fail-open but log the error
            }
        };

        let mut violations = Vec::new();

        for policy in &policies {
            if !policy.spec.enabled {
                continue;
            }

            for rule in &policy.spec.rules {
                let violated = match rule.condition.as_str() {
                    "max_cpu_cores" => {
                        if let Some(max) = rule.value {
                            config.cpu.cores > max as u32
                        } else {
                            false
                        }
                    }
                    "min_memory_gib" | "min_memory" => {
                        if let Some(min_gib) = rule.value {
                            let mem_gib = crate::utils::parse_memory_gib(&config.memory.size);
                            mem_gib < min_gib
                        } else {
                            false
                        }
                    }
                    "max_disk_gib" => {
                        if let Some(max) = rule.value {
                            let total: f64 = config
                                .disks
                                .iter()
                                .map(|d| crate::utils::parse_memory_gib(&d.size))
                                .sum();
                            total > max
                        } else {
                            false
                        }
                    }
                    "require_cloud_init" => config.cloud_init.is_none(),
                    "require_network" => config.interfaces.is_empty(),
                    _ => false,
                };

                if violated {
                    violations.push(PolicyViolation {
                        policy_name: policy.metadata.name.clone().unwrap_or_default(),
                        enforcement: policy.spec.enforcement_action.clone(),
                        message: rule.message.clone(),
                    });
                }
            }
        }

        Ok(violations)
    }

    /// Maps known error types to safe user-facing messages.
    /// For anything else, returns a generic message to avoid leaking internals.
    fn sanitize_error(e: &dyn std::fmt::Display) -> String {
        // `{}` on an anyhow::Error built with `.context(...)` only shows the
        // outer context message — the real cause (e.g. a KubeVirt admission
        // webhook denial) is invisible to `e.to_string()` and every pattern
        // below silently never matches. `{:#}` (alternate) walks the full
        // chain ("context: cause1: cause2"); harmless no-op for any other
        // Display impl that doesn't special-case the alternate flag.
        let msg = format!("{e:#}");
        // This is the single choke point ~75 handler error paths in this file
        // funnel through before returning a client-facing message — most of
        // them never separately log the real cause, so an error that doesn't
        // match one of the patterns below (falls through to a generic
        // "Internal server error") was previously unrecoverable from the API
        // response AND invisible in the logs. Log it here once, centrally,
        // instead of requiring every call site to remember to.
        log::error!("API error (sanitized for client): {msg}");
        let lower = msg.to_lowercase();

        if lower.starts_with("invalid cron expression") {
            return "Invalid cron expression — use five fields (minute hour day month weekday, UTC), e.g. 0 2 * * *".to_string();
        }

        // Forge VM / CRUD: Kubernetes returns NotFound for missing namespaces and missing APIs —
        // distinguish those instead of collapsing everything to "Resource not found".
        if lower.contains("namespaces") && lower.contains("not found") {
            return "Namespace not found — create the namespace first or use one that exists"
                .to_string();
        }

        // Named KubeVirt resources (VM/VMI not found) — must precede the API-discovery heuristic below.
        if (lower.contains("virtualmachines.kubevirt.io")
            || lower.contains("virtualmachineinstances.kubevirt.io")
            || lower.contains("virtualmachinesnapshot"))
            && lower.contains("not found")
        {
            return "Resource not found".to_string();
        }

        if lower.contains("could not find the requested resource")
            || lower.contains("doesn't have a resource type")
            || (lower.contains("kubevirt.io")
                && lower.contains("could not find")
                && !lower.contains("virtualmachine"))
        {
            return "KubeVirt API unavailable — install KubeVirt (VirtualMachine CRDs must exist on this cluster)"
                .to_string();
        }

        if msg.contains("NodePort") && msg.contains("already used") {
            return msg;
        }

        if lower.contains("guest agent")
            || lower.contains("guest runtime")
            || lower.contains("guestkit")
            || lower.contains("virt-launcher")
            || lower.contains("qemu-agent-command")
        {
            return msg;
        }

        // Kubernetes admission / validation / conflict errors are actionable and
        // carry no secrets (kubectl surfaces them verbatim). Collapsing them to
        // "Internal server error" hides the real cause — e.g. a restore rejected
        // by the KubeVirt webhook, or an AlreadyExists conflict. Surface them.
        if lower.contains("admission webhook")
            || lower.contains("denied the request")
            || lower.contains(" is invalid")
            || lower.contains("already exists")
            || lower.contains("cannot restore")
            || lower.contains("still running")
            || lower.contains("is not running")
            || lower.contains("no vmi found")
            // "<resource> "<name>" is forbidden: exceeded quota: <quota-name>, ..." —
            // doesn't match the bare `starts_with("Forbidden")` check below (K8s
            // prefixes it with the resource/name, not the word "Forbidden"), so this
            // was previously falling all the way through to "Internal server error"
            // even though it's exactly the kind of actionable, no-secrets K8s error
            // this block already exists to surface (e.g. VM creation rejected by a
            // ResourceQuota — the user needs to see which quota and its limit/used).
            || lower.contains("is forbidden")
            || lower.contains("exceeded quota")
            // Every raw KubeVirt/K8s 409 formats as "{message} (Conflict)"
            // (see format_kube_error) — catch the whole family in one place
            // instead of enumerating each specific message going forward.
            || lower.ends_with("(conflict)")
        {
            return msg;
        }

        // Map known error patterns to safe, generic messages
        if msg.starts_with("NotFound") || msg.contains("not found") {
            "Resource not found".to_string()
        } else if msg.starts_with("Conflict") {
            "Resource conflict".to_string()
        } else if msg.starts_with("Unauthorized") {
            "Unauthorized".to_string()
        } else if msg.starts_with("Forbidden") {
            "Forbidden".to_string()
        } else if msg.starts_with("Timeout")
            || lower.contains("timed out")
            // Go's canonical context-timeout string — surfaces verbatim from any
            // k8s-client call that hits its deadline (e.g. a slow apiserver);
            // didn't contain "timed out"/"timeout" so it fell through to the
            // generic 500 despite this branch already existing to handle exactly
            // this category.
            || lower.contains("deadline exceeded")
        {
            "Request timed out".to_string()
        } else if msg.starts_with("connection")
            || lower.contains("connection refused")
            // Same gap as above but for the "service unavailable" bucket: Go's
            // networking-error prefix ("dial tcp ...") and "connection reset by
            // peer" are both common when the apiserver is briefly unreachable
            // (restart/network blip) but neither starts with "connection" nor
            // contains "connection refused".
            || lower.contains("dial tcp")
            || lower.contains("connection reset")
        {
            "Service unavailable".to_string()
        } else {
            "Internal server error".to_string()
        }
    }

    /// Maps a `sanitize_error()` output back to the HTTP status it actually
    /// represents. Handlers acting on a named VM (start/stop/restart/pause/
    /// unpause/delete/migrate/clone) previously hardcoded 500 for every
    /// failure — including "Resource not found" (the VM was deleted/renamed
    /// out from under the caller) and "is forbidden"/quota errors (like the
    /// create-VM 403 fix) — which is misleading for a client trying to
    /// distinguish "retry me" from "this VM doesn't exist"/"you're not
    /// allowed to do this". Call with the *sanitized* message (not the raw
    /// error) so this only ever inspects the same safe text already sent to
    /// the client.
    fn status_for_sanitized_error(msg: &str) -> u16 {
        let lower = msg.to_lowercase();
        if lower.contains("not found") {
            404
        } else if lower.contains("already exists") || lower.contains("conflict") {
            409
        } else if lower.contains("forbidden") || lower.contains("exceeded quota") {
            403
        } else if lower.contains("unauthorized") {
            401
        } else if lower.contains("timed out") || lower.contains("service unavailable") {
            503
        } else {
            500
        }
    }

    // ── VM Endpoints ──────────────────────────────────────────────

    fn ok_json<T: Serialize>(data: &T) -> (StatusCode, Json<serde_json::Value>) {
        let value = serde_json::to_value(data).unwrap_or_else(
            |e| serde_json::json!({"error": format!("serialization failed: {}", e)}),
        );
        // Extract the status code from the serialized response if present,
        // so that 201/204/etc. responses get the correct HTTP status.
        let status_code = value
            .get("status")
            .and_then(|v| v.as_u64())
            .and_then(|s| StatusCode::from_u16(s as u16).ok())
            .unwrap_or(StatusCode::OK);
        (status_code, Json(value))
    }

    fn err_json(status: u16, code: &str, message: &str) -> (StatusCode, Json<serde_json::Value>) {
        let ctx = req_ctx(HttpMethod::GET, "");
        let resp = ApiResponse::error(status, code, message, &ctx.request_id);
        let value = serde_json::to_value(&resp).unwrap_or_else(
            |_| serde_json::json!({"status": status, "error": code, "message": "internal error"}),
        );
        (
            StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(value),
        )
    }

    fn format_age(ts: &chrono::DateTime<chrono::Utc>) -> String {
        let now = chrono::Utc::now();
        let dur = now.signed_duration_since(*ts);
        let mins = dur.num_minutes();
        if mins < 1 {
            "just now".to_string()
        } else if mins < 60 {
            format!("{}m", mins)
        } else if mins < 1440 {
            format!("{}h", mins / 60)
        } else {
            format!("{}d{}h", mins / 1440, (mins % 1440) / 60)
        }
    }

    fn parse_memory(s: &str) -> u64 {
        let s = s.trim();
        if let Some(val) = s.strip_suffix("Ti") {
            (val.parse::<f64>().unwrap_or(0.0) * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64
        } else if let Some(val) = s.strip_suffix("Gi") {
            (val.parse::<f64>().unwrap_or(0.0) * 1024.0 * 1024.0 * 1024.0) as u64
        } else if let Some(val) = s.strip_suffix("Mi") {
            (val.parse::<f64>().unwrap_or(0.0) * 1024.0 * 1024.0) as u64
        } else if let Some(val) = s.strip_suffix("Ki") {
            (val.parse::<f64>().unwrap_or(0.0) * 1024.0) as u64
        } else if let Some(val) = s.strip_suffix('T') {
            (val.parse::<f64>().unwrap_or(0.0) * 1_000_000_000_000.0) as u64
        } else if let Some(val) = s.strip_suffix('G') {
            (val.parse::<f64>().unwrap_or(0.0) * 1_000_000_000.0) as u64
        } else if let Some(val) = s.strip_suffix('M') {
            (val.parse::<f64>().unwrap_or(0.0) * 1_000_000.0) as u64
        } else {
            s.parse::<u64>().unwrap_or(0)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use axum::http::Method;

        #[test]
        fn cors_allows_every_verb_the_routes_use() {
            for m in ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"] {
                assert!(CORS_METHODS.iter().any(|x| x.as_str() == m), "{m} missing");
            }
        }

        #[test]
        fn readonly_can_read_but_not_mutate() {
            assert!(authorize_request(ApiRole::ReadOnly, &Method::GET, "/api/v1/vms").is_ok());
            let err = authorize_request(ApiRole::ReadOnly, &Method::POST, "/api/v1/vms")
                .expect_err("readonly must not create VMs");
            assert_eq!(err.0, StatusCode::FORBIDDEN);
        }

        #[test]
        fn write_can_mutate_but_admin_routes_stay_admin() {
            assert!(authorize_request(ApiRole::Write, &Method::POST, "/api/v1/vms/bulk").is_ok());
            assert!(
                authorize_request(ApiRole::Write, &Method::POST, "/api/v1/platform/upgrade")
                    .is_err()
            );
            assert!(
                authorize_request(ApiRole::Admin, &Method::POST, "/api/v1/platform/upgrade")
                    .is_ok()
            );
        }

        #[test]
        fn k8s_name_validation_rejects_traversal() {
            assert!(is_valid_k8s_name("ubuntu-demo"));
            assert!(!is_valid_k8s_name("../etc"));
            assert!(!is_valid_k8s_name("UPPER"));
            assert!(!is_valid_k8s_name(""));
        }
    }
}
