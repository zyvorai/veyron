use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ApiConfig;

/// API server state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

impl std::fmt::Display for ServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerState::Stopped => write!(f, "stopped"),
            ServerState::Starting => write!(f, "starting"),
            ServerState::Running => write!(f, "running"),
            ServerState::Stopping => write!(f, "stopping"),
            ServerState::Error => write!(f, "error"),
        }
    }
}

/// API server instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiServer {
    pub config: ApiConfig,
    pub state: ServerState,
    pub stats: ServerStats,
    pub started_at: Option<DateTime<Utc>>,
    pub version: String,
}

/// Server statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    pub total_requests: u64,
    pub active_connections: u32,
    pub requests_per_second: f64,
    pub avg_response_ms: f64,
    pub error_count: u64,
    pub status_codes: HashMap<u16, u64>,
}

impl ServerStats {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            active_connections: 0,
            requests_per_second: 0.0,
            avg_response_ms: 0.0,
            error_count: 0,
            status_codes: HashMap::new(),
        }
    }

    pub fn record_request(&mut self, status_code: u16, duration_ms: f64) {
        self.total_requests += 1;
        *self.status_codes.entry(status_code).or_insert(0) += 1;

        if status_code >= 500 {
            self.error_count += 1;
        }

        // Welford's online mean: avoids precision loss at high request counts
        // delta = x - mean; mean += delta / n
        let delta = duration_ms - self.avg_response_ms;
        self.avg_response_ms += delta / self.total_requests as f64;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 100.0;
        }
        ((self.total_requests - self.error_count) as f64 / self.total_requests as f64) * 100.0
    }

    pub fn error_rate(&self) -> f64 {
        100.0 - self.success_rate()
    }
}

impl Default for ServerStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiServer {
    pub fn new(config: ApiConfig) -> Self {
        Self {
            config,
            state: ServerState::Stopped,
            stats: ServerStats::new(),
            started_at: None,
            version: "v1".to_string(),
        }
    }

    pub fn start(&mut self) {
        self.state = ServerState::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn stop(&mut self) {
        self.state = ServerState::Stopped;
        self.started_at = None;
    }

    pub fn is_running(&self) -> bool {
        self.state == ServerState::Running
    }

    pub fn uptime_secs(&self) -> Option<i64> {
        self.started_at
            .map(|started| (Utc::now() - started).num_seconds())
    }

    pub fn api_version(&self) -> &str {
        &self.version
    }

    pub fn health_status(&self) -> HealthStatus {
        if self.state != ServerState::Running {
            return HealthStatus {
                status: "unhealthy".to_string(),
                version: self.version.clone(),
                uptime_secs: 0,
                checks: HashMap::new(),
            };
        }

        let mut checks = HashMap::new();
        checks.insert("server".to_string(), "ok".to_string());

        if self.config.tls_enabled {
            checks.insert("tls".to_string(), "ok".to_string());
        }

        if self.config.auth_enabled {
            checks.insert("auth".to_string(), "ok".to_string());
        }

        HealthStatus {
            status: "healthy".to_string(),
            version: self.version.clone(),
            uptime_secs: self.uptime_secs().unwrap_or(0),
            checks,
        }
    }
}

/// Health status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_secs: i64,
    pub checks: HashMap<String, String>,
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        self.status == "healthy"
    }

    pub fn check_count(&self) -> usize {
        self.checks.len()
    }
}

/// API endpoint registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub method: String,
    pub path: String,
    pub description: String,
    pub auth_required: bool,
    pub rate_limited: bool,
}

impl Endpoint {
    pub fn new(method: &str, path: &str, description: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            description: description.to_string(),
            auth_required: false,
            rate_limited: true,
        }
    }

    pub fn with_auth(mut self) -> Self {
        self.auth_required = true;
        self
    }

    pub fn without_rate_limit(mut self) -> Self {
        self.rate_limited = false;
        self
    }
}

