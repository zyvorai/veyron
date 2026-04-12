use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Replication mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationMode {
    Synchronous,
    Asynchronous,
    SemiSynchronous,
}

impl std::fmt::Display for ReplicationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplicationMode::Synchronous => write!(f, "Synchronous"),
            ReplicationMode::Asynchronous => write!(f, "Asynchronous"),
            ReplicationMode::SemiSynchronous => write!(f, "Semi-Synchronous"),
        }
    }
}

/// Replication status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationStatus {
    Initializing,
    Active,
    Paused,
    Error,
    Stopped,
}

impl std::fmt::Display for ReplicationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplicationStatus::Initializing => write!(f, "Initializing"),
            ReplicationStatus::Active => write!(f, "Active"),
            ReplicationStatus::Paused => write!(f, "Paused"),
            ReplicationStatus::Error => write!(f, "Error"),
            ReplicationStatus::Stopped => write!(f, "Stopped"),
        }
    }
}

/// Replication pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationPair {
    pub id: String,
    pub name: String,
    pub source_resource: String,
    pub target_resource: String,
    pub source_site: String,
    pub target_site: String,
    pub mode: ReplicationMode,
    pub status: ReplicationStatus,
    pub interval_seconds: u64,
    pub bytes_replicated: u64,
    pub last_sync: Option<DateTime<Utc>>,
    pub lag_seconds: u64,
    pub created_at: DateTime<Utc>,
}

impl ReplicationPair {
    pub fn new(
        name: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
        source_site: impl Into<String>,
        target_site: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "rep-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            source_resource: source.into(),
            target_resource: target.into(),
            source_site: source_site.into(),
            target_site: target_site.into(),
            mode: ReplicationMode::Asynchronous,
            status: ReplicationStatus::Initializing,
            interval_seconds: 300,
            bytes_replicated: 0,
            last_sync: None,
            lag_seconds: 0,
            created_at: Utc::now(),
        }
    }

    pub fn with_mode(mut self, mode: ReplicationMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_interval(mut self, seconds: u64) -> Self {
        self.interval_seconds = seconds;
        self
    }

    pub fn start(&mut self) {
        self.status = ReplicationStatus::Active;
    }

    pub fn pause(&mut self) {
        self.status = ReplicationStatus::Paused;
    }

    pub fn stop(&mut self) {
        self.status = ReplicationStatus::Stopped;
    }

    pub fn error(&mut self) {
        self.status = ReplicationStatus::Error;
    }

    pub fn update_sync(&mut self, bytes: u64) {
        self.last_sync = Some(Utc::now());
        self.bytes_replicated += bytes;
    }

    pub fn update_lag(&mut self, seconds: u64) {
        self.lag_seconds = seconds;
    }

    pub fn is_active(&self) -> bool {
        self.status == ReplicationStatus::Active
    }

    pub fn is_synchronous(&self) -> bool {
        self.mode == ReplicationMode::Synchronous
    }

    pub fn is_healthy(&self) -> bool {
        self.status == ReplicationStatus::Active && self.lag_seconds < 60
    }
}

/// Replication manager
pub struct ReplicationManager {
    pairs: HashMap<String, ReplicationPair>,
}

impl ReplicationManager {
    pub fn new() -> Self {
        Self {
            pairs: HashMap::new(),
        }
    }

    pub fn add_pair(&mut self, pair: ReplicationPair) -> String {
        let id = pair.id.clone();
        self.pairs.insert(id.clone(), pair);
        id
    }

    pub fn get_pair(&self, id: &str) -> Option<&ReplicationPair> {
        self.pairs.get(id)
    }

    pub fn get_pair_mut(&mut self, id: &str) -> Option<&mut ReplicationPair> {
        self.pairs.get_mut(id)
    }

    pub fn remove_pair(&mut self, id: &str) -> bool {
        self.pairs.remove(id).is_some()
    }

    pub fn pair_count(&self) -> usize {
        self.pairs.len()
    }

