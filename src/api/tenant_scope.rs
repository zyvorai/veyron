// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//! Opt-in, fail-closed tenant authorization. Unknown routes are denied for scoped callers.
use crate::api::{auth_context::AuthContext, http_server::web::ApiRole};
use crate::enterprise::planning::dns_label;
use axum::{
    Json,
    extract::{Query, Request},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, BTreeSet};

type Policy = BTreeMap<String, BTreeSet<String>>;
static POLICY: Lazy<Result<Option<Policy>, String>> =
    Lazy::new(|| parse_policy(std::env::var("VEYRON_TENANT_SCOPES").ok().as_deref()));
fn parse_policy(raw: Option<&str>) -> Result<Option<Policy>, String> {
    let Some(raw) = raw else { return Ok(None) };
    let p: Policy = serde_json::from_str(raw).map_err(|_| "invalid VEYRON_TENANT_SCOPES JSON")?;
    if p.iter().any(|(subject, ns)| {
        subject.trim().is_empty() || ns.is_empty() || ns.iter().any(|n| !dns_label(n) || n == "all")
    }) {
        return Err("tenant policy needs subjects and explicit DNS namespaces".into());
    }
    Ok(Some(p))
}
pub fn namespaces(ctx: &AuthContext) -> Result<Option<&'static BTreeSet<String>>, String> {
    match &*POLICY {
        Err(e) => Err(e.clone()),
        Ok(None) => Ok(None),
        Ok(Some(p)) => match p.get(&ctx.subject) {
            Some(ns) => Ok(Some(ns)),
            None if ctx.role == ApiRole::Admin => Ok(None),
            None => Err("caller has no tenant namespace assignment".into()),
        },
    }
}
pub fn authorize_namespace(ctx: &AuthContext, ns: &str) -> Result<(), String> {
    if !dns_label(ns) || ns == "all" {
        return Err("one explicit namespace is required".into());
    }
    if let Some(allowed) = namespaces(ctx)?
        && !allowed.contains(ns)
    {
        return Err("namespace outside caller scope".into());
    }
    Ok(())
}
fn canonical(path: &str) -> String {
    path.strip_prefix("/api/v1/veyron/")
        .map(|p| format!("/api/v1/{p}"))
        .unwrap_or_else(|| path.to_string())
}
fn route_allowed(
    method: &Method,
    path: &str,
    ns: Option<&str>,
    allowed: &BTreeSet<String>,
) -> bool {
    let path = canonical(path);
    let parts = path.trim_matches('/').split('/').collect::<Vec<_>>();
    if path == "/api/v1/enterprise/capabilities" && *method == Method::GET {
        return true;
    }
    if path == "/api/v1/enterprise/blueprints/validate" && *method == Method::POST {
        return true;
    }
    if path == "/api/v1/enterprise/operations"
        || (parts.len() == 5 && parts[3] == "operations")
        || (parts.len() == 6 && parts[3] == "operations" && parts[5] == "cancel")
    {
        return path.starts_with("/api/v1/enterprise/")
            && matches!(*method, Method::GET | Method::POST)
            && ns.is_some_and(|n| allowed.contains(n));
    }
    if path == "/api/v1/vms" && *method == Method::GET {
        return ns.is_some_and(|n| allowed.contains(n));
    }
    if parts.len() >= 5 && parts[..3] == ["api", "v1", "vms"] && allowed.contains(parts[3]) {
        return matches!(
            (method.as_str(), parts.get(5).copied(), parts.len()),
            ("GET", None, 5) | ("POST", Some("start" | "stop"), 6)
        );
    }
    false
}
pub async fn middleware(request: Request, next: Next) -> Response {
    let Some(ctx) = request.extensions().get::<AuthContext>() else {
        return next.run(request).await;
    };
    let allowed = match namespaces(ctx) {
        Ok(None) => return next.run(request).await,
        Ok(Some(ns)) => ns,
        Err(e) => {
            return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":e}))).into_response();
        }
    };
    let query = Query::<BTreeMap<String, String>>::try_from_uri(request.uri());
    let ns = query
        .as_ref()
        .ok()
        .and_then(|q| q.0.get("namespace"))
        .map(String::as_str);
    if !route_allowed(request.method(), request.uri().path(), ns, allowed) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error":"route not available to namespace-scoped callers"})),
        )
            .into_response();
    }
    next.run(request).await
}
#[cfg(test)]
mod tests {
    use super::*;
    fn scopes() -> BTreeSet<String> {
        BTreeSet::from(["tenant-a".into()])
    }
    #[test]
    fn fail_closed_policy() {
        assert!(parse_policy(Some("broken")).is_err());
        assert!(parse_policy(Some(r#"{"user":["all"]}"#)).is_err());
        assert!(parse_policy(Some(r#"{"user":[]}"#)).is_err());
    }
    #[test]
    fn routes_cannot_escape_namespace() {
        let ns = scopes();
        assert!(route_allowed(
            &Method::GET,
            "/api/v1/vms/tenant-a/web",
            None,
            &ns
        ));
        assert!(!route_allowed(
            &Method::GET,
            "/api/v1/vms/tenant-b/web",
            None,
            &ns
        ));
        assert!(!route_allowed(
            &Method::GET,
            "/api/v1/vms",
            Some("all"),
            &ns
        ));
        assert!(!route_allowed(
            &Method::POST,
            "/api/v1/vms/tenant-a/web/guest/exec",
            None,
            &ns
        ));
    }
    #[test]
    fn aliases_follow_same_scope() {
        assert!(route_allowed(
            &Method::POST,
            "/api/v1/veyron/vms/tenant-a/web/start",
            None,
            &scopes()
        ));
        assert!(!route_allowed(
            &Method::POST,
            "/api/v1/veyron/dr/failover",
            None,
            &scopes()
        ));
    }
    #[test]
    fn operation_reads_require_explicit_scope() {
        assert!(route_allowed(
            &Method::GET,
            "/api/v1/enterprise/operations",
            Some("tenant-a"),
            &scopes()
        ));
        assert!(!route_allowed(
            &Method::GET,
            "/api/v1/enterprise/operations",
            None,
            &scopes()
        ));
    }
}
