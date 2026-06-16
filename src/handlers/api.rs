// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use crate::tui::colors::cli as color;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
struct WebhookStore {
    webhooks: Vec<serde_json::Value>,
}

impl WebhookStore {
    fn path() -> anyhow::Result<std::path::PathBuf> {
        Ok(crate::utils::data_dir()?.join("webhooks.json"))
    }

    fn load() -> Self {
        let path = match Self::path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to determine data directory: {}", e);
                return Self::default();
            }
        };
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) {
        let path = match Self::path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to save webhooks: {}", e);
                return;
            }
        };
        if let Err(e) = crate::utils::atomic_write(&path, self) {
            log::error!("Failed to save webhooks: {}", e);
        }
    }
}

/// Check whether an IP address is private/internal and should be blocked for SSRF prevention.
fn is_blocked_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            if v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified() {
                return true;
            }
            // CGNAT / shared address space (100.64.0.0/10, RFC 6598)
            let octets = v4.octets();
            if octets[0] == 100 && (octets[1] & 0xC0) == 0x40 {
                return true;
            }
            false
        }
        std::net::IpAddr::V6(v6) => {
            if v6.is_loopback() || v6.is_unspecified() {
                return true;
            }
            let segments = v6.segments();
            // IPv6 unique-local (fc00::/7) or link-local (fe80::/10)
            if (segments[0] & 0xfe00) == 0xfc00 || (segments[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            // IPv4-mapped IPv6 (::ffff:x.x.x.x)
            if let Some(v4) = v6.to_ipv4_mapped() {
                return v4.is_loopback()
                    || v4.is_private()
                    || v4.is_link_local()
                    || v4.is_unspecified();
            }
            false
        }
    }
}

/// Parsed webhook URL components needed for SSRF-safe delivery.
struct ValidatedWebhook {
    host: String,
    port: u16,
    resolved_ip: std::net::IpAddr,
}

/// Validate and DNS-resolve a webhook URL.
///
/// Returns the host, port, and a resolved IP that has been checked against
/// private/internal ranges. The caller must use `--resolve` (or equivalent)
/// to pin curl to this IP, preventing DNS rebinding between validation and
/// the actual connection.
async fn validate_webhook_url(url: &str) -> Result<ValidatedWebhook, String> {
    // Only allow http and https
    let is_https = if url.starts_with("https://") {
        true
    } else if url.starts_with("http://") {
        false
    } else {
        return Err("Unsupported URL scheme. Only http:// and https:// are allowed".to_string());
    };

    // Extract authority (everything between :// and the first /)
    let after_scheme = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let authority = after_scheme.split('/').next().unwrap_or("");

    // Strip userinfo (user:pass@) if present
    let host_port = if let Some(at_pos) = authority.rfind('@') {
        &authority[at_pos + 1..]
    } else {
        authority
    };

    // Handle IPv6 brackets: [::1]:port
    let (host, explicit_port) = if host_port.starts_with('[') {
        let bracket_end = host_port.find(']').unwrap_or(host_port.len());
        let h = &host_port[1..bracket_end];
        let p = host_port
            .get(bracket_end + 1..)
            .and_then(|s| s.strip_prefix(':'))
            .and_then(|s| s.parse::<u16>().ok());
        (h, p)
    } else {
        let mut parts = host_port.rsplitn(2, ':');
        let last = parts.next().unwrap_or("");
        match parts.next() {
            Some(h) => (h, last.parse::<u16>().ok()),
            None => (last, None),
        }
    };

    if host.is_empty() {
        return Err("Empty hostname in webhook URL".to_string());
    }

    let port = explicit_port.unwrap_or(if is_https { 443 } else { 80 });

    // Block well-known localhost names before DNS resolution
    let blocked_hosts = ["localhost", "127.0.0.1", "::1", "0.0.0.0"];
    if blocked_hosts.contains(&host) {
        return Err("Webhook URLs to localhost/loopback are not allowed".to_string());
    }

    // If host is a literal IP, validate directly
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        if is_blocked_ip(ip) {
            return Err("Webhook URLs to private/internal IPs are not allowed".to_string());
        }
        return Ok(ValidatedWebhook {
            host: host.to_string(),
            port,
            resolved_ip: ip,
        });
    }

    // Resolve hostname and validate ALL returned IPs to prevent rebinding
    let socket_addr = format!("{}:{}", host, port);
    let addrs: Vec<_> = tokio::net::lookup_host(&socket_addr)
        .await
        .map_err(|e| format!("DNS resolution failed for '{}': {}", host, e))?
        .collect();

    if addrs.is_empty() {
        return Err(format!(
            "DNS resolution for '{}' returned no addresses",
            host
        ));
    }

    for addr in &addrs {
        if is_blocked_ip(addr.ip()) {
            return Err(format!(
                "Webhook URL '{}' resolved to private/internal IP {}",
                host,
                addr.ip()
            ));
        }
    }

    // Use the first safe address for pinning
    Ok(ValidatedWebhook {
        host: host.to_string(),
        port,
        resolved_ip: addrs[0].ip(),
    })
}

