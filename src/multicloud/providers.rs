use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::CloudProvider;

/// Provider credentials
#[derive(Clone, Serialize, Deserialize)]
pub struct ProviderCredentials {
    pub id: String,
    pub provider: CloudProvider,
    pub credential_type: CredentialType,
    #[serde(skip_serializing)]
    pub access_key: Option<String>,
    #[serde(skip_serializing)]
    pub secret_key: Option<String>,
    pub tenant_id: Option<String>,
    pub subscription_id: Option<String>,
    pub project_id: Option<String>,
    #[serde(skip_serializing)]
    pub service_account: Option<String>,
    pub region: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Credential type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialType {
    AccessKey,
    ServicePrincipal,
    ServiceAccount,
    APIToken,
    OAuth2,
}

impl std::fmt::Debug for ProviderCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderCredentials")
            .field("id", &self.id)
            .field("provider", &self.provider)
            .field("credential_type", &self.credential_type)
            .field("access_key", &"[REDACTED]")
            .field("secret_key", &"[REDACTED]")
            .field("tenant_id", &self.tenant_id)
            .field("subscription_id", &self.subscription_id)
            .field("project_id", &self.project_id)
            .field("service_account", &"[REDACTED]")
            .field("region", &self.region)
            .field("enabled", &self.enabled)
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl ProviderCredentials {
    pub fn new(
        provider: CloudProvider,
        credential_type: CredentialType,
        region: impl Into<String>,
    ) -> Self {
        let id = format!("cred-{}-{}", provider, Utc::now().timestamp_micros());

        Self {
            id,
            provider,
            credential_type,
            access_key: None,
            secret_key: None,
            tenant_id: None,
            subscription_id: None,
            project_id: None,
            service_account: None,
            region: region.into(),
            enabled: true,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    pub fn with_access_key(
        mut self,
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Self {
        self.access_key = Some(access_key.into());
        self.secret_key = Some(secret_key.into());
        self
    }

    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_subscription(mut self, subscription_id: impl Into<String>) -> Self {
        self.subscription_id = Some(subscription_id.into());
        self
    }

    pub fn with_project(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    pub fn with_service_account(mut self, service_account: impl Into<String>) -> Self {
        self.service_account = Some(service_account.into());
        self
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expires_at {
            Utc::now() > expiry
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        self.enabled && !self.is_expired()
    }
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub provider: CloudProvider,
    pub endpoint: String,
    pub default_region: String,
    pub credential_id: String,
    pub rate_limit: Option<u32>,
    pub timeout_seconds: u32,
    pub retry_attempts: u32,
    pub metadata: HashMap<String, String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl ProviderConfig {
    pub fn new(
        name: impl Into<String>,
        provider: CloudProvider,
        endpoint: impl Into<String>,
        default_region: impl Into<String>,
        credential_id: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "prov-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            provider,
            endpoint: endpoint.into(),
            default_region: default_region.into(),
            credential_id: credential_id.into(),
            rate_limit: None,
            timeout_seconds: 30,
            retry_attempts: 3,
            metadata: HashMap::new(),
            enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_rate_limit(mut self, limit: u32) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    pub fn with_timeout(mut self, seconds: u32) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn with_retries(mut self, attempts: u32) -> Self {
        self.retry_attempts = attempts;
        self
    }

    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

/// Provider manager
pub struct ProviderManager {
    credentials: HashMap<String, ProviderCredentials>,
    configs: HashMap<String, ProviderConfig>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    pub fn add_credentials(&mut self, creds: ProviderCredentials) -> String {
        let id = creds.id.clone();
        self.credentials.insert(id.clone(), creds);
        id
    }

    pub fn get_credentials(&self, id: &str) -> Option<&ProviderCredentials> {
        self.credentials.get(id)
    }

    pub fn get_credentials_mut(&mut self, id: &str) -> Option<&mut ProviderCredentials> {
        self.credentials.get_mut(id)
    }

    pub fn credentials_count(&self) -> usize {
        self.credentials.len()
    }

    pub fn add_config(&mut self, config: ProviderConfig) -> String {
        let id = config.id.clone();
        self.configs.insert(id.clone(), config);
        id
    }

    pub fn get_config(&self, id: &str) -> Option<&ProviderConfig> {
        self.configs.get(id)
    }

    pub fn get_config_mut(&mut self, id: &str) -> Option<&mut ProviderConfig> {
        self.configs.get_mut(id)
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    pub fn credentials_by_provider(&self, provider: &CloudProvider) -> Vec<&ProviderCredentials> {
        self.credentials
            .values()
            .filter(|c| &c.provider == provider)
            .collect()
    }

    pub fn valid_credentials(&self) -> Vec<&ProviderCredentials> {
        self.credentials.values().filter(|c| c.is_valid()).collect()
    }

    pub fn expired_credentials(&self) -> Vec<&ProviderCredentials> {
        self.credentials
            .values()
            .filter(|c| c.is_expired())
            .collect()
    }

    pub fn configs_by_provider(&self, provider: &CloudProvider) -> Vec<&ProviderConfig> {
        self.configs
            .values()
            .filter(|c| &c.provider == provider)
            .collect()
    }

    pub fn enabled_configs(&self) -> Vec<&ProviderConfig> {
        self.configs.values().filter(|c| c.enabled).collect()
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_credentials() {
        let creds =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::AccessKey, "us-east-1");

        assert_eq!(creds.provider, CloudProvider::AWS);
        assert_eq!(creds.credential_type, CredentialType::AccessKey);
        assert_eq!(creds.region, "us-east-1");
        assert!(creds.enabled);
    }

    #[test]
    fn test_credentials_with_access_key() {
        let creds =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::AccessKey, "us-east-1")
                .with_access_key(
                    "AKIAIOSFODNN7EXAMPLE",
                    "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
                );

        assert_eq!(creds.access_key, Some("AKIAIOSFODNN7EXAMPLE".to_string()));
        assert_eq!(
            creds.secret_key,
            Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string())
        );
    }

    #[test]
    fn test_credentials_with_tenant() {
        let creds = ProviderCredentials::new(
            CloudProvider::Azure,
            CredentialType::ServicePrincipal,
            "eastus",
        )
        .with_tenant("tenant-123");

        assert_eq!(creds.tenant_id, Some("tenant-123".to_string()));
    }

    #[test]
    fn test_credentials_with_subscription() {
        let creds = ProviderCredentials::new(
            CloudProvider::Azure,
            CredentialType::ServicePrincipal,
            "eastus",
        )
        .with_subscription("sub-456");

        assert_eq!(creds.subscription_id, Some("sub-456".to_string()));
    }

    #[test]
    fn test_credentials_with_project() {
        let creds = ProviderCredentials::new(
            CloudProvider::GCP,
            CredentialType::ServiceAccount,
            "us-central1",
        )
        .with_project("my-project");

        assert_eq!(creds.project_id, Some("my-project".to_string()));
    }

    #[test]
    fn test_credentials_with_service_account() {
        let creds = ProviderCredentials::new(
            CloudProvider::GCP,
            CredentialType::ServiceAccount,
            "us-central1",
        )
        .with_service_account("service@project.iam.gserviceaccount.com");

        assert_eq!(
            creds.service_account,
            Some("service@project.iam.gserviceaccount.com".to_string())
        );
    }

    #[test]
    fn test_credentials_with_expiry() {
        let expiry = Utc::now() + chrono::TimeDelta::days(30);
        let creds =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::OAuth2, "us-east-1")
                .with_expiry(expiry);

        assert_eq!(creds.expires_at, Some(expiry));
    }

    #[test]
    fn test_credentials_is_expired() {
        let past = Utc::now() - chrono::TimeDelta::days(1);
        let creds =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::AccessKey, "us-east-1")
                .with_expiry(past);

        assert!(creds.is_expired());

        let future = Utc::now() + chrono::TimeDelta::days(30);
        let creds2 =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::AccessKey, "us-east-1")
                .with_expiry(future);

        assert!(!creds2.is_expired());
    }

    #[test]
    fn test_credentials_is_valid() {
        let creds =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::AccessKey, "us-east-1");
        assert!(creds.is_valid());

        let past = Utc::now() - chrono::TimeDelta::days(1);
        let creds2 =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::OAuth2, "us-east-1")
                .with_expiry(past);
        assert!(!creds2.is_valid());
    }

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig::new(
            "aws-production",
            CloudProvider::AWS,
            "https://ec2.amazonaws.com",
            "us-east-1",
            "cred-123",
        );

        assert_eq!(config.name, "aws-production");
        assert_eq!(config.provider, CloudProvider::AWS);
        assert_eq!(config.default_region, "us-east-1");
        assert_eq!(config.credential_id, "cred-123");
        assert!(config.enabled);
    }

    #[test]
    fn test_config_with_rate_limit() {
        let config = ProviderConfig::new(
            "test",
            CloudProvider::AWS,
            "https://endpoint",
            "us-east-1",
            "cred-1",
        )
        .with_rate_limit(1000);

        assert_eq!(config.rate_limit, Some(1000));
    }

    #[test]
    fn test_config_with_timeout() {
        let config = ProviderConfig::new(
            "test",
            CloudProvider::Azure,
            "https://endpoint",
            "eastus",
            "cred-1",
        )
        .with_timeout(60);

        assert_eq!(config.timeout_seconds, 60);
    }

    #[test]
    fn test_config_with_retries() {
        let config = ProviderConfig::new(
            "test",
            CloudProvider::GCP,
            "https://endpoint",
            "us-central1",
            "cred-1",
        )
        .with_retries(5);

        assert_eq!(config.retry_attempts, 5);
    }

    #[test]
    fn test_config_add_metadata() {
        let mut config = ProviderConfig::new(
            "test",
            CloudProvider::AWS,
            "https://endpoint",
            "us-east-1",
            "cred-1",
        );

        config.add_metadata("environment", "production");
        config.add_metadata("team", "platform");

        assert_eq!(config.metadata.len(), 2);
        assert_eq!(
            config.metadata.get("environment"),
            Some(&"production".to_string())
        );
    }

    #[test]
    fn test_config_enable_disable() {
        let mut config = ProviderConfig::new(
            "test",
            CloudProvider::AWS,
            "https://endpoint",
            "us-east-1",
            "cred-1",
        );

        assert!(config.enabled);

        config.disable();
        assert!(!config.enabled);

        config.enable();
        assert!(config.enabled);
    }

    #[test]
    fn test_provider_manager() {
        let mut manager = ProviderManager::new();

        let creds =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::AccessKey, "us-east-1");
        let id = manager.add_credentials(creds);

        assert_eq!(manager.credentials_count(), 1);
        assert!(manager.get_credentials(&id).is_some());
    }

    #[test]
    fn test_manager_add_config() {
        let mut manager = ProviderManager::new();

        let config = ProviderConfig::new(
            "test",
            CloudProvider::AWS,
            "https://endpoint",
            "us-east-1",
            "cred-1",
        );
        let id = manager.add_config(config);

        assert_eq!(manager.config_count(), 1);
        assert!(manager.get_config(&id).is_some());
    }

    #[test]
    fn test_manager_credentials_by_provider() {
        let mut manager = ProviderManager::new();

        manager.add_credentials(ProviderCredentials::new(
            CloudProvider::AWS,
            CredentialType::AccessKey,
            "us-east-1",
        ));
        manager.add_credentials(ProviderCredentials::new(
            CloudProvider::Azure,
            CredentialType::ServicePrincipal,
            "eastus",
        ));
        manager.add_credentials(ProviderCredentials::new(
            CloudProvider::AWS,
            CredentialType::OAuth2,
            "us-west-2",
        ));

        let aws_creds = manager.credentials_by_provider(&CloudProvider::AWS);
        assert_eq!(aws_creds.len(), 2);
    }

    #[test]
    fn test_manager_valid_credentials() {
        let mut manager = ProviderManager::new();

        let creds1 =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::AccessKey, "us-east-1");

        let past = Utc::now() - chrono::TimeDelta::days(1);
        let creds2 =
            ProviderCredentials::new(CloudProvider::Azure, CredentialType::OAuth2, "eastus")
                .with_expiry(past);

        manager.add_credentials(creds1);
        manager.add_credentials(creds2);

        let valid = manager.valid_credentials();
        assert_eq!(valid.len(), 1);
    }

