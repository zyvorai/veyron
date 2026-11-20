// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Backup & Disaster Recovery - VM backup and restore

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub vm_name: String,
    pub backup_name: String,
    pub backup_type: BackupType,
    pub target: BackupTarget,
    pub retention_policy: RetentionPolicy,
    pub compression: CompressionType,
    pub encryption_enabled: bool,
}

impl BackupConfig {
    pub fn new(vm_name: impl Into<String>, backup_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            backup_name: backup_name.into(),
            backup_type: BackupType::Full,
            target: BackupTarget::default(),
            retention_policy: RetentionPolicy::default(),
            compression: CompressionType::Gzip,
            encryption_enabled: true,
        }
    }

    pub fn with_type(mut self, backup_type: BackupType) -> Self {
        self.backup_type = backup_type;
        self
    }

    pub fn with_target(mut self, target: BackupTarget) -> Self {
        self.target = target;
        self
    }

    pub fn with_retention(mut self, policy: RetentionPolicy) -> Self {
        self.retention_policy = policy;
        self
    }

    pub fn without_encryption(mut self) -> Self {
        self.encryption_enabled = false;
        self
    }
}

/// Backup type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupType {
    Full,         // Complete VM backup
    Incremental,  // Changes since last backup
    Differential, // Changes since last full backup
}

impl BackupType {
    pub fn as_str(&self) -> &str {
        match self {
            BackupType::Full => "full",
            BackupType::Incremental => "incremental",
            BackupType::Differential => "differential",
        }
    }
}

/// Backup target location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupTarget {
    S3 {
        bucket: String,
        region: String,
        prefix: String,
    },
    NFS {
        server: String,
        path: String,
    },
    Local {
        path: String,
    },
}

impl Default for BackupTarget {
    fn default() -> Self {
        BackupTarget::Local {
            path: "/var/lib/veyron/backups".to_string(),
        }
    }
}

/// Compression type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

impl std::fmt::Display for CompressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionType::None => write!(f, "None"),
            CompressionType::Gzip => write!(f, "Gzip"),
            CompressionType::Zstd => write!(f, "Zstd"),
            CompressionType::Lz4 => write!(f, "Lz4"),
        }
    }
}

impl CompressionType {
    pub fn extension(&self) -> &str {
        match self {
            CompressionType::None => "",
            CompressionType::Gzip => ".gz",
            CompressionType::Zstd => ".zst",
            CompressionType::Lz4 => ".lz4",
        }
    }
}

/// Retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub keep_daily: u32,
    pub keep_weekly: u32,
    pub keep_monthly: u32,
    pub keep_yearly: u32,
    pub max_age_days: Option<u32>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_daily: 7,
            keep_weekly: 4,
            keep_monthly: 12,
            keep_yearly: 5,
            max_age_days: Some(365),
        }
    }
}

impl RetentionPolicy {
    pub fn short_term() -> Self {
        Self {
            keep_daily: 3,
            keep_weekly: 2,
            keep_monthly: 0,
            keep_yearly: 0,
            max_age_days: Some(30),
        }
    }

    pub fn long_term() -> Self {
        Self {
            keep_daily: 30,
            keep_weekly: 12,
            keep_monthly: 24,
            keep_yearly: 10,
            max_age_days: None,
        }
    }

    /// Check if a single backup should be kept based on max_age_days.
    ///
    /// For GFS (Grandfather-Father-Son) retention that evaluates keep_daily,
    /// keep_weekly, keep_monthly, and keep_yearly, use `filter_backups` instead,
    /// which requires the full list of backups to determine which to keep.
    pub fn should_keep(&self, backup_date: DateTime<Utc>, now: DateTime<Utc>) -> bool {
        if let Some(max_days) = self.max_age_days {
            let age_days = now.signed_duration_since(backup_date).num_days();
            if age_days > max_days as i64 {
                return false;
            }
        }

        true
    }

    /// Filter a list of backups using full GFS retention logic.
    ///
    /// Given a list of `(name, date)` pairs and the current time, returns the
    /// names of backups that should be kept according to the retention policy:
    /// - `max_age_days`: discard backups older than this
    /// - `keep_daily`: keep the most recent backup from each of the last N days
    /// - `keep_weekly`: keep the most recent backup from each of the last N ISO weeks
    /// - `keep_monthly`: keep the most recent backup from each of the last N months
    /// - `keep_yearly`: keep the most recent backup from each of the last N years
    pub fn filter_backups(
        &self,
        backups: &[(String, DateTime<Utc>)],
        now: DateTime<Utc>,
    ) -> Vec<String> {
        let mut keep = std::collections::HashSet::new();

        // Sort by date descending
        let mut sorted: Vec<_> = backups.to_vec();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.1));

        // Keep by max_age_days
        if let Some(max_days) = self.max_age_days {
            let cutoff = now - chrono::Duration::days(max_days as i64);
            for (name, date) in &sorted {
                if *date >= cutoff {
                    keep.insert(name.clone());
                }
            }
        } else {
            // If no max_age, start by keeping all
            for (name, _) in &sorted {
                keep.insert(name.clone());
            }
        }

        // Keep daily (most recent per day)
        if self.keep_daily > 0 {
            let mut days_seen = std::collections::HashSet::new();
            for (name, date) in &sorted {
                let day = date.format("%Y-%m-%d").to_string();
                if days_seen.len() < self.keep_daily as usize && days_seen.insert(day) {
                    keep.insert(name.clone());
                }
            }
        }

        // Keep weekly (most recent per ISO week)
        if self.keep_weekly > 0 {
            let mut weeks_seen = std::collections::HashSet::new();
            for (name, date) in &sorted {
                let week = date.format("%G-W%V").to_string();
                if weeks_seen.len() < self.keep_weekly as usize && weeks_seen.insert(week) {
                    keep.insert(name.clone());
                }
            }
        }

        // Keep monthly
        if self.keep_monthly > 0 {
            let mut months_seen = std::collections::HashSet::new();
            for (name, date) in &sorted {
                let month = date.format("%Y-%m").to_string();
                if months_seen.len() < self.keep_monthly as usize && months_seen.insert(month) {
                    keep.insert(name.clone());
                }
            }
        }

        // Keep yearly
        if self.keep_yearly > 0 {
            let mut years_seen = std::collections::HashSet::new();
            for (name, date) in &sorted {
                let year = date.format("%Y").to_string();
                if years_seen.len() < self.keep_yearly as usize && years_seen.insert(year) {
                    keep.insert(name.clone());
                }
            }
        }

        keep.into_iter().collect()
    }
}

