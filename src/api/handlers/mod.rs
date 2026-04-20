pub mod alerts;
pub mod audit;
pub mod autoscaler;
pub mod backups;
pub mod chaos_engineering;
pub mod cilium;
pub mod clones;
pub mod compliance;
pub mod costs;
pub mod custom_dashboards;
pub mod custom_resources;
pub mod dependencies;
pub mod disks;
pub mod events;
pub mod forecasting;
pub mod gitops;
pub mod health;
pub mod heatmap;
pub mod helm;
pub mod hpa;
pub mod incidents;
pub mod ingress;
pub mod logs;
pub mod metrics;
pub mod migrations;
pub mod monitoring;
pub mod namespaces;
pub mod network;
pub mod network_policies;
pub mod nodes;
pub mod notifications;
pub mod observability;
pub mod operators;
pub mod performance;
pub mod pods;
pub mod quotas;
pub mod rbac;
pub mod rdp;
pub mod recommendations;
pub mod scheduling;
pub mod security;
pub mod slo;
pub mod snapshots;
pub mod storage;
pub mod templates;
pub mod topology;
pub mod traces;
pub mod vmis;
pub mod webhooks;
pub mod workloads;

/// VMRogue CRD management handlers (vmrogue.io/v1alpha1).
pub mod crds;

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
        // Real K8s data handlers (wired to KubeClient via SharedState)
        // NOTE: pods, nodes, events, namespaces, snapshots, templates are served from
        // `http_server.rs` under `/api/v1/...` (ApiResponse envelope); do not merge here or routes duplicate after nest.
        .merge(vmis::router(state.clone()))
        .merge(metrics::router(state.clone()))
        .merge(costs::router(state.clone()))
        .merge(crds::router(state.clone()))
        .merge(logs::router(state.clone()))
        .merge(migrations::router(state.clone()))
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
        .merge(security::router(state.clone()))
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
}