    #[test]
    fn test_manager_expired_credentials() {
        let mut manager = ProviderManager::new();

        let past = Utc::now() - chrono::TimeDelta::days(1);
        let creds1 =
            ProviderCredentials::new(CloudProvider::AWS, CredentialType::OAuth2, "us-east-1")
                .with_expiry(past);

        let creds2 = ProviderCredentials::new(
            CloudProvider::Azure,
            CredentialType::ServicePrincipal,
            "eastus",
        );

        manager.add_credentials(creds1);
        manager.add_credentials(creds2);

        let expired = manager.expired_credentials();
        assert_eq!(expired.len(), 1);
    }

    #[test]
    fn test_manager_configs_by_provider() {
        let mut manager = ProviderManager::new();

        manager.add_config(ProviderConfig::new(
            "c1",
            CloudProvider::AWS,
            "e1",
            "us-east-1",
            "cred-1",
        ));
        manager.add_config(ProviderConfig::new(
            "c2",
            CloudProvider::Azure,
            "e2",
            "eastus",
            "cred-2",
        ));
        manager.add_config(ProviderConfig::new(
            "c3",
            CloudProvider::AWS,
            "e3",
            "us-west-2",
            "cred-3",
        ));

        let aws_configs = manager.configs_by_provider(&CloudProvider::AWS);
        assert_eq!(aws_configs.len(), 2);
    }

    #[test]
    fn test_manager_enabled_configs() {
        let mut manager = ProviderManager::new();

        let config1 = ProviderConfig::new("c1", CloudProvider::AWS, "e1", "us-east-1", "cred-1");
        let mut config2 = ProviderConfig::new("c2", CloudProvider::Azure, "e2", "eastus", "cred-2");
        config2.disable();

        manager.add_config(config1);
        manager.add_config(config2);

        let enabled = manager.enabled_configs();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_credential_type_equality() {
        assert_eq!(CredentialType::AccessKey, CredentialType::AccessKey);
        assert_ne!(CredentialType::AccessKey, CredentialType::ServiceAccount);
    }
}
