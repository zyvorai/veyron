#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use k8s_openapi::api::core::v1::Container;

/// Workload response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadResponse {
    pub name: String,
    pub namespace: String,
    pub workload_type: String,
    pub replicas: u32,
    pub ready_replicas: u32,
    pub cpu_request: String,
    pub memory_request: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkloadQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/workloads", get(list_workloads))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_workloads(
    State(state): State<SharedState>,
    Query(query): Query<WorkloadQuery>,
) -> Json<Vec<WorkloadResponse>> {
    use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};

    let s = state.read().await;
    let client = s.client().client();
    let mut results: Vec<WorkloadResponse> = Vec::new();
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());

    // Fetch all workload types concurrently
    let deploy_api: kube::api::Api<Deployment> = if namespace == "all" {
        kube::api::Api::all(client.clone())
    } else {
        kube::api::Api::namespaced(client.clone(), &namespace)
    };
    let sts_api: kube::api::Api<StatefulSet> = if namespace == "all" {
        kube::api::Api::all(client.clone())
    } else {
        kube::api::Api::namespaced(client.clone(), &namespace)
    };
    let ds_api: kube::api::Api<DaemonSet> = if namespace == "all" {
        kube::api::Api::all(client.clone())
    } else {
        kube::api::Api::namespaced(client.clone(), &namespace)
    };

    let lp = kube::api::ListParams::default();
    let (deploy_result, sts_result, ds_result) =
        tokio::join!(deploy_api.list(&lp), sts_api.list(&lp), ds_api.list(&lp),);

    // Deployments
    if let Ok(deploys) = deploy_result {
        for d in &deploys.items {
            let status = d.status.as_ref();
            let containers = d
                .spec
                .as_ref()
                .and_then(|s| s.template.spec.as_ref())
                .map(|spec| spec.containers.as_slice())
                .unwrap_or(&[]);
            let (cpu_request, memory_request) = summarize_container_requests(containers);
            results.push(WorkloadResponse {
                name: d.metadata.name.clone().unwrap_or_default(),
                namespace: d.metadata.namespace.clone().unwrap_or_default(),
                workload_type: "Deployment".to_string(),
                replicas: d.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1) as u32,
                ready_replicas: status.and_then(|s| s.ready_replicas).unwrap_or(0) as u32,
                cpu_request,
                memory_request,
                status: if status.and_then(|s| s.ready_replicas).unwrap_or(0)
                    >= d.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1)
                {
                    "Ready".to_string()
                } else {
                    "Progressing".to_string()
                },
            });
        }
    }

    // StatefulSets
    if let Ok(stss) = sts_result {
        for s in &stss.items {
            let status = s.status.as_ref();
            let containers = s
                .spec
                .as_ref()
                .and_then(|sp| sp.template.spec.as_ref())
                .map(|spec| spec.containers.as_slice())
                .unwrap_or(&[]);
            let (cpu_request, memory_request) = summarize_container_requests(containers);
            results.push(WorkloadResponse {
                name: s.metadata.name.clone().unwrap_or_default(),
                namespace: s.metadata.namespace.clone().unwrap_or_default(),
                workload_type: "StatefulSet".to_string(),
                replicas: s.spec.as_ref().and_then(|sp| sp.replicas).unwrap_or(1) as u32,
                ready_replicas: status.and_then(|st| st.ready_replicas).unwrap_or(0) as u32,
                cpu_request,
                memory_request,
                status: if status.and_then(|st| st.ready_replicas).unwrap_or(0)
                    >= s.spec.as_ref().and_then(|sp| sp.replicas).unwrap_or(1)
                {
                    "Ready".to_string()
                } else {
                    "Progressing".to_string()
                },
            });
        }
    }

    // DaemonSets
    if let Ok(dss) = ds_result {
        for d in &dss.items {
            let status = d.status.as_ref();
            let desired = status.map(|s| s.desired_number_scheduled).unwrap_or(0);
            let ready = status.map(|s| s.number_ready).unwrap_or(0);
            let containers = d
                .spec
                .as_ref()
                .and_then(|sp| sp.template.spec.as_ref())
                .map(|spec| spec.containers.as_slice())
                .unwrap_or(&[]);
            let (cpu_request, memory_request) = summarize_container_requests(containers);
            results.push(WorkloadResponse {
                name: d.metadata.name.clone().unwrap_or_default(),
                namespace: d.metadata.namespace.clone().unwrap_or_default(),
                workload_type: "DaemonSet".to_string(),
                replicas: desired as u32,
                ready_replicas: ready as u32,
                cpu_request,
                memory_request,
                status: if ready >= desired {
                    "Ready".to_string()
                } else {
                    "Progressing".to_string()
                },
            });
        }
    }

    Json(results)
}

