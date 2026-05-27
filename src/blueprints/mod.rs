// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Multi-VM Blueprints - Deploy complete application stacks with one command
// This is an innovative feature for deploying complex topologies

mod builtin;
mod storage;
pub mod validator;

pub use validator::{check_circular_dependencies, resolve_deployment_order, validate_blueprint};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;

pub static BLUEPRINTS: LazyLock<RwLock<BlueprintManager>> =
    LazyLock::new(|| match BlueprintManager::new() {
        Ok(manager) => RwLock::new(manager),
        Err(e) => {
            log::error!(
                "Failed to initialize BlueprintManager: {}. Using empty manager.",
                e
            );
            RwLock::new(BlueprintManager::empty())
        }
    });

/// VMSpec defines a single VM in a blueprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMSpec {
    pub name: String,
    pub template: String,
    pub profile: Option<String>,
    pub cpu: Option<u32>,
    pub memory: Option<String>,
    pub disk_size: Option<String>,
    pub depends_on: Vec<String>,
    pub labels: HashMap<String, String>,
}

/// Blueprint defines a multi-VM deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub description: String,
    pub vms: Vec<VMSpec>,
    pub tags: Vec<String>,
}

/// BlueprintManager manages all available blueprints (builtin and custom)
pub struct BlueprintManager {
    builtin_blueprints: HashMap<String, Blueprint>,
    custom_blueprints: HashMap<String, Blueprint>,
    storage: storage::BlueprintStorage,
}

impl BlueprintManager {
    pub fn new() -> Result<Self> {
        let builtin_blueprints = builtin::builtin_blueprints();
        let storage = storage::BlueprintStorage::new()?;
        let custom_blueprints = storage.load_all()?;

        Ok(Self {
            builtin_blueprints,
            custom_blueprints,
            storage,
        })
    }

    /// Create an empty manager (fallback when initialization fails)
    fn empty() -> Self {
        Self {
            builtin_blueprints: builtin::builtin_blueprints(),
            custom_blueprints: HashMap::new(),
            storage: storage::BlueprintStorage::in_memory(),
        }
    }

    /// Get a blueprint by name (checks custom first, then builtin)
    pub fn get(&self, name: &str) -> Option<Blueprint> {
        self.custom_blueprints
            .get(name)
            .or_else(|| self.builtin_blueprints.get(name))
            .cloned()
    }

    /// List all blueprints (builtin + custom)
    pub fn list(&self) -> Vec<Blueprint> {
        let mut blueprints: Vec<_> = self
            .builtin_blueprints
            .values()
            .chain(self.custom_blueprints.values())
            .cloned()
            .collect();

        blueprints.sort_by(|a, b| a.name.cmp(&b.name));
        blueprints
    }

    /// Check if a blueprint exists
    pub fn exists(&self, name: &str) -> bool {
        self.custom_blueprints.contains_key(name) || self.builtin_blueprints.contains_key(name)
    }

    /// Check if a blueprint is builtin
    pub fn is_builtin(&self, name: &str) -> bool {
        self.builtin_blueprints.contains_key(name)
    }

    /// Create a custom blueprint
    pub fn create_custom(&mut self, blueprint: Blueprint) -> Result<()> {
        // Validate the blueprint
        validator::validate_blueprint(&blueprint)?;

        // Check if blueprint already exists
        if self.exists(&blueprint.name) {
            anyhow::bail!("Blueprint '{}' already exists", blueprint.name);
        }

        // Save to disk
        self.storage.save(&blueprint)?;

        // Add to in-memory map
        self.custom_blueprints
            .insert(blueprint.name.clone(), blueprint);

        Ok(())
    }

    /// Update a custom blueprint
    pub fn update_custom(&mut self, blueprint: Blueprint) -> Result<()> {
        // Validate the blueprint
        validator::validate_blueprint(&blueprint)?;

        // Check if it's a builtin blueprint
        if self.is_builtin(&blueprint.name) {
            anyhow::bail!("Cannot modify builtin blueprint '{}'", blueprint.name);
        }

        // Check if blueprint exists
        if !self.custom_blueprints.contains_key(&blueprint.name) {
            anyhow::bail!("Custom blueprint '{}' not found", blueprint.name);
        }

        // Save to disk
        self.storage.save(&blueprint)?;

        // Update in-memory map
        self.custom_blueprints
            .insert(blueprint.name.clone(), blueprint);

        Ok(())
    }

