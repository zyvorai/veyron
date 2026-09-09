// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Golden image / CDI DataVolume catalog and import.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
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
    pub veyron_context: super::feature_context::VmrogueFeatureContext,
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

/// Create an upload-source DataVolume and mint a CDI upload token, so a local
/// ISO/QCOW2 can be pushed straight into the cluster (the `virtctl image-upload`
/// flow, without requiring virtctl).
#[derive(Debug, Clone, Deserialize)]
pub struct UploadDataVolumeRequest {
    pub name: String,
    pub namespace: String,
    #[serde(default)]
    pub storage_class: Option<String>,
    #[serde(default = "default_import_size")]
    pub size: String,
    /// `filesystem` (default, for ISOs) or `block`.
    #[serde(default)]
    pub volume_mode: Option<String>,
}

/// Promote a completed DataVolume to an immutable, versioned golden image and
/// point a `DataSource` at it.
///
/// The DataSource is the stable catalog handle: VMs reference it via `sourceRef`,
/// so publishing a new version repoints every future clone without touching a
/// single VM.
#[derive(Debug, Clone, Deserialize)]
pub struct PublishImageRequest {
    /// The source DataVolume/PVC to publish (must be `Succeeded`/`Bound`).
    pub name: String,
    pub namespace: String,
    /// Catalog name clients ask for, e.g. `windows-server-2022`.
    pub data_source: String,
    /// Optional version tag recorded on the image, e.g. `2026-07`.
    #[serde(default)]
    pub version: Option<String>,
    /// Skip the readiness check (for republishing an already-known-good PVC).
    #[serde(default)]
    pub force: bool,
    /// Cluster instancetype a VM should default to when it infers from this image
    /// (e.g. `windows-medium`). Stamped as the `default-instancetype` label.
    #[serde(default)]
    pub instancetype: Option<String>,
    /// Cluster preference likewise (e.g. `windows-server`).
    #[serde(default)]
    pub preference: Option<String>,
    /// OS family hint (`windows`/`linux`) used to pick sensible instancetype/preference
    /// defaults when the two above are not given.
    #[serde(default)]
    pub os: Option<String>,
}

/// Default (instancetype, preference) for an OS family, so a published image is
/// self-describing even when the caller does not spell the pair out.
#[cfg(feature = "web")]
fn default_inference_pair(os: Option<&str>) -> Option<(&'static str, &'static str)> {
    match os.map(|s| s.to_ascii_lowercase()) {
        Some(ref o) if o.contains("windows") => Some(("windows-medium", "windows-server")),
        Some(ref o) if o.contains("linux") || o.contains("ubuntu") || o.contains("rhel") => {
            Some(("u1.medium", "linux"))
        }
        _ => None,
    }
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/images/catalog", get(list_image_catalog))
        .route("/images/import", post(import_data_volume))
        .route("/images/upload", post(upload_data_volume))
        .route("/images/publish", post(publish_image))
        .route("/images/datasources", get(list_data_sources))
        .route("/images/:ns/:name", delete(delete_image))
        .with_state(state)
}

#[cfg(feature = "web")]
fn datavolume_api_resource() -> kube::api::ApiResource {
    kube::api::ApiResource {
        group: "cdi.kubevirt.io".to_string(),
        version: "v1beta1".to_string(),
        api_version: "cdi.kubevirt.io/v1beta1".to_string(),
        kind: "DataVolume".to_string(),
        plural: "datavolumes".to_string(),
    }
}

#[cfg(feature = "web")]
fn datasource_api_resource() -> kube::api::ApiResource {
    kube::api::ApiResource {
        group: "cdi.kubevirt.io".to_string(),
        version: "v1beta1".to_string(),
        api_version: "cdi.kubevirt.io/v1beta1".to_string(),
        kind: "DataSource".to_string(),
        plural: "datasources".to_string(),
    }
}

#[cfg(feature = "web")]
type ImageResult =
    Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)>;

#[cfg(feature = "web")]
fn bad_request(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": msg })),
    )
}

#[cfg(feature = "web")]
fn upstream_error(e: impl std::fmt::Display) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": e.to_string() })),
    )
}

