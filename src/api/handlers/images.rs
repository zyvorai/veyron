// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Golden image / CDI DataVolume catalog and import.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageCatalogEntry {
    pub name: String,
    pub namespace: String,
    pub kind: String,
    pub source_type: String,
    pub status: String,
    pub capacity: String,
    pub storage_class: String,
    pub age: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageCatalogResponse {
    pub vmrogue_context: super::feature_context::VmrogueFeatureContext,
    pub images: Vec<ImageCatalogEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportDataVolumeRequest {
    pub name: String,
    pub namespace: String,
    pub url: Option<String>,
    pub registry: Option<String>,
    #[serde(default)]
    pub storage_class: Option<String>,
    #[serde(default = "default_import_size")]
    pub size: String,
}

fn default_import_size() -> String {
    "20Gi".to_string()
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/images/catalog", get(list_image_catalog))
        .route("/images/import", post(import_data_volume))
        .with_state(state)
}

#[cfg(feature = "web")]
fn image_context() -> super::feature_context::VmrogueFeatureContext {
    super::feature_context::VmrogueFeatureContext {
        data_source: "cdi_datavolumes_and_pvcs".to_string(),
        scope: "CDI DataVolumes and image-tagged PVCs in namespace scope.".to_string(),
        limitations: "POST /images/import creates CDI DataVolume CRs. Packer builds remain in docs/WINDOWS_PACKER_GITOPS_PIPELINE.md.".to_string(),
    }
}

#[cfg(feature = "web")]
async fn list_image_catalog(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<ImageCatalogResponse> {
    use k8s_openapi::api::core::v1::PersistentVolumeClaim;
    use kube::api::{Api, ApiResource, DynamicObject, ListParams};

    let s = state.read().await;
    let client = s.client().client();
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);

    let mut images = Vec::new();

    let ar = ApiResource {
        group: "cdi.kubevirt.io".to_string(),
        version: "v1beta1".to_string(),
        api_version: "cdi.kubevirt.io/v1beta1".to_string(),
        kind: "DataVolume".to_string(),
        plural: "datavolumes".to_string(),
    };
    let dv_api: Api<DynamicObject> = if namespace_scope::is_all_namespaces(&scope) {
        Api::all_with(client.clone(), &ar)
    } else {
        Api::namespaced_with(client.clone(), &scope, &ar)
    };

    if let Ok(dvs) = dv_api.list(&ListParams::default().limit(100)).await {
        for dv in dvs.items {
            let Some(name) = dv.metadata.name else { continue };
            let ns = dv.metadata.namespace.unwrap_or_else(|| scope.clone());
            let phase = dv
                .data
                .get("status")
                .and_then(|st| st.get("phase"))
                .and_then(|p| p.as_str())
                .unwrap_or("Unknown");
            let source_type = dv
                .data
                .get("spec")
                .and_then(|sp| sp.get("source"))
                .and_then(|src| {
                    if src.get("http").is_some() {
                        Some("http")
                    } else if src.get("registry").is_some() {
                        Some("registry")
                    } else if src.get("pvc").is_some() {
                        Some("pvc")
                    } else {
                        Some("other")
                    }
                })
                .unwrap_or("unknown");
            images.push(ImageCatalogEntry {
                name: name.clone(),
                namespace: ns,
                kind: "DataVolume".to_string(),
                source_type: source_type.to_string(),
                status: phase.to_string(),
                capacity: String::new(),
                storage_class: String::new(),
                age: String::new(),
                tags: vec!["cdi".to_string()],
            });
        }
    }

    let pvc_api: Api<PersistentVolumeClaim> = if namespace_scope::is_all_namespaces(&scope) {
        Api::all(client)
    } else {
        Api::namespaced(client, &scope)
    };

    if let Ok(pvcs) = pvc_api
        .list(
            &ListParams::default()
                .labels("vmrogue.io/type=golden-image")
                .limit(100),
        )
        .await
    {
        for pvc in pvcs.items {
            let Some(name) = pvc.metadata.name else { continue };
            let ns = pvc.metadata.namespace.unwrap_or_else(|| scope.clone());
            let capacity = pvc
                .status
                .as_ref()
                .and_then(|st| st.capacity.as_ref())
                .and_then(|c| c.get("storage"))
                .map(|q| q.0.clone())
                .unwrap_or_default();
            let sc = pvc
                .spec
                .as_ref()
                .and_then(|sp| sp.storage_class_name.clone())
                .unwrap_or_default();
            images.push(ImageCatalogEntry {
                name,
                namespace: ns,
                kind: "PVC".to_string(),
                source_type: "golden-image".to_string(),
                status: pvc
                    .status
                    .as_ref()
                    .and_then(|st| st.phase.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
                capacity,
                storage_class: sc,
                age: String::new(),
                tags: vec!["golden-image".to_string()],
            });
        }
    }

    Json(ImageCatalogResponse {
        vmrogue_context: image_context(),
        images,
    })
}

#[cfg(feature = "web")]
async fn import_data_volume(
    State(state): State<SharedState>,
    Json(req): Json<ImportDataVolumeRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    use kube::api::{ApiResource, DynamicObject, PostParams};

    if req.name.is_empty() || req.namespace.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "name and namespace required" })),
        ));
    }
    let source = if let Some(url) = req.url.as_ref().filter(|u| !u.is_empty()) {
        serde_json::json!({ "http": { "url": url } })
    } else if let Some(reg) = req.registry.as_ref().filter(|r| !r.is_empty()) {
        serde_json::json!({ "registry": { "url": reg } })
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "url or registry source required" })),
        ));
    };

    let mut pvc_spec = serde_json::json!({
        "accessModes": ["ReadWriteOnce"],
        "resources": { "requests": { "storage": req.size } }
    });
    if let Some(sc) = req.storage_class.as_ref().filter(|s| !s.is_empty()) {
        pvc_spec["storageClassName"] = serde_json::Value::String(sc.clone());
    }

    let dv = serde_json::json!({
        "apiVersion": "cdi.kubevirt.io/v1beta1",
        "kind": "DataVolume",
        "metadata": {
            "name": req.name,
            "namespace": req.namespace,
            "labels": { "vmrogue.io/type": "imported-image" }
        },
        "spec": {
            "source": source,
            "pvc": pvc_spec
        }
    });

    let ar = ApiResource {
        group: "cdi.kubevirt.io".to_string(),
        version: "v1beta1".to_string(),
        api_version: "cdi.kubevirt.io/v1beta1".to_string(),
        kind: "DataVolume".to_string(),
        plural: "datavolumes".to_string(),
    };
    let s = state.read().await;
    let client = s.client().client();
    let api: kube::Api<DynamicObject> =
        kube::Api::namespaced_with(client, &req.namespace, &ar);

    let obj: DynamicObject = serde_json::from_value(dv).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    match api.create(&PostParams::default(), &obj).await {
        Ok(created) => Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({
                "status": "created",
                "name": created.metadata.name,
                "namespace": created.metadata.namespace,
                "vmrogue_context": image_context(),
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}
