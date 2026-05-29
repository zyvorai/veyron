// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::copilot::{
    CopilotAskRequest, CopilotResponse, YamlBuildRequest, backup_advisor, copilot_ask,
    network_lens, recommend_template, scheduling_explainer, vm_doctor, yaml_preview,
};

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/experience/copilot/ask", post(copilot_ask_handler))
        .route("/experience/copilot/doctor/:ns/:name", get(copilot_doctor_handler))
        .route(
            "/experience/copilot/scheduling/:ns/:name",
            get(copilot_scheduling_handler),
        )
        .route("/experience/copilot/yaml/preview", post(copilot_yaml_handler))
        .route("/experience/copilot/recommend", post(copilot_recommend_handler))
        .route("/experience/copilot/backup", get(copilot_backup_handler))
        .route(
            "/experience/copilot/network/:ns/:name",
            get(copilot_network_handler),
        )
        .with_state(state)
}

#[cfg(feature = "web")]
async fn copilot_ask_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
    Json(body): Json<CopilotAskRequest>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(
        body.namespace.clone().or(q.namespace.clone()),
        &default_ns,
    );
    Json(copilot_ask(&client, &scope, &body).await)
}

#[cfg(feature = "web")]
async fn copilot_doctor_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Json<CopilotResponse> {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    Json(vm_doctor(&client, &ns, &name).await)
}

#[cfg(feature = "web")]
async fn copilot_scheduling_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Json<CopilotResponse> {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    Json(scheduling_explainer(&client, &ns, &name).await)
}

#[cfg(feature = "web")]
async fn copilot_yaml_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
    Json(body): Json<YamlBuildRequest>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(
        body.namespace.clone().or(q.namespace.clone()),
        &default_ns,
    );
    Json(yaml_preview(&client, &scope, body).await)
}

#[derive(serde::Deserialize)]
#[cfg(feature = "web")]
struct RecommendRequest {
    description: String,
}

#[cfg(feature = "web")]
async fn copilot_recommend_handler(Json(body): Json<RecommendRequest>) -> Json<CopilotResponse> {
    Json(recommend_template(&body.description))
}

#[cfg(feature = "web")]
async fn copilot_backup_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    Json(backup_advisor(&client, &scope).await)
}

#[cfg(feature = "web")]
async fn copilot_network_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Json<CopilotResponse> {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    Json(network_lens(&client, &ns, &name).await)
}