// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Application-wide configuration file
// Loaded from ~/.config/veyron/config.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Global application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Default Kubernetes namespace
    pub namespace: String,

    /// Path to kubeconfig file (overrides KUBECONFIG env)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubeconfig: Option<String>,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// API server configuration
    pub api: ApiServerConfig,

    /// Output defaults
    pub output: OutputConfig,

    /// TUI preferences
    pub tui: TuiPreferences,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            kubeconfig: None,
            logging: LoggingConfig::default(),
            api: ApiServerConfig::default(),
            output: OutputConfig::default(),
            tui: TuiPreferences::default(),
        }
    }
}

impl AppConfig {
    /// System-wide config path
    pub fn system_path() -> PathBuf {
        PathBuf::from("/etc/veyron/config.toml")
    }

    /// User config path (~/.config/veyron/config.toml)
    pub fn user_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Ok(config_dir.join("veyron").join("config.toml"))
    }

    /// Alias for user_path (backwards compat)
    pub fn default_path() -> Result<PathBuf> {
        Self::user_path()
    }

    /// Load configuration with layered resolution:
    /// 1. Start with compiled defaults
    /// 2. Load /etc/veyron/config.toml (system-wide)
    /// 3. Overlay ~/.config/veyron/config.toml (user)
    pub fn load() -> Result<Self> {
        // Start from defaults
        let mut config = Self::default();

        // Layer 1: system-wide config
        let system_path = Self::system_path();
        if system_path.exists() {
            let system_config = Self::load_from(system_path)?;
            config.merge(system_config);
        }

        // Layer 2: user config overrides system
        let user_path = Self::user_path()?;
        if user_path.exists() {
            let user_config = Self::load_from(user_path)?;
            config.merge(user_config);
        }

        Ok(config)
    }

    /// Load configuration from a specific file
    pub fn load_from(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))
    }

    /// Save configuration to user location (~/.config/veyron/config.toml)
    pub fn save(&self) -> Result<()> {
        let path = Self::user_path()?;
        self.save_to(&path)
    }

    /// Save configuration to a specific file
    pub fn save_to(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        // Write to a temp file first, then rename for atomicity.
        // On Unix, create with restricted permissions (0o600) BEFORE writing
        // content to avoid a race where the file is world-readable.
        let tmp_path = path.with_extension("tmp");

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp_path)
                .with_context(|| format!("Failed to create config file: {}", tmp_path.display()))?;
            f.write_all(content.as_bytes())
                .with_context(|| format!("Failed to write config file: {}", tmp_path.display()))?;
            f.flush()?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(&tmp_path, &content)
                .with_context(|| format!("Failed to write config file: {}", tmp_path.display()))?;
        }

        std::fs::rename(&tmp_path, path)
            .with_context(|| format!("Failed to finalize config file: {}", path.display()))?;

        Ok(())
    }

    /// Merge another config into this one (non-default values override)
    fn merge(&mut self, other: Self) {
        let defaults = Self::default();

        if other.namespace != defaults.namespace {
            self.namespace = other.namespace;
        }
        if other.kubeconfig.is_some() {
            self.kubeconfig = other.kubeconfig;
        }

        // Logging
        if other.logging.level != defaults.logging.level {
            self.logging.level = other.logging.level;
        }
        if other.logging.format != defaults.logging.format {
            self.logging.format = other.logging.format;
        }
        if other.logging.file.is_some() {
            self.logging.file = other.logging.file;
        }

        // API
        if other.api.port != defaults.api.port {
            self.api.port = other.api.port;
        }
        if other.api.host != defaults.api.host {
            self.api.host = other.api.host;
        }
        if other.api.tls != defaults.api.tls {
            self.api.tls = other.api.tls;
        }
        if other.api.tls_cert.is_some() {
            self.api.tls_cert = other.api.tls_cert;
        }
        if other.api.tls_key.is_some() {
            self.api.tls_key = other.api.tls_key;
        }
        if other.api.auth != defaults.api.auth {
            self.api.auth = other.api.auth;
        }
        if other.api.rate_limit != defaults.api.rate_limit {
            self.api.rate_limit = other.api.rate_limit;
        }
        if other.api.cors != defaults.api.cors {
            self.api.cors = other.api.cors;
        }
        if other.api.cors_origins != defaults.api.cors_origins {
            self.api.cors_origins = other.api.cors_origins;
        }
        if other.api.request_timeout != defaults.api.request_timeout {
            self.api.request_timeout = other.api.request_timeout;
        }

        // Output
        if other.output.format != defaults.output.format {
            self.output.format = other.output.format;
        }
        if other.output.color != defaults.output.color {
            self.output.color = other.output.color;
        }
        if other.output.timestamps != defaults.output.timestamps {
            self.output.timestamps = other.output.timestamps;
        }

        // TUI
        if other.tui.refresh_interval != defaults.tui.refresh_interval {
            self.tui.refresh_interval = other.tui.refresh_interval;
        }
        if other.tui.interactive != defaults.tui.interactive {
            self.tui.interactive = other.tui.interactive;
        }
        if other.tui.splash != defaults.tui.splash {
            self.tui.splash = other.tui.splash;
        }
    }

    /// Generate default configuration file content
    pub fn default_config_string() -> Result<String> {
        let config = Self::default();
        toml::to_string_pretty(&config).context("Failed to serialize default config")
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level: error, warn, info, debug, trace
    pub level: String,

    /// Log format: text, json
    pub format: String,

    /// Log to file instead of stderr
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
            file: None,
        }
    }
}

