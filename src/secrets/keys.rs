use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Key type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyType {
    Symmetric,
    Asymmetric,
    MasterKey,
    DataKey,
}

/// Key status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyStatus {
    Active,
    Inactive,
    Compromised,
    Destroyed,
}

/// Encryption key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    pub id: String,
    pub name: String,
    pub key_type: KeyType,
    pub status: KeyStatus,
    pub size_bits: u32,
    pub version: u32,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub last_rotated: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl EncryptionKey {
    pub fn new(name: impl Into<String>, key_type: KeyType, size_bits: u32) -> Self {
        let name_str = name.into();
        let id = format!(
            "key-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            key_type,
            status: KeyStatus::Active,
            size_bits,
            version: 1,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            last_rotated: None,
            expires_at: None,
        }
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    pub fn rotate(&mut self) {
        log::warn!("Key rotation only updates metadata. Actual key material regeneration is not yet implemented.");
        self.version += 1;
        self.last_rotated = Some(Utc::now());
    }

    pub fn deactivate(&mut self) {
        self.status = KeyStatus::Inactive;
    }

    pub fn mark_compromised(&mut self) {
        self.status = KeyStatus::Compromised;
    }

    pub fn destroy(&mut self) {
        self.status = KeyStatus::Destroyed;
    }

    pub fn is_active(&self) -> bool {
        self.status == KeyStatus::Active
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expires_at {
            Utc::now() > expiry
        } else {
            false
        }
    }

    pub fn is_usable(&self) -> bool {
        self.is_active() && !self.is_expired()
    }
}

/// Key manager
pub struct KeyManager {
    keys: HashMap<String, EncryptionKey>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    pub fn add_key(&mut self, key: EncryptionKey) -> String {
        let id = key.id.clone();
        self.keys.insert(id.clone(), key);
        id
    }

    pub fn get_key(&self, id: &str) -> Option<&EncryptionKey> {
        self.keys.get(id)
    }

    pub fn get_key_mut(&mut self, id: &str) -> Option<&mut EncryptionKey> {
        self.keys.get_mut(id)
    }

    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    pub fn keys_by_type(&self, key_type: &KeyType) -> Vec<&EncryptionKey> {
        self.keys
            .values()
            .filter(|k| &k.key_type == key_type)
            .collect()
    }

    pub fn active_keys(&self) -> Vec<&EncryptionKey> {
        self.keys.values().filter(|k| k.is_active()).collect()
    }

    pub fn usable_keys(&self) -> Vec<&EncryptionKey> {
        self.keys.values().filter(|k| k.is_usable()).collect()
    }

    pub fn compromised_keys(&self) -> Vec<&EncryptionKey> {
        self.keys
            .values()
            .filter(|k| k.status == KeyStatus::Compromised)
            .collect()
    }

    pub fn master_keys(&self) -> Vec<&EncryptionKey> {
        self.keys_by_type(&KeyType::MasterKey)
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_key() {
        let key = EncryptionKey::new("master-key", KeyType::MasterKey, 256);

        assert_eq!(key.name, "master-key");
        assert_eq!(key.key_type, KeyType::MasterKey);
        assert_eq!(key.size_bits, 256);
        assert_eq!(key.status, KeyStatus::Active);
        assert_eq!(key.version, 1);
    }

    #[test]
    fn test_key_with_expiry() {
        let expiry = Utc::now() + chrono::TimeDelta::days(365);
        let key = EncryptionKey::new("temp-key", KeyType::DataKey, 128).with_expiry(expiry);

        assert_eq!(key.expires_at, Some(expiry));
    }

    #[test]
    fn test_key_add_metadata() {
        let mut key = EncryptionKey::new("key", KeyType::Symmetric, 256);

        key.add_metadata("environment", "production");
        key.add_metadata("purpose", "database-encryption");

        assert_eq!(key.metadata.len(), 2);
        assert_eq!(
            key.metadata.get("environment"),
            Some(&"production".to_string())
        );
    }

    #[test]
    fn test_key_rotate() {
        let mut key = EncryptionKey::new("key", KeyType::DataKey, 256);

        assert_eq!(key.version, 1);
        assert!(key.last_rotated.is_none());

        key.rotate();

        assert_eq!(key.version, 2);
        assert!(key.last_rotated.is_some());
    }

    #[test]
    fn test_key_deactivate() {
        let mut key = EncryptionKey::new("key", KeyType::DataKey, 256);

        assert_eq!(key.status, KeyStatus::Active);

        key.deactivate();

        assert_eq!(key.status, KeyStatus::Inactive);
    }

    #[test]
    fn test_key_mark_compromised() {
        let mut key = EncryptionKey::new("key", KeyType::DataKey, 256);

        key.mark_compromised();

        assert_eq!(key.status, KeyStatus::Compromised);
    }

    #[test]
    fn test_key_destroy() {
        let mut key = EncryptionKey::new("key", KeyType::DataKey, 256);

        key.destroy();

        assert_eq!(key.status, KeyStatus::Destroyed);
    }

    #[test]
    fn test_key_is_active() {
        let key1 = EncryptionKey::new("key1", KeyType::DataKey, 256);
        assert!(key1.is_active());

        let mut key2 = EncryptionKey::new("key2", KeyType::DataKey, 256);
        key2.deactivate();
        assert!(!key2.is_active());
    }

    #[test]
    fn test_key_is_expired() {
        let past = Utc::now() - chrono::TimeDelta::days(1);
        let key1 = EncryptionKey::new("key1", KeyType::DataKey, 256).with_expiry(past);
        assert!(key1.is_expired());

        let future = Utc::now() + chrono::TimeDelta::days(365);
        let key2 = EncryptionKey::new("key2", KeyType::DataKey, 256).with_expiry(future);
        assert!(!key2.is_expired());
    }

    #[test]
    fn test_key_is_usable() {
        let key1 = EncryptionKey::new("key1", KeyType::DataKey, 256);
        assert!(key1.is_usable());

        let mut key2 = EncryptionKey::new("key2", KeyType::DataKey, 256);
        key2.deactivate();
        assert!(!key2.is_usable());

        let past = Utc::now() - chrono::TimeDelta::days(1);
        let key3 = EncryptionKey::new("key3", KeyType::DataKey, 256).with_expiry(past);
        assert!(!key3.is_usable());
    }

    #[test]
    fn test_key_manager() {
        let mut manager = KeyManager::new();

        let key = EncryptionKey::new("master", KeyType::MasterKey, 256);
        let id = manager.add_key(key);

        assert_eq!(manager.key_count(), 1);
        assert!(manager.get_key(&id).is_some());
    }

    #[test]
    fn test_manager_keys_by_type() {
        let mut manager = KeyManager::new();

        manager.add_key(EncryptionKey::new("k1", KeyType::MasterKey, 256));
        manager.add_key(EncryptionKey::new("k2", KeyType::DataKey, 128));
        manager.add_key(EncryptionKey::new("k3", KeyType::MasterKey, 256));

        let master_keys = manager.keys_by_type(&KeyType::MasterKey);
        assert_eq!(master_keys.len(), 2);
    }

    #[test]
    fn test_manager_active_keys() {
        let mut manager = KeyManager::new();

        let key1 = EncryptionKey::new("k1", KeyType::DataKey, 256);
        let mut key2 = EncryptionKey::new("k2", KeyType::DataKey, 256);
        key2.deactivate();

        manager.add_key(key1);
        manager.add_key(key2);

        let active = manager.active_keys();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_usable_keys() {
        let mut manager = KeyManager::new();

        let key1 = EncryptionKey::new("k1", KeyType::DataKey, 256);

        let past = Utc::now() - chrono::TimeDelta::days(1);
        let key2 = EncryptionKey::new("k2", KeyType::DataKey, 256).with_expiry(past);

        manager.add_key(key1);
        manager.add_key(key2);

        let usable = manager.usable_keys();
        assert_eq!(usable.len(), 1);
    }

    #[test]
    fn test_manager_compromised_keys() {
        let mut manager = KeyManager::new();

        let mut key1 = EncryptionKey::new("k1", KeyType::DataKey, 256);
        key1.mark_compromised();

        let key2 = EncryptionKey::new("k2", KeyType::DataKey, 256);

        manager.add_key(key1);
        manager.add_key(key2);

        let compromised = manager.compromised_keys();
        assert_eq!(compromised.len(), 1);
    }

    #[test]
    fn test_manager_master_keys() {
        let mut manager = KeyManager::new();

        manager.add_key(EncryptionKey::new("k1", KeyType::MasterKey, 256));
        manager.add_key(EncryptionKey::new("k2", KeyType::DataKey, 128));

        let master = manager.master_keys();
        assert_eq!(master.len(), 1);
    }

    #[test]
    fn test_key_type_equality() {
        assert_eq!(KeyType::MasterKey, KeyType::MasterKey);
        assert_ne!(KeyType::MasterKey, KeyType::DataKey);
    }

    #[test]
    fn test_key_status_equality() {
        assert_eq!(KeyStatus::Active, KeyStatus::Active);
        assert_ne!(KeyStatus::Active, KeyStatus::Inactive);
    }
}
