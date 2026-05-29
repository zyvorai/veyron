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
    CopilotAskRequest, CopilotResponse, YamlBuildRequest, BlueprintSaveRequest, BlueprintSaveResponse,
    GitOpsExportRequest, GitOpsExportResponse, backup_advisor, copilot_ask, export_gitops,
    finalize_copilot, guest_inspector, network_lens, recommend_template, save_blueprint,
    scheduling_explainer, security_sentinel, security_sentinel_fleet, storage_doctor, vm_doctor,
    yaml_preview,
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
        .route(
            "/experience/copilot/blueprint/save",
            post(copilot_blueprint_save_handler),
        )
        .route(
            "/experience/copilot/gitops/export",
            post(copilot_gitops_export_handler),
        )
        .route("/experience/copilot/recommend", post(copilot_recommend_handler))
        .route("/experience/copilot/backup", get(copilot_backup_handler))
        .route(
            "/experience/copilot/network/:ns/:name",
            get(copilot_network_handler),
        )
        .route(
            "/experience/copilot/guest/:ns/:name",
            get(copilot_guest_handler),
        )
        .route("/experience/copilot/storage", get(copilot_storage_handler))
        .route(
            "/experience/copilot/security/:ns/:name",
            get(copilot_security_handler),
        )
        .route("/experience/copilot/security", get(copilot_security_fleet_handler))
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
    Json(finalize_copilot(copilot_ask(&client, &scope, &body).await).await)
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
    Json(finalize_copilot(vm_doctor(&client, &ns, &name).await).await)
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
    Json(finalize_copilot(scheduling_explainer(&client, &ns, &name).await).await)
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
    Json(finalize_copilot(yaml_preview(&client, &scope, body).await).await)
}

#[cfg(feature = "web")]
async fn copilot_blueprint_save_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
    Json(body): Json<BlueprintSaveRequest>,
) -> Result<Json<BlueprintSaveResponse>, (axum::http::StatusCode, String)> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(
        body.build.namespace.clone().or(q.namespace.clone()),
        &default_ns,
    );
    save_blueprint(&client, &scope, body)
        .await
        .map(Json)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))
}

#[cfg(feature = "web")]
async fn copilot_gitops_export_handler(
    Json(body): Json<GitOpsExportRequest>,
) -> Json<GitOpsExportResponse> {
    Json(export_gitops(body))
}

#[derive(serde::Deserialize)]
#[cfg(feature = "web")]
struct RecommendRequest {
    description: String,
}

#[cfg(feature = "web")]
async fn copilot_recommend_handler(Json(body): Json<RecommendRequest>) -> Json<CopilotResponse> {
    Json(finalize_copilot(recommend_template(&body.description)).await)
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
    Json(finalize_copilot(backup_advisor(&client, &scope).await).await)
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
    Json(finalize_copilot(network_lens(&client, &ns, &name).await).await)
}

#[cfg(feature = "web")]
async fn copilot_guest_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Json<CopilotResponse> {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    Json(finalize_copilot(guest_inspector(&client, &ns, &name).await).await)
}

#[cfg(feature = "web")]
async fn copilot_storage_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    Json(finalize_copilot(storage_doctor(&client, &scope).await).await)
}

#[cfg(feature = "web")]
async fn copilot_security_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Json<CopilotResponse> {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    Json(finalize_copilot(security_sentinel(&client, &ns, &name).await).await)
}

#[cfg(feature = "web")]
async fn copilot_security_fleet_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    Json(finalize_copilot(security_sentinel_fleet(&client, &scope).await).await)
}