    /// Delete a custom blueprint
    pub fn delete_custom(&mut self, name: &str) -> Result<()> {
        // Check if it's a builtin blueprint
        if self.is_builtin(name) {
            anyhow::bail!("Cannot delete builtin blueprint '{}'", name);
        }

        // Check if blueprint exists
        if !self.custom_blueprints.contains_key(name) {
            anyhow::bail!("Custom blueprint '{}' not found", name);
        }

        // Delete from disk
        self.storage.delete(name)?;

        // Remove from in-memory map
        self.custom_blueprints.remove(name);

        Ok(())
    }

    /// Search blueprints by tag
    pub fn search_by_tag(&self, tag: &str) -> Vec<Blueprint> {
        let tag_lower = tag.to_lowercase();
        let mut matches: Vec<_> = self
            .list()
            .into_iter()
            .filter(|b| b.tags.iter().any(|t| t.to_lowercase().contains(&tag_lower)))
            .collect();

        matches.sort_by(|a, b| a.name.cmp(&b.name));
        matches
    }

    /// Reload custom blueprints from disk
    pub fn reload(&mut self) -> Result<()> {
        self.custom_blueprints = self.storage.load_all()?;
        Ok(())
    }
}

impl Default for BlueprintManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            log::error!(
                "Failed to initialize BlueprintManager: {}. Using empty manager.",
                e
            );
            Self::empty()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blueprint_manager() {
        let manager = BlueprintManager::new().unwrap();

        assert!(manager.exists("lamp"));
        assert!(manager.exists("k8s-cluster"));
        assert!(manager.exists("3tier"));
        assert!(!manager.exists("nonexistent"));

        let lamp = manager.get("lamp").unwrap();
        assert_eq!(lamp.vms.len(), 2);
    }

    #[test]
    fn test_list_blueprints() {
        let manager = BlueprintManager::new().unwrap();
        let blueprints = manager.list();

        assert!(blueprints.len() >= 12);
    }

    #[test]
    fn test_search_by_tag() {
        let manager = BlueprintManager::new().unwrap();

        let web_blueprints = manager.search_by_tag("web");
        assert!(!web_blueprints.is_empty());

        let k8s_blueprints = manager.search_by_tag("kubernetes");
        assert!(!k8s_blueprints.is_empty());
    }

    #[test]
    fn test_vm_dependencies() {
        let manager = BlueprintManager::new().unwrap();
        let lamp = manager.get("lamp").unwrap();

        let web_vm = lamp.vms.iter().find(|v| v.name == "web-server").unwrap();
        assert!(web_vm.depends_on.contains(&"mysql-db".to_string()));
    }

    #[test]
    fn test_builtin_check() {
        let manager = BlueprintManager::new().unwrap();

        assert!(manager.is_builtin("lamp"));
        assert!(manager.is_builtin("k8s-cluster"));
        assert!(!manager.is_builtin("nonexistent"));
    }

    #[test]
    fn test_create_custom() {
        let mut manager = BlueprintManager::new().unwrap();

        let custom = Blueprint {
            name: "custom-test".to_string(),
            description: "Custom test blueprint".to_string(),
            vms: vec![VMSpec {
                name: "test-vm".to_string(),
                template: "ubuntu".to_string(),
                profile: Some("dev".to_string()),
                cpu: None,
                memory: None,
                disk_size: None,
                depends_on: vec![],
                labels: HashMap::new(),
            }],
            tags: vec!["test".to_string()],
        };

        // Should succeed (may fail if blueprint already exists from previous test run)
        let result = manager.create_custom(custom.clone());
        if result.is_err() {
            // Clean up and retry
            let _ = manager.delete_custom("custom-test");
            manager.create_custom(custom.clone()).unwrap();
        }

        // Should exist now
        assert!(manager.exists("custom-test"));
        assert!(!manager.is_builtin("custom-test"));

        // Creating again should fail
        assert!(manager.create_custom(custom).is_err());

        // Clean up
        let _ = manager.delete_custom("custom-test");
    }
}
