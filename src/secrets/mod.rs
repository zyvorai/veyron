use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod access;
pub mod encryption;
pub mod keys;
pub mod rotation;
pub mod vault;

/// Secret type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretType {
    Password,
    APIToken,
    Certificate,
    PrivateKey,
    SSHKey,
    OAuth2Token,
    DatabaseCredentials,
    Custom(String),
}

/// Secret status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretStatus {
    Active,
    Expired,
    Revoked,
    Pending,
}

/// Secret
#[derive(Clone, Serialize, Deserialize)]
pub struct Secret {
    pub id: String,
    pub name: String,
    pub secret_type: SecretType,
    #[serde(skip_serializing)]
    pub value: String,
    pub status: SecretStatus,
    pub version: u32,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_accessed: Option<DateTime<Utc>>,
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Secret")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("secret_type", &self.secret_type)
            .field("value", &"[REDACTED]")
            .field("status", &self.status)
            .field("version", &self.version)
            .field("metadata", &self.metadata)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("expires_at", &self.expires_at)
            .field("last_accessed", &self.last_accessed)
            .finish()
    }
}

impl Secret {
    pub fn new(
        name: impl Into<String>,
        secret_type: SecretType,
        value: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "secret-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            secret_type,
            value: value.into(),
            status: SecretStatus::Active,
            version: 1,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            last_accessed: None,
        }
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    pub fn rotate(&mut self, new_value: impl Into<String>) -> anyhow::Result<()> {
        if self.status == SecretStatus::Revoked {
            anyhow::bail!("Cannot rotate a revoked secret '{}'", self.name);
        }
        // Clear old value before replacing
        self.value.clear();
        self.value = new_value.into();
        self.version += 1;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn revoke(&mut self) {
        self.status = SecretStatus::Revoked;
        self.value = String::new();
        self.updated_at = Utc::now();
    }

    pub fn access(&mut self) {
        self.last_accessed = Some(Utc::now());
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expires_at {
            Utc::now() > expiry
        } else {
            false
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == SecretStatus::Active && !self.is_expired()
    }

    pub fn days_until_expiry(&self) -> Option<i64> {
        self.expires_at
            .map(|expiry| (expiry - Utc::now()).num_days())
    }
}

/// Secret reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretReference {
    pub id: String,
    pub secret_id: String,
    pub consumer: String,
    pub purpose: String,
    pub created_at: DateTime<Utc>,
}

impl SecretReference {
    pub fn new(
        secret_id: impl Into<String>,
        consumer: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Self {
        let secret_id_str = secret_id.into();
        let id = format!("ref-{}-{}", secret_id_str, Utc::now().timestamp_micros());

        Self {
            id,
            secret_id: secret_id_str,
            consumer: consumer.into(),
            purpose: purpose.into(),
            created_at: Utc::now(),
        }
    }
}

/// Secret manager
pub struct SecretManager {
    secrets: HashMap<String, Secret>,
    references: HashMap<String, SecretReference>,
}

impl SecretManager {
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
            references: HashMap::new(),
        }
    }

    pub fn add_secret(&mut self, secret: Secret) -> String {
        let id = secret.id.clone();
        self.secrets.insert(id.clone(), secret);
        id
    }

    pub fn get_secret(&self, id: &str) -> Option<&Secret> {
        self.secrets.get(id)
    }

    pub fn get_secret_mut(&mut self, id: &str) -> Option<&mut Secret> {
        self.secrets.get_mut(id)
    }

    pub fn secret_count(&self) -> usize {
        self.secrets.len()
    }

    pub fn add_reference(&mut self, reference: SecretReference) -> String {
        let id = reference.id.clone();
        self.references.insert(id.clone(), reference);
        id
    }

    pub fn reference_count(&self) -> usize {
        self.references.len()
    }

    pub fn secrets_by_type(&self, secret_type: &SecretType) -> Vec<&Secret> {
        self.secrets
            .values()
            .filter(|s| &s.secret_type == secret_type)
            .collect()
    }

    pub fn active_secrets(&self) -> Vec<&Secret> {
        self.secrets.values().filter(|s| s.is_active()).collect()
    }