/// API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiServerConfig {
    /// Port to listen on
    pub port: u16,

    /// Host/address to bind to
    pub host: String,

    /// Enable TLS
    pub tls: bool,

    /// Path to TLS certificate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_cert: Option<String>,

    /// Path to TLS key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_key: Option<String>,

    /// Authentication method: none, api-key, bearer, basic, oauth2, mtls
    pub auth: String,

    /// Rate limit (requests per minute, 0 to disable)
    pub rate_limit: u32,

    /// CORS enabled
    pub cors: bool,

    /// Allowed CORS origins
    #[serde(default)]
    pub cors_origins: Vec<String>,

    /// Request timeout in seconds
    pub request_timeout: u64,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            tls: false,
            tls_cert: None,
            tls_key: None,
            auth: "none".to_string(),
            rate_limit: 60,
            cors: true,
            cors_origins: Vec::new(),
            request_timeout: 30,
        }
    }
}

/// Output preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Default output format: table, yaml, json
    pub format: String,

    /// Use colored output
    pub color: bool,

    /// Show timestamps in output
    pub timestamps: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: "table".to_string(),
            color: true,
            timestamps: false,
        }
    }
}

/// TUI preferences (complements tui.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiPreferences {
    /// Auto-refresh interval in seconds
    pub refresh_interval: u64,

    /// Start in interactive mode by default
    pub interactive: bool,

    /// Show splash screen on startup
    pub splash: bool,
}

