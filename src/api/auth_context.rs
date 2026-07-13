// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Request auth context and route-level RBAC rules.

use axum::http::Method;

use crate::api::http_server::web::ApiRole;

/// Authenticated caller role attached to each authorized request.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthContext {
    pub role: ApiRole,
}

impl AuthContext {
    pub fn allows(&self, required: &ApiRole) -> bool {
        role_rank(&self.role) >= role_rank(required)
    }
}

fn role_rank(role: &ApiRole) -> u8 {
    match role {
        ApiRole::ReadOnly => 0,
        ApiRole::Write => 1,
        ApiRole::Admin => 2,
    }
}

/// Minimum role required for a route. Safe methods always require ReadOnly.
pub fn min_role_for_route(method: &Method, path: &str) -> ApiRole {
    if matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS) {
        return ApiRole::ReadOnly;
    }

    // The `/api/v1/veyron/*` product alias (`http_server.rs`) re-mounts the entire
    // handler set under a second prefix. Canonicalize it back to `/api/v1/*` before
    // evaluating the admin gates below — otherwise a Write-scoped key could reach
    // admin-only mutations (tenants, dr, gitops/catalog sync, cluster activate) via
    // the alias, which the canonical-prefix checks would not catch.
    let canonical: std::borrow::Cow<'_, str> = match path.strip_prefix("/api/v1/veyron/") {
        Some(rest) => std::borrow::Cow::Owned(format!("/api/v1/{rest}")),
        None => std::borrow::Cow::Borrowed(path),
    };
    let path: &str = &canonical;

    if path.starts_with("/api/v1/clusters/") && path.ends_with("/activate") {
        return ApiRole::Admin;
    }
    if path.contains("/restore") {
        return ApiRole::Admin;
    }
    if path == "/api/v1/gitops/sync" || path == "/api/v1/catalog/sync" {
        return ApiRole::Admin;
    }
    if path.starts_with("/api/v1/tenants") {
        return ApiRole::Admin;
    }
    if path.starts_with("/api/v1/dr/") {
        return ApiRole::Admin;
    }
    // Node maintenance (cordon/uncordon/drain) affects cluster-wide scheduling.
    if path.starts_with("/api/v1/nodes/")
        && (path.ends_with("/cordon") || path.ends_with("/uncordon"))
    {
        return ApiRole::Admin;
    }
    // Destructive orphan-volume reclaim (DELETE deletes PVCs).
    if path == "/api/v1/storage/orphans" {
        return ApiRole::Admin;
    }
    if path.starts_with("/api/v1/soc/playbooks/") {
        return ApiRole::Admin;
    }
    // Publishing a golden image repoints the DataSource every future VM clones from —
    // a bad image silently propagates to the whole fleet's next builds.
    if path == "/api/v1/images/publish" {
        return ApiRole::Admin;
    }
    if path.starts_with("/api/v1/platform/") && path != "/api/v1/platform/guestkit/binary" {
        return ApiRole::Admin;
    }

    ApiRole::Write
}

pub fn is_ws_console_path(path: &str) -> bool {
    path.ends_with("/vnc") || path.ends_with("/serial")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_can_get_vms() {
        assert_eq!(
            min_role_for_route(&Method::GET, "/api/v1/vms"),
            ApiRole::ReadOnly
        );
    }

    #[test]
    fn write_can_create_vm() {
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/vms"),
            ApiRole::Write
        );
    }

    #[test]
    fn admin_required_for_cluster_activate() {
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/clusters/prod/activate"),
            ApiRole::Admin
        );
    }

    #[test]
    fn admin_required_for_snapshot_restore() {
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/snapshots/default/my-snap/restore"),
            ApiRole::Admin
        );
    }

    #[test]
    fn admin_required_for_golden_image_publish() {
        // Fleet-wide blast radius: repoints the DataSource every future clone uses.
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/images/publish"),
            ApiRole::Admin
        );
        // ...including via the product alias prefix.
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/veyron/images/publish"),
            ApiRole::Admin
        );
        // Uploading/importing an image is only Write — it publishes nothing.
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/images/upload"),
            ApiRole::Write
        );
    }

    #[test]
    fn admin_required_for_node_cordon() {
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/nodes/node01/cordon"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/nodes/node01/uncordon"),
            ApiRole::Admin
        );
    }

    #[test]
    fn write_required_for_hotplug_and_bulk() {
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/vms/default/web/hotplug"),
            ApiRole::Write
        );
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/vms/bulk"),
            ApiRole::Write
        );
    }

    #[test]
    fn admin_required_for_tenant_create() {
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/tenants"),
            ApiRole::Admin
        );
    }

    #[test]
    fn admin_gates_apply_through_veyron_alias() {
        // The /api/v1/veyron/* alias must not bypass admin-only gates.
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/veyron/tenants"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/veyron/dr/failover"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/veyron/gitops/sync"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/veyron/clusters/prod/activate"),
            ApiRole::Admin
        );
        // Non-admin alias routes still resolve to their normal role.
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/veyron/vms"),
            ApiRole::Write
        );
    }

    #[test]
    fn write_blocked_from_admin_route() {
        let ctx = AuthContext {
            role: ApiRole::Write,
        };
        assert!(!ctx.allows(&ApiRole::Admin));
        assert!(ctx.allows(&ApiRole::Write));
    }

    #[test]
    fn ws_console_paths_detected() {
        assert!(is_ws_console_path("/api/v1/vms/ns/vm/vnc"));
        assert!(is_ws_console_path("/api/v1/vms/ns/vm/serial"));
        assert!(!is_ws_console_path("/api/v1/vms/ns/vm/start"));
    }
}
