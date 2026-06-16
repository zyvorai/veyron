// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Profile Storage - Filesystem persistence for custom profiles

use super::Profile;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// ProfileStorage handles reading and writing profiles to the filesystem
pub struct ProfileStorage {
    config_dir: PathBuf,
    read_only: bool,
}

impl ProfileStorage {
    /// Create a new ProfileStorage instance
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_profiles_dir()?;

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create profiles directory")?;
        }

        Ok(Self {
            config_dir,
            read_only: false,
        })
    }

    /// Create an in-memory (no-op) storage for fallback scenarios
    pub fn in_memory() -> Self {
        Self {
            config_dir: PathBuf::from("/dev/null"),
            read_only: true,
        }
    }

    /// Get the profiles directory path
    fn get_profiles_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not determine config directory")?;

        Ok(config_dir.join("veyron").join("profiles"))
    }

    /// Get the file path for a profile.
    /// Name is sanitized to prevent path traversal.
    fn profile_path(&self, name: &str) -> PathBuf {
        // Strip path separators to prevent directory traversal
        let safe_name: String = name
            .chars()
            .filter(|c| *c != '/' && *c != '\\' && *c != '.')
            .collect();
        let safe_name = if safe_name.is_empty() {
            "unnamed"
        } else {
            &safe_name
        };
        self.config_dir.join(format!("{}.yaml", safe_name))
    }

    /// Save a profile to disk
    pub fn save(&self, profile: &Profile) -> Result<()> {
        if self.read_only {
            anyhow::bail!("Profile storage is in read-only mode");
        }
        let path = self.profile_path(&profile.name);

        let yaml = serde_yml::to_string(profile).context("Failed to serialize profile to YAML")?;

        fs::write(&path, yaml)
            .with_context(|| format!("Failed to write profile to {}", path.display()))?;

        Ok(())
    }

    /// Load a profile from disk
    pub fn load(&self, name: &str) -> Result<Profile> {
        let path = self.profile_path(name);

        if !path.exists() {
            anyhow::bail!("Profile '{}' not found", name);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read profile from {}", path.display()))?;

        let profile: Profile = serde_yml::from_str(&content)
            .with_context(|| format!("Failed to parse profile from {}", path.display()))?;

        Ok(profile)
    }

    /// Load all custom profiles from disk
    pub fn load_all(&self) -> Result<HashMap<String, Profile>> {
        let mut profiles = HashMap::new();

        // If read-only or directory doesn't exist, return empty map
        if self.read_only || !self.config_dir.exists() {
            return Ok(profiles);
        }

        let entries =
            fs::read_dir(&self.config_dir).context("Failed to read profiles directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            // Only process .yaml files
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }

            // Get profile name from filename
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());

            if let Some(name) = name {
                match self.load(&name) {
                    Ok(profile) => {
                        profiles.insert(name, profile);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load profile '{}': {}", name, e);
                    }
                }
            }
        }

        Ok(profiles)
    }

    /// Delete a profile from disk
    pub fn delete(&self, name: &str) -> Result<()> {
        if self.read_only {
            anyhow::bail!("Profile storage is in read-only mode");
        }
        let path = self.profile_path(name);

        if !path.exists() {
            anyhow::bail!("Profile '{}' not found", name);
        }

        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete profile from {}", path.display()))?;

        Ok(())
    }

    /// Check if a profile exists on disk (public API for external consumers)
    #[allow(dead_code)]
    pub fn exists(&self, name: &str) -> bool {
        self.profile_path(name).exists()
    }

    /// List all custom profile names (public API for external consumers)
    #[allow(dead_code)]
    pub fn list_names(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();

        if !self.config_dir.exists() {
            return Ok(names);
        }

        let entries =
            fs::read_dir(&self.config_dir).context("Failed to read profiles directory")?;

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

impl Default for ProfileStorage {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self::in_memory())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage() -> (ProfileStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProfileStorage {
            config_dir: temp_dir.path().to_path_buf(),
            read_only: false,
        };
        (storage, temp_dir)
    }

    fn create_test_profile(name: &str) -> Profile {
        Profile {
            name: name.to_string(),
            description: "Test profile".to_string(),
            cpu_cores: 4,
            cpu_sockets: 1,
            cpu_threads: 1,
            memory: "8Gi".to_string(),
            disk_size: "40Gi".to_string(),
            use_cases: vec!["testing".to_string()],
            recommended_os: vec!["ubuntu".to_string()],
        }
    }

    #[test]
    fn test_save_and_load() {
        let (storage, _temp) = create_test_storage();
        let profile = create_test_profile("test-profile");

        // Save
        storage.save(&profile).unwrap();

        // Load
        let loaded = storage.load("test-profile").unwrap();
        assert_eq!(loaded.name, profile.name);
        assert_eq!(loaded.cpu_cores, profile.cpu_cores);
        assert_eq!(loaded.memory, profile.memory);
    }

    #[test]
    fn test_load_all() {
        let (storage, _temp) = create_test_storage();

        // Create multiple profiles
        storage.save(&create_test_profile("profile1")).unwrap();
        storage.save(&create_test_profile("profile2")).unwrap();
        storage.save(&create_test_profile("profile3")).unwrap();

        // Load all
        let profiles = storage.load_all().unwrap();
        assert_eq!(profiles.len(), 3);
        assert!(profiles.contains_key("profile1"));
        assert!(profiles.contains_key("profile2"));
        assert!(profiles.contains_key("profile3"));
    }

    #[test]
    fn test_delete() {
        let (storage, _temp) = create_test_storage();
        let profile = create_test_profile("to-delete");

        // Save and verify exists
        storage.save(&profile).unwrap();
        assert!(storage.exists("to-delete"));

        // Delete
        storage.delete("to-delete").unwrap();
        assert!(!storage.exists("to-delete"));

        // Try to delete again should fail
        assert!(storage.delete("to-delete").is_err());
    }

    #[test]
    fn test_list_names() {
        let (storage, _temp) = create_test_storage();

        storage.save(&create_test_profile("alpha")).unwrap();
        storage.save(&create_test_profile("beta")).unwrap();
        storage.save(&create_test_profile("gamma")).unwrap();

        let names = storage.list_names().unwrap();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
    }
}
