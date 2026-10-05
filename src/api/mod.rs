// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "web")]
pub mod atlas;
#[cfg(feature = "web")]
pub mod auth_context;
#[cfg(feature = "web")]
pub mod dashboard_paths;
pub mod handlers;
pub mod http_server;
#[cfg(feature = "web")]
pub mod integrations;
pub mod kryton;
#[cfg(feature = "web")]
pub mod loki;
pub mod middleware;
#[cfg(feature = "web")]
pub mod oidc;
pub mod openapi;
#[cfg(feature = "web")]
pub mod opencost;
pub mod pagination;
pub mod pam_auth;
#[cfg(feature = "web")]
pub mod prometheus;
pub mod routes;
pub mod server;
pub mod webhooks;
pub mod websocket;
#[cfg(feature = "web")]
pub mod ws_ticket;

/// API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
    pub cors_enabled: bool,
    pub cors_origins: Vec<String>,
    pub auth_enabled: bool,
    pub auth_method: AuthMethod,
    pub rate_limit: RateLimitConfig,
    pub max_request_size: usize,
    pub request_timeout_secs: u64,
    pub created_at: DateTime<Utc>,
}

/// Authentication method
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    None,
    ApiKey,
    Bearer,
    BasicAuth,
    OAuth2,
    MTLS,
}

impl std::fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMethod::None => write!(f, "none"),
            AuthMethod::ApiKey => write!(f, "api-key"),
            AuthMethod::Bearer => write!(f, "bearer"),
            AuthMethod::BasicAuth => write!(f, "basic"),
            AuthMethod::OAuth2 => write!(f, "oauth2"),
            AuthMethod::MTLS => write!(f, "mtls"),
        }
    }
}

impl AuthMethod {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => Some(AuthMethod::None),
            "api-key" | "apikey" => Some(AuthMethod::ApiKey),
            "bearer" | "token" => Some(AuthMethod::Bearer),
            "basic" | "basic-auth" => Some(AuthMethod::BasicAuth),
            "oauth2" | "oauth" => Some(AuthMethod::OAuth2),
            "mtls" | "mutual-tls" => Some(AuthMethod::MTLS),
            _ => None,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub per_client: bool,
}

impl RateLimitConfig {
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            enabled: true,
            requests_per_minute,
            burst_size: std::cmp::max(1, requests_per_minute / 2),
            per_client: true,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            requests_per_minute: 0,
            burst_size: 0,
            per_client: false,
        }
    }

    pub fn with_burst(mut self, burst_size: u32) -> Self {
        self.burst_size = burst_size;
        self
    }

    pub fn global(mut self) -> Self {
        self.per_client = false;
        self
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::new(60)
    }
}

impl ApiConfig {
    pub fn new(port: u16) -> Self {
        Self {
            id: format!("api-{}", Utc::now().timestamp()),
            host: "127.0.0.1".to_string(),
            port,
            tls_enabled: false,
            tls_cert: None,
            tls_key: None,
            cors_enabled: false,
            cors_origins: vec![],
            auth_enabled: true,
            auth_method: AuthMethod::None,
            rate_limit: RateLimitConfig::default(),
            max_request_size: 10 * 1024 * 1024, // 10MB
            request_timeout_secs: 30,
            created_at: Utc::now(),
        }
    }

    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn with_tls(mut self, cert: impl Into<String>, key: impl Into<String>) -> Self {
        self.tls_enabled = true;
        self.tls_cert = Some(cert.into());
        self.tls_key = Some(key.into());
        self
    }

    pub fn with_auth(mut self, method: AuthMethod) -> Self {
        self.auth_enabled = method != AuthMethod::None;
        self.auth_method = method;
        self
    }

    pub fn with_cors_origins(mut self, origins: Vec<String>) -> Self {
        self.cors_origins = origins;
        self
    }

    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit = config;
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.request_timeout_secs = secs;
        self
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn base_url(&self) -> String {
        let scheme = if self.tls_enabled { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.host, self.port)
    }

    pub fn is_secure(&self) -> bool {
        self.tls_enabled && self.auth_enabled
    }
}

