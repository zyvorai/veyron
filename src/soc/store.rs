// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::detections::SocDetection;
use super::event::SecurityEvent;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, Patch, PatchParams, PostParams};
use std::collections::BTreeMap;

pub const SOC_EVENTS_LABEL_VALUE: &str = "soc-events";
pub const SOC_DETECTIONS_LABEL_VALUE: &str = "soc-detections";
pub const SOC_HUNTS_LABEL_VALUE: &str = "soc-hunt";
pub const SOC_PLAYBOOKS_LABEL_VALUE: &str = "soc-playbook";

const EVENTS_CM_NAME: &str = "veyron-soc-events";
const DETECTIONS_CM_NAME: &str = "veyron-soc-detections";
const EVENTS_KEY: &str = "events.json";
const DETECTIONS_KEY: &str = "detections.json";

pub const MAX_STORED_EVENTS: usize = 500;

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
    let start = events.len().saturating_sub(MAX_STORED_EVENTS);
    let trimmed = events[start..].to_vec();
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
pub async fn append_events(client: &kube::Client, namespace: &str, new_events: Vec<SecurityEvent>) {
    if new_events.is_empty() {
        return;
    }
    let mut events = load_events(client, namespace).await;
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
