// HTTP Server - Real web server using axum for serving API and dashboard
//
// This module is only compiled when the "web" feature is enabled.

#[cfg(feature = "web")]
pub mod web {
    use crate::api::{ApiResponse, HttpMethod, RequestContext};
    use crate::kube::KubeClient;
    use crate::tui::state::VmInfo;
    use axum::{
        extract::{DefaultBodyLimit, Path, Query, State},
        http::{header, HeaderMap, StatusCode},
        middleware,
        response::{Html, IntoResponse, Json},
        routing::{delete, get, post},
        Router,
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
                return Err(anyhow::anyhow!(
                    "TLS key file not found: {}",
                    self.key_path
                ));
            }
            Ok(())
        }
    }

    pub struct WebState {
        pub namespace: String,
        pub kube_client: KubeClient,
        pub api_key: Option<String>,
        rate_limiter: RateLimiterState,
    }

    impl WebState {
        pub async fn new(namespace: String, rate_limit_per_minute: u64) -> anyhow::Result<Self> {
            let api_key = std::env::var("VMROGUE_API_KEY").ok().filter(|k| !k.is_empty());
            if api_key.is_none() {
                log::warn!(
                    "VMROGUE_API_KEY is not set - API will reject all requests. \
                     Set VMROGUE_API_KEY to enable access."
                );
            }
            let kube_client = KubeClient::new().await?;
            Ok(Self {
                namespace,
                kube_client,
                api_key,
                rate_limiter: RateLimiterState::new(rate_limit_per_minute, 60),
            })
        }

        pub fn client(&self) -> &KubeClient {
            &self.kube_client
        }
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
        if !bytes[bytes.len() - 1].is_ascii_lowercase()
            && !bytes[bytes.len() - 1].is_ascii_digit()
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
        // Allow health endpoint without auth
        if request.uri().path() == "/api/v1/health" {
            return next.run(request).await.into_response();
        }

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

        // Check X-API-Key header first, then Authorization: Bearer
        let provided_key = headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                headers
                    .get(header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer ").map(|s| s.to_string()))
            });

        match provided_key {
            Some(key) if constant_time_eq(key.as_bytes(), expected_key.as_bytes()) => {
                next.run(request).await.into_response()
            }
            _ => {
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
        // Allow health endpoint without rate limiting
        if request.uri().path() == "/api/v1/health" {
            return next.run(request).await.into_response();
        }

        let s = state.read().await;
        if !s.rate_limiter.check_rate_limit() {
            let (status, json) = err_json(429, "RATE_LIMITED", "Too many requests. Please slow down.");
            return (status, json).into_response();
        }
        drop(s);
        next.run(request).await.into_response()
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
        headers.insert("content-security-policy", "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'".parse().unwrap());
        headers.insert("referrer-policy", "no-referrer".parse().unwrap());
        // HSTS: instruct browsers to only use HTTPS for 1 year
        headers.insert("strict-transport-security", "max-age=31536000; includeSubDomains".parse().unwrap());
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
            // Dashboard
            .route("/dashboard", get(dashboard_handler))
            // VM endpoints
            .route("/api/v1/vms", get(list_vms_handler))
            .route("/api/v1/vms/:ns/:name", get(get_vm_handler))
            .route("/api/v1/vms/:ns/:name", delete(delete_vm_handler))
            .route("/api/v1/vms/:ns/:name/start", post(start_vm_handler))
            .route("/api/v1/vms/:ns/:name/stop", post(stop_vm_handler))
            .route("/api/v1/vms/:ns/:name/restart", post(restart_vm_handler))
            // Snapshots
            .route("/api/v1/snapshots", get(list_snapshots_handler))
            .route("/api/v1/snapshots/:ns/:vm", get(list_vm_snapshots_handler))
            .route(
                "/api/v1/snapshots/:ns/:name/delete",
                post(delete_snapshot_handler),
            )
            // Events
            .route("/api/v1/events", get(list_events_handler))
            .route("/api/v1/events/recent", get(recent_events_handler))
            // Dashboard overview
            .route("/api/v1/dashboard/overview", get(dashboard_overview_handler))
            // Health
            .route("/api/v1/health", get(health_handler))
            .with_state(state.clone())
            // Layers applied in reverse order (outermost = last .layer() call)
            .layer(middleware::from_fn(security_headers_middleware))
            .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
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
        let state = Arc::new(RwLock::new(WebState::new(namespace, rate_limit_per_minute).await?));
        let app = build_router(state);
        let addr = format!("{}:{}", host, port);

        if let Some(tls) = tls_config {
            log::info!("Starting HTTPS server on {}", addr);

            // Install the ring crypto provider (already used by kube-client)
            let _ = rustls::crypto::ring::default_provider().install_default();

            let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                &tls.cert_path,
                &tls.key_path,
            )
            .await?;
            let addr: std::net::SocketAddr = addr.parse()?;
            axum_server::bind_rustls(addr, rustls_config)
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

    fn req_ctx(method: HttpMethod, path: &str) -> RequestContext {
        RequestContext::new(method, path)
    }

    /// Sanitize internal error details before sending to clients.
    ///
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
                let vm_infos: Vec<VmInfo> = vms.iter().map(|vm| VmInfo::from_vm_with_ip(vm, None)).collect();
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

    // ── Snapshot Endpoints ────────────────────────────────────────

    async fn list_snapshots_handler(
        State(state): State<SharedState>,
        Query(query): Query<VmQuery>,
    ) -> impl IntoResponse {
        let namespace = {
            let s = state.read().await;
            query.namespace.clone().unwrap_or_else(|| s.namespace.clone())
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
                            .or_else(|| {
                                e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339())
                            })
                            .unwrap_or_default(),
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/events");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    async fn recent_events_handler(
        State(state): State<SharedState>,
    ) -> impl IntoResponse {
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
                            .or_else(|| {
                                e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339())
                            })
                            .unwrap_or_default(),
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/events/recent");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        }
    }

    // ── Dashboard Overview ────────────────────────────────────────

    async fn dashboard_overview_handler(
        State(state): State<SharedState>,
    ) -> impl IntoResponse {
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

        let overview = DashboardOverview {
            cluster: ClusterStats {
                total_vms: total,
                running_vms: running,
                stopped_vms: stopped,
                error_vms: error,
                total_vcpus_allocated: total_cpus,
                total_memory_allocated_gb: (total_memory_bytes as f64)
                    / (1024.0 * 1024.0 * 1024.0),
                total_snapshots: snapshot_count,
            },
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

    // ── Helpers ─────────────────────────────────────────────────────

    fn ok_json<T: Serialize>(data: &T) -> (StatusCode, Json<serde_json::Value>) {
        let value = serde_json::to_value(data).unwrap_or_else(|e| {
            serde_json::json!({"error": format!("serialization failed: {}", e)})
        });
        // Extract the status code from the serialized response if present,
        // so that 201/204/etc. responses get the correct HTTP status.
        let status_code = value
            .get("status")
            .and_then(|v| v.as_u64())
            .and_then(|s| StatusCode::from_u16(s as u16).ok())
            .unwrap_or(StatusCode::OK);
        (status_code, Json(value))
    }

    fn err_json(
        status: u16,
        code: &str,
        message: &str,
    ) -> (StatusCode, Json<serde_json::Value>) {
        let ctx = req_ctx(HttpMethod::GET, "");
        let resp = ApiResponse::error(status, code, message, &ctx.request_id);
        let value = serde_json::to_value(&resp).unwrap_or_else(|_| {
            serde_json::json!({"status": status, "error": code, "message": "internal error"})
        });
        (
            StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(value),
        )
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