/// API request context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub request_id: String,
    pub method: HttpMethod,
    pub path: String,
    pub client_ip: String,
    pub user_agent: Option<String>,
    pub auth_user: Option<String>,
    pub namespace: String,
    pub timestamp: DateTime<Utc>,
}

/// HTTP method
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    OPTIONS,
    HEAD,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::PUT => write!(f, "PUT"),
            HttpMethod::PATCH => write!(f, "PATCH"),
            HttpMethod::DELETE => write!(f, "DELETE"),
            HttpMethod::OPTIONS => write!(f, "OPTIONS"),
            HttpMethod::HEAD => write!(f, "HEAD"),
        }
    }
}

impl HttpMethod {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::GET),
            "POST" => Some(HttpMethod::POST),
            "PUT" => Some(HttpMethod::PUT),
            "PATCH" => Some(HttpMethod::PATCH),
            "DELETE" => Some(HttpMethod::DELETE),
            "OPTIONS" => Some(HttpMethod::OPTIONS),
            "HEAD" => Some(HttpMethod::HEAD),
            _ => None,
        }
    }

    pub fn is_safe(&self) -> bool {
        matches!(
            self,
            HttpMethod::GET | HttpMethod::HEAD | HttpMethod::OPTIONS
        )
    }

    pub fn is_idempotent(&self) -> bool {
        matches!(
            self,
            HttpMethod::GET
                | HttpMethod::PUT
                | HttpMethod::DELETE
                | HttpMethod::HEAD
                | HttpMethod::OPTIONS
        )
    }
}

impl RequestContext {
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        Self {
            request_id: format!(
                "req-{}-{:016x}",
                Utc::now().timestamp_micros(),
                rand::random::<u64>()
            ),
            method,
            path: path.into(),
            client_ip: "127.0.0.1".to_string(),
            user_agent: None,
            auth_user: None,
            namespace: "default".to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn with_client_ip(mut self, ip: impl Into<String>) -> Self {
        self.client_ip = ip.into();
        self
    }

    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.auth_user = Some(user.into());
        self
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth_user.is_some()
    }
}

/// API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub status: u16,
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub metadata: ResponseMetadata,
}

/// API error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: Option<u64>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T, request_id: &str) -> Self {
        Self {
            status: 200,
            success: true,
            data: Some(data),
            error: None,
            metadata: ResponseMetadata {
                request_id: request_id.to_string(),
                timestamp: Utc::now(),
                duration_ms: None,
            },
        }
    }

    pub fn created(data: T, request_id: &str) -> Self {
        Self {
            status: 201,
            success: true,
            data: Some(data),
            error: None,
            metadata: ResponseMetadata {
                request_id: request_id.to_string(),
                timestamp: Utc::now(),
                duration_ms: None,
            },
        }
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.metadata.duration_ms = Some(duration_ms);
        self
    }
}

impl ApiResponse<()> {
    pub fn no_content(request_id: &str) -> Self {
        Self {
            status: 204,
            success: true,
            data: None,
            error: None,
            metadata: ResponseMetadata {
                request_id: request_id.to_string(),
                timestamp: Utc::now(),
                duration_ms: None,
            },
        }
    }

    pub fn error(status: u16, code: &str, message: &str, request_id: &str) -> Self {
        Self {
            status,
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
            metadata: ResponseMetadata {
                request_id: request_id.to_string(),
                timestamp: Utc::now(),
                duration_ms: None,
            },
        }
    }

    pub fn not_found(resource: &str, request_id: &str) -> Self {
        Self::error(
            404,
            "NOT_FOUND",
            &format!("{} not found", resource),
            request_id,
        )
    }

    pub fn unauthorized(request_id: &str) -> Self {
        Self::error(401, "UNAUTHORIZED", "Authentication required", request_id)
    }

    pub fn forbidden(request_id: &str) -> Self {
        Self::error(403, "FORBIDDEN", "Insufficient permissions", request_id)
    }

    pub fn bad_request(message: &str, request_id: &str) -> Self {
        Self::error(400, "BAD_REQUEST", message, request_id)
    }