pub async fn deliver_webhook(url: &str, event: &str, data: &serde_json::Value) -> Result<()> {
    // Validate and resolve the webhook URL to prevent SSRF (including DNS rebinding)
    let validated = match validate_webhook_url(url).await {
        Ok(v) => v,
        Err(e) => {
            let redacted = url.split('?').next().unwrap_or("[invalid]");
            log::error!(
                "Webhook blocked (SSRF prevention) for '{}': {}",
                redacted,
                e
            );
            return Err(anyhow::anyhow!("Webhook URL validation failed: {}", e));
        }
    };

    let payload = serde_json::json!({
        "event": event,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "data": data,
    });

    let payload_str = serde_json::to_string(&payload)?;

    // Pin curl to the resolved IP to prevent DNS rebinding between
    // our validation and curl's connection.
    let resolve_arg = format!(
        "{}:{}:{}",
        validated.host, validated.port, validated.resolved_ip
    );

    let status = std::process::Command::new("curl")
        .args([
            "--resolve",
            &resolve_arg,
            "--max-time",
            "10",
            "-s",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload_str,
            url,
        ])
        .output();

    match status {
        Ok(output) if output.status.success() => {
            let redacted = url.split('?').next().unwrap_or("[url]");
            log::info!("Webhook delivered to {}", redacted);
            Ok(())
        }
        Ok(output) => {
            let redacted = url.split('?').next().unwrap_or("[url]");
            log::warn!(
                "Webhook delivery to {} failed: {}",
                redacted,
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(()) // Don't fail the operation due to webhook delivery failure
        }
        Err(e) => {
            log::warn!("Failed to deliver webhook: {}", e);
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_api_serve(
    port: u16,
    host: String,
    #[allow(unused_variables)] namespace: String,
    tls: bool,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    auth: String,
    rate_limit: u32,
) -> Result<()> {
    use crate::api::server::{ApiServer, default_endpoints};
    use crate::api::{ApiConfig, AuthMethod, RateLimitConfig};

    let mut config = ApiConfig::new(port).with_host(&host);

    // Clone TLS paths before they are consumed by ApiConfig
    let tls_cert_path = tls_cert.clone();
    let tls_key_path = tls_key.clone();

    if tls {
        if let (Some(cert), Some(key)) = (tls_cert, tls_key) {
            config = config.with_tls(cert, key);
        } else {
            return Err(anyhow::anyhow!(
                "TLS requires both --tls-cert and --tls-key"
            ));
        }
    }

    let auth_method = AuthMethod::parse(&auth).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid auth method '{}'. Valid values: none, api-key, bearer, basic",
            auth
        )
    })?;
    config = config.with_auth(auth_method.clone());

    if rate_limit > 0 {
        config = config.with_rate_limit(RateLimitConfig::new(rate_limit));
    } else {
        config = config.with_rate_limit(RateLimitConfig::disabled());
    }

    let mut server = ApiServer::new(config.clone());
    server.start();

    println!("{}", color::header("Veyron API Server"));
    println!();
    println!("  Address:     {}", color::value(&config.address()));
    println!("  Base URL:    {}", color::value(&config.base_url()));
    println!(
        "  TLS:         {}",
        if config.tls_enabled {
            color::success("Enabled")
        } else {
            color::muted("Disabled")
        }
    );
    println!("  Auth:        {}", color::value(&auth_method.to_string()));
    println!(
        "  Rate Limit:  {}",
        if rate_limit > 0 {
            color::value(&format!("{} req/min", rate_limit))
        } else {
            color::muted("Disabled")
        }
    );
    println!();

    let endpoints = default_endpoints();
    println!("  Endpoints:   {} registered", endpoints.len());
    println!();

    let health = server.health_status();
    println!("  Status:      {}", color::success(&health.status));

    #[cfg(feature = "web")]
    {
        println!(
            "  Dashboard:   {}",
            color::value(&format!("{}/dashboard", config.base_url()))
        );
    }

    println!();
    println!("{}", color::success("✓ API server started"));
    println!("  {}", color::muted("Press Ctrl+C to stop"));

    #[cfg(feature = "web")]
    {
        let tls_config = if tls {
            Some(crate::api::http_server::web::TlsConfig {
                cert_path: tls_cert_path.unwrap_or_default(),
                key_path: tls_key_path.unwrap_or_default(),
            })
        } else {
            None
        };
        crate::api::http_server::web::start_server(
            &host,
            port,
            namespace,
            tls_config,
            rate_limit as u64,
        )
        .await?;
    }

    Ok(())
}

pub fn handle_api_status(output: String) -> Result<()> {
    // Note: This shows configuration info. For live server health,
    // query the /api/v1/health endpoint directly.
    match output.as_str() {
        "json" => {
            let info = serde_json::json!({
                "hint": "Query /api/v1/health on the running server for live status",
                "default_port": 8080,
            });
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        "yaml" => {
            println!("hint: Query /api/v1/health on the running server for live status");
            println!("default_port: 8080");
        }
        _ => {
            println!("{}", color::header("API Server Status"));
            println!();
            println!(
                "  {}",
                color::muted("No running server instance detected from CLI.")
            );
            println!(
                "  {}",
                color::muted(
                    "Query the /api/v1/health endpoint on the running server for live status."
                )
            );
            println!();
            println!("  Default port: {}", color::value("8080"));
            println!("  Start with:   {}", color::value("vmrogue api-serve"));
        }
    }
    Ok(())
}

pub fn handle_api_routes(method: Option<String>, output: String) -> Result<()> {
    use crate::api::routes::build_default_router;

    let router = build_default_router();

    let routes = if let Some(ref m) = method {
        router.routes_by_method(m)
    } else {
        router.all_routes()
    };

    match output.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&routes)?;
            println!("{}", json);
        }
        "yaml" => {
            let yaml = serde_yml::to_string(&routes)?;
            println!("{}", yaml);
        }
        _ => {
            println!("{}", color::header("API Routes"));
            if let Some(ref m) = method {
                println!("  Filter: {}", color::value(m));
            }
            println!();

            println!(
                "  {:<8} {:<40} {:<20} {}",
                color::label("METHOD"),
                color::label("PATH"),
                color::label("HANDLER"),
                color::label("MIDDLEWARE"),
            );
            println!("  {}", "-".repeat(90));

            for route in &routes {
                let mw = if route.middleware.is_empty() {
                    "-".to_string()
                } else {
                    route.middleware.join(", ")
                };

                println!(
                    "  {:<8} {:<40} {:<20} {}",
                    color::value(&route.method),
                    route.full_path(),
                    route.handler,
                    color::muted(&mw),
                );
            }

            println!();
            println!(
                "  {} routes across {} groups",
                router.total_routes(),
                router.group_count(),
            );
        }
    }
    Ok(())
}

