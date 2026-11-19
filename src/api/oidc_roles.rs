// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Per-namespace OIDC group -> role mapping (`VEYRON_OIDC_ROLE_MAP`).
//!
//! ```json
//! {
//!   "*":      { "platform-admins": "admin", "auditors": "readonly" },
//!   "team-a": { "team-a-devs": "write", "team-a-viewers": "readonly" }
//! }
//! ```
//!
//! Keys are namespaces (or `*`, which applies everywhere and is the floor for every namespace);
//! values map an IdP group (case-insensitive, exact match) to `admin`, `write` or `readonly`.
//!
//! Deny by default: a caller whose groups match nothing gets no role at all (the legacy
//! behaviour of falling back to `readonly` applies only when this variable is unset). An invalid
//! map is treated as a configuration error that denies every OIDC caller rather than being
//! half-applied. A namespace-specific grant only applies to routes that name the namespace in
//! the path (see `key_store::path_namespace`); cluster-wide routes need a `*` grant.

use std::collections::BTreeMap;

pub const ENV_ROLE_MAP: &str = "VEYRON_OIDC_ROLE_MAP";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    ReadOnly,
    Write,
    Admin,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s.trim().to_ascii_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "write" => Some(Role::Write),
            "readonly" | "read" => Some(Role::ReadOnly),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Write => "write",
            Role::ReadOnly => "readonly",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleMap {
    /// namespace (or `*`) -> lowercase group -> role
    grants: BTreeMap<String, BTreeMap<String, Role>>,
}

impl RoleMap {
    pub fn parse(raw: &str) -> Result<RoleMap, String> {
        let v: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_str(raw)
            .map_err(|e| format!("{ENV_ROLE_MAP} must be a JSON object of objects: {e}"))?;
        let mut grants = BTreeMap::new();
        for (ns, groups) in v {
            if !crate::api::key_store::valid_scope(&ns) {
                return Err(format!(
                    "{ENV_ROLE_MAP}: '{ns}' is not a namespace name or *"
                ));
            }
            let mut out: BTreeMap<String, Role> = BTreeMap::new();
            for (group, role) in groups {
                let group = group.trim().to_lowercase();
                if group.is_empty() {
                    return Err(format!("{ENV_ROLE_MAP}: empty group name under '{ns}'"));
                }
                let role = Role::parse(&role).ok_or_else(|| {
                    format!("{ENV_ROLE_MAP}: unknown role '{role}' (admin, write, readonly)")
                })?;
                let e = out.entry(group).or_insert(role);
                *e = (*e).max(role);
            }
            grants.insert(ns, out);
        }
        Ok(RoleMap { grants })
    }

    /// Highest role the groups earn under `*` and, when `namespace` is given, that namespace.
    /// `None` means no grant: deny.
    pub fn role_for(&self, groups: &[&str], namespace: Option<&str>) -> Option<Role> {
        let lower: Vec<String> = groups.iter().map(|g| g.trim().to_lowercase()).collect();
        let mut best: Option<Role> = None;
        let scopes = std::iter::once("*").chain(namespace.filter(|n| *n != "*"));
        for scope in scopes {
            if let Some(map) = self.grants.get(scope) {
                for g in &lower {
                    if let Some(r) = map.get(g) {
                        best = best.max(Some(*r));
                    }
                }
            }
        }
        best
    }
}

/// Outcome of reading the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Config {
    /// Variable unset: legacy mapping applies.
    Unset,
    Map(RoleMap),
    /// Variable set but unusable: every OIDC caller is denied.
    Invalid(String),
}

pub fn parse_config(raw: Option<&str>) -> Config {
    match raw.map(str::trim).filter(|v| !v.is_empty()) {
        None => Config::Unset,
        Some(r) => match RoleMap::parse(r) {
            Ok(m) => Config::Map(m),
            Err(e) => Config::Invalid(e),
        },
    }
}

