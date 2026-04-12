// Log Management - Centralized log aggregation and analysis

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
    pub fields: HashMap<String, String>,
}

impl LogEntry {
    pub fn new(level: LogLevel, source: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            source: source.into(),
            message: Self::sanitize_log_message(&message.into()),
            fields: HashMap::new(),
        }
    }

    /// Sanitize log message by stripping control characters (except newline and tab)
    fn sanitize_log_message(msg: &str) -> String {
        msg.chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect()
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    pub fn is_error(&self) -> bool {
        matches!(self.level, LogLevel::Error | LogLevel::Critical)
    }

    pub fn is_warning(&self) -> bool {
        self.level == LogLevel::Warning
    }
}

/// Log level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Log query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub levels: Vec<LogLevel>,
    pub sources: Vec<String>,
    pub search_text: Option<String>,
    pub limit: usize,
}

impl LogQuery {
    pub fn new() -> Self {
        Self {
            start_time: None,
            end_time: None,
            levels: Vec::new(),
            sources: Vec::new(),
            search_text: None,
            limit: 100,
        }
    }

    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.levels.push(level);
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.sources.push(source.into());
        self
    }

    pub fn with_search(mut self, text: impl Into<String>) -> Self {
        self.search_text = Some(text.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn matches(&self, entry: &LogEntry) -> bool {
        // Check time range
        if let Some(start) = self.start_time {
            if entry.timestamp < start {
                return false;
            }
        }
        if let Some(end) = self.end_time {
            if entry.timestamp > end {
                return false;
            }
        }

        // Check level
        if !self.levels.is_empty() && !self.levels.contains(&entry.level) {
            return false;
        }

        // Check source
        if !self.sources.is_empty() && !self.sources.contains(&entry.source) {
            return false;
        }

        // Check search text
        if let Some(ref search) = self.search_text {
            if !entry.message.contains(search) {
                return false;
            }
        }

        true
    }
}

impl Default for LogQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Log aggregator
pub struct LogAggregator {
    entries: std::collections::VecDeque<LogEntry>,
    max_entries: usize,
}

impl LogAggregator {
    pub fn new() -> Self {
        Self {
            entries: std::collections::VecDeque::new(),
            max_entries: 10_000,
        }
    }

    pub fn add_entry(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn query(&self, query: &LogQuery) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| query.matches(e))
            .take(query.limit)
            .collect()
    }

    pub fn error_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_error()).count()
    }

    pub fn warning_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_warning()).count()
    }

    pub fn total_count(&self) -> usize {
        self.entries.len()
    }

    pub fn sources(&self) -> Vec<String> {
        let mut sources: Vec<String> = self.entries.iter().map(|e| e.source.clone()).collect();
        sources.sort();
        sources.dedup();
        sources
    }

    /// Get log statistics by level
    pub fn stats_by_level(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();

        for entry in &self.entries {
            *stats.entry(entry.level.to_string()).or_insert(0) += 1;
        }

        stats
    }

    /// Get recent errors
    pub fn recent_errors(&self, count: usize) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.is_error())
            .rev()
            .take(count)
            .collect()
    }
}

impl Default for LogAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Log pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPattern {
    pub pattern: String,
    pub count: usize,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub examples: Vec<String>,
}

impl LogPattern {
    pub fn new(pattern: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            pattern: pattern.into(),
            count: 0,
            first_seen: now,
            last_seen: now,
            examples: Vec::new(),
        }
    }

    pub fn record_occurrence(&mut self, message: impl Into<String>) {
        self.count += 1;
        self.last_seen = Utc::now();

        if self.examples.len() < 3 {
            self.examples.push(message.into());
        }
    }
}

/// Log analyzer
pub struct LogAnalyzer;

