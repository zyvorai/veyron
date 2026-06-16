// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! VMRogue CRD management API handlers (vmrogue.io/v1alpha1).
//!
//! CRUD endpoints for VMRogueVM, VMRogueBlueprint, VMRoguePolicy,
//! VMRogueInsight, and VMRogueAction custom resources.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

#[cfg(feature = "web")]
use kube::{
    Api,
    api::{DeleteParams, Patch, PatchParams, PostParams},
};

use crate::operator_crds::*;
use rand::Rng;

/// Generic CRD list response (API responses only — dashboard consumes JSON).
#[derive(Debug, Clone, Serialize)]
pub struct CrdListResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[cfg(feature = "web")]
fn crd_list_failure_message(err: &kube::Error) -> String {
    match err {
        kube::Error::Api(e) if e.code == 404 => {
            "vmrogue.io CustomResourceDefinitions are not installed on this cluster. Install operator CRDs (see deploy/operator) and retry.".to_string()
        }
        kube::Error::Api(e) if e.code == 403 => {
            "Forbidden listing vmrogue.io resources — extend RBAC for the VMRogue service account.".to_string()
        }
        _ => format!("Could not list VMRogue resources: {err}"),
    }
}

#[cfg(feature = "web")]
fn crd_list_err<T: Serialize>(err: kube::Error) -> Json<CrdListResponse<T>> {
    Json(CrdListResponse {
        items: vec![],
        total: 0,
        message: Some(crd_list_failure_message(&err)),
    })
}

/// VMRogueVM summary for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMRogueVMSummary {
    pub name: String,
    pub namespace: String,
    pub template: Option<String>,
    pub cpu_cores: u32,
    pub memory: String,
    pub phase: Option<String>,
    pub kubevirt_vm: Option<String>,
    pub node: Option<String>,
    pub ip: Option<String>,
    pub created_at: String,
}

/// VMRogueBlueprint summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMRogueBlueprintSummary {
    pub name: String,
    pub namespace: String,
    pub description: Option<String>,
    pub vm_count: usize,
    pub phase: Option<String>,
    pub ready_vms: Option<i32>,
    pub tags: Vec<String>,
    pub created_at: String,
}

/// Full VMRogueBlueprint for Blueprint Studio editing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMRogueBlueprintDetail {
    pub name: String,
    pub namespace: String,
    pub description: Option<String>,
    pub vm_count: usize,
    pub phase: Option<String>,
    pub ready_vms: Option<i32>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub spec: VMRogueBlueprintSpec,
}

/// VMRoguePolicy summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMRoguePolicySummary {
    pub name: String,
    pub namespace: String,
    pub enabled: bool,
    pub enforcement: String,
    pub severity: String,
    pub framework: Option<String>,
    pub rule_count: usize,
    pub matching_vms: Option<i32>,
    pub violating_vms: Option<i32>,
    pub created_at: String,
}

/// VMRogueInsight summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMRogueInsightSummary {
    pub name: String,
    pub namespace: String,
    pub insight_type: String,
    pub severity: String,
    pub vm_ref: Option<String>,
    pub title: String,
    pub state: Option<String>,
    pub created_at: String,
}

/// VMRogueAction summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMRogueActionSummary {
    pub name: String,
    pub namespace: String,
    pub action_type: String,
    pub vm_ref: Option<String>,
    pub approved: bool,
    pub phase: Option<String>,
    pub message: Option<String>,
    pub created_at: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        // VMRogueVM endpoints
        .route("/crds/vmroguevms", get(list_vmrogue_vms))
        .route("/crds/vmroguevms", post(create_vmrogue_vm))
        .route(
            "/crds/vmroguevms/{ns}/{name}",
            get(get_vmrogue_vm).delete(delete_vmrogue_vm),
        )
        // VMRogueBlueprint endpoints
        .route("/crds/blueprints", get(list_blueprints))
        .route("/crds/blueprints", post(create_blueprint))
        .route(
            "/crds/blueprints/{ns}/{name}",
            get(get_blueprint)
                .put(update_blueprint)
                .delete(delete_blueprint),
        )
        // VMRoguePolicy endpoints
        .route("/crds/policies", get(list_policies))
        .route("/crds/policies", post(create_policy))
        .route(
            "/crds/policies/{ns}/{name}",
            get(get_policy).delete(delete_policy),
        )
        // VMRogueInsight endpoints
        .route("/crds/insights", get(list_insights))
        .route(
            "/crds/insights/{ns}/{name}",
            get(get_insight).delete(delete_insight),
        )
        // VMRogueAction endpoints
        .route("/crds/actions", get(list_actions))
        .route("/crds/actions", post(create_action))
        .route(
            "/crds/actions/{ns}/{name}",
            get(get_action).delete(delete_action),
        )
        .route("/crds/actions/{ns}/{name}/approve", post(approve_action))
        .route(
            "/crds/templates",
            get(list_catalog_templates).post(create_catalog_template),
        )
        .route(
            "/crds/profiles",
            get(list_catalog_profiles).post(create_catalog_profile),
        )
        .with_state(state)
}

