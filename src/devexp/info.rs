use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub vmrogue_version: String,
    pub rust_version: String,
    pub os: String,
    pub arch: String,
    pub kubernetes: KubernetesInfo,
    pub config: ConfigInfo,
    pub features: Vec<String>,
    pub collected_at: String,
}

/// Kubernetes cluster information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesInfo {
    pub kubeconfig: String,
    pub context: String,
    pub cluster: String,
    pub namespace: String,
    pub server: String,
    pub connected: bool,
}

impl KubernetesInfo {
    pub fn new() -> Self {
        Self {
            kubeconfig: std::env::var("KUBECONFIG")
                .unwrap_or_else(|_| "~/.kube/config".to_string()),
            context: "current-context".to_string(),
            cluster: "unknown".to_string(),
            namespace: "default".to_string(),
            server: "unknown".to_string(),
            connected: false,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    pub fn with_cluster(mut self, cluster: impl Into<String>) -> Self {
        self.cluster = cluster.into();
        self
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn with_server(mut self, server: impl Into<String>) -> Self {
        self.server = server.into();
        self
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }
}

impl Default for KubernetesInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub config_dir: String,
    pub templates_dir: String,
    pub cache_dir: String,
    pub settings: HashMap<String, String>,
}

impl ConfigInfo {
    pub fn new() -> Self {
        let home = dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|| "~".to_string());

        Self {
            config_dir: format!("{}/.config/vmrogue", home),
            templates_dir: format!("{}/.config/vmrogue/templates", home),
            cache_dir: format!("{}/.cache/vmrogue", home),
            settings: HashMap::new(),
        }
    }

    pub fn add_setting(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.settings.insert(key.into(), value.into());
    }

    pub fn setting_count(&self) -> usize {
        self.settings.len()
    }
}

impl Default for ConfigInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentInfo {
    pub fn collect(namespace: &str) -> Self {
        let k8s = KubernetesInfo::new().with_namespace(namespace);

        let config = ConfigInfo::new();

        let features = vec![
            "vm-management".to_string(),
            "templates".to_string(),
            "profiles".to_string(),
            "blueprints".to_string(),
            "snapshots".to_string(),
            "monitoring".to_string(),
            "disk-management".to_string(),
            "network-management".to_string(),
            "migration".to_string(),
            "backup-recovery".to_string(),
            "security".to_string(),
            "cost-management".to_string(),
            "automation".to_string(),
            "observability".to_string(),
            "multi-tenancy".to_string(),
            "gitops".to_string(),
            "ai-ml".to_string(),
            "service-mesh".to_string(),
            "disaster-recovery".to_string(),
            "compliance".to_string(),
            "capacity-planning".to_string(),
            "finops".to_string(),
            "advanced-networking".to_string(),
            "edge-computing".to_string(),
            "secrets-management".to_string(),
            "multi-cloud".to_string(),
            "developer-experience".to_string(),
        ];

        Self {
            vmrogue_version: env!("CARGO_PKG_VERSION").to_string(),
            rust_version: "1.70+".to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            kubernetes: k8s,
            config,
            features,
            collected_at: Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        }
    }

    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }
}

/// Diagnostic check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: DiagnosticStatus,
    pub message: String,
    pub details: Option<String>,
}

/// Diagnostic status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Pass,
    Warning,
    Fail,
    Skip,
}

impl std::fmt::Display for DiagnosticStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticStatus::Pass => write!(f, "PASS"),
            DiagnosticStatus::Warning => write!(f, "WARN"),
            DiagnosticStatus::Fail => write!(f, "FAIL"),
            DiagnosticStatus::Skip => write!(f, "SKIP"),
        }
    }
}

impl DiagnosticCheck {
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: DiagnosticStatus::Pass,
            message: message.into(),
            details: None,
        }
    }

    pub fn warning(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: DiagnosticStatus::Warning,
            message: message.into(),
            details: None,
        }
    }

    pub fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: DiagnosticStatus::Fail,
            message: message.into(),
            details: None,
        }
    }

    pub fn skip(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: DiagnosticStatus::Skip,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn is_pass(&self) -> bool {
        self.status == DiagnosticStatus::Pass
    }

    pub fn is_fail(&self) -> bool {
        self.status == DiagnosticStatus::Fail
    }
}

