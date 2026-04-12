// GitOps Integration - Git-based declarative VM management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod manifests;
pub mod reconciliation;
pub mod repository;
pub mod sync;

/// GitOps configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsConfig {
    pub repository_url: String,
    pub branch: String,
    pub path: String,
    pub sync_interval_seconds: u64,
    pub auto_sync: bool,
    pub prune: bool,
    pub self_heal: bool,
    pub credentials: Option<GitCredentials>,
}

impl GitOpsConfig {
    pub fn new(repository_url: impl Into<String>) -> Self {
        Self {
            repository_url: repository_url.into(),
            branch: "main".to_string(),
            path: ".".to_string(),
            sync_interval_seconds: 300, // 5 minutes
            auto_sync: true,
            prune: false,
            self_heal: true,
            credentials: None,
        }
    }

    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn with_sync_interval(mut self, seconds: u64) -> Self {
        self.sync_interval_seconds = seconds;
        self
    }

    pub fn with_credentials(mut self, credentials: GitCredentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    pub fn disable_auto_sync(mut self) -> Self {
        self.auto_sync = false;
        self
    }

    pub fn enable_prune(mut self) -> Self {
        self.prune = true;
        self
    }
}

/// Git credentials
#[derive(Clone, Deserialize)]
pub enum GitCredentials {
    SSH { private_key_path: String },
    HTTPS { username: String, password: String },
    Token { token: String },
}

impl std::fmt::Debug for GitCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitCredentials::SSH { .. } => f
                .debug_struct("SSH")
                .field("private_key_path", &"[REDACTED]")
                .finish(),
            GitCredentials::HTTPS { username, .. } => f
                .debug_struct("HTTPS")
                .field("username", username)
                .field("password", &"[REDACTED]")
                .finish(),
            GitCredentials::Token { .. } => f
                .debug_struct("Token")
                .field("token", &"[REDACTED]")
                .finish(),
        }
    }
}

impl Serialize for GitCredentials {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStructVariant;
        match self {
            GitCredentials::SSH { .. } => {
                let mut sv = serializer.serialize_struct_variant("GitCredentials", 0, "SSH", 1)?;
                sv.serialize_field("private_key_path", "[REDACTED]")?;
                sv.end()
            }
            GitCredentials::HTTPS { username, .. } => {
                let mut sv =
                    serializer.serialize_struct_variant("GitCredentials", 1, "HTTPS", 2)?;
                sv.serialize_field("username", username)?;
                sv.serialize_field("password", "[REDACTED]")?;
                sv.end()
            }
            GitCredentials::Token { .. } => {
                let mut sv =
                    serializer.serialize_struct_variant("GitCredentials", 2, "Token", 1)?;
                sv.serialize_field("token", "[REDACTED]")?;
                sv.end()
            }
        }
    }
}

/// GitOps application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsApplication {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub config: GitOpsConfig,
    pub sync_status: SyncStatus,
    pub health_status: HealthStatus,
    pub created_at: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
    pub revision: Option<String>,
}

impl GitOpsApplication {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        config: GitOpsConfig,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "app-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            config,
            sync_status: SyncStatus::OutOfSync,
            health_status: HealthStatus::Unknown,
            created_at: Utc::now(),
            last_sync: None,
            revision: None,
        }
    }

    pub fn is_synced(&self) -> bool {
        matches!(self.sync_status, SyncStatus::Synced)
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.health_status, HealthStatus::Healthy)
    }

    pub fn update_sync_status(&mut self, status: SyncStatus, revision: Option<String>) {
        self.sync_status = status;
        self.revision = revision;
        self.last_sync = Some(Utc::now());
    }

    pub fn update_health_status(&mut self, status: HealthStatus) {
        self.health_status = status;
    }
}

