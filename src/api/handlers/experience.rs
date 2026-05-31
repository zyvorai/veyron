// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! ZeusOS experience layer — home, search, fix-it errors.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap},
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
pub struct PinnedVmSummary {
    pub namespace: String,
    pub name: String,
    pub status: String,
    pub cpu: String,
    pub memory: String,
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
    pub copilot_briefing: Vec<crate::copilot::CopilotBriefingItem>,
    pub warnings: Vec<String>,
    pub pinned_vms: Vec<PinnedVmSummary>,
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

#[derive(Debug, Clone, Deserialize)]
pub struct ExperienceSearchQuery {
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
        scope: "ZeusOS experience API — aggregates KubeVirt/K8s into product vocabulary."
            .to_string(),
        limitations: "Guest metrics use QEMU agent + optional Prometheus.".to_string(),
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
    crate::copilot::fixit::vm_health_score(status, drift_detected)
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

// ── Fix-it translator (shared with tests) ───────────────────────────

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
        product: "ZeusOS".to_string(),
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
                cfg.default_disk_size_label()
            ),
            os_family: os.to_string(),
            cpu: cfg.cpu.cores,
            memory: cfg.memory.size.clone(),
            disk: cfg.default_disk_size_label(),
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

// ── Fix-it translator (shared with tests) ───────────────────────────

pub fn translate_error_message(message: &str, context: Option<&str>) -> TranslateErrorResponse {
    let r = crate::copilot::fixit::explain_error_message(message, context);
    TranslateErrorResponse {
        title: r.title,
        explanation: r.summary,
        fixes: r
            .actions
            .into_iter()
            .map(|a| FixAction {
                label: a.label,
                action: a.action,
                page: a.page,
            })
            .collect(),
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
            id: "copilot".to_string(),
            title: "VMRogue Copilot".to_string(),
            subtitle: "Doctor, YAML, backups, network".to_string(),
            icon: "copilot".to_string(),
            action: "open_copilot".to_string(),
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

    let mut pinned_vms = Vec::new();
    for vm in vms.iter().filter(|vm| {
        vm.status
            .as_ref()
            .and_then(|s| s.printable_status.as_deref())
            == Some("Running")
    }) {
        let name = vm.metadata.name.clone().unwrap_or_default();
        let ns = vm
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| scope.clone());
        let status = vm
            .status
            .as_ref()
            .and_then(|s| s.printable_status.clone())
            .unwrap_or_else(|| "Running".to_string());
        let (cpu, memory) = {
            let info = crate::tui::state::VmInfo::from_vm(vm);
            (info.cpu, info.memory)
        };
        pinned_vms.push(PinnedVmSummary {
            namespace: ns,
            name,
            status,
            cpu,
            memory,
        });
        if pinned_vms.len() >= 4 {
            break;
        }
    }

    let copilot_briefing = crate::copilot::fleet_briefing(&client, &scope).await;

    Json(ExperienceHomeResponse {
        vmrogue_context: experience_context(),
        experience: ExperienceContext {
            product: "ZeusOS".to_string(),
            tagline: "A macOS-like control plane for Kubernetes-native virtualization.".to_string(),
        },
        greeting,
        workspace: workspace_label,
        health_summary,
        health_score,
        stats,
        control_center,
        action_cards,
        copilot_briefing,
        warnings,
        pinned_vms,
    })
}

// ── Search ──────────────────────────────────────────────────────────

#[cfg(feature = "web")]
async fn experience_search(
    State(state): State<SharedState>,
    Query(q): Query<ExperienceSearchQuery>,
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
        ("dashboard", "ZeusOS Dashboard", "Fleet overview"),
        ("vms", "Virtual Machines", "KubeVirt fleet"),
        ("app-store", "Template Store", "App Store for VM images"),
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
            id: "live-migrate".to_string(),
            title: "Live migrate a VM".to_string(),
            subtitle: "Open Virtual Machines".to_string(),
            action: "navigate:vms".to_string(),
        });
    }

    if needle.contains("create") || needle.contains("forge") || needle == "vm" {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "forge-vm".to_string(),
            title: "Forge a new VM".to_string(),
            subtitle: "Open create dialog".to_string(),
            action: "forge_vm".to_string(),
        });
    }
    if needle.contains("backup") {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "backups".to_string(),
            title: "View backups".to_string(),
            subtitle: "Snapshots & Velero".to_string(),
            action: "navigate:backups".to_string(),
        });
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "backup-advisor".to_string(),
            title: "Backup Advisor".to_string(),
            subtitle: "Find VMs without snapshots or schedules".to_string(),
            action: "copilot:backup".to_string(),
        });
    }

    if needle.contains("copilot")
        || needle.contains("ai ")
        || needle == "ai"
        || needle.contains("doctor")
        || needle.contains("diagnose")
    {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "copilot".to_string(),
            title: "VMRogue Copilot".to_string(),
            subtitle: "Ask about VM health, YAML, errors".to_string(),
            action: "open_copilot".to_string(),
        });
    }
    if needle.contains("unhealthy") || needle.contains("degraded") {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "unhealthy-vms".to_string(),
            title: "Show unhealthy VMs".to_string(),
            subtitle: "VMRogue Doctor fleet scan".to_string(),
            action: "copilot:unhealthy".to_string(),
        });
    }
    if needle.contains("network") || needle.contains("connectivity") || needle.contains("firewall") {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "network-lens".to_string(),
            title: "Network Lens".to_string(),
            subtitle: "Inspect VM interfaces and policies".to_string(),
            action: "open_copilot".to_string(),
        });
    }
    if needle.contains("scheduling") || needle.contains("pending") || needle.contains("unschedulable") {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "scheduling-explainer".to_string(),
            title: "Scheduling Explainer".to_string(),
            subtitle: "Why a VM is stuck Pending".to_string(),
            action: "copilot:scheduling".to_string(),
        });
    }
    if needle.contains("cost") || needle.contains("expensive") || needle.contains("spend") {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "cost-advisor".to_string(),
            title: "Cost Advisor".to_string(),
            subtitle: "Fleet spend and top costly VMs".to_string(),
            action: "copilot:cost".to_string(),
        });
    }
    if needle.contains("compliance") || needle.contains("cis") || needle.contains("soc") {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "compliance-advisor".to_string(),
            title: "Compliance Advisor".to_string(),
            subtitle: "Framework scores and failing checks".to_string(),
            action: "copilot:compliance".to_string(),
        });
    }
    if needle.contains("drift") {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "drift-advisor".to_string(),
            title: "Operator drift scan".to_string(),
            subtitle: "VMRogueVM spec vs cluster state".to_string(),
            action: "copilot:drift".to_string(),
        });
    }
    if needle.contains("quota") || needle.contains("limit") {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "quota-advisor".to_string(),
            title: "Quota Advisor".to_string(),
            subtitle: "Namespaces near ResourceQuota limits".to_string(),
            action: "copilot:quota".to_string(),
        });
    }
    if needle.contains("catalog") || (needle.contains("template") && needle.contains("sync")) {
        results.push(SearchResultItem {
            kind: "action".to_string(),
            id: "catalog-advisor".to_string(),
            title: "Catalog Advisor".to_string(),
            subtitle: "VMTemplate / VMProfile sync health".to_string(),
            action: "copilot:catalog".to_string(),
        });
    }

    results.truncate(25);
    Json(ExperienceSearchResponse { query, results })
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
