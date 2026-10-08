// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Request auth context and route-level RBAC rules.

use axum::http::Method;

use crate::api::http_server::web::ApiRole;

/// Authenticated caller attached to each authorized request.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthContext {
    pub role: ApiRole,
    /// Who is calling: API key name, local username, JWT/OIDC `sub`, or `ws-ticket`.
    pub subject: String,
}

impl AuthContext {
    pub fn allows(&self, required: &ApiRole) -> bool {
        role_rank(&self.role) >= role_rank(required)
    }
}

pub fn role_rank(role: &ApiRole) -> u8 {
    match role {
        ApiRole::ReadOnly => 0,
        ApiRole::Write => 1,
        ApiRole::Admin => 2,
    }
}

/// Minimum role required for a route. Safe methods always require ReadOnly.
pub fn min_role_for_route(method: &Method, path: &str) -> ApiRole {
    // User-account management (list/create/delete/reset password) is Admin-only
    // even for GET — usernames+roles are sensitive and this must NOT fall
    // through to the blanket "safe methods are ReadOnly" rule below.
    // `/api/v1/auth/login` itself is unauthenticated (see dashboard_paths) and
    // never reaches this check.
    if path.starts_with("/api/v1/auth/users") || path.starts_with("/api/v1/veyron/auth/users") {
        return ApiRole::Admin;
    }
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

    if path.starts_with("/api/v1/enterprise/assess/")
        || path == "/api/v1/enterprise/placement"
        || path == "/api/v1/enterprise/blueprints/validate"
    {
        return ApiRole::ReadOnly;
    }

    // AI requests that only read or draft. Anything they want to change becomes a
    // proposal that a Write/Admin caller must approve, and MCP tools check roles per tool.
    if matches!(
        path,
        "/api/v1/ai/chat/stream"
            | "/api/v1/ai/chat"
            | "/api/v1/ai/search"
            | "/api/v1/ai/intent/vm"
            | "/api/v1/ai/policies/draft"
            | "/mcp"
    ) {
        return ApiRole::ReadOnly;
    }
    if path == "/api/v1/ai/settings/llm" || path.starts_with("/api/v1/ai/mcp-servers") {
        return ApiRole::Admin;
    }

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
    // Node maintenance (cordon/uncordon/drain/reboot) affects cluster-wide scheduling.
    if path.starts_with("/api/v1/nodes/")
        && (path.ends_with("/cordon") || path.ends_with("/uncordon") || path.ends_with("/reboot"))
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
    // Kairon's MachineImage catalog is cluster-scoped and shared by every
    // namespace; removing an upload can break images that still point at it.
    if path.starts_with("/api/v1/machine-images")
        || (method == Method::DELETE && path.starts_with("/api/v1/image-store/"))
        || (path.starts_with("/api/v1/vms/") && path.ends_with("/capture"))
    {
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
    fn enterprise_assessments_are_readonly_but_power_is_write() {
        for prefix in ["/api/v1", "/api/v1/veyron"] {
            assert_eq!(
                min_role_for_route(
                    &Method::POST,
                    &format!("{prefix}/enterprise/assess/recovery")
                ),
                ApiRole::ReadOnly
            );
            assert_eq!(
                min_role_for_route(&Method::POST, &format!("{prefix}/enterprise/operations")),
                ApiRole::Write
            );
        }
    }

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
    fn kairon_image_catalog_roles() {
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/machine-images"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::DELETE, "/api/v1/veyron/machine-images/win"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::DELETE, "/api/v1/image-store/win.iso"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::PUT, "/api/v1/image-store/win.iso"),
            ApiRole::Write
        );
        assert_eq!(
            min_role_for_route(&Method::GET, "/api/v1/machine-images"),
            ApiRole::ReadOnly
        );
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/vms/default/web/capture"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::GET, "/api/v1/image-captures"),
            ApiRole::ReadOnly
        );
        assert_eq!(
            min_role_for_route(
                &Method::POST,
                "/api/v1/vms/default/web/cdroms/install/eject"
            ),
            ApiRole::Write
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
    fn admin_required_for_user_management_including_get() {
        assert_eq!(
            min_role_for_route(&Method::GET, "/api/v1/auth/users"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/auth/users"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::DELETE, "/api/v1/auth/users/alice"),
            ApiRole::Admin
        );
        assert_eq!(
            min_role_for_route(&Method::POST, "/api/v1/veyron/auth/users"),
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
            subject: "ci".into(),
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
