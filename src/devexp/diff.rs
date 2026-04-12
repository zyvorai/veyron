use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration difference type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffType {
    Added,
    Removed,
    Modified,
    Unchanged,
}

impl std::fmt::Display for DiffType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffType::Added => write!(f, "added"),
            DiffType::Removed => write!(f, "removed"),
            DiffType::Modified => write!(f, "modified"),
            DiffType::Unchanged => write!(f, "unchanged"),
        }
    }
}

/// A single difference entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub path: String,
    pub diff_type: DiffType,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

impl DiffEntry {
    pub fn added(path: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            diff_type: DiffType::Added,
            old_value: None,
            new_value: Some(value.into()),
        }
    }

    pub fn removed(path: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            diff_type: DiffType::Removed,
            old_value: Some(value.into()),
            new_value: None,
        }
    }

    pub fn modified(
        path: impl Into<String>,
        old: impl Into<String>,
        new: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            diff_type: DiffType::Modified,
            old_value: Some(old.into()),
            new_value: Some(new.into()),
        }
    }

    pub fn unchanged(path: impl Into<String>, value: impl Into<String>) -> Self {
        let val = value.into();
        Self {
            path: path.into(),
            diff_type: DiffType::Unchanged,
            old_value: Some(val.clone()),
            new_value: Some(val),
        }
    }

    pub fn is_changed(&self) -> bool {
        self.diff_type != DiffType::Unchanged
    }

    pub fn symbol(&self) -> &str {
        match self.diff_type {
            DiffType::Added => "+",
            DiffType::Removed => "-",
            DiffType::Modified => "~",
            DiffType::Unchanged => " ",
        }
    }
}

/// Configuration diff result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDiff {
    pub source_name: String,
    pub target_name: String,
    pub entries: Vec<DiffEntry>,
    pub summary: DiffSummary,
}

/// Diff summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_fields: usize,
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub unchanged: usize,
}

impl DiffSummary {
    pub fn has_changes(&self) -> bool {
        self.added > 0 || self.removed > 0 || self.modified > 0
    }

    pub fn change_count(&self) -> usize {
        self.added + self.removed + self.modified
    }

    pub fn change_percentage(&self) -> f64 {
        if self.total_fields == 0 {
            return 0.0;
        }
        (self.change_count() as f64 / self.total_fields as f64) * 100.0
    }
}

/// Configuration differ
pub struct ConfigDiffer;

impl ConfigDiffer {
    /// Compare two YAML configuration strings
    pub fn diff_yaml(
        source_name: &str,
        source: &str,
        target_name: &str,
        target: &str,
    ) -> ConfigDiff {
        let source_map = Self::flatten_yaml(source);
        let target_map = Self::flatten_yaml(target);

        let mut entries = Vec::new();

        // Check for modified and removed fields
        for (key, old_value) in &source_map {
            if let Some(new_value) = target_map.get(key) {
                if old_value == new_value {
                    entries.push(DiffEntry::unchanged(key.clone(), old_value.clone()));
                } else {
                    entries.push(DiffEntry::modified(
                        key.clone(),
                        old_value.clone(),
                        new_value.clone(),
                    ));
                }
            } else {
                entries.push(DiffEntry::removed(key.clone(), old_value.clone()));
            }
        }

        // Check for added fields
        for (key, new_value) in &target_map {
            if !source_map.contains_key(key) {
                entries.push(DiffEntry::added(key.clone(), new_value.clone()));
            }
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));

        let summary = DiffSummary {
            total_fields: entries.len(),
            added: entries
                .iter()
                .filter(|e| e.diff_type == DiffType::Added)
                .count(),
            removed: entries
                .iter()
                .filter(|e| e.diff_type == DiffType::Removed)
                .count(),
            modified: entries
                .iter()
                .filter(|e| e.diff_type == DiffType::Modified)
                .count(),
            unchanged: entries
                .iter()
                .filter(|e| e.diff_type == DiffType::Unchanged)
                .count(),
        };

