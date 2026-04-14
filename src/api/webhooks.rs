use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Check if an IP address is private/internal
fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback()           // 127.0.0.0/8
            || v4.is_private()         // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
            || v4.is_link_local()      // 169.254.0.0/16
            || v4.is_broadcast()       // 255.255.255.255
            || v4.is_unspecified() // 0.0.0.0
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()           // ::1
            || v6.is_unspecified()     // ::
            // fc00::/7 (unique local) and fe80::/10 (link-local)
            || (v6.segments()[0] & 0xfe00) == 0xfc00
            || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Validate that a webhook URL is safe (HTTPS, no internal/private addresses)
fn validate_webhook_url(url: &str) -> anyhow::Result<()> {
    // Must be HTTPS
    if !url.starts_with("https://") {
        anyhow::bail!("Webhook URL must use HTTPS");
    }

    // Extract host from URL
    let after_scheme = url.strip_prefix("https://").unwrap_or(url);
    let host_port = after_scheme.split('/').next().unwrap_or("");
    let host = if host_port.starts_with('[') {
        // IPv6 bracket notation: [::1]:8080
        host_port
            .split(']')
            .next()
            .unwrap_or("")
            .trim_start_matches('[')
    } else {
        host_port.split(':').next().unwrap_or("")
    };

    if host.is_empty() {
        anyhow::bail!("Webhook URL has no host");
    }

    // Reject known dangerous hostnames
    let lower = host.to_lowercase();
    if lower == "localhost" || lower.ends_with(".localhost") || lower == "metadata.google.internal"
    {
        anyhow::bail!("Webhook URL must not point to internal addresses");
    }

    // If host parses as an IP, check if it's private
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        if is_private_ip(&ip) {
            anyhow::bail!("Webhook URL must not point to private/internal IP addresses");
        }
    }

    Ok(())
}

/// Webhook event types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebhookEvent {
    VMCreated,
    VMDeleted,
    VMStarted,
    VMStopped,
    VMRestarted,
    VMFailed,
    VMHealthChanged,
    SnapshotCreated,
    SnapshotRestored,
    BackupCompleted,
    BackupFailed,
    MigrationStarted,
    MigrationCompleted,
    AlertTriggered,
    AlertResolved,
    Custom(String),
}

impl std::fmt::Display for WebhookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookEvent::VMCreated => write!(f, "vm.created"),
            WebhookEvent::VMDeleted => write!(f, "vm.deleted"),
            WebhookEvent::VMStarted => write!(f, "vm.started"),
            WebhookEvent::VMStopped => write!(f, "vm.stopped"),
            WebhookEvent::VMRestarted => write!(f, "vm.restarted"),
            WebhookEvent::VMFailed => write!(f, "vm.failed"),
            WebhookEvent::VMHealthChanged => write!(f, "vm.health_changed"),
            WebhookEvent::SnapshotCreated => write!(f, "snapshot.created"),
            WebhookEvent::SnapshotRestored => write!(f, "snapshot.restored"),
            WebhookEvent::BackupCompleted => write!(f, "backup.completed"),
            WebhookEvent::BackupFailed => write!(f, "backup.failed"),
            WebhookEvent::MigrationStarted => write!(f, "migration.started"),
            WebhookEvent::MigrationCompleted => write!(f, "migration.completed"),
            WebhookEvent::AlertTriggered => write!(f, "alert.triggered"),
            WebhookEvent::AlertResolved => write!(f, "alert.resolved"),
            WebhookEvent::Custom(name) => write!(f, "custom.{}", name),
        }
    }
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub events: Vec<WebhookEvent>,
    #[serde(skip_serializing)]
    pub secret: Option<String>,
    pub headers: HashMap<String, String>,
    pub enabled: bool,
    pub retry_count: u32,
    pub retry_delay_secs: u64,
    pub timeout_secs: u64,
    pub created_at: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
    pub delivery_count: u64,
    pub failure_count: u64,
}