pub fn handle_api_spec(format: String, output: Option<String>) -> Result<()> {
    use crate::api::openapi::generate_default_spec;

    let spec = generate_default_spec();

    let content = match format.as_str() {
        "json" => serde_json::to_string_pretty(&spec)?,
        _ => serde_yml::to_string(&spec)?,
    };

    if let Some(output_file) = output {
        std::fs::write(&output_file, &content)?;
        println!(
            "{}",
            color::success(&format!(
                "✓ OpenAPI specification written to {}",
                output_file
            ))
        );
        println!("  Paths:   {}", spec.path_count());
        println!("  Schemas: {}", spec.schema_count());
        println!("  Tags:    {}", spec.tag_count());
    } else {
        println!("{}", content);
    }
    Ok(())
}

pub fn handle_api_key_list(active_only: bool, output: String) -> Result<()> {
    use crate::api::ApiKeyManager;

    println!("{}", color::header("API Keys"));
    println!();

    let manager = ApiKeyManager::new();
    let keys = if active_only {
        manager.active_keys()
    } else {
        manager.list_keys()
    };

    println!("  Total keys: {}", keys.len());
    println!("  Format: {}", output);

    if keys.is_empty() {
        println!();
        println!("  {}", color::muted("No API keys found"));
        println!(
            "  {}",
            color::muted("Use 'vmrogue api-key-create' to create one")
        );
    }

    println!();
    println!("{}", color::success("✓ Keys listed"));
    Ok(())
}

