// Synchronization - GitOps sync logic and strategies

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sync strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStrategy {
    Auto,      // Automatic sync on detection
    Manual,    // Manual trigger required
    Scheduled, // Sync on schedule
    Hook,      // Webhook-triggered
}

/// Sync policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPolicy {
    pub strategy: SyncStrategy,
    pub auto_prune: bool,
    pub self_heal: bool,
    pub allow_empty: bool,
    pub retry_limit: u32,
    pub retry_backoff_seconds: u64,
}

impl SyncPolicy {
    pub fn new(strategy: SyncStrategy) -> Self {
        Self {
            strategy,
            auto_prune: false,
            self_heal: true,
            allow_empty: false,
            retry_limit: 3,
            retry_backoff_seconds: 30,
        }
    }

    pub fn auto() -> Self {
        Self::new(SyncStrategy::Auto)
    }

    pub fn manual() -> Self {
        Self::new(SyncStrategy::Manual)
    }

    pub fn with_prune(mut self) -> Self {
        self.auto_prune = true;
        self
    }

    pub fn disable_self_heal(mut self) -> Self {
        self.self_heal = false;
        self
    }
}

impl Default for SyncPolicy {
    fn default() -> Self {
        Self::auto()
    }
}

/// Resource sync state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceSyncState {
    Synced,
    OutOfSync,
    Progressing,
    Failed,
    Unknown,
}

/// Synced resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncedResource {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub state: ResourceSyncState,
    pub git_revision: String,
    pub last_synced: DateTime<Utc>,
    pub message: Option<String>,
}

impl SyncedResource {
    pub fn new(
        kind: impl Into<String>,
        name: impl Into<String>,
        namespace: impl Into<String>,
        revision: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            name: name.into(),
            namespace: namespace.into(),
            state: ResourceSyncState::Synced,
            git_revision: revision.into(),
            last_synced: Utc::now(),
            message: None,
        }
    }

    pub fn with_state(mut self, state: ResourceSyncState) -> Self {
        self.state = state;
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn is_synced(&self) -> bool {
        matches!(self.state, ResourceSyncState::Synced)
    }

    pub fn is_healthy(&self) -> bool {
        matches!(
            self.state,
            ResourceSyncState::Synced | ResourceSyncState::Progressing
        )
    }
}

/// Sync plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPlan {
    pub resources_to_create: Vec<String>,
    pub resources_to_update: Vec<String>,
    pub resources_to_delete: Vec<String>,
    pub total_resources: usize,
}

impl SyncPlan {
    pub fn new() -> Self {
        Self {
            resources_to_create: Vec::new(),
            resources_to_update: Vec::new(),
            resources_to_delete: Vec::new(),
            total_resources: 0,
        }
    }

    pub fn add_create(&mut self, resource: impl Into<String>) {
        self.resources_to_create.push(resource.into());
        self.total_resources += 1;
    }

    pub fn add_update(&mut self, resource: impl Into<String>) {
        self.resources_to_update.push(resource.into());
        self.total_resources += 1;
    }

    pub fn add_delete(&mut self, resource: impl Into<String>) {
        self.resources_to_delete.push(resource.into());
        self.total_resources += 1;
    }

    pub fn has_changes(&self) -> bool {
        self.total_resources > 0
    }

    pub fn create_count(&self) -> usize {
        self.resources_to_create.len()
    }

    pub fn update_count(&self) -> usize {
        self.resources_to_update.len()
    }

    pub fn delete_count(&self) -> usize {
        self.resources_to_delete.len()
    }
}

impl Default for SyncPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Sync result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub success: bool,
    pub resources_synced: usize,
    pub resources_failed: usize,
    pub revision: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub errors: Vec<String>,
}

impl SyncResult {
    pub fn new(revision: impl Into<String>) -> Self {
        Self {
            success: false,
            resources_synced: 0,
            resources_failed: 0,
            revision: revision.into(),
            started_at: Utc::now(),
            completed_at: None,
            errors: Vec::new(),
        }
    }

    pub fn mark_success(&mut self) {
        self.success = true;
        self.completed_at = Some(Utc::now());
    }

    pub fn mark_failure(&mut self) {
        self.success = false;
        self.completed_at = Some(Utc::now());
    }

    pub fn add_synced_resource(&mut self) {
        self.resources_synced += 1;
    }

