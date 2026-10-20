// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

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

/// Returns the events whose ids have not been exported yet (oldest first, at
/// most `limit` of the newest), recording them in `seen`. Ids no longer present
/// in `events` are dropped from `seen` so it stays bounded.
pub fn take_unexported(
    events: &[super::event::SecurityEvent],
    seen: &mut std::collections::HashSet<String>,
    limit: usize,
) -> Vec<super::event::SecurityEvent> {
    let current: std::collections::HashSet<&str> = events.iter().map(|e| e.id.as_str()).collect();
    seen.retain(|id| current.contains(id.as_str()));
    let mut fresh: Vec<_> = events
        .iter()
        .filter(|e| !seen.contains(&e.id))
        .cloned()
        .collect();
    if fresh.len() > limit {
        fresh.drain(0..fresh.len() - limit);
    }
    for e in events {
        seen.insert(e.id.clone());
    }
    fresh
}

#[cfg(feature = "web")]
pub fn spawn_export_loop(namespace: String, client: kube::Client, interval_secs: u64) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        let mut seen = std::collections::HashSet::new();
        loop {
            tick.tick().await;
            let events = super::store::load_events(&client, &namespace).await;
            let fresh = take_unexported(&events, &mut seen, 50);
            push_events_batch(&fresh).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soc::event::SecurityEvent;

    fn ev(id: &str) -> SecurityEvent {
        serde_json::from_value(serde_json::json!({
            "id": id, "timestamp": "t", "severity": "low", "category": "c",
            "source": "s", "actor": "a", "target": "", "namespace": "n",
            "action": "x", "outcome": "ok", "message": "m"
        }))
        .unwrap()
    }

    #[test]
    fn exports_each_event_once_in_order() {
        let mut seen = std::collections::HashSet::new();
        let mut events = vec![ev("1"), ev("2")];
        let ids = |v: Vec<SecurityEvent>| v.into_iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(take_unexported(&events, &mut seen, 50)), ["1", "2"]);
        assert!(take_unexported(&events, &mut seen, 50).is_empty());
        events.push(ev("3"));
        assert_eq!(ids(take_unexported(&events, &mut seen, 50)), ["3"]);
        assert_eq!(ids(take_unexported(&events[..], &mut seen, 50)).len(), 0);
    }
}
