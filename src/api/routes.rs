// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API route definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub method: String,
    pub handler: String,
    pub middleware: Vec<String>,
    pub version: ApiVersion,
}

/// API version
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiVersion {
    V1,
    V2,
    Custom(String),
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiVersion::V1 => write!(f, "v1"),
            ApiVersion::V2 => write!(f, "v2"),
            ApiVersion::Custom(v) => write!(f, "{}", v),
        }
    }
}

impl ApiVersion {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "v1" => Some(ApiVersion::V1),
            "v2" => Some(ApiVersion::V2),
            other => Some(ApiVersion::Custom(other.to_string())),
        }
    }

    pub fn prefix(&self) -> String {
        format!("/api/{}", self)
    }
}

impl Route {
    pub fn new(method: &str, path: &str, handler: &str) -> Self {
        Self {
            path: path.to_string(),
            method: method.to_string(),
            handler: handler.to_string(),
            middleware: Vec::new(),
            version: ApiVersion::V1,
        }
    }

    pub fn with_middleware(mut self, middleware: &str) -> Self {
        self.middleware.push(middleware.to_string());
        self
    }

    pub fn with_version(mut self, version: ApiVersion) -> Self {
        self.version = version;
        self
    }

    pub fn full_path(&self) -> String {
        format!("{}{}", self.version.prefix(), self.path)
    }

    pub fn has_middleware(&self, name: &str) -> bool {
        self.middleware.iter().any(|m| m == name)
    }
}

/// Route group for organizing related routes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteGroup {
    pub name: String,
    pub prefix: String,
    pub routes: Vec<Route>,
    pub middleware: Vec<String>,
}

