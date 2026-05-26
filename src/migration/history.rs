// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Migration History - Historical tracking of migrations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationHistory {
    pub records: Vec<MigrationRecord>,
    pub max_records: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub id: String,
    pub vm_name: String,
    pub namespace: String,
    pub source_node: String,
    pub target_node: String,
    pub migration_type: String,
    pub status: MigrationOutcome,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_secs: Option<u64>,
    pub data_transferred_bytes: u64,
    pub initiated_by: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationOutcome {
    Succeeded,
    Failed(String),
    Cancelled,
    RolledBack,
}

impl MigrationHistory {
    /// Default persistence path
    fn persistence_path() -> anyhow::Result<std::path::PathBuf> {
        Ok(crate::utils::data_dir()?.join("migration_history.json"))
    }

    /// Load migration history from disk
    pub fn load() -> Self {
        let path = match Self::persistence_path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to determine data directory: {}", e);
                return Self::default();
            }
        };
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(history) => return history,
                    Err(e) => log::warn!("Failed to parse migration history: {}", e),
                },
                Err(e) => log::warn!("Failed to read migration history: {}", e),
            }
        }
        Self::default()
    }

    /// Save migration history to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::persistence_path()?;
        crate::utils::atomic_write(&path, self)
    }

    pub fn new(max: usize) -> Self {
        Self {
            records: Vec::new(),
            max_records: max,
        }
    }

    pub fn record(&mut self, record: MigrationRecord) {
        self.records.insert(0, record);
        self.records.truncate(self.max_records);
        if let Err(e) = self.save() {
            log::warn!("Failed to persist migration history: {}", e);
        }
    }

    pub fn by_vm(&self, vm_name: &str) -> Vec<&MigrationRecord> {
        self.records
            .iter()
            .filter(|r| r.vm_name == vm_name)
            .collect()
    }
    pub fn recent(&self, limit: usize) -> Vec<&MigrationRecord> {
        self.records.iter().take(limit).collect()
    }
    pub fn failed(&self) -> Vec<&MigrationRecord> {
        self.records
            .iter()
            .filter(|r| matches!(r.status, MigrationOutcome::Failed(_)))
            .collect()
    }
    pub fn success_rate(&self) -> f64 {
        if self.records.is_empty() {
            return 0.0;
        }
        let succeeded = self
            .records
            .iter()
            .filter(|r| r.status == MigrationOutcome::Succeeded)
            .count();
        succeeded as f64 / self.records.len() as f64 * 100.0
    }
}

impl Default for MigrationHistory {
    fn default() -> Self {
        Self::new(1000)
    }
}
