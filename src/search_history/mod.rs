// Search History - Track and manage search queries

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistory {
    pub entries: Vec<SearchEntry>,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEntry {
    pub query: String,
    pub timestamp: DateTime<Utc>,
    pub result_count: usize,
    pub duration_ms: u64,
}

impl SearchHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    pub fn add(&mut self, query: String, result_count: usize, duration_ms: u64) {
        self.entries.push(SearchEntry {
            query,
            timestamp: Utc::now(),
            result_count,
            duration_ms,
        });
        if self.entries.len() > self.max_entries {
            let excess = self.entries.len().saturating_sub(self.max_entries);
            if excess > 0 {
                self.entries.drain(0..excess);
            }
        }
    }

    pub fn recent(&self, limit: usize) -> Vec<&SearchEntry> {
        self.entries.iter().take(limit).collect()
    }

    pub fn search(&self, pattern: &str) -> Vec<&SearchEntry> {
        let p = pattern.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.query.to_lowercase().contains(&p))
            .collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn most_frequent(&self, limit: usize) -> Vec<(String, usize)> {
        let mut counts = std::collections::HashMap::new();
        for entry in &self.entries {
            *counts.entry(entry.query.clone()).or_insert(0) += 1;
        }
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(limit);
        sorted
    }
}

impl Default for SearchHistory {
    fn default() -> Self {
        Self::new(100)
    }
}