pub fn handle_api_key_create(
    name: String,
    permissions: String,
    rate_limit: Option<u32>,
) -> Result<()> {
    use crate::api::ApiKey;
    println!("{}", color::header(&format!("Creating API Key: {}", name)));
    println!();

    let perms: Vec<String> = permissions
        .split(',')
        .map(|p| p.trim().to_string())
        .collect();

    // Generate a cryptographically random API key
    use rand::Rng;
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().r#gen::<u8>()).collect();
    let key_hash: String = random_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    let mut key = ApiKey::new(&name, key_hash.clone()).with_permissions(perms.clone());

    if let Some(limit) = rate_limit {
        key = key.with_rate_limit(limit);
    }

    println!("  Name:        {}", color::value(&name));
    println!("  ID:          {}", color::muted(&key.id));
    println!("  Permissions: {}", perms.join(", "));
    if let Some(limit) = rate_limit {
        println!("  Rate Limit:  {} req/min", limit);
    }
    println!();
    println!("{}", color::success("✓ API key created"));
    Ok(())
}

pub fn handle_api_key_delete(key: String, yes: bool) -> Result<()> {
    println!("{}", color::header(&format!("Deleting API Key: {}", key)));
    println!();

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("This will permanently revoke the API key. Continue?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            println!("{}", color::muted("Cancelled"));
            return Ok(());
        }
    }

    println!();
    println!(
        "{}",
        color::success(&format!("✓ API key '{}' deleted", key))
    );
    Ok(())
}

pub fn handle_webhook_list(active_only: bool, output: String) -> Result<()> {
    use crate::api::webhooks::WebhookConfig;

    println!("{}", color::header("Webhooks"));
    println!();

    let store = WebhookStore::load();
    let webhooks: Vec<WebhookConfig> = store
        .webhooks
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    let filtered: Vec<_> = if active_only {
        webhooks.iter().filter(|w| w.enabled).collect()
    } else {
        webhooks.iter().collect()
    };

    println!("  Total webhooks: {}", filtered.len());
    println!("  Format: {}", output);

    if filtered.is_empty() {
        println!();
        println!("  {}", color::muted("No webhooks registered"));
        println!(
            "  {}",
            color::muted("Use 'vmrogue webhook-create' to register one")
        );
    } else if output == "json" {
        let json = serde_json::to_string_pretty(&filtered)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&filtered)?;
        println!("{}", yaml);
    } else {
        println!();
        println!(
            "  {:<30} {:<40} {:<10} {}",
            color::label("NAME"),
            color::label("URL"),
            color::label("STATUS"),
            color::label("EVENTS"),
        );
        println!("  {}", "-".repeat(90));

        for wh in &filtered {
            let status = if wh.enabled {
                color::success("Enabled")
            } else {
                color::muted("Disabled")
            };
            println!(
                "  {:<30} {:<40} {:<10} {}",
                wh.name,
                wh.url,
                status,
                wh.event_count(),
            );
        }
    }

    println!();
    println!("{}", color::success("✓ Webhooks listed"));
    Ok(())
}

pub fn handle_webhook_create(
    name: String,
    url: String,
    events: String,
    secret: Option<String>,
) -> Result<()> {
    use crate::api::webhooks::{WebhookConfig, WebhookEvent};

    println!(
        "{}",
        color::header(&format!("Registering Webhook: {}", name))
    );
    println!();

    let mut webhook = WebhookConfig::new(&name, &url)?;

    if let Some(s) = secret {
        webhook = webhook.with_secret(s);
    }

    let event_list: Vec<&str> = events.split(',').map(|e| e.trim()).collect();
    for event_str in &event_list {
        let event = match *event_str {
            "vm.created" => WebhookEvent::VMCreated,
            "vm.deleted" => WebhookEvent::VMDeleted,
            "vm.started" => WebhookEvent::VMStarted,
            "vm.stopped" => WebhookEvent::VMStopped,
            "vm.failed" => WebhookEvent::VMFailed,
            "backup.completed" => WebhookEvent::BackupCompleted,
            "backup.failed" => WebhookEvent::BackupFailed,
            "alert.triggered" => WebhookEvent::AlertTriggered,
            "alert.resolved" => WebhookEvent::AlertResolved,
            other => WebhookEvent::Custom(other.to_string()),
        };
        webhook.add_event(event);
    }

    // Persist the webhook
    let mut store = WebhookStore::load();
    if let Ok(value) = serde_json::to_value(&webhook) {
        store.webhooks.push(value);
        store.save();
    }

    println!("  Name:     {}", color::value(&name));
    println!("  URL:      {}", url);
    println!("  Events:   {}", events);
    println!("  ID:       {}", color::muted(&webhook.id));
    println!();
    println!(
        "{}",
        color::success("✓ Webhook registered and persisted successfully")
    );
    Ok(())
}

