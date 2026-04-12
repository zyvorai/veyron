// Retention Policy Enforcement
// Automatically delete old snapshots based on retention policies

use super::{RetentionPolicy, SnapshotManager};
use anyhow::Result;
use chrono::{TimeDelta as Duration, Utc};

/// Retention policy enforcer
pub struct RetentionEnforcer {
    manager: SnapshotManager,
}

impl RetentionEnforcer {
    /// Create a new retention enforcer
    pub fn new(manager: SnapshotManager) -> Self {
        Self { manager }
    }

    /// Apply retention policy to snapshots for a VM
    pub async fn apply_policy(
        &self,
        vm_name: &str,
        policy: &RetentionPolicy,
    ) -> Result<Vec<String>> {
        let mut snapshots = self.manager.list_snapshots_for_vm(vm_name).await?;
        let mut deleted = Vec::new();

        // Sort by creation time, newest first
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Track the actual remaining count across both enforcement loops
        let mut remaining_count = snapshots.len();

        // Apply max_snapshots policy
        if let Some(max_snapshots) = policy.max_snapshots {
            if remaining_count > max_snapshots as usize {
                // Delete oldest snapshots beyond the limit
                for snapshot in snapshots.iter().skip(max_snapshots as usize) {
                    // Don't delete if we need to keep last N
                    if let Some(keep_last_n) = policy.keep_last_n {
                        if remaining_count <= keep_last_n as usize {
                            break;
                        }
                    }

                    // Skip snapshots that may be in use by active restores
                    if self.manager.is_snapshot_in_use(&snapshot.name).await.unwrap_or(true) {
                        log::warn!("Skipping deletion of snapshot '{}': may be in use by an active restore", snapshot.name);
                        continue;
                    }

                    match self.manager.delete_snapshot(&snapshot.name).await {
                        Ok(_) => {
                            deleted.push(snapshot.name.clone());
                            remaining_count -= 1;
                        }
                        Err(e) => {
                            log::error!("Failed to delete snapshot '{}': {}", snapshot.name, e);
                            // Continue trying to delete other snapshots
                        }
                    }
                }
            }
        }

        // Apply max_age_days policy
        if let Some(max_age_days) = policy.max_age_days {
            let cutoff_date = Utc::now() - Duration::days(max_age_days as i64);

            for snapshot in &snapshots {
                // Skip already deleted
                if deleted.contains(&snapshot.name) {
                    continue;
                }

                // Don't delete if we need to keep last N
                if let Some(keep_last_n) = policy.keep_last_n {
                    if remaining_count <= keep_last_n as usize {
                        break;
                    }
                }

                // Check age
                if let Some(created_at) = snapshot.created_at {
                    if created_at < cutoff_date {
                        // Skip snapshots that may be in use by active restores
                        if self.manager.is_snapshot_in_use(&snapshot.name).await.unwrap_or(true) {
                            log::warn!("Skipping deletion of snapshot '{}': may be in use by an active restore", snapshot.name);
                            continue;
                        }

                        match self.manager.delete_snapshot(&snapshot.name).await {
                            Ok(_) => {
                                deleted.push(snapshot.name.clone());
                                remaining_count -= 1;
                            }
                            Err(e) => {
                                log::error!("Failed to delete snapshot '{}': {}", snapshot.name, e);
                                // Continue trying to delete other snapshots
                            }
                        }
                    }
                }
            }
        }

        Ok(deleted)
    }

    /// Apply retention policy to all snapshots in namespace
    pub async fn apply_policy_global(&self, policy: &RetentionPolicy) -> Result<Vec<String>> {
        let snapshots = self.manager.list_all_snapshots().await?;
        let mut deleted = Vec::new();

        // Group by VM name
        let mut vm_snapshots: std::collections::HashMap<String, Vec<_>> =
            std::collections::HashMap::new();
        for snapshot in snapshots {
            vm_snapshots
                .entry(snapshot.vm_name.clone())
                .or_insert_with(Vec::new)
                .push(snapshot);
        }

        // Apply policy to each VM's snapshots
        for (vm_name, _) in vm_snapshots {
            let vm_deleted = self.apply_policy(&vm_name, policy).await?;
            deleted.extend(vm_deleted);
        }

        Ok(deleted)
    }

    /// Preview which snapshots would be deleted (dry run)
    pub async fn preview_policy(
        &self,
        vm_name: &str,
        policy: &RetentionPolicy,
    ) -> Result<Vec<String>> {
        let mut snapshots = self.manager.list_snapshots_for_vm(vm_name).await?;
        let mut would_delete = Vec::new();

        // Sort by creation time, newest first
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Check max_snapshots policy
        if let Some(max_snapshots) = policy.max_snapshots {
            if snapshots.len() > max_snapshots as usize {
                for snapshot in snapshots.iter().skip(max_snapshots as usize) {
                    if let Some(keep_last_n) = policy.keep_last_n {
                        let remaining = snapshots.len() - would_delete.len();
                        if remaining <= keep_last_n as usize {
                            continue;
                        }
                    }
                    would_delete.push(snapshot.name.clone());
                }
            }
        }

        // Check max_age_days policy
        if let Some(max_age_days) = policy.max_age_days {
            let cutoff_date = Utc::now() - Duration::days(max_age_days as i64);

            for snapshot in &snapshots {
                if would_delete.contains(&snapshot.name) {
                    continue;
                }

                if let Some(created_at) = snapshot.created_at {
                    if created_at < cutoff_date {
                        if let Some(keep_last_n) = policy.keep_last_n {
                            let remaining = snapshots.len() - would_delete.len();
                            if remaining <= keep_last_n as usize {
                                continue;
                            }
                        }
                        would_delete.push(snapshot.name.clone());
                    }
                }
            }
        }

        Ok(would_delete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::snapshots::SnapshotConfig;

    #[tokio::test]
    async fn test_retention_enforcer_creation() {
        let manager = match SnapshotManager::new("default").await {
            Ok(m) => m,
            Err(_) => return, // Skip test if no cluster available
        };
        let enforcer = RetentionEnforcer::new(manager);

        let policy = RetentionPolicy {
            max_snapshots: Some(5),
            max_age_days: Some(30),
            keep_last_n: Some(2),
        };

        // Preview should not fail
        let result = enforcer.preview_policy("test-vm", &policy).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retention_policy_preview() {
        let manager = match SnapshotManager::new("default").await {
            Ok(m) => m,
            Err(_) => return, // Skip test if no cluster available
        };
        let enforcer = RetentionEnforcer::new(manager);

        let policy = RetentionPolicy {
            max_snapshots: Some(1),
            max_age_days: None,
            keep_last_n: Some(1),
        };

        let would_delete = enforcer.preview_policy("my-vm", &policy).await.unwrap();
        // Should preview some deletions based on the policy
        // (actual behavior depends on mock data)
        assert!(would_delete.len() <= 1);
    }
}
