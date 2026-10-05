// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use serde::{Deserialize, Serialize};

/// PAM authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PamConfig {
    /// PAM service name (e.g., "veyron", "login")
    pub service_name: String,
    /// Whether PAM auth is enabled
    pub enabled: bool,
    /// Allowed PAM groups (empty = all groups allowed)
    pub allowed_groups: Vec<String>,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
}

impl PamConfig {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            enabled: false,
            allowed_groups: Vec::new(),
            session_timeout_secs: 3600,
        }
    }

    pub fn with_groups(mut self, groups: Vec<String>) -> Self {
        self.allowed_groups = groups;
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.session_timeout_secs = secs;
        self
    }

    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }
}

impl Default for PamConfig {
    fn default() -> Self {
        Self::new("veyron")
    }
}

/// PAM authentication result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PamAuthResult {
    pub authenticated: bool,
    pub username: String,
    pub groups: Vec<String>,
    pub error: Option<String>,
}

impl PamAuthResult {
    pub fn success(username: impl Into<String>, groups: Vec<String>) -> Self {
        Self {
            authenticated: true,
            username: username.into(),
            groups,
            error: None,
        }
    }

    pub fn failure(username: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            authenticated: false,
            username: username.into(),
            groups: Vec::new(),
            error: Some(error.into()),
        }
    }
}

/// Validate username format for PAM authentication.
pub fn validate_username(username: &str) -> bool {
    // PAM typically limits usernames to 32 characters (LOGIN_NAME_MAX)
    if username.is_empty() || username.len() > 32 {
        return false;
    }
    // Disallow leading dot, all-dots, and path traversal patterns
    if username.starts_with('.') || username.contains("..") {
        return false;
    }
    // Allow alphanumeric, dash, underscore, dot
    username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Check whether a user belongs to any of the allowed groups.
pub fn check_group_membership(user_groups: &[String], allowed_groups: &[String]) -> bool {
    if allowed_groups.is_empty() {
        return true; // no group restriction
    }
    user_groups.iter().any(|g| allowed_groups.contains(g))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pam_config_default() {
        let config = PamConfig::default();
        assert_eq!(config.service_name, "veyron");
        assert!(!config.enabled);
    }

    #[test]
    fn test_validate_username() {
        assert!(validate_username("admin"));
        assert!(validate_username("user.name"));
        assert!(validate_username("user-name_1"));
        assert!(!validate_username(""));
        assert!(!validate_username("user name")); // space not allowed
        assert!(!validate_username(".hidden")); // leading dot
        assert!(!validate_username("user..name")); // path traversal
        assert!(!validate_username("..")); // directory traversal
    }

    #[test]
    fn test_check_group_membership() {
        let user_groups = vec!["wheel".to_string(), "users".to_string()];
        let allowed = vec!["wheel".to_string(), "admin".to_string()];
        assert!(check_group_membership(&user_groups, &allowed));

        let no_match = vec!["docker".to_string()];
        assert!(!check_group_membership(&user_groups, &no_match));

        // Empty allowed = all pass
        assert!(check_group_membership(&user_groups, &[]));
    }

    #[test]
    fn test_pam_auth_result() {
        let success = PamAuthResult::success("admin", vec!["wheel".to_string()]);
        assert!(success.authenticated);

        let failure = PamAuthResult::failure("admin", "bad password");
        assert!(!failure.authenticated);
        assert!(failure.error.is_some());
    }
}
