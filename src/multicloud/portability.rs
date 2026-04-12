use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::CloudProvider;

/// Migration strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStrategy {
    LiveMigration,
    SnapshotAndRestore,
    BlueGreen,
    Canary,
    LiftAndShift,
}

/// Migration status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    Planned,
    Validating,
    InProgress,
    Rollback,
    Completed,
    Failed,
}

/// Workload migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadMigration {
    pub id: String,
    pub name: String,
    pub workload_id: String,
    pub source_provider: CloudProvider,
    pub source_region: String,
    pub target_provider: CloudProvider,
    pub target_region: String,
    pub strategy: MigrationStrategy,
    pub status: MigrationStatus,
    pub progress_percent: u8,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub estimated_duration_minutes: Option<u32>,
    pub created_at: DateTime<Utc>,
}

impl WorkloadMigration {
    pub fn new(
        name: impl Into<String>,
        workload_id: impl Into<String>,
        source_provider: CloudProvider,
        source_region: impl Into<String>,
        target_provider: CloudProvider,
        target_region: impl Into<String>,
        strategy: MigrationStrategy,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "mig-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            workload_id: workload_id.into(),
            source_provider,
            source_region: source_region.into(),
            target_provider,
            target_region: target_region.into(),
            strategy,
            status: MigrationStatus::Planned,
            progress_percent: 0,
            started_at: None,
            completed_at: None,
            estimated_duration_minutes: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_duration_estimate(mut self, minutes: u32) -> Self {
        self.estimated_duration_minutes = Some(minutes);
        self
    }

    pub fn start(&mut self) {
        self.status = MigrationStatus::InProgress;
        self.started_at = Some(Utc::now());
    }

    pub fn update_progress(&mut self, percent: u8) {
        self.progress_percent = percent.min(100);
    }

    pub fn set_status(&mut self, status: MigrationStatus) {
        self.status = status;
        if self.status == MigrationStatus::Completed {
            self.completed_at = Some(Utc::now());
            self.progress_percent = 100;
        }
    }

    pub fn is_complete(&self) -> bool {
        self.status == MigrationStatus::Completed
    }

    pub fn is_in_progress(&self) -> bool {
        self.status == MigrationStatus::InProgress
    }

    pub fn is_cross_provider(&self) -> bool {
        self.source_provider != self.target_provider
    }
}

/// Portability config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortabilityConfig {
    pub id: String,
    pub name: String,
    pub workload_id: String,
    pub supported_providers: Vec<CloudProvider>,
    pub preferred_provider: CloudProvider,
    pub auto_migrate_on_failure: bool,
    pub auto_migrate_on_cost: bool,
    pub cost_threshold_percent: u32,
    pub validation_enabled: bool,
    pub rollback_enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl PortabilityConfig {
    pub fn new(
        name: impl Into<String>,
        workload_id: impl Into<String>,
        preferred_provider: CloudProvider,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "port-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            workload_id: workload_id.into(),
            supported_providers: vec![preferred_provider.clone()],
            preferred_provider,
            auto_migrate_on_failure: false,
            auto_migrate_on_cost: false,
            cost_threshold_percent: 20,
            validation_enabled: true,
            rollback_enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn add_supported_provider(&mut self, provider: CloudProvider) {
        if !self.supported_providers.contains(&provider) {
            self.supported_providers.push(provider);
        }
    }

    pub fn enable_auto_failover(mut self) -> Self {
        self.auto_migrate_on_failure = true;
        self
    }

    pub fn enable_cost_optimization(mut self, threshold_percent: u32) -> Self {
        self.auto_migrate_on_cost = true;
        self.cost_threshold_percent = threshold_percent;
        self
    }

    pub fn disable_validation(mut self) -> Self {
        self.validation_enabled = false;
        self
    }

    pub fn disable_rollback(mut self) -> Self {
        self.rollback_enabled = false;
        self
    }

    pub fn provider_count(&self) -> usize {
        self.supported_providers.len()
    }
}

/// Portability manager
pub struct PortabilityManager {
    migrations: HashMap<String, WorkloadMigration>,
    configs: HashMap<String, PortabilityConfig>,
}

impl PortabilityManager {
    pub fn new() -> Self {
        Self {
            migrations: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    pub fn add_migration(&mut self, migration: WorkloadMigration) -> String {
        let id = migration.id.clone();
        self.migrations.insert(id.clone(), migration);
        id
    }

    pub fn get_migration(&self, id: &str) -> Option<&WorkloadMigration> {
        self.migrations.get(id)
    }

    pub fn get_migration_mut(&mut self, id: &str) -> Option<&mut WorkloadMigration> {
        self.migrations.get_mut(id)
    }

    pub fn migration_count(&self) -> usize {
        self.migrations.len()
    }

    pub fn add_config(&mut self, config: PortabilityConfig) -> String {
        let id = config.id.clone();
        self.configs.insert(id.clone(), config);
        id
    }

    pub fn get_config(&self, id: &str) -> Option<&PortabilityConfig> {
        self.configs.get(id)
    }

    pub fn get_config_mut(&mut self, id: &str) -> Option<&mut PortabilityConfig> {
        self.configs.get_mut(id)
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    pub fn migrations_by_status(&self, status: &MigrationStatus) -> Vec<&WorkloadMigration> {
        self.migrations
            .values()
            .filter(|m| &m.status == status)
            .collect()
    }

    pub fn in_progress_migrations(&self) -> Vec<&WorkloadMigration> {
        self.migrations
            .values()
            .filter(|m| m.is_in_progress())
            .collect()
    }

    pub fn completed_migrations(&self) -> Vec<&WorkloadMigration> {
        self.migrations
            .values()
            .filter(|m| m.is_complete())
            .collect()
    }

    pub fn cross_provider_migrations(&self) -> Vec<&WorkloadMigration> {
        self.migrations
            .values()
            .filter(|m| m.is_cross_provider())
            .collect()
    }

    pub fn migrations_by_strategy(&self, strategy: &MigrationStrategy) -> Vec<&WorkloadMigration> {
        self.migrations
            .values()
            .filter(|m| &m.strategy == strategy)
            .collect()
    }

    pub fn configs_with_auto_failover(&self) -> Vec<&PortabilityConfig> {
        self.configs
            .values()
            .filter(|c| c.auto_migrate_on_failure)
            .collect()
    }

    pub fn configs_with_cost_optimization(&self) -> Vec<&PortabilityConfig> {
        self.configs
            .values()
            .filter(|c| c.auto_migrate_on_cost)
            .collect()
    }
}

impl Default for PortabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_migration() {
        let migration = WorkloadMigration::new(
            "vm-migration",
            "wl-123",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::LiveMigration,
        );

        assert_eq!(migration.name, "vm-migration");
        assert_eq!(migration.workload_id, "wl-123");
        assert_eq!(migration.source_provider, CloudProvider::AWS);
        assert_eq!(migration.target_provider, CloudProvider::Azure);
        assert_eq!(migration.strategy, MigrationStrategy::LiveMigration);
        assert_eq!(migration.status, MigrationStatus::Planned);
        assert_eq!(migration.progress_percent, 0);
    }

    #[test]
    fn test_migration_with_duration_estimate() {
        let migration = WorkloadMigration::new(
            "mig",
            "wl-1",
            CloudProvider::GCP,
            "us-central1",
            CloudProvider::AWS,
            "us-east-1",
            MigrationStrategy::SnapshotAndRestore,
        )
        .with_duration_estimate(30);

        assert_eq!(migration.estimated_duration_minutes, Some(30));
    }

    #[test]
    fn test_migration_start() {
        let mut migration = WorkloadMigration::new(
            "mig",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::BlueGreen,
        );

        migration.start();
        assert_eq!(migration.status, MigrationStatus::InProgress);
        assert!(migration.started_at.is_some());
    }

    #[test]
    fn test_migration_update_progress() {
        let mut migration = WorkloadMigration::new(
            "mig",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::GCP,
            "us-central1",
            MigrationStrategy::Canary,
        );

        migration.update_progress(50);
        assert_eq!(migration.progress_percent, 50);

        migration.update_progress(150); // Over 100
        assert_eq!(migration.progress_percent, 100);
    }

    #[test]
    fn test_migration_set_status() {
        let mut migration = WorkloadMigration::new(
            "mig",
            "wl-1",
            CloudProvider::Azure,
            "eastus",
            CloudProvider::AWS,
            "us-east-1",
            MigrationStrategy::LiveMigration,
        );

        migration.set_status(MigrationStatus::Completed);
        assert_eq!(migration.status, MigrationStatus::Completed);
        assert_eq!(migration.progress_percent, 100);
        assert!(migration.completed_at.is_some());
    }

    #[test]
    fn test_migration_is_complete() {
        let mut migration = WorkloadMigration::new(
            "mig",
            "wl-1",
            CloudProvider::GCP,
            "us-central1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::SnapshotAndRestore,
        );

        assert!(!migration.is_complete());

        migration.set_status(MigrationStatus::Completed);
        assert!(migration.is_complete());
    }

    #[test]
    fn test_migration_is_in_progress() {
        let mut migration = WorkloadMigration::new(
            "mig",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::GCP,
            "us-central1",
            MigrationStrategy::LiveMigration,
        );

        assert!(!migration.is_in_progress());

        migration.start();
        assert!(migration.is_in_progress());
    }

    #[test]
    fn test_migration_is_cross_provider() {
        let migration1 = WorkloadMigration::new(
            "cross",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::LiveMigration,
        );
        assert!(migration1.is_cross_provider());

        let migration2 = WorkloadMigration::new(
            "same",
            "wl-2",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            MigrationStrategy::LiveMigration,
        );
        assert!(!migration2.is_cross_provider());
    }

    #[test]
    fn test_portability_config() {
        let config = PortabilityConfig::new("config-1", "wl-123", CloudProvider::AWS);

        assert_eq!(config.name, "config-1");
        assert_eq!(config.workload_id, "wl-123");
        assert_eq!(config.preferred_provider, CloudProvider::AWS);
        assert!(!config.auto_migrate_on_failure);
        assert!(!config.auto_migrate_on_cost);
        assert!(config.validation_enabled);
        assert!(config.rollback_enabled);
    }

    #[test]
    fn test_config_add_supported_provider() {
        let mut config = PortabilityConfig::new("config", "wl-1", CloudProvider::AWS);

        config.add_supported_provider(CloudProvider::Azure);
        config.add_supported_provider(CloudProvider::GCP);
        config.add_supported_provider(CloudProvider::Azure); // Duplicate

        assert_eq!(config.provider_count(), 3); // AWS (initial) + Azure + GCP
    }

    #[test]
    fn test_config_enable_auto_failover() {
        let config =
            PortabilityConfig::new("config", "wl-1", CloudProvider::AWS).enable_auto_failover();

        assert!(config.auto_migrate_on_failure);
    }

    #[test]
    fn test_config_enable_cost_optimization() {
        let config = PortabilityConfig::new("config", "wl-1", CloudProvider::Azure)
            .enable_cost_optimization(15);

        assert!(config.auto_migrate_on_cost);
        assert_eq!(config.cost_threshold_percent, 15);
    }

    #[test]
    fn test_config_disable_validation() {
        let config =
            PortabilityConfig::new("config", "wl-1", CloudProvider::GCP).disable_validation();

        assert!(!config.validation_enabled);
    }

    #[test]
    fn test_config_disable_rollback() {
        let config =
            PortabilityConfig::new("config", "wl-1", CloudProvider::AWS).disable_rollback();

        assert!(!config.rollback_enabled);
    }

    #[test]
    fn test_portability_manager() {
        let mut manager = PortabilityManager::new();

        let migration = WorkloadMigration::new(
            "mig",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::LiveMigration,
        );
        let id = manager.add_migration(migration);

        assert_eq!(manager.migration_count(), 1);
        assert!(manager.get_migration(&id).is_some());
    }

    #[test]
    fn test_manager_add_config() {
        let mut manager = PortabilityManager::new();

        let config = PortabilityConfig::new("config", "wl-1", CloudProvider::AWS);
        let id = manager.add_config(config);

        assert_eq!(manager.config_count(), 1);
        assert!(manager.get_config(&id).is_some());
    }

    #[test]
    fn test_manager_migrations_by_status() {
        let mut manager = PortabilityManager::new();

        let mut migration1 = WorkloadMigration::new(
            "m1",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::LiveMigration,
        );
        migration1.start();

        let migration2 = WorkloadMigration::new(
            "m2",
            "wl-2",
            CloudProvider::GCP,
            "us-central1",
            CloudProvider::AWS,
            "us-east-1",
            MigrationStrategy::SnapshotAndRestore,
        );

        manager.add_migration(migration1);
        manager.add_migration(migration2);

        let in_progress = manager.migrations_by_status(&MigrationStatus::InProgress);
        assert_eq!(in_progress.len(), 1);
    }

    #[test]
    fn test_manager_in_progress_migrations() {
        let mut manager = PortabilityManager::new();

        let mut migration1 = WorkloadMigration::new(
            "m1",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::BlueGreen,
        );
        migration1.start();

        let migration2 = WorkloadMigration::new(
            "m2",
            "wl-2",
            CloudProvider::GCP,
            "us-central1",
            CloudProvider::AWS,
            "us-east-1",
            MigrationStrategy::Canary,
        );

        manager.add_migration(migration1);
        manager.add_migration(migration2);

        let in_progress = manager.in_progress_migrations();
        assert_eq!(in_progress.len(), 1);
    }

    #[test]
    fn test_manager_completed_migrations() {
        let mut manager = PortabilityManager::new();

        let mut migration1 = WorkloadMigration::new(
            "m1",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::LiveMigration,
        );
        migration1.set_status(MigrationStatus::Completed);

        let migration2 = WorkloadMigration::new(
            "m2",
            "wl-2",
            CloudProvider::GCP,
            "us-central1",
            CloudProvider::AWS,
            "us-east-1",
            MigrationStrategy::SnapshotAndRestore,
        );

        manager.add_migration(migration1);
        manager.add_migration(migration2);

        let completed = manager.completed_migrations();
        assert_eq!(completed.len(), 1);
    }

    #[test]
    fn test_manager_cross_provider_migrations() {
        let mut manager = PortabilityManager::new();

        let migration1 = WorkloadMigration::new(
            "m1",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::LiveMigration,
        );
        let migration2 = WorkloadMigration::new(
            "m2",
            "wl-2",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            MigrationStrategy::SnapshotAndRestore,
        );

        manager.add_migration(migration1);
        manager.add_migration(migration2);

        let cross_provider = manager.cross_provider_migrations();
        assert_eq!(cross_provider.len(), 1);
    }

    #[test]
    fn test_manager_migrations_by_strategy() {
        let mut manager = PortabilityManager::new();

        manager.add_migration(WorkloadMigration::new(
            "m1",
            "wl-1",
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            MigrationStrategy::LiveMigration,
        ));
        manager.add_migration(WorkloadMigration::new(
            "m2",
            "wl-2",
            CloudProvider::GCP,
            "us-central1",
            CloudProvider::AWS,
            "us-east-1",
            MigrationStrategy::SnapshotAndRestore,
        ));
        manager.add_migration(WorkloadMigration::new(
            "m3",
            "wl-3",
            CloudProvider::Azure,
            "eastus",
            CloudProvider::GCP,
            "us-central1",
            MigrationStrategy::LiveMigration,
        ));

        let live_migrations = manager.migrations_by_strategy(&MigrationStrategy::LiveMigration);
        assert_eq!(live_migrations.len(), 2);
    }

    #[test]
    fn test_manager_configs_with_auto_failover() {
        let mut manager = PortabilityManager::new();

        let config1 =
            PortabilityConfig::new("c1", "wl-1", CloudProvider::AWS).enable_auto_failover();
        let config2 = PortabilityConfig::new("c2", "wl-2", CloudProvider::Azure);

        manager.add_config(config1);
        manager.add_config(config2);

        let with_failover = manager.configs_with_auto_failover();
        assert_eq!(with_failover.len(), 1);
    }

    #[test]
    fn test_manager_configs_with_cost_optimization() {
        let mut manager = PortabilityManager::new();

        let config1 =
            PortabilityConfig::new("c1", "wl-1", CloudProvider::AWS).enable_cost_optimization(20);
        let config2 = PortabilityConfig::new("c2", "wl-2", CloudProvider::Azure);

        manager.add_config(config1);
        manager.add_config(config2);

        let with_cost_opt = manager.configs_with_cost_optimization();
        assert_eq!(with_cost_opt.len(), 1);
    }

    #[test]
    fn test_migration_strategy_equality() {
        assert_eq!(
            MigrationStrategy::LiveMigration,
            MigrationStrategy::LiveMigration
        );
        assert_ne!(
            MigrationStrategy::LiveMigration,
            MigrationStrategy::BlueGreen
        );
    }

    #[test]
    fn test_migration_status_equality() {
        assert_eq!(MigrationStatus::InProgress, MigrationStatus::InProgress);
        assert_ne!(MigrationStatus::InProgress, MigrationStatus::Completed);
    }
}
