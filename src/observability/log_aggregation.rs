// Log Aggregation - Multi-source log streaming and filtering

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAggregator {
    pub sources: Vec<LogSource>,
    pub entries: Vec<LogEntry>,
    pub config: LogAggregationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSource {
    pub name: String,
    pub source_type: LogSourceType,
    pub enabled: bool,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogSourceType {
    Pod,
    Node,
    VM,
    Container,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub level: LogLevel,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAggregationConfig {
    pub max_entries: usize,
    pub retention_hours: u64,
    pub default_level: LogLevel,
}

impl Default for LogAggregationConfig {
    fn default() -> Self {
        Self {
            max_entries: 50000,
            retention_hours: 720,
            default_level: LogLevel::Info,
        }
    }
}

impl LogAggregator {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            entries: Vec::new(),
            config: LogAggregationConfig::default(),
        }
    }

    pub fn add_source(&mut self, source: LogSource) {
        self.sources.push(source);
    }

    pub fn add_entry(&mut self, entry: LogEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.config.max_entries {
            let drain_count = self.entries.len().min(1000);
            self.entries.drain(0..drain_count);
        }
    }

    pub fn query(
        &self,
        source: Option<&str>,
        level: Option<&LogLevel>,
        search: Option<&str>,
        limit: usize,
    ) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .rev()
            .filter(|e| source.is_none_or(|s| e.source == s))
            .filter(|e| level.is_none_or(|l| e.level >= *l))
            .filter(|e| search.is_none_or(|s| e.message.to_lowercase().contains(&s.to_lowercase())))
            .take(limit)
            .collect()
    }

    pub fn stats(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for entry in &self.entries {
            *counts.entry(format!("{:?}", entry.level)).or_insert(0) += 1;
        }
        counts
    }

    pub fn error_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.level >= LogLevel::Error)
            .count()
    }
}

impl Default for LogAggregator {
    fn default() -> Self {
        Self::new()
    }
}