/// Role for a caller under the configured policy. `legacy` is the role from the exact-match
/// `VEYRON_OIDC_GROUP_ADMIN` / `_WRITE` lists (never the substring or readonly fallbacks); it
/// counts as a `*` grant so existing admin groups keep working once a map is added.
/// Returns `None` (deny) when the map is invalid or nothing matches.
pub fn effective_role(
    config: &Config,
    legacy: Option<Role>,
    groups: &[&str],
    namespace: Option<&str>,
) -> Option<Role> {
    match config {
        Config::Unset => legacy,
        Config::Invalid(_) => None,
        Config::Map(m) => m.role_for(groups, namespace).max(legacy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = r#"{
        "*": {"Platform-Admins": "admin", "auditors": "readonly"},
        "team-a": {"team-a-devs": "write", "team-a-viewers": "readonly", "team-a-owners": "admin"},
        "team-b": {"team-b-devs": "write"}
    }"#;

    fn cfg() -> Config {
        parse_config(Some(MAP))
    }

    fn role(groups: &[&str], ns: Option<&str>) -> Option<&'static str> {
        effective_role(&cfg(), None, groups, ns).map(Role::as_str)
    }

    #[test]
    fn matrix() {
        // (groups, namespace, expected)
        let cases: &[(&[&str], Option<&str>, Option<&str>)] = &[
            // Deny by default: no groups, unknown groups, near-misses.
            (&[], Some("team-a"), None),
            (&["random"], Some("team-a"), None),
            (&["team-a-dev"], Some("team-a"), None),
            (&["team-a-devs-extra"], Some("team-a"), None),
            // Namespace grants apply only to their namespace.
            (&["team-a-devs"], Some("team-a"), Some("write")),
            (&["team-a-devs"], Some("team-b"), None),
            (&["team-a-devs"], Some("kube-system"), None),
            (&["team-a-devs"], None, None),
            (&["team-a-owners"], Some("team-a"), Some("admin")),
            // Highest wins across groups.
            (
                &["team-a-viewers", "team-a-devs"],
                Some("team-a"),
                Some("write"),
            ),
            (
                &["team-a-devs", "team-b-devs"],
                Some("team-b"),
                Some("write"),
            ),
            // `*` applies everywhere, including no namespace, and is a floor.
            (&["platform-admins"], None, Some("admin")),
            (&["platform-admins"], Some("team-a"), Some("admin")),
            (&["auditors"], None, Some("readonly")),
            (&["auditors", "team-a-devs"], Some("team-a"), Some("write")),
            (
                &["auditors", "team-a-devs"],
                Some("team-b"),
                Some("readonly"),
            ),
            // Group names are case-insensitive.
            (&["PLATFORM-ADMINS"], None, Some("admin")),
            (&["Team-A-Devs"], Some("team-a"), Some("write")),
            // A literal "*" namespace argument does not widen anything.
            (&["team-a-devs"], Some("*"), None),
        ];
        for (groups, ns, want) in cases {
            assert_eq!(role(groups, *ns), *want, "groups={groups:?} ns={ns:?}");
        }
    }

    #[test]
    fn unset_keeps_legacy_behaviour() {
        let c = parse_config(None);
        assert_eq!(c, Config::Unset);
        assert_eq!(parse_config(Some("  ")), Config::Unset);
        assert_eq!(
            effective_role(&c, Some(Role::Admin), &[], None),
            Some(Role::Admin)
        );
        assert_eq!(effective_role(&c, None, &["x"], Some("a")), None);
    }

    #[test]
    fn legacy_exact_groups_count_as_global_grants() {
        let c = cfg();
        assert_eq!(
            effective_role(&c, Some(Role::Admin), &["nobody"], Some("team-a")),
            Some(Role::Admin)
        );
        assert_eq!(
            effective_role(&c, Some(Role::Write), &["team-a-owners"], Some("team-a")),
            Some(Role::Admin)
        );
    }

    #[test]
    fn invalid_config_denies_everyone() {
        for bad in [
            "not json",
            "[]",
            r#"{"team-a": "admin"}"#,
            r#"{"team-a": {"g": "root"}}"#,
            r#"{"Team A": {"g": "admin"}}"#,
            r#"{"team-a": {"": "admin"}}"#,
            r#"{"-bad": {"g": "admin"}}"#,
        ] {
            let c = parse_config(Some(bad));
            assert!(matches!(c, Config::Invalid(_)), "{bad}");
            assert_eq!(effective_role(&c, Some(Role::Admin), &["g"], None), None);
        }
    }

    #[test]
    fn duplicate_groups_after_lowercasing_keep_highest() {
        let m = RoleMap::parse(r#"{"*": {"Ops": "readonly", "ops": "write"}}"#).unwrap();
        assert_eq!(m.role_for(&["OPS"], None), Some(Role::Write));
    }

    #[test]
    fn role_parse() {
        assert_eq!(Role::parse(" Admin "), Some(Role::Admin));
        assert_eq!(Role::parse("read"), Some(Role::ReadOnly));
        assert_eq!(Role::parse("owner"), None);
        assert!(Role::Admin > Role::Write && Role::Write > Role::ReadOnly);
    }
}