// ── VMRogueVM handlers ─────────────────────────────────────────

#[cfg(feature = "web")]
async fn list_vmrogue_vms(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CrdListResponse<VMRogueVMSummary>> {
    let s = state.read().await;
    let client = s.client().client();
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);

    match namespace_scope::list_namespaced_resource::<VMRogueVM>(&client, &scope).await {
        Ok(list) => {
            let items: Vec<VMRogueVMSummary> = list
                .iter()
                .map(|vm| VMRogueVMSummary {
                    name: vm.metadata.name.clone().unwrap_or_default(),
                    namespace: vm.metadata.namespace.clone().unwrap_or_default(),
                    template: vm.spec.template.clone(),
                    cpu_cores: vm.spec.cpu.cores,
                    memory: vm.spec.memory.size.clone(),
                    phase: vm.status.as_ref().and_then(|s| s.phase.clone()),
                    kubevirt_vm: vm.status.as_ref().and_then(|s| s.kubevirt_vm_name.clone()),
                    node: vm.status.as_ref().and_then(|s| s.node_name.clone()),
                    ip: vm.status.as_ref().and_then(|s| s.ip_address.clone()),
                    created_at: vm
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|t| t.0.to_rfc3339())
                        .unwrap_or_default(),
                })
                .collect();
            let total = items.len();
            Json(CrdListResponse {
                items,
                total,
                message: None,
            })
        }
        Err(e) => crd_list_err(e),
    }
}