    pub fn active_pairs(&self) -> Vec<&ReplicationPair> {
        self.pairs.values().filter(|p| p.is_active()).collect()
    }

    pub fn by_mode(&self, mode: &ReplicationMode) -> Vec<&ReplicationPair> {
        self.pairs.values().filter(|p| &p.mode == mode).collect()
    }

    pub fn unhealthy_pairs(&self) -> Vec<&ReplicationPair> {
        self.pairs.values().filter(|p| !p.is_healthy()).collect()
    }

    pub fn total_bytes_replicated(&self) -> u64 {
        self.pairs.values().map(|p| p.bytes_replicated).sum()
    }
}

impl Default for ReplicationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replication_mode_display() {
        assert_eq!(ReplicationMode::Synchronous.to_string(), "Synchronous");
        assert_eq!(ReplicationMode::Asynchronous.to_string(), "Asynchronous");
        assert_eq!(
            ReplicationMode::SemiSynchronous.to_string(),
            "Semi-Synchronous"
        );
    }

    #[test]
    fn test_replication_status_display() {
        assert_eq!(ReplicationStatus::Initializing.to_string(), "Initializing");
        assert_eq!(ReplicationStatus::Active.to_string(), "Active");
        assert_eq!(ReplicationStatus::Paused.to_string(), "Paused");
    }

    #[test]
    fn test_replication_pair() {
        let pair = ReplicationPair::new(
            "DB Replication",
            "db-primary",
            "db-secondary",
            "site-1",
            "site-2",
        );

        assert_eq!(pair.name, "DB Replication");
        assert_eq!(pair.source_resource, "db-primary");
        assert_eq!(pair.target_resource, "db-secondary");
        assert_eq!(pair.mode, ReplicationMode::Asynchronous);
        assert_eq!(pair.status, ReplicationStatus::Initializing);
    }

    #[test]
    fn test_pair_builder() {
        let pair = ReplicationPair::new("Replication", "src", "tgt", "s1", "s2")
            .with_mode(ReplicationMode::Synchronous)
            .with_interval(60);

        assert_eq!(pair.mode, ReplicationMode::Synchronous);
        assert_eq!(pair.interval_seconds, 60);
        assert!(pair.is_synchronous());
    }

    #[test]
    fn test_pair_lifecycle() {
        let mut pair = ReplicationPair::new("Test", "src", "tgt", "s1", "s2");

        assert_eq!(pair.status, ReplicationStatus::Initializing);

        pair.start();
        assert_eq!(pair.status, ReplicationStatus::Active);
        assert!(pair.is_active());

        pair.pause();
        assert_eq!(pair.status, ReplicationStatus::Paused);

        pair.start();
        assert!(pair.is_active());

        pair.error();
        assert_eq!(pair.status, ReplicationStatus::Error);

        pair.stop();
        assert_eq!(pair.status, ReplicationStatus::Stopped);
    }

    #[test]
    fn test_pair_sync() {
        let mut pair = ReplicationPair::new("Test", "src", "tgt", "s1", "s2");

        assert_eq!(pair.bytes_replicated, 0);
        assert!(pair.last_sync.is_none());

        pair.update_sync(1024);
        assert_eq!(pair.bytes_replicated, 1024);
        assert!(pair.last_sync.is_some());

        pair.update_sync(2048);
        assert_eq!(pair.bytes_replicated, 3072);
    }

    #[test]
    fn test_pair_lag() {
        let mut pair = ReplicationPair::new("Test", "src", "tgt", "s1", "s2");

        assert_eq!(pair.lag_seconds, 0);

        pair.update_lag(30);
        assert_eq!(pair.lag_seconds, 30);
    }

    #[test]
    fn test_pair_health() {
        let mut pair = ReplicationPair::new("Test", "src", "tgt", "s1", "s2");

        pair.start();
        pair.update_lag(30);
        assert!(pair.is_healthy());

        pair.update_lag(120);
        assert!(!pair.is_healthy());

        pair.error();
        assert!(!pair.is_healthy());
    }

    #[test]
    fn test_replication_manager() {
        let mut manager = ReplicationManager::new();

        let pair = ReplicationPair::new("Test", "src", "tgt", "s1", "s2");
        let id = manager.add_pair(pair);

        assert_eq!(manager.pair_count(), 1);
        assert!(manager.get_pair(&id).is_some());
    }

    #[test]
    fn test_manager_active_pairs() {
        let mut manager = ReplicationManager::new();

        let mut pair1 = ReplicationPair::new("P1", "s1", "t1", "site1", "site2");
        pair1.start();

        let pair2 = ReplicationPair::new("P2", "s2", "t2", "site1", "site2");

        manager.add_pair(pair1);
        manager.add_pair(pair2);

        let active = manager.active_pairs();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_by_mode() {
        let mut manager = ReplicationManager::new();

        manager.add_pair(
            ReplicationPair::new("P1", "s1", "t1", "site1", "site2")
                .with_mode(ReplicationMode::Synchronous),
        );
        manager.add_pair(
            ReplicationPair::new("P2", "s2", "t2", "site1", "site2")
                .with_mode(ReplicationMode::Asynchronous),
        );
        manager.add_pair(
            ReplicationPair::new("P3", "s3", "t3", "site1", "site2")
                .with_mode(ReplicationMode::Synchronous),
        );

        let sync = manager.by_mode(&ReplicationMode::Synchronous);
        assert_eq!(sync.len(), 2);
    }

    #[test]
    fn test_manager_unhealthy_pairs() {
        let mut manager = ReplicationManager::new();

        let mut pair1 = ReplicationPair::new("P1", "s1", "t1", "site1", "site2");
        pair1.start();
        pair1.update_lag(30);

        let mut pair2 = ReplicationPair::new("P2", "s2", "t2", "site1", "site2");
        pair2.start();
        pair2.update_lag(120);

        manager.add_pair(pair1);
        manager.add_pair(pair2);

        let unhealthy = manager.unhealthy_pairs();
        assert_eq!(unhealthy.len(), 1);
    }

    #[test]
    fn test_manager_total_bytes() {
        let mut manager = ReplicationManager::new();

        let mut pair1 = ReplicationPair::new("P1", "s1", "t1", "site1", "site2");
        pair1.update_sync(1000);

        let mut pair2 = ReplicationPair::new("P2", "s2", "t2", "site1", "site2");
        pair2.update_sync(2000);

        manager.add_pair(pair1);
        manager.add_pair(pair2);

        assert_eq!(manager.total_bytes_replicated(), 3000);
    }

    #[test]
    fn test_manager_remove_pair() {
        let mut manager = ReplicationManager::new();

        let pair = ReplicationPair::new("Test", "src", "tgt", "s1", "s2");
        let id = manager.add_pair(pair);

        assert!(manager.remove_pair(&id));
        assert_eq!(manager.pair_count(), 0);
    }

    #[test]
    fn test_manager_get_pair_mut() {
        let mut manager = ReplicationManager::new();

        let pair = ReplicationPair::new("Test", "src", "tgt", "s1", "s2");
        let id = manager.add_pair(pair);

        if let Some(pair_mut) = manager.get_pair_mut(&id) {
            pair_mut.start();
        }

        let pair = manager.get_pair(&id).unwrap();
        assert!(pair.is_active());
    }

    #[test]
    fn test_replication_mode_equality() {
        assert_eq!(ReplicationMode::Synchronous, ReplicationMode::Synchronous);
        assert_ne!(ReplicationMode::Synchronous, ReplicationMode::Asynchronous);
    }

    #[test]
    fn test_replication_status_equality() {
        assert_eq!(ReplicationStatus::Active, ReplicationStatus::Active);
        assert_ne!(ReplicationStatus::Active, ReplicationStatus::Paused);
    }
}
