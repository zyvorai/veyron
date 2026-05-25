//! Optional external backend helpers (OpenCost, Loki, Alertmanager, Argo CD, notification channels).

#[cfg(feature = "web")]
use anyhow::{Context, Result};
#[cfg(feature = "web")]
use serde::Deserialize;
#[cfg(feature = "web")]
use std::time::Duration;

#[cfg(feature = "web")]
pub fn env_var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

#[cfg(feature = "web")]
pub fn cost_backend() -> &'static str {
    match env_var("VMROGUE_COST_BACKEND").as_deref() {
        Some("opencost") => "opencost",
        _ => "static",
    }
}

#[cfg(feature = "web")]
pub async fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("reqwest client")
}

#[cfg(feature = "web")]
pub async fn post_json(url: &str, body: &serde_json::Value, bearer: Option<&str>) -> Result<bool> {
    let client = http_client().await?;
    let mut req = client.post(url).json(body);
    if let Some(t) = bearer {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await.context("POST")?;
    Ok(resp.status().is_success())
}

#[cfg(feature = "web")]
pub async fn get_json(url: &str, bearer: Option<&str>) -> Result<serde_json::Value> {
    let client = http_client().await?;
    let mut req = client.get(url);
    if let Some(t) = bearer {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await.context("GET")?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    resp.json().await.context("json")
}

/// Deliver Slack incoming-webhook message.
#[cfg(feature = "web")]
pub async fn deliver_slack(webhook_url: &str, text: &str) -> Result<bool> {
    let body = serde_json::json!({ "text": text });
    post_json(webhook_url, &body, None).await
}

/// PagerDuty Events API v2.
#[cfg(feature = "web")]
pub async fn deliver_pagerduty(routing_key: &str, summary: &str, severity: &str) -> Result<bool> {
    let body = serde_json::json!({
        "routing_key": routing_key,
        "event_action": "trigger",
        "payload": {
            "summary": summary,
            "severity": severity,
            "source": "vmrogue"
        }
    });
    post_json("https://events.pagerduty.com/v2/enqueue", &body, None).await
}

/// Generic email HTTP relay (SendGrid/Mailgun-style JSON) or log-only fallback.
#[cfg(feature = "web")]
pub async fn deliver_email(relay_url: &str, subject: &str, body: &str, recipients: &[String]) -> Result<bool> {
    let payload = serde_json::json!({
        "subject": subject,
        "text": body,
        "to": recipients,
        "from": env_var("VMROGUE_EMAIL_FROM").unwrap_or_else(|| "vmrogue@local".to_string())
    });
    post_json(relay_url, &payload, env_var("VMROGUE_EMAIL_RELAY_TOKEN").as_deref()).await
}

/// OIDC userinfo validation — returns role string when token is valid.
#[cfg(feature = "web")]
pub async fn oidc_userinfo_role(token: &str) -> Result<Option<String>> {
    let url = env_var("VMROGUE_OIDC_USERINFO_URL").context("VMROGUE_OIDC_USERINFO_URL not set")?;
    let client = http_client().await?;
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .context("oidc userinfo")?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let info: serde_json::Value = resp.json().await.context("userinfo json")?;
    let role_claim =
        env_var("VMROGUE_OIDC_ROLE_CLAIM").unwrap_or_else(|| "groups".to_string());
    if let Some(v) = info.get(&role_claim) {
        if let Some(s) = v.as_str() {
            return Ok(Some(s.to_string()));
        }
        if let Some(arr) = v.as_array() {
            for item in arr {
                if let Some(s) = item.as_str() {
                    if s.contains("admin") {
                        return Ok(Some("admin".to_string()));
                    }
                    if s.contains("write") {
                        return Ok(Some("write".to_string()));
                    }
                }
            }
        }
    }
    Ok(Some("readonly".to_string()))
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct AmAlert {
    pub labels: Option<std::collections::HashMap<String, String>>,
    pub annotations: Option<std::collections::HashMap<String, String>>,
    pub status: Option<AmAlertStatus>,
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct AmAlertStatus {
    pub state: Option<String>,
}

#[cfg(feature = "web")]
pub async fn fetch_alertmanager_alerts(base: &str) -> Result<Vec<AmAlert>> {
    let url = format!("{}/api/v2/alerts", base.trim_end_matches('/'));
    let v = get_json(&url, env_var("VMROGUE_ALERTMANAGER_TOKEN").as_deref()).await?;
    Ok(serde_json::from_value(v).unwrap_or_else(|_| Vec::new()))
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    #[test]
    fn cost_backend_defaults_static() {
        std::env::remove_var("VMROGUE_COST_BACKEND");
        assert_eq!(cost_backend(), "static");
    }
}
