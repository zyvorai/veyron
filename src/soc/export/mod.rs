// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

#[cfg(feature = "web")]
pub mod elastic;
#[cfg(feature = "web")]
pub mod qradar;
#[cfg(feature = "web")]
pub mod sentinel;
#[cfg(feature = "web")]
pub mod splunk;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocExportStatus {
    pub backend: String,
    pub configured: bool,
    pub last_push_ok: bool,
    pub last_error: Option<String>,
    pub events_pushed: u64,
}

#[cfg(feature = "web")]
pub fn export_status_all() -> Vec<SocExportStatus> {
    vec![
        elastic::status(),
        splunk::status(),
        sentinel::status(),
        qradar::status(),
    ]
}

#[cfg(feature = "web")]
pub async fn push_events_batch(events: &[super::event::SecurityEvent]) {
    if events.is_empty() {
        return;
    }
    let _ = elastic::push_events(events).await;
    let _ = splunk::push_events(events).await;
    let _ = sentinel::push_events(events).await;
    let _ = qradar::push_events(events).await;
}

#[cfg(feature = "web")]
pub fn spawn_export_loop(namespace: String, client: kube::Client, interval_secs: u64) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            tick.tick().await;
            let events = super::store::load_events(&client, &namespace).await;
            let recent: Vec<_> = events.iter().rev().take(50).cloned().collect();
            push_events_batch(&recent).await;
        }
    });
}
