// State Persistence - Save and restore application state

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    pub version: String,
    pub saved_at: DateTime<Utc>,
    pub namespace: String,
    pub view_state: ViewState,
    pub preferences: UserPreferences,
    pub cached_data: CachedData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewState {
    pub active_view: String,
    pub selected_index: usize,
    pub sort_mode: String,
    pub filters: HashMap<String, String>,
    pub expanded_panels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: String,
    pub refresh_interval_secs: u64,
    pub show_stats_bar: bool,
    pub default_namespace: String,
    pub favorite_vms: Vec<String>,
    pub column_visibility: HashMap<String, bool>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            refresh_interval_secs: 5,
            show_stats_bar: true,
            default_namespace: "default".to_string(),
            favorite_vms: Vec::new(),
            column_visibility: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CachedData {
    pub vm_count: usize,
    pub last_known_vms: Vec<String>,
    pub cluster_name: Option<String>,
}

impl PersistedState {
    pub fn new(namespace: &str) -> Self {
        Self {
            version: "1.0".to_string(),
            saved_at: Utc::now(),
            namespace: namespace.to_string(),
            view_state: ViewState {
                active_view: "vms".to_string(),
                selected_index: 0,
                sort_mode: "default".to_string(),
                filters: HashMap::new(),
                expanded_panels: Vec::new(),
            },
            preferences: UserPreferences::default(),
            cached_data: CachedData::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::state_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::state_path()?;
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn state_path() -> anyhow::Result<PathBuf> {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("vmrogue");
        Ok(dir.join("state.json"))
    }

    pub fn exists() -> bool {
        Self::state_path().map(|p| p.exists()).unwrap_or(false)
    }

    pub fn delete() -> anyhow::Result<()> {
        let path = Self::state_path()?;
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
