// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

pub mod alerts;
#[cfg(feature = "web")]
pub mod atlas;
pub mod audit;
pub mod auth;
pub mod autoscaler;
pub mod backups;
pub mod capacity;
pub mod catalog;
pub mod chaos_engineering;
pub mod cilium;
pub mod clones;
pub mod clusters;
pub mod compliance;
pub mod compute;
pub mod copilot;
pub mod costs;
pub mod custom_dashboards;
pub mod custom_resources;
pub mod dependencies;
pub mod disks;
pub mod dr;
#[cfg(feature = "web")]
pub mod enterprise;
pub mod events;
pub mod experience;
pub mod feature_context;
pub mod forecasting;
pub mod gitops;
pub mod gpus;
pub mod guest_ops;
pub mod health;
pub mod heatmap;
pub mod helm;
pub mod hpa;
pub mod images;
pub mod incidents;
pub mod ingress;
pub mod integrations;
pub mod kryton;
pub mod logs;
pub mod metrics;
pub mod migration_policies;
pub mod migrations;
pub mod monitoring;
pub mod multus;
pub mod namespace_scope;
pub mod namespaces;
#[cfg(feature = "web")]
pub mod netra;
pub mod network;
pub mod network_policies;
pub mod nodes;
pub mod notifications;
pub mod observability;
pub mod operators;
#[cfg(feature = "web")]
pub mod paqtra;
pub mod performance;
pub mod pods;
pub mod quotas;
pub mod rbac;
pub mod rdp;
pub mod recommendations;
pub mod scheduling;
pub mod security;
pub mod self_healing;
pub mod slo;
pub mod snapshots;
pub mod soc;
pub mod storage;
pub mod templates;
pub mod tenants;
pub mod topology;
pub mod traces;
pub mod velero;
pub mod vmis;
pub mod webhooks;
pub mod windows;
pub mod workloads;

/// Veyron CRD management handlers (veyron.io/v1alpha1).
pub mod crds;

/// Map a failed upstream Kubernetes list/query into a `500` JSON error.
///
/// Convention (CLAUDE.md): handlers must never silently return `0`/empty where
/// real data is expected. Use `list.await.map_err(kube_list_error("VMs"))?`
/// instead of `unwrap_or_default()` so an apiserver outage / RBAC denial surfaces
/// as a 5xx rather than a healthy-looking empty result.
#[cfg(feature = "web")]
pub fn kube_list_error<E: std::fmt::Display>(
    resource: &'static str,
) -> impl FnOnce(E) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    move |e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": "UPSTREAM_QUERY_FAILED",
                "message": format!("failed to list {resource}: {e}"),
            })),
        )
    }
}

/// Build the combined API router from all handler sub-routers.
///
/// Handlers receive SharedState and call into `KubeClient`. Some domains return
/// heuristic analytics (costs, forecasting, observability); core VM lifecycle data is live K8s.
///
/// NOTE: Authentication is NOT applied here. The parent router (see
/// `http_server::web::build_router`) is responsible for layering the
/// `auth_middleware` so that all merged routes are protected uniformly.
#[cfg(feature = "web")]
pub fn all_routes(
    state: std::sync::Arc<tokio::sync::RwLock<crate::api::http_server::web::WebState>>,
) -> axum::Router {
    axum::Router::new()
        .merge(auth::router(state.clone()))
        // Real K8s data handlers (wired to KubeClient via SharedState)
        // NOTE: pods, nodes, events, namespaces, snapshots, templates are served from
        // `http_server.rs` under `/api/v1/...` (ApiResponse envelope); do not merge here or routes duplicate after nest.
        .merge(vmis::router(state.clone()))
        .merge(metrics::router(state.clone()))
        .merge(costs::router(state.clone()))
        .merge(crds::router(state.clone()))
        .merge(logs::router(state.clone()))
        .merge(gpus::router(state.clone()))
        .merge(migrations::router(state.clone()))
        .merge(migration_policies::router(state.clone()))
        .merge(storage::router(state.clone()))
        .merge(disks::router(state.clone()))
        .merge(network::router(state.clone()))
        .merge(topology::router(state.clone()))
        .merge(dependencies::router(state.clone()))
        // K8s-wired handlers (Phase 2)
        .merge(clones::router(state.clone()))
        .merge(rbac::router(state.clone()))
        .merge(quotas::router(state.clone()))
        .merge(network_policies::router(state.clone()))
        .merge(workloads::router(state.clone()))
        .merge(ingress::router(state.clone()))
        .merge(hpa::router(state.clone()))
        .merge(backups::router(state.clone()))
        .merge(atlas::router(state.clone()))
        .merge(kryton::router(state.clone()))
        .merge(compute::router(state.clone()))
        .merge(guest_ops::router(state.clone()))
        .merge(capacity::router(state.clone()))
        .merge(self_healing::router(state.clone()))
        .merge(catalog::router(state.clone()))
        .merge(experience::router(state.clone()))
        .merge(copilot::router(state.clone()))
        .merge(crate::ai::api::router(state.clone()))
        .merge(security::router(state.clone()))
        .merge(soc::router(state.clone()))
        .merge(compliance::router(state.clone()))
        .merge(monitoring::router(state.clone()))
        // K8s-wired handlers (Phase 2 batch 2)
        .merge(autoscaler::router(state.clone()))
        .merge(forecasting::router(state.clone()))
        .merge(alerts::router(state.clone()))
        .merge(audit::router(state.clone()))
        .merge(notifications::router(state.clone()))
        .merge(gitops::router(state.clone()))
        .merge(scheduling::router(state.clone()))
        .merge(cilium::router(state.clone()))
        .merge(observability::router(state.clone()))
        .merge(performance::router(state.clone()))
        .merge(recommendations::router(state.clone()))
        .merge(heatmap::router(state.clone()))
        .merge(custom_dashboards::router(state.clone()))
        .merge(webhooks::router(state.clone()))
        .merge(helm::router(state.clone()))
        .merge(operators::router(state.clone()))
        .merge(custom_resources::router(state.clone()))
        .merge(chaos_engineering::router(state.clone()))
        .merge(slo::router(state.clone()))
        .merge(incidents::router(state.clone()))
        .merge(traces::router(state.clone()))
        // RDP (VM discovery + config, proxy requires external gateway)
        .merge(rdp::router(state.clone()))
        .merge(dr::router(state.clone()))
        .merge(windows::router())
        .merge(tenants::router(state.clone()))
        .merge(enterprise::router(state.clone()))
        .merge(multus::router(state.clone()))
        .merge(images::router(state.clone()))
        .merge(velero::router(state.clone()))
        .merge(clusters::router(state.clone()))
        .merge(integrations::router(state.clone()))
        .merge(netra::router(state.clone()))
        .merge(paqtra::router(state.clone()))
}
