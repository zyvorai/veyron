// Notification delivery - routes notifications to their configured channels

/// Deliver a budget alert notification via its configured channel.
pub async fn deliver_budget_alert(
    alert: &crate::cost::budgets::BudgetAlert,
    budget_name: &str,
    utilization: f64,
) {
    use crate::cost::budgets::NotificationType;

    match &alert.notification_type {
        NotificationType::Log => {
            log::warn!(
                "Budget alert: '{}' at {:.1}% utilization (threshold: {:.0}%)",
                budget_name,
                utilization,
                alert.threshold_percent,
            );
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
            log::warn!(
                "Email delivery not yet implemented. Budget '{}' at {:.1}% (recipients: {:?})",
                budget_name,
                utilization,
                alert.recipients,
            );
        }
        NotificationType::Slack => {
            log::warn!(
                "Slack delivery not yet implemented. Budget '{}' at {:.1}%",
                budget_name,
                utilization,
            );
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
            log::warn!(
                "Email delivery not yet implemented: {} (recipients: {:?})",
                notification.subject,
                recipients,
            );
            notification.mark_failed("Email delivery not yet implemented");
        }
        NotificationChannel::Slack { channel, .. } => {
            log::warn!(
                "Slack delivery not yet implemented: {} (channel: {})",
                notification.subject,
                channel,
            );
            notification.mark_failed("Slack delivery not yet implemented");
        }
        NotificationChannel::PagerDuty { .. } => {
            log::warn!(
                "PagerDuty delivery not yet implemented: {}",
                notification.subject,
            );
            notification.mark_failed("PagerDuty delivery not yet implemented");
        }
        NotificationChannel::SMS { phone_numbers } => {
            log::warn!(
                "SMS delivery not yet implemented: {} (phones: {:?})",
                notification.subject,
                phone_numbers,
            );
            notification.mark_failed("SMS delivery not yet implemented");
        }
    }
}
