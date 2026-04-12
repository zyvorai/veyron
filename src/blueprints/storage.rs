// Blueprint Storage - Filesystem persistence for custom blueprints

use super::Blueprint;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// BlueprintStorage handles reading and writing blueprints to the filesystem
pub struct BlueprintStorage {
    config_dir: PathBuf,
    read_only: bool,
}

impl BlueprintStorage {
    /// Create a new BlueprintStorage instance
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_blueprints_dir()?;

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create blueprints directory")?;
        }

        Ok(Self { config_dir, read_only: false })
    }

    /// Create an in-memory (no-op) storage for fallback scenarios
    pub fn in_memory() -> Self {
        Self {
            config_dir: PathBuf::from("/dev/null"),
            read_only: true,
        }
    }

    /// Get the blueprints directory path
    fn get_blueprints_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not determine config directory")?;

        Ok(config_dir.join("vmrogue").join("blueprints"))
    }

    /// Get the file path for a blueprint.
    /// Name is sanitized to prevent path traversal.
    fn blueprint_path(&self, name: &str) -> PathBuf {
        // Strip path separators to prevent directory traversal
        let safe_name: String = name
            .chars()
            .filter(|c| *c != '/' && *c != '\\' && *c != '.')
            .collect();
        let safe_name = if safe_name.is_empty() { "unnamed" } else { &safe_name };
        self.config_dir.join(format!("{}.yaml", safe_name))
    }

    /// Save a blueprint to disk
    pub fn save(&self, blueprint: &Blueprint) -> Result<()> {
        if self.read_only {
            anyhow::bail!("Blueprint storage is in read-only mode");
        }
        let path = self.blueprint_path(&blueprint.name);

        let yaml =
            serde_yml::to_string(blueprint).context("Failed to serialize blueprint to YAML")?;

        fs::write(&path, yaml)
            .with_context(|| format!("Failed to write blueprint to {}", path.display()))?;

        Ok(())
    }

    /// Load a blueprint from disk
    pub fn load(&self, name: &str) -> Result<Blueprint> {
        let path = self.blueprint_path(name);

        if !path.exists() {
            anyhow::bail!("Blueprint '{}' not found", name);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read blueprint from {}", path.display()))?;

        let blueprint: Blueprint = serde_yml::from_str(&content)
            .with_context(|| format!("Failed to parse blueprint from {}", path.display()))?;

        Ok(blueprint)
    }

    /// Load all custom blueprints from disk
    pub fn load_all(&self) -> Result<HashMap<String, Blueprint>> {
        let mut blueprints = HashMap::new();

        // If read-only or directory doesn't exist, return empty map
        if self.read_only || !self.config_dir.exists() {
            return Ok(blueprints);
        }

        let entries =
            fs::read_dir(&self.config_dir).context("Failed to read blueprints directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            // Only process .yaml files
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }

            // Get blueprint name from filename
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());

            if let Some(name) = name {
                match self.load(&name) {
                    Ok(blueprint) => {
                        blueprints.insert(name, blueprint);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load blueprint '{}': {}", name, e);
                    }
                }
            }
        }

        Ok(blueprints)
    }

    /// Delete a blueprint from disk
    pub fn delete(&self, name: &str) -> Result<()> {
        if self.read_only {
            anyhow::bail!("Blueprint storage is in read-only mode");
        }
        let path = self.blueprint_path(name);

        if !path.exists() {
            anyhow::bail!("Blueprint '{}' not found", name);
        }

        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete blueprint from {}", path.display()))?;

        Ok(())
    }

    /// Check if a blueprint exists on disk (public API for external consumers)
    #[allow(dead_code)]
    pub fn exists(&self, name: &str) -> bool {
        self.blueprint_path(name).exists()
    }

    /// List all custom blueprint names (public API for external consumers)
    #[allow(dead_code)]
    pub fn list_names(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();

        if !self.config_dir.exists() {
            return Ok(names);
        }

        let entries =
            fs::read_dir(&self.config_dir).context("Failed to read blueprints directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(name.to_string());
                }
            }
        }

        names.sort();
        Ok(names)
    }
}

impl Default for BlueprintStorage {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self::in_memory())
    }
}