#[cfg(feature = "web")]
async fn get_vmrogue_vm(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Result<Json<VMRogueVMSummary>, StatusCode> {
    let s = state.read().await;
    let api: Api<VMRogueVM> = Api::namespaced(s.client().client().clone(), &ns);

    match api.get(&name).await {
        Ok(vm) => Ok(Json(VMRogueVMSummary {
            name: vm.metadata.name.clone().unwrap_or_default(),
            namespace: ns,
            template: vm.spec.template.clone(),
            cpu_cores: vm.spec.cpu.cores,
            memory: vm.spec.memory.size.clone(),
            phase: vm.status.as_ref().and_then(|s| s.phase.clone()),
            kubevirt_vm: vm.status.as_ref().and_then(|s| s.kubevirt_vm_name.clone()),
            node: vm.status.as_ref().and_then(|s| s.node_name.clone()),
            ip: vm.status.as_ref().and_then(|s| s.ip_address.clone()),
            created_at: vm
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default(),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateVMRogueVMRequest {
    pub namespace: Option<String>,
    pub name: Option<String>,
    #[serde(flatten)]
    pub spec: VMRogueVMSpec,
}

#[cfg(feature = "web")]
async fn create_vmrogue_vm(
    State(state): State<SharedState>,
    Json(body): Json<CreateVMRogueVMRequest>,
) -> Result<Json<VMRogueVMSummary>, StatusCode> {
    let s = state.read().await;
    let ns = body
        .namespace
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| s.namespace.clone());
    let api: Api<VMRogueVM> = Api::namespaced(s.client().client().clone(), &ns);

    let name = body.name.filter(|n| !n.is_empty()).unwrap_or_else(|| {
        format!(
            "vrvm-{}-{}",
            body.spec
                .template
                .clone()
                .unwrap_or_else(|| "vm".to_string())
                .replace('.', "-"),
            random_suffix()
        )
    });

    let vm = VMRogueVM::new(&name, body.spec);
    match api.create(&PostParams::default(), &vm).await {
        Ok(created) => Ok(Json(VMRogueVMSummary {
            name: created.metadata.name.clone().unwrap_or_default(),
            namespace: ns,
            template: created.spec.template.clone(),
            cpu_cores: created.spec.cpu.cores,
            memory: created.spec.memory.size.clone(),
            phase: None,
            kubevirt_vm: None,
            node: None,
            ip: None,
            created_at: created
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default(),
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "web")]
async fn delete_vmrogue_vm(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> StatusCode {
    let s = state.read().await;
    let api: Api<VMRogueVM> = Api::namespaced(s.client().client().clone(), &ns);
    match api.delete(&name, &DeleteParams::default()).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

// ── VMRogueBlueprint handlers ──────────────────────────────────

fn blueprint_detail(bp: &VMRogueBlueprint, ns: &str) -> VMRogueBlueprintDetail {
    VMRogueBlueprintDetail {
        name: bp.metadata.name.clone().unwrap_or_default(),
        namespace: ns.to_string(),
        description: bp.spec.description.clone(),
        vm_count: bp.spec.vms.len(),
        phase: bp.status.as_ref().and_then(|s| s.phase.clone()),
        ready_vms: bp.status.as_ref().and_then(|s| s.ready_vms),
        tags: bp.spec.tags.clone(),
        created_at: bp
            .metadata
            .creation_timestamp
            .as_ref()
            .map(|t| t.0.to_rfc3339())
            .unwrap_or_default(),
        spec: bp.spec.clone(),
    }
}

fn blueprint_summary(bp: &VMRogueBlueprint, ns: &str) -> VMRogueBlueprintSummary {
    VMRogueBlueprintSummary {
        name: bp.metadata.name.clone().unwrap_or_default(),
        namespace: ns.to_string(),
        description: bp.spec.description.clone(),
        vm_count: bp.spec.vms.len(),
        phase: bp.status.as_ref().and_then(|s| s.phase.clone()),
        ready_vms: bp.status.as_ref().and_then(|s| s.ready_vms),
        tags: bp.spec.tags.clone(),
        created_at: bp
            .metadata
            .creation_timestamp
            .as_ref()
            .map(|t| t.0.to_rfc3339())
            .unwrap_or_default(),
    }
}

#[cfg(feature = "web")]
async fn list_blueprints(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CrdListResponse<VMRogueBlueprintSummary>> {
    let s = state.read().await;
    let client = s.client().client();
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);

    match namespace_scope::list_namespaced_resource::<VMRogueBlueprint>(&client, &scope).await {
        Ok(list) => {
            let items: Vec<VMRogueBlueprintSummary> = list
                .iter()
                .map(|bp| {
                    blueprint_summary(bp, bp.metadata.namespace.as_deref().unwrap_or("default"))
                })
                .collect();
            let total = items.len();
            Json(CrdListResponse {
                items,
                total,
                message: None,
            })
        }
        Err(e) => crd_list_err(e),
    }
}

#[cfg(feature = "web")]
async fn get_blueprint(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Result<Json<VMRogueBlueprintDetail>, StatusCode> {
    let s = state.read().await;
    let api: Api<VMRogueBlueprint> = Api::namespaced(s.client().client().clone(), &ns);
    match api.get(&name).await {
        Ok(bp) => Ok(Json(blueprint_detail(&bp, &ns))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[cfg(feature = "web")]
async fn update_blueprint(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(spec): Json<VMRogueBlueprintSpec>,
) -> Result<Json<VMRogueBlueprintDetail>, StatusCode> {
    let s = state.read().await;
    let api: Api<VMRogueBlueprint> = Api::namespaced(s.client().client().clone(), &ns);
    let patch = Patch::Merge(serde_json::json!({ "spec": spec }));
    match api
        .patch(&name, &PatchParams::apply("vmrogue"), &patch)
        .await
    {
        Ok(bp) => Ok(Json(blueprint_detail(&bp, &ns))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "web")]
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBlueprintRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(flatten)]
    pub spec: VMRogueBlueprintSpec,
}

#[cfg(feature = "web")]
async fn create_blueprint(
    State(state): State<SharedState>,
    Json(body): Json<CreateBlueprintRequest>,
) -> Result<Json<VMRogueBlueprintSummary>, StatusCode> {
    let s = state.read().await;
    let ns = &s.namespace;
    let api: Api<VMRogueBlueprint> = Api::namespaced(s.client().client().clone(), ns);

    let name = body
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("bp-{}", random_suffix()));
    let spec = body.spec;
    let bp = VMRogueBlueprint::new(&name, spec.clone());
    match api.create(&PostParams::default(), &bp).await {
        Ok(created) => Ok(Json(VMRogueBlueprintSummary {
            name: created.metadata.name.clone().unwrap_or_default(),
            namespace: ns.clone(),
            description: spec.description,
            vm_count: spec.vms.len(),
            phase: None,
            ready_vms: None,
            tags: spec.tags,
            created_at: created
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default(),
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "web")]
async fn delete_blueprint(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> StatusCode {
    let s = state.read().await;
    let api: Api<VMRogueBlueprint> = Api::namespaced(s.client().client().clone(), &ns);
    match api.delete(&name, &DeleteParams::default()).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

// ── VMRoguePolicy handlers ────────────────────────────────────

#[cfg(feature = "web")]
async fn list_policies(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CrdListResponse<VMRoguePolicySummary>> {
    let s = state.read().await;
    let client = s.client().client();
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);

    match namespace_scope::list_namespaced_resource::<VMRoguePolicy>(&client, &scope).await {
        Ok(list) => {
            let items: Vec<VMRoguePolicySummary> = list
                .iter()
                .map(|p| VMRoguePolicySummary {
                    name: p.metadata.name.clone().unwrap_or_default(),
                    namespace: p.metadata.namespace.clone().unwrap_or_default(),
                    enabled: p.spec.enabled,
                    enforcement: p.spec.enforcement_action.clone(),
                    severity: p.spec.severity.clone(),
                    framework: p.spec.framework.clone(),
                    rule_count: p.spec.rules.len(),
                    matching_vms: p.status.as_ref().and_then(|s| s.matching_vms),
                    violating_vms: p.status.as_ref().and_then(|s| s.violating_vms),
                    created_at: p
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|t| t.0.to_rfc3339())
                        .unwrap_or_default(),
                })
                .collect();
            let total = items.len();
            Json(CrdListResponse {
                items,
                total,
                message: None,
            })
        }
        Err(e) => crd_list_err(e),
    }
}

#[cfg(feature = "web")]
async fn get_policy(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Result<Json<VMRoguePolicySummary>, StatusCode> {
    let s = state.read().await;
    let api: Api<VMRoguePolicy> = Api::namespaced(s.client().client().clone(), &ns);
    match api.get(&name).await {
        Ok(p) => Ok(Json(VMRoguePolicySummary {
            name: p.metadata.name.clone().unwrap_or_default(),
            namespace: ns,
            enabled: p.spec.enabled,
            enforcement: p.spec.enforcement_action.clone(),
            severity: p.spec.severity.clone(),
            framework: p.spec.framework.clone(),
            rule_count: p.spec.rules.len(),
            matching_vms: p.status.as_ref().and_then(|s| s.matching_vms),
            violating_vms: p.status.as_ref().and_then(|s| s.violating_vms),
            created_at: p
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default(),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[cfg(feature = "web")]
async fn create_policy(
    State(state): State<SharedState>,
    Json(spec): Json<VMRoguePolicySpec>,
) -> Result<Json<VMRoguePolicySummary>, StatusCode> {
    let s = state.read().await;
    let ns = &s.namespace;
    let api: Api<VMRoguePolicy> = Api::namespaced(s.client().client().clone(), ns);
    let name = format!("pol-{}", random_suffix());
    let p = VMRoguePolicy::new(&name, spec.clone());
    match api.create(&PostParams::default(), &p).await {
        Ok(created) => Ok(Json(VMRoguePolicySummary {
            name: created.metadata.name.clone().unwrap_or_default(),
            namespace: ns.clone(),
            enabled: spec.enabled,
            enforcement: spec.enforcement_action,
            severity: spec.severity,
            framework: spec.framework,
            rule_count: spec.rules.len(),
            matching_vms: None,
            violating_vms: None,
            created_at: created
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default(),
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "web")]
async fn delete_policy(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> StatusCode {
    let s = state.read().await;
    let api: Api<VMRoguePolicy> = Api::namespaced(s.client().client().clone(), &ns);
    match api.delete(&name, &DeleteParams::default()).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

// ── VMRogueInsight handlers ───────────────────────────────────

#[cfg(feature = "web")]
async fn list_insights(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CrdListResponse<VMRogueInsightSummary>> {
    let s = state.read().await;
    let client = s.client().client();
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);

    match namespace_scope::list_namespaced_resource::<VMRogueInsight>(&client, &scope).await {
        Ok(list) => {
            let items: Vec<VMRogueInsightSummary> = list
                .iter()
                .map(|i| VMRogueInsightSummary {
                    name: i.metadata.name.clone().unwrap_or_default(),
                    namespace: i.metadata.namespace.clone().unwrap_or_default(),
                    insight_type: i.spec.insight_type.clone(),
                    severity: i.spec.severity.clone(),
                    vm_ref: i.spec.vm_ref.clone(),
                    title: i.spec.title.clone(),
                    state: i.status.as_ref().and_then(|s| s.state.clone()),
                    created_at: i
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|t| t.0.to_rfc3339())
                        .unwrap_or_default(),
                })
                .collect();
            let total = items.len();
            Json(CrdListResponse {
                items,
                total,
                message: None,
            })
        }
        Err(e) => crd_list_err(e),
    }
}

#[cfg(feature = "web")]
async fn get_insight(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Result<Json<VMRogueInsightSummary>, StatusCode> {
    let s = state.read().await;
    let api: Api<VMRogueInsight> = Api::namespaced(s.client().client().clone(), &ns);
    match api.get(&name).await {
        Ok(i) => Ok(Json(VMRogueInsightSummary {
            name: i.metadata.name.clone().unwrap_or_default(),
            namespace: ns,
            insight_type: i.spec.insight_type.clone(),
            severity: i.spec.severity.clone(),
            vm_ref: i.spec.vm_ref.clone(),
            title: i.spec.title.clone(),
            state: i.status.as_ref().and_then(|s| s.state.clone()),
            created_at: i
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default(),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[cfg(feature = "web")]
async fn delete_insight(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> StatusCode {
    let s = state.read().await;
    let api: Api<VMRogueInsight> = Api::namespaced(s.client().client().clone(), &ns);
    match api.delete(&name, &DeleteParams::default()).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

// ── VMRogueAction handlers ────────────────────────────────────

#[cfg(feature = "web")]
async fn list_actions(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<CrdListResponse<VMRogueActionSummary>> {
    let s = state.read().await;
    let client = s.client().client();
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);

    match namespace_scope::list_namespaced_resource::<VMRogueAction>(&client, &scope).await {
        Ok(list) => {
            let items: Vec<VMRogueActionSummary> = list
                .iter()
                .map(|a| VMRogueActionSummary {
                    name: a.metadata.name.clone().unwrap_or_default(),
                    namespace: a.metadata.namespace.clone().unwrap_or_default(),
                    action_type: a.spec.action_type.clone(),
                    vm_ref: a.spec.vm_ref.clone(),
                    approved: a.spec.approved,
                    phase: a.status.as_ref().and_then(|s| s.phase.clone()),
                    message: a.status.as_ref().and_then(|s| s.message.clone()),
                    created_at: a
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|t| t.0.to_rfc3339())
                        .unwrap_or_default(),
                })
                .collect();
            let total = items.len();
            Json(CrdListResponse {
                items,
                total,
                message: None,
            })
        }
        Err(e) => crd_list_err(e),
    }
}

#[cfg(feature = "web")]
async fn get_action(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Result<Json<VMRogueActionSummary>, StatusCode> {
    let s = state.read().await;
    let api: Api<VMRogueAction> = Api::namespaced(s.client().client().clone(), &ns);
    match api.get(&name).await {
        Ok(a) => Ok(Json(VMRogueActionSummary {
            name: a.metadata.name.clone().unwrap_or_default(),
            namespace: ns,
            action_type: a.spec.action_type.clone(),
            vm_ref: a.spec.vm_ref.clone(),
            approved: a.spec.approved,
            phase: a.status.as_ref().and_then(|s| s.phase.clone()),
            message: a.status.as_ref().and_then(|s| s.message.clone()),
            created_at: a
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default(),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[cfg(feature = "web")]
async fn create_action(
    State(state): State<SharedState>,
    Json(spec): Json<VMRogueActionSpec>,
) -> Result<Json<VMRogueActionSummary>, StatusCode> {
    let s = state.read().await;
    let ns = &s.namespace;
    let api: Api<VMRogueAction> = Api::namespaced(s.client().client().clone(), ns);
    let name = format!("act-{}", random_suffix());
    let a = VMRogueAction::new(&name, spec.clone());
    match api.create(&PostParams::default(), &a).await {
        Ok(created) => Ok(Json(VMRogueActionSummary {
            name: created.metadata.name.clone().unwrap_or_default(),
            namespace: ns.clone(),
            action_type: spec.action_type,
            vm_ref: spec.vm_ref,
            approved: spec.approved,
            phase: None,
            message: None,
            created_at: created
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default(),
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "web")]
async fn delete_action(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> StatusCode {
    let s = state.read().await;
    let api: Api<VMRogueAction> = Api::namespaced(s.client().client().clone(), &ns);
    match api.delete(&name, &DeleteParams::default()).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

#[cfg(feature = "web")]
async fn approve_action(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> StatusCode {
    let s = state.read().await;
    let api: Api<VMRogueAction> = Api::namespaced(s.client().client().clone(), &ns);

    match api.get(&name).await {
        Ok(mut action) => {
            action.spec.approved = true;
            match api.replace(&name, &PostParams::default(), &action).await {
                Ok(_) => StatusCode::OK,
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
        Err(_) => StatusCode::NOT_FOUND,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogTemplateSummary {
    pub name: String,
    pub family: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogProfileSummary {
    pub name: String,
    pub cores: u32,
    pub memory: String,
    pub disk_size: String,
    pub description: Option<String>,
}

#[cfg(feature = "web")]
async fn list_catalog_templates(
    State(state): State<SharedState>,
) -> Json<CrdListResponse<CatalogTemplateSummary>> {
    use kube::Api;

    let s = state.read().await;
    let api: Api<VMTemplate> = Api::all(s.client().client());

    match api.list(&kube::api::ListParams::default().limit(200)).await {
        Ok(list) => {
            let items: Vec<CatalogTemplateSummary> = list
                .iter()
                .map(|t| CatalogTemplateSummary {
                    name: t.metadata.name.clone().unwrap_or_default(),
                    family: t.spec.family.clone(),
                    description: t.spec.description.clone(),
                    tags: t.spec.tags.clone(),
                })
                .collect();
            let total = items.len();
            Json(CrdListResponse {
                items,
                total,
                message: None,
            })
        }
        Err(e) => crd_list_err(e),
    }
}

#[cfg(feature = "web")]
async fn list_catalog_profiles(
    State(state): State<SharedState>,
) -> Json<CrdListResponse<CatalogProfileSummary>> {
    use kube::Api;

    let s = state.read().await;
    let api: Api<VMProfile> = Api::all(s.client().client());

    match api.list(&kube::api::ListParams::default().limit(100)).await {
        Ok(list) => {
            let items: Vec<CatalogProfileSummary> = list
                .iter()
                .map(|p| CatalogProfileSummary {
                    name: p.metadata.name.clone().unwrap_or_default(),
                    cores: p.spec.cores,
                    memory: p.spec.memory.clone(),
                    disk_size: p.spec.disk_size.clone(),
                    description: p.spec.description.clone(),
                })
                .collect();
            let total = items.len();
            Json(CrdListResponse {
                items,
                total,
                message: None,
            })
        }
        Err(e) => crd_list_err(e),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCatalogTemplateBody {
    pub name: String,
    /// Built-in Rust template to copy as VMTemplate `spec.default`.
    pub from_embedded: String,
    pub description: Option<String>,
    pub family: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCatalogProfileBody {
    pub name: String,
    pub cores: u32,
    pub memory: String,
    pub disk_size: String,
    pub description: Option<String>,
    /// Optional built-in profile name to copy sizing from.
    pub from_embedded: Option<String>,
}

#[cfg(feature = "web")]
async fn create_catalog_template(
    State(state): State<SharedState>,
    Json(body): Json<CreateCatalogTemplateBody>,
) -> Result<Json<CatalogTemplateSummary>, (StatusCode, String)> {
    use std::collections::BTreeMap;

    let name = body.name.trim();
    if name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "name required".to_string()));
    }
    let embedded = body.from_embedded.trim();
    if embedded.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "from_embedded required (built-in template name)".to_string(),
        ));
    }
    let config = crate::templates::TEMPLATES.get(embedded).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("unknown embedded template: {embedded}"),
        )
    })?;
    let default_spec = VMRogueVMSpec::from(&config);
    let family = body
        .family
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| crate::catalog::template_family(embedded));
    let recommended = crate::profiles::ProfileManager::suggest_profile_for_template(embedded)
        .map(|p| vec![p])
        .unwrap_or_default();

    let doc = VMTemplate {
        metadata: kube::api::ObjectMeta {
            name: Some(name.to_string()),
            labels: Some(BTreeMap::from([(
                "vmrogue.io/managed-by".to_string(),
                "veyron-api".to_string(),
            )])),
            ..Default::default()
        },
        spec: VMTemplateSpec {
            description: body
                .description
                .clone()
                .filter(|s| !s.is_empty())
                .or_else(|| Some(format!("VMRogue catalog template: {name}"))),
            tags: body.tags.clone().unwrap_or_else(|| vec![family.clone()]),
            family: Some(family),
            recommended_profiles: recommended,
            min_kubevirt_version: None,
            default: default_spec,
        },
        status: None,
    };

    let s = state.read().await;
    let api: Api<VMTemplate> = Api::all(s.client().client());
    match api.create(&PostParams::default(), &doc).await {
        Ok(created) => Ok(Json(CatalogTemplateSummary {
            name: created.metadata.name.clone().unwrap_or_default(),
            family: created.spec.family.clone(),
            description: created.spec.description.clone(),
            tags: created.spec.tags.clone(),
        })),
        Err(kube::Error::Api(e)) if e.code == 409 => Err((
            StatusCode::CONFLICT,
            format!("VMTemplate '{name}' already exists"),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("create VMTemplate: {e}"),
        )),
    }
}

#[cfg(feature = "web")]
async fn create_catalog_profile(
    State(state): State<SharedState>,
    Json(body): Json<CreateCatalogProfileBody>,
) -> Result<Json<CatalogProfileSummary>, (StatusCode, String)> {
    use std::collections::BTreeMap;

    let name = body.name.trim();
    if name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "name required".to_string()));
    }

    let (cores, memory, disk_size, description, sockets, threads, use_cases) =
        if let Some(ref embedded) = body.from_embedded {
            let profiles = crate::profiles::PROFILES.read().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("lock profiles: {e}"),
                )
            })?;
            let p = profiles.get(embedded).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("unknown embedded profile: {embedded}"),
                )
            })?;
            (
                p.cpu_cores,
                p.memory.clone(),
                p.disk_size.clone(),
                Some(p.description.clone()),
                p.cpu_sockets,
                p.cpu_threads,
                p.use_cases.clone(),
            )
        } else {
            (
                body.cores,
                body.memory.clone(),
                body.disk_size.clone(),
                body.description.clone(),
                1,
                1,
                vec![],
            )
        };

    if cores == 0 {
        return Err((StatusCode::BAD_REQUEST, "cores must be >= 1".to_string()));
    }
    if memory.trim().is_empty() || disk_size.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "memory and disk_size required".to_string(),
        ));
    }

    let doc = VMProfile {
        metadata: kube::api::ObjectMeta {
            name: Some(name.to_string()),
            labels: Some(BTreeMap::from([(
                "vmrogue.io/managed-by".to_string(),
                "veyron-api".to_string(),
            )])),
            ..Default::default()
        },
        spec: VMProfileSpec {
            description: description
                .filter(|s| !s.is_empty())
                .or_else(|| body.description.clone().filter(|s| !s.is_empty())),
            cores,
            sockets,
            threads,
            memory,
            disk_size,
            use_cases,
            recommended_templates: vec![],
        },
        status: None,
    };

    let s = state.read().await;
    let api: Api<VMProfile> = Api::all(s.client().client());
    match api.create(&PostParams::default(), &doc).await {
        Ok(created) => Ok(Json(CatalogProfileSummary {
            name: created.metadata.name.clone().unwrap_or_default(),
            cores: created.spec.cores,
            memory: created.spec.memory.clone(),
            disk_size: created.spec.disk_size.clone(),
            description: created.spec.description.clone(),
        })),
        Err(kube::Error::Api(e)) if e.code == 409 => Err((
            StatusCode::CONFLICT,
            format!("VMProfile '{name}' already exists"),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("create VMProfile: {e}"),
        )),
    }
}

/// Generate a random 8-char lowercase alphanumeric suffix for resource names.
#[cfg(feature = "web")]
fn random_suffix() -> String {
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..36u8);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + idx - 10) as char
            }
        })
        .collect()
}