impl WebhookConfig {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> anyhow::Result<Self> {
        let name_str = name.into();
        let url_str = url.into();
        validate_webhook_url(&url_str)?;
        Ok(Self {
            id: format!(
                "wh-{}-{}",
                name_str.to_lowercase().replace(' ', "-"),
                Utc::now().timestamp()
            ),
            name: name_str,
            url: url_str,
            events: Vec::new(),
            secret: None,
            headers: HashMap::new(),
            enabled: true,
            retry_count: 3,
            retry_delay_secs: 10,
            timeout_secs: 30,
            created_at: Utc::now(),
            last_triggered: None,
            delivery_count: 0,
            failure_count: 0,
        })
    }

    pub fn add_event(&mut self, event: WebhookEvent) {
        if !self.events.contains(&event) {
            self.events.push(event);
        }
    }

    pub fn remove_event(&mut self, event: &WebhookEvent) -> bool {
        let len_before = self.events.len();
        self.events.retain(|e| e != event);
        self.events.len() < len_before
    }

    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Add a custom header. Validates that the key and value are safe HTTP header values
    /// (no CRLF injection, reasonable size, no forbidden headers).
    pub fn add_header(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> anyhow::Result<()> {
        let key = key.into();
        let value = value.into();

        // Reject CRLF injection
        if key.contains('\r') || key.contains('\n') || value.contains('\r') || value.contains('\n')
        {
            anyhow::bail!("Header key/value must not contain CR or LF characters");
        }
        // Reject oversized headers
        if key.len() > 256 || value.len() > 8192 {
            anyhow::bail!("Header key must be <= 256 bytes, value <= 8192 bytes");
        }
        // Reject forbidden headers
        let lower = key.to_lowercase();
        if ["host", "content-length", "transfer-encoding"].contains(&lower.as_str()) {
            anyhow::bail!("Header '{}' is forbidden for webhook custom headers", key);
        }
        // Validate header name: only visible ASCII chars except delimiters (RFC 7230)
        if !key
            .bytes()
            .all(|b| b > 32 && b < 127 && !b"\"(),/:;<=>?@[\\]{}".contains(&b))
        {
            anyhow::bail!("Header key contains invalid characters");
        }

        self.headers.insert(key, value);
        Ok(())
    }

    pub fn with_retry(mut self, count: u32, delay_secs: u64) -> Self {
        self.retry_count = count;
        self.retry_delay_secs = delay_secs;
        self
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn record_delivery(&mut self, success: bool) {
        self.delivery_count += 1;
        self.last_triggered = Some(Utc::now());
        if !success {
            self.failure_count += 1;
        }
    }

    pub fn subscribes_to(&self, event: &WebhookEvent) -> bool {
        self.events.contains(event)
    }

    pub fn success_rate(&self) -> f64 {
        if self.delivery_count == 0 {
            return 100.0;
        }
        ((self.delivery_count - self.failure_count) as f64 / self.delivery_count as f64) * 100.0
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

/// Webhook delivery payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub data: HashMap<String, String>,
    pub source: String,
}

impl WebhookPayload {
    pub fn new(event: &WebhookEvent) -> Self {
        Self {
            event: event.to_string(),
            timestamp: Utc::now(),
            data: HashMap::new(),
            source: "vmrogue".to_string(),
        }
    }

    pub fn add_data(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
    }

    pub fn data_count(&self) -> usize {
        self.data.len()
    }
}

/// Webhook manager
pub struct WebhookManager {
    webhooks: HashMap<String, WebhookConfig>,
}

impl WebhookManager {
    pub fn new() -> Self {
        Self {
            webhooks: HashMap::new(),
        }
    }

    pub fn register(&mut self, webhook: WebhookConfig) -> String {
        let id = webhook.id.clone();
        self.webhooks.insert(id.clone(), webhook);
        id
    }

    pub fn unregister(&mut self, id: &str) -> bool {
        self.webhooks.remove(id).is_some()
    }

    pub fn get(&self, id: &str) -> Option<&WebhookConfig> {
        self.webhooks.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut WebhookConfig> {
        self.webhooks.get_mut(id)
    }

    pub fn list(&self) -> Vec<&WebhookConfig> {
        self.webhooks.values().collect()
    }

    pub fn active_webhooks(&self) -> Vec<&WebhookConfig> {
        self.webhooks.values().filter(|w| w.enabled).collect()
    }

    pub fn webhooks_for_event(&self, event: &WebhookEvent) -> Vec<&WebhookConfig> {
        self.webhooks
            .values()
            .filter(|w| w.enabled && w.subscribes_to(event))
            .collect()
    }

    pub fn webhook_count(&self) -> usize {
        self.webhooks.len()
    }
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_display() {
        assert_eq!(WebhookEvent::VMCreated.to_string(), "vm.created");
        assert_eq!(WebhookEvent::VMDeleted.to_string(), "vm.deleted");
        assert_eq!(WebhookEvent::VMStarted.to_string(), "vm.started");
        assert_eq!(
            WebhookEvent::BackupCompleted.to_string(),
            "backup.completed"
        );
        assert_eq!(WebhookEvent::AlertTriggered.to_string(), "alert.triggered");
        assert_eq!(
            WebhookEvent::Custom("deploy".to_string()).to_string(),
            "custom.deploy"
        );
    }

    #[test]
    fn test_webhook_config_new() {
        let wh = WebhookConfig::new("test", "https://example.com/webhook").unwrap();
        assert_eq!(wh.name, "test");
        assert_eq!(wh.url, "https://example.com/webhook");
        assert!(wh.enabled);
        assert_eq!(wh.retry_count, 3);
        assert_eq!(wh.delivery_count, 0);
    }

    #[test]
    fn test_webhook_rejects_http() {
        assert!(WebhookConfig::new("test", "http://example.com").is_err());
    }

    #[test]
    fn test_webhook_rejects_internal_urls() {
        assert!(WebhookConfig::new("test", "https://127.0.0.1/hook").is_err());
        assert!(WebhookConfig::new("test", "https://localhost/hook").is_err());
        assert!(WebhookConfig::new("test", "https://169.254.169.254/latest").is_err());
        assert!(WebhookConfig::new("test", "https://192.168.1.1/hook").is_err());
        assert!(WebhookConfig::new("test", "https://10.0.0.1/hook").is_err());
    }

    #[test]
    fn test_webhook_add_event() {
        let mut wh = WebhookConfig::new("test", "https://example.com").unwrap();
        wh.add_event(WebhookEvent::VMCreated);
        wh.add_event(WebhookEvent::VMDeleted);
        wh.add_event(WebhookEvent::VMCreated); // Duplicate

        assert_eq!(wh.event_count(), 2);
        assert!(wh.subscribes_to(&WebhookEvent::VMCreated));
    }

    #[test]
    fn test_webhook_remove_event() {
        let mut wh = WebhookConfig::new("test", "https://example.com").unwrap();
        wh.add_event(WebhookEvent::VMCreated);

        assert!(wh.remove_event(&WebhookEvent::VMCreated));
        assert!(!wh.remove_event(&WebhookEvent::VMCreated));
        assert_eq!(wh.event_count(), 0);
    }

    #[test]
    fn test_webhook_with_secret() {
        let wh = WebhookConfig::new("test", "https://example.com")
            .unwrap()
            .with_secret("my-secret");
        assert_eq!(wh.secret, Some("my-secret".to_string()));
    }

    #[test]
    fn test_webhook_add_header() {
        let mut wh = WebhookConfig::new("test", "https://example.com").unwrap();
        wh.add_header("X-Custom", "value").unwrap();
        assert_eq!(wh.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_webhook_rejects_crlf_header() {
        let mut wh = WebhookConfig::new("test", "https://example.com").unwrap();
        assert!(wh.add_header("X-Bad\r\n", "value").is_err());
        assert!(wh.add_header("X-OK", "bad\nvalue").is_err());
    }

    #[test]
    fn test_webhook_rejects_forbidden_header() {
        let mut wh = WebhookConfig::new("test", "https://example.com").unwrap();
        assert!(wh.add_header("Host", "evil.com").is_err());
        assert!(wh.add_header("Content-Length", "999").is_err());
    }

    #[test]
    fn test_webhook_with_retry() {
        let wh = WebhookConfig::new("test", "https://example.com")
            .unwrap()
            .with_retry(5, 30);
        assert_eq!(wh.retry_count, 5);
        assert_eq!(wh.retry_delay_secs, 30);
    }

    #[test]
    fn test_webhook_enable_disable() {
        let mut wh = WebhookConfig::new("test", "https://example.com").unwrap();
        assert!(wh.enabled);

        wh.disable();
        assert!(!wh.enabled);

        wh.enable();
        assert!(wh.enabled);
    }

    #[test]
    fn test_webhook_record_delivery() {
        let mut wh = WebhookConfig::new("test", "https://example.com").unwrap();

        wh.record_delivery(true);
        wh.record_delivery(true);
        wh.record_delivery(false);

        assert_eq!(wh.delivery_count, 3);
        assert_eq!(wh.failure_count, 1);
        assert!(wh.last_triggered.is_some());
    }

    #[test]
    fn test_webhook_success_rate() {
        let mut wh = WebhookConfig::new("test", "https://example.com").unwrap();
        assert_eq!(wh.success_rate(), 100.0);

        wh.record_delivery(true);
        wh.record_delivery(false);

        assert_eq!(wh.success_rate(), 50.0);
    }

    #[test]
    fn test_webhook_payload_new() {
        let mut payload = WebhookPayload::new(&WebhookEvent::VMCreated);
        payload.add_data("vm_name", "test-vm");
        payload.add_data("namespace", "default");

        assert_eq!(payload.event, "vm.created");
        assert_eq!(payload.data_count(), 2);
        assert_eq!(payload.source, "vmrogue");
    }

    #[test]
    fn test_webhook_manager_register() {
        let mut manager = WebhookManager::new();
        let wh = WebhookConfig::new("test", "https://example.com").unwrap();
        let id = manager.register(wh);

        assert_eq!(manager.webhook_count(), 1);
        assert!(manager.get(&id).is_some());
    }

    #[test]
    fn test_webhook_manager_unregister() {
        let mut manager = WebhookManager::new();
        let id = manager.register(WebhookConfig::new("test", "https://example.com").unwrap());

        assert!(manager.unregister(&id));
        assert!(!manager.unregister(&id));
    }

    #[test]
    fn test_webhook_manager_active_webhooks() {
        let mut manager = WebhookManager::new();

        let mut disabled = WebhookConfig::new("disabled", "https://example.com").unwrap();
        disabled.disable();

        manager.register(WebhookConfig::new("active", "https://example.com").unwrap());
        manager.register(disabled);

        assert_eq!(manager.active_webhooks().len(), 1);
    }

    #[test]
    fn test_webhook_manager_webhooks_for_event() {
        let mut manager = WebhookManager::new();

        let mut wh1 = WebhookConfig::new("vm-events", "https://example.com/vm").unwrap();
        wh1.add_event(WebhookEvent::VMCreated);
        wh1.add_event(WebhookEvent::VMDeleted);

        let mut wh2 = WebhookConfig::new("backup-events", "https://example.com/backup").unwrap();
        wh2.add_event(WebhookEvent::BackupCompleted);

        manager.register(wh1);
        manager.register(wh2);

        let vm_hooks = manager.webhooks_for_event(&WebhookEvent::VMCreated);
        assert_eq!(vm_hooks.len(), 1);

        let backup_hooks = manager.webhooks_for_event(&WebhookEvent::BackupCompleted);
        assert_eq!(backup_hooks.len(), 1);

        let no_hooks = manager.webhooks_for_event(&WebhookEvent::AlertTriggered);
        assert_eq!(no_hooks.len(), 0);
    }
}
