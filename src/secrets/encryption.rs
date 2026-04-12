use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Encryption algorithm
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    AES256GCM,
    AES128GCM,
    ChaCha20Poly1305,
    RSA2048,
    RSA4096,
}

/// Encryption provider
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionProvider {
    Local,
    AwsKms,
    AzureKeyVault,
    GcpKms,
    HashiCorpVault,
    Custom(String),
}

/// Encryption config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub id: String,
    pub name: String,
    pub algorithm: EncryptionAlgorithm,
    pub provider: EncryptionProvider,
    pub key_id: String,
    pub rotation_enabled: bool,
    pub rotation_days: u32,
    pub created_at: DateTime<Utc>,
}

impl EncryptionConfig {
    pub fn new(
        name: impl Into<String>,
        algorithm: EncryptionAlgorithm,
        provider: EncryptionProvider,
        key_id: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "enc-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            algorithm,
            provider,
            key_id: key_id.into(),
            rotation_enabled: false,
            rotation_days: 90,
            created_at: Utc::now(),
        }
    }

    pub fn enable_rotation(mut self, days: u32) -> Self {
        self.rotation_enabled = true;
        self.rotation_days = days;
        self
    }
}

/// Encrypted data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub id: String,
    pub ciphertext: String,
    pub algorithm: EncryptionAlgorithm,
    pub key_id: String,
    pub iv: Option<String>,
    pub tag: Option<String>,
    pub encrypted_at: DateTime<Utc>,
}

impl EncryptedData {
    pub fn new(
        ciphertext: impl Into<String>,
        algorithm: EncryptionAlgorithm,
        key_id: impl Into<String>,
    ) -> Self {
        let id = format!("enc-data-{}", Utc::now().timestamp_micros());

        Self {
            id,
            ciphertext: ciphertext.into(),
            algorithm,
            key_id: key_id.into(),
            iv: None,
            tag: None,
            encrypted_at: Utc::now(),
        }
    }

    pub fn with_iv(mut self, iv: impl Into<String>) -> Self {
        self.iv = Some(iv.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Validate that AEAD algorithms have required IV and authentication tag.
    pub fn validate(&self) -> anyhow::Result<()> {
        let requires_iv_and_tag = matches!(
            self.algorithm,
            EncryptionAlgorithm::AES256GCM
                | EncryptionAlgorithm::AES128GCM
                | EncryptionAlgorithm::ChaCha20Poly1305
        );

        if requires_iv_and_tag {
            if self.iv.is_none() {
                anyhow::bail!(
                    "AEAD algorithm {:?} requires an IV (initialization vector), but none was provided",
                    self.algorithm
                );
            }
            if self.tag.is_none() {
                anyhow::bail!(
                    "AEAD algorithm {:?} requires an authentication tag, but none was provided",
                    self.algorithm
                );
            }
        }

        Ok(())
    }
}

/// Encryption manager
pub struct EncryptionManager {
    configs: HashMap<String, EncryptionConfig>,
    encrypted_data: HashMap<String, EncryptedData>,
}

impl EncryptionManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            encrypted_data: HashMap::new(),
        }
    }

    pub fn add_config(&mut self, config: EncryptionConfig) -> String {
        let id = config.id.clone();
        self.configs.insert(id.clone(), config);
        id
    }

    pub fn get_config(&self, id: &str) -> Option<&EncryptionConfig> {
        self.configs.get(id)
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    pub fn add_encrypted_data(&mut self, data: EncryptedData) -> String {
        let id = data.id.clone();
        self.encrypted_data.insert(id.clone(), data);
        id
    }

    pub fn get_encrypted_data(&self, id: &str) -> Option<&EncryptedData> {
        self.encrypted_data.get(id)
    }

    pub fn encrypted_data_count(&self) -> usize {
        self.encrypted_data.len()
    }

    pub fn configs_by_provider(&self, provider: &EncryptionProvider) -> Vec<&EncryptionConfig> {
        self.configs
            .values()
            .filter(|c| &c.provider == provider)
            .collect()
    }

    pub fn configs_by_algorithm(&self, algorithm: &EncryptionAlgorithm) -> Vec<&EncryptionConfig> {
        self.configs
            .values()
            .filter(|c| &c.algorithm == algorithm)
            .collect()
    }

    pub fn configs_with_rotation(&self) -> Vec<&EncryptionConfig> {
        self.configs
            .values()
            .filter(|c| c.rotation_enabled)
            .collect()
    }
}

impl Default for EncryptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_config() {
        let config = EncryptionConfig::new(
            "default-encryption",
            EncryptionAlgorithm::AES256GCM,
            EncryptionProvider::Local,
            "key-123",
        );