/// `POST /api/v1/images/upload` — create an upload-target DataVolume and return a
/// CDI upload token. The caller then PUTs the image bytes to the returned proxy URL.
#[cfg(feature = "web")]
async fn upload_data_volume(
    State(state): State<SharedState>,
    Json(req): Json<UploadDataVolumeRequest>,
) -> ImageResult {
    use kube::api::{DynamicObject, PostParams};

    if req.name.is_empty() || req.namespace.is_empty() {
        return Err(bad_request("name and namespace required"));
    }

    let mut storage = serde_json::json!({
        "resources": { "requests": { "storage": req.size } }
    });
    if let Some(sc) = req.storage_class.as_ref().filter(|s| !s.is_empty()) {
        storage["storageClassName"] = serde_json::Value::String(sc.clone());
    }
    if let Some(vm) = req.volume_mode.as_ref().filter(|v| !v.is_empty()) {
        // ISOs want Filesystem; raw disk images are usually Block.
        let mode = if vm.eq_ignore_ascii_case("block") {
            "Block"
        } else {
            "Filesystem"
        };
        storage["volumeMode"] = serde_json::Value::String(mode.to_string());
    }

    let dv = serde_json::json!({
        "apiVersion": "cdi.kubevirt.io/v1beta1",
        "kind": "DataVolume",
        "metadata": {
            "name": req.name,
            "namespace": req.namespace,
            "labels": {
                "veyron.io/type": "imported-image",
                "veyron.io/image-phase": "uploading"
            }
        },
        // `upload: {}` puts the DataVolume in UploadReady and opens the proxy path.
        "spec": { "source": { "upload": {} }, "storage": storage }
    });

    let client = { state.read().await.client().client() };
    let ar = datavolume_api_resource();
    let dv_api: kube::Api<DynamicObject> =
        kube::Api::namespaced_with(client.clone(), &req.namespace, &ar);

    let obj: DynamicObject = serde_json::from_value(dv).map_err(|e| bad_request(&e.to_string()))?;

    // Idempotent: reuse an existing DataVolume of the same name rather than failing
    // a retried upload.
    if dv_api.get_opt(&req.name).await.ok().flatten().is_none() {
        dv_api
            .create(&PostParams::default(), &obj)
            .await
            .map_err(upstream_error)?;
    }

    // Mint a short-lived token scoped to this PVC.
    let token_ar = kube::api::ApiResource {
        group: "upload.cdi.kubevirt.io".to_string(),
        version: "v1beta1".to_string(),
        api_version: "upload.cdi.kubevirt.io/v1beta1".to_string(),
        kind: "UploadTokenRequest".to_string(),
        plural: "uploadtokenrequests".to_string(),
    };
    let token_api: kube::Api<DynamicObject> =
        kube::Api::namespaced_with(client, &req.namespace, &token_ar);
    let token_req: DynamicObject = serde_json::from_value(serde_json::json!({
        "apiVersion": "upload.cdi.kubevirt.io/v1beta1",
        "kind": "UploadTokenRequest",
        "metadata": { "name": req.name, "namespace": req.namespace },
        "spec": { "pvcName": req.name }
    }))
    .map_err(|e| bad_request(&e.to_string()))?;

    let token = token_api
        .create(&PostParams::default(), &token_req)
        .await
        .ok()
        .and_then(|o| {
            o.data
                .get("status")
                .and_then(|s| s.get("token"))
                .and_then(|t| t.as_str())
                .map(String::from)
        });

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "upload_ready",
            "name": req.name,
            "namespace": req.namespace,
            "token": token,
            // The proxy is cluster-internal; expose it (Ingress/port-forward) to push bytes.
            "upload_proxy": "https://cdi-uploadproxy.cdi.svc/v1beta1/upload",
            "hint": "PUT the image to <upload_proxy> with header 'Authorization: Bearer <token>'. \
                    Equivalent: virtctl image-upload dv <name> -n <namespace> --image-path <file>",
            "veyron_context": image_context(),
        })),
    ))
}

