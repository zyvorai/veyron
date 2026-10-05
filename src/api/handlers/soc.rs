// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

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
use crate::soc::{asm, collect, detections, event::SecurityEvent, export, hunts, playbooks, store};

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

#[derive(Debug, Clone, Deserialize)]
pub struct SocEventsQuery {
    pub namespace: Option<String>,
    pub severity: Option<String>,
    pub category: Option<String>,
    pub limit: Option<usize>,
    #[serde(default)]
    pub refresh: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SocDetectionsQuery {
    pub namespace: Option<String>,
    #[serde(default)]
    pub refresh: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AckDetectionRequest {
    pub acked_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SocEventsResponse {
    pub events: Vec<SecurityEvent>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SocDetectionsResponse {
    pub detections: Vec<detections::SocDetection>,
    pub open_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SocExportStatusResponse {
    pub exporters: Vec<export::SocExportStatus>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/soc/events", get(list_soc_events).post(sync_soc_events))
        .route("/soc/detections", get(list_soc_detections))
        .route("/soc/detections/:id", get(get_soc_detection))
        .route("/soc/detections/:id/ack", post(ack_soc_detection))
        .route("/soc/export/status", get(soc_export_status))
        .route("/soc/hunts", get(list_soc_hunts))
        .route("/soc/hunts/run", post(run_soc_hunt))
        .route("/soc/attack-surface", get(attack_surface))
        .route("/soc/playbooks", get(list_soc_playbooks))
        .route("/soc/playbooks/trigger", post(trigger_soc_playbook))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn soc_namespace(state: &SharedState, q_ns: Option<String>) -> (String, kube::Client) {
    let s = state.read().await;
    let ns = namespace_scope::resolve_opt(q_ns, &s.namespace);
    let client = s.client().client();
    (ns, client)
}

#[cfg(feature = "web")]
async fn list_soc_events(
    State(state): State<SharedState>,
    Query(q): Query<SocEventsQuery>,
) -> Json<SocEventsResponse> {
    let (ns, k8s) = soc_namespace(&state, q.namespace.clone()).await;
    let mut events = store::load_events(&k8s, &ns).await;
    if q.refresh || events.is_empty() {
        let kube_client = state.read().await.client().clone();
        let collected = collect::collect_from_cluster(&kube_client, &ns).await;
        store::append_events(&k8s, &ns, collected).await;
        events = store::load_events(&k8s, &ns).await;
    }
    if let Some(ref sev) = q.severity {
        let s = sev.to_lowercase();
        events.retain(|e| e.severity.to_lowercase() == s);
    }
    if let Some(ref cat) = q.category {
        let c = cat.to_lowercase();
        events.retain(|e| e.category.to_lowercase() == c);
    }
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let limit = q.limit.unwrap_or(200).min(500);
    let total = events.len();
    events.truncate(limit);
    Json(SocEventsResponse { events, total })
}

#[cfg(feature = "web")]
async fn sync_soc_events(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<SocEventsResponse> {
    let (ns, k8s) = soc_namespace(&state, q.namespace.clone()).await;
    let kube_client = state.read().await.client().clone();
    let collected = collect::collect_from_cluster(&kube_client, &ns).await;
    store::append_events(&k8s, &ns, collected).await;
    let events = store::load_events(&k8s, &ns).await;
    let total = events.len();
    Json(SocEventsResponse { events, total })
}

#[cfg(feature = "web")]
async fn list_soc_detections(
    State(state): State<SharedState>,
    Query(q): Query<SocDetectionsQuery>,
) -> Json<SocDetectionsResponse> {
    let (ns, k8s) = soc_namespace(&state, q.namespace.clone()).await;
    let kube_client = state.read().await.client().clone();
    let stored = store::load_detections(&k8s, &ns).await;
    let fresh = detections::evaluate_detections(&kube_client, &ns).await;
    let old_ids: std::collections::HashSet<String> = stored.iter().map(|d| d.id.clone()).collect();
    let merged = detections::merge_detections(fresh, &stored);
    store::save_detections(&k8s, &ns, &merged).await;

    for d in merged.iter().filter(|d| d.status == "open") {
        let ev = detections::detection_to_event(d);
        store::append_events(&k8s, &ns, vec![ev]).await;
        if !old_ids.contains(&d.id) {
            let det_json = serde_json::to_value(d).unwrap_or_default();
            playbooks::fire_detection_webhooks("detection.fired", &det_json).await;
        }
    }
    export::push_events_batch(
        &store::load_events(&k8s, &ns)
            .await
            .iter()
            .rev()
            .take(20)
            .cloned()
            .collect::<Vec<_>>(),
    )
    .await;

    let open_count = merged.iter().filter(|d| d.status == "open").count() as u32;
    Json(SocDetectionsResponse {
        detections: merged,
        open_count,
    })
}

#[cfg(feature = "web")]
async fn get_soc_detection(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Result<Json<detections::SocDetection>, StatusCode> {
    let (ns, k8s) = soc_namespace(&state, q.namespace).await;
    let list = store::load_detections(&k8s, &ns).await;
    list.into_iter()
        .find(|d| d.id == id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

#[cfg(feature = "web")]
async fn ack_soc_detection(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<DashboardNamespaceQuery>,
    Json(body): Json<AckDetectionRequest>,
) -> Result<Json<detections::SocDetection>, StatusCode> {
    let (ns, k8s) = soc_namespace(&state, q.namespace).await;
    let mut list = store::load_detections(&k8s, &ns).await;
    let det = list
        .iter_mut()
        .find(|d| d.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    det.status = "acknowledged".to_string();
    det.acked_by = body.acked_by.or(Some("analyst".into()));
    det.acked_at = Some(chrono::Utc::now().to_rfc3339());
    let out = det.clone();
    store::save_detections(&k8s, &ns, &list).await;
    let ev = detections::detection_to_event(&out);
    store::append_events(&k8s, &ns, vec![ev]).await;
    Ok(Json(out))
}

#[cfg(feature = "web")]
async fn soc_export_status() -> Json<SocExportStatusResponse> {
    Json(SocExportStatusResponse {
        exporters: export::export_status_all(),
    })
}

#[cfg(feature = "web")]
async fn list_soc_hunts(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<hunts::SavedHunt>> {
    let (ns, k8s) = soc_namespace(&state, q.namespace).await;
    Json(hunts::list_saved_hunts(&k8s, &ns).await)
}

#[cfg(feature = "web")]
async fn run_soc_hunt(
    Json(req): Json<hunts::RunHuntRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    hunts::run_hunt(&req).await.map(Json).map_err(|e| {
        log::warn!("soc hunt: {e}");
        StatusCode::BAD_GATEWAY
    })
}

#[cfg(feature = "web")]
async fn attack_surface(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<asm::AttackSurfaceReport> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace, &s.namespace);
    Json(asm::scan_attack_surface(s.client(), &scope).await)
}

#[cfg(feature = "web")]
async fn list_soc_playbooks(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<playbooks::SocPlaybook>> {
    let (ns, k8s) = soc_namespace(&state, q.namespace).await;
    Json(playbooks::list_playbooks(&k8s, &ns).await)
}

#[cfg(feature = "web")]
async fn trigger_soc_playbook(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
    Json(req): Json<playbooks::TriggerPlaybookRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (ns, k8s) = soc_namespace(&state, q.namespace).await;
    let pbs = playbooks::list_playbooks(&k8s, &ns).await;
    playbooks::trigger_playbook(&req, &pbs)
        .await
        .map(Json)
        .map_err(|e| {
            log::warn!("soc playbook: {e}");
            StatusCode::BAD_GATEWAY
        })
}

/// Record API audit from middleware (public to http_server).
#[cfg(feature = "web")]
pub async fn record_api_audit(
    state: &SharedState,
    actor: &str,
    method: &str,
    path: &str,
    outcome: &str,
) {
    if !is_soc_audit_path(path, method) {
        return;
    }
    let s = state.read().await;
    let ns = s.namespace.clone();
    let k8s = s.client().client();
    drop(s);
    let ev = collect::api_audit_event(actor, method, path, outcome, None);
    store::append_events(&k8s, &ns, vec![ev]).await;
}

#[cfg(feature = "web")]
fn is_soc_audit_path(path: &str, method: &str) -> bool {
    if method == "GET" || method == "HEAD" || method == "OPTIONS" {
        return false;
    }
    path.contains("/vms/")
        || path.contains("/snapshots")
        || path.contains("/rdp-expose")
        || path.contains("/expose")
        || path.contains("/guest-agent")
        || path.contains("/soc/")
}
