// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Path rules shared by dashboard middleware.

/// Routes reachable without an API key (shell HTML, health, static assets).
pub fn is_auth_exempt_path(path: &str) -> bool {
    path == "/api/v1/health"
        || path == "/api/v1/platform/guestkit/binary"
        || path == "/api/v1/auth/oidc/config"
        || path == "/api/v1/auth/oidc/token"
        || path == "/api/v1/auth/login"
        || path == "/"
        || path == "/dashboard"
        || path.starts_with("/assets/")
}

/// Dashboard API prefixes exempt from the global rate limiter.
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
    "/api/v1/crds",
    "/api/v1/catalog",
    "/api/v1/experience",
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
    "/api/v1/tenants",
    "/api/v1/network/nads",
    "/api/v1/images/catalog",
    "/api/v1/velero/status",
    "/api/v1/backups",
    "/api/v1/dr",
    "/api/v1/metrics/timeline",
    "/api/v1/traces",
    "/api/v1/integrations",
    "/api/v1/clusters",
    "/api/v1/compliance",
    "/api/v1/heatmap",
    "/api/v1/dashboards",
    "/api/v1/network-policies",
    "/api/v1/cilium",
    "/api/v1/autoscaler",
];

pub fn is_operator_api_path(path: &str) -> bool {
    OPERATOR_API_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

pub fn is_rate_limit_exempt_path(path: &str, method: &str) -> bool {
    // Fixed low-risk endpoints (health, OIDC, shell) are exempt regardless of method.
    if path == "/api/v1/health"
        || path == "/api/v1/auth/oidc/config"
        || path == "/api/v1/auth/oidc/token"
        || path == "/"
        || path == "/dashboard"
    {
        return true;
    }
    // The broad dashboard-API prefix exemption is for high-frequency *polling*.
    // Never exempt mutating methods — many exempt prefixes (/vms, /backups, /dr,
    // /clusters, /snapshots) have expensive write sub-routes that must stay
    // rate-limited (create VM, DR failover, restore, …).
    matches!(method, "GET" | "HEAD" | "OPTIONS") && is_operator_api_path(path)
}

/// POST routes that invoke the optional LLM backend (separate rate bucket).
pub fn is_ai_rate_limited_path(path: &str, method: &str) -> bool {
    if method != "POST" {
        return false;
    }
    path == "/api/v1/experience/copilot/chat" || path == "/api/v1/experience/copilot/ask"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_exempt_includes_dashboard_shell_and_health() {
        for path in [
            "/",
            "/dashboard",
            "/assets/novnc.min.js",
            "/api/v1/health",
            "/api/v1/platform/guestkit/binary",
            "/api/v1/auth/oidc/config",
            "/api/v1/auth/oidc/token",
            "/api/v1/auth/login",
        ] {
            assert!(is_auth_exempt_path(path), "{path}");
        }
        assert!(!is_auth_exempt_path("/api/v1/vms"));
        assert!(!is_auth_exempt_path("/api/v1/auth/users"));
    }

    #[test]
    fn dashboard_api_prefixes_cover_common_reads() {
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
            "/api/v1/tenants",
            "/api/v1/network/nads",
            "/api/v1/images/catalog",
            "/api/v1/velero/status",
            "/api/v1/backups",
            "/api/v1/crds/templates",
            "/api/v1/crds/profiles",
            "/api/v1/catalog/status",
            "/api/v1/experience/home",
            "/api/v1/experience/desktop",
            "/api/v1/metrics/timeline",
            "/api/v1/traces",
            "/api/v1/integrations/status",
            "/api/v1/clusters",
        ] {
            assert!(is_operator_api_path(path), "{path}");
        }
    }

    #[test]
    fn rate_limit_exempt_includes_dashboard_api_and_shell() {
        // Shell + fixed endpoints exempt regardless of method.
        assert!(is_rate_limit_exempt_path("/dashboard", "GET"));
        // Dashboard-API polling (GET) exempt; mutations are NOT.
        assert!(is_rate_limit_exempt_path("/api/v1/alerts", "GET"));
        assert!(is_rate_limit_exempt_path("/api/v1/costs/summary", "GET"));
        assert!(!is_rate_limit_exempt_path("/api/v1/vms", "POST"));
        assert!(!is_rate_limit_exempt_path(
            "/api/v1/backups/x/restore",
            "POST"
        ));
        assert!(!is_rate_limit_exempt_path("/api/v1/dr/failover", "POST"));
    }

    #[test]
    fn ai_rate_limited_paths_are_post_copilot_only() {
        assert!(is_ai_rate_limited_path(
            "/api/v1/experience/copilot/chat",
            "POST"
        ));
        assert!(is_ai_rate_limited_path(
            "/api/v1/experience/copilot/ask",
            "POST"
        ));
        assert!(!is_ai_rate_limited_path(
            "/api/v1/experience/copilot/chat",
            "GET"
        ));
        assert!(!is_ai_rate_limited_path(
            "/api/v1/experience/copilot/status",
            "GET"
        ));
    }
}
