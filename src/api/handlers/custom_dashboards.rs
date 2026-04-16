#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

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
pub fn router() -> Router {
    Router::new().route("/dashboards", get(list_dashboards))
}

#[cfg(feature = "web")]
async fn list_dashboards() -> impl IntoResponse {
    not_implemented("Custom dashboards")
}