        ConfigDiff {
            source_name: source_name.to_string(),
            target_name: target_name.to_string(),
            entries,
            summary,
        }
    }

    /// Flatten a YAML string into key-value pairs with dotted paths
    fn flatten_yaml(yaml_str: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();

        for line in yaml_str.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = trimmed.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                if !value.is_empty() {
                    map.insert(key, value);
                }
            }
        }

        map
    }

    /// Generate a formatted diff output
    pub fn format_diff(diff: &ConfigDiff, show_unchanged: bool) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "--- {}\n+++ {}\n\n",
            diff.source_name, diff.target_name
        ));

        for entry in &diff.entries {
            if !show_unchanged && !entry.is_changed() {
                continue;
            }

            match entry.diff_type {
                DiffType::Added => {
                    output.push_str(&format!(
                        "+ {}: {}\n",
                        entry.path,
                        entry.new_value.as_deref().unwrap_or("")
                    ));
                }
                DiffType::Removed => {
                    output.push_str(&format!(
                        "- {}: {}\n",
                        entry.path,
                        entry.old_value.as_deref().unwrap_or("")
                    ));
                }
                DiffType::Modified => {
                    output.push_str(&format!(
                        "~ {}: {} -> {}\n",
                        entry.path,
                        entry.old_value.as_deref().unwrap_or(""),
                        entry.new_value.as_deref().unwrap_or("")
                    ));
                }
                DiffType::Unchanged => {
                    output.push_str(&format!(
                        "  {}: {}\n",
                        entry.path,
                        entry.old_value.as_deref().unwrap_or("")
                    ));
                }
            }
        }

        output.push_str(&format!(
            "\nSummary: {} added, {} removed, {} modified, {} unchanged\n",
            diff.summary.added, diff.summary.removed, diff.summary.modified, diff.summary.unchanged,
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_entry_added() {
        let entry = DiffEntry::added("cpu.cores", "4");
        assert_eq!(entry.diff_type, DiffType::Added);
        assert!(entry.old_value.is_none());
        assert_eq!(entry.new_value.as_deref(), Some("4"));
        assert!(entry.is_changed());
        assert_eq!(entry.symbol(), "+");
    }

    #[test]
    fn test_diff_entry_removed() {
        let entry = DiffEntry::removed("cpu.cores", "2");
        assert_eq!(entry.diff_type, DiffType::Removed);
        assert_eq!(entry.old_value.as_deref(), Some("2"));
        assert!(entry.new_value.is_none());
        assert!(entry.is_changed());
        assert_eq!(entry.symbol(), "-");
    }

    #[test]
    fn test_diff_entry_modified() {
        let entry = DiffEntry::modified("memory", "4Gi", "8Gi");
        assert_eq!(entry.diff_type, DiffType::Modified);
        assert_eq!(entry.old_value.as_deref(), Some("4Gi"));
        assert_eq!(entry.new_value.as_deref(), Some("8Gi"));
        assert!(entry.is_changed());
        assert_eq!(entry.symbol(), "~");
    }

    #[test]
    fn test_diff_entry_unchanged() {
        let entry = DiffEntry::unchanged("name", "test-vm");
        assert_eq!(entry.diff_type, DiffType::Unchanged);
        assert!(!entry.is_changed());
        assert_eq!(entry.symbol(), " ");
    }

    #[test]
    fn test_diff_type_display() {
        assert_eq!(DiffType::Added.to_string(), "added");
        assert_eq!(DiffType::Removed.to_string(), "removed");
        assert_eq!(DiffType::Modified.to_string(), "modified");
        assert_eq!(DiffType::Unchanged.to_string(), "unchanged");
    }

    #[test]
    fn test_diff_summary_has_changes() {
        let summary = DiffSummary {
            total_fields: 5,
            added: 1,
            removed: 0,
            modified: 0,
            unchanged: 4,
        };
        assert!(summary.has_changes());

        let no_changes = DiffSummary {
            total_fields: 5,
            added: 0,
            removed: 0,
            modified: 0,
            unchanged: 5,
        };
        assert!(!no_changes.has_changes());
    }

    #[test]
    fn test_diff_summary_change_count() {
        let summary = DiffSummary {
            total_fields: 10,
            added: 2,
            removed: 1,
            modified: 3,
            unchanged: 4,
        };
        assert_eq!(summary.change_count(), 6);
    }

    #[test]
    fn test_diff_summary_change_percentage() {
        let summary = DiffSummary {
            total_fields: 10,
            added: 2,
            removed: 1,
            modified: 2,
            unchanged: 5,
        };
        assert!((summary.change_percentage() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_diff_summary_empty() {
        let summary = DiffSummary {
            total_fields: 0,
            added: 0,
            removed: 0,
            modified: 0,
            unchanged: 0,
        };
        assert_eq!(summary.change_percentage(), 0.0);
    }

    #[test]
    fn test_diff_yaml_identical() {
        let yaml = "name: test-vm\ncpu: 2\nmemory: 4Gi";
        let diff = ConfigDiffer::diff_yaml("a.yaml", yaml, "b.yaml", yaml);

        assert!(!diff.summary.has_changes());
        assert_eq!(diff.summary.unchanged, 3);
    }

    #[test]
    fn test_diff_yaml_modified() {
        let source = "name: test-vm\ncpu: 2\nmemory: 4Gi";
        let target = "name: test-vm\ncpu: 4\nmemory: 8Gi";
        let diff = ConfigDiffer::diff_yaml("a.yaml", source, "b.yaml", target);

        assert!(diff.summary.has_changes());
        assert_eq!(diff.summary.modified, 2);
        assert_eq!(diff.summary.unchanged, 1);
    }

    #[test]
    fn test_diff_yaml_added() {
        let source = "name: test-vm\ncpu: 2";
        let target = "name: test-vm\ncpu: 2\nmemory: 4Gi";
        let diff = ConfigDiffer::diff_yaml("a.yaml", source, "b.yaml", target);

        assert_eq!(diff.summary.added, 1);
        assert_eq!(diff.summary.unchanged, 2);
    }

    #[test]
    fn test_diff_yaml_removed() {
        let source = "name: test-vm\ncpu: 2\nmemory: 4Gi";
        let target = "name: test-vm\ncpu: 2";
        let diff = ConfigDiffer::diff_yaml("a.yaml", source, "b.yaml", target);

        assert_eq!(diff.summary.removed, 1);
        assert_eq!(diff.summary.unchanged, 2);
    }

    #[test]
    fn test_format_diff_changes_only() {
        let source = "name: test-vm\ncpu: 2\nmemory: 4Gi";
        let target = "name: test-vm\ncpu: 4\nmemory: 4Gi";
        let diff = ConfigDiffer::diff_yaml("a.yaml", source, "b.yaml", target);

        let output = ConfigDiffer::format_diff(&diff, false);
        assert!(output.contains("~ cpu:"));
        assert!(!output.contains("  name:"));
    }

    #[test]
    fn test_format_diff_with_unchanged() {
        let source = "name: test-vm\ncpu: 2";
        let target = "name: test-vm\ncpu: 4";
        let diff = ConfigDiffer::diff_yaml("a.yaml", source, "b.yaml", target);

        let output = ConfigDiffer::format_diff(&diff, true);
        assert!(output.contains("~ cpu:"));
        assert!(output.contains("  name:"));
    }

    #[test]
    fn test_format_diff_header() {
        let diff = ConfigDiffer::diff_yaml("old.yaml", "cpu: 2", "new.yaml", "cpu: 4");
        let output = ConfigDiffer::format_diff(&diff, false);
        assert!(output.contains("--- old.yaml"));
        assert!(output.contains("+++ new.yaml"));
    }
}