/// `POST /api/v1/images/publish` — mark a finished DataVolume as a golden image and
/// (re)point a `DataSource` at it.
#[cfg(feature = "web")]
async fn publish_image(
    State(state): State<SharedState>,
    Json(req): Json<PublishImageRequest>,
) -> ImageResult {
    use kube::api::{DynamicObject, Patch, PatchParams};

    if req.name.is_empty() || req.namespace.is_empty() || req.data_source.is_empty() {
        return Err(bad_request("name, namespace and data_source required"));
    }

    let client = { state.read().await.client().client() };
    let dv_ar = datavolume_api_resource();
    let dv_api: kube::Api<DynamicObject> =
        kube::Api::namespaced_with(client.clone(), &req.namespace, &dv_ar);

    // Publishing a half-imported disk would hand every cloned VM a corrupt image.
    if !req.force {
        let dv = dv_api
            .get_opt(&req.name)
            .await
            .map_err(upstream_error)?
            .ok_or_else(|| bad_request(&format!("DataVolume '{}' not found", req.name)))?;
        let phase = dv
            .data
            .get("status")
            .and_then(|s| s.get("phase"))
            .and_then(|p| p.as_str())
            .unwrap_or("Unknown");
        if phase != "Succeeded" {
            return Err(bad_request(&format!(
                "DataVolume '{}' is '{}', not 'Succeeded' — publishing an incomplete image \
                 would give every cloned VM a corrupt disk (pass force=true to override)",
                req.name, phase
            )));
        }
    }

    // Tag the underlying disk so the catalog lists it and nothing reaps it.
    let mut labels = serde_json::json!({
        "veyron.io/type": "golden-image",
        "veyron.io/image-phase": "published",
    });
    if let Some(v) = req.version.as_ref().filter(|v| !v.is_empty()) {
        labels["veyron.io/image-version"] = serde_json::Value::String(v.clone());
    }
    // Inference labels: a VM whose disk infers-from-volume this image auto-sizes to
    // these. Explicit params win; otherwise fall back to OS-family defaults.
    let defaults = default_inference_pair(req.os.as_deref());
    let instancetype = req
        .instancetype
        .clone()
        .or_else(|| defaults.map(|(i, _)| i.to_string()));
    let preference = req
        .preference
        .clone()
        .or_else(|| defaults.map(|(_, p)| p.to_string()));
    if let Some(it) = instancetype.as_ref().filter(|s| !s.is_empty()) {
        labels["instancetype.kubevirt.io/default-instancetype"] =
            serde_json::Value::String(it.clone());
    }
    if let Some(pr) = preference.as_ref().filter(|s| !s.is_empty()) {
        labels["instancetype.kubevirt.io/default-preference"] =
            serde_json::Value::String(pr.clone());
    }
    let label_patch = serde_json::json!({ "metadata": { "labels": labels } });

    let pp = PatchParams::apply("veyron-image-publish").force();
    let _ = dv_api
        .patch(
            &req.name,
            &PatchParams::default(),
            &Patch::Merge(&label_patch),
        )
        .await;
    let pvc_api: kube::Api<k8s_openapi::api::core::v1::PersistentVolumeClaim> =
        kube::Api::namespaced(client.clone(), &req.namespace);
    let _ = pvc_api
        .patch(
            &req.name,
            &PatchParams::default(),
            &Patch::Merge(&label_patch),
        )
        .await;

    // The DataSource is the stable pointer; server-side apply so republishing a new
    // version simply repoints it.
    let ds_ar = datasource_api_resource();
    let ds_api: kube::Api<DynamicObject> =
        kube::Api::namespaced_with(client, &req.namespace, &ds_ar);
    // KubeVirt reads default-instancetype/preference off the DataSource when a VM's
    // disk infers from a sourceRef volume, so the labels must live here too.
    let mut ds_labels = serde_json::json!({
        "veyron.io/type": "golden-image",
        "veyron.io/managed-by": "veyron",
    });
    if let Some(it) = instancetype.as_ref().filter(|s| !s.is_empty()) {
        ds_labels["instancetype.kubevirt.io/default-instancetype"] =
            serde_json::Value::String(it.clone());
    }
    if let Some(pr) = preference.as_ref().filter(|s| !s.is_empty()) {
        ds_labels["instancetype.kubevirt.io/default-preference"] =
            serde_json::Value::String(pr.clone());
    }
    let ds: DynamicObject = serde_json::from_value(serde_json::json!({
        "apiVersion": "cdi.kubevirt.io/v1beta1",
        "kind": "DataSource",
        "metadata": {
            "name": req.data_source,
            "namespace": req.namespace,
            "labels": ds_labels
        },
        "spec": { "source": { "pvc": { "namespace": req.namespace, "name": req.name } } }
    }))
    .map_err(|e| bad_request(&e.to_string()))?;

    ds_api
        .patch(&req.data_source, &pp, &Patch::Apply(&ds))
        .await
        .map_err(upstream_error)?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "published",
            "data_source": req.data_source,
            "namespace": req.namespace,
            "backing_pvc": req.name,
            "version": req.version,
            "hint": format!(
                "create VMs from this image with: POST /api/v1/vms {{\"image\":{{\"name\":\"{}\",\"namespace\":\"{}\"}}}}",
                req.data_source, req.namespace
            ),
            "veyron_context": image_context(),
        })),
    ))
}