impl LogAnalyzer {
    /// Detect common log patterns
    pub fn detect_patterns(entries: &[LogEntry]) -> Vec<LogPattern> {
        let mut patterns: HashMap<String, LogPattern> = HashMap::new();

        for entry in entries {
            // Simplified pattern detection - extract error codes, keywords, etc.
            let pattern_key = Self::extract_pattern(&entry.message);

            patterns
                .entry(pattern_key.clone())
                .or_insert_with(|| LogPattern::new(pattern_key))
                .record_occurrence(&entry.message);
        }

        let mut result: Vec<LogPattern> = patterns.into_values().collect();
        result.sort_by(|a, b| b.count.cmp(&a.count));
        result
    }

    fn extract_pattern(message: &str) -> String {
        // Extract meaningful patterns by normalizing variable parts of log messages.
        // Replace UUIDs, IPs, numbers, timestamps, and hex strings with placeholders.
        let mut pattern = message.to_string();

        // Normalize common variable parts to create groupable patterns
        // Replace quoted strings
        while let Some(start) = pattern.find('"') {
            if start + 1 >= pattern.len() {
                break;
            }
            if let Some(end) = pattern[start + 1..].find('"') {
                pattern.replace_range(start..=start + 1 + end, "<STR>");
            } else {
                break;
            }
        }

        // Replace sequences of digits (timestamps, IDs, ports, etc.)
        let mut result = String::with_capacity(pattern.len());
        let mut in_digits = false;
        for ch in pattern.chars() {
            if ch.is_ascii_digit() {
                if !in_digits {
                    result.push_str("<N>");
                    in_digits = true;
                }
            } else {
                in_digits = false;
                result.push(ch);
            }
        }

        // Classify by severity keyword for grouping
        let prefix = if result.contains("ERROR") || result.contains("error") {
            "ERR:"
        } else if result.contains("WARN") || result.contains("warn") {
            "WARN:"
        } else if result.contains("FATAL") || result.contains("CRIT") {
            "CRIT:"
        } else {
            "INFO:"
        };

        // Truncate to keep patterns manageable (safe for multi-byte UTF-8)
        let truncated = if result.len() > 80 {
            let end = result
                .char_indices()
                .take_while(|(i, _)| *i <= 80)
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            format!("{}...", &result[..end])
        } else {
            result
        };

        format!("{}{}", prefix, truncated)
    }