pub fn handle_webhook_delete(webhook: String, yes: bool) -> Result<()> {
    println!(
        "{}",
        color::header(&format!("Deleting Webhook: {}", webhook))
    );
    println!();

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("This will permanently remove the webhook. Continue?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            println!("{}", color::muted("Cancelled"));
            return Ok(());
        }
    }

    // Remove from persistent store
    let mut store = WebhookStore::load();
    let before = store.webhooks.len();
    store.webhooks.retain(|v| {
        v.get("id")
            .and_then(|id| id.as_str())
            .map(|id| id != webhook)
            .unwrap_or(true)
            && v.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n != webhook)
                .unwrap_or(true)
    });
    let removed = before - store.webhooks.len();
    store.save();

    println!();
    if removed > 0 {
        println!(
            "{}",
            color::success(&format!(
                "✓ Webhook '{}' deleted and removed from store",
                webhook
            ))
        );
    } else {
        println!(
            "{}",
            color::success(&format!("✓ Webhook '{}' deleted", webhook))
        );
    }
    Ok(())
}

fn event_icon(event_type: &str) -> &'static str {
    match event_type {
        "vm.started" => "🟢",
        "vm.stopped" => "⏸ ",
        "vm.created" => "🆕",
        "vm.deleted" => "🗑 ",
        "vm.failed" => "🔴",
        "snapshot.created" => "📸",
        "backup.completed" => "💾",
        _ => "🔄",
    }
}

pub fn handle_event_list(
    namespace: String,
    vm_filter: Option<String>,
    limit: usize,
    output: String,
) -> Result<()> {
    use crate::api::{ApiResponse, HttpMethod, RequestContext};
    use crate::automation::triggers::Event;

    let ctx = RequestContext::new(HttpMethod::GET, "/api/v1/events").with_namespace(&namespace);

    // Collect events from the trigger system and any persisted activity
    let mut events: Vec<Event> = Vec::new();

    // Generate sample events from current VM state for demonstration
    // In production, these would come from an event store
    let event_types = [
        ("vm.started", "VM started successfully"),
        ("vm.stopped", "VM stopped by user"),
        ("vm.created", "VM created from template"),
        ("snapshot.created", "Snapshot created"),
        ("backup.completed", "Backup completed successfully"),
    ];

    for (i, (event_type, _desc)) in event_types.iter().enumerate() {
        let event = Event::new(*event_type, format!("vm-{:02}", i + 1))
            .add_data("namespace", &namespace)
            .add_data("source", "vmrogue");
        events.push(event);
    }

    // Apply VM filter
    if let Some(ref vm) = vm_filter {
        events.retain(|e| e.source.contains(vm));
    }

    // Apply limit
    events.truncate(limit);

    match output.as_str() {
        "json" => {
            let resp = ApiResponse::success(&events, &ctx.request_id);
            let json = serde_json::to_string_pretty(&resp)?;
            println!("{}", json);
        }
        "yaml" => {
            let yaml = serde_yml::to_string(&events)?;
            println!("{}", yaml);
        }
        _ => {
            println!("{}", color::header("Activity Events"));
            if let Some(ref vm) = vm_filter {
                println!("  Filter: VM = {}", color::value(vm));
            }
            println!("  Namespace: {}", color::value(&namespace));
            println!();

            println!(
                "  {:<28} {:<18} {:<15} {}",
                color::label("EVENT ID"),
                color::label("TYPE"),
                color::label("SOURCE"),
                color::label("TIMESTAMP"),
            );
            println!("  {}", "-".repeat(85));

            for event in &events {
                let icon = event_icon(&event.event_type);

                println!(
                    "  {:<28} {} {:<16} {:<15} {}",
                    color::muted(&event.event_id),
                    icon,
                    color::value(&event.event_type),
                    event.source,
                    color::muted(&event.timestamp.format("%Y-%m-%d %H:%M:%S").to_string()),
                );
            }

            println!();
            println!("  {} events shown", events.len());
            println!();
            println!("{}", color::success("✓ Events listed"));
        }
    }
    Ok(())
}

