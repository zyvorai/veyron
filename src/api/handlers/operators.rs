// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
use super::namespace_scope::{self, DashboardNamespaceQuery};
#[cfg(feature = "web")]
use k8s_openapi::{
    api::apps::v1::Deployment, api::core::v1::Node, api::storage::v1::StorageClass,
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};

/// Operator response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorResponse {
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub status: String,
    pub managed_resources: Vec<String>,
    pub installed_at: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/operators", get(list_operators))
        .route("/platform/versions", get(platform_versions))
        .route("/platform/capabilities", get(platform_capabilities))
        .route("/platform/upgrade", axum::routing::post(platform_upgrade))
        .with_state(state)
}

/// Report which Day-2 operations the **target cluster** actually supports, based
/// on KubeVirt feature gates and installed components. Prevents version-gated ops
/// (live hotplug, storage-class migration) from silently failing at a customer
/// site — the dashboard/CLI/preflight can gate on this.
#[cfg(feature = "web")]
async fn platform_capabilities(State(state): State<SharedState>) -> Json<serde_json::Value> {
    use kube::api::{Api, ApiResource, DynamicObject};
    use kube::core::GroupVersionKind;

    let client = { state.read().await.kube_client.client().clone() };

    // KubeVirt CR → feature gates.
    let kv_ar = ApiResource::from_gvk(&GroupVersionKind::gvk("kubevirt.io", "v1", "KubeVirt"));
    let kv: Api<DynamicObject> = Api::namespaced_with(client.clone(), "kubevirt", &kv_ar);
    let kv_obj = kv.get_opt("kubevirt").await.ok().flatten();
    let gates: Vec<String> = kv_obj
        .as_ref()
        .and_then(|o| o.data.get("spec"))
        .and_then(|s| s.get("configuration"))
        .and_then(|c| c.get("developerConfiguration"))
        .and_then(|d| d.get("featureGates"))
        .and_then(|g| g.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let has = |g: &str| gates.iter().any(|x| x == g);
    let kv_version = kv_obj
        .as_ref()
        .and_then(|o| o.data.get("status"))
        .and_then(|s| {
            s.get("observedKubeVirtVersion")
                .or_else(|| s.get("operatorVersion"))
        })
        .and_then(|v| v.as_str())
        .map(String::from);

    // KubeVirt backend storage for persistent TPM/EFI state (Windows 11, BitLocker).
    let vm_state_storage_class = kv_obj
        .as_ref()
        .and_then(|o| o.data.get("spec"))
        .and_then(|s| s.get("configuration"))
        .and_then(|c| c.get("vmStateStorageClass"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    // Component presence. NOTE: a servable API type is NOT the same as a usable
    // component — an installed-but-empty snapshot CRD would greenlight a cluster
    // that cannot actually snapshot anything. Only use this for control-plane CRs
    // (CDI, Velero); capability claims below are gated on real *objects*.
    let crd_exists = |group: &str, version: &str, kind: &str| {
        let ar = ApiResource::from_gvk(&GroupVersionKind::gvk(group, version, kind));
        let cl = client.clone();
        async move {
            let api: Api<DynamicObject> = Api::all_with(cl, &ar);
            api.list(&kube::api::ListParams::default().limit(1))
                .await
                .is_ok()
        }
    };
    let cdi_present = crd_exists("cdi.kubevirt.io", "v1beta1", "CDI").await;
    let velero_present = crd_exists("velero.io", "v1", "Backup").await;

    // --- StorageClasses: which exist, which is default, and is it node-local? ---
    let sc_api: Api<StorageClass> = Api::all(client.clone());
    let sc_list = sc_api.list(&Default::default()).await.ok();
    let storage_classes: Vec<serde_json::Value> = sc_list
        .as_ref()
        .map(|l| {
            l.items
                .iter()
                .map(|sc| {
                    let name = sc.metadata.name.clone().unwrap_or_default();
                    let is_default = sc
                        .metadata
                        .annotations
                        .as_ref()
                        .and_then(|a| a.get(DEFAULT_SC_ANNOTATION))
                        .map(|v| v == "true")
                        .unwrap_or(false);
                    serde_json::json!({
                        "name": name,
                        "provisioner": sc.provisioner,
                        "default": is_default,
                        "node_local": is_node_local_provisioner(&sc.provisioner),
                        "rwx_capable": is_rwx_capable_provisioner(&sc.provisioner),
                        "allow_expansion": sc.allow_volume_expansion.unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let default_sc = storage_classes
        .iter()
        .find(|sc| sc["default"] == serde_json::json!(true));
    let default_storage_class = default_sc
        .and_then(|sc| sc["name"].as_str())
        .map(String::from);
    // The §7 trap: a default StorageClass that pins every VM disk to one node.
    let default_storage_class_node_local = default_sc
        .map(|sc| sc["node_local"] == serde_json::json!(true))
        .unwrap_or(false);
    let rwx_capable = storage_classes
        .iter()
        .any(|sc| sc["rwx_capable"] == serde_json::json!(true));
    // A class VM disks can actually live on (shared, not node-pinned).
    let shared_storage_class_present = storage_classes
        .iter()
        .any(|sc| sc["node_local"] == serde_json::json!(false));

    // --- VolumeSnapshotClasses: real objects, not just the CRD. ---
    let vsc_ar = ApiResource::from_gvk(&GroupVersionKind::gvk(
        "snapshot.storage.k8s.io",
        "v1",
        "VolumeSnapshotClass",
    ));
    let vsc_api: Api<DynamicObject> = Api::all_with(client.clone(), &vsc_ar);
    let vsc_list = vsc_api.list(&Default::default()).await.ok();
    let snapshot_class_crd_present = vsc_list.is_some();
    let volume_snapshot_classes: Vec<serde_json::Value> = vsc_list
        .as_ref()
        .map(|l| {
            l.items
                .iter()
                .map(|o| {
                    serde_json::json!({
                        "name": o.metadata.name.clone().unwrap_or_default(),
                        "driver": o.data.get("driver").and_then(|d| d.as_str()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // Snapshots need a *class*, not merely the API type.
    let snapshots_usable = !volume_snapshot_classes.is_empty();
    // A snapshot class is only useful if its driver matches a StorageClass provisioner.
    let snapshot_drivers_matched = volume_snapshot_classes.iter().any(|vsc| {
        vsc["driver"].as_str().is_some_and(|d| {
            storage_classes
                .iter()
                .any(|sc| sc["provisioner"].as_str() == Some(d))
        })
    });

    // --- Nodes: count + hardware virtualization actually exposed to KubeVirt. ---
    let node_api: Api<Node> = Api::all(client.clone());
    let nodes = node_api.list(&Default::default()).await.ok();
    let node_count = nodes.as_ref().map(|l| l.items.len()).unwrap_or(0);
    let kvm_nodes = nodes
        .as_ref()
        .map(|l| {
            l.items
                .iter()
                .filter(|n| {
                    n.status
                        .as_ref()
                        .and_then(|s| s.allocatable.as_ref())
                        .and_then(|a| a.get(KVM_DEVICE_RESOURCE))
                        .map(|q| q.0 != "0")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0);

    // --- Derived, first-class capability flags ---
    // Live migration needs somewhere to migrate *to* and shared RWX storage.
    let live_migration = node_count >= 2 && rwx_capable;
    // The golden-image workflow needs CDI (clone/import) and non-node-local storage.
    let windows_golden_images = cdi_present && shared_storage_class_present;

    Json(serde_json::json!({
        "kubevirt_version": kv_version,
        "feature_gates": gates,
        "day2_ops": {
            // Live CPU/memory hotplug needs the VM live-update rollout feature.
            "cpu_memory_live_hotplug": has("VMLiveUpdateFeatures"),
            // Storage-class disk migration needs the volumes update strategy.
            "storage_class_migration": has("VolumesUpdateStrategy") || has("VolumeMigration"),
            // Snapshots need an actual VolumeSnapshotClass object, not just the CRD.
            "snapshots": snapshots_usable,
            "velero_backups": velero_present,
            "cdi_import_convert": cdi_present,
            "live_migration": live_migration,
            "windows_golden_images": windows_golden_images,
            // Persistent TPM/EFI state (Windows 11, BitLocker) needs backend storage.
            "persistent_tpm_efi": has("VMPersistentState") && vm_state_storage_class.is_some(),
            // These are unconditional (plain API/subresource calls).
            "node_cordon": true, "run_strategy": true, "bulk_actions": true,
            "drift_remediation": true, "self_healing": true, "guest_ops_if_agent": true
        },
        "components": {
            "cdi": cdi_present, "velero": velero_present,
            "volume_snapshot_class": snapshot_class_crd_present
        },
        "storage": {
            "storage_classes": storage_classes,
            "default_storage_class": default_storage_class,
            "default_storage_class_node_local": default_storage_class_node_local,
            "shared_storage_class_present": shared_storage_class_present,
            "rwx_capable": rwx_capable,
            "volume_snapshot_classes": volume_snapshot_classes,
            "snapshot_drivers_matched": snapshot_drivers_matched,
            "vm_state_storage_class": vm_state_storage_class
        },
        "nodes": {
            "count": node_count,
            "kvm_capable": kvm_nodes
        },
        "note": "guest ops (patch/reclaim/app-consistent) additionally require a guest agent (GuestKit/QGA) in the VM image"
    }))
}

/// Marks the cluster-wide default StorageClass.
#[cfg(feature = "web")]
const DEFAULT_SC_ANNOTATION: &str = "storageclass.kubernetes.io/is-default-class";

/// KubeVirt advertises hardware virtualization as an extended node resource.
#[cfg(feature = "web")]
const KVM_DEVICE_RESOURCE: &str = "devices.kubevirt.io/kvm";

/// Provisioners whose volumes are pinned to a single node: a VM on one of these
/// cannot live-migrate and loses its disk if the node dies.
#[cfg(feature = "web")]
fn is_node_local_provisioner(provisioner: &str) -> bool {
    matches!(
        provisioner,
        "rancher.io/local-path"
            | "kubernetes.io/no-provisioner"
            | "openebs.io/local"
            | "topolvm.io"
    )
}

/// Provisioners that can back a `ReadWriteMany` volume — the KubeVirt
/// prerequisite for live migration of a PVC-backed VM.
#[cfg(feature = "web")]
fn is_rwx_capable_provisioner(provisioner: &str) -> bool {
    provisioner.contains("cephfs")
        || provisioner.contains("nfs")
        || provisioner.contains("glusterfs")
        || provisioner.contains("azurefile")
        || provisioner.contains("efs")
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformUpgradeRequest {
    /// `kubevirt` or `cdi`.
    pub component: String,
    /// Target image tag, e.g. `"v1.9.0"`.
    pub image_tag: String,
    /// Dry run unless explicitly false — reports the planned change without patching.
    #[serde(default = "default_true")]
    pub dry_run: bool,
}

#[cfg(feature = "web")]
fn default_true() -> bool {
    true
}

/// Trigger a KubeVirt or CDI upgrade by patching the operator CR's `spec.imageTag`.
/// Admin-gated (/platform/* is admin) and **dry run by default** — the operator
/// then performs the rollout. This is the guarded execution counterpart to
/// `GET /platform/versions`.
#[cfg(feature = "web")]
async fn platform_upgrade(
    State(state): State<SharedState>,
    Json(req): Json<PlatformUpgradeRequest>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use kube::api::{Api, ApiResource, DynamicObject, Patch, PatchParams};
    use kube::core::GroupVersionKind;

    let (gvk, ns, name): (GroupVersionKind, Option<&str>, &str) = match req.component.as_str() {
        "kubevirt" => (
            GroupVersionKind::gvk("kubevirt.io", "v1", "KubeVirt"),
            Some("kubevirt"),
            "kubevirt",
        ),
        "cdi" => (
            GroupVersionKind::gvk("cdi.kubevirt.io", "v1beta1", "CDI"),
            None,
            "cdi",
        ),
        other => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                format!("unknown component '{other}' (expected kubevirt|cdi)"),
            )
                .into_response();
        }
    };
    if req.image_tag.trim().is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "image_tag is required".to_string(),
        )
            .into_response();
    }

    let client = { state.read().await.kube_client.client().clone() };
    let ar = ApiResource::from_gvk(&gvk);
    let api: Api<DynamicObject> = match ns {
        Some(n) => Api::namespaced_with(client, n, &ar),
        None => Api::all_with(client, &ar),
    };
    let current = api.get_opt(name).await.ok().flatten();
    let current_tag = current
        .as_ref()
        .and_then(|o| o.data.get("spec"))
        .and_then(|s| s.get("imageTag"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if req.dry_run {
        return Json(serde_json::json!({
            "dry_run": true, "component": req.component,
            "current_image_tag": current_tag, "target_image_tag": req.image_tag,
            "note": "pass dry_run=false to apply — the operator then rolls out the upgrade"
        }))
        .into_response();
    }

    let patch = serde_json::json!({ "spec": { "imageTag": req.image_tag } });
    match api
        .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
    {
        Ok(_) => Json(serde_json::json!({
            "ok": true, "component": req.component,
            "from": current_tag, "to": req.image_tag,
            "note": "imageTag patched — operator is rolling out the upgrade"
        }))
        .into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_GATEWAY,
            format!("upgrade patch failed: {e}"),
        )
            .into_response(),
    }
}

/// Read the installed KubeVirt and CDI versions from their operator CRs so an
/// operator can see what platform layer is running (basis for upgrade decisions).
/// Read-only — actual version rollout is a deliberately separate, guarded action.
#[cfg(feature = "web")]
async fn platform_versions(State(state): State<SharedState>) -> Json<serde_json::Value> {
    use kube::api::{Api, ApiResource, DynamicObject};
    use kube::core::GroupVersionKind;

    let client = { state.read().await.kube_client.client().clone() };

    // KubeVirt CR: kubevirt.io/v1 KubeVirt "kubevirt" in ns "kubevirt".
    let kv_ar = ApiResource::from_gvk(&GroupVersionKind::gvk("kubevirt.io", "v1", "KubeVirt"));
    let kv: Api<DynamicObject> = Api::namespaced_with(client.clone(), "kubevirt", &kv_ar);
    let kubevirt = kv.get_opt("kubevirt").await.ok().flatten().map(|o| {
        let st = o.data.get("status").cloned().unwrap_or_default();
        serde_json::json!({
            "operatorVersion": st.get("operatorVersion"),
            "observedVersion": st.get("observedKubeVirtVersion"),
            "targetVersion": st.get("targetKubeVirtVersion"),
            "phase": st.get("phase"),
        })
    });

    // CDI CR: cdi.kubevirt.io/v1beta1 CDI "cdi" (cluster-scoped).
    let cdi_ar = ApiResource::from_gvk(&GroupVersionKind::gvk("cdi.kubevirt.io", "v1beta1", "CDI"));
    let cdi_api: Api<DynamicObject> = Api::all_with(client, &cdi_ar);
    let cdi = cdi_api.get_opt("cdi").await.ok().flatten().map(|o| {
        let st = o.data.get("status").cloned().unwrap_or_default();
        serde_json::json!({
            "operatorVersion": st.get("operatorVersion"),
            "observedVersion": st.get("observedVersion"),
            "targetVersion": st.get("targetVersion"),
            "phase": st.get("phase"),
        })
    });

    Json(serde_json::json!({
        "kubevirt": kubevirt,
        "cdi": cdi,
        "note": "installed platform versions; upgrade rollout is a separate guarded operation"
    }))
}

/// Discover operators by listing Deployments with common operator name patterns.
#[cfg(feature = "web")]
async fn list_operators(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<OperatorResponse>> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let client = s.client().client();
    let mut results = Vec::new();
    let crd_api: kube::api::Api<CustomResourceDefinition> = kube::api::Api::all(client.clone());
    let crds = crd_api
        .list(&kube::api::ListParams::default())
        .await
        .map(|l| l.items)
        .unwrap_or_default();

    let namespaces: Vec<String> = if namespace_scope::is_all_namespaces(&scope) {
        let mut ns = vec![
            "kubevirt".to_string(),
            "olm".to_string(),
            "operators".to_string(),
            "veyron-system".to_string(),
        ];
        if let Ok(all) = s.client().list_namespaces().await {
            for n in all.into_iter().filter_map(|n| n.metadata.name) {
                if !ns.contains(&n) {
                    ns.push(n);
                }
            }
        }
        ns
    } else {
        vec![
            "kubevirt".to_string(),
            "olm".to_string(),
            "operators".to_string(),
            "veyron-system".to_string(),
            scope.clone(),
        ]
    };

    for ns in &namespaces {
        let api: kube::api::Api<Deployment> =
            kube::api::Api::namespaced(client.clone(), ns.as_str());
        if let Ok(deploys) = api.list(&kube::api::ListParams::default()).await {
            for d in &deploys.items {
                let name = d.metadata.name.as_deref().unwrap_or("");
                // Heuristic: operators typically have "operator", "controller", or "manager" in name
                if name.contains("operator")
                    || name.contains("controller")
                    || name.contains("manager")
                {
                    let ready = d
                        .status
                        .as_ref()
                        .and_then(|s| s.ready_replicas)
                        .unwrap_or(0);
                    let desired = d.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1);

                    results.push(OperatorResponse {
                        name: name.to_string(),
                        namespace: ns.clone(),
                        version: infer_operator_version(d),
                        status: if ready >= desired {
                            "Running".to_string()
                        } else {
                            "Degraded".to_string()
                        },
                        managed_resources: infer_managed_resources(name, &crds),
                        installed_at: d
                            .metadata
                            .creation_timestamp
                            .as_ref()
                            .map(|t| t.0.to_rfc3339())
                            .unwrap_or_default(),
                    });
                }
            }
        }
    }

    Json(results)
}

#[cfg(feature = "web")]
fn infer_operator_version(deployment: &Deployment) -> String {
    if let Some(labels) = deployment.metadata.labels.as_ref() {
        for key in ["app.kubernetes.io/version", "version"] {
            if let Some(version) = labels.get(key) {
                return version.clone();
            }
        }
    }

    deployment
        .spec
        .as_ref()
        .and_then(|s| s.template.spec.as_ref())
        .and_then(|spec| spec.containers.first())
        .and_then(|container| container.image.as_deref())
        .and_then(extract_image_tag)
        .unwrap_or_default()
}

#[cfg(feature = "web")]
fn extract_image_tag(image: &str) -> Option<String> {
    let without_digest = image.split('@').next().unwrap_or(image);
    let last_segment = without_digest.rsplit('/').next().unwrap_or(without_digest);
    let colon_count = last_segment.matches(':').count();
    if colon_count == 1 {
        return last_segment
            .rsplit_once(':')
            .map(|(_, tag)| tag.to_string())
            .filter(|tag| !tag.is_empty());
    }
    None
}

#[cfg(feature = "web")]
fn infer_managed_resources(operator_name: &str, crds: &[CustomResourceDefinition]) -> Vec<String> {
    let tokens = operator_tokens(operator_name);
    let mut managed = Vec::new();

    for crd in crds {
        let spec = &crd.spec;
        let haystacks = [
            crd.metadata.name.as_deref().unwrap_or(""),
            spec.group.as_str(),
            spec.names.kind.as_str(),
            spec.names.plural.as_str(),
        ];

        if tokens
            .iter()
            .any(|token| haystacks.iter().any(|h| h.to_lowercase().contains(token)))
        {
            managed.push(spec.names.kind.clone());
        }
    }

    managed.sort();
    managed.dedup();
    managed
}

#[cfg(feature = "web")]
fn operator_tokens(name: &str) -> Vec<String> {
    let ignored = [
        "operator",
        "controller",
        "manager",
        "deployment",
        "system",
        "app",
        "cluster",
    ];

    name.split(|c: char| !c.is_ascii_alphanumeric())
        .map(|part| part.to_lowercase())
        .filter(|part| part.len() > 2 && !ignored.contains(&part.as_str()))
        .collect()
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    #[test]
    fn extract_image_tag_reads_container_tag() {
        assert_eq!(
            extract_image_tag("quay.io/example/veyron-operator:v0.2.0"),
            Some("v0.2.0".to_string())
        );
        assert_eq!(
            extract_image_tag("quay.io/example/image@sha256:deadbeef"),
            None
        );
    }

    #[test]
    fn operator_tokens_drops_generic_words() {
        assert_eq!(operator_tokens("veyron-operator-manager"), vec!["veyron"]);
    }
}
