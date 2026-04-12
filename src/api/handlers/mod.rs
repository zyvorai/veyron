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
pub mod vmis;
pub mod webhooks;
pub mod workloads;

/// Build the combined API router from all handler sub-routers.
///
/// NOTE: Authentication is NOT applied here. The parent router (see
/// `http_server::web::build_router`) is responsible for layering the
/// `auth_middleware` so that all merged routes are protected uniformly.
#[cfg(feature = "web")]
pub fn all_routes() -> axum::Router {
    axum::Router::new()
        .merge(vmis::router())
        .merge(pods::router())
        .merge(nodes::router())
        .merge(events::router())
        .merge(snapshots::router())
        .merge(clones::router())
        .merge(disks::router())
        .merge(templates::router())
        .merge(migrations::router())
        .merge(namespaces::router())
        .merge(quotas::router())
        .merge(metrics::router())
        .merge(health::router())
        .merge(logs::router())
        .merge(costs::router())
        .merge(security::router())
        .merge(rbac::router())
        .merge(compliance::router())
        .merge(autoscaler::router())
        .merge(topology::router())
        .merge(dependencies::router())
        .merge(forecasting::router())
        .merge(alerts::router())
        .merge(audit::router())
        .merge(notifications::router())
        .merge(gitops::router())
        .merge(backups::router())
        .merge(network::router())
        .merge(network_policies::router())
        .merge(storage::router())
        .merge(workloads::router())
        .merge(scheduling::router())
        .merge(hpa::router())
        .merge(ingress::router())
        .merge(cilium::router())
        .merge(monitoring::router())
        .merge(observability::router())
        .merge(performance::router())
        .merge(recommendations::router())
        .merge(heatmap::router())
        .merge(custom_dashboards::router())
        .merge(webhooks::router())
        .merge(helm::router())
        .merge(operators::router())
        .merge(custom_resources::router())
        .merge(chaos_engineering::router())
        .merge(slo::router())
        .merge(rdp::router())
}