/// Backup status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatus {
    pub backup_name: String,
    pub vm_name: String,
    pub state: BackupState,
    pub backup_type: BackupType,
    pub size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub progress_percent: u8,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub verification_status: Option<VerificationStatus>,
}

impl BackupStatus {
    pub fn new(vm_name: impl Into<String>, backup_name: impl Into<String>) -> Self {
        Self {
            backup_name: backup_name.into(),
            vm_name: vm_name.into(),
            state: BackupState::Pending,
            backup_type: BackupType::Full,
            size_bytes: 0,
            compressed_size_bytes: 0,
            progress_percent: 0,
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
            verification_status: None,
        }
    }

    pub fn duration_secs(&self) -> i64 {
        match self.completed_at {
            Some(completed) => completed
                .signed_duration_since(self.created_at)
                .num_seconds(),
            None => Utc::now()
                .signed_duration_since(self.created_at)
                .num_seconds(),
        }
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.size_bytes == 0 {
            return 0.0;
        }
        (self.compressed_size_bytes as f64 / self.size_bytes as f64) * 100.0
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.state, BackupState::Completed | BackupState::Failed)
    }
}

/// Backup state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupState {
    Pending,
    Creating,
    Compressing,
    Uploading,
    Verifying,
    Completed,
    Failed,
}

impl std::fmt::Display for BackupState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupState::Pending => write!(f, "Pending"),
            BackupState::Creating => write!(f, "Creating"),
            BackupState::Compressing => write!(f, "Compressing"),
            BackupState::Uploading => write!(f, "Uploading"),
            BackupState::Verifying => write!(f, "Verifying"),
            BackupState::Completed => write!(f, "Completed"),
            BackupState::Failed => write!(f, "Failed"),
        }
    }
}

/// Verification status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationStatus {
    Passed,
    Failed,
    Skipped,
}

pub mod recovery;
pub mod schedule;
pub mod verify;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta as Duration;

    #[test]
    fn test_backup_config() {
        let config = BackupConfig::new("my-vm", "backup-2024-01-01")
            .with_type(BackupType::Full)
            .without_encryption();

        assert_eq!(config.vm_name, "my-vm");
        assert_eq!(config.backup_type, BackupType::Full);
        assert!(!config.encryption_enabled);
    }

    #[test]
    fn test_backup_type() {
        assert_eq!(BackupType::Full.as_str(), "full");
        assert_eq!(BackupType::Incremental.as_str(), "incremental");
        assert_eq!(BackupType::Differential.as_str(), "differential");
    }

    #[test]
    fn test_compression_extension() {
        assert_eq!(CompressionType::Gzip.extension(), ".gz");
        assert_eq!(CompressionType::Zstd.extension(), ".zst");
        assert_eq!(CompressionType::None.extension(), "");
    }

    #[test]
    fn test_retention_policy() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.keep_daily, 7);
        assert_eq!(policy.keep_weekly, 4);

        let short_term = RetentionPolicy::short_term();
        assert_eq!(short_term.keep_daily, 3);
        assert_eq!(short_term.max_age_days, Some(30));

        let long_term = RetentionPolicy::long_term();
        assert_eq!(long_term.keep_yearly, 10);
        assert_eq!(long_term.max_age_days, None);
    }

    #[test]
    fn test_backup_status() {
        let mut status = BackupStatus::new("test-vm", "backup-001");
        assert_eq!(status.state, BackupState::Pending);
        assert!(!status.is_complete());

        status.state = BackupState::Completed;
        status.completed_at = Some(Utc::now());
        assert!(status.is_complete());
    }

    #[test]
    fn test_compression_ratio() {
        let mut status = BackupStatus::new("vm", "backup");
        status.size_bytes = 10_000_000_000; // 10GB
        status.compressed_size_bytes = 3_000_000_000; // 3GB

        assert_eq!(status.compression_ratio(), 30.0); // 30% of original
    }

    #[test]
    fn test_retention_should_keep() {
        let policy = RetentionPolicy {
            keep_daily: 7,
            keep_weekly: 4,
            keep_monthly: 12,
            keep_yearly: 5,
            max_age_days: Some(30),
        };

        let now = Utc::now();
        let recent = now - Duration::days(10);
        let old = now - Duration::days(40);

        assert!(policy.should_keep(recent, now));
        assert!(!policy.should_keep(old, now));
    }

    #[test]
    fn test_backup_target_default() {
        let target = BackupTarget::default();
        match target {
            BackupTarget::Local { path } => {
                assert_eq!(path, "/var/lib/veyron/backups");
            }
            _ => panic!("Expected Local target"),
        }
    }
}
