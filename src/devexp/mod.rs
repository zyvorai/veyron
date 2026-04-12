use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod completions;
pub mod config_templates;
pub mod diff;
pub mod info;
pub mod init;

/// Developer experience configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevExpConfig {
    pub id: String,
    pub default_namespace: String,
    pub default_output_format: OutputPreference,
    pub editor: Option<String>,
    pub shell: ShellType,
    pub aliases: HashMap<String, String>,
    pub recent_vms: Vec<String>,
    pub max_recent: usize,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Output format preference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputPreference {
    Yaml,
    Json,
    Table,
}

impl std::fmt::Display for OutputPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputPreference::Yaml => write!(f, "yaml"),
            OutputPreference::Json => write!(f, "json"),
            OutputPreference::Table => write!(f, "table"),
        }
    }
}

/// Supported shell types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellType::Bash => write!(f, "bash"),
            ShellType::Zsh => write!(f, "zsh"),
            ShellType::Fish => write!(f, "fish"),
            ShellType::PowerShell => write!(f, "powershell"),
            ShellType::Elvish => write!(f, "elvish"),
        }
    }
}

impl ShellType {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(ShellType::Bash),
            "zsh" => Some(ShellType::Zsh),
            "fish" => Some(ShellType::Fish),
            "powershell" | "pwsh" => Some(ShellType::PowerShell),
            "elvish" => Some(ShellType::Elvish),
            _ => None,
        }
    }
}

impl DevExpConfig {
    pub fn new() -> Self {
        Self {
            id: format!("devexp-{}", Utc::now().timestamp()),
            default_namespace: "default".to_string(),
            default_output_format: OutputPreference::Table,
            editor: std::env::var("EDITOR").ok(),
            shell: detect_shell(),
            aliases: HashMap::new(),
            recent_vms: Vec::new(),
            max_recent: 10,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    pub fn set_namespace(&mut self, namespace: impl Into<String>) {
        self.default_namespace = namespace.into();
        self.last_updated = Utc::now();
    }

    pub fn set_output_format(&mut self, format: OutputPreference) {
        self.default_output_format = format;
        self.last_updated = Utc::now();
    }

    pub fn add_alias(&mut self, name: impl Into<String>, command: impl Into<String>) {
        self.aliases.insert(name.into(), command.into());
        self.last_updated = Utc::now();
    }

    pub fn remove_alias(&mut self, name: &str) -> bool {
        let removed = self.aliases.remove(name).is_some();
        if removed {
            self.last_updated = Utc::now();
        }
        removed
    }

    pub fn add_recent_vm(&mut self, vm_name: impl Into<String>) {
        let name = vm_name.into();
        self.recent_vms.retain(|v| v != &name);
        self.recent_vms.insert(0, name);
        if self.recent_vms.len() > self.max_recent {
            self.recent_vms.truncate(self.max_recent);
        }
        self.last_updated = Utc::now();
    }

    pub fn alias_count(&self) -> usize {
        self.aliases.len()
    }

    pub fn recent_count(&self) -> usize {
        self.recent_vms.len()
    }
}

impl Default for DevExpConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect current shell from environment
fn detect_shell() -> ShellType {
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("zsh") {
            return ShellType::Zsh;
        } else if shell.contains("fish") {
            return ShellType::Fish;
        } else if shell.contains("elvish") {
            return ShellType::Elvish;
        }
    }
    ShellType::Bash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devexp_config_new() {
        let config = DevExpConfig::new();
        assert_eq!(config.default_namespace, "default");
        assert_eq!(config.default_output_format, OutputPreference::Table);
        assert_eq!(config.max_recent, 10);
        assert!(config.recent_vms.is_empty());
        assert!(config.aliases.is_empty());
    }

    #[test]
    fn test_set_namespace() {
        let mut config = DevExpConfig::new();
        config.set_namespace("production");
        assert_eq!(config.default_namespace, "production");
    }

    #[test]
    fn test_set_output_format() {
        let mut config = DevExpConfig::new();
        config.set_output_format(OutputPreference::Json);
        assert_eq!(config.default_output_format, OutputPreference::Json);
    }

    #[test]
    fn test_add_alias() {
        let mut config = DevExpConfig::new();
        config.add_alias("ls", "list --output table");
        assert_eq!(config.alias_count(), 1);
        assert_eq!(config.aliases.get("ls").unwrap(), "list --output table");
    }

    #[test]
    fn test_remove_alias() {
        let mut config = DevExpConfig::new();
        config.add_alias("ls", "list");
        assert!(config.remove_alias("ls"));
        assert!(!config.remove_alias("nonexistent"));
        assert_eq!(config.alias_count(), 0);
    }

    #[test]
    fn test_add_recent_vm() {
        let mut config = DevExpConfig::new();
        config.add_recent_vm("vm-1");
        config.add_recent_vm("vm-2");
        config.add_recent_vm("vm-3");

        assert_eq!(config.recent_count(), 3);
        assert_eq!(config.recent_vms[0], "vm-3");
        assert_eq!(config.recent_vms[1], "vm-2");
    }

    #[test]
    fn test_recent_vm_dedup() {
        let mut config = DevExpConfig::new();
        config.add_recent_vm("vm-1");
        config.add_recent_vm("vm-2");
        config.add_recent_vm("vm-1");

        assert_eq!(config.recent_count(), 2);
        assert_eq!(config.recent_vms[0], "vm-1");
    }

    #[test]
    fn test_recent_vm_max_limit() {
        let mut config = DevExpConfig::new();
        config.max_recent = 3;

        config.add_recent_vm("vm-1");
        config.add_recent_vm("vm-2");
        config.add_recent_vm("vm-3");
        config.add_recent_vm("vm-4");

        assert_eq!(config.recent_count(), 3);
        assert_eq!(config.recent_vms[0], "vm-4");
    }

    #[test]
    fn test_output_preference_display() {
        assert_eq!(OutputPreference::Yaml.to_string(), "yaml");
        assert_eq!(OutputPreference::Json.to_string(), "json");
        assert_eq!(OutputPreference::Table.to_string(), "table");
    }

    #[test]
    fn test_shell_type_display() {
        assert_eq!(ShellType::Bash.to_string(), "bash");
        assert_eq!(ShellType::Zsh.to_string(), "zsh");
        assert_eq!(ShellType::Fish.to_string(), "fish");
        assert_eq!(ShellType::PowerShell.to_string(), "powershell");
        assert_eq!(ShellType::Elvish.to_string(), "elvish");
    }

    #[test]
    fn test_shell_type_from_str() {
        assert_eq!(ShellType::parse("bash"), Some(ShellType::Bash));
        assert_eq!(ShellType::parse("zsh"), Some(ShellType::Zsh));
        assert_eq!(ShellType::parse("fish"), Some(ShellType::Fish));
        assert_eq!(ShellType::parse("powershell"), Some(ShellType::PowerShell));
        assert_eq!(ShellType::parse("pwsh"), Some(ShellType::PowerShell));
        assert_eq!(ShellType::parse("elvish"), Some(ShellType::Elvish));
        assert_eq!(ShellType::parse("unknown"), None);
    }

    #[test]
    fn test_output_preference_equality() {
        assert_eq!(OutputPreference::Yaml, OutputPreference::Yaml);
        assert_ne!(OutputPreference::Yaml, OutputPreference::Json);
    }

    #[test]
    fn test_shell_type_equality() {
        assert_eq!(ShellType::Bash, ShellType::Bash);
        assert_ne!(ShellType::Bash, ShellType::Zsh);
    }
}