    pub fn rate_limited(request_id: &str) -> Self {
        Self::error(429, "RATE_LIMITED", "Too many requests", request_id)
    }

    pub fn internal_error(request_id: &str) -> Self {
        Self::error(500, "INTERNAL_ERROR", "Internal server error", request_id)
    }
}

/// API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub permissions: Vec<String>,
    pub rate_limit: Option<u32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

impl ApiKey {
    pub fn new(name: impl Into<String>, key_hash: impl Into<String>) -> Self {
        let name_str = name.into();
        Self {
            id: format!(
                "key-{}-{}",
                name_str.to_lowercase().replace(' ', "-"),
                Utc::now().timestamp_micros()
            ),
            name: name_str,
            key_hash: key_hash.into(),
            permissions: vec!["read".to_string()],
            rate_limit: None,
            expires_at: None,
            enabled: true,
            created_at: Utc::now(),
            last_used: None,
        }
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_rate_limit(mut self, limit: u32) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn record_usage(&mut self) {
        self.last_used = Some(Utc::now());
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        self.enabled && !self.is_expired()
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions
            .iter()
            .any(|p| p == permission || p == "admin")
    }
}

/// API key manager
pub struct ApiKeyManager {
    keys: HashMap<String, ApiKey>,
}

impl ApiKeyManager {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    pub fn add_key(&mut self, key: ApiKey) -> String {
        let id = key.id.clone();
        self.keys.insert(id.clone(), key);
        id
    }

    pub fn get_key(&self, id: &str) -> Option<&ApiKey> {
        self.keys.get(id)
    }

    pub fn get_key_mut(&mut self, id: &str) -> Option<&mut ApiKey> {
        self.keys.get_mut(id)
    }

    pub fn remove_key(&mut self, id: &str) -> bool {
        self.keys.remove(id).is_some()
    }

    pub fn list_keys(&self) -> Vec<&ApiKey> {
        self.keys.values().collect()
    }

    pub fn active_keys(&self) -> Vec<&ApiKey> {
        self.keys.values().filter(|k| k.is_valid()).collect()
    }

    pub fn expired_keys(&self) -> Vec<&ApiKey> {
        self.keys.values().filter(|k| k.is_expired()).collect()
    }

    pub fn key_count(&self) -> usize {
        self.keys.len()
    }
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_new() {
        let config = ApiConfig::new(8080);
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "127.0.0.1");
        assert!(!config.tls_enabled);
        assert!(!config.cors_enabled);
        assert!(config.auth_enabled);
    }

    #[test]
    fn test_api_config_with_host() {
        let config = ApiConfig::new(8080).with_host("127.0.0.1");
        assert_eq!(config.host, "127.0.0.1");
    }

    #[test]
    fn test_api_config_with_tls() {
        let config = ApiConfig::new(443).with_tls("/path/cert.pem", "/path/key.pem");
        assert!(config.tls_enabled);
        assert_eq!(config.tls_cert, Some("/path/cert.pem".to_string()));
    }

    #[test]
    fn test_api_config_with_auth() {
        let config = ApiConfig::new(8080).with_auth(AuthMethod::Bearer);
        assert!(config.auth_enabled);
        assert_eq!(config.auth_method, AuthMethod::Bearer);
    }

    #[test]
    fn test_api_config_address() {
        let config = ApiConfig::new(8080);
        assert_eq!(config.address(), "127.0.0.1:8080");
    }

    #[test]
    fn test_api_config_base_url() {
        let config = ApiConfig::new(8080);
        assert_eq!(config.base_url(), "http://127.0.0.1:8080");

        let tls_config = ApiConfig::new(443).with_tls("cert", "key");
        assert_eq!(tls_config.base_url(), "https://127.0.0.1:443");
    }

    #[test]
    fn test_api_config_is_secure() {
        let config = ApiConfig::new(443)
            .with_tls("cert", "key")
            .with_auth(AuthMethod::Bearer);
        assert!(config.is_secure());

        let insecure = ApiConfig::new(8080);
        assert!(!insecure.is_secure());
    }

