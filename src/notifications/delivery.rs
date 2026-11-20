// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Notification channel delivery (Slack, email relay, PagerDuty).

#[cfg(feature = "web")]
use crate::api::integrations;

/// Deliver a budget alert notification via its configured channel.
pub async fn deliver_budget_alert(
    alert: &crate::cost::budgets::BudgetAlert,
    budget_name: &str,
    utilization: f64,
) {
    use crate::cost::budgets::NotificationType;

    let summary = format!(
        "Budget alert: '{}' at {:.1}% utilization (threshold: {:.0}%)",
        budget_name, utilization, alert.threshold_percent
    );

    match &alert.notification_type {
        NotificationType::Log => {
            log::warn!("{}", summary);
        }
        NotificationType::Webhook => {
            use crate::api::webhooks::{WebhookConfig, WebhookEvent, WebhookPayload};

            let mut payload = WebhookPayload::new(&WebhookEvent::AlertTriggered);
            payload.add_data("budget_name", budget_name);
            payload.add_data("utilization_percent", format!("{:.1}", utilization));
            payload.add_data(
                "threshold_percent",
                format!("{:.0}", alert.threshold_percent),
            );

            for recipient in &alert.recipients {
                if let Ok(config) =
                    WebhookConfig::new(format!("budget-alert-{}", budget_name), recipient)
                {
                    match crate::api::webhooks::deliver_webhook(&config, &payload).await {
                        Ok(true) => log::info!("Budget alert delivered to {}", recipient),
                        Ok(false) => log::warn!("Budget alert delivery failed to {}", recipient),
                        Err(e) => log::error!("Budget alert delivery error: {}", e),
                    }
                }
            }
        }
        NotificationType::Email => {
            #[cfg(feature = "web")]
            if let Some(relay) = integrations::env_var("VEYRON_EMAIL_RELAY_URL") {
                match integrations::deliver_email(&relay, &summary, &summary, &alert.recipients)
                    .await
                {
                    Ok(true) => log::info!("Budget email alert sent for '{}'", budget_name),
                    Ok(false) | Err(_) => {
                        log::warn!("Budget email delivery failed for '{}'", budget_name)
                    }
                }
            } else {
                log::warn!(
                    "Email: set VEYRON_EMAIL_RELAY_URL. {} (recipients: {:?})",
                    summary,
                    alert.recipients
                );
            }
            #[cfg(not(feature = "web"))]
            log::warn!("Email delivery requires web feature. {}", summary);
        }
        NotificationType::Slack => {
            #[cfg(feature = "web")]
            if let Some(url) = integrations::env_var("VEYRON_SLACK_WEBHOOK_URL") {
                match integrations::deliver_slack(&url, &summary).await {
                    Ok(true) => log::info!("Budget Slack alert sent for '{}'", budget_name),
                    Ok(false) | Err(_) => log::warn!("Budget Slack delivery failed"),
                }
            } else {
                log::warn!("Slack: set VEYRON_SLACK_WEBHOOK_URL. {}", summary);
            }
            #[cfg(not(feature = "web"))]
            log::warn!("Slack delivery requires web feature. {}", summary);
        }
    }
}

