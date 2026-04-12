// VM Profiles System - Pre-configured resource profiles for different workloads
// This is an innovative feature that makes VM creation easier and optimized

mod builtin;
mod storage;
mod validator;

pub use validator::{validate_name, validate_profile};

use anyhow::Result;
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

pub static PROFILES: LazyLock<RwLock<ProfileManager>> = LazyLock::new(|| {
    match ProfileManager::new() {
        Ok(manager) => RwLock::new(manager),
        Err(e) => {
            log::error!("Failed to initialize ProfileManager: {}. Using empty manager.", e);
            RwLock::new(ProfileManager::empty())
        }
    }
});

/// Profile represents a pre-configured resource allocation template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: String,
    pub cpu_cores: u32,
    pub cpu_sockets: u32,
    pub cpu_threads: u32,
    pub memory: String,
    pub disk_size: String,
    pub use_cases: Vec<String>,
    pub recommended_os: Vec<String>,
}

/// ProfileManager manages all available profiles (builtin and custom)
pub struct ProfileManager {
    builtin_profiles: HashMap<String, Profile>,
    custom_profiles: HashMap<String, Profile>,
    storage: storage::ProfileStorage,
}

impl ProfileManager {
    pub fn new() -> Result<Self> {
        let builtin_profiles = builtin::builtin_profiles();
        let storage = storage::ProfileStorage::new()?;
        let custom_profiles = storage.load_all()?;

        Ok(Self {
            builtin_profiles,
            custom_profiles,
            storage,
        })
    }

    /// Create an empty manager (fallback when initialization fails)
    fn empty() -> Self {
        Self {
            builtin_profiles: builtin::builtin_profiles(),
            custom_profiles: HashMap::new(),
            storage: storage::ProfileStorage::in_memory(),
        }
    }

    /// Get a profile by name (checks custom first, then builtin)
    pub fn get(&self, name: &str) -> Option<Profile> {
        self.custom_profiles
            .get(name)
            .or_else(|| self.builtin_profiles.get(name))
            .cloned()
    }

    /// List all profiles (builtin + custom)
    pub fn list(&self) -> Vec<Profile> {
        let mut profiles: Vec<_> = self
            .builtin_profiles
            .values()
            .chain(self.custom_profiles.values())
            .cloned()
            .collect();

        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        profiles
    }

    /// Check if a profile exists
    pub fn exists(&self, name: &str) -> bool {
        self.custom_profiles.contains_key(name) || self.builtin_profiles.contains_key(name)
    }

    /// Check if a profile is builtin
    pub fn is_builtin(&self, name: &str) -> bool {
        self.builtin_profiles.contains_key(name)
    }

    /// Create a custom profile
    pub fn create_custom(&mut self, profile: Profile) -> Result<()> {
        // Validate the profile
        validator::validate_profile(&profile)?;

        // Check if profile already exists
        if self.exists(&profile.name) {
            anyhow::bail!("Profile '{}' already exists", profile.name);
        }

        // Save to disk
        self.storage.save(&profile)?;

        // Add to in-memory map
        self.custom_profiles.insert(profile.name.clone(), profile);

        Ok(())
    }

    /// Update a custom profile
    pub fn update_custom(&mut self, profile: Profile) -> Result<()> {
        // Validate the profile
        validator::validate_profile(&profile)?;

        // Check if it's a builtin profile
        if self.is_builtin(&profile.name) {
            anyhow::bail!("Cannot modify builtin profile '{}'", profile.name);
        }

        // Check if profile exists
        if !self.custom_profiles.contains_key(&profile.name) {
            anyhow::bail!("Custom profile '{}' not found", profile.name);
        }

        // Save to disk
        self.storage.save(&profile)?;

        // Update in-memory map
        self.custom_profiles.insert(profile.name.clone(), profile);

        Ok(())
    }

    /// Delete a custom profile
    pub fn delete_custom(&mut self, name: &str) -> Result<()> {
        // Check if it's a builtin profile
        if self.is_builtin(name) {
            anyhow::bail!("Cannot delete builtin profile '{}'", name);
        }

        // Check if profile exists
        if !self.custom_profiles.contains_key(name) {
            anyhow::bail!("Custom profile '{}' not found", name);
        }

        // Delete from disk
        self.storage.delete(name)?;

        // Remove from in-memory map
        self.custom_profiles.remove(name);

        Ok(())
    }

    /// Get profile recommendation based on use case keywords
    pub fn recommend(&self, use_case: &str) -> Vec<Profile> {
        let use_case_lower = use_case.to_lowercase();
        let mut matches: Vec<_> = self
            .list()
            .into_iter()
            .filter(|p| {
                p.use_cases.iter().any(|uc| {
                    uc.to_lowercase().contains(&use_case_lower)
                        || use_case_lower.contains(&uc.to_lowercase())
                })
            })
            .collect();

        matches.sort_by(|a, b| a.name.cmp(&b.name));
        matches
    }

    /// Reload custom profiles from disk
    pub fn reload(&mut self) -> Result<()> {
        self.custom_profiles = self.storage.load_all()?;
        Ok(())
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            log::error!("Failed to initialize ProfileManager: {}. Using empty manager.", e);
            Self::empty()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_manager() {
        let manager = ProfileManager::new().unwrap();

        assert!(manager.exists("dev"));
        assert!(manager.exists("prod"));
        assert!(manager.exists("high-perf"));
        assert!(!manager.exists("nonexistent"));

        let dev = manager.get("dev").unwrap();
        assert_eq!(dev.cpu_cores, 1);
        assert_eq!(dev.memory, "2Gi");
    }

    #[test]
    fn test_list_profiles() {
        let manager = ProfileManager::new().unwrap();
        let profiles = manager.list();

        assert!(profiles.len() >= 8);
    }

    #[test]
    fn test_recommend() {
        let manager = ProfileManager::new().unwrap();

        let db_profiles = manager.recommend("database");
        assert!(!db_profiles.is_empty());
        assert!(db_profiles.iter().any(|p| p.name == "database"));

        let web_profiles = manager.recommend("web");
        assert!(!web_profiles.is_empty());
        assert!(web_profiles.iter().any(|p| p.name == "web"));
    }

    #[test]
    fn test_builtin_check() {
        let manager = ProfileManager::new().unwrap();

        assert!(manager.is_builtin("dev"));
        assert!(manager.is_builtin("prod"));
        assert!(!manager.is_builtin("nonexistent"));
    }

    #[test]
    fn test_create_custom() {
        let mut manager = ProfileManager::new().unwrap();

        let custom = Profile {
            name: "custom-test".to_string(),
            description: "Custom test profile".to_string(),
            cpu_cores: 4,
            cpu_sockets: 1,
            cpu_threads: 1,
            memory: "8Gi".to_string(),
            disk_size: "40Gi".to_string(),
            use_cases: vec![],
            recommended_os: vec![],
        };

        // Should succeed (may fail if profile already exists from previous test run)
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
