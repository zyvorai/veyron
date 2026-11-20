// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

/// Dashboard response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub panels: Vec<DashboardPanel>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Dashboard panel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPanel {
    pub id: String,
    pub title: String,
    pub panel_type: String,
    pub query: String,
    pub position: PanelPosition,
}

/// Panel position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelPosition {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/dashboards", get(list_dashboards))
        .with_state(state)
}

/// List dashboards stored as ConfigMaps labeled `veyron.io/type=dashboard`.
#[cfg(feature = "web")]
async fn list_dashboards(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<DashboardResponse>> {
    use k8s_openapi::api::core::v1::ConfigMap;

    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let client = s.client().client();
    let params = kube::api::ListParams::default().labels("veyron.io/type=dashboard");

    let cms = if namespace_scope::is_all_namespaces(&scope) {
        let api: kube::api::Api<ConfigMap> = kube::api::Api::all(client.clone());
        match api.list(&params).await {
            Ok(list) => list.items,
            Err(_) => {
                let mut out = Vec::new();
                for ns in namespace_scope::kubernetes_namespace_names(&client).await {
                    let api: kube::api::Api<ConfigMap> =
                        kube::api::Api::namespaced(client.clone(), &ns);
                    if let Ok(l) = api.list(&params).await {
                        out.extend(l.items);
                    }
                }
                out
            }
        }
    } else {
        let api: kube::api::Api<ConfigMap> = kube::api::Api::namespaced(client.clone(), &scope);
        match api.list(&params).await {
            Ok(list) => list.items,
            Err(_) => return Json(vec![]),
        }
    };

    let results: Vec<DashboardResponse> = cms
        .iter()
        .filter_map(|cm| {
            let data = cm.data.as_ref()?;
            // Parse panels from JSON stored in the "panels" key
            let panels: Vec<DashboardPanel> = data
                .get("panels")
                .and_then(|json_str| serde_json::from_str(json_str).ok())
                .unwrap_or_default();
            let ts = cm
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default();
            Some(DashboardResponse {
                id: cm.metadata.uid.clone().unwrap_or_default(),
                name: data.get("name").cloned().unwrap_or_default(),
                description: data.get("description").cloned().unwrap_or_default(),
                panels,
                created_by: data
                    .get("created_by")
                    .cloned()
                    .unwrap_or_else(|| "system".to_string()),
                created_at: ts.clone(),
                updated_at: data.get("updated_at").cloned().unwrap_or(ts),
            })
        })
        .collect();

    Json(results)
}