impl Default for TuiPreferences {
    fn default() -> Self {
        Self {
            refresh_interval: 5,
            interactive: false,
            splash: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.namespace, "default");
        assert_eq!(config.api.port, 8080);
        assert_eq!(config.api.host, "127.0.0.1");
        assert!(!config.api.tls);
        assert_eq!(config.api.auth, "none");
        assert_eq!(config.api.rate_limit, 60);
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.output.format, "table");
        assert!(config.output.color);
        assert_eq!(config.tui.refresh_interval, 5);
    }

    #[test]
    fn test_serialize_default_config() {
        let content = AppConfig::default_config_string().unwrap();
        assert!(content.contains("namespace"));
        assert!(content.contains("port"));
        assert!(content.contains("8080"));
        assert!(content.contains("[api]"));
        assert!(content.contains("[logging]"));
        assert!(content.contains("[output]"));
        assert!(content.contains("[tui]"));
    }

    #[test]
    fn test_deserialize_minimal_config() {
        let toml = r#"
            namespace = "production"

            [api]
            port = 9090
        "#;

        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.namespace, "production");
        assert_eq!(config.api.port, 9090);
        // Defaults should be preserved for unspecified fields
        assert_eq!(config.api.host, "127.0.0.1");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_deserialize_full_config() {
        let toml = r#"
            namespace = "staging"
            kubeconfig = "/home/user/.kube/staging"

            [logging]
            level = "debug"
            format = "json"
            file = "/var/log/veyron.log"

            [api]
            port = 443
            host = "127.0.0.1"
            tls = true
            tls_cert = "/etc/ssl/cert.pem"
            tls_key = "/etc/ssl/key.pem"
            auth = "bearer"
            rate_limit = 120
            cors = true
            cors_origins = ["https://dashboard.example.com"]
            request_timeout = 60

            [output]
            format = "json"
            color = false
            timestamps = true

            [tui]
            refresh_interval = 10
            interactive = true
            splash = false
        "#;

        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.namespace, "staging");
        assert_eq!(
            config.kubeconfig,
            Some("/home/user/.kube/staging".to_string())
        );
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.logging.format, "json");
        assert_eq!(config.logging.file, Some("/var/log/veyron.log".to_string()));
        assert_eq!(config.api.port, 443);
        assert_eq!(config.api.host, "127.0.0.1");
        assert!(config.api.tls);
        assert_eq!(config.api.auth, "bearer");
        assert_eq!(config.api.rate_limit, 120);
        assert_eq!(
            config.api.cors_origins,
            vec!["https://dashboard.example.com".to_string()]
        );
        assert_eq!(config.output.format, "json");
        assert!(!config.output.color);
        assert!(config.output.timestamps);
        assert_eq!(config.tui.refresh_interval, 10);
        assert!(config.tui.interactive);
        assert!(!config.tui.splash);
    }

    #[test]
    fn test_roundtrip_serialize_deserialize() {
        let config = AppConfig {
            namespace: "my-ns".to_string(),
            kubeconfig: Some("/path/to/kubeconfig".to_string()),
            api: ApiServerConfig {
                port: 9999,
                ..Default::default()
            },
            ..Default::default()
        };

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();

        assert_eq!(config.namespace, deserialized.namespace);
        assert_eq!(config.kubeconfig, deserialized.kubeconfig);
        assert_eq!(config.api.port, deserialized.api.port);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = PathBuf::from("/tmp/nonexistent-veyron-test-config.toml");
        let config = AppConfig::load_from(path).unwrap();
        assert_eq!(config.namespace, "default");
        assert_eq!(config.api.port, 8080);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = AppConfig {
            namespace: "test-ns".to_string(),
            api: ApiServerConfig {
                port: 3000,
                host: "localhost".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        config.save_to(&path).unwrap();
        assert!(path.exists());

        let loaded = AppConfig::load_from(path).unwrap();
        assert_eq!(loaded.namespace, "test-ns");
        assert_eq!(loaded.api.port, 3000);
        assert_eq!(loaded.api.host, "localhost");
    }

    #[test]
    fn test_merge_overrides_non_defaults() {
        let mut base = AppConfig::default();

        let overlay = AppConfig {
            namespace: "production".to_string(),
            api: ApiServerConfig {
                port: 443,
                auth: "bearer".to_string(),
                ..Default::default()
            },
            logging: LoggingConfig {
                level: "debug".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        base.merge(overlay);

        assert_eq!(base.namespace, "production");
        assert_eq!(base.api.port, 443);
        assert_eq!(base.api.auth, "bearer");
        assert_eq!(base.api.host, "127.0.0.1"); // kept from default
        assert_eq!(base.logging.level, "debug");
    }

    #[test]
    fn test_merge_preserves_base_when_overlay_is_default() {
        let mut base = AppConfig {
            namespace: "staging".to_string(),
            api: ApiServerConfig {
                port: 9090,
                ..Default::default()
            },
            ..Default::default()
        };

        let overlay = AppConfig::default();
        base.merge(overlay);

        // Base values should be preserved since overlay is all defaults
        assert_eq!(base.namespace, "staging");
        assert_eq!(base.api.port, 9090);
    }

    #[test]
    fn test_system_path() {
        assert_eq!(
            AppConfig::system_path(),
            PathBuf::from("/etc/veyron/config.toml")
        );
    }
}