/// Get default API endpoints
pub fn default_endpoints() -> Vec<Endpoint> {
    vec![
        // VM operations
        Endpoint::new("GET", "/api/v1/vms", "List virtual machines"),
        Endpoint::new("POST", "/api/v1/vms", "Create a virtual machine").with_auth(),
        Endpoint::new("GET", "/api/v1/vms/:name", "Get VM details"),
        Endpoint::new("PUT", "/api/v1/vms/:name", "Update VM configuration").with_auth(),
        Endpoint::new("DELETE", "/api/v1/vms/:name", "Delete a VM").with_auth(),
        Endpoint::new("POST", "/api/v1/vms/:name/start", "Start a VM").with_auth(),
        Endpoint::new("POST", "/api/v1/vms/:name/stop", "Stop a VM").with_auth(),
        Endpoint::new("POST", "/api/v1/vms/:name/restart", "Restart a VM").with_auth(),
        // Templates
        Endpoint::new("GET", "/api/v1/templates", "List available templates"),
        Endpoint::new("GET", "/api/v1/templates/:name", "Get template details"),
        // Profiles
        Endpoint::new("GET", "/api/v1/profiles", "List resource profiles"),
        Endpoint::new("GET", "/api/v1/profiles/:name", "Get profile details"),
        // Blueprints
        Endpoint::new("GET", "/api/v1/blueprints", "List multi-VM blueprints"),
        Endpoint::new(
            "POST",
            "/api/v1/blueprints/:name/deploy",
            "Deploy a blueprint",
        )
        .with_auth(),
        // Snapshots
        Endpoint::new("GET", "/api/v1/vms/:name/snapshots", "List VM snapshots"),
        Endpoint::new("POST", "/api/v1/vms/:name/snapshots", "Create VM snapshot").with_auth(),
        Endpoint::new(
            "POST",
            "/api/v1/snapshots/:id/restore",
            "Restore from snapshot",
        )
        .with_auth(),
        // Events/Activity
        Endpoint::new("GET", "/api/v1/events", "List all activity events"),
        Endpoint::new("GET", "/api/v1/events/recent", "List recent activity events"),
        Endpoint::new(
            "GET",
            "/api/v1/events/vm/:name",
            "List events for a specific VM",
        ),
        // Health
        Endpoint::new("GET", "/api/v1/health", "API health check").without_rate_limit(),
        Endpoint::new("GET", "/api/v1/vms/:name/health", "VM health check"),
        // Monitoring
        Endpoint::new("GET", "/api/v1/vms/:name/metrics", "Get VM metrics"),
        Endpoint::new("GET", "/api/v1/metrics", "Get cluster metrics"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_state_display() {
        assert_eq!(ServerState::Stopped.to_string(), "stopped");
        assert_eq!(ServerState::Running.to_string(), "running");
        assert_eq!(ServerState::Starting.to_string(), "starting");
        assert_eq!(ServerState::Stopping.to_string(), "stopping");
        assert_eq!(ServerState::Error.to_string(), "error");
    }

    #[test]
    fn test_api_server_new() {
        let config = ApiConfig::new(8080);
        let server = ApiServer::new(config);
        assert_eq!(server.state, ServerState::Stopped);
        assert!(!server.is_running());
        assert!(server.started_at.is_none());
    }

    #[test]
    fn test_api_server_start_stop() {
        let config = ApiConfig::new(8080);
        let mut server = ApiServer::new(config);

        server.start();
        assert!(server.is_running());
        assert!(server.started_at.is_some());

        server.stop();
        assert!(!server.is_running());
        assert!(server.started_at.is_none());
    }

    #[test]
    fn test_api_server_health_stopped() {
        let config = ApiConfig::new(8080);
        let server = ApiServer::new(config);
        let health = server.health_status();
        assert!(!health.is_healthy());
    }

    #[test]
    fn test_api_server_health_running() {
        let config = ApiConfig::new(8080);
        let mut server = ApiServer::new(config);
        server.start();

        let health = server.health_status();
        assert!(health.is_healthy());
        assert!(health.check_count() >= 1);
    }

    #[test]
    fn test_api_server_health_with_tls() {
        let config = ApiConfig::new(443).with_tls("cert", "key");
        let mut server = ApiServer::new(config);
        server.start();

        let health = server.health_status();
        assert!(health.checks.contains_key("tls"));
    }

    #[test]
    fn test_server_stats_new() {
        let stats = ServerStats::new();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.success_rate(), 100.0);
    }

    #[test]
    fn test_server_stats_record_request() {
        let mut stats = ServerStats::new();
        stats.record_request(200, 15.0);
        stats.record_request(201, 25.0);

        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.avg_response_ms, 20.0);
    }

    #[test]
    fn test_server_stats_error_tracking() {
        let mut stats = ServerStats::new();
        stats.record_request(200, 10.0);
        stats.record_request(500, 50.0);

        assert_eq!(stats.error_count, 1);
        assert_eq!(stats.success_rate(), 50.0);
        assert_eq!(stats.error_rate(), 50.0);
    }

    #[test]
    fn test_server_stats_status_codes() {
        let mut stats = ServerStats::new();
        stats.record_request(200, 10.0);
        stats.record_request(200, 10.0);
        stats.record_request(404, 5.0);

        assert_eq!(stats.status_codes.get(&200), Some(&2));
        assert_eq!(stats.status_codes.get(&404), Some(&1));
    }

    #[test]
    fn test_endpoint_new() {
        let ep = Endpoint::new("GET", "/api/v1/vms", "List VMs");
        assert_eq!(ep.method, "GET");
        assert_eq!(ep.path, "/api/v1/vms");
        assert!(!ep.auth_required);
        assert!(ep.rate_limited);
    }

    #[test]
    fn test_endpoint_with_auth() {
        let ep = Endpoint::new("POST", "/api/v1/vms", "Create VM").with_auth();
        assert!(ep.auth_required);
    }

    #[test]
    fn test_endpoint_without_rate_limit() {
        let ep = Endpoint::new("GET", "/api/v1/health", "Health").without_rate_limit();
        assert!(!ep.rate_limited);
    }

    #[test]
    fn test_default_endpoints() {
        let endpoints = default_endpoints();
        assert!(!endpoints.is_empty());
        assert!(endpoints.iter().any(|e| e.path == "/api/v1/vms"));
        assert!(endpoints.iter().any(|e| e.path == "/api/v1/health"));
    }

    #[test]
    fn test_health_status_is_healthy() {
        let health = HealthStatus {
            status: "healthy".to_string(),
            version: "v1".to_string(),
            uptime_secs: 100,
            checks: HashMap::new(),
        };
        assert!(health.is_healthy());
    }
}
