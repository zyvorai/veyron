// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

#[cfg(feature = "web")]
use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::copilot::{
    BlueprintSaveRequest, BlueprintSaveResponse, CopilotAskRequest, CopilotChatRequest,
    CopilotResponse, GitOpsExportRequest, GitOpsExportResponse, YamlBuildRequest, alert_advisor,
    backup_advisor, catalog_advisor, cilium_advisor, compliance_advisor, copilot_ask, copilot_chat,
    cost_advisor, cost_advisor_vm, drift_advisor, export_gitops, finalize_copilot,
    forecast_advisor, gitops_advisor, guest_filesystem_report, guest_inspector,
    integrations_advisor, migration_advisor, network_lens, node_advisor, observability_advisor,
    performance_advisor, quota_advisor, recommend_template, save_blueprint, scheduling_explainer,
    scheduling_fleet_advisor, security_sentinel, security_sentinel_fleet, slo_advisor,
    status_snapshot, storage_doctor, velero_dr_advisor, vm_doctor, yaml_preview,
};

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};
#[cfg(feature = "web")]
use super::soc;
#[cfg(feature = "web")]
use crate::api::auth_context::AuthContext;
#[cfg(feature = "web")]
use crate::api::http_server::web::ApiRole;
#[cfg(feature = "web")]
use crate::copilot::plan;

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/experience/copilot/ask", post(copilot_ask_handler))
        .route("/experience/copilot/chat", post(copilot_chat_handler))
        .route(
            "/experience/copilot/plans/:id",
            get(plan_get_handler).delete(plan_cancel_handler),
        )
        .route(
            "/experience/copilot/plans/:id/execute",
            post(plan_execute_handler),
        )
        .route("/experience/copilot/status", get(copilot_status_handler))
        .route(
            "/experience/copilot/doctor/:ns/:name",
            get(copilot_doctor_handler),
        )
        .route(
            "/experience/copilot/scheduling/:ns/:name",
            get(copilot_scheduling_handler),
        )
        .route(
            "/experience/copilot/yaml/preview",
            post(copilot_yaml_handler),
        )
        .route(
            "/experience/copilot/blueprint/save",
            post(copilot_blueprint_save_handler),
        )
        .route(
            "/experience/copilot/gitops/export",
            post(copilot_gitops_export_handler),
        )
        .route(
            "/experience/copilot/recommend",
            post(copilot_recommend_handler),
        )
        .route("/experience/copilot/backup", get(copilot_backup_handler))
        .route("/experience/copilot/cost", get(copilot_cost_handler))
        .route(
            "/experience/copilot/cost/:ns/:name",
            get(copilot_cost_vm_handler),
        )
        .route(
            "/experience/copilot/network/:ns/:name",
            get(copilot_network_handler),
        )
        .route(
            "/experience/copilot/guest/:ns/:name",
            get(copilot_guest_handler),
        )
        .route(
            "/experience/copilot/guest/:ns/:name/filesystem",
            get(copilot_guest_filesystem_handler),
        )
        .route("/experience/copilot/storage", get(copilot_storage_handler))
        .route(
            "/experience/copilot/security/:ns/:name",
            get(copilot_security_handler),
        )
        .route(
            "/experience/copilot/security",
            get(copilot_security_fleet_handler),
        )
        .route(
            "/experience/copilot/performance",
            get(copilot_performance_handler),
        )
        .route("/experience/copilot/gitops", get(copilot_gitops_handler))
        .route(
            "/experience/copilot/forecast",
            get(copilot_forecast_handler),
        )
        .route(
            "/experience/copilot/integrations",
            get(copilot_integrations_handler),
        )
        .route(
            "/experience/copilot/scheduling",
            get(copilot_scheduling_fleet_handler),
        )
        .route(
            "/experience/copilot/compliance",
            get(copilot_compliance_handler),
        )
        .route(
            "/experience/copilot/observability",
            get(copilot_observability_handler),
        )
        .route("/experience/copilot/cilium", get(copilot_cilium_handler))
        .route("/experience/copilot/nodes", get(copilot_nodes_handler))
        .route("/experience/copilot/drift", get(copilot_drift_handler))
        .route("/experience/copilot/alerts", get(copilot_alerts_handler))
        .route("/experience/copilot/slo", get(copilot_slo_handler))
        .route(
            "/experience/copilot/migrations",
            get(copilot_migrations_handler),
        )
        .route("/experience/copilot/quotas", get(copilot_quotas_handler))
        .route("/experience/copilot/catalog", get(copilot_catalog_handler))
        .route(
            "/experience/copilot/velero-dr",
            get(copilot_velero_dr_handler),
        )
        .with_state(state)
}