    /// Find anomalies in log frequency
    pub fn detect_anomalies(entries: &[LogEntry], threshold: f64) -> Vec<String> {
        let mut anomalies = Vec::new();

        // Count logs per source
        let mut counts: HashMap<String, usize> = HashMap::new();
        for entry in entries {
            *counts.entry(entry.source.clone()).or_insert(0) += 1;
        }

        let avg = if !counts.is_empty() {
            counts.values().sum::<usize>() as f64 / counts.len() as f64
        } else {
            0.0
        };

        // Find sources with unusually high log volume
        for (source, count) in counts {
            if count as f64 > avg * threshold {
                anomalies.push(format!("{} ({}x normal)", source, count as f64 / avg));
            }
        }

        anomalies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry() {
        let entry = LogEntry::new(LogLevel::Error, "vm-controller", "Failed to start VM")
            .with_field("vm_name", "test-vm")
            .with_field("error_code", "E001");

        assert_eq!(entry.level, LogLevel::Error);
        assert!(entry.is_error());
        assert_eq!(entry.fields.get("vm_name"), Some(&"test-vm".to_string()));
    }

    #[test]
    fn test_log_levels() {
        assert!(LogLevel::Critical > LogLevel::Error);
        assert!(LogLevel::Error > LogLevel::Warning);
        assert!(LogLevel::Warning > LogLevel::Info);
        assert!(LogLevel::Info > LogLevel::Debug);
    }

    #[test]
    fn test_log_query() {
        let query = LogQuery::new()
            .with_level(LogLevel::Error)
            .with_source("api")
            .with_search("timeout")
            .with_limit(50);

        let matching = LogEntry::new(LogLevel::Error, "api", "Request timeout");
        assert!(query.matches(&matching));

        let non_matching = LogEntry::new(LogLevel::Info, "api", "Request timeout");
        assert!(!query.matches(&non_matching));
    }

    #[test]
    fn test_log_aggregator() {
        let mut aggregator = LogAggregator::new();

        aggregator.add_entry(LogEntry::new(LogLevel::Info, "api", "Request received"));
        aggregator.add_entry(LogEntry::new(LogLevel::Error, "api", "Connection failed"));
        aggregator.add_entry(LogEntry::new(LogLevel::Warning, "scheduler", "High load"));

        assert_eq!(aggregator.total_count(), 3);
        assert_eq!(aggregator.error_count(), 1);
        assert_eq!(aggregator.warning_count(), 1);
    }

    #[test]
    fn test_query_with_time_range() {
        let mut aggregator = LogAggregator::new();

        let now = Utc::now();
        let mut old_entry = LogEntry::new(LogLevel::Info, "api", "Old message");
        old_entry.timestamp = now - chrono::TimeDelta::hours(2);

        aggregator.add_entry(old_entry);
        aggregator.add_entry(LogEntry::new(LogLevel::Info, "api", "Recent message"));

        let query = LogQuery::new().with_time_range(
            now - chrono::TimeDelta::hours(1),
            now + chrono::TimeDelta::hours(1),
        );

        let results = aggregator.query(&query);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_stats_by_level() {
        let mut aggregator = LogAggregator::new();

        aggregator.add_entry(LogEntry::new(LogLevel::Info, "api", "Message 1"));
        aggregator.add_entry(LogEntry::new(LogLevel::Info, "api", "Message 2"));
        aggregator.add_entry(LogEntry::new(LogLevel::Error, "api", "Error"));

        let stats = aggregator.stats_by_level();
        assert_eq!(stats.get("INFO"), Some(&2));
        assert_eq!(stats.get("ERROR"), Some(&1));
    }

    #[test]
    fn test_recent_errors() {
        let mut aggregator = LogAggregator::new();

        aggregator.add_entry(LogEntry::new(LogLevel::Error, "api", "Error 1"));
        aggregator.add_entry(LogEntry::new(LogLevel::Info, "api", "Info"));
        aggregator.add_entry(LogEntry::new(LogLevel::Error, "api", "Error 2"));

        let errors = aggregator.recent_errors(5);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_log_sources() {
        let mut aggregator = LogAggregator::new();

        aggregator.add_entry(LogEntry::new(LogLevel::Info, "api", "Message"));
        aggregator.add_entry(LogEntry::new(LogLevel::Info, "scheduler", "Message"));
        aggregator.add_entry(LogEntry::new(LogLevel::Info, "api", "Message"));

        let sources = aggregator.sources();
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&"api".to_string()));
        assert!(sources.contains(&"scheduler".to_string()));
    }

    #[test]
    fn test_log_pattern() {
        let mut pattern = LogPattern::new("ERROR_PATTERN");

        pattern.record_occurrence("Error: Connection failed");
        pattern.record_occurrence("Error: Timeout");

        assert_eq!(pattern.count, 2);
        assert_eq!(pattern.examples.len(), 2);
    }

    #[test]
    fn test_detect_patterns() {
        let entries = vec![
            LogEntry::new(LogLevel::Error, "api", "ERROR: Failed"),
            LogEntry::new(LogLevel::Error, "api", "ERROR: Timeout"),
            LogEntry::new(LogLevel::Info, "api", "Request processed"),
        ];

        let patterns = LogAnalyzer::detect_patterns(&entries);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_detect_anomalies() {
        let mut entries = Vec::new();

        // Normal source
        for _ in 0..10 {
            entries.push(LogEntry::new(LogLevel::Info, "api", "Message"));
        }

        // Anomalous source (10x normal - clearly anomalous)
        for _ in 0..100 {
            entries.push(LogEntry::new(LogLevel::Info, "scheduler", "Message"));
        }

        let anomalies = LogAnalyzer::detect_anomalies(&entries, 1.5);
        assert!(!anomalies.is_empty());
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }
}
