// HTTP Server - Real web server using axum for serving API and dashboard
//
// This module is only compiled when the "web" feature is enabled.

#[cfg(feature = "web")]
pub mod web {
    use crate::api::{ApiResponse, HttpMethod, RequestContext};
    use crate::config::VMConfigBuilder;
    use crate::kube::KubeClient;
    use crate::profiles::PROFILES;
    use crate::templates::TEMPLATES;
    use crate::tui::state::VmInfo;
    use axum::{
        Router,
        extract::{DefaultBodyLimit, Path, Query, State, WebSocketUpgrade, ws::{Message, WebSocket}},
        http::{HeaderMap, StatusCode, header},
        middleware,
        response::{Html, IntoResponse, Json},
        routing::{delete, get, post},
    };
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tower_http::cors::{AllowOrigin, CorsLayer};
    use tower_http::timeout::TimeoutLayer;

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
        pub api_key: Option<String>,
        pub api_keys: Vec<ApiKeyEntry>,
        rate_limiter: RateLimiterState,
    }

    impl WebState {
        pub async fn new(namespace: String, rate_limit_per_minute: u64) -> anyhow::Result<Self> {
            let api_key = std::env::var("VMROGUE_API_KEY")
                .ok()
                .filter(|k| !k.is_empty());
            if api_key.is_none() {
                log::warn!(
                    "VMROGUE_API_KEY is not set - API will reject all requests. \
                     Set VMROGUE_API_KEY to enable access."
                );
            }

            // Load additional API keys with roles from VMROGUE_API_KEYS
            // Format: "name1:key1:admin,name2:key2:readonly,name3:key3:write"
            let mut api_keys = Vec::new();
            if let Some(ref primary) = api_key {
                api_keys.push(ApiKeyEntry {
                    key: primary.clone(),
                    role: ApiRole::Admin,
                    name: "primary".to_string(),
                });
            }
            if let Ok(keys_str) = std::env::var("VMROGUE_API_KEYS") {
                for entry in keys_str.split(',') {
                    let parts: Vec<&str> = entry.trim().split(':').collect();
                    if parts.len() >= 2 {
                        let name = parts[0].to_string();
                        let key = parts[1].to_string();
                        let role = match parts.get(2).map(|s| *s) {
                            Some("admin") => ApiRole::Admin,
                            Some("write") => ApiRole::Write,
                            Some("readonly") | Some("read") => ApiRole::ReadOnly,
                            _ => ApiRole::ReadOnly,
                        };
                        api_keys.push(ApiKeyEntry { key, role, name });
                    }
                }
            }

            let kube_client = KubeClient::new().await?;
            Ok(Self {
                namespace,
                kube_client,
                api_key,
                api_keys,
                rate_limiter: RateLimiterState::new(rate_limit_per_minute, 60),
            })
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

        /// Authenticate a JWT Bearer token. Validates standard claims
        /// (exp, iss) and extracts role from configurable claim.
        ///
        /// Set VMROGUE_JWT_ISSUER to enable JWT validation.
        /// Set VMROGUE_JWT_ROLE_CLAIM to customize role claim (default: "role").
        /// JWT roles: "admin", "write", "readonly" (default: "readonly").
        pub fn authenticate_jwt(&self, token: &str) -> Option<ApiRole> {
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() != 3 {
                return None;
            }

            // Decode payload (part 1)
            let payload_bytes = base64url_decode(parts[1])?;
            let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;

            // Check expiration
            if let Some(exp) = payload.get("exp").and_then(|v| v.as_i64()) {
                let now = chrono::Utc::now().timestamp();
                if now > exp {
                    log::debug!("JWT token expired");
                    return None;
                }
            }

            // Check issuer if configured
            if let Ok(expected_issuer) = std::env::var("VMROGUE_JWT_ISSUER") {
                let iss = payload.get("iss").and_then(|v| v.as_str()).unwrap_or("");
                if iss != expected_issuer {
                    log::debug!("JWT issuer mismatch: got '{}', expected '{}'", iss, expected_issuer);
                    return None;
                }
            } else {
                // No issuer configured — JWT auth is disabled
                return None;
            }

            // Extract role from claim
            let role_claim = std::env::var("VMROGUE_JWT_ROLE_CLAIM")
                .unwrap_or_else(|_| "role".to_string());
            let role_str = payload
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
    }

    /// Decode a base64url-encoded string (no padding).
    fn base64url_decode(input: &str) -> Option<Vec<u8>> {
        let mut s = input.replace('-', "+").replace('_', "/");
        match s.len() % 4 {
            2 => s.push_str("=="),
            3 => s.push('='),
            0 => {}
            _ => return None,
        }
        base64::engine::Engine::decode(&base64::engine::general_purpose::STANDARD, &s).ok()
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

    /// API key authentication middleware.
    ///
    /// If the `VMROGUE_API_KEY` env var was set at startup, every request must present
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
        if path == "/api/v1/health"
            || path == "/dashboard"
            || path.starts_with("/assets/")
        {
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
                    "API key not configured. Set VMROGUE_API_KEY environment variable.",
                );
                return (status, json).into_response();
            }
        };
        drop(s);

        // Check X-API-Key header, Authorization: Bearer, or ?token= query param (for WebSocket)
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
                request
                    .uri()
                    .query()
                    .and_then(|q| {
                        q.split('&')
                            .find_map(|p| p.strip_prefix("token=").map(|s| s.to_string()))
                    })
            });

        match provided_key {
            Some(key) if constant_time_eq(key.as_bytes(), expected_key.as_bytes()) => {
                // Primary key — check RBAC via multi-key table
                let s2 = state.read().await;
                if let Some(role) = s2.authenticate(&key) {
                    // Read-only keys cannot make mutating requests
                    if *role == ApiRole::ReadOnly
                        && request.method() != axum::http::Method::GET
                        && request.method() != axum::http::Method::HEAD
                    {
                        drop(s2);
                        let (status, json) = err_json(
                            403,
                            "FORBIDDEN",
                            "Read-only API key cannot perform mutating operations",
                        );
                        return (status, json).into_response();
                    }
                }
                drop(s2);
                next.run(request).await.into_response()
            }
            Some(key) => {
                // Check multi-key table for additional keys
                let s2 = state.read().await;
                let role = s2.authenticate(&key).cloned().or_else(|| {
                    // Try JWT Bearer token validation
                    s2.authenticate_jwt(&key)
                });
                match role {
                    Some(role) => {
                        if role == ApiRole::ReadOnly
                            && request.method() != axum::http::Method::GET
                            && request.method() != axum::http::Method::HEAD
                        {
                            drop(s2);
                            let (status, json) = err_json(
                                403,
                                "FORBIDDEN",
                                "Read-only token cannot perform mutating operations",
                            );
                            return (status, json).into_response();
                        }
                        drop(s2);
                        next.run(request).await.into_response()
                    }
                    None => {
                        drop(s2);
                        let (status, json) =
                            err_json(401, "UNAUTHORIZED", "Invalid or missing API key");
                        (status, json).into_response()
                    }
                }
            }
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
        if path == "/api/v1/health"
            || path == "/dashboard"
            || path.starts_with("/api/v1/vms")
            || path.starts_with("/api/v1/events")
            || path.starts_with("/api/v1/nodes")
            || path.starts_with("/api/v1/pods")
            || path.starts_with("/api/v1/snapshots")
            || path.starts_with("/api/v1/dashboard")
            || path.starts_with("/api/v1/templates")
            || path.starts_with("/api/v1/profiles")
            || path.starts_with("/api/v1/namespaces")
            || path.starts_with("/api/v1/activity")
        {
            return next.run(request).await.into_response();
        }

        let s = state.read().await;
        if !s.rate_limiter.check_rate_limit() {
            let (status, json) =
                err_json(429, "RATE_LIMITED", "Too many requests. Please slow down.");
            return (status, json).into_response();
        }
        drop(s);
        next.run(request).await.into_response()
    }

    // ── WebSocket Metrics Streaming ──────────────────────────────

    async fn metrics_websocket_handler(
        ws: WebSocketUpgrade,
        State(state): State<SharedState>,
    ) -> impl IntoResponse {
        ws.on_upgrade(move |socket| metrics_stream(socket, state))
    }

    async fn metrics_stream(mut socket: WebSocket, state: SharedState) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            interval.tick().await;

            // Clone client outside the lock to avoid holding it during K8s calls
            let client = {
                let s = state.read().await;
                s.client().clone()
            };

            let metrics = {
                let vms = client.list_all_vms().await.unwrap_or_default();
                let nodes = client.list_nodes().await.unwrap_or_default();

                let total_vms = vms.len() as u32;
                let running_vms = vms
                    .iter()
                    .filter(|vm| {
                        vm.status
                            .as_ref()
                            .and_then(|s| s.printable_status.as_deref())
                            .map(|s| s == "Running")
                            .unwrap_or(false)
                    })
                    .count() as u32;

                let mut total_cpu: u32 = 0;
                let mut total_mem: u64 = 0;
                for node in &nodes {
                    if let Some(cap) = node.status.as_ref().and_then(|s| s.capacity.as_ref()) {
                        if let Some(cpu) = cap.get("cpu") {
                            total_cpu += cpu.0.parse::<u32>().unwrap_or(0);
                        }
                        if let Some(mem) = cap.get("memory") {
                            total_mem += crate::utils::parse_memory_bytes(&mem.0);
                        }
                    }
                }

                serde_json::json!({
                    "type": "cluster_metrics",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "total_vms": total_vms,
                    "running_vms": running_vms,
                    "total_cpu_cores": total_cpu,
                    "total_memory_bytes": total_mem,
                    "node_count": nodes.len(),
                })
            };

            if socket
                .send(Message::Text(metrics.to_string().into()))
                .await
                .is_err()
            {
                break; // Client disconnected
            }
        }
    }

    // ── Security headers middleware ─────────────────────────────

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
        headers.insert("content-security-policy", "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com; connect-src 'self' ws: wss:".parse().unwrap());
        headers.insert("referrer-policy", "no-referrer".parse().unwrap());
        // HSTS: instruct browsers to only use HTTPS for 1 year
        headers.insert(
            "strict-transport-security",
            "max-age=31536000; includeSubDomains".parse().unwrap(),
        );
        response
    }

    // ── CORS configuration ──────────────────────────────────────

    /// Build a CORS layer. If `VMROGUE_CORS_ORIGINS` is set (comma-separated
    /// list of origins), allow those origins. Otherwise default to same-origin
    /// only (no extra origins allowed).
    fn build_cors_layer() -> CorsLayer {
        let origins = std::env::var("VMROGUE_CORS_ORIGINS").ok();

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
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::DELETE,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                "x-api-key".parse().unwrap(),
            ])
    }

    // ── Router ──────────────────────────────────────────────────

    pub fn build_router(state: SharedState) -> Router {
        Router::new()
            // Dashboard & static assets
            .route("/dashboard", get(dashboard_handler))
            .route("/assets/novnc.min.js", get(novnc_handler))
            // VM endpoints
            .route("/api/v1/vms", get(list_vms_handler))
            .route("/api/v1/vms", post(create_vm_handler))
            // Batch must be before parameterized :ns/:name routes to avoid ambiguity
            .route("/api/v1/vms/batch", post(batch_vm_handler))
            .route("/api/v1/vms/:ns/:name", get(get_vm_handler))
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
            .route("/api/v1/vms/:ns/:name", axum::routing::put(update_vm_handler))
            .route("/api/v1/vms/:ns/:name/vnc", get(vnc_websocket_handler))
            .route("/api/v1/ws/metrics", get(metrics_websocket_handler))
            .route("/api/v1/vms/:ns/:name/security", get(vm_security_handler))
            .route("/api/v1/vms/:ns/:name/events", get(vm_events_handler))
            // Snapshots
            .route("/api/v1/snapshots", get(list_snapshots_handler))
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
            // Events
            .route("/api/v1/events", get(list_events_handler))
            .route("/api/v1/events/recent", get(recent_events_handler))
            // Cluster resources
            .route("/api/v1/nodes", get(list_nodes_handler))
            .route("/api/v1/pods", get(list_pods_handler))
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
            // Health
            .route("/api/v1/health", get(health_handler))
            .with_state(state.clone())
            // Merge handler sub-routers (real K8s data + stubs)
            // Merged after .with_state() because handler routers manage their own state
            .merge(crate::api::handlers::all_routes(state.clone()))
            // Layers applied in reverse order (outermost = last .layer() call)
            .layer(middleware::from_fn(security_headers_middleware))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                rate_limit_middleware,
            ))
            .layer(middleware::from_fn_with_state(state, auth_middleware))
            .layer(build_cors_layer())
            // Request timeout: 30 seconds
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                std::time::Duration::from_secs(30),
            ))
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

        // Initialize kube client at startup instead of lazily per-request
        let state = Arc::new(RwLock::new(
            WebState::new(namespace, rate_limit_per_minute).await?,
        ));
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
            let redirect_port = if port == 443 { 80 } else { port.saturating_sub(1) };
            let redirect_host = host.to_string();
            let https_port = port;
            tokio::spawn(async move {
                let redirect_addr = format!("{}:{}", redirect_host, redirect_port);
                log::info!(
                    "Starting HTTP→HTTPS redirect on {} → port {}",
                    redirect_addr,
                    https_port
                );
                let redirect_app = Router::new().fallback(
                    move |req: axum::extract::Request| async move {
                        let host = req
                            .headers()
                            .get("host")
                            .and_then(|h| h.to_str().ok())
                            .unwrap_or("localhost");
                        // Strip port from host header if present
                        let hostname = host.split(':').next().unwrap_or(host);
                        let path = req.uri().path_and_query()
                            .map(|pq| pq.as_str())
                            .unwrap_or("/");
                        let https_url = if https_port == 443 {
                            format!("https://{}{}", hostname, path)
                        } else {
                            format!("https://{}:{}{}", hostname, https_port, path)
                        };
                        axum::response::Redirect::temporary(&https_url).into_response()
                    },
                );
                match tokio::net::TcpListener::bind(&redirect_addr).await {
                    Ok(listener) => {
                        let _ = axum::serve(listener, redirect_app).await;
                    }
                    Err(e) => {
                        log::warn!("Could not start HTTP redirect server on {}: {}", redirect_addr, e);
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

    async fn dashboard_handler() -> Html<&'static str> {
        Html(include_str!("web/dashboard.html"))
    }

    async fn novnc_handler() -> impl IntoResponse {
        (
            [(header::CONTENT_TYPE, "application/javascript")],
            include_str!("web/vendor/novnc.min.js"),
        )
    }

    fn req_ctx(method: HttpMethod, path: &str) -> RequestContext {
        RequestContext::new(method, path)
    }

    /// Sanitize internal error details before sending to clients.
    ///
    // ── Policy Enforcement ──────────────────────────────────────

    struct PolicyViolation {
        policy_name: String,
        enforcement: String,
        message: String,
    }

    async fn check_policies(
        client: &KubeClient,
        config: &crate::config::VMConfig,
    ) -> anyhow::Result<Vec<PolicyViolation>> {
        use crate::operator_crds::VMRoguePolicy;

        let api: kube::Api<VMRoguePolicy> =
            kube::Api::namespaced(client.client(), &config.namespace);

        let policies = match api.list(&kube::api::ListParams::default()).await {
            Ok(list) => list.items,
            Err(e) => {
                let msg = e.to_string();
                // CRDs not installed or no permission — skip policy check
                if msg.contains("not found") || msg.contains("NotFound") || msg.contains("the server could not find") {
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
                            let total: f64 = config.disks.iter()
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
        let msg = e.to_string();

        // Map known error patterns to safe, generic messages
        if msg.starts_with("NotFound") || msg.contains("not found") {
            "Resource not found".to_string()
        } else if msg.starts_with("Conflict") {
            "Resource conflict".to_string()
        } else if msg.starts_with("Unauthorized") {
            "Unauthorized".to_string()
        } else if msg.starts_with("Forbidden") {
            "Forbidden".to_string()
        } else if msg.starts_with("Timeout") || msg.contains("timed out") {
            "Request timed out".to_string()
        } else if msg.starts_with("connection") || msg.contains("connection refused") {
            "Service unavailable".to_string()
        } else {
            "Internal server error".to_string()
        }
    }

    // ── VM Endpoints ──────────────────────────────────────────────

    #[derive(Deserialize)]
    pub struct VmQuery {
        pub namespace: Option<String>,
    }

    async fn list_vms_handler(
        State(state): State<SharedState>,
        Query(_query): Query<VmQuery>,
    ) -> impl IntoResponse {
        let (client, namespace) = {
            let s = state.read().await;
            (s.kube_client.clone(), s.namespace.clone())
        };

        match client.list_vms(&namespace).await {
            Ok(vms) => {
                let vm_infos: Vec<VmInfo> = vms
                    .iter()
                    .map(|vm| VmInfo::from_vm_with_ip(vm, None))
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms");
                ok_json(&ApiResponse::success(&vm_infos, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    async fn get_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }

        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        match client.get_vm(&ns, &name).await {
            Ok(vm) => {
                let ip = client.get_vm_ip(&ns, &name).await.unwrap_or(None);
                let info = VmInfo::from_vm_with_ip(&vm, ip);

                // Also fetch VMI details if running
                let vmi_detail = client.get_vmi(&ns, &name).await.ok();
                let detail = VmDetail {
                    info,
                    vmi_status: vmi_detail.and_then(|v| v.status),
                };

                let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name");
                ok_json(&ApiResponse::success(&detail, &ctx.request_id))
            }
            Err(e) => {
                let msg = sanitize_error(&e);
                if msg.contains("NotFound") || msg.contains("not found") {
                    err_json(404, "NOT_FOUND", &format!("VM '{}' not found", name))
                } else {
                    err_json(500, "INTERNAL_ERROR", &msg)
                }
            }
        }
    }

    async fn start_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }

        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        match client.start_vm(&ns, &name).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/start");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("VM '{}' started", name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => err_json(500, "START_FAILED", &sanitize_error(&e)),
        }
    }

    async fn stop_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }

        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        match client.stop_vm(&ns, &name).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/stop");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("VM '{}' stopped", name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => err_json(500, "STOP_FAILED", &sanitize_error(&e)),
        }
    }

    async fn restart_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }

        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        match client.restart_vm(&ns, &name).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/restart");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("VM '{}' restarted", name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => err_json(500, "RESTART_FAILED", &sanitize_error(&e)),
        }
    }

    // ── VNC Console WebSocket Proxy ─────────────────────────────

    async fn vnc_websocket_handler(
        ws: WebSocketUpgrade,
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        // Check VM is running
        match client.is_running(&ns, &name).await {
            Ok(true) => {}
            Ok(false) => {
                return (
                    StatusCode::BAD_REQUEST,
                    "VM is not running",
                )
                    .into_response();
            }
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to check VM status",
                )
                    .into_response();
            }
        }

        log::info!("VNC WebSocket upgrade requested for {}/{}", ns, name);
        ws.protocols(["binary"])
            .on_upgrade(move |socket| vnc_proxy(socket, client, ns, name))
            .into_response()
    }

    async fn vnc_proxy(mut client_ws: WebSocket, _kube_client: KubeClient, ns: String, name: String) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        // Find a free port for virtctl proxy
        let port = {
            let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
                Ok(l) => l,
                Err(e) => {
                    log::error!("Failed to bind ephemeral port: {}", e);
                    return;
                }
            };
            listener.local_addr().unwrap().port()
        };

        // Spawn virtctl vnc --proxy-only
        let mut child = match tokio::process::Command::new("virtctl")
            .args(["vnc", &name, "-n", &ns, "--proxy-only", "--port", &port.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to spawn virtctl: {}", e);
                let _ = client_ws
                    .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code: 1011,
                        reason: "virtctl not available".into(),
                    })))
                    .await;
                return;
            }
        };

        // Wait for virtctl to start listening, then connect
        // Important: virtctl accepts only one connection, so we must use the
        // successful connect attempt directly — don't probe then reconnect.
        let mut stream = None;
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(_) => continue,
            }
        }

        let stream = match stream {
            Some(s) => s,
            None => {
                log::error!("virtctl VNC proxy did not start on port {}", port);
                child.kill().await.ok();
                let _ = client_ws
                    .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code: 1011,
                        reason: "VNC proxy failed to start".into(),
                    })))
                    .await;
                return;
            }
        };

        let (mut tcp_read, mut tcp_write) = stream.into_split();

        log::info!("VNC proxy connected for {}/{} on port {}", ns, name, port);

        // Bidirectional proxy: WebSocket ↔ TCP
        let mut buf = vec![0u8; 65536];
        loop {
            tokio::select! {
                // Browser → VNC (WebSocket → TCP)
                msg = client_ws.recv() => {
                    match msg {
                        Some(Ok(Message::Binary(data))) => {
                            if tcp_write.write_all(&data).await.is_err() { break; }
                        }
                        Some(Ok(Message::Text(text))) => {
                            if tcp_write.write_all(text.as_bytes()).await.is_err() { break; }
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
                // VNC → Browser (TCP → WebSocket)
                result = tcp_read.read(&mut buf) => {
                    match result {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            if client_ws.send(Message::Binary(buf[..n].to_vec().into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }

        // Cleanup
        child.kill().await.ok();
        let _ = client_ws.send(Message::Close(None)).await;
        log::info!("VNC proxy closed for {}/{}", ns, name);
    }

    // ── VM Security Posture ──────────────────────────────────────

    async fn vm_security_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }

        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        match client.get_vm(&ns, &name).await {
            Ok(vm) => {
                let domain = &vm.spec.template.spec.domain;
                let mut checks = Vec::new();
                let mut score = 100u32;

                // Check Secure Boot
                let has_secure_boot = domain
                    .firmware
                    .as_ref()
                    .and_then(|f| f.bootloader.as_ref())
                    .and_then(|b| b.efi.as_ref())
                    .is_some_and(|e| e.secure_boot.unwrap_or(false));
                checks.push(serde_json::json!({
                    "name": "Secure Boot",
                    "pass": has_secure_boot,
                    "detail": if has_secure_boot { "UEFI Secure Boot enabled" } else { "Secure Boot not configured" },
                }));
                if !has_secure_boot {
                    score = score.saturating_sub(15);
                }

                // Check TPM
                let has_tpm = domain
                    .devices
                    .as_ref()
                    .and_then(|d| d.tpm.as_ref())
                    .is_some();
                checks.push(serde_json::json!({
                    "name": "TPM 2.0",
                    "pass": has_tpm,
                    "detail": if has_tpm { "TPM device attached" } else { "No TPM device" },
                }));
                if !has_tpm {
                    score = score.saturating_sub(10);
                }

                // Check RNG
                let has_rng = domain
                    .devices
                    .as_ref()
                    .and_then(|d| d.rng.as_ref())
                    .is_some();
                checks.push(serde_json::json!({
                    "name": "Hardware RNG",
                    "pass": has_rng,
                    "detail": if has_rng { "virtio-rng configured" } else { "No RNG device" },
                }));
                if !has_rng {
                    score = score.saturating_sub(5);
                }

                // Check eviction strategy
                let has_eviction = vm
                    .spec
                    .template
                    .spec
                    .eviction_strategy
                    .as_ref()
                    .is_some_and(|e| !e.is_empty());
                checks.push(serde_json::json!({
                    "name": "Eviction Strategy",
                    "pass": has_eviction,
                    "detail": if has_eviction { "Live migration on eviction" } else { "No eviction strategy" },
                }));
                if !has_eviction {
                    score = score.saturating_sub(10);
                }

                // Check resource limits
                let has_limits = domain
                    .resources
                    .limits
                    .as_ref()
                    .is_some_and(|l| !l.is_empty());
                checks.push(serde_json::json!({
                    "name": "Resource Limits",
                    "pass": has_limits,
                    "detail": if has_limits { "CPU/memory limits set" } else { "No resource limits defined" },
                }));
                if !has_limits {
                    score = score.saturating_sub(10);
                }

                let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/security");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({
                        "score": score,
                        "checks": checks,
                    }),
                    &ctx.request_id,
                ))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    // ── Clone, Pause, Unpause, Migrate, Update, VM Events ──────

    #[derive(Deserialize)]
    struct CloneRequest {
        new_name: String,
    }

    async fn clone_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
        Json(req): Json<CloneRequest>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name), ("new_name", &req.new_name)]) {
            return resp;
        }
        let client = { state.read().await.kube_client.clone() };
        match client.clone_vm(&ns, &name, &req.new_name).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/clone");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("VM '{}' cloned to '{}'", name, req.new_name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => {
                log::error!("Clone VM failed: {}", e);
                err_json(500, "CLONE_FAILED", &sanitize_error(&e))
            }
        }
    }

    async fn pause_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }
        let client = { state.read().await.kube_client.clone() };
        match client.pause_vm(&ns, &name).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/pause");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("VM '{}' paused", name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => err_json(500, "PAUSE_FAILED", &sanitize_error(&e)),
        }
    }

    async fn unpause_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }
        let client = { state.read().await.kube_client.clone() };
        match client.unpause_vm(&ns, &name).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/unpause");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("VM '{}' unpaused", name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => err_json(500, "UNPAUSE_FAILED", &sanitize_error(&e)),
        }
    }

    async fn migrate_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }
        let client = { state.read().await.kube_client.clone() };
        match client.migrate_vm(&ns, &name).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/migrate");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("Migration initiated for VM '{}'", name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => err_json(500, "MIGRATE_FAILED", &sanitize_error(&e)),
        }
    }

    #[derive(Deserialize)]
    struct UpdateVmRequest {
        cpus: Option<u32>,
        memory: Option<String>,
    }

    async fn update_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
        Json(req): Json<UpdateVmRequest>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }
        let client = { state.read().await.kube_client.clone() };
        match client.update_vm_resources(&ns, &name, req.cpus, req.memory.as_deref()).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("VM '{}' updated", name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => err_json(500, "UPDATE_FAILED", &sanitize_error(&e)),
        }
    }

    async fn vm_events_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }
        let client = { state.read().await.kube_client.clone() };

        use k8s_openapi::api::core::v1::Event;
        use kube::Api;
        let events_api: Api<Event> = Api::namespaced(client.client(), &ns);
        let lp = kube::api::ListParams::default().limit(100);
        match events_api.list(&lp).await {
            Ok(event_list) => {
                let items: Vec<EventItem> = event_list
                    .items
                    .into_iter()
                    .filter(|e| e.involved_object.name.as_deref() == Some(&name))
                    .map(|e| EventItem {
                        type_: e.type_.unwrap_or_default(),
                        reason: e.reason.unwrap_or_default(),
                        message: e.message.unwrap_or_default(),
                        namespace: e.metadata.namespace.unwrap_or_default(),
                        involved_object: e.involved_object.name.unwrap_or_default(),
                        timestamp: e.last_timestamp
                            .map(|t| t.0.to_rfc3339())
                            .or_else(|| e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339()))
                            .unwrap_or_default(),
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/events");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    // ── Storage Endpoints ────────────────────────────────────────

    async fn list_pvcs_handler(
        State(state): State<SharedState>,
        Query(query): Query<VmQuery>,
    ) -> impl IntoResponse {
        let (client, namespace) = {
            let s = state.read().await;
            (s.kube_client.clone(), s.namespace.clone())
        };
        let ns = query.namespace.as_deref().unwrap_or(&namespace);

        match client.list_pvcs(ns).await {
            Ok(pvcs) => {
                let items: Vec<serde_json::Value> = pvcs
                    .iter()
                    .map(|pvc| {
                        let meta = &pvc.metadata;
                        let spec = pvc.spec.as_ref();
                        let status = pvc.status.as_ref();
                        serde_json::json!({
                            "name": meta.name.clone().unwrap_or_default(),
                            "namespace": meta.namespace.clone().unwrap_or_default(),
                            "status": status.and_then(|s| s.phase.as_ref()).map(|s| s.to_string()).unwrap_or_default(),
                            "capacity": status
                                .and_then(|s| s.capacity.as_ref())
                                .and_then(|c| c.get("storage"))
                                .map(|v| v.0.clone())
                                .unwrap_or_default(),
                            "storage_class": spec.and_then(|s| s.storage_class_name.clone()).unwrap_or_default(),
                            "access_modes": spec.map(|s| s.access_modes.clone().unwrap_or_default()).unwrap_or_default(),
                        })
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/storage/pvcs");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    async fn list_storage_classes_handler(State(state): State<SharedState>) -> impl IntoResponse {
        let client = { state.read().await.kube_client.clone() };
        match client.list_storage_classes().await {
            Ok(scs) => {
                let items: Vec<serde_json::Value> = scs
                    .iter()
                    .map(|sc| {
                        serde_json::json!({
                            "name": sc.metadata.name.clone().unwrap_or_default(),
                            "provisioner": sc.provisioner,
                            "reclaim_policy": sc.reclaim_policy.clone().unwrap_or_default(),
                            "volume_binding_mode": sc.volume_binding_mode.clone().unwrap_or_default(),
                            "is_default": sc.metadata.annotations.as_ref()
                                .and_then(|a| a.get("storageclass.kubernetes.io/is-default-class"))
                                .map(|v| v == "true")
                                .unwrap_or(false),
                        })
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/storage/classes");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    // ── OpenAPI ──────────────────────────────────────────────────

    async fn openapi_handler() -> impl IntoResponse {
        let spec = crate::api::openapi::generate_default_spec();
        let json = serde_json::to_value(&spec).unwrap_or_default();
        (StatusCode::OK, Json(json))
    }

    async fn delete_vm_handler(
        State(state): State<SharedState>,
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }

        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        match client.delete_vm(&ns, &name).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::DELETE, "/api/v1/vms/:ns/:name");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("VM '{}' deleted", name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => err_json(500, "DELETE_FAILED", &sanitize_error(&e)),
        }
    }

    // ── Batch VM Operations ─────────────────────────────────────

    #[derive(Deserialize)]
    struct BatchVmRequest {
        action: String,
        vms: Vec<BatchVmTarget>,
    }

    #[derive(Deserialize)]
    struct BatchVmTarget {
        namespace: String,
        name: String,
    }

    async fn batch_vm_handler(
        State(state): State<SharedState>,
        Json(req): Json<BatchVmRequest>,
    ) -> impl IntoResponse {
        // Limit batch size to prevent K8s API abuse
        const MAX_BATCH_SIZE: usize = 50;
        if req.vms.len() > MAX_BATCH_SIZE {
            return err_json(
                400,
                "BATCH_TOO_LARGE",
                &format!("Batch size {} exceeds maximum of {}", req.vms.len(), MAX_BATCH_SIZE),
            );
        }

        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        let mut results = Vec::new();

        for vm in &req.vms {
            if !is_valid_k8s_name(&vm.name) || !is_valid_k8s_name(&vm.namespace) {
                results.push(serde_json::json!({
                    "name": vm.name,
                    "namespace": vm.namespace,
                    "status": "error",
                    "message": "Invalid Kubernetes name"
                }));
                continue;
            }

            let result: Result<(), anyhow::Error> = match req.action.as_str() {
                "start" => client.start_vm(&vm.namespace, &vm.name).await.map(|_| ()),
                "stop" => client.stop_vm(&vm.namespace, &vm.name).await.map(|_| ()),
                "restart" => client.restart_vm(&vm.namespace, &vm.name).await.map(|_| ()),
                "delete" => client.delete_vm(&vm.namespace, &vm.name).await,
                _ => {
                    results.push(serde_json::json!({
                        "name": vm.name,
                        "namespace": vm.namespace,
                        "status": "error",
                        "message": format!("Unknown action: {}", req.action)
                    }));
                    continue;
                }
            };

            match result {
                Ok(()) => results.push(serde_json::json!({
                    "name": vm.name,
                    "namespace": vm.namespace,
                    "status": "success",
                    "message": format!("VM '{}' {}ed", vm.name, req.action)
                })),
                Err(e) => results.push(serde_json::json!({
                    "name": vm.name,
                    "namespace": vm.namespace,
                    "status": "error",
                    "message": sanitize_error(&e)
                })),
            }
        }

        let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/batch");
        ok_json(&ApiResponse::success(
            &serde_json::json!({"results": results}),
            &ctx.request_id,
        ))
    }

    // ── Create VM Endpoint ────────────────────────────────────────

    #[derive(Deserialize)]
    struct CreateVmRequest {
        name: String,
        namespace: Option<String>,
        template: Option<String>,
        profile: Option<String>,
        cpus: Option<u32>,
        memory: Option<String>,
        disk_size: Option<String>,
        cloud_init: Option<String>,
        start: Option<bool>,
    }

    async fn create_vm_handler(
        State(state): State<SharedState>,
        Json(req): Json<CreateVmRequest>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("name", &req.name)]) {
            return resp;
        }

        let ns = req.namespace.as_deref().unwrap_or("default");
        if let Some(resp) = validate_k8s_params(&[("namespace", ns)]) {
            return resp;
        }

        // Start with template or blank config
        let mut builder = if let Some(ref tpl_name) = req.template {
            if let Some(tpl_config) = TEMPLATES.get(tpl_name) {
                let mut b = VMConfigBuilder::new(&req.name);
                b = b.namespace(ns);
                b = b.cpu(tpl_config.cpu.cores, tpl_config.cpu.sockets, tpl_config.cpu.threads);
                b = b.memory(&tpl_config.memory.size);
                // Preserve disk types from template (containerDisk, blank, etc.)
                for disk in &tpl_config.disks {
                    b = b.add_disk(disk.clone());
                }
                // Preserve network interfaces from template
                for iface in &tpl_config.interfaces {
                    b = b.add_interface(iface.clone());
                }
                // Preserve cloud-init, features, clock from template
                if let Some(ref ci) = tpl_config.cloud_init {
                    b = b.cloud_init(&ci.user_data);
                }
                if tpl_config.enable_rng {
                    b = b.enable_rng();
                }
                if let Some(ref clock) = tpl_config.clock {
                    b = b.clock(clock.clone());
                }
                if let Some(ref features) = tpl_config.features {
                    b = b.features(features.clone());
                }
                if let Some(ref firmware) = tpl_config.firmware {
                    b = b.firmware(firmware.clone());
                }
                // Preserve labels from template
                b = b.labels(tpl_config.labels.clone());
                b
            } else {
                return err_json(400, "INVALID_TEMPLATE", &format!("Template '{}' not found", tpl_name));
            }
        } else {
            let mut b = VMConfigBuilder::new(&req.name);
            b = b.namespace(ns);
            b = b.add_blank_disk("rootdisk", "10Gi", 1);
            b = b.add_pod_network("default");
            b
        };

        // Apply profile defaults (before manual overrides)
        if let Some(ref profile_name) = req.profile {
            if let Ok(profiles) = PROFILES.read() {
                if let Some(profile) = profiles.get(profile_name) {
                    builder = builder.cpu(profile.cpu_cores, profile.cpu_sockets, profile.cpu_threads);
                    builder = builder.memory(&profile.memory);
                    // Profile disk size applied below via config.disks
                }
            }
        }

        // Apply manual overrides (take priority over profile)
        if let Some(cpus) = req.cpus {
            builder = builder.cpu(cpus, 1, 1);
        }
        if let Some(ref mem) = req.memory {
            builder = builder.memory(mem);
        }

        // Apply cloud-init
        if let Some(ref ci) = req.cloud_init {
            if !ci.is_empty() {
                builder = builder.cloud_init(ci);
            }
        }

        let mut config = builder.build();

        // Override disk size if specified
        if let Some(ref ds) = req.disk_size {
            if let Some(disk) = config.disks.first_mut() {
                disk.size = ds.clone();
            }
        } else if let Some(ref profile_name) = req.profile {
            // Apply profile disk size if no explicit override
            if let Ok(profiles) = PROFILES.read() {
                if let Some(profile) = profiles.get(profile_name) {
                    if let Some(disk) = config.disks.first_mut() {
                        disk.size = profile.disk_size.clone();
                    }
                }
            }
        }

        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        // Evaluate VMRoguePolicy CRDs before creation
        if let Ok(violations) = check_policies(&client, &config).await {
            if !violations.is_empty() {
                let deny_violations: Vec<_> = violations
                    .iter()
                    .filter(|v| v.enforcement == "Deny")
                    .collect();
                if !deny_violations.is_empty() {
                    let messages: Vec<String> = deny_violations
                        .iter()
                        .map(|v| format!("[{}] {}", v.policy_name, v.message))
                        .collect();
                    return err_json(
                        403,
                        "POLICY_VIOLATION",
                        &messages.join("; "),
                    );
                }
                // Warn-level violations are logged but don't block creation
                for v in violations.iter().filter(|v| v.enforcement == "Warn") {
                    log::warn!("Policy warning for VM '{}': [{}] {}", req.name, v.policy_name, v.message);
                }
            }
        }

        match client.create_vm(&config).await {
            Ok(_) => {
                // Auto-start if requested
                let started = if req.start.unwrap_or(false) {
                    client.start_vm(ns, &req.name).await.is_ok()
                } else {
                    false
                };
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({
                        "message": format!("VM '{}' created{}", req.name, if started { " and started" } else { "" }),
                        "name": req.name,
                        "namespace": ns,
                        "started": started,
                    }),
                    &ctx.request_id,
                ))
            }
            Err(e) => {
                let msg = sanitize_error(&e);
                if msg.contains("already exists") || msg.contains("conflict") {
                    err_json(409, "VM_EXISTS", &format!("VM '{}' already exists", req.name))
                } else {
                    err_json(500, "CREATE_FAILED", &msg)
                }
            }
        }
    }

    // ── Templates Endpoint ───────────────────────────────────────

    async fn list_templates_handler() -> impl IntoResponse {
        let templates: Vec<serde_json::Value> = TEMPLATES
            .list()
            .iter()
            .filter_map(|name| {
                TEMPLATES.get(name).map(|t| {
                    serde_json::json!({
                        "name": name,
                        "cpu": t.cpu.cores,
                        "memory": t.memory.size,
                        "disk": t.disks.first().map(|d| d.size.as_str()).unwrap_or("10Gi"),
                    })
                })
            })
            .collect();
        let ctx = req_ctx(HttpMethod::GET, "/api/v1/templates");
        ok_json(&ApiResponse::success(&templates, &ctx.request_id))
    }

    // ── Snapshot Endpoints ────────────────────────────────────────

    async fn list_snapshots_handler(
        State(state): State<SharedState>,
        Query(query): Query<VmQuery>,
    ) -> impl IntoResponse {
        let namespace = {
            let s = state.read().await;
            query
                .namespace
                .clone()
                .unwrap_or_else(|| s.namespace.clone())
        };

        match crate::snapshots::SnapshotManager::new(&namespace).await {
            Ok(manager) => match manager.list_all_snapshots().await {
                Ok(snapshots) => {
                    let items: Vec<SnapshotItem> = snapshots
                        .into_iter()
                        .map(|s| {
                            let age = s.age();
                            let status = s.status.to_string();
                            SnapshotItem {
                                name: s.name,
                                vm_name: s.vm_name,
                                status,
                                ready: s.ready_to_use,
                                age,
                            }
                        })
                        .collect();
                    let ctx = req_ctx(HttpMethod::GET, "/api/v1/snapshots");
                    ok_json(&ApiResponse::success(&items, &ctx.request_id))
                }
                Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
            },
            Err(e) => err_json(503, "SERVICE_UNAVAILABLE", &sanitize_error(&e)),
        }
    }

    async fn list_vm_snapshots_handler(
        Path((ns, vm)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("vm", &vm)]) {
            return resp;
        }

        match crate::snapshots::SnapshotManager::new(&ns).await {
            Ok(manager) => match manager.list_snapshots_for_vm(&vm).await {
                Ok(snapshots) => {
                    let items: Vec<SnapshotItem> = snapshots
                        .into_iter()
                        .map(|s| {
                            let age = s.age();
                            let status = s.status.to_string();
                            SnapshotItem {
                                name: s.name,
                                vm_name: s.vm_name,
                                status,
                                ready: s.ready_to_use,
                                age,
                            }
                        })
                        .collect();
                    let ctx = req_ctx(HttpMethod::GET, "/api/v1/snapshots/:ns/:vm");
                    ok_json(&ApiResponse::success(&items, &ctx.request_id))
                }
                Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
            },
            Err(e) => err_json(503, "SERVICE_UNAVAILABLE", &sanitize_error(&e)),
        }
    }

    async fn delete_snapshot_handler(
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }

        match crate::snapshots::SnapshotManager::new(&ns).await {
            Ok(manager) => match manager.delete_snapshot(&name).await {
                Ok(_) => {
                    let ctx = req_ctx(HttpMethod::POST, "/api/v1/snapshots/:ns/:name/delete");
                    ok_json(&ApiResponse::success(
                        &serde_json::json!({"message": format!("Snapshot '{}' deleted", name)}),
                        &ctx.request_id,
                    ))
                }
                Err(e) => err_json(500, "DELETE_FAILED", &sanitize_error(&e)),
            },
            Err(e) => err_json(503, "SERVICE_UNAVAILABLE", &sanitize_error(&e)),
        }
    }

    // ── Events ────────────────────────────────────────────────────

    #[derive(Deserialize)]
    pub struct EventsQuery {
        pub limit: Option<u32>,
    }

    async fn list_events_handler(
        State(state): State<SharedState>,
        Query(query): Query<EventsQuery>,
    ) -> impl IntoResponse {
        let (client, namespace) = {
            let s = state.read().await;
            (s.kube_client.clone(), s.namespace.clone())
        };

        let limit = query.limit.unwrap_or(50).min(1000);

        use k8s_openapi::api::core::v1::Event;
        use kube::Api;
        let events_api: Api<Event> = Api::namespaced(client.client(), &namespace);
        let lp = kube::api::ListParams::default().limit(limit);
        match events_api.list(&lp).await {
            Ok(event_list) => {
                let items: Vec<EventItem> = event_list
                    .items
                    .into_iter()
                    .map(|e| EventItem {
                        type_: e.type_.unwrap_or_default(),
                        reason: e.reason.unwrap_or_default(),
                        message: e.message.unwrap_or_default(),
                        namespace: e.metadata.namespace.unwrap_or_default(),
                        involved_object: e.involved_object.name.unwrap_or_default(),
                        timestamp: e
                            .last_timestamp
                            .map(|t| t.0.to_rfc3339())
                            .or_else(|| e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339()))
                            .unwrap_or_default(),
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/events");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    async fn recent_events_handler(State(state): State<SharedState>) -> impl IntoResponse {
        let (client, namespace) = {
            let s = state.read().await;
            (s.kube_client.clone(), s.namespace.clone())
        };

        use k8s_openapi::api::core::v1::Event;
        use kube::Api;
        let events_api: Api<Event> = Api::namespaced(client.client(), &namespace);
        let lp = kube::api::ListParams::default().limit(500);
        match events_api.list(&lp).await {
            Ok(event_list) => {
                let one_hour_ago = chrono::Utc::now() - chrono::TimeDelta::hours(1);
                let items: Vec<EventItem> = event_list
                    .items
                    .into_iter()
                    .filter(|e| {
                        // Keep events from the last hour based on timestamp
                        let ts = e
                            .last_timestamp
                            .as_ref()
                            .map(|t| t.0)
                            .or_else(|| e.metadata.creation_timestamp.as_ref().map(|t| t.0));
                        match ts {
                            Some(t) => t >= one_hour_ago,
                            None => false, // exclude events with no timestamp
                        }
                    })
                    .map(|e| EventItem {
                        type_: e.type_.unwrap_or_default(),
                        reason: e.reason.unwrap_or_default(),
                        message: e.message.unwrap_or_default(),
                        namespace: e.metadata.namespace.unwrap_or_default(),
                        involved_object: e.involved_object.name.unwrap_or_default(),
                        timestamp: e
                            .last_timestamp
                            .map(|t| t.0.to_rfc3339())
                            .or_else(|| e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339()))
                            .unwrap_or_default(),
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/events/recent");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    // ── Snapshot Create & Restore ─────────────────────────────────

    #[derive(Deserialize)]
    struct CreateSnapshotRequest {
        snapshot_name: Option<String>,
    }

    async fn create_snapshot_handler(
        Path((ns, vm)): Path<(String, String)>,
        Json(req): Json<CreateSnapshotRequest>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &vm)]) {
            return resp;
        }

        let snap_name = req.snapshot_name.unwrap_or_else(|| {
            let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
            format!("{}-snap-{}", vm, ts)
        });

        match crate::snapshots::SnapshotManager::new(&ns).await {
            Ok(manager) => {
                let config = crate::snapshots::SnapshotConfig::new(&vm, &snap_name);
                match manager.create_snapshot(&config).await {
                    Ok(info) => {
                        let ctx = req_ctx(HttpMethod::POST, "/api/v1/snapshots/:ns/:vm/create");
                        ok_json(&ApiResponse::success(
                            &serde_json::json!({
                                "message": format!("Snapshot '{}' created", snap_name),
                                "name": info.name,
                                "vm_name": info.vm_name,
                            }),
                            &ctx.request_id,
                        ))
                    }
                    Err(e) => err_json(500, "CREATE_FAILED", &sanitize_error(&e)),
                }
            }
            Err(e) => err_json(503, "SERVICE_UNAVAILABLE", &sanitize_error(&e)),
        }
    }

    async fn restore_snapshot_handler(
        Path((ns, name)): Path<(String, String)>,
    ) -> impl IntoResponse {
        if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
            return resp;
        }

        match crate::snapshots::restore::RestoreManager::new(&ns).await {
            Ok(manager) => {
                // Derive VM name from snapshot name (convention: vmname-snap-timestamp)
                let vm_name = name
                    .rsplit_once("-snap-")
                    .map(|(v, _)| v.to_string())
                    .unwrap_or_else(|| name.clone());
                match manager.restore_in_place(&vm_name, &name).await {
                    Ok(info) => {
                        let ctx =
                            req_ctx(HttpMethod::POST, "/api/v1/snapshots/:ns/:name/restore");
                        ok_json(&ApiResponse::success(
                            &serde_json::json!({
                                "message": format!("Restore '{}' initiated", info.name),
                                "name": info.name,
                            }),
                            &ctx.request_id,
                        ))
                    }
                    Err(e) => err_json(500, "RESTORE_FAILED", &sanitize_error(&e)),
                }
            }
            Err(e) => err_json(503, "SERVICE_UNAVAILABLE", &sanitize_error(&e)),
        }
    }

    // ── Cluster Resources ────────────────────────────────────────

    async fn list_nodes_handler(State(state): State<SharedState>) -> impl IntoResponse {
        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        match client.list_nodes().await {
            Ok(nodes) => {
                let items: Vec<NodeItem> = nodes
                    .iter()
                    .map(|node| {
                        let meta = &node.metadata;
                        let name = meta.name.clone().unwrap_or_default();
                        let labels = meta.labels.as_ref();

                        let roles: Vec<String> = labels
                            .map(|l| {
                                l.keys()
                                    .filter_map(|k| k.strip_prefix("node-role.kubernetes.io/"))
                                    .map(|r| r.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();

                        let status = node
                            .status
                            .as_ref()
                            .and_then(|s| s.conditions.as_ref())
                            .and_then(|conds| {
                                conds
                                    .iter()
                                    .find(|c| c.type_ == "Ready")
                                    .map(|c| {
                                        if c.status == "True" {
                                            "Ready"
                                        } else {
                                            "NotReady"
                                        }
                                    })
                            })
                            .unwrap_or("Unknown")
                            .to_string();

                        let capacity = node
                            .status
                            .as_ref()
                            .and_then(|s| s.capacity.as_ref());
                        let allocatable = node
                            .status
                            .as_ref()
                            .and_then(|s| s.allocatable.as_ref());
                        let node_info = node
                            .status
                            .as_ref()
                            .and_then(|s| s.node_info.as_ref());

                        NodeItem {
                            name,
                            status,
                            roles,
                            cpu_capacity: capacity
                                .and_then(|c| c.get("cpu"))
                                .map(|v| v.0.clone())
                                .unwrap_or_default(),
                            memory_capacity: capacity
                                .and_then(|c| c.get("memory"))
                                .map(|v| v.0.clone())
                                .unwrap_or_default(),
                            cpu_allocatable: allocatable
                                .and_then(|a| a.get("cpu"))
                                .map(|v| v.0.clone())
                                .unwrap_or_default(),
                            memory_allocatable: allocatable
                                .and_then(|a| a.get("memory"))
                                .map(|v| v.0.clone())
                                .unwrap_or_default(),
                            kubelet_version: node_info
                                .map(|i| i.kubelet_version.clone())
                                .unwrap_or_default(),
                            os_image: node_info
                                .map(|i| i.os_image.clone())
                                .unwrap_or_default(),
                            kernel_version: node_info
                                .map(|i| i.kernel_version.clone())
                                .unwrap_or_default(),
                            age: meta
                                .creation_timestamp
                                .as_ref()
                                .map(|t| format_age(&t.0))
                                .unwrap_or_default(),
                        }
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/nodes");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    async fn list_pods_handler(
        State(state): State<SharedState>,
        Query(query): Query<VmQuery>,
    ) -> impl IntoResponse {
        let (client, namespace) = {
            let s = state.read().await;
            (s.kube_client.clone(), s.namespace.clone())
        };
        let ns = query.namespace.as_deref().unwrap_or(&namespace);

        match client.list_pods(ns).await {
            Ok(pods) => {
                let items: Vec<PodItem> = pods
                    .iter()
                    .map(|pod| {
                        let meta = &pod.metadata;
                        let spec = pod.spec.as_ref();
                        let status = pod.status.as_ref();

                        let containers: Vec<String> = spec
                            .map(|s| s.containers.iter().map(|c| c.name.clone()).collect())
                            .unwrap_or_default();

                        let restarts: u32 = status
                            .and_then(|s| s.container_statuses.as_ref())
                            .map(|cs| cs.iter().map(|c| c.restart_count as u32).sum())
                            .unwrap_or(0);

                        PodItem {
                            name: meta.name.clone().unwrap_or_default(),
                            namespace: meta.namespace.clone().unwrap_or_default(),
                            phase: status
                                .and_then(|s| s.phase.clone())
                                .unwrap_or_else(|| "Unknown".to_string()),
                            node_name: spec
                                .and_then(|s| s.node_name.clone())
                                .unwrap_or_else(|| "N/A".to_string()),
                            ip: status
                                .and_then(|s| s.pod_ip.clone())
                                .unwrap_or_else(|| "N/A".to_string()),
                            containers,
                            restarts,
                            age: meta
                                .creation_timestamp
                                .as_ref()
                                .map(|t| format_age(&t.0))
                                .unwrap_or_default(),
                        }
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/pods");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    async fn list_profiles_handler() -> impl IntoResponse {
        let profiles = PROFILES.read().unwrap();
        let items: Vec<ProfileItem> = profiles
            .list()
            .into_iter()
            .map(|p| ProfileItem {
                name: p.name.clone(),
                description: p.description.clone(),
                cpu_cores: p.cpu_cores,
                memory: p.memory.clone(),
                disk_size: p.disk_size.clone(),
                use_cases: p.use_cases.clone(),
            })
            .collect();
        let ctx = req_ctx(HttpMethod::GET, "/api/v1/profiles");
        ok_json(&ApiResponse::success(&items, &ctx.request_id))
    }

    async fn list_namespaces_handler(State(state): State<SharedState>) -> impl IntoResponse {
        let client = {
            let s = state.read().await;
            s.kube_client.clone()
        };

        match client.list_namespaces().await {
            Ok(ns_list) => {
                let names: Vec<String> = ns_list
                    .iter()
                    .filter_map(|ns| ns.metadata.name.clone())
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/namespaces");
                ok_json(&ApiResponse::success(&names, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    async fn activity_feed_handler(State(state): State<SharedState>) -> impl IntoResponse {
        let (client, namespace) = {
            let s = state.read().await;
            (s.kube_client.clone(), s.namespace.clone())
        };

        use k8s_openapi::api::core::v1::Event;
        use kube::Api;
        let events_api: Api<Event> = Api::namespaced(client.client(), &namespace);
        let lp = kube::api::ListParams::default().limit(200);
        match events_api.list(&lp).await {
            Ok(event_list) => {
                let mut items: Vec<EventItem> = event_list
                    .items
                    .into_iter()
                    .filter(|e| {
                        let kind = e
                            .involved_object
                            .kind
                            .as_deref()
                            .unwrap_or("");
                        kind == "VirtualMachine"
                            || kind == "VirtualMachineInstance"
                            || kind == "VirtualMachineSnapshot"
                    })
                    .map(|e| EventItem {
                        type_: e.type_.unwrap_or_default(),
                        reason: e.reason.unwrap_or_default(),
                        message: e.message.unwrap_or_default(),
                        namespace: e.metadata.namespace.unwrap_or_default(),
                        involved_object: e.involved_object.name.unwrap_or_default(),
                        timestamp: e
                            .last_timestamp
                            .map(|t| t.0.to_rfc3339())
                            .or_else(|| {
                                e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339())
                            })
                            .unwrap_or_default(),
                    })
                    .collect();
                items.truncate(20);
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/activity");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    // ── Dashboard Overview ────────────────────────────────────────

    async fn dashboard_overview_handler(State(state): State<SharedState>) -> impl IntoResponse {
        let (client, namespace) = {
            let s = state.read().await;
            (s.kube_client.clone(), s.namespace.clone())
        };

        let vms = match client.list_vms(&namespace).await {
            Ok(vms) => vms,
            Err(e) => {
                return err_json(500, "INTERNAL_ERROR", &sanitize_error(&e));
            }
        };
        let total = vms.len();
        let mut running = 0usize;
        let mut stopped = 0usize;
        for vm in &vms {
            match vm
                .status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
            {
                Some("Running") => running += 1,
                Some("Stopped") => stopped += 1,
                _ => {}
            }
        }
        let error = total.saturating_sub(running + stopped);

        // Count total allocated CPU/memory
        let mut total_cpus = 0u32;
        let mut total_memory_bytes = 0u64;
        for vm in &vms {
            total_cpus += vm
                .spec
                .template
                .spec
                .domain
                .cpu
                .as_ref()
                .and_then(|c| c.cores)
                .unwrap_or(1);
            if let Some(mem_str) = vm
                .spec
                .template
                .spec
                .domain
                .resources
                .requests
                .as_ref()
                .and_then(|r| r.get("memory"))
            {
                total_memory_bytes += parse_memory(mem_str);
            }
        }

        let snapshot_count = match crate::snapshots::SnapshotManager::new(&namespace).await {
            Ok(m) => m.list_all_snapshots().await.map(|s| s.len()).unwrap_or(0),
            Err(_) => 0,
        };

        // Aggregate node capacity
        let node_summary = match client.list_nodes().await {
            Ok(nodes) => {
                let total_nodes = nodes.len();
                let ready_nodes = nodes
                    .iter()
                    .filter(|n| {
                        n.status
                            .as_ref()
                            .and_then(|s| s.conditions.as_ref())
                            .and_then(|conds| conds.iter().find(|c| c.type_ == "Ready"))
                            .is_some_and(|c| c.status == "True")
                    })
                    .count();
                let mut cap_cpu = 0u64;
                let mut cap_mem = 0u64;
                let mut alloc_cpu = 0u64;
                let mut alloc_mem = 0u64;
                for node in &nodes {
                    if let Some(cap) = node.status.as_ref().and_then(|s| s.capacity.as_ref()) {
                        cap_cpu += cap
                            .get("cpu")
                            .map(|v| v.0.parse::<u64>().unwrap_or(0))
                            .unwrap_or(0);
                        cap_mem += cap
                            .get("memory")
                            .map(|v| parse_memory(&v.0))
                            .unwrap_or(0);
                    }
                    if let Some(alloc) =
                        node.status.as_ref().and_then(|s| s.allocatable.as_ref())
                    {
                        alloc_cpu += alloc
                            .get("cpu")
                            .map(|v| v.0.parse::<u64>().unwrap_or(0))
                            .unwrap_or(0);
                        alloc_mem += alloc
                            .get("memory")
                            .map(|v| parse_memory(&v.0))
                            .unwrap_or(0);
                    }
                }
                NodeSummary {
                    total_nodes,
                    ready_nodes,
                    total_cpu_capacity: cap_cpu,
                    total_memory_capacity_gb: (cap_mem as f64) / (1024.0 * 1024.0 * 1024.0),
                    total_cpu_allocatable: alloc_cpu,
                    total_memory_allocatable_gb: (alloc_mem as f64)
                        / (1024.0 * 1024.0 * 1024.0),
                }
            }
            Err(_) => NodeSummary {
                total_nodes: 0,
                ready_nodes: 0,
                total_cpu_capacity: 0,
                total_memory_capacity_gb: 0.0,
                total_cpu_allocatable: 0,
                total_memory_allocatable_gb: 0.0,
            },
        };

        let overview = DashboardOverview {
            cluster: ClusterStats {
                total_vms: total,
                running_vms: running,
                stopped_vms: stopped,
                error_vms: error,
                total_vcpus_allocated: total_cpus,
                total_memory_allocated_gb: (total_memory_bytes as f64) / (1024.0 * 1024.0 * 1024.0),
                total_snapshots: snapshot_count,
            },
            nodes: node_summary,
        };

        let ctx = req_ctx(HttpMethod::GET, "/api/v1/dashboard/overview");
        ok_json(&ApiResponse::success(&overview, &ctx.request_id))
    }

    // ── Health ─────────────────────────────────────────────────────

    async fn health_handler() -> impl IntoResponse {
        let ctx = req_ctx(HttpMethod::GET, "/api/v1/health");
        let health = serde_json::json!({
            "status": "healthy",
            "version": "v1",
            "service": "vmrogue-api"
        });
        ok_json(&ApiResponse::success(&health, &ctx.request_id))
    }

    // ── Types ──────────────────────────────────────────────────────

    #[derive(Serialize)]
    struct VmDetail {
        #[serde(flatten)]
        info: VmInfo,
        vmi_status: Option<crate::kube::types::VirtualMachineInstanceStatus>,
    }

    #[derive(Serialize)]
    struct SnapshotItem {
        name: String,
        vm_name: String,
        status: String,
        ready: bool,
        age: String,
    }

    #[derive(Serialize)]
    struct EventItem {
        #[serde(rename = "type")]
        type_: String,
        reason: String,
        message: String,
        namespace: String,
        involved_object: String,
        timestamp: String,
    }

    #[derive(Serialize)]
    struct DashboardOverview {
        cluster: ClusterStats,
        nodes: NodeSummary,
    }

    #[derive(Serialize)]
    struct ClusterStats {
        total_vms: usize,
        running_vms: usize,
        stopped_vms: usize,
        error_vms: usize,
        total_vcpus_allocated: u32,
        total_memory_allocated_gb: f64,
        total_snapshots: usize,
    }

    #[derive(Serialize)]
    struct NodeSummary {
        total_nodes: usize,
        ready_nodes: usize,
        total_cpu_capacity: u64,
        total_memory_capacity_gb: f64,
        total_cpu_allocatable: u64,
        total_memory_allocatable_gb: f64,
    }

    #[derive(Serialize)]
    struct NodeItem {
        name: String,
        status: String,
        roles: Vec<String>,
        cpu_capacity: String,
        memory_capacity: String,
        cpu_allocatable: String,
        memory_allocatable: String,
        kubelet_version: String,
        os_image: String,
        kernel_version: String,
        age: String,
    }

    #[derive(Serialize)]
    struct PodItem {
        name: String,
        namespace: String,
        phase: String,
        node_name: String,
        ip: String,
        containers: Vec<String>,
        restarts: u32,
        age: String,
    }

    #[derive(Serialize)]
    struct ProfileItem {
        name: String,
        description: String,
        cpu_cores: u32,
        memory: String,
        disk_size: String,
        use_cases: Vec<String>,
    }

    // ── Helpers ─────────────────────────────────────────────────────

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

    /// Certificate verifier that accepts all certs (for K8s self-signed API server certs).
    #[derive(Debug)]
    #[allow(dead_code)]
    struct AcceptAllVerifier;

    impl rustls::client::danger::ServerCertVerifier for AcceptAllVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &rustls::pki_types::CertificateDer<'_>,
            _intermediates: &[rustls::pki_types::CertificateDer<'_>],
            _server_name: &rustls::pki_types::ServerName<'_>,
            _ocsp_response: &[u8],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
            vec![
                rustls::SignatureScheme::RSA_PKCS1_SHA256,
                rustls::SignatureScheme::RSA_PKCS1_SHA384,
                rustls::SignatureScheme::RSA_PKCS1_SHA512,
                rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
                rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
                rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
                rustls::SignatureScheme::RSA_PSS_SHA256,
                rustls::SignatureScheme::RSA_PSS_SHA384,
                rustls::SignatureScheme::RSA_PSS_SHA512,
                rustls::SignatureScheme::ED25519,
            ]
        }
    }

    /// Build a rustls ClientConfig that authenticates to the K8s API server.
    /// Supports client certificate auth (k3s/kubeadm) and falls back to no client auth.
    #[allow(dead_code)]
    fn build_k8s_tls_config() -> rustls::ClientConfig {
        #![allow(unused_imports)]

        // Try reading client cert/key from kubeconfig
        if let Ok(kubeconfig_path) = std::env::var("KUBECONFIG") {
            if let Ok(contents) = std::fs::read_to_string(&kubeconfig_path) {
                let mut cert_b64 = None;
                let mut key_b64 = None;
                for line in contents.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("client-certificate-data:") {
                        cert_b64 = Some(
                            trimmed
                                .trim_start_matches("client-certificate-data:")
                                .trim()
                                .to_string(),
                        );
                    }
                    if trimmed.starts_with("client-key-data:") {
                        key_b64 = Some(
                            trimmed
                                .trim_start_matches("client-key-data:")
                                .trim()
                                .to_string(),
                        );
                    }
                }

                if let (Some(cert_b64), Some(key_b64)) = (cert_b64, key_b64) {
                    use base64::Engine;
                    let decoder = base64::engine::general_purpose::STANDARD;
                    if let (Ok(cert_pem), Ok(key_pem)) =
                        (decoder.decode(&cert_b64), decoder.decode(&key_b64))
                    {
                        // Parse PEM cert
                        let mut certs = Vec::new();
                        let mut cursor = &cert_pem[..];
                        while let Ok(Some(item)) =
                            rustls_pemfile::read_one(&mut cursor)
                        {
                            if let rustls_pemfile::Item::X509Certificate(cert) = item {
                                certs.push(cert);
                            }
                        }

                        // Parse PEM key
                        let mut key_cursor = &key_pem[..];
                        let private_key =
                            rustls_pemfile::private_key(&mut key_cursor)
                                .ok()
                                .flatten();

                        if !certs.is_empty() {
                            if let Some(key) = private_key {
                                if let Ok(cfg) = rustls::ClientConfig::builder()
                                    .dangerous()
                                    .with_custom_certificate_verifier(Arc::new(
                                        AcceptAllVerifier,
                                    ))
                                    .with_client_auth_cert(certs, key)
                                {
                                    log::info!("VNC proxy: using client certificate auth");
                                    return cfg;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: no client auth
        log::warn!("VNC proxy: no client certificate found, using anonymous TLS");
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
            .with_no_client_auth()
    }

    /// Simple base64 encoding for WebSocket key generation.
    #[allow(dead_code)]
    fn base64_encode_simple(value: u128) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bytes = value.to_le_bytes();
        let mut result = String::with_capacity(24);
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let triple = (b0 << 16) | (b1 << 8) | b2;
            result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
            }
            if chunk.len() > 2 {
                result.push(CHARS[(triple & 0x3F) as usize] as char);
            }
        }
        // Pad to exactly 24 chars (standard WS key length)
        while result.len() < 24 {
            result.push('=');
        }
        result.truncate(24);
        result
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
}
