use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vault type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultType {
    Local,
    Cloud,
    Hybrid,
}

/// Vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub vault_type: VaultType,
    pub endpoint: String,
    pub sealed: bool,
    pub secret_count: u32,
    pub max_secrets: Option<u32>,
    pub created_at: DateTime<Utc>,
}

impl Vault {
    pub fn new(
        name: impl Into<String>,
        vault_type: VaultType,
        endpoint: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "vault-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            vault_type,
            endpoint: endpoint.into(),
            sealed: true,
            secret_count: 0,
            max_secrets: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_max_secrets(mut self, max: u32) -> Self {
        self.max_secrets = Some(max);
        self
    }

    pub fn unseal(&mut self, unseal_key: &str) -> anyhow::Result<()> {
        if unseal_key.is_empty() {
            anyhow::bail!("Unseal key must not be empty");
        }
        // Vault unseal verification is not yet implemented
        anyhow::bail!("Vault unseal key verification is not yet implemented. Cannot unseal without proper key verification.")
    }

    pub fn seal(&mut self) {
        self.sealed = true;
    }

    pub fn is_full(&self) -> bool {
        if let Some(max) = self.max_secrets {
            self.secret_count >= max
        } else {
            false
        }
    }

    pub fn is_available(&self) -> bool {
        !self.sealed && !self.is_full()
    }
}

/// Vault manager
pub struct VaultManager {
    vaults: HashMap<String, Vault>,
}

impl VaultManager {
    pub fn new() -> Self {
        Self {
            vaults: HashMap::new(),
        }
    }

    pub fn add_vault(&mut self, vault: Vault) -> String {
        let id = vault.id.clone();
        self.vaults.insert(id.clone(), vault);
        id
    }

    pub fn get_vault(&self, id: &str) -> Option<&Vault> {
        self.vaults.get(id)
    }

    pub fn get_vault_mut(&mut self, id: &str) -> Option<&mut Vault> {
        self.vaults.get_mut(id)
    }

    pub fn vault_count(&self) -> usize {
        self.vaults.len()
    }

    pub fn vaults_by_type(&self, vault_type: &VaultType) -> Vec<&Vault> {
        self.vaults
            .values()
            .filter(|v| &v.vault_type == vault_type)
            .collect()
    }

    pub fn unsealed_vaults(&self) -> Vec<&Vault> {
        self.vaults.values().filter(|v| !v.sealed).collect()
    }

    pub fn available_vaults(&self) -> Vec<&Vault> {
        self.vaults.values().filter(|v| v.is_available()).collect()
    }
}

impl Default for VaultManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault() {
        let vault = Vault::new("production", VaultType::Cloud, "https://vault.example.com");

        assert_eq!(vault.name, "production");
        assert_eq!(vault.vault_type, VaultType::Cloud);
        assert_eq!(vault.endpoint, "https://vault.example.com");
        assert!(vault.sealed);
    }

    #[test]
    fn test_vault_with_max_secrets() {
        let vault = Vault::new("test", VaultType::Local, "local").with_max_secrets(1000);

        assert_eq!(vault.max_secrets, Some(1000));
    }

    #[test]
    fn test_vault_unseal_seal() {
        let mut vault = Vault::new("test", VaultType::Local, "local");

        assert!(vault.sealed);

        // Unseal is not yet implemented and should return an error
        let result = vault.unseal("test-unseal-key");
        assert!(result.is_err());
        assert!(vault.sealed); // Vault should remain sealed

        // Empty key should also error
        let result = vault.unseal("");
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_is_full() {
        let mut vault = Vault::new("test", VaultType::Local, "local").with_max_secrets(10);

        assert!(!vault.is_full());

        vault.secret_count = 10;
        assert!(vault.is_full());
    }

    #[test]
    fn test_vault_is_available() {
        let mut vault = Vault::new("test", VaultType::Local, "local").with_max_secrets(10);

        assert!(!vault.is_available()); // Sealed

        // Manually unseal for testing since unseal() now rejects all keys
        vault.sealed = false;
        assert!(vault.is_available());

        vault.secret_count = 10;
        assert!(!vault.is_available()); // Full
    }

    #[test]
    fn test_vault_manager() {
        let mut manager = VaultManager::new();

        let vault = Vault::new("test", VaultType::Local, "local");
        let id = manager.add_vault(vault);

        assert_eq!(manager.vault_count(), 1);
        assert!(manager.get_vault(&id).is_some());
    }

    #[test]
    fn test_manager_vaults_by_type() {
        let mut manager = VaultManager::new();

        manager.add_vault(Vault::new("v1", VaultType::Cloud, "e1"));
        manager.add_vault(Vault::new("v2", VaultType::Local, "e2"));
        manager.add_vault(Vault::new("v3", VaultType::Cloud, "e3"));

        let cloud_vaults = manager.vaults_by_type(&VaultType::Cloud);
        assert_eq!(cloud_vaults.len(), 2);
    }

    #[test]
    fn test_manager_unsealed_vaults() {
        let mut manager = VaultManager::new();

        let mut vault1 = Vault::new("v1", VaultType::Local, "e1");
        // Manually unseal for testing since unseal() now rejects all keys
        vault1.sealed = false;

        let vault2 = Vault::new("v2", VaultType::Local, "e2");

        manager.add_vault(vault1);
        manager.add_vault(vault2);

        let unsealed = manager.unsealed_vaults();
        assert_eq!(unsealed.len(), 1);
    }

    #[test]
    fn test_manager_available_vaults() {
        let mut manager = VaultManager::new();

        let mut vault1 = Vault::new("v1", VaultType::Local, "e1");
        // Manually unseal for testing since unseal() now rejects all keys
        vault1.sealed = false;

        let vault2 = Vault::new("v2", VaultType::Local, "e2");

        manager.add_vault(vault1);
        manager.add_vault(vault2);

        let available = manager.available_vaults();
        assert_eq!(available.len(), 1);
    }

    #[test]
    fn test_vault_type_equality() {
        assert_eq!(VaultType::Cloud, VaultType::Cloud);
        assert_ne!(VaultType::Cloud, VaultType::Local);
    }
}
