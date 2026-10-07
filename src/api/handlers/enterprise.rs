// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//! Durable owner-scoped operations and deterministic enterprise assessments.
use crate::api::{
    auth_context::AuthContext,
    http_server::web::{ApiRole, SharedState},
    tenant_scope,
    vm_backend::{VmBackend, selected},
};
use crate::enterprise::{
    blueprints,
    planning::{self, Capacity, PlacementNode, PlacementRequest},
};
use crate::kube::KubeClient;
use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use k8s_openapi::api::core::v1::{ConfigMap, Node, Pod};
use kube::{
    Api,
    api::{DeleteParams, ListParams, PostParams},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::Duration;

type ApiError = (StatusCode, Json<Value>);
fn error(status: StatusCode, message: impl ToString) -> ApiError {
    (status, Json(json!({"error":message.to_string()})))
}
fn bad(e: impl ToString) -> ApiError {
    error(StatusCode::BAD_REQUEST, e)
}
fn upstream(e: impl ToString) -> ApiError {
    error(StatusCode::BAD_GATEWAY, e)
}
fn scope(ctx: &AuthContext, ns: &str) -> Result<(), ApiError> {
    tenant_scope::authorize_namespace(ctx, ns).map_err(|e| error(StatusCode::FORBIDDEN, e))
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/enterprise/capabilities", get(capabilities))
        .route("/enterprise/blueprints/validate", post(blueprint))
        .route("/enterprise/placement", post(placement))
        .route("/enterprise/assess/maintenance", post(maintenance))
        .route("/enterprise/assess/migration", post(migration))
        .route("/enterprise/assess/gpu", post(gpu))
        .route("/enterprise/assess/recovery", post(recovery))
        .route("/enterprise/assess/fleet", post(fleet))
        .route("/enterprise/operations", get(list).post(submit))
        .route("/enterprise/operations/:id", get(read))
        .route("/enterprise/operations/:id/cancel", post(cancel))
        .with_state(state)
}
async fn capabilities() -> Json<Value> {
    Json(
        json!({"backend":selected().as_str(),"execution":{"start":true,"stop":true,"snapshot":false,"restore":false,"blueprint":false,"maintenance":false,"migration":false,"gpu":false,"fleet":false},"assessments":["blueprint","maintenance","migration","gpu","recovery","fleet"],"placement_live_inventory":selected().as_str()=="kubevirt","power_success":"observed controller state, not application health"}),
    )
}
async fn blueprint(
    Extension(ctx): Extension<AuthContext>,
    Json(bp): Json<blueprints::Blueprint>,
) -> Result<Json<blueprints::BlueprintPlan>, ApiError> {
    scope(&ctx, &bp.namespace)?;
    blueprints::validate(&bp).map(Json).map_err(bad)
}
async fn maintenance(
    Json(req): Json<planning::MaintenanceRequest>,
) -> Result<Json<Value>, ApiError> {
    let plan = planning::maintenance(&req).map_err(bad)?;
    Ok(Json(
        json!({"moves":plan,"evidence_only":true,"execution_supported":false,"warnings":["VM sizes, migration readiness and destination capacity are caller supplied; validate against live inventory before acting."]}),
    ))
}
async fn migration(Json(e): Json<planning::MigrationEvidence>) -> Json<planning::Assessment> {
    Json(planning::migration(&e))
}
async fn gpu(Json(e): Json<planning::GpuEvidence>) -> Json<planning::Assessment> {
    Json(planning::gpu(&e))
}
async fn recovery(Json(mut e): Json<planning::RecoveryEvidence>) -> Json<planning::Assessment> {
    e.now = chrono::Utc::now().timestamp();
    Json(planning::recovery(&e))
}
async fn fleet(Json(e): Json<Vec<planning::FleetCluster>>) -> Json<planning::Assessment> {
    Json(planning::fleet(&e))
}

fn resources(
    map: Option<&BTreeMap<String, k8s_openapi::apimachinery::pkg::api::resource::Quantity>>,
) -> Result<Capacity, String> {
    let mut r = Capacity::default();
    if let Some(map) = map {
        r.cpu_millis = map
            .get("cpu")
            .map(|q| planning::quantity(&q.0, true))
            .transpose()?
            .unwrap_or(0);
        r.memory_bytes = map
            .get("memory")
            .map(|q| planning::quantity(&q.0, false))
            .transpose()?
            .unwrap_or(0);
        r.gpu = map
            .get("nvidia.com/gpu")
            .map(|q| planning::quantity(&q.0, false))
            .transpose()?
            .unwrap_or(0);
    }
    Ok(r)
}
fn add(total: &mut Capacity, r: &Capacity) -> Result<(), String> {
    total.cpu_millis = total
        .cpu_millis
        .checked_add(r.cpu_millis)
        .ok_or("CPU overflow")?;
    total.memory_bytes = total
        .memory_bytes
        .checked_add(r.memory_bytes)
        .ok_or("memory overflow")?;
    total.gpu = total.gpu.checked_add(r.gpu).ok_or("GPU overflow")?;
    Ok(())
}
async fn placement(
    State(state): State<SharedState>,
    Json(req): Json<PlacementRequest>,
) -> Result<Json<Value>, ApiError> {
    if selected().as_str() != "kubevirt" {
        return Err(error(
            StatusCode::NOT_IMPLEMENTED,
            "Kairon reservation accounting is not implemented; refusing optimistic placement",
        ));
    }
    let client = state.read().await.client().client();
    let nodes = Api::<Node>::all(client.clone())
        .list(&ListParams::default())
        .await
        .map_err(upstream)?;
    let pods = Api::<Pod>::all(client)
        .list(&ListParams::default())
        .await
        .map_err(upstream)?;
    let mut used: BTreeMap<String, Capacity> = BTreeMap::new();
    for pod in pods.items {
        if pod
            .status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .is_some_and(|p| matches!(p, "Succeeded" | "Failed"))
        {
            continue;
        }
        let Some(spec) = pod.spec else { continue };
        let Some(node) = spec.node_name else { continue };
        // Summing all init containers is conservative (Kubernetes normally takes a max).
        for c in spec
            .containers
            .iter()
            .chain(spec.init_containers.iter().flatten())
        {
            let r = resources(c.resources.as_ref().and_then(|r| r.requests.as_ref()))
                .map_err(upstream)?;
            add(used.entry(node.clone()).or_default(), &r).map_err(upstream)?;
        }
        add(
            used.entry(node).or_default(),
            &resources(spec.overhead.as_ref()).map_err(upstream)?,
        )
        .map_err(upstream)?;
    }
    let mut inventory = Vec::new();
    for n in nodes.items {
        let mut available =
            resources(n.status.as_ref().and_then(|s| s.allocatable.as_ref())).map_err(upstream)?;
        if let Some(r) = used.get(n.metadata.name.as_deref().unwrap_or("")) {
            available.subtract(r);
        }
        let ready = n
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .is_some_and(|cs| cs.iter().any(|c| c.type_ == "Ready" && c.status == "True"));
        let schedulable = !n
            .spec
            .as_ref()
            .and_then(|s| s.unschedulable)
            .unwrap_or(false);
        let tainted = n
            .spec
            .as_ref()
            .and_then(|s| s.taints.as_ref())
            .is_some_and(|ts| {
                ts.iter()
                    .any(|t| matches!(t.effect.as_str(), "NoSchedule" | "NoExecute"))
            });
        inventory.push(PlacementNode {
            name: n.metadata.name.unwrap_or_default(),
            available,
            ready,
            schedulable,
            tainted,
            labels: n.metadata.labels.unwrap_or_default(),
        });
    }
    Ok(Json(
        json!({"candidates":planning::placement(&inventory,&req).map_err(bad)?,"execution_supported":false,"warnings":["Point-in-time Pod requests; conservative init-container accounting. GPU accounting covers nvidia.com/gpu only. No CPU-model, NUMA, CSI or application checks."]}),
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PowerAction {
    Start,
    Stop,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRequest {
    pub namespace: String,
    pub vm: String,
    pub action: PowerAction,
    pub idempotency_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Queued,
    Running,
    Succeeded,
    Failed,
    NeedsReview,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub owner: String,
    pub namespace: String,
    pub vm: String,
    pub action: PowerAction,
    pub backend: String,
    pub context: Option<String>,
    pub request_hash: String,
    pub phase: Phase,
    pub created_at: i64,
    pub updated_at: i64,
    pub message: String,
}
const LABEL: &str = "veyron.io/type=enterprise-operation";
const DATA: &str = "operation.json";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeQuery {
    pub namespace: String,
}
fn digest(v: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(v.as_ref()))
}
fn identity(subject: &str, req: &OperationRequest) -> (String, String) {
    // Length-prefixed JSON avoids ambiguous concatenation and map-order dependence.
    let id = digest(
        serde_json::to_vec(&(subject, &req.namespace, &req.idempotency_key)).expect("tuple JSON"),
    );
    let hash =
        digest(serde_json::to_vec(&(&req.namespace, &req.vm, &req.action)).expect("tuple JSON"));
    (format!("veyron-op-{}", &id[..48]), hash)
}
fn decode(cm: &ConfigMap) -> Result<Operation, ApiError> {
    let raw = cm
        .data
        .as_ref()
        .and_then(|d| d.get(DATA))
        .ok_or_else(|| upstream("operation payload missing"))?;
    serde_json::from_str(raw).map_err(upstream)
}
fn set(cm: &mut ConfigMap, op: &Operation) -> Result<(), ApiError> {
    cm.data
        .get_or_insert_with(BTreeMap::new)
        .insert(DATA.into(), serde_json::to_string(op).map_err(upstream)?);
    Ok(())
}
fn visible(op: &Operation, ctx: &AuthContext) -> bool {
    op.owner == ctx.subject
        || (ctx.role == ApiRole::Admin && tenant_scope::namespaces(ctx).is_ok_and(|n| n.is_none()))
}
fn check_id(id: &str) -> Result<(), ApiError> {
    if !planning::dns_label(id) || !id.starts_with("veyron-op-") {
        Err(bad("invalid operation ID"))
    } else {
        Ok(())
    }
}
async fn mark_stale(
    api: &Api<ConfigMap>,
    mut cm: ConfigMap,
) -> Result<(ConfigMap, Operation), ApiError> {
    let mut op = decode(&cm)?;
    if op.phase == Phase::Running
        && chrono::Utc::now().timestamp().saturating_sub(op.updated_at) > 90
    {
        op.phase = Phase::NeedsReview;
        op.message="worker heartbeat expired; inspect VM state before a new operation. No automatic replay.".into();
        op.updated_at = chrono::Utc::now().timestamp();
        set(&mut cm, &op)?;
        let name = op.id.clone();
        match api.replace(&name, &PostParams::default(), &cm).await {
            Ok(saved) => cm = saved,
            Err(kube::Error::Api(e)) if e.code == 409 => {
                cm = api.get(&name).await.map_err(upstream)?;
                op = decode(&cm)?;
            }
            Err(e) => return Err(upstream(e)),
        }
    }
    Ok((cm, op))
}
async fn submit(
    State(state): State<SharedState>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<ScopeQuery>,
    Json(req): Json<OperationRequest>,
) -> Result<(StatusCode, Json<Operation>), ApiError> {
    scope(&ctx, &req.namespace)?;
    if query.namespace != req.namespace
        || !planning::dns_label(&req.vm)
        || req.idempotency_key.is_empty()
        || req.idempotency_key.len() > 128
        || req.idempotency_key.chars().any(char::is_control)
    {
        return Err(bad(
            "namespace must match query; VM must be a DNS label; idempotency key must be 1..128 characters",
        ));
    }
    let (client, context) = {
        let s = state.read().await;
        (s.client().clone(), s.active_kube_context.clone())
    };
    let api = Api::<ConfigMap>::namespaced(client.client(), &req.namespace);
    let (id, hash) = identity(&ctx.subject, &req);
    let now = chrono::Utc::now().timestamp();
    let op = Operation {
        id: id.clone(),
        owner: ctx.subject.clone(),
        namespace: req.namespace.clone(),
        vm: req.vm,
        action: req.action,
        backend: selected().as_str().into(),
        context,
        request_hash: hash.clone(),
        phase: Phase::Queued,
        created_at: now,
        updated_at: now,
        message: "queued".into(),
    };
    let mut cm = ConfigMap {
        metadata: kube::api::ObjectMeta {
            name: Some(id.clone()),
            namespace: Some(req.namespace),
            labels: Some(BTreeMap::from([(
                "veyron.io/type".into(),
                "enterprise-operation".into(),
            )])),
            ..Default::default()
        },
        ..Default::default()
    };
    set(&mut cm, &op)?;
    let (cm, status) = match api.create(&PostParams::default(), &cm).await {
        Ok(cm) => (cm, StatusCode::ACCEPTED),
        Err(kube::Error::Api(e)) if e.code == 409 => {
            (api.get(&id).await.map_err(upstream)?, StatusCode::OK)
        }
        Err(e) => return Err(upstream(e)),
    };
    let (cm, stored) = mark_stale(&api, cm).await?;
    if stored.owner != ctx.subject
        || stored.request_hash != hash
        || stored.context != op.context
        || stored.backend != op.backend
    {
        return Err(error(
            StatusCode::CONFLICT,
            "idempotency key already used for another request, context or backend",
        ));
    }
    if stored.phase == Phase::Queued {
        tokio::spawn(async move {
            if let Err(e) = worker(client, cm).await {
                log::error!("enterprise operation worker: {}", e.1.0);
            }
        });
    }
    Ok((status, Json(stored)))
}
async fn list(
    State(state): State<SharedState>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<ScopeQuery>,
) -> Result<Json<Vec<Operation>>, ApiError> {
    scope(&ctx, &q.namespace)?;
    let api = Api::<ConfigMap>::namespaced(state.read().await.client().client(), &q.namespace);
    let cms = api
        .list(&ListParams::default().labels(LABEL))
        .await
        .map_err(upstream)?;
    let mut result = Vec::new();
    for cm in cms.items {
        let op = decode(&cm)?;
        if visible(&op, &ctx) {
            result.push(mark_stale(&api, cm).await?.1);
        }
    }
    result.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
    Ok(Json(result))
}
async fn owned(
    state: &SharedState,
    ctx: &AuthContext,
    q: &ScopeQuery,
    id: &str,
) -> Result<(Api<ConfigMap>, ConfigMap, Operation), ApiError> {
    scope(ctx, &q.namespace)?;
    check_id(id)?;
    let api = Api::<ConfigMap>::namespaced(state.read().await.client().client(), &q.namespace);
    let cm = api.get(id).await.map_err(|e| match e {
        kube::Error::Api(e) if e.code == 404 => error(StatusCode::NOT_FOUND, "operation not found"),
        e => upstream(e),
    })?;
    let op = decode(&cm)?;
    if !visible(&op, ctx) {
        return Err(error(StatusCode::NOT_FOUND, "operation not found"));
    }
    let (cm, op) = mark_stale(&api, cm).await?;
    Ok((api, cm, op))
}
async fn read(
    State(state): State<SharedState>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<ScopeQuery>,
    Path(id): Path<String>,
) -> Result<Json<Operation>, ApiError> {
    Ok(Json(owned(&state, &ctx, &q, &id).await?.2))
}
async fn cancel(
    State(state): State<SharedState>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<ScopeQuery>,
    Path(id): Path<String>,
) -> Result<Json<Operation>, ApiError> {
    let (api, mut cm, mut op) = owned(&state, &ctx, &q, &id).await?;
    if op.phase != Phase::Queued {
        return Err(error(
            StatusCode::CONFLICT,
            "only queued operations can be cancelled",
        ));
    }
    op.phase = Phase::Cancelled;
    op.updated_at = chrono::Utc::now().timestamp();
    op.message = "cancelled before claim".into();
    set(&mut cm, &op)?;
    api.replace(&id, &PostParams::default(), &cm)
        .await
        .map_err(|e| error(StatusCode::CONFLICT, e))?;
    Ok(Json(op))
}
async fn worker(client: KubeClient, mut cm: ConfigMap) -> Result<(), ApiError> {
    let mut op = decode(&cm)?;
    if op.phase != Phase::Queued {
        return Ok(());
    }
    let api = Api::<ConfigMap>::namespaced(client.client(), &op.namespace);
    op.phase = Phase::Running;
    op.updated_at = chrono::Utc::now().timestamp();
    op.message = "claimed".into();
    set(&mut cm, &op)?;
    cm = match api.replace(&op.id, &PostParams::default(), &cm).await {
        Ok(cm) => cm,
        Err(kube::Error::Api(e)) if e.code == 409 => return Ok(()),
        Err(e) => return Err(upstream(e)),
    };
    let lock_name = format!(
        "veyron-power-{}",
        &digest(serde_json::to_vec(&(&op.namespace, &op.vm)).expect("identity JSON"))[..48]
    );
    let lock = ConfigMap {
        metadata: kube::api::ObjectMeta {
            name: Some(lock_name.clone()),
            labels: Some(BTreeMap::from([(
                "veyron.io/type".into(),
                "enterprise-lock".into(),
            )])),
            ..Default::default()
        },
        data: Some(BTreeMap::from([("operation".into(), op.id.clone())])),
        ..Default::default()
    };
    let lock = match api.create(&PostParams::default(), &lock).await {
        Ok(lock) => lock,
        Err(kube::Error::Api(e)) if e.code == 409 => {
            op.phase = Phase::Failed;
            op.updated_at = chrono::Utc::now().timestamp();
            op.message =
                "another power workflow holds this VM lock; inspect that workflow before retrying"
                    .into();
            set(&mut cm, &op)?;
            api.replace(&op.id, &PostParams::default(), &cm)
                .await
                .map_err(upstream)?;
            return Ok(());
        }
        Err(e) => return Err(upstream(e)),
    };
    let backend = VmBackend::for_client(&client);
    let result = tokio::time::timeout(
        Duration::from_secs(900),
        execute(&backend, &client, &api, &mut cm, &mut op),
    )
    .await;
    match result {
        Ok(Ok(())) => {
            op.phase = Phase::Succeeded;
            op.message = "controller power state observed; application health not verified".into();
        }
        Ok(Err(e)) => {
            op.phase = Phase::NeedsReview;
            op.message = format!("inspect VM before another request: {e}");
        }
        Err(_) => {
            op.phase = Phase::NeedsReview;
            op.message = "15 minute deadline exceeded; no automatic replay".into();
        }
    }
    op.updated_at = chrono::Utc::now().timestamp();
    set(&mut cm, &op)?;
    api.replace(&op.id, &PostParams::default(), &cm)
        .await
        .map_err(upstream)?;
    if op.phase == Phase::Succeeded {
        let params = DeleteParams {
            preconditions: Some(kube::api::Preconditions {
                uid: lock.metadata.uid,
                resource_version: lock.metadata.resource_version,
            }),
            ..Default::default()
        };
        api.delete(&lock_name, &params).await.map_err(upstream)?;
    }
    // Ambiguous workflows retain the lock for explicit administrator review.
    Ok(())
}
async fn execute(
    backend: &VmBackend,
    client: &KubeClient,
    api: &Api<ConfigMap>,
    cm: &mut ConfigMap,
    op: &mut Operation,
) -> Result<(), String> {
    backend
        .get(&op.namespace, &op.vm)
        .await
        .map_err(|e| e.to_string())?;
    match op.action {
        PowerAction::Start => backend.start(&op.namespace, &op.vm).await,
        PowerAction::Stop => backend.stop(&op.namespace, &op.vm).await,
    }
    .map_err(|e| e.to_string())?;
    let mut last = std::time::Instant::now();
    loop {
        let observed = if op.backend == "kairon" {
            let object = backend
                .get(&op.namespace, &op.vm)
                .await
                .map_err(|e| e.to_string())?;
            let phase = object.pointer("/status/phase").and_then(Value::as_str);
            matches!(
                (&op.action, phase),
                (PowerAction::Start, Some("Running")) | (PowerAction::Stop, Some("Stopped"))
            )
        } else {
            match client.get_vmi(&op.namespace, &op.vm).await {
                Ok(vmi) => {
                    op.action == PowerAction::Start
                        && vmi.status.as_ref().and_then(|s| s.phase.as_deref()) == Some("Running")
                }
                Err(e)
                    if e.downcast_ref::<kube::Error>()
                        .is_some_and(|e| matches!(e,kube::Error::Api(e) if e.code==404)) =>
                {
                    op.action == PowerAction::Stop
                }
                Err(e) => return Err(e.to_string()),
            }
        };
        if observed {
            return Ok(());
        }
        if last.elapsed() >= Duration::from_secs(15) {
            op.updated_at = chrono::Utc::now().timestamp();
            op.message = "waiting for observed power state".into();
            set(cm, op).map_err(|e| e.1.0.to_string())?;
            *cm = api
                .replace(&op.id, &PostParams::default(), cm)
                .await
                .map_err(|e| e.to_string())?;
            last = std::time::Instant::now();
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn req() -> OperationRequest {
        OperationRequest {
            namespace: "apps".into(),
            vm: "web".into(),
            action: PowerAction::Start,
            idempotency_key: "retry-key".into(),
        }
    }
    #[test]
    fn idempotency_is_owner_scoped_and_payload_sensitive() {
        let a = req();
        let mut b = req();
        b.action = PowerAction::Stop;
        assert_eq!(identity("alice", &a).0, identity("alice", &b).0);
        assert_ne!(identity("alice", &a).1, identity("alice", &b).1);
        assert_ne!(identity("alice", &a).0, identity("bob", &a).0);
    }
    #[test]
    fn unknown_operation_fields_rejected() {
        assert!(serde_json::from_value::<OperationRequest>(json!({"namespace":"apps","vm":"web","action":"start","idempotency_key":"a","extra":true})).is_err());
    }
    #[test]
    fn resource_accounting_rejects_unknown_quantities() {
        let map = BTreeMap::from([(
            "cpu".into(),
            k8s_openapi::apimachinery::pkg::api::resource::Quantity("1e3".into()),
        )]);
        assert!(resources(Some(&map)).is_err());
    }
}