#[cfg(feature = "web")]
async fn copilot_status_handler() -> Json<crate::copilot::CopilotStatusResponse> {
    Json(status_snapshot())
}

#[cfg(feature = "web")]
async fn copilot_chat_handler(
    State(state): State<SharedState>,
    auth: Option<Extension<AuthContext>>,
    Query(q): Query<DashboardNamespaceQuery>,
    Json(mut body): Json<CopilotChatRequest>,
) -> Json<crate::copilot::CopilotChatResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope =
        namespace_scope::resolve_opt(body.namespace.clone().or(q.namespace.clone()), &default_ns);
    // Fail closed: no auth context ⇒ the agent may not draft mutating plans.
    body.can_propose = auth
        .as_ref()
        .is_some_and(|Extension(c)| c.allows(&ApiRole::Write));
    let mut resp = copilot_chat(&client, &scope, &body).await;

    if !resp.plans.is_empty() {
        let actor = auth
            .as_ref()
            .map(|Extension(c)| c.subject.clone())
            .unwrap_or_else(|| "unknown".into());
        let kube = state.read().await.client().client();
        let mut saved = Vec::new();
        for mut plan in std::mem::take(&mut resp.plans) {
            plan.created_by = actor.clone();
            match plan::save(&kube, &default_ns, &plan).await {
                Ok(()) => {
                    soc::record_agent_audit(
                        &state,
                        &actor,
                        "plan_created",
                        &format!("{} step(s), needs {}", plan.steps.len(), plan.min_role),
                        "",
                        None,
                        "success",
                        &plan.id,
                        &plan.prompt,
                    )
                    .await;
                    saved.push(plan);
                }
                Err(e) => {
                    log::warn!("could not persist copilot plan {}: {e}", plan.id);
                    resp.message.content.push_str(
                        "\n\n(A plan was drafted but could not be saved, so it cannot be approved.)",
                    );
                }
            }
        }
        resp.plans = saved;
    }
    Json(resp)
}

#[cfg(feature = "web")]
fn plan_err(status: u16, msg: impl Into<String>) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST),
        Json(serde_json::json!({ "error": msg.into() })),
    )
        .into_response()
}

#[cfg(feature = "web")]
async fn load_plan(
    state: &SharedState,
    id: &str,
) -> Result<(plan::Plan, Option<String>, String, kube::Client), (u16, String)> {
    let (ns, kube) = {
        let s = state.read().await;
        (s.namespace.clone(), s.client().client())
    };
    match plan::load(&kube, &ns, id).await {
        Ok(Some((p, rv))) => Ok((p, rv, ns, kube)),
        Ok(None) => Err((404, "plan not found".into())),
        Err(e) => Err((502, format!("could not read plan: {e}"))),
    }
}

#[cfg(feature = "web")]
async fn plan_get_handler(State(state): State<SharedState>, Path(id): Path<String>) -> Response {
    match load_plan(&state, &id).await {
        Ok((p, ..)) => Json(p).into_response(),
        Err((code, msg)) => plan_err(code, msg),
    }
}

#[cfg(feature = "web")]
#[derive(Debug, serde::Deserialize, Default)]
struct PlanExecuteBody {
    /// Names of every destructive target, typed back by the user.
    #[serde(default)]
    confirm_names: Vec<String>,
    #[serde(default)]
    stop_on_error: Option<bool>,
}

#[cfg(feature = "web")]
async fn plan_execute_handler(
    State(state): State<SharedState>,
    auth: Option<Extension<AuthContext>>,
    Path(id): Path<String>,
    body: Option<Json<PlanExecuteBody>>,
) -> Response {
    let Some(Extension(caller)) = auth else {
        return plan_err(403, "no authenticated role");
    };
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let actor = caller.subject.clone();
    let (mut p, rv, ns, kube) = match load_plan(&state, &id).await {
        Ok(v) => v,
        Err((code, msg)) => return plan_err(code, msg),
    };
    if let Err((code, msg)) = plan::check_can_act(&p, &caller, &actor, true, chrono::Utc::now()) {
        return plan_err(code, msg);
    }
    if let Err(msg) = plan::check_confirmation(&p, &body.confirm_names) {
        return plan_err(400, msg);
    }
    // Claim the plan atomically: a concurrent execute loses the resourceVersion race.
    p.status = "executing".into();
    if plan::update(&kube, &ns, &p, rv).await.is_err() {
        return plan_err(409, "plan is already being executed");
    }
    let client = state.read().await.kube_client.clone();
    plan::execute_steps(&client, &mut p.steps, body.stop_on_error.unwrap_or(true)).await;
    p.status = plan::final_status(&p.steps).into();
    for s in &p.steps {
        let ok = s.result.as_ref().is_some_and(|r| r.ok);
        soc::record_agent_audit(
            &state,
            &actor,
            &format!("plan_step_{}", s.action),
            &s.preview,
            &s.namespace,
            (s.kind == "vm").then_some(s.name.as_str()),
            if ok { "success" } else { "failure" },
            &p.id,
            &p.prompt,
        )
        .await;
    }
    if let Err(e) = plan::update(&kube, &ns, &p, None).await {
        log::warn!("could not record result of plan {}: {e}", p.id);
    }
    Json(p).into_response()
}