    #[test]
    fn test_auth_method_display() {
        assert_eq!(AuthMethod::None.to_string(), "none");
        assert_eq!(AuthMethod::ApiKey.to_string(), "api-key");
        assert_eq!(AuthMethod::Bearer.to_string(), "bearer");
        assert_eq!(AuthMethod::BasicAuth.to_string(), "basic");
        assert_eq!(AuthMethod::OAuth2.to_string(), "oauth2");
        assert_eq!(AuthMethod::MTLS.to_string(), "mtls");
    }

    #[test]
    fn test_auth_method_from_str() {
        assert_eq!(AuthMethod::parse("bearer"), Some(AuthMethod::Bearer));
        assert_eq!(AuthMethod::parse("token"), Some(AuthMethod::Bearer));
        assert_eq!(AuthMethod::parse("apikey"), Some(AuthMethod::ApiKey));
        assert_eq!(AuthMethod::parse("oauth2"), Some(AuthMethod::OAuth2));
        assert_eq!(AuthMethod::parse("mtls"), Some(AuthMethod::MTLS));
        assert_eq!(AuthMethod::parse("unknown"), None);
    }

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::new(120);
        assert!(config.enabled);
        assert_eq!(config.requests_per_minute, 120);
        assert_eq!(config.burst_size, 60);
        assert!(config.per_client);
    }

    #[test]
    fn test_rate_limit_disabled() {
        let config = RateLimitConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_rate_limit_with_burst() {
        let config = RateLimitConfig::new(100).with_burst(200);
        assert_eq!(config.burst_size, 200);
    }

    #[test]
    fn test_rate_limit_global() {
        let config = RateLimitConfig::new(100).global();
        assert!(!config.per_client);
    }

    #[test]
    fn test_http_method_display() {
        assert_eq!(HttpMethod::GET.to_string(), "GET");
        assert_eq!(HttpMethod::POST.to_string(), "POST");
        assert_eq!(HttpMethod::DELETE.to_string(), "DELETE");
    }

    #[test]
    fn test_http_method_from_str() {
        assert_eq!(HttpMethod::parse("GET"), Some(HttpMethod::GET));
        assert_eq!(HttpMethod::parse("post"), Some(HttpMethod::POST));
        assert_eq!(HttpMethod::parse("INVALID"), None);
    }

    #[test]
    fn test_http_method_is_safe() {
        assert!(HttpMethod::GET.is_safe());
        assert!(HttpMethod::HEAD.is_safe());
        assert!(HttpMethod::OPTIONS.is_safe());
        assert!(!HttpMethod::POST.is_safe());
        assert!(!HttpMethod::DELETE.is_safe());
    }

    #[test]
    fn test_http_method_is_idempotent() {
        assert!(HttpMethod::GET.is_idempotent());
        assert!(HttpMethod::PUT.is_idempotent());
        assert!(HttpMethod::DELETE.is_idempotent());
        assert!(!HttpMethod::POST.is_idempotent());
        assert!(!HttpMethod::PATCH.is_idempotent());
    }

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new(HttpMethod::GET, "/api/v1/vms");
        assert_eq!(ctx.method, HttpMethod::GET);
        assert_eq!(ctx.path, "/api/v1/vms");
        assert!(!ctx.is_authenticated());
    }

    #[test]
    fn test_request_context_with_user() {
        let ctx = RequestContext::new(HttpMethod::POST, "/api/v1/vms")
            .with_user("admin")
            .with_client_ip("10.0.0.1");
        assert!(ctx.is_authenticated());
        assert_eq!(ctx.auth_user, Some("admin".to_string()));
        assert_eq!(ctx.client_ip, "10.0.0.1");
    }

    #[test]
    fn test_api_response_success() {
        let resp = ApiResponse::success("hello", "req-1");
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert!(resp.data.is_some());
    }

    #[test]
    fn test_api_response_created() {
        let resp = ApiResponse::created("new-vm", "req-1");
        assert_eq!(resp.status, 201);
        assert!(resp.success);
    }

    #[test]
    fn test_api_response_no_content() {
        let resp = ApiResponse::no_content("req-1");
        assert_eq!(resp.status, 204);
        assert!(resp.success);
    }

    #[test]
    fn test_api_response_not_found() {
        let resp = ApiResponse::not_found("VM", "req-1");
        assert_eq!(resp.status, 404);
        assert!(!resp.success);
    }

    #[test]
    fn test_api_response_unauthorized() {
        let resp = ApiResponse::unauthorized("req-1");
        assert_eq!(resp.status, 401);
    }

    #[test]
    fn test_api_response_forbidden() {
        let resp = ApiResponse::forbidden("req-1");
        assert_eq!(resp.status, 403);
    }

    #[test]
    fn test_api_response_bad_request() {
        let resp = ApiResponse::bad_request("invalid input", "req-1");
        assert_eq!(resp.status, 400);
    }

    #[test]
    fn test_api_response_rate_limited() {
        let resp = ApiResponse::rate_limited("req-1");
        assert_eq!(resp.status, 429);
    }

    #[test]
    fn test_api_response_internal_error() {
        let resp = ApiResponse::internal_error("req-1");
        assert_eq!(resp.status, 500);
    }

    #[test]
    fn test_api_response_with_duration() {
        let resp = ApiResponse::success("data", "req-1").with_duration(42);
        assert_eq!(resp.metadata.duration_ms, Some(42));
    }

    #[test]
    fn test_api_key_new() {
        let key = ApiKey::new("test-key", "hash123");
        assert_eq!(key.name, "test-key");
        assert!(key.enabled);
        assert!(!key.is_expired());
        assert!(key.is_valid());
    }

    #[test]
    fn test_api_key_permissions() {
        let key = ApiKey::new("key", "hash")
            .with_permissions(vec!["read".to_string(), "write".to_string()]);
        assert!(key.has_permission("read"));
        assert!(key.has_permission("write"));
        assert!(!key.has_permission("admin"));
    }

    #[test]
    fn test_api_key_admin_permission() {
        let key = ApiKey::new("key", "hash").with_permissions(vec!["admin".to_string()]);
        assert!(key.has_permission("read"));
        assert!(key.has_permission("write"));
        assert!(key.has_permission("anything"));
    }

    #[test]
    fn test_api_key_disable_enable() {
        let mut key = ApiKey::new("key", "hash");
        assert!(key.is_valid());

        key.disable();
        assert!(!key.is_valid());

        key.enable();
        assert!(key.is_valid());
    }

    #[test]
    fn test_api_key_record_usage() {
        let mut key = ApiKey::new("key", "hash");
        assert!(key.last_used.is_none());

        key.record_usage();
        assert!(key.last_used.is_some());
    }

    #[test]
    fn test_api_key_manager() {
        let mut manager = ApiKeyManager::new();
        let key = ApiKey::new("test", "hash");
        let id = manager.add_key(key);

        assert_eq!(manager.key_count(), 1);
        assert!(manager.get_key(&id).is_some());
    }

    #[test]
    fn test_api_key_manager_remove() {
        let mut manager = ApiKeyManager::new();
        let id = manager.add_key(ApiKey::new("test", "hash"));

        assert!(manager.remove_key(&id));
        assert!(!manager.remove_key(&id));
    }

    #[test]
    fn test_api_key_manager_active_keys() {
        let mut manager = ApiKeyManager::new();

        let mut disabled_key = ApiKey::new("disabled", "hash1");
        disabled_key.disable();

        manager.add_key(ApiKey::new("active", "hash2"));
        manager.add_key(disabled_key);

        assert_eq!(manager.active_keys().len(), 1);
    }

    #[test]
    fn test_api_config_with_timeout() {
        let config = ApiConfig::new(8080).with_timeout(60);
        assert_eq!(config.request_timeout_secs, 60);
    }

    #[test]
    fn test_api_config_with_cors_origins() {
        let config =
            ApiConfig::new(8080).with_cors_origins(vec!["https://example.com".to_string()]);
        assert_eq!(config.cors_origins.len(), 1);
        assert_eq!(config.cors_origins[0], "https://example.com");
    }
}