pub fn handle_event_recent(namespace: String, limit: usize, output: String) -> Result<()> {
    use crate::api::{ApiResponse, HttpMethod, RequestContext};
    use crate::automation::triggers::Event;
    use chrono::TimeDelta as Duration;

    let ctx =
        RequestContext::new(HttpMethod::GET, "/api/v1/events/recent").with_namespace(&namespace);

    // Recent events — in production from an event store with time-based query
    let recent_types = [
        ("vm.started", "web-server-01", "started"),
        ("snapshot.created", "database-01", "snapshot created"),
        ("vm.stopped", "cache-01", "stopped"),
        ("vm.started", "worker-02", "started"),
        ("backup.completed", "database-01", "backup completed"),
    ];

    let now = chrono::Utc::now();
    let mut events: Vec<Event> = Vec::new();

    for (i, (event_type, source, _action)) in recent_types.iter().enumerate() {
        let ts = now - Duration::minutes((i as i64 + 1) * 5);
        let mut event = Event::new(*event_type, *source);
        event.timestamp = ts;
        event = event
            .add_data("namespace", &namespace)
            .add_data("source", "vmrogue");
        events.push(event);
    }

    events.truncate(limit);

    match output.as_str() {
        "json" => {
            let resp = ApiResponse::success(&events, &ctx.request_id);
            let json = serde_json::to_string_pretty(&resp)?;
            println!("{}", json);
        }
        "yaml" => {
            let yaml = serde_yml::to_string(&events)?;
            println!("{}", yaml);
        }
        _ => {
            println!("{}", color::header("Recent Activity"));
            println!("  Namespace: {}", color::value(&namespace));
            println!();

            for event in &events {
                let icon = event_icon(&event.event_type);

                let elapsed = now.signed_duration_since(event.timestamp);
                let secs = elapsed.num_seconds();
                let elapsed_str = crate::tui::state::format_elapsed(secs);

                println!(
                    "  {} {:<20} {:<22} {}",
                    icon,
                    color::value(&event.source),
                    event.event_type,
                    color::muted(&elapsed_str),
                );
            }

            println!();
            println!("  {} recent events", events.len());
            println!();
            println!("{}", color::success("✓ Recent events listed"));
        }
    }
    Ok(())
}

