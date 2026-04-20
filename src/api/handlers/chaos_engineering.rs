#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use kube::{
    Api,
    api::{ApiResource, DynamicObject, ListParams},
};

/// Chaos experiment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosExperiment {
    pub id: String,
    pub name: String,
    pub experiment_type: String,
    pub target: String,
    pub namespace: String,
    pub status: String,
    pub duration: String,
    pub results: Option<ChaosResults>,
    pub created_at: String,
}

/// Chaos experiment results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosResults {
    pub success: bool,
    pub affected_resources: u32,
    pub recovery_time_seconds: Option<u64>,
    pub observations: Vec<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/chaos/experiments", get(list_experiments))
        .with_state(state)
}

/// List chaos experiments by checking for Chaos Mesh or LitmusChaos CRDs.
#[cfg(feature = "web")]
async fn list_experiments(State(state): State<SharedState>) -> Json<Vec<ChaosExperiment>> {
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;

    let s = state.read().await;
    let client = s.client().client();
    let api: Api<CustomResourceDefinition> = Api::all(client.clone());
    let crds = match api.list(&ListParams::default()).await {
        Ok(list) => list.items,
        Err(_) => return Json(vec![]),
    };

    let chaos_crds: Vec<_> = crds.into_iter().filter(is_chaos_crd).collect();

    let mut experiments = Vec::new();
    for crd in chaos_crds {
        let spec = &crd.spec;
        let version = preferred_chaos_version(&spec.versions);
        let resource = ApiResource {
            group: spec.group.clone(),
            version: version.clone(),
            api_version: format!("{}/{}", spec.group, version),
            kind: spec.names.kind.clone(),
            plural: spec.names.plural.clone(),
        };

        let api: Api<DynamicObject> = Api::all_with(client.clone(), &resource);
        let objects = match api.list(&ListParams::default()).await {
            Ok(list) => list.items,
            Err(_) => continue,
        };

        for obj in objects {
            experiments.push(dynamic_object_to_experiment(&obj, &spec.names.kind));
        }
    }

    Json(experiments)
}

#[cfg(feature = "web")]
fn is_chaos_crd(
    crd: &k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
) -> bool {
    let name = crd.metadata.name.as_deref().unwrap_or("").to_lowercase();
    let group = crd.spec.group.to_lowercase();
    let kind = crd.spec.names.kind.to_lowercase();

    let is_chaos_family =
        group.contains("chaos-mesh") || group.contains("litmuschaos") || name.contains("chaos");
    let is_experiment_like =
        kind.contains("chaos") || kind.contains("experiment") || kind.contains("engine");

    is_chaos_family && is_experiment_like
}

#[cfg(feature = "web")]
fn preferred_chaos_version(
    versions: &[k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinitionVersion],
) -> String {
    versions
        .iter()
        .find(|v| v.storage)
        .or_else(|| versions.iter().find(|v| v.served))
        .or_else(|| versions.first())
        .map(|v| v.name.clone())
        .unwrap_or_default()
}

#[cfg(feature = "web")]
fn dynamic_object_to_experiment(obj: &DynamicObject, kind: &str) -> ChaosExperiment {
    let status = obj
        .data
        .get("status")
        .and_then(extract_status)
        .unwrap_or_else(|| "Unknown".to_string());
    let target = obj
        .data
        .get("spec")
        .and_then(extract_target)
        .unwrap_or_else(|| obj.metadata.name.clone().unwrap_or_default());
    let duration = obj
        .data
        .get("spec")
        .and_then(|spec| spec.get("duration"))
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string();

    ChaosExperiment {
        id: obj.metadata.uid.clone().unwrap_or_default(),
        name: obj.metadata.name.clone().unwrap_or_default(),
        experiment_type: kind.to_string(),
        target,
        namespace: obj.metadata.namespace.clone().unwrap_or_default(),
        status,
        duration,
        results: None,
        created_at: obj
            .metadata
            .creation_timestamp
            .as_ref()
            .map(|t| t.0.to_rfc3339())
            .unwrap_or_default(),
    }
}

fn extract_status(status: &Value) -> Option<String> {
    for key in ["phase", "experimentStatus", "state"] {
        if let Some(value) = status.get(key).and_then(|v| v.as_str()) {
            return Some(value.to_string());
        }
    }

    status
        .get("conditions")
        .and_then(|v| v.as_array())
        .and_then(|conditions| conditions.iter().find(|c| c.get("status").is_some()))
        .and_then(|c| c.get("type").or_else(|| c.get("status")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn extract_target(spec: &Value) -> Option<String> {
    if let Some(selector) = spec.get("selector") {
        if let Some(namespaces) = selector.get("namespaces").and_then(|v| v.as_array()) {
            let joined = namespaces
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(",");
            if !joined.is_empty() {
                return Some(joined);
            }
        }
        if let Some(label_selectors) = selector.get("labelSelectors").and_then(|v| v.as_object()) {
            let joined = label_selectors
                .iter()
                .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or("*")))
                .collect::<Vec<_>>()
                .join(",");
            if !joined.is_empty() {
                return Some(joined);
            }
        }
    }

    if let Some(mode) = spec.get("mode").and_then(|v| v.as_str()) {
        return Some(mode.to_string());
    }

    spec.get("action")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_status_prefers_phase() {
        assert_eq!(
            extract_status(&json!({"phase":"Running"})),
            Some("Running".to_string())
        );
    }

    #[test]
    fn extract_target_reads_namespaces_selector() {
        assert_eq!(
            extract_target(&json!({"selector":{"namespaces":["default","prod"]}})),
            Some("default,prod".to_string())
        );
    }
}
