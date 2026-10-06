// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//
// Cluster resource handlers: storage, templates, snapshots, events, nodes, pods, dashboard, health.

use super::*;

pub(super) async fn list_pvcs_handler(
    State(state): State<SharedState>,
    Query(query): Query<VmQuery>,
) -> impl IntoResponse {
    let (client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let ns = query.namespace.as_deref().unwrap_or(&namespace);

    let pvcs = if ns == "all" {
        client.list_pvcs_for_scope("all").await
    } else {
        client.list_pvcs(ns).await.unwrap_or_default()
    };

    let items: Vec<serde_json::Value> = pvcs
        .iter()
        .map(|pvc| {
            let meta = &pvc.metadata;
            let spec = pvc.spec.as_ref();
            let status = pvc.status.as_ref();
            serde_json::json!({
                "name": meta.name.clone().unwrap_or_default(),
                "namespace": meta.namespace.clone().unwrap_or_default(),
                "status": status.and_then(|s| s.phase.as_ref()).map(|s| s.to_string()).unwrap_or_default(),
                "capacity": status
                    .and_then(|s| s.capacity.as_ref())
                    .and_then(|c| c.get("storage"))
                    .map(|v| v.0.clone())
                    .unwrap_or_default(),
                "storage_class": spec.and_then(|s| s.storage_class_name.clone()).unwrap_or_default(),
                "access_modes": spec.map(|s| s.access_modes.clone().unwrap_or_default()).unwrap_or_default(),
            })
        })
        .collect();
    let ctx = req_ctx(HttpMethod::GET, "/api/v1/storage/pvcs");
    ok_json(&ApiResponse::success(&items, &ctx.request_id))
}

pub(super) async fn list_storage_classes_handler(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let client = { state.read().await.kube_client.clone() };
    match client.list_storage_classes().await {
        Ok(scs) => {
            let items: Vec<serde_json::Value> = scs
                .iter()
                .map(|sc| {
                    serde_json::json!({
                        "name": sc.metadata.name.clone().unwrap_or_default(),
                        "provisioner": sc.provisioner,
                        "reclaim_policy": sc.reclaim_policy.clone().unwrap_or_default(),
                        "volume_binding_mode": sc.volume_binding_mode.clone().unwrap_or_default(),
                        "is_default": sc.metadata.annotations.as_ref()
                            .and_then(|a| a.get("storageclass.kubernetes.io/is-default-class"))
                            .map(|v| v == "true")
                            .unwrap_or(false),
                    })
                })
                .collect();
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/storage/classes");
            ok_json(&ApiResponse::success(&items, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

// ── OpenAPI ──────────────────────────────────────────────────

pub(super) async fn openapi_handler() -> impl IntoResponse {
    let spec = crate::api::openapi::generate_default_spec();
    let json = serde_json::to_value(&spec).unwrap_or_default();
    (StatusCode::OK, Json(json))
}

pub(super) async fn list_templates_handler() -> impl IntoResponse {
    let templates: Vec<serde_json::Value> = TEMPLATES
        .list()
        .iter()
        .filter_map(|name| {
            TEMPLATES.get(name).map(|t| {
                let disk = t.default_disk_size_label();
                serde_json::json!({
                    "name": name,
                    "cpu": t.cpu.cores,
                    "memory": t.memory.size,
                    "disk": disk,
                    "default_cpus": t.cpu.cores,
                    "default_memory": t.memory.size,
                    "default_disk_size": disk,
                })
            })
        })
        .collect();
    let ctx = req_ctx(HttpMethod::GET, "/api/v1/templates");
    ok_json(&ApiResponse::success(&templates, &ctx.request_id))
}

// ── Snapshot Endpoints ────────────────────────────────────────

pub(super) async fn list_snapshots_handler(
    State(state): State<SharedState>,
    Query(query): Query<VmQuery>,
) -> impl IntoResponse {
    let namespace = {
        let s = state.read().await;
        query
            .namespace
            .clone()
            .unwrap_or_else(|| s.namespace.clone())
    };

    let client = {
        let s = state.read().await;
        s.kube_client.client()
    };

    match crate::snapshots::SnapshotManager::list_snapshots_in_scope(client, &namespace).await {
        Ok(snapshots) => {
            let items: Vec<SnapshotItem> = snapshots
                .into_iter()
                .map(|s| {
                    let age = s.age();
                    let status = s.status.to_string();
                    SnapshotItem {
                        name: s.name,
                        vm_name: s.vm_name,
                        namespace: s.namespace,
                        status,
                        ready: s.ready_to_use,
                        age,
                    }
                })
                .collect();
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/snapshots");
            ok_json(&ApiResponse::success(&items, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

pub(super) async fn list_vm_snapshots_handler(
    Path((ns, vm)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("vm", &vm)]) {
        return resp;
    }

    match crate::snapshots::SnapshotManager::new(&ns).await {
        Ok(manager) => match manager.list_snapshots_for_vm(&vm).await {
            Ok(snapshots) => {
                let items: Vec<SnapshotItem> = snapshots
                    .into_iter()
                    .map(|s| {
                        let age = s.age();
                        let status = s.status.to_string();
                        SnapshotItem {
                            name: s.name,
                            vm_name: s.vm_name,
                            namespace: ns.clone(),
                            status,
                            ready: s.ready_to_use,
                            age,
                        }
                    })
                    .collect();
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/snapshots/:ns/:vm");
                ok_json(&ApiResponse::success(&items, &ctx.request_id))
            }
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        },
        Err(e) => err_json(503, "SERVICE_UNAVAILABLE", &sanitize_error(&e)),
    }
}

pub(super) async fn delete_snapshot_handler(
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    match crate::snapshots::SnapshotManager::new(&ns).await {
        Ok(manager) => match manager.delete_snapshot(&name).await {
            Ok(_) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/snapshots/:ns/:name/delete");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({"message": format!("Snapshot '{}' deleted", name)}),
                    &ctx.request_id,
                ))
            }
            Err(e) => {
                let msg = sanitize_error(&e);
                err_json(status_for_sanitized_error(&msg), "DELETE_FAILED", &msg)
            }
        },
        Err(e) => err_json(503, "SERVICE_UNAVAILABLE", &sanitize_error(&e)),
    }
}

// ── Events ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct EventsQuery {
    pub limit: Option<u32>,
    pub namespace: Option<String>,
}

pub(super) async fn list_events_handler(
    State(state): State<SharedState>,
    Query(query): Query<EventsQuery>,
) -> impl IntoResponse {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };

    let limit = query.limit.unwrap_or(50).min(1000);

    use k8s_openapi::api::core::v1::Event;
    use kube::Api;
    let list_result = if query.namespace.as_deref() == Some("all") {
        let events_api: Api<Event> = Api::all(client.client());
        let lp = kube::api::ListParams::default().limit(limit);
        events_api.list(&lp).await
    } else {
        let ns = query
            .namespace
            .clone()
            .unwrap_or_else(|| default_ns.clone());
        let events_api: Api<Event> = Api::namespaced(client.client(), &ns);
        let lp = kube::api::ListParams::default().limit(limit);
        events_api.list(&lp).await
    };

    match list_result {
        Ok(event_list) => {
            let items: Vec<EventItem> = event_list
                .items
                .into_iter()
                .map(|e| EventItem {
                    type_: e.type_.unwrap_or_default(),
                    reason: e.reason.unwrap_or_default(),
                    message: e.message.unwrap_or_default(),
                    namespace: e.metadata.namespace.unwrap_or_default(),
                    involved_object: e.involved_object.name.unwrap_or_default(),
                    timestamp: e
                        .last_timestamp
                        .map(|t| t.0.to_rfc3339())
                        .or_else(|| e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339()))
                        .unwrap_or_default(),
                })
                .collect();
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/events");
            ok_json(&ApiResponse::success(&items, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

pub(super) async fn recent_events_handler(
    State(state): State<SharedState>,
    Query(query): Query<EventsQuery>,
) -> impl IntoResponse {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };

    use k8s_openapi::api::core::v1::Event;
    use kube::Api;
    let list_result = if query.namespace.as_deref() == Some("all") {
        let events_api: Api<Event> = Api::all(client.client());
        let lp = kube::api::ListParams::default().limit(500);
        events_api.list(&lp).await
    } else {
        let ns = query
            .namespace
            .clone()
            .unwrap_or_else(|| default_ns.clone());
        let events_api: Api<Event> = Api::namespaced(client.client(), &ns);
        let lp = kube::api::ListParams::default().limit(500);
        events_api.list(&lp).await
    };

    match list_result {
        Ok(event_list) => {
            let one_hour_ago = chrono::Utc::now() - chrono::TimeDelta::hours(1);
            let items: Vec<EventItem> = event_list
                .items
                .into_iter()
                .filter(|e| {
                    // Keep events from the last hour based on timestamp
                    let ts = e
                        .last_timestamp
                        .as_ref()
                        .map(|t| t.0)
                        .or_else(|| e.metadata.creation_timestamp.as_ref().map(|t| t.0));
                    match ts {
                        Some(t) => t >= one_hour_ago,
                        None => false, // exclude events with no timestamp
                    }
                })
                .map(|e| EventItem {
                    type_: e.type_.unwrap_or_default(),
                    reason: e.reason.unwrap_or_default(),
                    message: e.message.unwrap_or_default(),
                    namespace: e.metadata.namespace.unwrap_or_default(),
                    involved_object: e.involved_object.name.unwrap_or_default(),
                    timestamp: e
                        .last_timestamp
                        .map(|t| t.0.to_rfc3339())
                        .or_else(|| e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339()))
                        .unwrap_or_default(),
                })
                .collect();
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/events/recent");
            ok_json(&ApiResponse::success(&items, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

// ── Snapshot Create & Restore ─────────────────────────────────

#[derive(Deserialize)]
pub(super) struct CreateSnapshotRequest {
    snapshot_name: Option<String>,
}

pub(super) async fn create_snapshot_handler(
    Path((ns, vm)): Path<(String, String)>,
    Json(req): Json<CreateSnapshotRequest>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &vm)]) {
        return resp;
    }

    let snap_name = req.snapshot_name.unwrap_or_else(|| {
        let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        format!("{}-snap-{}", vm, ts)
    });

    match crate::snapshots::SnapshotManager::new(&ns).await {
        Ok(manager) => {
            let config = crate::snapshots::SnapshotConfig::new(&vm, &snap_name);
            match manager.create_snapshot(&config).await {
                Ok(info) => {
                    let ctx = req_ctx(HttpMethod::POST, "/api/v1/snapshots/:ns/:vm/create");
                    ok_json(&ApiResponse::success(
                        &serde_json::json!({
                            "message": format!("Snapshot '{}' created", snap_name),
                            "name": info.name,
                            "vm_name": info.vm_name,
                        }),
                        &ctx.request_id,
                    ))
                }
                Err(e) => {
                    // sanitize_error collapses unmatched causes to a generic
                    // "Internal server error" for the client — log the real
                    // cause server-side or it's unrecoverable from the API
                    // response alone (e.g. an admission-webhook denial like
                    // "snapshot feature gate not enabled").
                    log::error!("Create snapshot '{}/{}' failed: {}", ns, vm, e);
                    let msg = sanitize_error(&e);
                    err_json(status_for_sanitized_error(&msg), "CREATE_FAILED", &msg)
                }
            }
        }
        Err(e) => err_json(503, "SERVICE_UNAVAILABLE", &sanitize_error(&e)),
    }
}

pub(super) async fn restore_snapshot_handler(
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    match crate::snapshots::restore::RestoreManager::new(&ns).await {
        Ok(manager) => {
            // Derive VM name from snapshot name (convention: vmname-snap-timestamp)
            let vm_name = name
                .rsplit_once("-snap-")
                .map(|(v, _)| v.to_string())
                .unwrap_or_else(|| name.clone());
            match manager.restore_in_place(&vm_name, &name).await {
                Ok(info) => {
                    let ctx = req_ctx(HttpMethod::POST, "/api/v1/snapshots/:ns/:name/restore");
                    ok_json(&ApiResponse::success(
                        &serde_json::json!({
                            "message": format!("Restore '{}' initiated", info.name),
                            "name": info.name,
                        }),
                        &ctx.request_id,
                    ))
                }
                Err(e) => {
                    let msg = sanitize_error(&e);
                    err_json(status_for_sanitized_error(&msg), "RESTORE_FAILED", &msg)
                }
            }
        }
        Err(e) => err_json(503, "SERVICE_UNAVAILABLE", &sanitize_error(&e)),
    }
}

#[derive(Deserialize)]
pub(super) struct SnapshotScheduleQuery {
    namespace: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct SnapshotScheduleCreateReq {
    namespace: Option<String>,
    vm_name: String,
    cron: String,
    snapshot_prefix: Option<String>,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    max_snapshots: u32,
}

pub(super) fn default_true() -> bool {
    true
}

pub(super) async fn list_snapshot_schedules_handler(
    State(state): State<SharedState>,
    Query(q): Query<SnapshotScheduleQuery>,
) -> impl IntoResponse {
    let filter_ns = q.namespace.clone();
    let (default_ns, client) = {
        let s = state.read().await;
        (s.namespace.clone(), s.kube_client.client())
    };

    match crate::snapshots::vm_snapshot_schedule::list_schedule_configmaps(client).await {
        Ok(mut cms) => {
            if filter_ns.as_deref() != Some("all") {
                let ns = filter_ns.unwrap_or(default_ns);
                cms.retain(|c| c.metadata.namespace.as_deref() == Some(ns.as_str()));
            }
            let rows: Vec<serde_json::Value> = cms
                .into_iter()
                .filter_map(|cm| {
                    let data = cm.data?;
                    let raw =
                        data.get(crate::snapshots::vm_snapshot_schedule::SCHEDULE_CM_DATA_KEY)?;
                    let rec: crate::snapshots::vm_snapshot_schedule::SnapshotScheduleRecord =
                        serde_json::from_str(raw).ok()?;
                    Some(serde_json::json!({
                        "name": cm.metadata.name,
                        "namespace": cm.metadata.namespace,
                        "vm_name": rec.vm_name,
                        "cron": rec.cron,
                        "enabled": rec.enabled,
                        "snapshot_prefix": rec.snapshot_prefix,
                        "max_snapshots": rec.max_snapshots,
                        "last_run": rec.last_run.map(|t| t.to_rfc3339()),
                    }))
                })
                .collect();
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/snapshot-schedules");
            ok_json(&ApiResponse::success(&rows, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

pub(super) async fn create_snapshot_schedule_handler(
    State(state): State<SharedState>,
    Json(req): Json<SnapshotScheduleCreateReq>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("vm_name", &req.vm_name)]) {
        return resp;
    }
    let ns = if let Some(ref n) = req.namespace {
        n.clone()
    } else {
        let s = state.read().await;
        s.namespace.clone()
    };
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.client()
    };

    match crate::snapshots::vm_snapshot_schedule::upsert_schedule_cm(
        client,
        &ns,
        &req.vm_name,
        &req.cron,
        req.snapshot_prefix.as_deref(),
        req.enabled,
        req.max_snapshots,
    )
    .await
    {
        Ok(cm_name) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/snapshot-schedules");
            ok_json(&ApiResponse::success(
                &serde_json::json!({
                    "message": "Snapshot schedule created",
                    "name": cm_name,
                    "namespace": ns,
                }),
                &ctx.request_id,
            ))
        }
        Err(e) => err_json(400, "INVALID_SCHEDULE", &sanitize_error(&e)),
    }
}

pub(super) async fn delete_snapshot_schedule_handler(
    State(state): State<SharedState>,
    Path((ns, cm_name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &cm_name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.client()
    };

    match crate::snapshots::vm_snapshot_schedule::delete_schedule_cm(client, &ns, &cm_name).await {
        Ok(()) => {
            let ctx = req_ctx(
                HttpMethod::DELETE,
                "/api/v1/snapshot-schedules/:ns/:cm_name",
            );
            ok_json(&ApiResponse::success(
                &serde_json::json!({ "message": "Schedule deleted" }),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "DELETE_FAILED", &msg)
        }
    }
}

// ── Cluster Resources ────────────────────────────────────────

pub(super) async fn list_nodes_handler(State(state): State<SharedState>) -> impl IntoResponse {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    let (nodes, usage, vm_counts) = tokio::join!(
        client.list_nodes(),
        node_usage(&client),
        vmis_per_node(&client)
    );
    match nodes {
        Ok(nodes) => {
            let items: Vec<NodeItem> = nodes
                .iter()
                .map(|node| {
                    let meta = &node.metadata;
                    let name = meta.name.clone().unwrap_or_default();
                    let labels = meta.labels.as_ref();

                    let roles: Vec<String> = labels
                        .map(|l| {
                            l.keys()
                                .filter_map(|k| k.strip_prefix("node-role.kubernetes.io/"))
                                .map(|r| r.to_string())
                                .collect()
                        })
                        .unwrap_or_default();

                    let status = node
                        .status
                        .as_ref()
                        .and_then(|s| s.conditions.as_ref())
                        .and_then(|conds| {
                            conds.iter().find(|c| c.type_ == "Ready").map(|c| {
                                if c.status == "True" {
                                    "Ready"
                                } else {
                                    "NotReady"
                                }
                            })
                        })
                        .unwrap_or("Unknown")
                        .to_string();

                    let capacity = node.status.as_ref().and_then(|s| s.capacity.as_ref());
                    let allocatable = node.status.as_ref().and_then(|s| s.allocatable.as_ref());
                    let node_info = node.status.as_ref().and_then(|s| s.node_info.as_ref());

                    let unschedulable = node
                        .spec
                        .as_ref()
                        .and_then(|s| s.unschedulable)
                        .unwrap_or(false);

                    let alloc = |k: &str| allocatable.and_then(|a| a.get(k)).map(|v| v.0.as_str());
                    let (cpu_percent, memory_percent) = match usage.get(&name) {
                        Some(&(cpu_n, mem_b)) => (
                            alloc("cpu")
                                .map(crate::utils::parse_cpu_nanocores)
                                .and_then(|total| usage_percent(cpu_n, total)),
                            alloc("memory")
                                .map(crate::utils::parse_memory_bytes)
                                .and_then(|total| usage_percent(mem_b, total)),
                        ),
                        None => (None, None),
                    };
                    let vm_count = vm_counts.get(&name).copied().unwrap_or(0);

                    NodeItem {
                        cpu_percent,
                        memory_percent,
                        vm_count,
                        name,
                        status,
                        roles,
                        cpu_capacity: capacity
                            .and_then(|c| c.get("cpu"))
                            .map(|v| v.0.clone())
                            .unwrap_or_default(),
                        memory_capacity: capacity
                            .and_then(|c| c.get("memory"))
                            .map(|v| v.0.clone())
                            .unwrap_or_default(),
                        cpu_allocatable: allocatable
                            .and_then(|a| a.get("cpu"))
                            .map(|v| v.0.clone())
                            .unwrap_or_default(),
                        memory_allocatable: allocatable
                            .and_then(|a| a.get("memory"))
                            .map(|v| v.0.clone())
                            .unwrap_or_default(),
                        kubelet_version: node_info
                            .map(|i| i.kubelet_version.clone())
                            .unwrap_or_default(),
                        os_image: node_info.map(|i| i.os_image.clone()).unwrap_or_default(),
                        kernel_version: node_info
                            .map(|i| i.kernel_version.clone())
                            .unwrap_or_default(),
                        age: meta
                            .creation_timestamp
                            .as_ref()
                            .map(|t| format_age(&t.0))
                            .unwrap_or_default(),
                        unschedulable,
                    }
                })
                .collect();
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/nodes");
            ok_json(&ApiResponse::success(&items, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct PodListQuery {
    namespace: Option<String>,
    /// `true` drops Succeeded and Failed pods (completed jobs, evicted or dead pods).
    #[serde(default)]
    active: bool,
}

#[derive(Serialize, Default)]
pub(super) struct PodSummary {
    total: usize,
    running: usize,
    pending: usize,
    succeeded: usize,
    failed: usize,
    unknown: usize,
    /// Failed pods per namespace, largest first (top 10).
    failed_by_namespace: Vec<(String, usize)>,
}

fn summarize_pods(pods: &[k8s_openapi::api::core::v1::Pod]) -> PodSummary {
    let mut out = PodSummary {
        total: pods.len(),
        ..Default::default()
    };
    let mut failed_ns: HashMap<String, usize> = HashMap::new();
    for pod in pods {
        match pod.status.as_ref().and_then(|s| s.phase.as_deref()) {
            Some("Running") => out.running += 1,
            Some("Pending") => out.pending += 1,
            Some("Succeeded") => out.succeeded += 1,
            Some("Failed") => {
                out.failed += 1;
                *failed_ns
                    .entry(pod.metadata.namespace.clone().unwrap_or_default())
                    .or_default() += 1;
            }
            _ => out.unknown += 1,
        }
    }
    let mut by_ns: Vec<_> = failed_ns.into_iter().collect();
    by_ns.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    by_ns.truncate(10);
    out.failed_by_namespace = by_ns;
    out
}

/// Pod counts by phase — what Mission Control needs, without shipping every pod.
pub(super) async fn pods_summary_handler(
    State(state): State<SharedState>,
    Query(query): Query<VmQuery>,
) -> impl IntoResponse {
    let (client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let ns = query.namespace.as_deref().unwrap_or("all");
    let ns = if ns.is_empty() {
        namespace.as_str()
    } else {
        ns
    };
    let pods = if ns == "all" {
        client.list_all_pods().await
    } else {
        client.list_pods(ns).await
    };
    match pods {
        Ok(pods) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/pods/summary");
            ok_json(&ApiResponse::success(
                &summarize_pods(&pods),
                &ctx.request_id,
            ))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

pub(super) async fn list_pods_handler(
    State(state): State<SharedState>,
    Query(query): Query<PodListQuery>,
) -> impl IntoResponse {
    let (client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let ns = query.namespace.as_deref().unwrap_or(&namespace);

    let pods_result = if ns == "all" {
        client.list_all_pods().await
    } else {
        client.list_pods(ns).await
    };

    match pods_result {
        Ok(pods) => {
            let items: Vec<PodItem> = pods
                .iter()
                .filter(|pod| {
                    !query.active
                        || !matches!(
                            pod.status.as_ref().and_then(|s| s.phase.as_deref()),
                            Some("Succeeded" | "Failed")
                        )
                })
                .map(|pod| {
                    let meta = &pod.metadata;
                    let spec = pod.spec.as_ref();
                    let status = pod.status.as_ref();

                    let containers: Vec<String> = spec
                        .map(|s| s.containers.iter().map(|c| c.name.clone()).collect())
                        .unwrap_or_default();

                    let restarts: u32 = status
                        .and_then(|s| s.container_statuses.as_ref())
                        .map(|cs| cs.iter().map(|c| c.restart_count as u32).sum())
                        .unwrap_or(0);
                    let ready_count = status
                        .and_then(|s| s.container_statuses.as_ref())
                        .map(|cs| cs.iter().filter(|c| c.ready).count())
                        .unwrap_or(0);

                    PodItem {
                        ready: format!("{ready_count}/{}", containers.len()),
                        name: meta.name.clone().unwrap_or_default(),
                        namespace: meta.namespace.clone().unwrap_or_default(),
                        phase: status
                            .and_then(|s| s.phase.clone())
                            .unwrap_or_else(|| "Unknown".to_string()),
                        node_name: spec
                            .and_then(|s| s.node_name.clone())
                            .unwrap_or_else(|| "N/A".to_string()),
                        ip: status
                            .and_then(|s| s.pod_ip.clone())
                            .unwrap_or_else(|| "N/A".to_string()),
                        containers,
                        restarts,
                        age: meta
                            .creation_timestamp
                            .as_ref()
                            .map(|t| format_age(&t.0))
                            .unwrap_or_default(),
                    }
                })
                .collect();
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/pods");
            ok_json(&ApiResponse::success(&items, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

pub(super) async fn get_pod_logs_handler(
    State(state): State<SharedState>,
    Path(name): Path<String>,
    Query(query): Query<VmQuery>,
) -> impl IntoResponse {
    use k8s_openapi::api::core::v1::Pod;
    use kube::api::LogParams;

    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = query.namespace.as_deref().unwrap_or(&default_ns);

    let pod_ns = if scope == "all" {
        let pods = client.list_all_pods().await.unwrap_or_default();
        pods.into_iter()
            .find(|p| p.metadata.name.as_deref() == Some(name.as_str()))
            .and_then(|p| p.metadata.namespace)
            .unwrap_or(default_ns)
    } else {
        scope.to_string()
    };

    let pods_api: kube::api::Api<Pod> = kube::api::Api::namespaced(client.client(), &pod_ns);

    let params = LogParams {
        tail_lines: Some(200),
        timestamps: true,
        ..Default::default()
    };

    let log_text = match pods_api.logs(&name, &params).await {
        Ok(text) => text,
        Err(e) => {
            return err_json(
                404,
                "POD_LOGS_UNAVAILABLE",
                &format!("Could not read logs for {}/{}: {}", pod_ns, name, e),
            );
        }
    };

    let entries: Vec<crate::api::handlers::pods::PodLogEntry> = log_text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (timestamp, message) = match line.find(' ') {
                Some(pos) => (line[..pos].to_string(), line[pos + 1..].to_string()),
                None => (String::new(), line.to_string()),
            };
            crate::api::handlers::pods::PodLogEntry {
                timestamp,
                container: "default".to_string(),
                message,
            }
        })
        .collect();

    let ctx = req_ctx(HttpMethod::GET, "/api/v1/pods/:name/logs");
    ok_json(&ApiResponse::success(&entries, &ctx.request_id))
}

pub(super) async fn list_profiles_handler() -> impl IntoResponse {
    let profiles = match PROFILES.read() {
        Ok(p) => p,
        Err(_) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/profiles");
            let empty: Vec<ProfileItem> = vec![];
            return ok_json(&ApiResponse::success(&empty, &ctx.request_id));
        }
    };
    let items: Vec<ProfileItem> = profiles
        .list()
        .into_iter()
        .map(|p| ProfileItem {
            name: p.name.clone(),
            description: p.description.clone(),
            cpu_cores: p.cpu_cores,
            memory: p.memory.clone(),
            disk_size: p.disk_size.clone(),
            use_cases: p.use_cases.clone(),
        })
        .collect();
    let ctx = req_ctx(HttpMethod::GET, "/api/v1/profiles");
    ok_json(&ApiResponse::success(&items, &ctx.request_id))
}

pub(super) async fn list_namespaces_handler(State(state): State<SharedState>) -> impl IntoResponse {
    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    match client.list_namespaces().await {
        Ok(ns_list) => {
            let names: Vec<String> = ns_list
                .iter()
                .filter_map(|ns| ns.metadata.name.clone())
                .collect();
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/namespaces");
            ok_json(&ApiResponse::success(&names, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

pub(super) async fn activity_feed_handler(State(state): State<SharedState>) -> impl IntoResponse {
    let (client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };

    use k8s_openapi::api::core::v1::Event;
    use kube::Api;
    let events_api: Api<Event> = Api::namespaced(client.client(), &namespace);
    let lp = kube::api::ListParams::default().limit(200);
    match events_api.list(&lp).await {
        Ok(event_list) => {
            let mut items: Vec<EventItem> = event_list
                .items
                .into_iter()
                .filter(|e| {
                    let kind = e.involved_object.kind.as_deref().unwrap_or("");
                    kind == "VirtualMachine"
                        || kind == "VirtualMachineInstance"
                        || kind == "VirtualMachineSnapshot"
                })
                .map(|e| EventItem {
                    type_: e.type_.unwrap_or_default(),
                    reason: e.reason.unwrap_or_default(),
                    message: e.message.unwrap_or_default(),
                    namespace: e.metadata.namespace.unwrap_or_default(),
                    involved_object: e.involved_object.name.unwrap_or_default(),
                    timestamp: e
                        .last_timestamp
                        .map(|t| t.0.to_rfc3339())
                        .or_else(|| e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339()))
                        .unwrap_or_default(),
                })
                .collect();
            items.truncate(20);
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/activity");
            ok_json(&ApiResponse::success(&items, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

// ── Dashboard Overview ────────────────────────────────────────

pub(super) async fn dashboard_overview_handler(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let (client, namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };

    let vms = match client.list_vms(&namespace).await {
        Ok(vms) => vms,
        Err(e) => {
            return err_json(500, "INTERNAL_ERROR", &sanitize_error(&e));
        }
    };
    let total = vms.len();
    let mut running = 0usize;
    let mut stopped = 0usize;
    for vm in &vms {
        match vm
            .status
            .as_ref()
            .and_then(|s| s.printable_status.as_deref())
        {
            Some("Running") => running += 1,
            Some("Stopped") => stopped += 1,
            _ => {}
        }
    }
    let error = total.saturating_sub(running + stopped);

    // Count total allocated CPU/memory
    let mut total_cpus = 0u32;
    let mut total_memory_bytes = 0u64;
    for vm in &vms {
        total_cpus += vm
            .spec
            .template
            .spec
            .domain
            .cpu
            .as_ref()
            .and_then(|c| c.cores)
            .unwrap_or(1);
        if let Some(mem_str) = vm
            .spec
            .template
            .spec
            .domain
            .resources
            .requests
            .as_ref()
            .and_then(|r| r.get("memory"))
        {
            total_memory_bytes += parse_memory(mem_str);
        }
    }

    let snapshot_count = match crate::snapshots::SnapshotManager::new(&namespace).await {
        Ok(m) => m.list_all_snapshots().await.map(|s| s.len()).unwrap_or(0),
        Err(_) => 0,
    };

    // Aggregate node capacity
    let node_summary = match client.list_nodes().await {
        Ok(nodes) => {
            let total_nodes = nodes.len();
            let ready_nodes = nodes
                .iter()
                .filter(|n| {
                    n.status
                        .as_ref()
                        .and_then(|s| s.conditions.as_ref())
                        .and_then(|conds| conds.iter().find(|c| c.type_ == "Ready"))
                        .is_some_and(|c| c.status == "True")
                })
                .count();
            let mut cap_cpu = 0u64;
            let mut cap_mem = 0u64;
            let mut alloc_cpu = 0u64;
            let mut alloc_mem = 0u64;
            for node in &nodes {
                if let Some(cap) = node.status.as_ref().and_then(|s| s.capacity.as_ref()) {
                    cap_cpu += cap
                        .get("cpu")
                        .map(|v| v.0.parse::<u64>().unwrap_or(0))
                        .unwrap_or(0);
                    cap_mem += cap.get("memory").map(|v| parse_memory(&v.0)).unwrap_or(0);
                }
                if let Some(alloc) = node.status.as_ref().and_then(|s| s.allocatable.as_ref()) {
                    alloc_cpu += alloc
                        .get("cpu")
                        .map(|v| v.0.parse::<u64>().unwrap_or(0))
                        .unwrap_or(0);
                    alloc_mem += alloc.get("memory").map(|v| parse_memory(&v.0)).unwrap_or(0);
                }
            }
            NodeSummary {
                total_nodes,
                ready_nodes,
                total_cpu_capacity: cap_cpu,
                total_memory_capacity_gb: (cap_mem as f64) / (1024.0 * 1024.0 * 1024.0),
                total_cpu_allocatable: alloc_cpu,
                total_memory_allocatable_gb: (alloc_mem as f64) / (1024.0 * 1024.0 * 1024.0),
            }
        }
        Err(_) => NodeSummary {
            total_nodes: 0,
            ready_nodes: 0,
            total_cpu_capacity: 0,
            total_memory_capacity_gb: 0.0,
            total_cpu_allocatable: 0,
            total_memory_allocatable_gb: 0.0,
        },
    };

    let platform = client.platform_readiness().await;

    let overview = DashboardOverview {
        cluster: ClusterStats {
            total_vms: total,
            running_vms: running,
            stopped_vms: stopped,
            error_vms: error,
            total_vcpus_allocated: total_cpus,
            total_memory_allocated_gb: (total_memory_bytes as f64) / (1024.0 * 1024.0 * 1024.0),
            total_snapshots: snapshot_count,
        },
        nodes: node_summary,
        platform,
    };

    let ctx = req_ctx(HttpMethod::GET, "/api/v1/dashboard/overview");
    ok_json(&ApiResponse::success(&overview, &ctx.request_id))
}

// ── WebSocket ticket ───────────────────────────────────────────

#[derive(Serialize)]
pub(super) struct WsTicketResponse {
    ticket: String,
    expires_in: u64,
}

pub(super) async fn ws_ticket_handler() -> impl IntoResponse {
    const TTL_SECS: u64 = 60;
    let ticket =
        crate::api::ws_ticket::issue_ws_ticket(Some(std::time::Duration::from_secs(TTL_SECS)));
    let ctx = req_ctx(HttpMethod::POST, "/api/v1/ws/ticket");
    ok_json(&ApiResponse::success(
        &WsTicketResponse {
            ticket,
            expires_in: TTL_SECS,
        },
        &ctx.request_id,
    ))
}

// ── Health ─────────────────────────────────────────────────────

pub(super) async fn health_handler() -> impl IntoResponse {
    let ctx = req_ctx(HttpMethod::GET, "/api/v1/health");
    let registry = std::env::var("ZEUS_OS_KUBEVIRT_DISK_IMAGE_REGISTRY").ok();
    let context_name = crate::api::handlers::clusters::effective_current_context(
        crate::kube::active_kube_context().as_deref(),
    );
    let cluster_context = if context_name.is_empty() {
        let ns = std::env::var("VEYRON_NAMESPACE").unwrap_or_else(|_| "default".to_string());
        format!("in-cluster ({ns})")
    } else {
        context_name
    };
    let health = serde_json::json!({
        "status": "healthy",
        "version": "v1",
        "service": "veyron-api",
        "kubevirt_disk_image_registry": registry,
        "cluster_context": cluster_context
    });
    ok_json(&ApiResponse::success(&health, &ctx.request_id))
}

// ── Types ──────────────────────────────────────────────────────

#[derive(Serialize)]
pub(super) struct VmDetail {
    #[serde(flatten)]
    pub(super) info: VmInfo,
    pub(super) vmi_status: Option<crate::kube::types::VirtualMachineInstanceStatus>,
}

#[derive(Serialize)]
pub(super) struct SnapshotItem {
    name: String,
    vm_name: String,
    namespace: String,
    status: String,
    ready: bool,
    age: String,
}

#[derive(Serialize)]
pub(super) struct EventItem {
    #[serde(rename = "type")]
    pub(super) type_: String,
    pub(super) reason: String,
    pub(super) message: String,
    pub(super) namespace: String,
    pub(super) involved_object: String,
    pub(super) timestamp: String,
}

#[derive(Serialize)]
pub(super) struct DashboardOverview {
    cluster: ClusterStats,
    nodes: NodeSummary,
    platform: crate::kube::PlatformReadiness,
}

#[derive(Serialize)]
pub(super) struct ClusterStats {
    total_vms: usize,
    running_vms: usize,
    stopped_vms: usize,
    error_vms: usize,
    total_vcpus_allocated: u32,
    total_memory_allocated_gb: f64,
    total_snapshots: usize,
}

#[derive(Serialize)]
pub(super) struct NodeSummary {
    total_nodes: usize,
    ready_nodes: usize,
    total_cpu_capacity: u64,
    total_memory_capacity_gb: f64,
    total_cpu_allocatable: u64,
    total_memory_allocatable_gb: f64,
}

#[derive(Serialize)]
pub(super) struct NodeItem {
    name: String,
    status: String,
    roles: Vec<String>,
    cpu_capacity: String,
    memory_capacity: String,
    cpu_allocatable: String,
    memory_allocatable: String,
    kubelet_version: String,
    os_image: String,
    kernel_version: String,
    age: String,
    /// True when the node is cordoned (`spec.unschedulable`).
    unschedulable: bool,
    /// Live usage over allocatable from metrics-server; absent when metrics-server is not installed.
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_percent: Option<f64>,
    /// VirtualMachineInstances scheduled on this node.
    vm_count: u32,
}

/// Per-node (CPU nanocores, memory bytes) from `metrics.k8s.io/v1beta1` NodeMetrics.
async fn node_usage(client: &KubeClient) -> HashMap<String, (u64, u64)> {
    let gvk = kube::api::GroupVersionKind::gvk("metrics.k8s.io", "v1beta1", "NodeMetrics");
    let ar = kube::api::ApiResource::from_gvk(&gvk);
    let api: kube::Api<kube::api::DynamicObject> = kube::Api::all_with(client.client(), &ar);
    let Ok(list) = api.list(&kube::api::ListParams::default()).await else {
        return HashMap::new();
    };
    list.items
        .into_iter()
        .filter_map(|m| {
            let name = m.metadata.name.clone()?;
            let usage = m.data.get("usage")?;
            let cpu = usage.get("cpu")?.as_str()?;
            let mem = usage.get("memory")?.as_str()?;
            Some((
                name,
                (
                    crate::utils::parse_cpu_nanocores(cpu),
                    crate::utils::parse_memory_bytes(mem),
                ),
            ))
        })
        .collect()
}

async fn vmis_per_node(client: &KubeClient) -> HashMap<String, u32> {
    use crate::kube::types::VirtualMachineInstance;
    let api: kube::Api<VirtualMachineInstance> = kube::Api::all(client.client());
    let mut counts = HashMap::new();
    if let Ok(list) = api.list(&kube::api::ListParams::default()).await {
        for node in list
            .items
            .iter()
            .filter_map(|v| v.status.as_ref()?.node_name.clone())
        {
            *counts.entry(node).or_insert(0) += 1;
        }
    }
    counts
}

fn usage_percent(used: u64, total: u64) -> Option<f64> {
    (total > 0).then(|| ((used as f64 / total as f64) * 1000.0).round() / 10.0)
}

#[derive(Serialize)]
pub(super) struct PodItem {
    name: String,
    namespace: String,
    phase: String,
    node_name: String,
    ip: String,
    containers: Vec<String>,
    /// Ready containers over total, e.g. `1/2`.
    ready: String,
    restarts: u32,
    age: String,
}

#[derive(Serialize)]
pub(super) struct ProfileItem {
    name: String,
    description: String,
    cpu_cores: u32,
    memory: String,
    disk_size: String,
    use_cases: Vec<String>,
}

// ── Helpers ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::core::v1::{Pod, PodStatus};

    fn pod(ns: &str, phase: &str) -> Pod {
        Pod {
            metadata: kube::api::ObjectMeta {
                namespace: Some(ns.into()),
                ..Default::default()
            },
            status: Some(PodStatus {
                phase: Some(phase.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn usage_percent_rounds_and_guards_zero() {
        assert_eq!(usage_percent(11_880, 12_000), Some(99.0));
        assert_eq!(usage_percent(1, 3), Some(33.3));
        assert_eq!(usage_percent(5, 0), None);
    }

    #[test]
    fn summarize_pods_counts_phases_and_ranks_failed_namespaces() {
        let pods = vec![
            pod("a", "Running"),
            pod("a", "Failed"),
            pod("b", "Failed"),
            pod("b", "Failed"),
            pod("c", "Succeeded"),
            pod("c", "Pending"),
            pod("c", "Weird"),
        ];
        let s = summarize_pods(&pods);
        assert_eq!(
            (
                s.total,
                s.running,
                s.pending,
                s.succeeded,
                s.failed,
                s.unknown
            ),
            (7, 1, 1, 1, 3, 1)
        );
        assert_eq!(
            s.failed_by_namespace,
            vec![("b".to_string(), 2), ("a".to_string(), 1)]
        );
    }
}
