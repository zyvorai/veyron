//! Path rules shared by dashboard / operator UI middleware.

/// Routes reachable without an API key (shell HTML, health, static assets).
pub fn is_auth_exempt_path(path: &str) -> bool {
    path == "/api/v1/health"
        || path == "/"
        || path == "/dashboard"
        || path.starts_with("/dashboard-next")
        || path.starts_with("/assets/")
}

/// Operator UI + classic dashboard API prefixes exempt from the global rate limiter.
pub const OPERATOR_API_PREFIXES: &[&str] = &[
    "/api/v1/vms",
    "/api/v1/ws",
    "/api/v1/events",
    "/api/v1/nodes",
    "/api/v1/pods",
    "/api/v1/snapshots",
    "/api/v1/snapshot-schedules",
    "/api/v1/dashboard",
    "/api/v1/templates",
    "/api/v1/profiles",
    "/api/v1/namespaces",
    "/api/v1/storage",
    "/api/v1/activity",
    "/api/v1/alerts",
    "/api/v1/custom-resources",
    "/api/v1/gitops",
    "/api/v1/monitoring",
    "/api/v1/security",
    "/api/v1/costs",
    "/api/v1/logs",
    "/api/v1/incidents",
];

pub fn is_operator_api_path(path: &str) -> bool {
    OPERATOR_API_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

pub fn is_rate_limit_exempt_path(path: &str) -> bool {
    path == "/api/v1/health"
        || path == "/"
        || path == "/dashboard"
        || path.starts_with("/dashboard-next")
        || is_operator_api_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_exempt_includes_operator_shell_and_health() {
        for path in [
            "/",
            "/dashboard",
            "/dashboard-next/",
            "/dashboard-next/assets/index.js",
            "/assets/novnc.min.js",
            "/api/v1/health",
        ] {
            assert!(is_auth_exempt_path(path), "{path}");
        }
        assert!(!is_auth_exempt_path("/api/v1/vms"));
    }

    #[test]
    fn operator_api_prefixes_cover_dashboard_next_reads() {
        for path in [
            "/api/v1/namespaces",
            "/api/v1/nodes",
            "/api/v1/storage/classes",
            "/api/v1/storage/pvcs?namespace=all",
            "/api/v1/vms?namespace=all",
            "/api/v1/templates",
            "/api/v1/alerts",
            "/api/v1/snapshots/default/my-vm",
            "/api/v1/custom-resources",
            "/api/v1/gitops/status",
            "/api/v1/pods?namespace=all",
            "/api/v1/events/recent",
            "/api/v1/monitoring/status",
            "/api/v1/security/posture",
            "/api/v1/security/findings",
            "/api/v1/costs/summary",
            "/api/v1/costs/budgets",
            "/api/v1/logs",
            "/api/v1/incidents/timeline",
            "/api/v1/snapshot-schedules",
        ] {
            assert!(is_operator_api_path(path), "{path}");
        }
    }

    #[test]
    fn rate_limit_exempt_includes_operator_api_and_shell() {
        assert!(is_rate_limit_exempt_path("/dashboard-next/"));
        assert!(is_rate_limit_exempt_path("/api/v1/alerts"));
        assert!(is_rate_limit_exempt_path("/api/v1/costs/summary"));
    }
}