impl RouteGroup {
    pub fn new(name: &str, prefix: &str) -> Self {
        Self {
            name: name.to_string(),
            prefix: prefix.to_string(),
            routes: Vec::new(),
            middleware: Vec::new(),
        }
    }

    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    pub fn with_middleware(mut self, middleware: &str) -> Self {
        self.middleware.push(middleware.to_string());
        self
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    pub fn get_routes(&self) -> Vec<Route> {
        self.routes
            .iter()
            .map(|r| {
                let mut route = r.clone();
                route.path = format!("{}{}", self.prefix, route.path);
                for mw in &self.middleware {
                    if !route.middleware.contains(mw) {
                        route.middleware.push(mw.clone());
                    }
                }
                route
            })
            .collect()
    }
}

/// Router for managing all API routes
pub struct Router {
    groups: HashMap<String, RouteGroup>,
    global_middleware: Vec<String>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
            global_middleware: Vec::new(),
        }
    }

    pub fn add_global_middleware(&mut self, middleware: &str) {
        self.global_middleware.push(middleware.to_string());
    }

    pub fn add_group(&mut self, group: RouteGroup) {
        self.groups.insert(group.name.clone(), group);
    }

    pub fn get_group(&self, name: &str) -> Option<&RouteGroup> {
        self.groups.get(name)
    }

    pub fn all_routes(&self) -> Vec<Route> {
        let mut all = Vec::new();
        for group in self.groups.values() {
            all.extend(group.get_routes());
        }
        all
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn total_routes(&self) -> usize {
        self.groups.values().map(|g| g.route_count()).sum()
    }

    pub fn routes_by_method(&self, method: &str) -> Vec<Route> {
        self.all_routes()
            .into_iter()
            .filter(|r| r.method == method)
            .collect()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the default Veyron API router
pub fn build_default_router() -> Router {
    let mut router = Router::new();
    router.add_global_middleware("request-id");
    router.add_global_middleware("logging");

    // VM routes
    let mut vm_group = RouteGroup::new("vms", "/vms");
    vm_group.add_route(Route::new("GET", "", "list_vms"));
    vm_group.add_route(Route::new("POST", "", "create_vm").with_middleware("auth"));
    vm_group.add_route(Route::new("GET", "/:name", "get_vm"));
    vm_group.add_route(Route::new("PUT", "/:name", "update_vm").with_middleware("auth"));
    vm_group.add_route(Route::new("DELETE", "/:name", "delete_vm").with_middleware("auth"));
    vm_group.add_route(Route::new("POST", "/:name/start", "start_vm").with_middleware("auth"));
    vm_group.add_route(Route::new("POST", "/:name/stop", "stop_vm").with_middleware("auth"));
    vm_group.add_route(Route::new("POST", "/:name/restart", "restart_vm").with_middleware("auth"));
    router.add_group(vm_group);

    // Template routes
    let mut template_group = RouteGroup::new("templates", "/templates");
    template_group.add_route(Route::new("GET", "", "list_templates"));
    template_group.add_route(Route::new("GET", "/:name", "get_template"));
    router.add_group(template_group);

    // Profile routes
    let mut profile_group = RouteGroup::new("profiles", "/profiles");
    profile_group.add_route(Route::new("GET", "", "list_profiles"));
    profile_group.add_route(Route::new("GET", "/:name", "get_profile"));
    router.add_group(profile_group);

    // Blueprint routes
    let mut blueprint_group = RouteGroup::new("blueprints", "/blueprints");
    blueprint_group.add_route(Route::new("GET", "", "list_blueprints"));
    blueprint_group.add_route(Route::new("GET", "/:name", "get_blueprint"));
    blueprint_group
        .add_route(Route::new("POST", "/:name/deploy", "deploy_blueprint").with_middleware("auth"));
    router.add_group(blueprint_group);

    // Snapshot routes
    let mut snapshot_group = RouteGroup::new("snapshots", "/snapshots");
    snapshot_group.add_route(Route::new("GET", "", "list_snapshots"));
    snapshot_group.add_route(Route::new("POST", "", "create_snapshot").with_middleware("auth"));
    snapshot_group.add_route(Route::new("GET", "/:id", "get_snapshot"));
    snapshot_group
        .add_route(Route::new("DELETE", "/:id", "delete_snapshot").with_middleware("auth"));
    snapshot_group
        .add_route(Route::new("POST", "/:id/restore", "restore_snapshot").with_middleware("auth"));
    router.add_group(snapshot_group);

    // Events/Activity routes
    let mut events_group = RouteGroup::new("events", "/events");
    events_group.add_route(Route::new("GET", "", "list_events"));
    events_group.add_route(Route::new("GET", "/recent", "list_recent_events"));
    events_group.add_route(Route::new("GET", "/vm/:name", "list_vm_events"));
    router.add_group(events_group);

    // RDP routes
    let mut rdp_group = RouteGroup::new("rdp", "/rdp");
    rdp_group.add_route(Route::new("GET", "/sessions", "list_rdp_sessions"));
    rdp_group
        .add_route(Route::new("POST", "/sessions", "create_rdp_session").with_middleware("auth"));
    rdp_group.add_route(Route::new("GET", "/sessions/:id", "get_rdp_session"));
    rdp_group.add_route(
        Route::new("DELETE", "/sessions/:id", "delete_rdp_session").with_middleware("auth"),
    );
    rdp_group.add_route(Route::new(
        "POST",
        "/sessions/:id/resize",
        "resize_rdp_session",
    ));
    rdp_group.add_route(Route::new(
        "GET",
        "/sessions/:id/clipboard",
        "get_clipboard",
    ));
    rdp_group.add_route(Route::new(
        "POST",
        "/sessions/:id/clipboard",
        "send_clipboard",
    ));
    rdp_group.add_route(Route::new(
        "GET",
        "/sessions/:id/stats",
        "get_session_stats",
    ));
    rdp_group.add_route(Route::new(
        "GET",
        "/sessions/:id/screenshot",
        "take_screenshot",
    ));
    rdp_group.add_route(Route::new(
        "POST",
        "/sessions/:id/disconnect",
        "disconnect_rdp_session",
    ));
    rdp_group.add_route(Route::new(
        "POST",
        "/sessions/:id/reconnect",
        "reconnect_rdp_session",
    ));
    rdp_group.add_route(Route::new("GET", "/vms", "list_rdp_capable_vms"));
    rdp_group.add_route(Route::new("GET", "/config/defaults", "get_default_config"));
    rdp_group.add_route(Route::new("GET", "/gateway", "get_gateway_config"));
    rdp_group
        .add_route(Route::new("POST", "/gateway", "set_gateway_config").with_middleware("auth"));
    router.add_group(rdp_group);

    // Health routes
    let mut health_group = RouteGroup::new("health", "/health");
    health_group.add_route(Route::new("GET", "", "health_check"));
    health_group.add_route(Route::new("GET", "/ready", "readiness_check"));
    health_group.add_route(Route::new("GET", "/live", "liveness_check"));
    router.add_group(health_group);

    router
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_display() {
        assert_eq!(ApiVersion::V1.to_string(), "v1");
        assert_eq!(ApiVersion::V2.to_string(), "v2");
        assert_eq!(ApiVersion::Custom("v3".to_string()).to_string(), "v3");
    }

    #[test]
    fn test_api_version_from_str() {
        assert_eq!(ApiVersion::parse("v1"), Some(ApiVersion::V1));
        assert_eq!(ApiVersion::parse("v2"), Some(ApiVersion::V2));
    }

    #[test]
    fn test_api_version_prefix() {
        assert_eq!(ApiVersion::V1.prefix(), "/api/v1");
        assert_eq!(ApiVersion::V2.prefix(), "/api/v2");
    }

    #[test]
    fn test_route_new() {
        let route = Route::new("GET", "/vms", "list_vms");
        assert_eq!(route.method, "GET");
        assert_eq!(route.path, "/vms");
        assert_eq!(route.handler, "list_vms");
        assert_eq!(route.version, ApiVersion::V1);
    }

    #[test]
    fn test_route_full_path() {
        let route = Route::new("GET", "/vms", "list_vms");
        assert_eq!(route.full_path(), "/api/v1/vms");
    }

    #[test]
    fn test_route_with_middleware() {
        let route = Route::new("POST", "/vms", "create_vm")
            .with_middleware("auth")
            .with_middleware("validate");

        assert!(route.has_middleware("auth"));
        assert!(route.has_middleware("validate"));
        assert!(!route.has_middleware("cache"));
    }

    #[test]
    fn test_route_with_version() {
        let route = Route::new("GET", "/vms", "list_vms").with_version(ApiVersion::V2);
        assert_eq!(route.full_path(), "/api/v2/vms");
    }

    #[test]
    fn test_route_group_new() {
        let group = RouteGroup::new("vms", "/vms");
        assert_eq!(group.name, "vms");
        assert_eq!(group.prefix, "/vms");
        assert_eq!(group.route_count(), 0);
    }

    #[test]
    fn test_route_group_add_route() {
        let mut group = RouteGroup::new("vms", "/vms");
        group.add_route(Route::new("GET", "", "list_vms"));
        group.add_route(Route::new("POST", "", "create_vm"));

        assert_eq!(group.route_count(), 2);
    }

    #[test]
    fn test_route_group_get_routes() {
        let mut group = RouteGroup::new("vms", "/vms").with_middleware("rate-limit");
        group.add_route(Route::new("GET", "/:id", "get_vm"));

        let routes = group.get_routes();
        assert_eq!(routes[0].path, "/vms/:id");
        assert!(routes[0].has_middleware("rate-limit"));
    }

    #[test]
    fn test_router_new() {
        let router = Router::new();
        assert_eq!(router.group_count(), 0);
        assert_eq!(router.total_routes(), 0);
    }

    #[test]
    fn test_router_add_group() {
        let mut router = Router::new();
        let mut group = RouteGroup::new("vms", "/vms");
        group.add_route(Route::new("GET", "", "list_vms"));
        group.add_route(Route::new("POST", "", "create_vm"));
        router.add_group(group);

        assert_eq!(router.group_count(), 1);
        assert_eq!(router.total_routes(), 2);
    }

    #[test]
    fn test_router_all_routes() {
        let mut router = Router::new();

        let mut g1 = RouteGroup::new("vms", "/vms");
        g1.add_route(Route::new("GET", "", "list"));

        let mut g2 = RouteGroup::new("templates", "/templates");
        g2.add_route(Route::new("GET", "", "list"));

        router.add_group(g1);
        router.add_group(g2);

        assert_eq!(router.all_routes().len(), 2);
    }

    #[test]
    fn test_router_routes_by_method() {
        let mut router = Router::new();
        let mut group = RouteGroup::new("vms", "/vms");
        group.add_route(Route::new("GET", "", "list_vms"));
        group.add_route(Route::new("POST", "", "create_vm"));
        group.add_route(Route::new("GET", "/:id", "get_vm"));
        router.add_group(group);

        let gets = router.routes_by_method("GET");
        assert_eq!(gets.len(), 2);

        let posts = router.routes_by_method("POST");
        assert_eq!(posts.len(), 1);
    }

    #[test]
    fn test_build_default_router() {
        let router = build_default_router();
        assert!(router.group_count() >= 5);
        assert!(router.total_routes() >= 15);
    }

    #[test]
    fn test_default_router_has_vm_routes() {
        let router = build_default_router();
        let group = router.get_group("vms");
        assert!(group.is_some());
        assert!(group.unwrap().route_count() >= 5);
    }

    #[test]
    fn test_default_router_has_health_routes() {
        let router = build_default_router();
        let group = router.get_group("health");
        assert!(group.is_some());
    }
}