#[cfg(feature = "web")]
async fn plan_cancel_handler(
    State(state): State<SharedState>,
    auth: Option<Extension<AuthContext>>,
    Path(id): Path<String>,
) -> Response {
    let Some(Extension(caller)) = auth else {
        return plan_err(403, "no authenticated role");
    };
    let actor = caller.subject.clone();
    let (mut p, rv, ns, kube) = match load_plan(&state, &id).await {
        Ok(v) => v,
        Err((code, msg)) => return plan_err(code, msg),
    };
    if let Err((code, msg)) = plan::check_can_act(&p, &caller, &actor, false, chrono::Utc::now()) {
        return plan_err(code, msg);
    }
    p.status = "cancelled".into();
    if plan::update(&kube, &ns, &p, rv).await.is_err() {
        return plan_err(409, "plan changed concurrently");
    }
    soc::record_agent_audit(
        &state,
        &actor,
        "plan_cancelled",
        "",
        "",
        None,
        "success",
        &p.id,
        &p.prompt,
    )
    .await;
    Json(p).into_response()
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
    let scope =
        namespace_scope::resolve_opt(body.namespace.clone().or(q.namespace.clone()), &default_ns);
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
    let scope =
        namespace_scope::resolve_opt(body.namespace.clone().or(q.namespace.clone()), &default_ns);
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
async fn copilot_cost_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    Json(finalize_copilot(cost_advisor(&client, &scope).await).await)
}

#[cfg(feature = "web")]
async fn copilot_cost_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Json<CopilotResponse> {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    Json(finalize_copilot(cost_advisor_vm(&client, &ns, &name).await).await)
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
async fn copilot_guest_filesystem_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Json<CopilotResponse> {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    Json(finalize_copilot(guest_filesystem_report(&client, &ns, &name).await).await)
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

#[cfg(feature = "web")]
async fn copilot_performance_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    Json(finalize_copilot(performance_advisor(&client, &scope).await).await)
}

#[cfg(feature = "web")]
async fn copilot_gitops_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    Json(finalize_copilot(gitops_advisor(&client, &scope).await).await)
}

#[cfg(feature = "web")]
async fn copilot_forecast_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    Json(finalize_copilot(forecast_advisor(&client, &scope).await).await)
}

#[cfg(feature = "web")]
async fn copilot_integrations_handler() -> Json<CopilotResponse> {
    Json(finalize_copilot(integrations_advisor().await).await)
}

#[cfg(feature = "web")]
async fn copilot_scheduling_fleet_handler(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CopilotResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    Json(finalize_copilot(scheduling_fleet_advisor(&client, &scope).await).await)
}

macro_rules! copilot_scope_handler {
    ($fn:ident, $advisor:expr) => {
        #[cfg(feature = "web")]
        async fn $fn(
            State(state): State<SharedState>,
            Query(q): Query<DashboardNamespaceQuery>,
        ) -> Json<CopilotResponse> {
            let (client, default_ns) = {
                let s = state.read().await;
                (s.kube_client.clone(), s.namespace.clone())
            };
            let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
            Json(finalize_copilot($advisor(&client, &scope).await).await)
        }
    };
}

copilot_scope_handler!(copilot_compliance_handler, compliance_advisor);
copilot_scope_handler!(copilot_observability_handler, observability_advisor);
copilot_scope_handler!(copilot_cilium_handler, cilium_advisor);
copilot_scope_handler!(copilot_nodes_handler, node_advisor);
copilot_scope_handler!(copilot_drift_handler, drift_advisor);
copilot_scope_handler!(copilot_alerts_handler, alert_advisor);
copilot_scope_handler!(copilot_slo_handler, slo_advisor);
copilot_scope_handler!(copilot_migrations_handler, migration_advisor);
copilot_scope_handler!(copilot_quotas_handler, quota_advisor);
copilot_scope_handler!(copilot_velero_dr_handler, velero_dr_advisor);

#[cfg(feature = "web")]
async fn copilot_catalog_handler(State(state): State<SharedState>) -> Json<CopilotResponse> {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    Json(finalize_copilot(catalog_advisor(&client).await).await)
}