    pub fn add_failed_resource(&mut self, error: impl Into<String>) {
        self.resources_failed += 1;
        self.errors.push(error.into());
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

    pub fn total_resources(&self) -> usize {
        self.resources_synced + self.resources_failed
    }
}

/// Sync engine
pub struct SyncEngine {
    policy: SyncPolicy,
    resources: HashMap<String, SyncedResource>,
}

impl SyncEngine {
    pub fn new(policy: SyncPolicy) -> Self {
        Self {
            policy,
            resources: HashMap::new(),
        }
    }

    pub fn with_auto_sync() -> Self {
        Self::new(SyncPolicy::auto())
    }

    pub fn with_manual_sync() -> Self {
        Self::new(SyncPolicy::manual())
    }

    pub fn get_policy(&self) -> &SyncPolicy {
        &self.policy
    }

    pub fn add_resource(&mut self, resource: SyncedResource) {
        let key = format!("{}/{}/{}", resource.kind, resource.namespace, resource.name);
        self.resources.insert(key, resource);
    }

    pub fn get_resource(&self, kind: &str, namespace: &str, name: &str) -> Option<&SyncedResource> {
        let key = format!("{}/{}/{}", kind, namespace, name);
        self.resources.get(&key)
    }

    pub fn remove_resource(&mut self, kind: &str, namespace: &str, name: &str) -> bool {
        let key = format!("{}/{}/{}", kind, namespace, name);
        self.resources.remove(&key).is_some()
    }

    pub fn list_resources(&self) -> Vec<&SyncedResource> {
        self.resources.values().collect()
    }

    pub fn synced_resources(&self) -> Vec<&SyncedResource> {
        self.resources.values().filter(|r| r.is_synced()).collect()
    }

    pub fn out_of_sync_resources(&self) -> Vec<&SyncedResource> {
        self.resources.values().filter(|r| !r.is_synced()).collect()
    }

    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    pub fn synced_count(&self) -> usize {
        self.synced_resources().len()
    }

    pub fn out_of_sync_count(&self) -> usize {
        self.out_of_sync_resources().len()
    }

