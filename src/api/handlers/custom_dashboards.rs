#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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

/// List dashboards stored as ConfigMaps labeled `vmrogue.io/type=dashboard`.
#[cfg(feature = "web")]
async fn list_dashboards(State(state): State<SharedState>) -> Json<Vec<DashboardResponse>> {
    use k8s_openapi::api::core::v1::ConfigMap;

    let s = state.read().await;
    let api: kube::api::Api<ConfigMap> =
        kube::api::Api::namespaced(s.client().client(), &s.namespace);
    let params = kube::api::ListParams::default().labels("vmrogue.io/type=dashboard");

    let cms = match api.list(&params).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<DashboardResponse> = cms
        .items
        .iter()
        .filter_map(|cm| {
            let data = cm.data.as_ref()?;
            Some(DashboardResponse {
                id: cm.metadata.uid.clone().unwrap_or_default(),
                name: data.get("name").cloned().unwrap_or_default(),
                description: data.get("description").cloned().unwrap_or_default(),
                panels: Vec::new(),
                created_by: data.get("created_by").cloned().unwrap_or_else(|| "system".to_string()),
                created_at: cm
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
                updated_at: cm
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            })
        })
        .collect();

    Json(results)
}
