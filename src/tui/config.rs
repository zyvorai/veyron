// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// TUI Configuration System
// Inspired by GuestKit's configuration architecture

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main TUI configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TuiConfig {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
}

impl TuiConfig {
    /// Load configuration from default location (~/.config/vmrogue/tui.toml)
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::default_path()?;
        Self::load_from_path(&path)
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)?;
        let config: TuiConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to default location
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::default_path()?;
        self.save_to_path(&path)
    }

    /// Save configuration to a specific path
    pub fn save_to_path(&self, path: &PathBuf) -> anyhow::Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get default configuration path
    pub fn default_path() -> anyhow::Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Ok(config_dir.join("vmrogue").join("tui.toml"))
    }
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Theme name (e.g., "default", "dark", "light", "kubernetes")
    #[serde(default = "default_theme_name")]
    pub name: String,

    /// Custom color overrides (hex format)
    /// Example: primary = "#8856DE"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn default_theme_name() -> String {
    "default".to_string()
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: default_theme_name(),
            primary: None,
            success: None,
            error: None,
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Show splash screen on startup
    #[serde(default = "default_true")]
    pub show_splash: bool,

    /// Splash screen duration in milliseconds
    #[serde(default = "default_splash_duration")]
    pub splash_duration_ms: u64,

    /// Show stats bar
    #[serde(default = "default_true")]
    pub show_stats_bar: bool,

    /// Default view on startup
    #[serde(default = "default_view")]
    pub default_view: String,

    /// Auto-refresh interval in seconds (0 to disable)
    #[serde(default = "default_refresh_interval")]
    pub auto_refresh_interval: u64,

    /// Table formatting style
    #[serde(default = "default_table_style")]
    pub table_style: String,
}

fn default_true() -> bool {
    true
}

fn default_splash_duration() -> u64 {
    800
}

fn default_view() -> String {
    "dashboard".to_string()
}

fn default_refresh_interval() -> u64 {
    5
}

fn default_table_style() -> String {
    "rounded".to_string()
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_splash: default_true(),
            splash_duration_ms: default_splash_duration(),
            show_stats_bar: default_true(),
            default_view: default_view(),
            auto_refresh_interval: default_refresh_interval(),
            table_style: default_table_style(),
        }
    }
}

/// Behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    /// Show confirmation prompt before deleting VMs
    #[serde(default = "default_true")]
    pub confirm_delete: bool,

    /// Show confirmation prompt before stopping VMs
    #[serde(default = "default_false")]
    pub confirm_stop: bool,

    /// Show confirmation prompt before restarting VMs
    #[serde(default = "default_false")]
    pub confirm_restart: bool,

    /// Search case sensitive by default
    #[serde(default = "default_false")]
    pub search_case_sensitive: bool,

    /// Search regex mode by default
    #[serde(default = "default_false")]
    pub search_regex: bool,

    /// Scroll amount (lines per page up/down)
    #[serde(default = "default_scroll_amount")]
    pub scroll_amount: usize,

    /// Maximum items in lists
    #[serde(default = "default_max_list_items")]
    pub max_list_items: usize,
}

fn default_false() -> bool {
    false
}

fn default_scroll_amount() -> usize {
    10
}

fn default_max_list_items() -> usize {
    1000
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            confirm_delete: default_true(),
            confirm_stop: default_false(),
            confirm_restart: default_false(),
            search_case_sensitive: default_false(),
            search_regex: default_false(),
            scroll_amount: default_scroll_amount(),
            max_list_items: default_max_list_items(),
        }
    }
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    /// Enable vim mode (hjkl navigation)
    #[serde(default = "default_true")]
    pub vim_mode: bool,

    /// Enable quick jump menu (Ctrl+P)
    #[serde(default = "default_true")]
    pub quick_jump: bool,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            vim_mode: default_true(),
            quick_jump: default_true(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TuiConfig::default();
        assert_eq!(config.theme.name, "default");
        assert!(config.ui.show_splash);
        assert_eq!(config.ui.splash_duration_ms, 800);
        assert!(config.behavior.confirm_delete);
        assert!(config.keybindings.vim_mode);
    }

    #[test]
    fn test_config_serialization() {
        let config = TuiConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[theme]"));
        assert!(toml_str.contains("[ui]"));
        assert!(toml_str.contains("[behavior]"));
        assert!(toml_str.contains("[keybindings]"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            [theme]
            name = "dark"

            [ui]
            show_splash = false
            default_view = "vms"

            [behavior]
            confirm_delete = false

            [keybindings]
            vim_mode = false
        "#;

        let config: TuiConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.theme.name, "dark");
        assert!(!config.ui.show_splash);
        assert_eq!(config.ui.default_view, "vms");
        assert!(!config.behavior.confirm_delete);
        assert!(!config.keybindings.vim_mode);
    }
}