/// Synchronization status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Synced,
    OutOfSync,
    Syncing,
    Failed { error: String },
    Unknown,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Synced => write!(f, "Synced"),
            SyncStatus::OutOfSync => write!(f, "OutOfSync"),
            SyncStatus::Syncing => write!(f, "Syncing"),
            SyncStatus::Failed { error } => write!(f, "Failed: {}", error),
            SyncStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Progressing,
    Suspended,
    Missing,
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Progressing => write!(f, "Progressing"),
            HealthStatus::Suspended => write!(f, "Suspended"),
            HealthStatus::Missing => write!(f, "Missing"),
            HealthStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOperation {
    pub id: String,
    pub app_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: SyncOperationStatus,
    pub resources_synced: usize,
    pub errors: Vec<String>,
    pub dry_run: bool,
}

impl SyncOperation {
    pub fn new(app_id: impl Into<String>, dry_run: bool) -> Self {
        Self {
            id: format!("sync-{}", Utc::now().timestamp_millis()),
            app_id: app_id.into(),
            started_at: Utc::now(),
            completed_at: None,
            status: SyncOperationStatus::Running,
            resources_synced: 0,
            errors: Vec::new(),
            dry_run,
        }
    }

    pub fn complete(&mut self, status: SyncOperationStatus) {
        self.completed_at = Some(Utc::now());
        self.status = status;
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    pub fn increment_resources(&mut self) {
        self.resources_synced += 1;
    }

    pub fn duration_seconds(&self) -> i64 {
        match self.completed_at {
            Some(completed) => completed
                .signed_duration_since(self.started_at)
                .num_seconds(),
            None => Utc::now()
                .signed_duration_since(self.started_at)
                .num_seconds(),
        }
    }
}

/// Sync operation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncOperationStatus {
    Running,
    Succeeded,
    Failed,
    Terminated,
}

/// GitOps manager
pub struct GitOpsManager {
    applications: HashMap<String, GitOpsApplication>,
    operations: Vec<SyncOperation>,
}

impl GitOpsManager {
    pub fn new() -> Self {
        Self {
            applications: HashMap::new(),
            operations: Vec::new(),
        }
    }

    pub fn add_application(&mut self, app: GitOpsApplication) {
        self.applications.insert(app.id.clone(), app);
    }

    pub fn get_application(&self, app_id: &str) -> Option<&GitOpsApplication> {
        self.applications.get(app_id)
    }

    pub fn get_application_mut(&mut self, app_id: &str) -> Option<&mut GitOpsApplication> {
        self.applications.get_mut(app_id)
    }

    pub fn remove_application(&mut self, app_id: &str) -> bool {
        self.applications.remove(app_id).is_some()
    }

    pub fn list_applications(&self) -> Vec<&GitOpsApplication> {
        self.applications.values().collect()
    }

    pub fn synced_applications(&self) -> Vec<&GitOpsApplication> {
        self.applications
            .values()
            .filter(|a| a.is_synced())
            .collect()
    }

    pub fn out_of_sync_applications(&self) -> Vec<&GitOpsApplication> {
        self.applications
            .values()
            .filter(|a| !a.is_synced())
            .collect()
    }

    pub fn healthy_applications(&self) -> Vec<&GitOpsApplication> {
        self.applications
            .values()
            .filter(|a| a.is_healthy())
            .collect()
    }

    pub fn start_sync(&mut self, app_id: &str, dry_run: bool) -> Option<String> {
        if self.applications.contains_key(app_id) {
            let operation = SyncOperation::new(app_id, dry_run);
            let op_id = operation.id.clone();
            self.operations.push(operation);
            Some(op_id)
        } else {
            None
        }
    }

    pub fn get_operation(&self, op_id: &str) -> Option<&SyncOperation> {
        self.operations.iter().find(|o| o.id == op_id)
    }

    pub fn get_operation_mut(&mut self, op_id: &str) -> Option<&mut SyncOperation> {
        self.operations.iter_mut().find(|o| o.id == op_id)
    }

    pub fn recent_operations(&self, limit: usize) -> Vec<&SyncOperation> {
        let mut ops: Vec<&SyncOperation> = self.operations.iter().collect();
        ops.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        ops.into_iter().take(limit).collect()
    }

