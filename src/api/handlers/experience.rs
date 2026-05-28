// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Zyvor CloudOS experience layer — home, search, fix-it errors, migration assistant.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

// ── Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ExperienceContext {
    pub product: String,
    pub tagline: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlCenterItem {
    pub label: String,
    pub value: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HomeActionCard {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperienceHomeResponse {
    pub vmrogue_context: super::feature_context::VmrogueFeatureContext,
    pub experience: ExperienceContext,
    pub greeting: String,
    pub workspace: String,
    pub health_summary: String,
    pub health_score: u8,
    pub stats: serde_json::Value,
    pub control_center: Vec<ControlCenterItem>,
    pub action_cards: Vec<HomeActionCard>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResultItem {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperienceSearchResponse {
    pub query: String,
    pub results: Vec<SearchResultItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranslateErrorRequest {
    pub message: String,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FixAction {
    pub label: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranslateErrorResponse {
    pub title: String,
    pub explanation: String,
    pub fixes: Vec<FixAction>,
    pub raw_message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MigrationInventoryItem {
    pub name: String,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub firmware: Option<String>,
    #[serde(default)]
    pub disk_format: Option<String>,
    #[serde(default)]
    pub disk_gb: Option<u32>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub has_vmware_tools: Option<bool>,
    #[serde(default)]
    pub virtio_ready: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationScanRequest {
    pub source: String,
    #[serde(default)]
    pub items: Vec<MigrationInventoryItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationReadinessCheck {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationScanItem {
    pub name: String,
    pub readiness: String,
    pub checks: Vec<MigrationReadinessCheck>,
    pub recommended_plan: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationScanResponse {
    pub source: String,
    pub source_label: String,
    pub total: u32,
    pub ready: u32,
    pub needs_preparation: u32,
    pub blocked: u32,
    pub items: Vec<MigrationScanItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationPlanRequest {
    pub source: String,
    pub item: MigrationInventoryItem,
    #[serde(default)]
    pub target_namespace: Option<String>,
    #[serde(default)]
    pub target_network: Option<String>,
    #[serde(default)]
    pub storage_class: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationPlanResponse {
    pub name: String,
    pub readiness: String,
    pub checks: Vec<MigrationReadinessCheck>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationExecuteRequest {
    pub name: String,
    pub namespace: String,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub import_url: Option<String>,
    #[serde(default)]
    pub storage_class: Option<String>,
    #[serde(default)]
    pub disk_size: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationExecuteResponse {
    pub status: String,
    pub message: String,
    pub datavolume: Option<String>,
    pub vmrogue_vm: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationSearchQuery {
    pub q: Option<String>,
    #[serde(flatten)]
    pub ns: DashboardNamespaceQuery,
}

// ── Router ──────────────────────────────────────────────────────────

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/experience/home", get(experience_home))
        .route("/experience/search", get(experience_search))
        .route("/experience/errors/translate", post(translate_error_handler))
        .route("/experience/migration/sources", get(migration_sources))
        .route("/experience/migration/scan", post(migration_scan))
        .route("/experience/migration/plan", post(migration_plan))
        .route("/experience/migration/execute", post(migration_execute))
        .route("/experience/migration/discover", post(migration_discover))
        .route("/experience/session", get(experience_session))
        .route("/experience/fleet/health", get(fleet_health))
        .route("/experience/templates", get(template_store))
        .route("/experience/locations", get(experience_locations))
        .with_state(state)
}

#[cfg(feature = "web")]
fn experience_context() -> super::feature_context::VmrogueFeatureContext {
    super::feature_context::VmrogueFeatureContext {
        data_source: "experience_compose".to_string(),
        scope: "Zyvor CloudOS experience API — aggregates KubeVirt/K8s into product vocabulary."
            .to_string(),
        limitations: "Health scores and migration readiness are heuristic until guest metrics and vCenter APIs are wired."
            .to_string(),
    }
}

#[cfg(feature = "web")]
async fn resolve_client_role(
    state: &crate::api::http_server::web::WebState,
    headers: &HeaderMap,
) -> crate::api::http_server::web::ApiRole {
    use crate::api::http_server::web::ApiRole;

    let key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(str::to_string)
        });

    let Some(key) = key else {
        return ApiRole::Admin;
    };

    if let Some(role) = state.authenticate(&key) {
        return role.clone();
    }
    if let Some(role) = state.authenticate_jwt(&key) {
        return role;
    }
    if crate::api::oidc::oidc_configured() {
        if let Some(rstr) = crate::api::oidc::oidc_role_from_bearer(&key).await {
            return match rstr.as_str() {
                "admin" => ApiRole::Admin,
                "write" => ApiRole::Write,
                _ => ApiRole::ReadOnly,
            };
        }
    }
    ApiRole::Admin
}

fn role_label(role: &crate::api::http_server::web::ApiRole) -> &'static str {
    use crate::api::http_server::web::ApiRole;
    match role {
        ApiRole::Admin => "admin",
        ApiRole::Write => "write",
        ApiRole::ReadOnly => "readonly",
    }
}

pub fn vm_health_score(status: &str, drift_detected: bool) -> (u8, String) {
    let base: u8 = match status {
        "Running" => 92,
        "Stopped" => 68,
        "Failed" | "Error" => 35,
        _ => 55,
    };
    let score = if drift_detected {
        base.saturating_sub(18)
    } else {
        base
    };
    let label = if score >= 85 {
        "Healthy"
    } else if score >= 65 {
        "Fair"
    } else if score >= 45 {
        "Needs attention"
    } else {
        "Critical"
    };
    (score, label.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperienceSessionResponse {
    pub product: String,
    pub role: String,
    pub can_write: bool,
    pub can_admin: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VmFleetHealthItem {
    pub namespace: String,
    pub name: String,
    pub status: String,
    pub health_score: u8,
    pub health_label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FleetHealthResponse {
    pub vmrogue_context: super::feature_context::VmrogueFeatureContext,
    pub items: Vec<VmFleetHealthItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateStoreItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub os_family: String,
    pub cpu: u32,
    pub memory: String,
    pub disk: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateStoreResponse {
    pub vmrogue_context: super::feature_context::VmrogueFeatureContext,
    pub templates: Vec<TemplateStoreItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocationItem {
    pub id: String,
    pub name: String,
    pub region: String,
    pub health: String,
    pub active: bool,
    pub vm_count: u32,
    pub node_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocationsResponse {
    pub vmrogue_context: super::feature_context::VmrogueFeatureContext,
    pub current_location: String,
    pub locations: Vec<LocationItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationDiscoverRequest {
    pub source: String,
    #[serde(default)]
    pub vcenter_host: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationDiscoverResponse {
    pub status: String,
    pub message: String,
    pub suggested_items: Vec<MigrationInventoryItem>,
}

#[cfg(feature = "web")]
async fn experience_session(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Json<ExperienceSessionResponse> {
    let s = state.read().await;
    let role = resolve_client_role(&s, &headers).await;
    let can_admin = role == crate::api::http_server::web::ApiRole::Admin;
    let can_write = can_admin || role == crate::api::http_server::web::ApiRole::Write;
    Json(ExperienceSessionResponse {
        product: "Zyvor CloudOS".to_string(),
        role: role_label(&role).to_string(),
        can_write,
        can_admin,
    })
}

#[cfg(feature = "web")]
async fn fleet_health(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<FleetHealthResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    let mut items = Vec::new();
    if let Ok(vms) = client.list_vms(&scope).await {
        for vm in vms {
            let name = vm.metadata.name.clone().unwrap_or_default();
            let ns = vm.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let status = vm
                .status
                .as_ref()
                .and_then(|s| s.printable_status.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            let (health_score, health_label) = vm_health_score(&status, false);
            items.push(VmFleetHealthItem {
                namespace: ns,
                name,
                status,
                health_score,
                health_label,
            });
        }
    }
    Json(FleetHealthResponse {
        vmrogue_context: experience_context(),
        items,
    })
}

#[cfg(feature = "web")]
async fn template_store() -> Json<TemplateStoreResponse> {
    use crate::templates::TEMPLATES;

    let names = TEMPLATES.list();
    let mut seen = std::collections::HashSet::new();
    let mut templates = Vec::new();
    for name in names {
        if !seen.insert(name.clone()) {
            continue;
        }
        let Some(cfg) = TEMPLATES.get(&name) else {
            continue;
        };
        let os = if name.starts_with("windows") {
            "windows"
        } else if name.contains("ubuntu") {
            "ubuntu"
        } else if name.contains("debian") {
            "debian"
        } else if name.contains("fedora") {
            "fedora"
        } else {
            "linux"
        };
        templates.push(TemplateStoreItem {
            id: name.clone(),
            title: name.clone(),
            subtitle: format!(
                "{} vCPU · {} · {}",
                cfg.cpu.cores,
                cfg.memory.size,
                cfg.disks
                    .first()
                    .map(|d| d.size.as_str())
                    .unwrap_or("—")
            ),
            os_family: os.to_string(),
            cpu: cfg.cpu.cores,
            memory: cfg.memory.size.clone(),
            disk: cfg
                .disks
                .first()
                .map(|d| d.size.clone())
                .unwrap_or_default(),
            tags: vec![os.to_string(), "kubevirt".to_string()],
        });
    }
    templates.sort_by(|a, b| a.id.cmp(&b.id));
    Json(TemplateStoreResponse {
        vmrogue_context: experience_context(),
        templates,
    })
}

#[cfg(feature = "web")]
async fn experience_locations(State(state): State<SharedState>) -> Json<LocationsResponse> {
    let s = state.read().await;
    let active = s.active_kube_context.as_deref();
    let current = super::clusters::effective_current_context(active);
    drop(s);

    let mut manager = crate::multi_cluster::MultiClusterManager::new();
    let _ = manager.discover_from_kubeconfig().await;
    let locations: Vec<LocationItem> = manager
        .clusters
        .iter()
        .map(|c| LocationItem {
            id: c.context.clone(),
            name: c.name.clone(),
            region: c.region.clone(),
            health: match c.health {
                crate::multi_cluster::ClusterHealth::Healthy => "healthy".to_string(),
                crate::multi_cluster::ClusterHealth::Degraded => "degraded".to_string(),
                crate::multi_cluster::ClusterHealth::Unhealthy => "unhealthy".to_string(),
                crate::multi_cluster::ClusterHealth::Unknown => "unknown".to_string(),
            },
            active: c.context == current,
            vm_count: c.vm_count as u32,
            node_count: c.node_count as u32,
        })
        .collect();

    let current_location = if locations.iter().any(|l| l.active) {
        locations
            .iter()
            .find(|l| l.active)
            .map(|l| l.name.clone())
            .unwrap_or_else(|| current.clone())
    } else {
        current.clone()
    };

    Json(LocationsResponse {
        vmrogue_context: experience_context(),
        current_location,
        locations,
    })
}

#[cfg(feature = "web")]
async fn migration_discover(Json(body): Json<MigrationDiscoverRequest>) -> Json<MigrationDiscoverResponse> {
    let host = body
        .vcenter_host
        .as_deref()
        .unwrap_or("vcenter.example.com");
    let msg = if body.source == "vmware" {
        format!(
            "vCenter API sync for {host} is not wired yet. Export VMs from vCenter (OVF/OVA or VMDK) \
             or paste inventory JSON, then run Scan readiness."
        )
    } else {
        "Use Scan readiness with exported inventory JSON for this source.".to_string()
    };
    Json(MigrationDiscoverResponse {
        status: "manual".to_string(),
        message: msg,
        suggested_items: vec![MigrationInventoryItem {
            name: "exported-workload".to_string(),
            os: Some("Linux".to_string()),
            firmware: Some("uefi".to_string()),
            disk_format: Some("vmdk".to_string()),
            disk_gb: Some(40),
            network: None,
            has_vmware_tools: Some(false),
            virtio_ready: Some(false),
        }],
    })
}

// ── Fix-it translator (shared with tests) ───────────────────────────

pub fn translate_error_message(message: &str, context: Option<&str>) -> TranslateErrorResponse {
    let lower = message.to_lowercase();
    let ctx = context.unwrap_or("general");

    let (title, explanation, fixes) = if lower.contains("persistentvolumeclaim")
        || lower.contains("unbound immediate persistentvolumeclaims")
        || lower.contains("waiting for a volume")
    {
        (
            "Disk storage is not ready",
            "This VM cannot start because its disk volume is still being provisioned or no storage is available for the requested StorageClass.",
            vec![
                FixAction {
                    label: "Check storage tiers".to_string(),
                    action: "navigate".to_string(),
                    page: Some("storage".to_string()),
                },
                FixAction {
                    label: "Retry start".to_string(),
                    action: "retry".to_string(),
                    page: None,
                },
            ],
        )
    } else if lower.contains("network attachment definition")
        || lower.contains("networkattachmentdefinition")
        || lower.contains("multus")
    {
        (
            "VM network is missing",
            "The VM references a network that does not exist in this workspace. Create the network or choose an existing one.",
            vec![
                FixAction {
                    label: "Open networks".to_string(),
                    action: "navigate".to_string(),
                    page: Some("cilium".to_string()),
                },
                FixAction {
                    label: "Edit VM network".to_string(),
                    action: "edit_network".to_string(),
                    page: Some("vms".to_string()),
                },
            ],
        )
    } else if lower.contains("insufficient cpu")
        || lower.contains("insufficient memory")
        || lower.contains("0/") && lower.contains("nodes are available")
    {
        (
            "Not enough cluster capacity",
            "No node can schedule this VM with its current CPU, memory, or placement rules.",
            vec![
                FixAction {
                    label: "View nodes".to_string(),
                    action: "navigate".to_string(),
                    page: Some("nodes".to_string()),
                },
                FixAction {
                    label: "Resize VM".to_string(),
                    action: "resize".to_string(),
                    page: Some("vms".to_string()),
                },
            ],
        )
    } else if lower.contains("forbidden") || lower.contains("403") {
        (
            "Permission denied",
            "Your API key or SSO role cannot perform this action on this resource.",
            vec![FixAction {
                label: "Review access".to_string(),
                action: "navigate".to_string(),
                page: Some("rbac".to_string()),
            }],
        )
    } else if ctx == "migrate" && (lower.contains("migration") || lower.contains("live migrate")) {
        (
            "Live migration blocked",
            "KubeVirt could not migrate this running VM. Common causes: shared storage missing, VM not running, or network/storage policy.",
            vec![
                FixAction {
                    label: "Check VM status".to_string(),
                    action: "navigate".to_string(),
                    page: Some("vms".to_string()),
                },
                FixAction {
                    label: "View nodes".to_string(),
                    action: "navigate".to_string(),
                    page: Some("nodes".to_string()),
                },
            ],
        )
    } else if lower.contains("not found") || lower.contains("404") {
        (
            "Resource not found",
            "The VM or related object may have been deleted or is in another workspace.",
            vec![FixAction {
                label: "Refresh VM list".to_string(),
                action: "refresh".to_string(),
                page: Some("vms".to_string()),
            }],
        )
    } else {
        (
            "Operation failed",
            "The platform returned an error. Check events and VM details for the underlying Kubernetes or KubeVirt cause.",
            vec![
                FixAction {
                    label: "View events".to_string(),
                    action: "navigate".to_string(),
                    page: Some("events".to_string()),
                },
                FixAction {
                    label: "Open VM".to_string(),
                    action: "navigate".to_string(),
                    page: Some("vms".to_string()),
                },
            ],
        )
    };

    TranslateErrorResponse {
        title: title.to_string(),
        explanation: explanation.to_string(),
        fixes,
        raw_message: message.to_string(),
    }
}

#[cfg(feature = "web")]
async fn translate_error_handler(
    Json(body): Json<TranslateErrorRequest>,
) -> Json<TranslateErrorResponse> {
    Json(translate_error_message(
        &body.message,
        body.context.as_deref(),
    ))
}

// ── Home ────────────────────────────────────────────────────────────

#[cfg(feature = "web")]
async fn experience_home(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<ExperienceHomeResponse> {
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    let workspace_label = if namespace_scope::is_all_namespaces(&scope) {
        "All workspaces".to_string()
    } else {
        scope.clone()
    };

    let vms = client.list_vms(&scope).await.unwrap_or_default();
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
    let failed = total.saturating_sub(running + stopped);

    let nodes = client.list_nodes().await.unwrap_or_default();
    let ready_nodes = nodes
        .iter()
        .filter(|n| {
            n.status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .and_then(|c| c.iter().find(|x| x.type_ == "Ready"))
                .is_some_and(|c| c.status == "True")
        })
        .count();

    let mut warnings = Vec::new();
    if failed > 0 {
        warnings.push(format!("{failed} VM(s) in failed or unknown state"));
    }
    if !nodes.is_empty() && ready_nodes < nodes.len() {
        warnings.push(format!(
            "{} node(s) not Ready",
            nodes.len().saturating_sub(ready_nodes)
        ));
    }

    let health_score = if total == 0 {
        96
    } else {
        let run_pct = (running as f64 / total as f64 * 70.0) as u8;
        let node_pct = if nodes.is_empty() {
            20
        } else {
            (ready_nodes as f64 / nodes.len() as f64 * 20.0) as u8
        };
        let warn_penalty = (warnings.len() as u8).min(15) * 3;
        (run_pct + node_pct + 10).saturating_sub(warn_penalty).min(100)
    };

    let health_summary = if health_score >= 90 {
        format!("{workspace_label} is healthy.")
    } else if health_score >= 70 {
        format!("{workspace_label} needs attention.")
    } else {
        format!("{workspace_label} has degraded workloads.")
    };

    let greeting = "Welcome back.".to_string();

    let stats = serde_json::json!({
        "vms_running": running,
        "vms_total": total,
        "vms_stopped": stopped,
        "nodes_online": ready_nodes,
        "nodes_total": nodes.len(),
        "warnings": warnings.len(),
    });

    let control_center = vec![
        ControlCenterItem {
            label: "Cluster health".to_string(),
            value: format!("{health_score}%"),
            status: if health_score >= 85 {
                "ok".to_string()
            } else if health_score >= 65 {
                "warn".to_string()
            } else {
                "error".to_string()
            },
        },
        ControlCenterItem {
            label: "Running VMs".to_string(),
            value: format!("{running} / {total}"),
            status: if failed > 0 { "warn".to_string() } else { "ok".to_string() },
        },
        ControlCenterItem {
            label: "Nodes ready".to_string(),
            value: format!("{ready_nodes} / {}", nodes.len()),
            status: if ready_nodes == nodes.len() || nodes.is_empty() {
                "ok".to_string()
            } else {
                "warn".to_string()
            },
        },
        ControlCenterItem {
            label: "Warnings".to_string(),
            value: warnings.len().to_string(),
            status: if warnings.is_empty() {
                "ok".to_string()
            } else {
                "warn".to_string()
            },
        },
    ];

    let action_cards = vec![
        HomeActionCard {
            id: "create-vm".to_string(),
            title: "Create VM".to_string(),
            subtitle: "Launch from template".to_string(),
            icon: "plus".to_string(),
            action: "create_vm".to_string(),
        },
        HomeActionCard {
            id: "migration".to_string(),
            title: "Migration Assistant".to_string(),
            subtitle: "VMware / OVA / VMDK".to_string(),
            icon: "migrate".to_string(),
            action: "navigate:migration".to_string(),
        },
        HomeActionCard {
            id: "console".to_string(),
            title: "Open Console".to_string(),
            subtitle: "VNC or serial".to_string(),
            icon: "console".to_string(),
            action: "navigate:vms".to_string(),
        },
        HomeActionCard {
            id: "backups".to_string(),
            title: "View Backups".to_string(),
            subtitle: "Snapshots & Velero".to_string(),
            icon: "backup".to_string(),
            action: "navigate:backups".to_string(),
        },
        HomeActionCard {
            id: "health".to_string(),
            title: "Run Health Check".to_string(),
            subtitle: "Fleet + nodes".to_string(),
            icon: "health".to_string(),
            action: "navigate:monitoring".to_string(),
        },
        HomeActionCard {
            id: "catalog".to_string(),
            title: "Template Store".to_string(),
            subtitle: "Sync & deploy".to_string(),
            icon: "catalog".to_string(),
            action: "navigate:app-store".to_string(),
        },
    ];

    Json(ExperienceHomeResponse {
        vmrogue_context: experience_context(),
        experience: ExperienceContext {
            product: "Zyvor CloudOS".to_string(),
            tagline: "A macOS-like control plane for Kubernetes-native virtualization.".to_string(),
        },
        greeting,
        workspace: workspace_label,
        health_summary,
        health_score,
        stats,
        control_center,
        action_cards,
        warnings,
    })
}

// ── Search ──────────────────────────────────────────────────────────

#[cfg(feature = "web")]
async fn experience_search(
    State(state): State<SharedState>,
    Query(q): Query<MigrationSearchQuery>,
) -> Json<ExperienceSearchResponse> {
    let query = q.q.clone().unwrap_or_default();
    let needle = query.to_lowercase();
    let mut results = Vec::new();

    if needle.is_empty() {
        return Json(ExperienceSearchResponse { query, results });
    }

    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.ns.namespace.clone(), &default_ns);

    if let Ok(vms) = client.list_vms(&scope).await {
        for vm in vms {
            let name = vm.metadata.name.as_deref().unwrap_or("");
            let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
            if name.to_lowercase().contains(&needle) || ns.to_lowercase().contains(&needle) {
                let status = vm
                    .status
                    .as_ref()
                    .and_then(|s| s.printable_status.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                results.push(SearchResultItem {
                    kind: "vm".to_string(),
                    id: format!("{ns}/{name}"),
                    title: name.to_string(),
                    subtitle: format!("{ns} · {status}"),
                    action: format!("open_vm:{ns}:{name}"),
                });
            }
        }
    }

    let pages = [
        ("dashboard", "Datacenter Home", "Fleet overview"),
        ("vms", "Virtual Machines", "KubeVirt fleet"),
        ("app-store", "Template Store", "App Store for VM images"),
        ("migration", "Migration Assistant", "VMware / OVA import"),
        ("catalog", "Catalog (advanced)", "VMTemplate CRD sync"),
        ("backups", "Backups", "Snapshots & Velero"),
        ("monitoring", "Activity Monitor", "Health & metrics"),
        ("integrations", "Integrations", "Prometheus, Grafana, …"),
    ];
    for (id, title, sub) in pages {
        if title.to_lowercase().contains(&needle)
            || id.contains(&needle)
            || sub.to_lowercase().contains(&needle)
        {
            results.push(SearchResultItem {
                kind: "page".to_string(),
                id: id.to_string(),
                title: title.to_string(),
                subtitle: sub.to_string(),
                action: format!("navigate:{id}"),
            });
        }
    }

    if needle.contains("migrate") {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "migrate".to_string(),
            title: "Live migrate a VM".to_string(),
            subtitle: "Open Virtual Machines".to_string(),
            action: "navigate:vms".to_string(),
        });
    }

    results.truncate(25);
    Json(ExperienceSearchResponse { query, results })
}

// ── Migration assistant ─────────────────────────────────────────────

fn source_label(source: &str) -> &str {
    match source {
        "vmware" => "VMware vCenter / ESXi",
        "ova" => "OVA / OVF package",
        "vmdk" => "VMDK disk file",
        "hyperv" => "Hyper-V",
        "proxmox" => "Proxmox",
        _ => "Custom source",
    }
}

fn assess_migration_item(item: &MigrationInventoryItem) -> (String, Vec<MigrationReadinessCheck>, Vec<String>) {
    let mut checks = Vec::new();
    let mut blocked = false;
    let mut needs_prep = false;

    let fw = item.firmware.as_deref().unwrap_or("bios").to_lowercase();
    if fw.contains("efi") || fw.contains("uefi") {
        checks.push(MigrationReadinessCheck {
            id: "firmware".to_string(),
            label: "UEFI firmware".to_string(),
            status: "ok".to_string(),
            detail: "Compatible with KubeVirt UEFI templates.".to_string(),
        });
    } else {
        checks.push(MigrationReadinessCheck {
            id: "firmware".to_string(),
            label: "BIOS firmware".to_string(),
            status: "warn".to_string(),
            detail: "Verify template uses matching firmware.".to_string(),
        });
        needs_prep = true;
    }

    let fmt = item.disk_format.as_deref().unwrap_or("vmdk").to_lowercase();
    if fmt.contains("vmdk") || fmt.contains("qcow") || fmt.contains("raw") {
        checks.push(MigrationReadinessCheck {
            id: "disk".to_string(),
            label: "Disk format".to_string(),
            status: "ok".to_string(),
            detail: format!("{fmt} can be imported via CDI."),
        });
    } else {
        checks.push(MigrationReadinessCheck {
            id: "disk".to_string(),
            label: "Disk format".to_string(),
            status: "error".to_string(),
            detail: "Convert to VMDK/QCOW2 before import.".to_string(),
        });
        blocked = true;
    }

    if item.virtio_ready == Some(true) {
        checks.push(MigrationReadinessCheck {
            id: "virtio".to_string(),
            label: "VirtIO drivers".to_string(),
            status: "ok".to_string(),
            detail: "Guest is VirtIO-ready.".to_string(),
        });
    } else {
        checks.push(MigrationReadinessCheck {
            id: "virtio".to_string(),
            label: "VirtIO drivers".to_string(),
            status: "warn".to_string(),
            detail: "Plan driver injection or install virtio-win in guest.".to_string(),
        });
        needs_prep = true;
    }

    if item.has_vmware_tools == Some(true) {
        checks.push(MigrationReadinessCheck {
            id: "tools".to_string(),
            label: "VMware Tools".to_string(),
            status: "warn".to_string(),
            detail: "Replace with QEMU guest agent / Cloudbase-Init after cutover.".to_string(),
        });
        needs_prep = true;
    }

    if item.network.is_none() {
        checks.push(MigrationReadinessCheck {
            id: "network".to_string(),
            label: "Network mapping".to_string(),
            status: "warn".to_string(),
            detail: "Map source network to a workspace network (NAD or pod).".to_string(),
        });
        needs_prep = true;
    } else {
        checks.push(MigrationReadinessCheck {
            id: "network".to_string(),
            label: "Network mapping".to_string(),
            status: "ok".to_string(),
            detail: format!("Target network: {}", item.network.as_deref().unwrap_or("default")),
        });
    }

    let readiness = if blocked {
        "blocked"
    } else if needs_prep {
        "needs_preparation"
    } else {
        "ready"
    };

    let plan = vec![
        "Convert disk to QCOW2/RAW if required (virt-v2v or qemu-img)".to_string(),
        "Import disk via CDI DataVolume (URL upload or registry)".to_string(),
        "Create VM from template + profile or VMRogueVM".to_string(),
        "Verify boot and install guest tools".to_string(),
        "Cut over traffic and decommission source VM".to_string(),
    ];

    (readiness.to_string(), checks, plan)
}

#[cfg(feature = "web")]
async fn migration_sources() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "sources": [
            {"id": "vmware", "label": "VMware vCenter / ESXi", "formats": ["vmdk", "ova"]},
            {"id": "ova", "label": "OVA / OVF package", "formats": ["ova", "ovf"]},
            {"id": "vmdk", "label": "VMDK file", "formats": ["vmdk"]},
            {"id": "hyperv", "label": "Hyper-V", "formats": ["vhdx"]},
            {"id": "proxmox", "label": "Proxmox", "formats": ["qcow2", "raw"]}
        ],
        "pipeline": "Source disk → CDI DataVolume → PVC → KubeVirt VirtualMachine"
    }))
}

#[cfg(feature = "web")]
async fn migration_scan(Json(body): Json<MigrationScanRequest>) -> Json<MigrationScanResponse> {
    let mut items_out = Vec::new();
    let inv = if body.items.is_empty() {
        vec![MigrationInventoryItem {
            name: "example-workload".to_string(),
            os: Some("Linux".to_string()),
            firmware: Some("uefi".to_string()),
            disk_format: Some("vmdk".to_string()),
            disk_gb: Some(40),
            network: None,
            has_vmware_tools: Some(false),
            virtio_ready: Some(true),
        }]
    } else {
        body.items.clone()
    };

    let mut ready = 0u32;
    let mut needs = 0u32;
    let mut blocked = 0u32;

    for item in &inv {
        let (readiness, checks, plan) = assess_migration_item(item);
        match readiness.as_str() {
            "ready" => ready += 1,
            "blocked" => blocked += 1,
            _ => needs += 1,
        }
        items_out.push(MigrationScanItem {
            name: item.name.clone(),
            readiness,
            checks,
            recommended_plan: plan,
        });
    }

    Json(MigrationScanResponse {
        source: body.source.clone(),
        source_label: source_label(&body.source).to_string(),
        total: items_out.len() as u32,
        ready,
        needs_preparation: needs,
        blocked,
        items: items_out,
    })
}

#[cfg(feature = "web")]
async fn migration_plan(Json(body): Json<MigrationPlanRequest>) -> Json<MigrationPlanResponse> {
    let (readiness, checks, steps) = assess_migration_item(&body.item);
    Json(MigrationPlanResponse {
        name: body.item.name.clone(),
        readiness,
        checks,
        steps,
    })
}

#[cfg(feature = "web")]
async fn migration_execute(
    State(state): State<SharedState>,
    Json(body): Json<MigrationExecuteRequest>,
) -> Result<Json<MigrationExecuteResponse>, (StatusCode, String)> {
    use kube::api::{Api, ApiResource, DynamicObject, PostParams};

    let name = body.name.trim();
    let ns = body.namespace.trim();
    if name.is_empty() || ns.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "name and namespace required".to_string()));
    }

    let s = state.read().await;
    let client = s.client().client();
    let mut dv_name: Option<String> = None;

    if let Some(url) = body.import_url.as_ref().filter(|u| !u.is_empty()) {
        let size = body
            .disk_size
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "40Gi".to_string());
        let sc = body.storage_class.clone();
        let dv_meta_name = format!("import-{}", name.replace('.', "-"));
        let mut spec = serde_json::json!({
            "apiVersion": "cdi.kubevirt.io/v1beta1",
            "kind": "DataVolume",
            "metadata": {
                "name": dv_meta_name,
                "namespace": ns,
                "labels": {
                    "vmrogue.io/managed-by": "vmrogue-migration",
                    "vmrogue.io/source-vm": name
                }
            },
            "spec": {
                "source": { "http": { "url": url } },
                "pvc": {
                    "accessModes": ["ReadWriteOnce"],
                    "resources": { "requests": { "storage": size } }
                }
            }
        });
        if let Some(ref sc_name) = sc {
            if let Some(pvc) = spec
                .pointer_mut("/spec/pvc")
                .and_then(|p| p.as_object_mut())
            {
                pvc.insert("storageClassName".to_string(), serde_json::json!(sc_name));
            }
        }
        let ar = ApiResource {
            group: "cdi.kubevirt.io".to_string(),
            version: "v1beta1".to_string(),
            api_version: "cdi.kubevirt.io/v1beta1".to_string(),
            kind: "DataVolume".to_string(),
            plural: "datavolumes".to_string(),
        };
        let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), ns, &ar);
        let obj: DynamicObject = serde_json::from_value(spec)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid DV spec: {e}")))?;
        api.create(&PostParams::default(), &obj)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("CDI import: {e}")))?;
        dv_name = Some(dv_meta_name);
    }

    let vrvm_name = if body.template.is_some() || body.profile.is_some() {
        use crate::operator_crds::{VMRogueVM, VMRogueVMSpec, CRDCPUSpec, CRDMemorySpec};
        let tpl = body.template.clone();
        let prof = body.profile.clone();
        let spec = VMRogueVMSpec {
            template: tpl,
            profile: prof,
            cpu: CRDCPUSpec {
                cores: 2,
                sockets: 1,
                threads: 1,
                model: None,
                dedicated_cpu_placement: None,
                isolate_emulator_thread: None,
            },
            memory: CRDMemorySpec {
                size: "4Gi".to_string(),
                hugepages_page_size: None,
                max_guest: None,
            },
            disks: vec![],
            interfaces: vec![],
            cloud_init: None,
            features: None,
            firmware: None,
            clock: None,
            eviction_strategy: None,
            termination_grace_period: None,
            enable_tpm: false,
            enable_rng: false,
            machine_type: None,
            running: Some(true),
            labels: Default::default(),
            annotations: Default::default(),
            allow_internet: true,
            windows: None,
        };
        let vm_name = format!("migrated-{}", name.replace('.', "-"));
        let vm = VMRogueVM::new(&vm_name, spec);
        let api: Api<VMRogueVM> = Api::namespaced(client.clone(), ns);
        api.create(&PostParams::default(), &vm)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("VMRogueVM: {e}")))?;
        Some(vm_name)
    } else {
        None
    };

    Ok(Json(MigrationExecuteResponse {
        status: "accepted".to_string(),
        message: if dv_name.is_some() {
            "CDI import started; create or attach VM when disk is Ready.".to_string()
        } else {
            "Migration plan recorded; provide import_url to start CDI import.".to_string()
        },
        datavolume: dv_name,
        vmrogue_vm: vrvm_name,
    }))
}

#[cfg(test)]
mod tests {
    use super::{translate_error_message, vm_health_score};

    #[test]
    fn translate_pvc_pending() {
        let (score, label) = vm_health_score("Running", false);
        assert!(score >= 85);
        assert_eq!(label, "Healthy");
        let t = translate_error_message(
            "0/5 nodes are available: pod has unbound immediate PersistentVolumeClaims",
            Some("start_vm"),
        );
        assert!(t.title.contains("Disk"));
        assert!(!t.fixes.is_empty());
    }

    #[test]
    fn translate_nad_missing() {
        let t = translate_error_message(
            "NetworkAttachmentDefinition \"prod-vlan\" not found",
            None,
        );
        assert!(t.explanation.contains("network"));
    }
}
