// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Audit Trail - Comprehensive audit event tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utils::generate_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    pub entries: Vec<AuditEntry>,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub action: AuditAction,
    pub resource_type: String,
    pub resource_name: String,
    pub namespace: String,
    pub details: Value,
    pub ip_address: String,
    pub success: bool,
    pub severity: AuditSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    Start,
    Stop,
    Restart,
    Migrate,
    Clone,
    Snapshot,
    Restore,
    ConfigChange,
    Login,
    Logout,
    Export,
    Import,
    ScaleUp,
    ScaleDown,
    Approve,
    Reject,
    ScheduleChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum AuditSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl AuditTrail {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    pub fn record(&mut self, entry: AuditEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            let excess = self.entries.len().saturating_sub(self.max_entries);
            if excess > 0 {
                self.entries.drain(0..excess);
            }
        }
    }

    pub fn log_action(
        &mut self,
        user: &str,
        action: AuditAction,
        resource_type: &str,
        resource_name: &str,
        namespace: &str,
        success: bool,
    ) {
        let severity = match &action {
            AuditAction::Delete | AuditAction::Migrate => AuditSeverity::High,
            AuditAction::Create | AuditAction::Update | AuditAction::ConfigChange => {
                AuditSeverity::Medium
            }
            AuditAction::Start | AuditAction::Stop | AuditAction::Restart => AuditSeverity::Low,
            _ => AuditSeverity::Info,
        };
        self.record(AuditEntry {
            id: generate_id("audit", resource_name),
            timestamp: Utc::now(),
            user: user.to_string(),
            action,
            resource_type: resource_type.to_string(),
            resource_name: resource_name.to_string(),
            namespace: namespace.to_string(),
            details: Value::Null,
            ip_address: "127.0.0.1".to_string(),
            success,
            severity,
        });
    }

    pub fn query(
        &self,
        user: Option<&str>,
        action: Option<&AuditAction>,
        namespace: Option<&str>,
        min_severity: Option<&AuditSeverity>,
    ) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| {
                user.is_none_or(|u| e.user == u)
                    && action.is_none_or(|a| e.action == *a)
                    && namespace.is_none_or(|ns| e.namespace == ns)
                    && min_severity.is_none_or(|s| e.severity >= *s)
            })
            .collect()
    }

    pub fn recent(&self, limit: usize) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn by_resource(&self, resource_name: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.resource_name == resource_name)
            .collect()
    }

    pub fn failed_actions(&self) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| !e.success).collect()
    }

    /// Load audit trail from persistent storage.
    pub fn load() -> Self {
        let path = Self::storage_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Self>(&content) {
                    Ok(trail) => return trail,
                    Err(e) => log::warn!("Failed to parse audit trail: {}", e),
                },
                Err(e) => log::warn!("Failed to read audit trail: {}", e),
            }
        }
        Self::default()
    }

    /// Save audit trail to persistent storage.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::storage_path();
        crate::utils::atomic_write(&path, self)?;
        Ok(())
    }

    /// Path to the audit trail storage file.
    fn storage_path() -> std::path::PathBuf {
        crate::utils::data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from(".vmrogue"))
            .join("audit_trail.json")
    }

    pub fn stats(&self) -> AuditStats {
        AuditStats {
            total: self.entries.len(),
            successful: self.entries.iter().filter(|e| e.success).count(),
            failed: self.entries.iter().filter(|e| !e.success).count(),
            critical: self
                .entries
                .iter()
                .filter(|e| e.severity == AuditSeverity::Critical)
                .count(),
            high: self
                .entries
                .iter()
                .filter(|e| e.severity == AuditSeverity::High)
                .count(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub critical: usize,
    pub high: usize,
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new(10000)
    }
}