    pub fn expired_secrets(&self) -> Vec<&Secret> {
        self.secrets.values().filter(|s| s.is_expired()).collect()
    }

    pub fn secrets_expiring_soon(&self, days: i64) -> Vec<&Secret> {
        self.secrets
            .values()
            .filter(|s| {
                if let Some(days_left) = s.days_until_expiry() {
                    days_left <= days && days_left >= 0
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn references_for_secret(&self, secret_id: &str) -> Vec<&SecretReference> {
        self.references
            .values()
            .filter(|r| r.secret_id == secret_id)
            .collect()
    }

    pub fn references_by_consumer(&self, consumer: &str) -> Vec<&SecretReference> {
        self.references
            .values()
            .filter(|r| r.consumer == consumer)
            .collect()
    }
}

impl Default for SecretManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret() {
        let secret = Secret::new("db-password", SecretType::Password, "encrypted-data");

        assert_eq!(secret.name, "db-password");
        assert_eq!(secret.secret_type, SecretType::Password);
        assert_eq!(secret.value, "encrypted-data");
        assert_eq!(secret.status, SecretStatus::Active);
        assert_eq!(secret.version, 1);
    }

    #[test]
    fn test_secret_with_expiry() {
        let expiry = Utc::now() + chrono::TimeDelta::days(30);
        let secret =
            Secret::new("api-token", SecretType::APIToken, "enc-token").with_expiry(expiry);

        assert_eq!(secret.expires_at, Some(expiry));
    }

    #[test]
    fn test_secret_add_metadata() {
        let mut secret = Secret::new("ssh-key", SecretType::SSHKey, "enc-key");

        secret.add_metadata("user", "admin");
        secret.add_metadata("environment", "production");

        assert_eq!(secret.metadata.len(), 2);
        assert_eq!(secret.metadata.get("user"), Some(&"admin".to_string()));
    }

    #[test]
    fn test_secret_rotate() {
        let mut secret = Secret::new("password", SecretType::Password, "old-value");

        assert_eq!(secret.version, 1);

        secret.rotate("new-value").unwrap();

        assert_eq!(secret.value, "new-value");
        assert_eq!(secret.version, 2);
    }

    #[test]
    fn test_secret_revoke() {
        let mut secret = Secret::new("token", SecretType::APIToken, "token-data");

        assert_eq!(secret.status, SecretStatus::Active);

        secret.revoke();

        assert_eq!(secret.status, SecretStatus::Revoked);
    }

    #[test]
    fn test_secret_access() {
        let mut secret = Secret::new("password", SecretType::Password, "data");

        assert!(secret.last_accessed.is_none());

        secret.access();

        assert!(secret.last_accessed.is_some());
    }

    #[test]
    fn test_secret_is_expired() {
        let past = Utc::now() - chrono::TimeDelta::days(1);
        let secret1 = Secret::new("token1", SecretType::APIToken, "data").with_expiry(past);
        assert!(secret1.is_expired());

        let future = Utc::now() + chrono::TimeDelta::days(30);
        let secret2 = Secret::new("token2", SecretType::APIToken, "data").with_expiry(future);
        assert!(!secret2.is_expired());

        let secret3 = Secret::new("token3", SecretType::APIToken, "data");
        assert!(!secret3.is_expired());
    }

    #[test]
    fn test_secret_is_active() {
        let secret1 = Secret::new("token1", SecretType::APIToken, "data");
        assert!(secret1.is_active());

        let mut secret2 = Secret::new("token2", SecretType::APIToken, "data");
        secret2.revoke();
        assert!(!secret2.is_active());

        let past = Utc::now() - chrono::TimeDelta::days(1);
        let secret3 = Secret::new("token3", SecretType::APIToken, "data").with_expiry(past);
        assert!(!secret3.is_active());
    }

    #[test]
    fn test_secret_days_until_expiry() {
        let future = Utc::now() + chrono::TimeDelta::days(15);
        let secret = Secret::new("token", SecretType::APIToken, "data").with_expiry(future);

        let days = secret.days_until_expiry().unwrap();
        assert!((14..=15).contains(&days));
    }

    #[test]
    fn test_secret_reference() {
        let reference = SecretReference::new("secret-123", "app-1", "database connection");

        assert_eq!(reference.secret_id, "secret-123");
        assert_eq!(reference.consumer, "app-1");
        assert_eq!(reference.purpose, "database connection");
    }

    #[test]
    fn test_secret_manager() {
        let mut manager = SecretManager::new();

        let secret = Secret::new("password", SecretType::Password, "encrypted");
        let id = manager.add_secret(secret);

        assert_eq!(manager.secret_count(), 1);
        assert!(manager.get_secret(&id).is_some());
    }

    #[test]
    fn test_manager_add_reference() {
        let mut manager = SecretManager::new();

        let reference = SecretReference::new("secret-1", "app-1", "auth");
        let _id = manager.add_reference(reference);

        assert_eq!(manager.reference_count(), 1);
    }

    #[test]
    fn test_manager_secrets_by_type() {
        let mut manager = SecretManager::new();

        manager.add_secret(Secret::new("pwd1", SecretType::Password, "enc1"));
        manager.add_secret(Secret::new("token1", SecretType::APIToken, "enc2"));
        manager.add_secret(Secret::new("pwd2", SecretType::Password, "enc3"));

        let passwords = manager.secrets_by_type(&SecretType::Password);
        assert_eq!(passwords.len(), 2);
    }

    #[test]
    fn test_manager_active_secrets() {
        let mut manager = SecretManager::new();

        let secret1 = Secret::new("s1", SecretType::Password, "enc1");
        let mut secret2 = Secret::new("s2", SecretType::Password, "enc2");
        secret2.revoke();

        manager.add_secret(secret1);
        manager.add_secret(secret2);

        let active = manager.active_secrets();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_expired_secrets() {
        let mut manager = SecretManager::new();

        let past = Utc::now() - chrono::TimeDelta::days(1);
        let future = Utc::now() + chrono::TimeDelta::days(30);

        manager.add_secret(Secret::new("s1", SecretType::Password, "enc1").with_expiry(past));
        manager.add_secret(Secret::new("s2", SecretType::Password, "enc2").with_expiry(future));

        let expired = manager.expired_secrets();
        assert_eq!(expired.len(), 1);
    }

    #[test]
    fn test_manager_secrets_expiring_soon() {
        let mut manager = SecretManager::new();

        let soon = Utc::now() + chrono::TimeDelta::days(5);
        let later = Utc::now() + chrono::TimeDelta::days(60);

        manager.add_secret(Secret::new("s1", SecretType::Password, "enc1").with_expiry(soon));
        manager.add_secret(Secret::new("s2", SecretType::Password, "enc2").with_expiry(later));

        let expiring = manager.secrets_expiring_soon(7);
        assert_eq!(expiring.len(), 1);
    }

    #[test]
    fn test_manager_references_for_secret() {
        let mut manager = SecretManager::new();

        manager.add_reference(SecretReference::new("secret-1", "app-1", "auth"));
        manager.add_reference(SecretReference::new("secret-2", "app-2", "db"));
        manager.add_reference(SecretReference::new("secret-1", "app-3", "api"));

        let refs = manager.references_for_secret("secret-1");
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_manager_references_by_consumer() {
        let mut manager = SecretManager::new();

        manager.add_reference(SecretReference::new("secret-1", "app-1", "auth"));
        manager.add_reference(SecretReference::new("secret-2", "app-2", "db"));
        manager.add_reference(SecretReference::new("secret-3", "app-1", "api"));

        let refs = manager.references_by_consumer("app-1");
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_secret_type_equality() {
        assert_eq!(SecretType::Password, SecretType::Password);
        assert_ne!(SecretType::Password, SecretType::APIToken);
    }

    #[test]
    fn test_secret_status_equality() {
        assert_eq!(SecretStatus::Active, SecretStatus::Active);
        assert_ne!(SecretStatus::Active, SecretStatus::Revoked);
    }
}
