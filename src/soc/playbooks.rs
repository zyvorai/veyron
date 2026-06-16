// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, ListParams};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocPlaybook {
    pub id: String,
    pub name: String,
    pub event_type: String,
    pub webhook_url: String,
    pub active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TriggerPlaybookRequest {
    pub playbook_id: Option<String>,
    pub event_type: String,
    pub payload: Option<serde_json::Value>,
}

#[cfg(feature = "web")]
pub async fn list_playbooks(client: &kube::Client, namespace: &str) -> Vec<SocPlaybook> {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    let lp = ListParams::default().labels(&format!(
        "veyron.io/type={}",
        super::store::SOC_PLAYBOOKS_LABEL_VALUE
    ));
    let Ok(list) = api.list(&lp).await else {
        return default_playbooks();
    };
    let mut out: Vec<SocPlaybook> = list
        .items
        .iter()
        .filter_map(|cm| {
            let data = cm.data.as_ref()?;
            Some(SocPlaybook {
                id: cm.metadata.uid.clone().unwrap_or_default(),
                name: data.get("name")?.clone(),
                event_type: data.get("event_type")?.clone(),
                webhook_url: data.get("webhook_url")?.clone(),
                active: data.get("active").map(|v| v == "true").unwrap_or(true),
            })
        })
        .collect();
    if out.is_empty() {
        out = default_playbooks();
    }
    out
}

fn default_playbooks() -> Vec<SocPlaybook> {
    let url = std::env::var("VEYRON_SOAR_WEBHOOK_URL")
        .ok()
        .filter(|s| !s.is_empty());
    match url {
        Some(u) => vec![SocPlaybook {
            id: "default-soar".into(),
            name: "Generic SOAR webhook".into(),
            event_type: "detection.fired".into(),
            webhook_url: u,
            active: true,
        }],
        None => Vec::new(),
    }
}

#[cfg(feature = "web")]
pub async fn trigger_playbook(
    req: &TriggerPlaybookRequest,
    playbooks: &[SocPlaybook],
) -> anyhow::Result<serde_json::Value> {
    let pb = if let Some(ref id) = req.playbook_id {
        playbooks
            .iter()
            .find(|p| p.id == *id)
            .ok_or_else(|| anyhow::anyhow!("playbook not found"))?
    } else {
        playbooks
            .iter()
            .find(|p| p.active && p.event_type == req.event_type)
            .or_else(|| playbooks.first())
            .ok_or_else(|| anyhow::anyhow!("no playbook configured"))?
    };

    let body = req.payload.clone().unwrap_or_else(|| {
        serde_json::json!({
            "event_type": req.event_type,
            "source": "veyron",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    });

    let client = reqwest::Client::new();
    let resp = client.post(&pb.webhook_url).json(&body).send().await?;
    Ok(serde_json::json!({
        "playbook": pb.name,
        "status": resp.status().as_u16(),
        "ok": resp.status().is_success(),
    }))
}

#[cfg(feature = "web")]
pub async fn fire_detection_webhooks(event_type: &str, detection: &serde_json::Value) {
    if let Ok(url) = std::env::var("VEYRON_SOAR_WEBHOOK_URL") {
        if !url.is_empty() {
            let client = reqwest::Client::new();
            let body = serde_json::json!({
                "event_type": event_type,
                "detection": detection,
                "source": "veyron-soc",
            });
            let _ = client.post(&url).json(&body).send().await;
        }
    }
}
