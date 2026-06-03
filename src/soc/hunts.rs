// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, ListParams};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedHunt {
    pub id: String,
    pub name: String,
    pub backend: String,
    pub query: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunHuntRequest {
    pub backend: String,
    pub query: String,
    #[serde(default = "default_time_range")]
    pub time_range: String,
}

fn default_time_range() -> String {
    "24h".to_string()
}

#[cfg(feature = "web")]
pub async fn list_saved_hunts(client: &kube::Client, namespace: &str) -> Vec<SavedHunt> {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    let lp = ListParams::default().labels(&format!(
        "vmrogue.io/type={}",
        super::store::SOC_HUNTS_LABEL_VALUE
    ));
    let Ok(list) = api.list(&lp).await else {
        return Vec::new();
    };
    list.items
        .iter()
        .filter_map(|cm| {
            let data = cm.data.as_ref()?;
            Some(SavedHunt {
                id: cm.metadata.uid.clone().unwrap_or_default(),
                name: data.get("name")?.clone(),
                backend: data.get("backend")?.clone(),
                query: data.get("query")?.clone(),
                description: data.get("description").cloned().unwrap_or_default(),
            })
        })
        .collect()
}

#[cfg(feature = "web")]
pub async fn run_hunt(req: &RunHuntRequest) -> anyhow::Result<serde_json::Value> {
    match req.backend.to_lowercase().as_str() {
        "elastic" => super::export::elastic::run_hunt(&req.query, &req.time_range).await,
        "splunk" => super::export::splunk::run_hunt(&req.query, &req.time_range).await,
        other => anyhow::bail!("unsupported hunt backend: {other}"),
    }
}