/// `GET /api/v1/images/datasources` — the golden-image catalog pointers.
#[cfg(feature = "web")]
async fn list_data_sources(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<serde_json::Value> {
    use kube::api::DynamicObject;

    let s = state.read().await;
    let client = s.client().client();
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let ar = datasource_api_resource();
    let api: kube::Api<DynamicObject> = if namespace_scope::is_all_namespaces(&scope) {
        kube::Api::all_with(client, &ar)
    } else {
        kube::Api::namespaced_with(client, &scope, &ar)
    };

    let items = api
        .list(&Default::default())
        .await
        .inspect_err(|e| {
            // Silently defaulting to an empty list makes a real failure
            // (RBAC, API discovery) indistinguishable from "nothing
            // published yet" — log it so it's diagnosable.
            log::error!("list DataSources (scope={scope}) failed: {e}");
        })
        .map(|l| {
            l.items
                .iter()
                .map(|o| {
                    let src = o
                        .data
                        .get("spec")
                        .and_then(|s| s.get("source"))
                        .and_then(|s| s.get("pvc"));
                    // A DataSource is only usable once its source PVC is ready.
                    let ready = o
                        .data
                        .get("status")
                        .and_then(|s| s.get("conditions"))
                        .and_then(|c| c.as_array())
                        .map(|conds| {
                            conds.iter().any(|c| {
                                c.get("type").and_then(|t| t.as_str()) == Some("Ready")
                                    && c.get("status").and_then(|s| s.as_str()) == Some("True")
                            })
                        })
                        .unwrap_or(false);
                    serde_json::json!({
                        "name": o.metadata.name,
                        "namespace": o.metadata.namespace,
                        "ready": ready,
                        "source_pvc": src.and_then(|p| p.get("name")),
                        "source_namespace": src.and_then(|p| p.get("namespace")),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Json(serde_json::json!({
        "data_sources": items,
        "veyron_context": image_context(),
    }))
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
            let Some(name) = dv.metadata.name else {
                continue;
            };
            let ns = dv.metadata.namespace.unwrap_or_else(|| scope.clone());
            let cdi_phase = dv
                .data
                .get("status")
                .and_then(|st| st.get("phase"))
                .and_then(|p| p.as_str())
                .unwrap_or("Unknown");
            let image_phase = dv
                .metadata
                .labels
                .as_ref()
                .and_then(|l| l.get("veyron.io/image-phase"))
                .cloned()
                .unwrap_or_else(|| {
                    if cdi_phase == "Succeeded" {
                        "ready".to_string()
                    } else {
                        "building".to_string()
                    }
                });
            let phase = cdi_phase;
            let source_type = dv
                .data
                .get("spec")
                .and_then(|sp| sp.get("source"))
                .map(|src| {
                    if src.get("http").is_some() {
                        "http"
                    } else if src.get("registry").is_some() {
                        "registry"
                    } else if src.get("pvc").is_some() {
                        "pvc"
                    } else {
                        "other"
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
                tags: vec!["cdi".to_string(), format!("image-phase:{image_phase}")],
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
                .labels("veyron.io/type=golden-image")
                .limit(100),
        )
        .await
    {
        for pvc in pvcs.items {
            let Some(name) = pvc.metadata.name else {
                continue;
            };
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
        veyron_context: image_context(),
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
            "labels": {
                "veyron.io/type": "imported-image",
                "veyron.io/image-phase": "building"
            }
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
    let api: kube::Api<DynamicObject> = kube::Api::namespaced_with(client, &req.namespace, &ar);

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
                "veyron_context": image_context(),
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

/// `DELETE /api/v1/images/:ns/:name` — remove a catalog PVC / DataVolume / DataSource by name.
#[cfg(feature = "web")]
async fn delete_image(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> ImageResult {
    use k8s_openapi::api::core::v1::PersistentVolumeClaim;
    use kube::api::{Api, DeleteParams, DynamicObject};

    if ns.is_empty() || name.is_empty() {
        return Err(bad_request("namespace and name required"));
    }
    let client = { state.read().await.client().client() };
    let mut deleted = Vec::new();
    let mut errors = Vec::new();

    let dv_api: Api<DynamicObject> =
        Api::namespaced_with(client.clone(), &ns, &datavolume_api_resource());
    match dv_api.delete(&name, &DeleteParams::default()).await {
        Ok(_) => deleted.push("DataVolume"),
        Err(e) => {
            let msg = e.to_string();
            if !msg.contains("NotFound") && !msg.contains("not found") {
                errors.push(format!("DataVolume: {msg}"));
            }
        }
    }

    let ds_api: Api<DynamicObject> =
        Api::namespaced_with(client.clone(), &ns, &datasource_api_resource());
    match ds_api.delete(&name, &DeleteParams::default()).await {
        Ok(_) => deleted.push("DataSource"),
        Err(e) => {
            let msg = e.to_string();
            if !msg.contains("NotFound") && !msg.contains("not found") {
                errors.push(format!("DataSource: {msg}"));
            }
        }
    }

    let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client, &ns);
    match pvc_api.delete(&name, &DeleteParams::default()).await {
        Ok(_) => deleted.push("PVC"),
        Err(e) => {
            let msg = e.to_string();
            if !msg.contains("NotFound") && !msg.contains("not found") {
                errors.push(format!("PVC: {msg}"));
            }
        }
    }

    if deleted.is_empty() && !errors.is_empty() {
        return Err(upstream_error(errors.join("; ")));
    }
    if deleted.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("image '{ns}/{name}' not found") })),
        ));
    }
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "deleted",
            "name": name,
            "namespace": ns,
            "deleted": deleted,
            "warnings": errors,
        })),
    ))
}
