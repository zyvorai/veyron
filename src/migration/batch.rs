// Batch Migration - Bulk migration orchestration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMigration {
    pub id: String,
    pub name: String,
    pub items: Vec<BatchItem>,
    pub status: BatchStatus,
    pub config: BatchConfig,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    pub vm_name: String,
    pub namespace: String,
    pub source_node: String,
    pub target_node: String,
    pub status: ItemStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BatchStatus {
    Planning,
    Ready,
    InProgress,
    Completed,
    PartiallyCompleted,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ItemStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    pub max_parallel: usize,
    pub continue_on_error: bool,
    pub pause_between_secs: u64,
    pub dry_run: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_parallel: 3,
            continue_on_error: true,
            pause_between_secs: 30,
            dry_run: false,
        }
    }
}

impl BatchMigration {
    pub fn new(name: &str) -> Self {
        Self {
            id: format!("batch-{}", Utc::now().timestamp_micros()),
            name: name.to_string(),
            items: Vec::new(),
            status: BatchStatus::Planning,
            config: BatchConfig::default(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn add_item(&mut self, item: BatchItem) {
        self.items.push(item);
    }

    /// Transition from Planning to Ready status
    pub fn mark_ready(&mut self) -> anyhow::Result<()> {
        if self.status != BatchStatus::Planning {
            anyhow::bail!(
                "Batch must be in Planning status to mark ready, currently: {:?}",
                self.status
            );
        }
        if self.items.is_empty() {
            anyhow::bail!("Cannot mark batch as ready with no items");
        }
        self.status = BatchStatus::Ready;
        Ok(())
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.items.is_empty() {
            anyhow::bail!("Cannot start batch migration with no items");
        }
        if self.status != BatchStatus::Ready {
            anyhow::bail!(
                "Batch must be in Ready status to start, currently: {:?}",
                self.status
            );
        }
        self.status = BatchStatus::InProgress;
        self.started_at = Some(Utc::now());
        Ok(())
    }

    pub fn complete(&mut self) {
        let success = self
            .items
            .iter()
            .filter(|i| i.status == ItemStatus::Completed)
            .count();
        let total = self.items.len();
        self.status = if success == total {
            BatchStatus::Completed
        } else if success == 0 {
            BatchStatus::Failed
        } else {
            BatchStatus::PartiallyCompleted
        };
        self.completed_at = Some(Utc::now());
    }

    pub fn cancel(&mut self) {
        self.status = BatchStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    pub fn progress(&self) -> (usize, usize) {
        let done = self
            .items
            .iter()
            .filter(|i| !matches!(i.status, ItemStatus::Pending | ItemStatus::InProgress))
            .count();
        (done, self.items.len())
    }

    pub fn success_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == ItemStatus::Completed)
            .count()
    }
    pub fn failed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == ItemStatus::Failed)
            .count()
    }
}