        assert_eq!(config.name, "default-encryption");
        assert_eq!(config.algorithm, EncryptionAlgorithm::AES256GCM);
        assert_eq!(config.provider, EncryptionProvider::Local);
        assert_eq!(config.key_id, "key-123");
        assert!(!config.rotation_enabled);
    }

    #[test]
    fn test_config_enable_rotation() {
        let config = EncryptionConfig::new(
            "rotated",
            EncryptionAlgorithm::AES256GCM,
            EncryptionProvider::AwsKms,
            "key-456",
        )
        .enable_rotation(30);

        assert!(config.rotation_enabled);
        assert_eq!(config.rotation_days, 30);
    }

    #[test]
    fn test_encrypted_data() {
        let data = EncryptedData::new(
            "ciphertext-here",
            EncryptionAlgorithm::ChaCha20Poly1305,
            "key-789",
        );

        assert_eq!(data.ciphertext, "ciphertext-here");
        assert_eq!(data.algorithm, EncryptionAlgorithm::ChaCha20Poly1305);
        assert_eq!(data.key_id, "key-789");
    }

    #[test]
    fn test_encrypted_data_with_iv() {
        let data = EncryptedData::new("cipher", EncryptionAlgorithm::AES256GCM, "key-1")
            .with_iv("random-iv");

        assert_eq!(data.iv, Some("random-iv".to_string()));
    }

    #[test]
    fn test_encrypted_data_with_tag() {
        let data = EncryptedData::new("cipher", EncryptionAlgorithm::AES256GCM, "key-1")
            .with_tag("auth-tag");

        assert_eq!(data.tag, Some("auth-tag".to_string()));
    }

    #[test]
    fn test_encryption_manager() {
        let mut manager = EncryptionManager::new();

        let config = EncryptionConfig::new(
            "test",
            EncryptionAlgorithm::AES256GCM,
            EncryptionProvider::Local,
            "key-1",
        );
        let id = manager.add_config(config);

        assert_eq!(manager.config_count(), 1);
        assert!(manager.get_config(&id).is_some());
    }

    #[test]
    fn test_manager_add_encrypted_data() {
        let mut manager = EncryptionManager::new();

        let data = EncryptedData::new("cipher", EncryptionAlgorithm::AES256GCM, "key-1");
        let id = manager.add_encrypted_data(data);

        assert_eq!(manager.encrypted_data_count(), 1);
        assert!(manager.get_encrypted_data(&id).is_some());
    }

    #[test]
    fn test_manager_configs_by_provider() {
        let mut manager = EncryptionManager::new();

        manager.add_config(EncryptionConfig::new(
            "c1",
            EncryptionAlgorithm::AES256GCM,
            EncryptionProvider::AwsKms,
            "k1",
        ));
        manager.add_config(EncryptionConfig::new(
            "c2",
            EncryptionAlgorithm::AES256GCM,
            EncryptionProvider::Local,
            "k2",
        ));
        manager.add_config(EncryptionConfig::new(
            "c3",
            EncryptionAlgorithm::AES256GCM,
            EncryptionProvider::AwsKms,
            "k3",
        ));

        let aws_configs = manager.configs_by_provider(&EncryptionProvider::AwsKms);
        assert_eq!(aws_configs.len(), 2);
    }

    #[test]
    fn test_manager_configs_by_algorithm() {
        let mut manager = EncryptionManager::new();

        manager.add_config(EncryptionConfig::new(
            "c1",
            EncryptionAlgorithm::AES256GCM,
            EncryptionProvider::Local,
            "k1",
        ));
        manager.add_config(EncryptionConfig::new(
            "c2",
            EncryptionAlgorithm::RSA2048,
            EncryptionProvider::Local,
            "k2",
        ));
        manager.add_config(EncryptionConfig::new(
            "c3",
            EncryptionAlgorithm::AES256GCM,
            EncryptionProvider::Local,
            "k3",
        ));

        let aes_configs = manager.configs_by_algorithm(&EncryptionAlgorithm::AES256GCM);
        assert_eq!(aes_configs.len(), 2);
    }

    #[test]
    fn test_manager_configs_with_rotation() {
        let mut manager = EncryptionManager::new();

        manager.add_config(
            EncryptionConfig::new(
                "c1",
                EncryptionAlgorithm::AES256GCM,
                EncryptionProvider::Local,
                "k1",
            )
            .enable_rotation(30),
        );
        manager.add_config(EncryptionConfig::new(
            "c2",
            EncryptionAlgorithm::AES256GCM,
            EncryptionProvider::Local,
            "k2",
        ));

        let with_rotation = manager.configs_with_rotation();
        assert_eq!(with_rotation.len(), 1);
    }

    #[test]
    fn test_encryption_algorithm_equality() {
        assert_eq!(
            EncryptionAlgorithm::AES256GCM,
            EncryptionAlgorithm::AES256GCM
        );
        assert_ne!(EncryptionAlgorithm::AES256GCM, EncryptionAlgorithm::RSA2048);
    }

    #[test]
    fn test_encryption_provider_equality() {
        assert_eq!(EncryptionProvider::AwsKms, EncryptionProvider::AwsKms);
        assert_ne!(EncryptionProvider::AwsKms, EncryptionProvider::Local);
    }
}