#[cfg(feature = "web")]
fn summarize_container_requests(containers: &[Container]) -> (String, String) {
    let mut cpu_millis = 0u64;
    let mut memory_bytes = 0u64;

    for container in containers {
        if let Some(requests) = container
            .resources
            .as_ref()
            .and_then(|r| r.requests.as_ref())
        {
            if let Some(cpu) = requests.get("cpu") {
                cpu_millis += parse_cpu_to_millis(cpu.0.as_str());
            }
            if let Some(memory) = requests.get("memory") {
                memory_bytes += parse_memory_to_bytes(memory.0.as_str());
            }
        }
    }

    (
        format_cpu_millis(cpu_millis),
        format_memory_bytes(memory_bytes),
    )
}

#[cfg(feature = "web")]
fn parse_cpu_to_millis(value: &str) -> u64 {
    if let Some(millis) = value.strip_suffix('m') {
        millis.parse::<u64>().unwrap_or(0)
    } else {
        let cores = value.parse::<f64>().unwrap_or(0.0);
        (cores * 1000.0).round() as u64
    }
}

#[cfg(feature = "web")]
fn parse_memory_to_bytes(value: &str) -> u64 {
    let units = [
        ("Ki", 1024u64),
        ("Mi", 1024u64.pow(2)),
        ("Gi", 1024u64.pow(3)),
        ("Ti", 1024u64.pow(4)),
        ("K", 1000u64),
        ("M", 1000u64.pow(2)),
        ("G", 1000u64.pow(3)),
        ("T", 1000u64.pow(4)),
    ];

    for (suffix, multiplier) in units {
        if let Some(raw) = value.strip_suffix(suffix) {
            let qty = raw.parse::<f64>().unwrap_or(0.0);
            return (qty * multiplier as f64).round() as u64;
        }
    }

    value.parse::<u64>().unwrap_or(0)
}

#[cfg(feature = "web")]
fn format_cpu_millis(cpu_millis: u64) -> String {
    if cpu_millis == 0 {
        return "-".to_string();
    }
    if cpu_millis % 1000 == 0 {
        return (cpu_millis / 1000).to_string();
    }
    format!("{}m", cpu_millis)
}

#[cfg(feature = "web")]
fn format_memory_bytes(memory_bytes: u64) -> String {
    if memory_bytes == 0 {
        return "-".to_string();
    }

    let gib = 1024u64.pow(3);
    let mib = 1024u64.pow(2);
    let kib = 1024u64;

    if memory_bytes % gib == 0 {
        return format!("{}Gi", memory_bytes / gib);
    }
    if memory_bytes % mib == 0 {
        return format!("{}Mi", memory_bytes / mib);
    }
    if memory_bytes >= gib {
        return format!("{:.1}Gi", memory_bytes as f64 / gib as f64);
    }
    if memory_bytes >= mib {
        return format!("{}Mi", (memory_bytes as f64 / mib as f64).round() as u64);
    }
    format!("{}Ki", (memory_bytes / kib).max(1))
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;
    use k8s_openapi::{
        api::core::v1::{Container, ResourceRequirements},
        apimachinery::pkg::api::resource::Quantity,
    };
    use std::collections::BTreeMap;

    #[test]
    fn summarize_requests_aggregates_cpu_and_memory() {
        let mut requests_a = BTreeMap::new();
        requests_a.insert("cpu".to_string(), Quantity("250m".to_string()));
        requests_a.insert("memory".to_string(), Quantity("512Mi".to_string()));

        let mut requests_b = BTreeMap::new();
        requests_b.insert("cpu".to_string(), Quantity("1".to_string()));
        requests_b.insert("memory".to_string(), Quantity("1Gi".to_string()));

        let containers = vec![
            Container {
                name: "a".to_string(),
                resources: Some(ResourceRequirements {
                    requests: Some(requests_a),
                    ..Default::default()
                }),
                ..Default::default()
            },
            Container {
                name: "b".to_string(),
                resources: Some(ResourceRequirements {
                    requests: Some(requests_b),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];

        let (cpu, memory) = summarize_container_requests(&containers);
        assert_eq!(cpu, "1250m");
        assert_eq!(memory, "1536Mi");
    }

    #[test]
    fn summarize_requests_handles_empty_requests() {
        let containers = vec![Container {
            name: "empty".to_string(),
            ..Default::default()
        }];

        let (cpu, memory) = summarize_container_requests(&containers);
        assert_eq!(cpu, "-");
        assert_eq!(memory, "-");
    }
}