    pub fn application_count(&self) -> usize {
        self.applications.len()
    }

    pub fn synced_count(&self) -> usize {
        self.synced_applications().len()
    }

    pub fn out_of_sync_count(&self) -> usize {
        self.out_of_sync_applications().len()
    }
}

impl Default for GitOpsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitops_config() {
        let config = GitOpsConfig::new("https://github.com/org/repo.git")
            .with_branch("develop")
            .with_path("vms")
            .with_sync_interval(600);

        assert_eq!(config.repository_url, "https://github.com/org/repo.git");
        assert_eq!(config.branch, "develop");
        assert_eq!(config.path, "vms");
        assert_eq!(config.sync_interval_seconds, 600);
    }

    #[test]
    fn test_gitops_config_defaults() {
        let config = GitOpsConfig::new("https://example.com/repo.git");

        assert_eq!(config.branch, "main");
        assert_eq!(config.path, ".");
        assert!(config.auto_sync);
        assert!(!config.prune);
        assert!(config.self_heal);
    }

    #[test]
    fn test_gitops_config_options() {
        let config = GitOpsConfig::new("https://example.com/repo.git")
            .disable_auto_sync()
            .enable_prune();

        assert!(!config.auto_sync);
        assert!(config.prune);
    }

    #[test]
    fn test_git_credentials() {
        let ssh = GitCredentials::SSH {
            private_key_path: "/home/user/.ssh/id_rsa".to_string(),
        };
        assert!(matches!(ssh, GitCredentials::SSH { .. }));

        let token = GitCredentials::Token {
            token: "ghp_token".to_string(),
        };
        assert!(matches!(token, GitCredentials::Token { .. }));
    }

    #[test]
    fn test_gitops_application() {
        let config = GitOpsConfig::new("https://example.com/repo.git");
        let app = GitOpsApplication::new("my-app", "default", config);

        assert_eq!(app.name, "my-app");
        assert_eq!(app.namespace, "default");
        assert!(!app.is_synced());
        assert!(!app.is_healthy());
    }

    #[test]
    fn test_application_sync_status() {
        let config = GitOpsConfig::new("https://example.com/repo.git");
        let mut app = GitOpsApplication::new("test", "default", config);

        app.update_sync_status(SyncStatus::Synced, Some("abc123".to_string()));

        assert!(app.is_synced());
        assert_eq!(app.revision, Some("abc123".to_string()));
        assert!(app.last_sync.is_some());
    }

    #[test]
    fn test_application_health_status() {
        let config = GitOpsConfig::new("https://example.com/repo.git");
        let mut app = GitOpsApplication::new("test", "default", config);

        app.update_health_status(HealthStatus::Healthy);

        assert!(app.is_healthy());
    }

    #[test]
    fn test_sync_status_display() {
        assert_eq!(SyncStatus::Synced.to_string(), "Synced");
        assert_eq!(SyncStatus::OutOfSync.to_string(), "OutOfSync");
        assert!(SyncStatus::Failed {
            error: "test".to_string()
        }
        .to_string()
        .contains("Failed"));
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "Healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "Degraded");
        assert_eq!(HealthStatus::Progressing.to_string(), "Progressing");
    }

    #[test]
    fn test_sync_operation() {
        let operation = SyncOperation::new("app-1", false);

        assert_eq!(operation.app_id, "app-1");
        assert!(!operation.dry_run);
        assert_eq!(operation.status, SyncOperationStatus::Running);
        assert_eq!(operation.resources_synced, 0);
    }

    #[test]
    fn test_sync_operation_completion() {
        let mut operation = SyncOperation::new("app-1", false);

        operation.increment_resources();
        operation.increment_resources();
        operation.complete(SyncOperationStatus::Succeeded);

        assert_eq!(operation.status, SyncOperationStatus::Succeeded);
        assert_eq!(operation.resources_synced, 2);
        assert!(operation.completed_at.is_some());
    }

    #[test]
    fn test_sync_operation_errors() {
        let mut operation = SyncOperation::new("app-1", false);

        operation.add_error("Failed to apply resource");
        operation.add_error("Connection timeout");

        assert_eq!(operation.errors.len(), 2);
    }

    #[test]
    fn test_sync_operation_duration() {
        let operation = SyncOperation::new("app-1", false);
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(operation.duration_seconds() >= 0);
    }

    #[test]
    fn test_gitops_manager() {
        let mut manager = GitOpsManager::new();

        let config = GitOpsConfig::new("https://example.com/repo.git");
        let app = GitOpsApplication::new("test-app", "default", config);

        manager.add_application(app);

        assert_eq!(manager.application_count(), 1);
    }

    #[test]
    fn test_manager_get_application() {
        let mut manager = GitOpsManager::new();

        let config = GitOpsConfig::new("https://example.com/repo.git");
        let app = GitOpsApplication::new("test", "default", config);
        let app_id = app.id.clone();

        manager.add_application(app);

        assert!(manager.get_application(&app_id).is_some());
    }

    #[test]
    fn test_manager_remove_application() {
        let mut manager = GitOpsManager::new();

        let config = GitOpsConfig::new("https://example.com/repo.git");
        let app = GitOpsApplication::new("test", "default", config);
        let app_id = app.id.clone();

        manager.add_application(app);
        assert_eq!(manager.application_count(), 1);

        assert!(manager.remove_application(&app_id));
        assert_eq!(manager.application_count(), 0);
    }

    #[test]
    fn test_manager_sync_status_filters() {
        let mut manager = GitOpsManager::new();

        let config = GitOpsConfig::new("https://example.com/repo.git");
        let mut app1 = GitOpsApplication::new("synced", "default", config.clone());
        app1.update_sync_status(SyncStatus::Synced, None);

        let app2 = GitOpsApplication::new("out-of-sync", "default", config);

        manager.add_application(app1);
        manager.add_application(app2);

        assert_eq!(manager.synced_count(), 1);
        assert_eq!(manager.out_of_sync_count(), 1);
    }

    #[test]
    fn test_manager_health_filters() {
        let mut manager = GitOpsManager::new();

        let config = GitOpsConfig::new("https://example.com/repo.git");
        let mut app = GitOpsApplication::new("healthy", "default", config);
        app.update_health_status(HealthStatus::Healthy);

        manager.add_application(app);

        assert_eq!(manager.healthy_applications().len(), 1);
    }

    #[test]
    fn test_manager_start_sync() {
        let mut manager = GitOpsManager::new();

        let config = GitOpsConfig::new("https://example.com/repo.git");
        let app = GitOpsApplication::new("test", "default", config);
        let app_id = app.id.clone();

        manager.add_application(app);

        let op_id = manager.start_sync(&app_id, false);
        assert!(op_id.is_some());
    }

    #[test]
    fn test_manager_get_operation() {
        let mut manager = GitOpsManager::new();

        let config = GitOpsConfig::new("https://example.com/repo.git");
        let app = GitOpsApplication::new("test", "default", config);
        let app_id = app.id.clone();

        manager.add_application(app);

        let op_id = manager.start_sync(&app_id, false).unwrap();
        assert!(manager.get_operation(&op_id).is_some());
    }

    #[test]
    fn test_manager_recent_operations() {
        let mut manager = GitOpsManager::new();

        let config = GitOpsConfig::new("https://example.com/repo.git");
        let app = GitOpsApplication::new("test", "default", config);
        let app_id = app.id.clone();

        manager.add_application(app);

        manager.start_sync(&app_id, false);
        manager.start_sync(&app_id, false);
        manager.start_sync(&app_id, false);

        let recent = manager.recent_operations(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_sync_operation_dry_run() {
        let operation = SyncOperation::new("app-1", true);
        assert!(operation.dry_run);
    }
}