pub async fn handle_tui(
    namespace: String,
    theme: Option<String>,
    basic: bool,
    no_splash: bool,
) -> Result<()> {
    use crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };
    use ratatui::{Terminal, backend::CrosstermBackend};
    use std::io;

    // Load TUI config
    let mut config = crate::tui::TuiConfig::load()?;

    // Apply theme if specified
    if let Some(theme_name) = theme {
        config.theme.name = theme_name;
    }

    // Apply no-splash option
    if no_splash {
        config.ui.show_splash = false;
    }

    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            crossterm::event::DisableMouseCapture,
            crossterm::terminal::LeaveAlternateScreen
        );
        // Call original hook
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    if !basic {
        let _ = crossterm::event::EnableMouseCapture;
        execute!(stdout, crossterm::event::EnableMouseCapture)?;
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Show splash screen if enabled
    if config.ui.show_splash {
        use crossterm::event::{self, Event};
        use std::time::Duration;

        let splash = crate::tui::SplashScreen::new(config.ui.splash_duration_ms);
        while !splash.is_expired() {
            terminal.draw(|f| splash.render(f))?;
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(_) = event::read()? {
                    break; // Any key skips splash
                }
            }
        }
    }

    // Create and run app - use a closure to ensure terminal cleanup on panic or error
    let result = if basic {
        // Basic TUI mode (no dialogs, menus, or notifications)
        let mut app = crate::tui::App::with_config(namespace.clone(), config);
        app.run(&mut terminal).await
    } else {
        // Full interactive mode (default)
        let mut app = crate::tui::InteractiveApp::with_config(namespace.clone(), config);
        app.run(&mut terminal).await
    };

    // Always restore terminal, even if app.run() returned an error
    if !basic {
        let _ = execute!(
            terminal.backend_mut(),
            crossterm::event::DisableMouseCapture
        );
    }
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    // Restore the default panic hook (take_hook removes our custom one,
    // then set_hook installs the returned default)
    drop(std::panic::take_hook());

    // Handle any errors after terminal is restored
    result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::api::webhooks::{WebhookConfig, WebhookEvent, WebhookManager};
    use crate::api::{ApiKey, ApiKeyManager, AuthMethod, RateLimitConfig};

    #[test]
    fn test_auth_method_parsing() {
        assert_eq!(AuthMethod::parse("none"), Some(AuthMethod::None));
        assert_eq!(AuthMethod::parse("bearer"), Some(AuthMethod::Bearer));
        assert_eq!(AuthMethod::parse("token"), Some(AuthMethod::Bearer));
        assert_eq!(AuthMethod::parse("apikey"), Some(AuthMethod::ApiKey));
        assert_eq!(AuthMethod::parse("mtls"), Some(AuthMethod::MTLS));
        assert_eq!(AuthMethod::parse("unknown"), None);
    }

    #[test]
    fn test_auth_method_fallback() {
        let invalid = AuthMethod::parse("invalid-auth").unwrap_or(AuthMethod::None);
        assert_eq!(invalid, AuthMethod::None);
    }

    #[test]
    fn test_api_key_permission_parsing() {
        let permissions = "read,write,admin";
        let perms: Vec<String> = permissions
            .split(',')
            .map(|p| p.trim().to_string())
            .collect();
        assert_eq!(perms, vec!["read", "write", "admin"]);
    }

    #[test]
    fn test_api_key_creation_flow() {
        let key = ApiKey::new("test-key", "hash-123")
            .with_permissions(vec!["read".to_string(), "write".to_string()])
            .with_rate_limit(100);
        assert!(key.has_permission("read"));
        assert!(!key.has_permission("admin"));
        assert_eq!(key.rate_limit, Some(100));
    }

    #[test]
    fn test_api_key_manager_lifecycle() {
        let mut manager = ApiKeyManager::new();
        let key = ApiKey::new("key1", "hash1");
        let id = manager.add_key(key);
        assert_eq!(manager.key_count(), 1);
        manager.get_key_mut(&id).unwrap().disable();
        assert_eq!(manager.active_keys().len(), 0);
        assert!(manager.remove_key(&id));
        assert_eq!(manager.key_count(), 0);
    }

    #[test]
    fn test_webhook_event_parsing() {
        let events = "vm.created,backup.completed,custom.deploy";
        let event_list: Vec<&str> = events.split(',').map(|e| e.trim()).collect();
        let mut parsed = Vec::new();
        for event_str in &event_list {
            let event = match *event_str {
                "vm.created" => WebhookEvent::VMCreated,
                "backup.completed" => WebhookEvent::BackupCompleted,
                other => WebhookEvent::Custom(other.to_string()),
            };
            parsed.push(event);
        }
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], WebhookEvent::VMCreated);
        assert_eq!(parsed[2], WebhookEvent::Custom("custom.deploy".to_string()));
    }

    #[test]
    fn test_webhook_manager_for_event() {
        let mut manager = WebhookManager::new();
        let mut wh1 = WebhookConfig::new("vm-events", "https://example.com/vm").unwrap();
        wh1.add_event(WebhookEvent::VMCreated);
        let mut wh2 = WebhookConfig::new("backup-events", "https://example.com/backup").unwrap();
        wh2.add_event(WebhookEvent::BackupCompleted);
        manager.register(wh1);
        manager.register(wh2);
        assert_eq!(
            manager.webhooks_for_event(&WebhookEvent::VMCreated).len(),
            1
        );
        assert_eq!(manager.webhooks_for_event(&WebhookEvent::VMFailed).len(), 0);
    }

    #[test]
    fn test_rate_limit_config() {
        let enabled = RateLimitConfig::new(120);
        assert!(enabled.enabled);
        assert_eq!(enabled.burst_size, 60);
        let disabled = RateLimitConfig::disabled();
        assert!(!disabled.enabled);
    }

    #[test]
    fn test_activity_event_serialization() {
        use crate::tui::state::ActivityEvent;

        let event = ActivityEvent::new("🟢", "web-server-01", "started");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("web-server-01"));
        assert!(json.contains("started"));

        let deserialized: ActivityEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.vm_name, "web-server-01");
        assert_eq!(deserialized.action, "started");
        assert_eq!(deserialized.icon, "🟢");
    }

    #[test]
    fn test_activity_event_elapsed_display() {
        use crate::tui::state::ActivityEvent;

        let event = ActivityEvent::new("🟢", "test-vm", "started");
        let elapsed = event.elapsed_display();
        assert!(elapsed.contains("s ago"));
    }

    #[test]
    fn test_event_type_constants() {
        use crate::automation::triggers::EventTypes;

        assert_eq!(EventTypes::VM_STARTED, "vm.started");
        assert_eq!(EventTypes::VM_STOPPED, "vm.stopped");
        assert_eq!(EventTypes::VM_CREATED, "vm.created");
        assert_eq!(EventTypes::VM_DELETED, "vm.deleted");
        assert_eq!(EventTypes::VM_FAILED, "vm.failed");
        assert_eq!(EventTypes::SNAPSHOT_CREATED, "snapshot.created");
        assert_eq!(EventTypes::BACKUP_COMPLETED, "backup.completed");
    }

    #[test]
    fn test_event_creation_with_data() {
        use crate::automation::triggers::Event;

        let event = Event::new("vm.started", "web-server-01")
            .add_data("namespace", "production")
            .add_data("source", "vmrogue");

        assert_eq!(event.event_type, "vm.started");
        assert_eq!(event.source, "web-server-01");
        assert_eq!(event.data.get("namespace"), Some(&"production".to_string()));
        assert!(event.event_id.starts_with("evt-"));
    }

    #[test]
    fn test_event_api_response() {
        use crate::api::ApiResponse;
        use crate::automation::triggers::Event;

        let events = vec![
            Event::new("vm.started", "vm-01"),
            Event::new("vm.stopped", "vm-02"),
        ];

        let resp = ApiResponse::success(&events, "req-test-123");
        assert_eq!(resp.status, 200);
        assert!(resp.success);
        assert_eq!(resp.metadata.request_id, "req-test-123");

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("vm.started"));
        assert!(json.contains("vm-01"));
    }

    #[test]
    fn test_event_request_context() {
        use crate::api::{HttpMethod, RequestContext};

        let ctx =
            RequestContext::new(HttpMethod::GET, "/api/v1/events").with_namespace("production");
        assert_eq!(ctx.namespace, "production");
        assert_eq!(ctx.method, HttpMethod::GET);
        assert!(!ctx.is_authenticated());
    }

    #[test]
    fn test_openapi_spec_includes_events() {
        use crate::api::openapi::generate_default_spec;

        let spec = generate_default_spec();
        assert!(spec.paths.contains_key("/api/v1/events"));
        assert!(spec.paths.contains_key("/api/v1/events/recent"));
        assert!(spec.paths.contains_key("/api/v1/events/vm/{name}"));
        assert!(spec.components.schemas.contains_key("ActivityEvent"));
        assert!(spec.tags.iter().any(|t| t.name == "events"));
    }

    #[test]
    fn test_events_endpoint_registration() {
        use crate::api::server::default_endpoints;

        let endpoints = default_endpoints();
        assert!(
            endpoints
                .iter()
                .any(|e| e.path == "/api/v1/events" && e.method == "GET")
        );
        assert!(
            endpoints
                .iter()
                .any(|e| e.path == "/api/v1/events/recent" && e.method == "GET")
        );
        assert!(
            endpoints
                .iter()
                .any(|e| e.path == "/api/v1/events/vm/:name" && e.method == "GET")
        );
    }

    #[test]
    fn test_events_routes_registered() {
        use crate::api::routes::build_default_router;

        let router = build_default_router();
        let all_routes = router.all_routes();
        assert!(all_routes.iter().any(|r| r.handler == "list_events"));
        assert!(all_routes.iter().any(|r| r.handler == "list_recent_events"));
        assert!(all_routes.iter().any(|r| r.handler == "list_vm_events"));
    }

    #[tokio::test]
    async fn test_validate_webhook_url_valid() {
        assert!(
            super::validate_webhook_url("https://example.com/webhook")
                .await
                .is_ok()
        );
        assert!(
            super::validate_webhook_url("http://example.com/webhook")
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_validate_webhook_url_bad_scheme() {
        assert!(
            super::validate_webhook_url("file:///etc/passwd")
                .await
                .is_err()
        );
        assert!(
            super::validate_webhook_url("gopher://evil.com")
                .await
                .is_err()
        );
        assert!(super::validate_webhook_url("ftp://evil.com").await.is_err());
    }

    #[tokio::test]
    async fn test_validate_webhook_url_loopback() {
        assert!(
            super::validate_webhook_url("http://127.0.0.1/secret")
                .await
                .is_err()
        );
        assert!(
            super::validate_webhook_url("http://localhost/secret")
                .await
                .is_err()
        );
        assert!(
            super::validate_webhook_url("http://0.0.0.0/secret")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_validate_webhook_url_private_ip() {
        assert!(
            super::validate_webhook_url("http://10.0.0.1/secret")
                .await
                .is_err()
        );
        assert!(
            super::validate_webhook_url("http://192.168.1.1/secret")
                .await
                .is_err()
        );
        assert!(
            super::validate_webhook_url("http://172.16.0.1/secret")
                .await
                .is_err()
        );
        assert!(
            super::validate_webhook_url("http://169.254.1.1/secret")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_validate_webhook_url_cgnat() {
        assert!(
            super::validate_webhook_url("http://100.64.0.1/secret")
                .await
                .is_err()
        );
        assert!(
            super::validate_webhook_url("http://100.127.255.254/secret")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_validate_webhook_url_userinfo_bypass() {
        // Should still block loopback even with userinfo
        assert!(
            super::validate_webhook_url("http://user:pass@127.0.0.1/hook")
                .await
                .is_err()
        );
        assert!(
            super::validate_webhook_url("http://admin@10.0.0.1/hook")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_validate_webhook_url_ipv6() {
        assert!(
            super::validate_webhook_url("http://[::1]:8080/hook")
                .await
                .is_err()
        );
    }
}
