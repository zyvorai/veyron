// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use super::detections::SocDetection;
use super::event::SecurityEvent;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, Patch, PatchParams, PostParams};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use tokio::sync::Mutex;

static APPEND_EVENTS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn events_lock() -> &'static Mutex<()> {
    APPEND_EVENTS_LOCK.get_or_init(|| Mutex::new(()))
}

pub const SOC_EVENTS_LABEL_VALUE: &str = "soc-events";
pub const SOC_DETECTIONS_LABEL_VALUE: &str = "soc-detections";
pub const SOC_HUNTS_LABEL_VALUE: &str = "soc-hunt";
pub const SOC_PLAYBOOKS_LABEL_VALUE: &str = "soc-playbook";

const EVENTS_CM_NAME: &str = "veyron-soc-events";
const DETECTIONS_CM_NAME: &str = "veyron-soc-detections";
const EVENTS_KEY: &str = "events.json";
const DETECTIONS_KEY: &str = "detections.json";

pub const MAX_STORED_EVENTS: usize = 500;

/// A ConfigMap holds at most 1 MiB; stay under it so a write is never rejected.
const MAX_STORED_BYTES: usize = 900_000;

/// Newest events that fit both the count and the size budget (oldest are dropped first).
pub fn trim_events(events: &[SecurityEvent]) -> Vec<SecurityEvent> {
    let start = events.len().saturating_sub(MAX_STORED_EVENTS);
    let mut used = 2usize; // the surrounding [ ]
    let mut keep_from = events.len();
    for (i, e) in events.iter().enumerate().skip(start).rev() {
        let n = serde_json::to_string(e).map(|s| s.len() + 1).unwrap_or(0);
        if used + n > MAX_STORED_BYTES {
            break;
        }
        used += n;
        keep_from = i;
    }
    events[keep_from..].to_vec()
}

fn type_label(value: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("veyron.io/type".to_string(), value.to_string());
    m
}

#[cfg(feature = "web")]
pub async fn load_events(client: &kube::Client, namespace: &str) -> Vec<SecurityEvent> {
    read_json_array(client, namespace, EVENTS_CM_NAME, EVENTS_KEY).await
}

#[cfg(feature = "web")]
pub async fn save_events(client: &kube::Client, namespace: &str, events: &[SecurityEvent]) {
    let trimmed = trim_events(events);
    write_json_array(
        client,
        namespace,
        EVENTS_CM_NAME,
        SOC_EVENTS_LABEL_VALUE,
        EVENTS_KEY,
        &trimmed,
    )
    .await;
}

#[cfg(feature = "web")]
async fn try_load_events(
    client: &kube::Client,
    namespace: &str,
) -> Result<Vec<SecurityEvent>, kube::Error> {
    let api: kube::api::Api<k8s_openapi::api::core::v1::ConfigMap> =
        kube::api::Api::namespaced(client.clone(), namespace);
    let cm = api.get(EVENTS_CM_NAME).await?;
    let raw = cm
        .data
        .as_ref()
        .and_then(|d| d.get(EVENTS_KEY))
        .map(String::as_str)
        .unwrap_or("[]");
    Ok(serde_json::from_str(raw).unwrap_or_default())
}

#[cfg(feature = "web")]
pub async fn append_events(client: &kube::Client, namespace: &str, new_events: Vec<SecurityEvent>) {
    if new_events.is_empty() {
        return;
    }
    let _guard = events_lock().lock().await;
    let existing = match try_load_events(client, namespace).await {
        Ok(v) => v,
        Err(kube::Error::Api(ref e)) if e.code == 404 => vec![],
        Err(e) => {
            log::warn!("soc store: skipping append to avoid data loss on K8s error: {e}");
            return;
        }
    };
    let mut events = existing;
    events.extend(new_events);
    save_events(client, namespace, &events).await;
}

#[cfg(feature = "web")]
pub async fn load_detections(client: &kube::Client, namespace: &str) -> Vec<SocDetection> {
    read_json_array(client, namespace, DETECTIONS_CM_NAME, DETECTIONS_KEY).await
}

#[cfg(feature = "web")]
pub async fn save_detections(client: &kube::Client, namespace: &str, detections: &[SocDetection]) {
    let owned: Vec<SocDetection> = detections.to_vec();
    write_json_array(
        client,
        namespace,
        DETECTIONS_CM_NAME,
        SOC_DETECTIONS_LABEL_VALUE,
        DETECTIONS_KEY,
        &owned,
    )
    .await;
}

#[cfg(feature = "web")]
async fn read_json_array<T: serde::de::DeserializeOwned>(
    client: &kube::Client,
    namespace: &str,
    name: &str,
    key: &str,
) -> Vec<T> {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    let cm = match api.get(name).await {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let data = match cm.data.as_ref() {
        Some(d) => d,
        None => return Vec::new(),
    };
    let raw = match data.get(key) {
        Some(s) => s,
        None => return Vec::new(),
    };
    serde_json::from_str(raw).unwrap_or_default()
}

#[cfg(feature = "web")]
async fn write_json_array<T: serde::Serialize + ?Sized>(
    client: &kube::Client,
    namespace: &str,
    name: &str,
    label_value: &str,
    key: &str,
    value: &T,
) {
    let json = match serde_json::to_string(value) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("soc store serialize: {e}");
            return;
        }
    };
    let mut data = BTreeMap::new();
    data.insert(key.to_string(), json);

    let cm = ConfigMap {
        metadata: kube::api::ObjectMeta {
            name: Some(name.to_string()),
            labels: Some(type_label(label_value)),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };

    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    if api.get(name).await.is_ok() {
        let patch = Patch::Merge(cm);
        let _ = api.patch(name, &PatchParams::default(), &patch).await;
    } else {
        let _ = api.create(&PostParams::default(), &cm).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(msg: String) -> SecurityEvent {
        SecurityEvent::new("audit", "low", "veyron", msg)
    }

    #[test]
    fn stored_events_stay_under_the_configmap_limit() {
        // 200 events of ~20 KB each is 4 MB: without a size budget the ConfigMap write is
        // rejected and the audit log stays frozen on whatever it held before.
        let events: Vec<_> = (0..200)
            .map(|i| ev(format!("{i}{}", "x".repeat(20_000))))
            .collect();
        let kept = trim_events(&events);
        let size = serde_json::to_string(&kept).unwrap().len();
        assert!(size < 1_000_000, "{size}");
        assert!(!kept.is_empty());
        // newest survive
        assert!(kept.last().unwrap().message.starts_with("199x"));
    }

    #[test]
    fn small_logs_are_kept_whole_up_to_the_count_cap() {
        let events: Vec<_> = (0..600).map(|i| ev(format!("e{i}"))).collect();
        let kept = trim_events(&events);
        assert_eq!(kept.len(), MAX_STORED_EVENTS);
        assert_eq!(kept[0].message, "e100");
    }
}