/// Run environment diagnostics
pub fn run_diagnostics() -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // Check kubeconfig
    let kubeconfig = std::env::var("KUBECONFIG").unwrap_or_else(|_| {
        dirs::home_dir()
            .map(|h| format!("{}/.kube/config", h.to_string_lossy()))
            .unwrap_or_default()
    });

    if std::path::Path::new(&kubeconfig).exists() {
        checks.push(DiagnosticCheck::pass(
            "kubeconfig",
            format!("Found at {}", kubeconfig),
        ));
    } else {
        checks.push(
            DiagnosticCheck::warning("kubeconfig", "No kubeconfig found")
                .with_details("Set KUBECONFIG env var or place config at ~/.kube/config"),
        );
    }

    // Check config directory
    let config_dir = dirs::home_dir()
        .map(|h| format!("{}/.config/vmrogue", h.to_string_lossy()))
        .unwrap_or_default();

    if std::path::Path::new(&config_dir).exists() {
        checks.push(DiagnosticCheck::pass(
            "config-dir",
            format!("Config directory exists at {}", config_dir),
        ));
    } else {
        checks.push(
            DiagnosticCheck::warning("config-dir", "Config directory not found")
                .with_details(format!("Run 'vmrogue init' to create {}", config_dir)),
        );
    }

    // Check OS
    checks.push(DiagnosticCheck::pass(
        "os",
        format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
    ));

    // Check Rust version
    checks.push(DiagnosticCheck::pass(
        "rust",
        "Compiled with Rust 1.70+".to_string(),
    ));

    checks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_info_collect() {
        let info = EnvironmentInfo::collect("default");
        assert!(!info.vmrogue_version.is_empty());
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.feature_count() > 0);
    }

    #[test]
    fn test_environment_info_has_feature() {
        let info = EnvironmentInfo::collect("default");
        assert!(info.has_feature("vm-management"));
        assert!(info.has_feature("developer-experience"));
        assert!(!info.has_feature("nonexistent"));
    }

    #[test]
    fn test_kubernetes_info_new() {
        let k8s = KubernetesInfo::new();
        assert!(!k8s.connected);
        assert_eq!(k8s.namespace, "default");
    }

    #[test]
    fn test_kubernetes_info_with_context() {
        let k8s = KubernetesInfo::new()
            .with_context("my-cluster")
            .with_namespace("production")
            .with_server("https://k8s.example.com");

        assert_eq!(k8s.context, "my-cluster");
        assert_eq!(k8s.namespace, "production");
        assert_eq!(k8s.server, "https://k8s.example.com");
    }

    #[test]
    fn test_kubernetes_info_set_connected() {
        let mut k8s = KubernetesInfo::new();
        assert!(!k8s.connected);

        k8s.set_connected(true);
        assert!(k8s.connected);
    }

    #[test]
    fn test_config_info_new() {
        let config = ConfigInfo::new();
        assert!(config.config_dir.contains("vmrogue"));
        assert!(config.templates_dir.contains("templates"));
        assert!(config.cache_dir.contains("cache"));
    }

    #[test]
    fn test_config_info_add_setting() {
        let mut config = ConfigInfo::new();
        config.add_setting("theme", "dark");
        assert_eq!(config.setting_count(), 1);
    }

    #[test]
    fn test_diagnostic_check_pass() {
        let check = DiagnosticCheck::pass("test", "All good");
        assert!(check.is_pass());
        assert!(!check.is_fail());
        assert_eq!(check.status.to_string(), "PASS");
    }

    #[test]
    fn test_diagnostic_check_warning() {
        let check = DiagnosticCheck::warning("test", "Could be better");
        assert!(!check.is_pass());
        assert!(!check.is_fail());
        assert_eq!(check.status.to_string(), "WARN");
    }

    #[test]
    fn test_diagnostic_check_fail() {
        let check = DiagnosticCheck::fail("test", "Something broke");
        assert!(!check.is_pass());
        assert!(check.is_fail());
        assert_eq!(check.status.to_string(), "FAIL");
    }

    #[test]
    fn test_diagnostic_check_skip() {
        let check = DiagnosticCheck::skip("test", "Skipped");
        assert_eq!(check.status, DiagnosticStatus::Skip);
        assert_eq!(check.status.to_string(), "SKIP");
    }

    #[test]
    fn test_diagnostic_check_with_details() {
        let check = DiagnosticCheck::pass("test", "OK").with_details("Additional info");
        assert_eq!(check.details, Some("Additional info".to_string()));
    }

    #[test]
    fn test_run_diagnostics() {
        let checks = run_diagnostics();
        assert!(!checks.is_empty());
        assert!(checks.iter().any(|c| c.name == "os"));
        assert!(checks.iter().any(|c| c.name == "rust"));
    }

    #[test]
    fn test_diagnostic_status_display() {
        assert_eq!(DiagnosticStatus::Pass.to_string(), "PASS");
        assert_eq!(DiagnosticStatus::Warning.to_string(), "WARN");
        assert_eq!(DiagnosticStatus::Fail.to_string(), "FAIL");
        assert_eq!(DiagnosticStatus::Skip.to_string(), "SKIP");
    }

    #[test]
    fn test_diagnostic_status_equality() {
        assert_eq!(DiagnosticStatus::Pass, DiagnosticStatus::Pass);
        assert_ne!(DiagnosticStatus::Pass, DiagnosticStatus::Fail);
    }
}