/// Deliver an observability alert notification via its configured channel.
pub async fn deliver_observability_notification(
    notification: &mut crate::observability::alerts::Notification,
) {
    use crate::observability::alerts::NotificationChannel;

    match &notification.channel {
        NotificationChannel::Webhook { url, headers } => {
            use crate::api::webhooks::{WebhookConfig, WebhookEvent, WebhookPayload};

            let mut payload = WebhookPayload::new(&WebhookEvent::AlertTriggered);
            payload.add_data("subject", &notification.subject);
            payload.add_data("message", &notification.message);
            payload.add_data("alert_id", &notification.alert_id);

            match WebhookConfig::new("obs-alert", url) {
                Ok(mut config) => {
                    for (k, v) in headers {
                        let _ = config.add_header(k, v);
                    }
                    match crate::api::webhooks::deliver_webhook(&config, &payload).await {
                        Ok(true) => notification.mark_sent(),
                        Ok(false) => notification.mark_failed("HTTP delivery failed"),
                        Err(e) => notification.mark_failed(format!("Delivery error: {}", e)),
                    }
                }
                Err(e) => {
                    notification.mark_failed(format!("Invalid webhook URL: {}", e));
                }
            }
        }
        NotificationChannel::Email { recipients } => {
            #[cfg(feature = "web")]
            if let Some(relay) = integrations::env_var("VEYRON_EMAIL_RELAY_URL") {
                match integrations::deliver_email(
                    &relay,
                    &notification.subject,
                    &notification.message,
                    recipients,
                )
                .await
                {
                    Ok(true) => notification.mark_sent(),
                    Ok(false) => notification.mark_failed("Email relay HTTP failed"),
                    Err(e) => notification.mark_failed(format!("Email error: {}", e)),
                }
            } else {
                notification.mark_failed("Set VEYRON_EMAIL_RELAY_URL for email delivery");
            }
            #[cfg(not(feature = "web"))]
            notification.mark_failed("Email delivery requires web feature");
        }
        NotificationChannel::Slack { channel, .. } => {
            let text = format!(
                "{} — {} ({})",
                notification.subject, notification.message, channel
            );
            #[cfg(feature = "web")]
            if let Some(url) = integrations::env_var("VEYRON_SLACK_WEBHOOK_URL") {
                match integrations::deliver_slack(&url, &text).await {
                    Ok(true) => notification.mark_sent(),
                    Ok(false) => notification.mark_failed("Slack webhook failed"),
                    Err(e) => notification.mark_failed(format!("Slack error: {}", e)),
                }
            } else {
                notification.mark_failed("Set VEYRON_SLACK_WEBHOOK_URL for Slack delivery");
            }
            #[cfg(not(feature = "web"))]
            let _ = text;
            #[cfg(not(feature = "web"))]
            notification.mark_failed("Slack delivery requires web feature");
        }
        NotificationChannel::PagerDuty {
            integration_key, ..
        } => {
            #[cfg(feature = "web")]
            {
                let key = if integration_key.is_empty() {
                    integrations::env_var("VEYRON_PAGERDUTY_ROUTING_KEY").unwrap_or_default()
                } else {
                    integration_key.clone()
                };
                if key.is_empty() {
                    notification.mark_failed("Set VEYRON_PAGERDUTY_ROUTING_KEY or routing_key");
                    return;
                }
                match integrations::deliver_pagerduty(&key, &notification.subject, "error").await {
                    Ok(true) => notification.mark_sent(),
                    Ok(false) => notification.mark_failed("PagerDuty enqueue failed"),
                    Err(e) => notification.mark_failed(format!("PagerDuty error: {}", e)),
                }
            }
            #[cfg(not(feature = "web"))]
            notification.mark_failed("PagerDuty requires web feature");
        }
        NotificationChannel::SMS { phone_numbers } => {
            #[cfg(feature = "web")]
            if let Some(relay) = integrations::env_var("VEYRON_SMS_WEBHOOK_URL") {
                match integrations::deliver_sms(&relay, &notification.subject, phone_numbers).await
                {
                    Ok(true) => notification.mark_sent(),
                    Ok(false) => notification.mark_failed("SMS relay returned failure"),
                    Err(e) => notification.mark_failed(format!("SMS error: {}", e)),
                }
            } else {
                log::warn!(
                    "SMS: set VEYRON_SMS_WEBHOOK_URL. {} (phones: {:?})",
                    notification.subject,
                    phone_numbers
                );
                notification.mark_failed("SMS requires VEYRON_SMS_WEBHOOK_URL");
            }
            #[cfg(not(feature = "web"))]
            notification.mark_failed("SMS requires web feature");
        }
    }
}

/// Send a test notification to configured channels (admin diagnostics).
#[cfg(feature = "web")]
pub async fn deliver_test_notification() -> Vec<(String, bool, String)> {
    let mut results = Vec::new();
    let msg = "Veyron notification channel test";

    if let Some(url) = integrations::env_var("VEYRON_SLACK_WEBHOOK_URL") {
        let ok = integrations::deliver_slack(&url, msg)
            .await
            .unwrap_or(false);
        results.push(("slack".into(), ok, url));
    }
    if let Some(relay) = integrations::env_var("VEYRON_EMAIL_RELAY_URL") {
        let to = integrations::env_var("VEYRON_EMAIL_TEST_TO")
            .map(|s| vec![s])
            .unwrap_or_else(|| vec!["test@local".into()]);
        let ok = integrations::deliver_email(&relay, "Veyron test", msg, &to)
            .await
            .unwrap_or(false);
        results.push(("email".into(), ok, relay));
    }
    if let Some(key) = integrations::env_var("VEYRON_PAGERDUTY_ROUTING_KEY") {
        let ok = integrations::deliver_pagerduty(&key, msg, "info")
            .await
            .unwrap_or(false);
        results.push(("pagerduty".into(), ok, "events.pagerduty.com".into()));
    }
    results
}
