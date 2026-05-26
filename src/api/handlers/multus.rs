// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Multus NetworkAttachmentDefinition inventory and VM attach helpers.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadRecord {
    pub name: String,
    pub namespace: String,
    pub config: String,
    pub scope: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NadQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/network/nads", get(list_nads))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_nads(
    State(state): State<SharedState>,
    Query(q): Query<NadQuery>,
) -> Json<Vec<NadRecord>> {
    use kube::api::{Api, ApiResource, DynamicObject, ListParams};

    let s = state.read().await;
    let client = s.client().client();
    let scope = q.namespace.clone().unwrap_or_else(|| "all".to_string());

    let ar = ApiResource {
        group: "k8s.cni.cncf.io".to_string(),
        version: "v1".to_string(),
        api_version: "k8s.cni.cncf.io/v1".to_string(),
        kind: "NetworkAttachmentDefinition".to_string(),
        plural: "network-attachment-definitions".to_string(),
    };

    let api: Api<DynamicObject> = if scope == "all" {
        Api::all_with(client, &ar)
    } else {
        Api::namespaced_with(client, &scope, &ar)
    };

    let list = match api.list(&ListParams::default().limit(200)).await {
        Ok(l) => l.items,
        Err(_) => return Json(vec![]),
    };

    let rows: Vec<NadRecord> = list
        .into_iter()
        .filter_map(|o| {
            let name = o.metadata.name?;
            let namespace = o.metadata.namespace.unwrap_or_else(|| "default".to_string());
            let config = o
                .data
                .get("spec")
                .and_then(|spec| spec.get("config"))
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            Some(NadRecord {
                name,
                namespace,
                config,
                scope: if scope == "all" {
                    "cluster".to_string()
                } else {
                    "namespace".to_string()
                },
            })
        })
        .collect();

    Json(rows)
}