    pub fn create_sync_plan(&self) -> SyncPlan {
        let mut plan = SyncPlan::new();

        for resource in self.out_of_sync_resources() {
            plan.add_update(format!("{}/{}", resource.kind, resource.name));
        }

        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_strategy() {
        assert_eq!(SyncStrategy::Auto, SyncStrategy::Auto);
        assert_ne!(SyncStrategy::Auto, SyncStrategy::Manual);
    }

    #[test]
    fn test_sync_policy_defaults() {
        let policy = SyncPolicy::auto();

        assert_eq!(policy.strategy, SyncStrategy::Auto);
        assert!(!policy.auto_prune);
        assert!(policy.self_heal);
        assert_eq!(policy.retry_limit, 3);
    }

    #[test]
    fn test_sync_policy_options() {
        let policy = SyncPolicy::manual().with_prune().disable_self_heal();

        assert_eq!(policy.strategy, SyncStrategy::Manual);
        assert!(policy.auto_prune);
        assert!(!policy.self_heal);
    }

    #[test]
    fn test_resource_sync_state() {
        assert_eq!(ResourceSyncState::Synced, ResourceSyncState::Synced);
        assert_ne!(ResourceSyncState::Synced, ResourceSyncState::OutOfSync);
    }

    #[test]
    fn test_synced_resource() {
        let resource = SyncedResource::new("VirtualMachine", "test-vm", "default", "abc123")
            .with_state(ResourceSyncState::Synced)
            .with_message("Successfully synced");

        assert_eq!(resource.kind, "VirtualMachine");
        assert_eq!(resource.name, "test-vm");
        assert_eq!(resource.namespace, "default");
        assert!(resource.is_synced());
        assert!(resource.is_healthy());
    }

    #[test]
    fn test_resource_health_check() {
        let synced = SyncedResource::new("VM", "test", "default", "rev")
            .with_state(ResourceSyncState::Synced);
        assert!(synced.is_healthy());

        let progressing = SyncedResource::new("VM", "test", "default", "rev")
            .with_state(ResourceSyncState::Progressing);
        assert!(progressing.is_healthy());

        let failed = SyncedResource::new("VM", "test", "default", "rev")
            .with_state(ResourceSyncState::Failed);
        assert!(!failed.is_healthy());
    }

    #[test]
    fn test_sync_plan() {
        let mut plan = SyncPlan::new();

        plan.add_create("vm-1");
        plan.add_update("vm-2");
        plan.add_delete("vm-3");

        assert!(plan.has_changes());
        assert_eq!(plan.total_resources, 3);
        assert_eq!(plan.create_count(), 1);
        assert_eq!(plan.update_count(), 1);
        assert_eq!(plan.delete_count(), 1);
    }

    #[test]
    fn test_sync_plan_empty() {
        let plan = SyncPlan::new();

        assert!(!plan.has_changes());
        assert_eq!(plan.total_resources, 0);
    }

    #[test]
    fn test_sync_result() {
        let mut result = SyncResult::new("abc123");

        result.add_synced_resource();
        result.add_synced_resource();
        result.add_failed_resource("Connection timeout");

        assert_eq!(result.resources_synced, 2);
        assert_eq!(result.resources_failed, 1);
        assert_eq!(result.total_resources(), 3);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_sync_result_success() {
        let mut result = SyncResult::new("abc123");

        result.add_synced_resource();
        result.mark_success();

        assert!(result.success);
        assert!(result.completed_at.is_some());
    }

    #[test]
    fn test_sync_result_failure() {
        let mut result = SyncResult::new("abc123");

        result.add_failed_resource("Error occurred");
        result.mark_failure();

        assert!(!result.success);
        assert!(result.completed_at.is_some());
    }

    #[test]
    fn test_sync_result_duration() {
        let result = SyncResult::new("abc123");
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(result.duration_seconds() >= 0);
    }

    #[test]
    fn test_sync_engine() {
        let engine = SyncEngine::with_auto_sync();

        assert_eq!(engine.get_policy().strategy, SyncStrategy::Auto);
        assert_eq!(engine.resource_count(), 0);
    }

    #[test]
    fn test_engine_add_resource() {
        let mut engine = SyncEngine::with_auto_sync();

        let resource = SyncedResource::new("VM", "test-vm", "default", "abc123");
        engine.add_resource(resource);

        assert_eq!(engine.resource_count(), 1);
    }

    #[test]
    fn test_engine_get_resource() {
        let mut engine = SyncEngine::with_auto_sync();

        let resource = SyncedResource::new("VM", "test-vm", "default", "abc123");
        engine.add_resource(resource);

        let found = engine.get_resource("VM", "default", "test-vm");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-vm");
    }

    #[test]
    fn test_engine_remove_resource() {
        let mut engine = SyncEngine::with_auto_sync();

        let resource = SyncedResource::new("VM", "test-vm", "default", "abc123");
        engine.add_resource(resource);

        assert!(engine.remove_resource("VM", "default", "test-vm"));
        assert_eq!(engine.resource_count(), 0);
    }

    #[test]
    fn test_engine_sync_status() {
        let mut engine = SyncEngine::with_auto_sync();

        let synced = SyncedResource::new("VM", "vm-1", "default", "abc123")
            .with_state(ResourceSyncState::Synced);

        let out_of_sync = SyncedResource::new("VM", "vm-2", "default", "abc123")
            .with_state(ResourceSyncState::OutOfSync);

        engine.add_resource(synced);
        engine.add_resource(out_of_sync);

        assert_eq!(engine.synced_count(), 1);
        assert_eq!(engine.out_of_sync_count(), 1);
    }

    #[test]
    fn test_engine_create_sync_plan() {
        let mut engine = SyncEngine::with_auto_sync();

        let out_of_sync = SyncedResource::new("VM", "vm-1", "default", "abc123")
            .with_state(ResourceSyncState::OutOfSync);

        engine.add_resource(out_of_sync);

        let plan = engine.create_sync_plan();
        assert!(plan.has_changes());
        assert_eq!(plan.update_count(), 1);
    }

    #[test]
    fn test_sync_engine_with_manual() {
        let engine = SyncEngine::with_manual_sync();

        assert_eq!(engine.get_policy().strategy, SyncStrategy::Manual);
    }

    #[test]
    fn test_sync_policy_default() {
        let policy = SyncPolicy::default();

        assert_eq!(policy.strategy, SyncStrategy::Auto);
    }
}
