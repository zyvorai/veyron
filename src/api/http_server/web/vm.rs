// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//
// VM handlers: list/get, expose (SSH/RDP/internet), guest agent, power, migrate, volumes, create/delete.

use super::*;
use crate::api::vm_backend::{self, BackendKind, VmBackend};

#[derive(Deserialize)]
pub struct VmQuery {
    pub namespace: Option<String>,
    /// Alias for `namespace` (dashboard legacy `?ns=`).
    #[serde(alias = "ns")]
    pub ns: Option<String>,
    /// Agent sandbox VMs are left out unless asked for (or their namespace is listed).
    #[serde(default)]
    pub include_sandboxes: bool,
}

/// First guest IP + node name from VMI status (VMI name matches VM name in KubeVirt).
pub(super) async fn vmi_ip_node_index(
    client: &KubeClient,
    scope_ns: &str,
) -> HashMap<(String, String), (Option<String>, Option<String>, bool)> {
    let vmis = if scope_ns == "all" {
        client.list_all_vmis().await.unwrap_or_default()
    } else {
        client.list_vmis(scope_ns).await.unwrap_or_default()
    };
    let mut m = HashMap::with_capacity(vmis.len());
    for vmi in vmis {
        let ns = vmi.metadata.namespace.clone().unwrap_or_default();
        let name = vmi.metadata.name.clone().unwrap_or_default();
        if ns.is_empty() || name.is_empty() {
            continue;
        }
        let ip = vmi.status.as_ref().and_then(|s| {
            s.interfaces.iter().find_map(|iface| {
                iface
                    .ip_address
                    .as_ref()
                    .filter(|ip| !ip.is_empty())
                    .cloned()
            })
        });
        let node = vmi.status.as_ref().and_then(|s| s.node_name.clone());
        let agent_connected = vmi.status.as_ref().is_some_and(|s| {
            s.conditions.iter().any(|c| {
                c.type_.as_deref() == Some("AgentConnected") && c.status.as_deref() == Some("True")
            })
        });
        m.insert((ns, name), (ip, node, agent_connected));
    }
    m
}

pub(super) async fn vrvm_drift_index(
    kube_client: &crate::kube::KubeClient,
    scope_ns: &str,
) -> std::collections::HashMap<(String, String), (bool, Option<String>)> {
    use crate::api::handlers::namespace_scope;
    use crate::operator_crds::VeyronVM;
    use std::collections::HashMap;

    let mut index = HashMap::new();
    let client = kube_client.client();
    let Ok(list) = namespace_scope::list_namespaced_resource::<VeyronVM>(&client, scope_ns).await
    else {
        return index;
    };

    for vm in list {
        let ns = vm
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let name = vm.metadata.name.clone().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let drift = vm
            .status
            .as_ref()
            .map(|s| s.drift_detected)
            .unwrap_or(false);
        let message = vm.status.as_ref().and_then(|s| s.drift_message.clone());
        index.insert((ns.clone(), name.clone()), (drift, message.clone()));
        if let Some(kv) = vm.status.as_ref().and_then(|s| s.kubevirt_vm_name.clone()) {
            if kv != name {
                index.insert((ns, kv), (drift, message));
            }
        }
    }
    index
}

pub(super) async fn list_vms_handler(
    State(state): State<SharedState>,
    Query(query): Query<VmQuery>,
) -> impl IntoResponse {
    let (client, default_namespace) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };

    let scope_ns = query
        .namespace
        .as_deref()
        .or(query.ns.as_deref())
        .unwrap_or(default_namespace.as_str());

    if vm_backend::selected() != BackendKind::KubeVirt {
        let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms");
        return match VmBackend::for_client(&client).list(scope_ns).await {
            Ok(vm_infos) => ok_json(&ApiResponse::success(&vm_infos, &ctx.request_id)),
            Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
        };
    }

    let vms = client.list_vms_for_scope(scope_ns).await;
    let vmi_index = vmi_ip_node_index(&client, scope_ns).await;
    let drift_index = vrvm_drift_index(&client, scope_ns).await;
    let hide_sandboxes = !query.include_sandboxes && scope_ns != crate::sandbox::namespace();
    let vm_infos: Vec<VmInfo> = vms
        .iter()
        .filter(|vm| {
            !hide_sandboxes
                || vm
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get(crate::sandbox::LABEL_SANDBOX))
                    .is_none_or(|v| v != "true")
        })
        .map(|vm| {
            let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
            let name = vm.metadata.name.as_deref().unwrap_or("");
            let (ip, node, agent_connected) = vmi_index
                .get(&(ns.to_string(), name.to_string()))
                .cloned()
                .unwrap_or((None, None, false));
            let mut info = VmInfo::from_vm_with_vmi_data(vm, ip, node);
            info.guest_agent_connected = Some(agent_connected);
            if let Some((drift, message)) = drift_index.get(&(ns.to_string(), name.to_string())) {
                info.veyron_managed = Some(true);
                info.drift_detected = Some(*drift);
                info.drift_message = message.clone();
            } else {
                info.veyron_managed = Some(false);
            }
            info
        })
        .collect();
    let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms");
    ok_json(&ApiResponse::success(&vm_infos, &ctx.request_id))
}

pub(super) async fn get_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    if vm_backend::selected() != BackendKind::KubeVirt {
        return match VmBackend::for_client(&client).get(&ns, &name).await {
            Ok(detail) => {
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name");
                ok_json(&ApiResponse::success(&detail, &ctx.request_id))
            }
            Err(e) => {
                let msg = sanitize_error(&e);
                if msg.contains("NotFound") || msg.contains("not found") {
                    err_json(404, "NOT_FOUND", &format!("VM '{}' not found", name))
                } else {
                    err_json(500, "INTERNAL_ERROR", &msg)
                }
            }
        };
    }

    match client.get_vm(&ns, &name).await {
        Ok(vm) => {
            let (ip, node) = client
                .get_vm_ip_and_node(&ns, &name)
                .await
                .unwrap_or((None, None));
            let info = VmInfo::from_vm_with_vmi_data(&vm, ip, node);

            // Also fetch VMI details if running
            let vmi_detail = client.get_vmi(&ns, &name).await.ok();
            let detail = VmDetail {
                info,
                vmi_status: vmi_detail.and_then(|v| v.status),
            };

            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name");
            ok_json(&ApiResponse::success(&detail, &ctx.request_id))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            if msg.contains("NotFound") || msg.contains("not found") {
                err_json(404, "NOT_FOUND", &format!("VM '{}' not found", name))
            } else {
                err_json(500, "INTERNAL_ERROR", &msg)
            }
        }
    }
}

pub(super) async fn get_vm_drift_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    use crate::operator_crds::VeyronVM;
    use kube::Api;

    let client = {
        let s = state.read().await;
        s.kube_client.client().clone()
    };
    let api: Api<VeyronVM> = Api::namespaced(client, &ns);

    match api.get(&name).await {
        Ok(vm) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/drift");
            let body = serde_json::json!({
                "name": name,
                "namespace": ns,
                "drift_detected": vm.status.as_ref().map(|s| s.drift_detected).unwrap_or(false),
                "drift_message": vm.status.as_ref().and_then(|s| s.drift_message.clone()),
                "resolved_spec_hash": vm.status.as_ref().and_then(|s| s.resolved_spec_hash.clone()),
                "template": vm.spec.template,
                "profile": vm.spec.profile,
            });
            ok_json(&ApiResponse::success(&body, &ctx.request_id))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            if msg.contains("NotFound") || msg.contains("not found") {
                err_json(
                    404,
                    "NOT_FOUND",
                    &format!("VeyronVM '{name}' not found in namespace '{ns}'"),
                )
            } else {
                err_json(500, "INTERNAL_ERROR", &msg)
            }
        }
    }
}

pub(super) fn json_int_or_string(tp: &IntOrString) -> serde_json::Value {
    match tp {
        IntOrString::Int(i) => serde_json::json!(i),
        IntOrString::String(s) => serde_json::json!(s),
    }
}

/// Build API JSON for the Veyron-managed expose `Service` (or disabled stub when absent).
pub(super) fn vm_expose_status_json(svc: Option<&Service>, vm_name: &str) -> serde_json::Value {
    let Some(svc) = svc else {
        return serde_json::json!({
            "enabled": false,
            "service_name": vm_expose_service_name(vm_name),
            "service_type": null,
            "cluster_ip": null,
            "ports": [],
        });
    };
    let spec = svc.spec.as_ref();
    let service_type = spec.and_then(|s| s.type_.clone());
    let cluster_ip = spec.and_then(|s| s.cluster_ip.clone());
    let ports_json: Vec<serde_json::Value> = spec
        .and_then(|s| s.ports.as_ref())
        .map(|ports| {
            ports
                .iter()
                .map(|p| {
                    let target_port = p
                        .target_port
                        .as_ref()
                        .map(json_int_or_string)
                        .unwrap_or(serde_json::Value::Null);
                    serde_json::json!({
                        "name": p.name,
                        "port": p.port,
                        "target_port": target_port,
                        "node_port": p.node_port,
                        "protocol": p.protocol,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    serde_json::json!({
        "enabled": true,
        "service_name": svc.metadata.name,
        "service_type": service_type,
        "cluster_ip": cluster_ip,
        "ports": ports_json,
    })
}

pub(super) async fn get_vm_expose_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    match client.get_vm_expose_service(&ns, &name).await {
        Ok(svc) => {
            let body = vm_expose_status_json(svc.as_ref(), &name);
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/expose");
            ok_json(&ApiResponse::success(&body, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

pub(super) async fn put_vm_expose_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<CreateVmExposeReq>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    if !req.enabled {
        match client.delete_vm_expose_service(&ns, &name).await {
            Ok(()) => {
                let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name/expose");
                let body = vm_expose_status_json(None, &name);
                ok_json(&ApiResponse::success(&body, &ctx.request_id))
            }
            Err(e) => {
                let msg = sanitize_error(&e);
                if msg.contains("not managed by Veyron") {
                    err_json(409, "CONFLICT", &msg)
                } else {
                    err_json(500, "INTERNAL_ERROR", &msg)
                }
            }
        }
    } else {
        #[cfg(feature = "kairon")]
        if crate::api::vm_backend::is_kairon() {
            return kairon_put_ssh_expose(&client, &ns, &name, req).await;
        }
        match client.get_vm(&ns, &name).await {
            Ok(_) => {}
            Err(e) => {
                let msg = sanitize_error(&e);
                if msg.contains("NotFound")
                    || msg.contains("not found")
                    || msg.eq("Resource not found")
                {
                    return err_json(404, "NOT_FOUND", &format!("VM '{}' not found", name));
                }
                return err_json(500, "INTERNAL_ERROR", &msg);
            }
        }
        let Some(cfg) = vm_expose_from_api_req(req) else {
            return err_json(
                400,
                "INVALID_REQUEST",
                "enabled=true requires a valid expose configuration",
            );
        };
        // NodePort/LoadBalancer node ports must fall within the cluster
        // NodePort range; reject anything outside 30000–32767 up front.
        if matches!(
            cfg.service_type.to_ascii_lowercase().as_str(),
            "nodeport" | "loadbalancer"
        ) {
            for p in &cfg.ports {
                if let Some(np) = p.node_port {
                    if !(30000..=32767).contains(&np) {
                        return err_json(
                            400,
                            "INVALID_NODE_PORT",
                            &format!(
                                "node_port {} is out of range — NodePort services must use 30000–32767 (suggested 30100–30199 per VM)",
                                np
                            ),
                        );
                    }
                }
            }
        }
        match client.upsert_vm_expose_service(&ns, &name, &cfg).await {
            Ok(()) => match client.get_vm_expose_service(&ns, &name).await {
                Ok(svc) => {
                    let body = vm_expose_status_json(svc.as_ref(), &name);
                    let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name/expose");
                    ok_json(&ApiResponse::success(&body, &ctx.request_id))
                }
                Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
            },
            Err(e) => {
                let msg = sanitize_error(&e);
                err_json(
                    status_for_sanitized_error(&msg),
                    "EXPOSE_UPSERT_FAILED",
                    &msg,
                )
            }
        }
    }
}

pub(super) async fn delete_vm_expose_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    match client.delete_vm_expose_service(&ns, &name).await {
        Ok(()) => {
            let body = vm_expose_status_json(None, &name);
            let ctx = req_ctx(HttpMethod::DELETE, "/api/v1/vms/:ns/:name/expose");
            ok_json(&ApiResponse::success(&body, &ctx.request_id))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            if msg.contains("not managed by Veyron") {
                err_json(409, "CONFLICT", &msg)
            } else {
                err_json(500, "INTERNAL_ERROR", &msg)
            }
        }
    }
}

pub(super) async fn vm_internet_get_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = {
        let s = state.read().await;
        s.kube_client.client()
    };
    let st = vm_internet::vm_internet_status(&client, &ns, &name).await;
    let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/network/internet");
    ok_json(&ApiResponse::success(&st, &ctx.request_id))
}

pub(super) async fn vm_internet_put_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    if let Err(e) = kube.get_vm(&ns, &name).await {
        let msg = sanitize_error(&e);
        if msg.contains("NotFound") || msg.contains("not found") {
            return err_json(404, "NOT_FOUND", &format!("VM '{}' not found", name));
        }
        return err_json(500, "INTERNAL_ERROR", &msg);
    }
    match vm_internet::ensure_vm_internet_egress(&kube.client(), &ns, &name).await {
        Ok(st) => {
            let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name/network/internet");
            ok_json(&ApiResponse::success(&st, &ctx.request_id))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(
                status_for_sanitized_error(&msg),
                "INTERNET_POLICY_FAILED",
                &msg,
            )
        }
    }
}

pub(super) async fn vm_internet_delete_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = {
        let s = state.read().await;
        s.kube_client.client()
    };
    match vm_internet::remove_vm_internet_egress(&client, &ns, &name).await {
        Ok(()) => {
            let ctx = req_ctx(HttpMethod::DELETE, "/api/v1/vms/:ns/:name/network/internet");
            let body = vm_internet::vm_internet_status(&client, &ns, &name).await;
            ok_json(&ApiResponse::success(&body, &ctx.request_id))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(
                status_for_sanitized_error(&msg),
                "INTERNET_POLICY_DELETE_FAILED",
                &msg,
            )
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct PutRdpExposeReq {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    service_type: Option<String>,
    /// Required for NodePort / LoadBalancer (pick a unique port per VM, e.g. 30100–30199).
    node_port: Option<i32>,
}

pub(super) async fn rdp_context_for_vm(
    kube: &KubeClient,
    ns: &str,
    name: &str,
) -> Result<vm_rdp::RdpAccessStatus, String> {
    let vm = kube
        .get_vm(ns, name)
        .await
        .map_err(|e| sanitize_error(&e))?;
    let vm_json = serde_json::to_value(&vm).map_err(|e| e.to_string())?;
    let spec_has_rdp = vm_json
        .pointer("/spec/template/spec/domain/devices/interfaces")
        .and_then(|i| i.as_array())
        .is_some_and(|ifaces| {
            ifaces.iter().any(|iface| {
                iface
                    .get("ports")
                    .and_then(|p| p.as_array())
                    .is_some_and(|ports| {
                        ports.iter().any(|p| {
                            p.get("port")
                                .and_then(|n| n.as_i64())
                                .is_some_and(|n| n == 3389)
                        })
                    })
            })
        });

    let (guest_ip, vmi_json) = kube
        .list_vmis(ns)
        .await
        .ok()
        .and_then(|vmis| {
            vmis.into_iter()
                .find(|v| v.metadata.name.as_deref() == Some(name))
                .map(|vmi| {
                    let ip = vmi.status.as_ref().and_then(|s| {
                        s.interfaces.iter().find_map(|iface| {
                            iface
                                .ip_address
                                .as_ref()
                                .filter(|ip| !ip.is_empty())
                                .cloned()
                        })
                    });
                    let json = serde_json::to_value(&vmi).ok();
                    (ip, json)
                })
        })
        .unwrap_or((None, None));
    let is_windows = windows_rdp::is_windows_guest(Some(&vm_json), vmi_json.as_ref());

    let node_ips: Vec<String> = kube
        .list_nodes()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|n| {
            n.status.as_ref().and_then(|st| {
                st.addresses.as_ref().and_then(|rows| {
                    rows.iter().find_map(|a| {
                        if a.type_ == "InternalIP" {
                            Some(a.address.clone())
                        } else {
                            None
                        }
                    })
                })
            })
        })
        .collect();

    vm_rdp::assemble_rdp_access(
        kube.client(),
        ns,
        name,
        guest_ip,
        is_windows,
        spec_has_rdp,
        node_ips,
    )
    .await
    .map_err(|e| sanitize_error(&e))
}

#[cfg(feature = "kairon")]
async fn kairon_put_ssh_expose(
    client: &crate::kube::KubeClient,
    ns: &str,
    name: &str,
    req: CreateVmExposeReq,
) -> (StatusCode, Json<serde_json::Value>) {
    use crate::api::kairon_ops::{self, ExposeTarget};
    let Some(cfg) = vm_expose_from_api_req(req) else {
        return err_json(
            400,
            "INVALID_REQUEST",
            "enabled=true requires a valid expose configuration",
        );
    };
    let guest_port = cfg
        .ports
        .first()
        .and_then(|p| u16::try_from(p.target_port).ok())
        .unwrap_or(22);
    let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name/expose");
    match kairon_ops::machine_expose_target(client, ns, name, guest_port).await {
        Ok(ExposeTarget::Forward { node, host_port }) => ok_json(&ApiResponse::success(
            &kairon_ops::forward_json(name, guest_port, &node, host_port),
            &ctx.request_id,
        )),
        Ok(ExposeTarget::GuestIp(ip)) => {
            let svc = crate::kube::vm_expose_service_name(name);
            let bound = async {
                client.upsert_vm_expose_service(ns, name, &cfg).await?;
                kairon_ops::bind_service_to_guest(client, ns, &svc, &ip).await
            };
            match bound.await {
                Ok(()) => match client.get_vm_expose_service(ns, name).await {
                    Ok(svc) => ok_json(&ApiResponse::success(
                        &vm_expose_status_json(svc.as_ref(), name),
                        &ctx.request_id,
                    )),
                    Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
                },
                Err(e) => {
                    let msg = sanitize_error(&e);
                    err_json(
                        status_for_sanitized_error(&msg),
                        "EXPOSE_UPSERT_FAILED",
                        &msg,
                    )
                }
            }
        }
        Err(e) => kairon_expose_error(&e),
    }
}

#[cfg(feature = "kairon")]
fn kairon_expose_error(e: &anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    let raw = format!("{e:#}");
    if raw.ends_with("(conflict)") {
        err_json(
            409,
            "EXPOSE_UNAVAILABLE",
            raw.trim_end_matches(" (conflict)"),
        )
    } else {
        let msg = sanitize_error(e);
        err_json(status_for_sanitized_error(&msg), "EXPOSE_FAILED", &msg)
    }
}

#[cfg(feature = "kairon")]
async fn kairon_rdp_status(
    kube: &crate::kube::KubeClient,
    ns: &str,
    name: &str,
) -> serde_json::Value {
    use crate::api::kairon_ops::{self, ExposeTarget};
    let svc_name = crate::kube::windows_rdp::veyron_rdp_service_name(name);
    let svc = kube::Api::<k8s_openapi::api::core::v1::Service>::namespaced(kube.client(), ns)
        .get_opt(&svc_name)
        .await
        .ok()
        .flatten();
    match kairon_ops::machine_expose_target(kube, ns, name, 3389).await {
        Ok(ExposeTarget::Forward { node, host_port }) => {
            kairon_ops::forward_json(name, 3389, &node, host_port)
        }
        target => serde_json::json!({
            "enabled": svc.is_some(),
            "service_name": svc.as_ref().and_then(|s| s.metadata.name.clone()),
            "service_type": svc.as_ref().and_then(|s| s.spec.as_ref()?.type_.clone()),
            "guest_ip": match &target { Ok(ExposeTarget::GuestIp(ip)) => Some(ip.clone()), _ => None },
            "reason": target.err().map(|e| format!("{e:#}").trim_end_matches(" (conflict)").to_string()),
        }),
    }
}

#[cfg(feature = "kairon")]
async fn kairon_put_rdp_expose(
    kube: &crate::kube::KubeClient,
    ns: &str,
    name: &str,
    svc_type: &str,
    node_port: i32,
) -> (StatusCode, Json<serde_json::Value>) {
    use crate::api::kairon_ops::{self, ExposeTarget};
    let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name/rdp-expose");
    match kairon_ops::machine_expose_target(kube, ns, name, 3389).await {
        Ok(ExposeTarget::Forward { node, host_port }) => ok_json(&ApiResponse::success(
            &kairon_ops::forward_json(name, 3389, &node, host_port),
            &ctx.request_id,
        )),
        Ok(ExposeTarget::GuestIp(ip)) => {
            if let Err(e) =
                vm_rdp::upsert_rdp_expose_service(kube.client(), ns, name, svc_type, node_port)
                    .await
            {
                let raw = e.to_string();
                if raw.contains("VEYRON_ALLOW_PUBLIC_RDP") {
                    return err_json(403, "PUBLIC_RDP_FORBIDDEN", &raw);
                }
                let msg = sanitize_error(&e);
                return err_json(
                    status_for_sanitized_error(&msg),
                    "RDP_EXPOSE_UPSERT_FAILED",
                    &msg,
                );
            }
            let svc = crate::kube::windows_rdp::veyron_rdp_service_name(name);
            if let Err(e) = kairon_ops::bind_service_to_guest(kube, ns, &svc, &ip).await {
                let msg = sanitize_error(&e);
                return err_json(
                    status_for_sanitized_error(&msg),
                    "RDP_EXPOSE_UPSERT_FAILED",
                    &msg,
                );
            }
            ok_json(&ApiResponse::success(
                &kairon_rdp_status(kube, ns, name).await,
                &ctx.request_id,
            ))
        }
        Err(e) => kairon_expose_error(&e),
    }
}

pub(super) async fn get_vm_rdp_expose_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::is_kairon() {
        let body = kairon_rdp_status(&kube, &ns, &name).await;
        let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/rdp-expose");
        return ok_json(&ApiResponse::success(&body, &ctx.request_id));
    }
    match rdp_context_for_vm(&kube, &ns, &name).await {
        Ok(body) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/rdp-expose");
            ok_json(&ApiResponse::success(&body, &ctx.request_id))
        }
        Err(msg) => err_json(500, "INTERNAL_ERROR", &msg),
    }
}

pub(super) async fn put_vm_rdp_expose_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<PutRdpExposeReq>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    if !req.enabled {
        match vm_rdp::delete_rdp_expose_service(kube.client(), &ns, &name).await {
            #[cfg(feature = "kairon")]
            Ok(()) if crate::api::vm_backend::is_kairon() => {
                let body = kairon_rdp_status(&kube, &ns, &name).await;
                let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name/rdp-expose");
                ok_json(&ApiResponse::success(&body, &ctx.request_id))
            }
            Ok(()) => match rdp_context_for_vm(&kube, &ns, &name).await {
                Ok(body) => {
                    let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name/rdp-expose");
                    ok_json(&ApiResponse::success(&body, &ctx.request_id))
                }
                Err(msg) => err_json(500, "INTERNAL_ERROR", &msg),
            },
            Err(e) => {
                let msg = sanitize_error(&e);
                err_json(
                    status_for_sanitized_error(&msg),
                    "RDP_EXPOSE_DELETE_FAILED",
                    &msg,
                )
            }
        }
    } else {
        // Only NodePort/LoadBalancer need a node_port; ClusterIP does not.
        let svc_type = match req
            .service_type
            .as_deref()
            .unwrap_or("NodePort")
            .to_ascii_lowercase()
            .as_str()
        {
            "loadbalancer" => "LoadBalancer",
            "clusterip" => "ClusterIP",
            _ => "NodePort",
        };
        let node_port = if svc_type == "NodePort" || svc_type == "LoadBalancer" {
            let Some(np) = req.node_port else {
                return err_json(
                    400,
                    "INVALID_REQUEST",
                    "node_port is required for a NodePort/LoadBalancer RDP service (use 30100–30199 per VM, within 30000–32767)",
                );
            };
            if !(30000..=32767).contains(&np) {
                return err_json(
                    400,
                    "INVALID_REQUEST",
                    "node_port must be in range 30000–32767",
                );
            }
            np
        } else {
            // ClusterIP ignores node_port; upsert won't set one on the service.
            req.node_port.unwrap_or(0)
        };
        #[cfg(feature = "kairon")]
        if crate::api::vm_backend::is_kairon() {
            return kairon_put_rdp_expose(&kube, &ns, &name, svc_type, node_port).await;
        }
        if let Err(e) = kube.get_vm(&ns, &name).await {
            let msg = sanitize_error(&e);
            if msg.contains("NotFound") || msg.contains("not found") || msg.eq("Resource not found")
            {
                return err_json(404, "NOT_FOUND", &format!("VM '{}' not found", name));
            }
            return err_json(500, "INTERNAL_ERROR", &msg);
        }
        // Auto-patch masquerade port 3389 if missing, then restart VMI.
        // Warn-only: service creation still proceeds even if the patch fails.
        match kube.ensure_rdp_masquerade_port(&ns, &name).await {
            Ok(true) => log::info!(
                "RDP expose: patched masquerade port 3389 on {}/{} and restarted VMI",
                ns,
                name
            ),
            Ok(false) => {}
            Err(e) => log::warn!(
                "RDP expose: could not patch masquerade port on {}/{}: {}",
                ns,
                name,
                e
            ),
        }

        match vm_rdp::upsert_rdp_expose_service(kube.client(), &ns, &name, svc_type, node_port)
            .await
        {
            Ok(()) => match rdp_context_for_vm(&kube, &ns, &name).await {
                Ok(body) => {
                    let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name/rdp-expose");
                    ok_json(&ApiResponse::success(&body, &ctx.request_id))
                }
                Err(msg) => err_json(500, "INTERNAL_ERROR", &msg),
            },
            Err(e) => {
                // Check the RAW error for the policy signature — sanitize_error
                // rewrites the message and would drop it, mis-mapping the refusal
                // to a 500. Publishing bare RDP is refused by policy, not broken.
                let raw = e.to_string();
                if raw.contains("VEYRON_ALLOW_PUBLIC_RDP") {
                    return err_json(403, "PUBLIC_RDP_FORBIDDEN", &raw);
                }
                let msg = sanitize_error(&e);
                err_json(
                    status_for_sanitized_error(&msg),
                    "RDP_EXPOSE_UPSERT_FAILED",
                    &msg,
                )
            }
        }
    }
}

pub(super) async fn guest_agent_enable_rdp_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    #[cfg(feature = "kairon")]
    let result = if crate::api::vm_backend::is_kairon() {
        crate::api::kairon_ops::set_windows_rdp(&kube, &ns, &name, true).await
    } else {
        kube.enable_rdp_via_guest_agent(&ns, &name).await
    };
    #[cfg(not(feature = "kairon"))]
    let result = kube.enable_rdp_via_guest_agent(&ns, &name).await;
    match result {
        Ok(resp) => {
            let ctx = req_ctx(
                HttpMethod::POST,
                "/api/v1/vms/:ns/:name/guest-agent/enable-rdp",
            );
            ok_json(&ApiResponse::success(&resp, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_ENABLE_RDP_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_agent_disable_rdp_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    #[cfg(feature = "kairon")]
    let result = if crate::api::vm_backend::is_kairon() {
        crate::api::kairon_ops::set_windows_rdp(&kube, &ns, &name, false).await
    } else {
        kube.disable_rdp_via_guest_agent(&ns, &name).await
    };
    #[cfg(not(feature = "kairon"))]
    let result = kube.disable_rdp_via_guest_agent(&ns, &name).await;
    match result {
        Ok(resp) => {
            let ctx = req_ctx(
                HttpMethod::POST,
                "/api/v1/vms/:ns/:name/guest-agent/disable-rdp",
            );
            ok_json(&ApiResponse::success(&resp, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_DISABLE_RDP_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_filesystem_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/guest-filesystem");
    match kube.guest_filesystem_metrics(&ns, &name).await {
        Ok(resp) => ok_json(&ApiResponse::success(&resp, &ctx.request_id)),
        // Guest filesystem data needs a connected guest runtime; when it's
        // unavailable (no/disconnected agent) that's expected, not a 5xx —
        // degrade to a 200 "unavailable" so the dashboard shows a placeholder
        // instead of a console error.
        Err(e) => ok_json(&ApiResponse::success(
            &serde_json::json!({
                "available": false,
                "filesystems": [],
                "reason": sanitize_error(&e),
            }),
            &ctx.request_id,
        )),
    }
}

pub(super) async fn guest_runtime_status_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::is_kairon() {
        return match crate::api::kairon_ops::guest_status(&kube, &ns, &name).await {
            Ok(resp) => {
                let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/guest/status");
                ok_json(&ApiResponse::success(&resp, &ctx.request_id))
            }
            Err(e) => err_json(500, "GUEST_STATUS_FAILED", &sanitize_error(&e)),
        };
    }
    match kube.guest_runtime_status(&ns, &name).await {
        Ok(resp) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/guest/status");
            ok_json(&ApiResponse::success(&resp, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_STATUS_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_evidence_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    match kube.guest_evidence(&ns, &name).await {
        Ok(resp) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/guest/evidence");
            ok_json(&ApiResponse::success(&resp, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_EVIDENCE_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_doctor_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let target = params.get("target").map(String::as_str).unwrap_or("kvm");
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    match kube.guest_doctor(&ns, &name, target).await {
        Ok(resp) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/guest/doctor");
            ok_json(&ApiResponse::success(&resp, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_DOCTOR_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_fix_plan_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    match kube.guest_fix_plan(&ns, &name, body).await {
        Ok(resp) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/guest/fix-plan");
            ok_json(&ApiResponse::success(&resp, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_FIX_PLAN_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_metrics_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    match kube.guest_metrics(&ns, &name).await {
        Ok(resp) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/guest/metrics");
            ok_json(&ApiResponse::success(&resp, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_METRICS_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_migrate_score_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let target = params.get("target").map(String::as_str).unwrap_or("kvm");
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    match kube.guest_migrate_score(&ns, &name, target).await {
        Ok(resp) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/guest/migrate-score");
            ok_json(&ApiResponse::success(&resp, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_MIGRATE_SCORE_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_exec_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::is_kairon() {
        return match crate::api::kairon_ops::guest_exec(&kube, &ns, &name, &body).await {
            Ok(resp) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/guest/exec");
                ok_json(&ApiResponse::success(&resp, &ctx.request_id))
            }
            Err(e) => err_json(500, "GUEST_EXEC_FAILED", &sanitize_error(&e)),
        };
    }
    match kube.guest_exec_via_guestkit(&ns, &name, body).await {
        Ok(resp) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/guest/exec");
            ok_json(&ApiResponse::success(&resp, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_EXEC_FAILED", &sanitize_error(&e)),
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct AgentDeployRequest {
    #[serde(default)]
    spawn: bool,
    os: Option<String>,
    bundle_url: Option<String>,
    iso: Option<String>,
}

/// GET /api/v1/vms/:ns/:name/guest/agent — GuestKit agent status (connected, version, RPC count).
pub(super) async fn guest_agent_status_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    match crate::handlers::agent::agent_status_json(&kube, &ns, &name).await {
        Ok(v) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/guest/agent");
            ok_json(&ApiResponse::success(&v, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_AGENT_STATUS_FAILED", &sanitize_error(&e)),
    }
}

/// POST /api/v1/vms/:ns/:name/guest/agent/deploy — deploy the GuestKit agent into the VM.
pub(super) async fn guest_agent_deploy_handler(
    State(_state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    body: Option<Json<AgentDeployRequest>>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let req = body.map(|Json(b)| b).unwrap_or_default();
    match crate::handlers::agent::deploy_agent_api(
        &ns,
        &name,
        req.os,
        req.spawn,
        req.bundle_url,
        req.iso,
    )
    .await
    {
        Ok(v) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/guest/agent/deploy");
            ok_json(&ApiResponse::success(&v, &ctx.request_id))
        }
        Err(e) => err_json(500, "GUEST_AGENT_DEPLOY_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guestkit_binary_handler() -> impl IntoResponse {
    match crate::kube::KubeClient::guestkit_binary_bytes() {
        Ok(bytes) => (
            [
                (axum::http::header::CONTENT_TYPE, "application/octet-stream"),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    "attachment; filename=\"guestkit\"",
                ),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => err_json(503, "GUESTKIT_BINARY_UNAVAILABLE", &sanitize_error(&e)).into_response(),
    }
}

pub(super) async fn delete_vm_rdp_expose_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let kube = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    match vm_rdp::delete_rdp_expose_service(kube.client(), &ns, &name).await {
        Ok(()) => match rdp_context_for_vm(&kube, &ns, &name).await {
            Ok(body) => {
                let ctx = req_ctx(HttpMethod::DELETE, "/api/v1/vms/:ns/:name/rdp-expose");
                ok_json(&ApiResponse::success(&body, &ctx.request_id))
            }
            Err(msg) => err_json(500, "INTERNAL_ERROR", &msg),
        },
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(
                status_for_sanitized_error(&msg),
                "RDP_EXPOSE_DELETE_FAILED",
                &msg,
            )
        }
    }
}

pub(super) async fn start_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    match VmBackend::for_client(&client).start(&ns, &name).await {
        Ok(_) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/start");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": format!("VM '{}' started", name)}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "START_FAILED", &msg)
        }
    }
}

pub(super) async fn stop_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    match VmBackend::for_client(&client).stop(&ns, &name).await {
        Ok(_) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/stop");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": format!("VM '{}' stopped", name)}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "STOP_FAILED", &msg)
        }
    }
}

pub(super) async fn restart_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    match VmBackend::for_client(&client).restart(&ns, &name).await {
        Ok(_) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/restart");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": format!("VM '{}' restarted", name)}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "RESTART_FAILED", &msg)
        }
    }
}

// ── VNC Console WebSocket Proxy ─────────────────────────────

pub(super) async fn vm_security_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    match client.get_vm(&ns, &name).await {
        Ok(vm) => {
            let domain = &vm.spec.template.spec.domain;
            let mut checks = Vec::new();
            let mut score = 100u32;

            // Check Secure Boot
            let has_secure_boot = domain
                .firmware
                .as_ref()
                .and_then(|f| f.bootloader.as_ref())
                .and_then(|b| b.efi.as_ref())
                .is_some_and(|e| e.secure_boot.unwrap_or(false));
            checks.push(serde_json::json!({
                "name": "Secure Boot",
                "pass": has_secure_boot,
                "detail": if has_secure_boot { "UEFI Secure Boot enabled" } else { "Secure Boot not configured" },
            }));
            if !has_secure_boot {
                score = score.saturating_sub(15);
            }

            // Check TPM
            let has_tpm = domain
                .devices
                .as_ref()
                .and_then(|d| d.tpm.as_ref())
                .is_some();
            checks.push(serde_json::json!({
                "name": "TPM 2.0",
                "pass": has_tpm,
                "detail": if has_tpm { "TPM device attached" } else { "No TPM device" },
            }));
            if !has_tpm {
                score = score.saturating_sub(10);
            }

            // Check RNG
            let has_rng = domain
                .devices
                .as_ref()
                .and_then(|d| d.rng.as_ref())
                .is_some();
            checks.push(serde_json::json!({
                "name": "Hardware RNG",
                "pass": has_rng,
                "detail": if has_rng { "virtio-rng configured" } else { "No RNG device" },
            }));
            if !has_rng {
                score = score.saturating_sub(5);
            }

            // Check eviction strategy
            let has_eviction = vm
                .spec
                .template
                .spec
                .eviction_strategy
                .as_ref()
                .is_some_and(|e| !e.is_empty());
            checks.push(serde_json::json!({
                "name": "Eviction Strategy",
                "pass": has_eviction,
                "detail": if has_eviction { "Live migration on eviction" } else { "No eviction strategy" },
            }));
            if !has_eviction {
                score = score.saturating_sub(10);
            }

            // Check resource limits
            let has_limits = domain
                .resources
                .limits
                .as_ref()
                .is_some_and(|l| !l.is_empty());
            checks.push(serde_json::json!({
                "name": "Resource Limits",
                "pass": has_limits,
                "detail": if has_limits { "CPU/memory limits set" } else { "No resource limits defined" },
            }));
            if !has_limits {
                score = score.saturating_sub(10);
            }

            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/security");
            ok_json(&ApiResponse::success(
                &serde_json::json!({
                    "score": score,
                    "checks": checks,
                }),
                &ctx.request_id,
            ))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

// ── Clone, Pause, Unpause, Migrate, Update, VM Events ──────

#[derive(Deserialize)]
pub(super) struct CloneRequest {
    new_name: String,
}

pub(super) async fn clone_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<CloneRequest>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[
        ("namespace", &ns),
        ("name", &name),
        ("new_name", &req.new_name),
    ]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match VmBackend::for_client(&client)
        .clone_vm(&ns, &name, &req.new_name)
        .await
    {
        Ok(_) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/clone");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": format!("VM '{}' cloned to '{}'", name, req.new_name)}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            log::error!("Clone VM failed: {}", e);
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "CLONE_FAILED", &msg)
        }
    }
}

pub(super) async fn pause_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match VmBackend::for_client(&client).pause(&ns, &name).await {
        Ok(_) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/pause");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": format!("VM '{}' paused", name)}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            let lower = msg.to_lowercase();
            let is_conflict = lower.contains("is not running")
                || lower.contains("no vmi found")
                || lower.ends_with("(conflict)");
            if is_conflict {
                err_json(409, "CONFLICT", &msg)
            } else {
                err_json(status_for_sanitized_error(&msg), "PAUSE_FAILED", &msg)
            }
        }
    }
}

pub(super) async fn unpause_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match VmBackend::for_client(&client).unpause(&ns, &name).await {
        Ok(_) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/unpause");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": format!("VM '{}' unpaused", name)}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            let lower = msg.to_lowercase();
            let is_conflict = lower.contains("is not running")
                || lower.contains("no vmi found")
                || lower.ends_with("(conflict)");
            if is_conflict {
                err_json(409, "CONFLICT", &msg)
            } else {
                err_json(status_for_sanitized_error(&msg), "UNPAUSE_FAILED", &msg)
            }
        }
    }
}

#[derive(Deserialize, Default)]
pub(super) struct MigrateVmRequestBody {
    #[serde(default)]
    target_hostname: Option<String>,
    #[serde(default)]
    added_node_selector: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    priority: Option<String>,
    /// Bypass the eligibility gate (KubeVirt will still enforce its own rules).
    #[serde(default)]
    force: bool,
    /// Kairon backend: `auto` (default) | `live` | `cold`.
    #[serde(default)]
    strategy: Option<String>,
}

/// 409 with the structured blocker list — migration is prevented by VM
/// state (passthrough GPU, not running, KubeVirt LiveMigratable=False),
/// not by a malformed request.
pub(super) fn migration_blocked_json(
    name: &str,
    elig: &crate::kube::migration_guard::MigrationEligibility,
) -> (StatusCode, Json<serde_json::Value>) {
    let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/migrate");
    let summary = elig
        .blockers
        .iter()
        .map(|b| b.message.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    let suggestion = elig.blockers.iter().find_map(|b| b.suggestion.clone());
    let resp = crate::api::ApiResponse::<serde_json::Value> {
        status: 409,
        success: false,
        data: Some(serde_json::json!({
            "eligible": false,
            "blockers": elig.blockers,
            "warnings": elig.warnings,
            "suggestion": suggestion,
        })),
        error: Some(crate::api::ApiError {
            code: "MIGRATION_BLOCKED".to_string(),
            message: format!("VM '{name}' cannot live-migrate: {summary}"),
            details: suggestion,
        }),
        metadata: crate::api::ResponseMetadata {
            request_id: ctx.request_id.clone(),
            timestamp: chrono::Utc::now(),
            duration_ms: None,
        },
    };
    (
        StatusCode::CONFLICT,
        Json(serde_json::to_value(&resp).unwrap_or_default()),
    )
}

pub(super) async fn migrate_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    body: Bytes,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let req: MigrateVmRequestBody = if body.is_empty() {
        MigrateVmRequestBody::default()
    } else {
        serde_json::from_slice(&body).unwrap_or_default()
    };
    if crate::api::vm_backend::selected() != crate::api::vm_backend::BackendKind::KubeVirt {
        let client = { state.read().await.kube_client.clone() };
        let mreq = crate::api::vm_backend::MigrateRequest {
            target_node: req.target_hostname,
            strategy: req.strategy,
            force: req.force,
        };
        return match VmBackend::for_client(&client)
            .migrate(&ns, &name, &mreq)
            .await
        {
            Ok(migration) => {
                let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/migrate");
                ok_json(&ApiResponse::success(
                    &serde_json::json!({
                        "message": format!("Migration initiated for VM '{}'", name),
                        "migration": migration,
                    }),
                    &ctx.request_id,
                ))
            }
            Err(e) => match e.downcast_ref::<crate::api::vm_backend::MigrationBlocked>() {
                Some(b) => err_json(409, "MIGRATION_BLOCKED", &b.0),
                None => {
                    let msg = sanitize_error(&e);
                    err_json(status_for_sanitized_error(&msg), "MIGRATE_FAILED", &msg)
                }
            },
        };
    }
    let mut opts = MigrateVmOptions {
        added_node_selector: req.added_node_selector,
        priority: req.priority,
    };
    if let Some(host) = req.target_hostname {
        let mut m = opts.added_node_selector.take().unwrap_or_default();
        m.insert("kubernetes.io/hostname".to_string(), host);
        opts.added_node_selector = Some(m);
    }
    let mopts = if opts.added_node_selector.is_none() && opts.priority.is_none() {
        None
    } else {
        Some(opts)
    };
    let client = { state.read().await.kube_client.clone() };
    // Eligibility gate: a passthrough-GPU VM (or one KubeVirt itself marks
    // non-migratable) would otherwise enter a doomed migration loop. A
    // failed *probe* never blocks — only a definitive "not eligible" does.
    if !req.force {
        if let Ok(elig) = client
            .migration_eligibility(
                &ns,
                &name,
                crate::kube::migration_guard::ClusterMigrationCaps::from_env(),
            )
            .await
        {
            if !elig.eligible {
                return migration_blocked_json(&name, &elig);
            }
        }
    }
    match client.migrate_vm(&ns, &name, mopts).await {
        Ok(_) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/migrate");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": format!("Migration initiated for VM '{}'", name)}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "MIGRATE_FAILED", &msg)
        }
    }
}

pub(super) async fn list_vm_migrations_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match VmBackend::for_client(&client)
        .list_migrations(&ns, &name)
        .await
    {
        Ok(list) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/migrations");
            ok_json(&ApiResponse::success(&list, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

pub(super) async fn delete_vm_migration_handler(
    State(state): State<SharedState>,
    Path((ns, name, migname)): Path<(String, String, String)>,
) -> impl IntoResponse {
    if let Some(resp) =
        validate_k8s_params(&[("namespace", &ns), ("name", &name), ("migname", &migname)])
    {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match VmBackend::for_client(&client)
        .cancel_migration(&ns, &migname)
        .await
    {
        Ok(()) => {
            let ctx = req_ctx(
                HttpMethod::DELETE,
                "/api/v1/vms/:ns/:name/migrations/:migname",
            );
            ok_json(&ApiResponse::success(
                &serde_json::json!({"deleted": migname}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "DELETE_FAILED", &msg)
        }
    }
}

pub(super) async fn guest_freeze_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match client.guest_freeze(&ns, &name).await {
        Ok(()) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/guest/freeze");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": "Guest filesystem freeze requested"}),
                &ctx.request_id,
            ))
        }
        Err(e) => err_json(500, "GUEST_FREEZE_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_unfreeze_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match client.guest_unfreeze(&ns, &name).await {
        Ok(()) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/guest/unfreeze");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": "Guest filesystem unfreeze requested"}),
                &ctx.request_id,
            ))
        }
        Err(e) => err_json(500, "GUEST_UNFREEZE_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn guest_softreboot_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match client.guest_soft_reboot(&ns, &name).await {
        Ok(()) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/guest/softreboot");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": "Guest soft reboot requested"}),
                &ctx.request_id,
            ))
        }
        Err(e) => err_json(500, "GUEST_SOFTREBOOT_FAILED", &sanitize_error(&e)),
    }
}

pub(super) async fn vm_volume_status_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match client.get_vmi_volume_status_json(&ns, &name).await {
        Ok(v) => {
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/volumes/status");
            ok_json(&ApiResponse::success(&v, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

#[derive(Deserialize)]
pub(super) struct HotplugVolumeBody {
    volume_name: String,
    pvc_name: String,
}

pub(super) async fn vm_data_disk_defaults_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match client.data_disk_defaults(&ns, &name).await {
        Ok(body) => {
            let ctx = req_ctx(
                HttpMethod::GET,
                "/api/v1/vms/:ns/:name/storage/data-disk/defaults",
            );
            ok_json(&ApiResponse::success(&body, &ctx.request_id))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(
                status_for_sanitized_error(&msg),
                "DATA_DISK_DEFAULTS_FAILED",
                &msg,
            )
        }
    }
}

pub(super) async fn vm_add_data_disk_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<crate::kube::vm_data_disk::AddDataDiskRequest>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match client.add_data_disk(&ns, &name, req).await {
        Ok(body) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/storage/data-disk");
            ok_json(&ApiResponse::success(&body, &ctx.request_id))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(
                status_for_sanitized_error(&msg),
                "ADD_DATA_DISK_FAILED",
                &msg,
            )
        }
    }
}

pub(super) async fn vm_hotplug_volume_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<HotplugVolumeBody>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    // virtctl's addvolume has no way to give the hotplugged volume an
    // internal label distinct from the PVC it references (see
    // add_vm_volume) — `--volume-name` must literally be the PVC's own
    // name. Reject up front rather than silently ignoring volume_name.
    if req.volume_name != req.pvc_name {
        return err_json(
            400,
            "INVALID_VOLUME_NAME",
            "volume_name must equal pvc_name — virtctl addvolume can only reference a \
             volume by its actual PVC name, it has no separate internal-label flag",
        );
    }
    let client = { state.read().await.kube_client.clone() };
    match client.add_vm_volume(&ns, &name, &req.pvc_name).await {
        Ok(()) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/volumes/hotplug");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": "virtctl addvolume invoked"}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "HOTPLUG_FAILED", &msg)
        }
    }
}

#[derive(Deserialize)]
pub(super) struct HotremoveVolumeBody {
    volume_name: String,
}

pub(super) async fn vm_hotremove_volume_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<HotremoveVolumeBody>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match client.remove_vm_volume(&ns, &name, &req.volume_name).await {
        Ok(()) => {
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/:ns/:name/volumes/hotremove");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": "virtctl removevolume invoked"}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "HOTREMOVE_FAILED", &msg)
        }
    }
}

pub(super) async fn vm_serial_console_hint_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let _ = state;
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/console/serial");
    ok_json(&ApiResponse::success(
        &serde_json::json!({
            "virtctl": format!("virtctl console {} -n {}", name, ns),
            "kubectl": format!("kubectl console -n {} vmi/{}  (if your cluster provides the kubectl console plugin)", ns, name),
            "websocket": format!("/api/v1/vms/{}/{}/serial?ticket=<ticket from POST /api/v1/ws/ticket> (same host as dashboard; subprotocol binary)", ns, name),
            "note": "Interactive serial uses the dashboard Serial button or this WebSocket URL; requires a running VMI."
        }),
        &ctx.request_id,
    ))
}

#[derive(Deserialize)]
pub(super) struct UpdateVmRequest {
    cpus: Option<u32>,
    memory: Option<String>,
}

pub(super) async fn update_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<UpdateVmRequest>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };
    match client
        .update_vm_resources(&ns, &name, req.cpus, req.memory.as_deref())
        .await
    {
        Ok(_) => {
            let ctx = req_ctx(HttpMethod::PUT, "/api/v1/vms/:ns/:name");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": format!("VM '{}' updated", name)}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "UPDATE_FAILED", &msg)
        }
    }
}

pub(super) async fn vm_events_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }
    let client = { state.read().await.kube_client.clone() };

    use k8s_openapi::api::core::v1::Event;
    use kube::Api;
    let events_api: Api<Event> = Api::namespaced(client.client(), &ns);
    let lp = kube::api::ListParams::default().limit(100);
    match events_api.list(&lp).await {
        Ok(event_list) => {
            let items: Vec<EventItem> = event_list
                .items
                .into_iter()
                .filter(|e| e.involved_object.name.as_deref() == Some(&name))
                .map(|e| EventItem {
                    type_: e.type_.unwrap_or_default(),
                    reason: e.reason.unwrap_or_default(),
                    message: e.message.unwrap_or_default(),
                    namespace: e.metadata.namespace.unwrap_or_default(),
                    involved_object: e.involved_object.name.unwrap_or_default(),
                    timestamp: e
                        .last_timestamp
                        .map(|t| t.0.to_rfc3339())
                        .or_else(|| e.metadata.creation_timestamp.map(|t| t.0.to_rfc3339()))
                        .unwrap_or_default(),
                })
                .collect();
            let ctx = req_ctx(HttpMethod::GET, "/api/v1/vms/:ns/:name/events");
            ok_json(&ApiResponse::success(&items, &ctx.request_id))
        }
        Err(e) => err_json(500, "INTERNAL_ERROR", &sanitize_error(&e)),
    }
}

// ── Storage Endpoints ────────────────────────────────────────

pub(super) async fn delete_vm_handler(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("namespace", &ns), ("name", &name)]) {
        return resp;
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    match VmBackend::for_client(&client).delete(&ns, &name).await {
        Ok(_) => {
            let ctx = req_ctx(HttpMethod::DELETE, "/api/v1/vms/:ns/:name");
            ok_json(&ApiResponse::success(
                &serde_json::json!({"message": format!("VM '{}' deleted", name)}),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            err_json(status_for_sanitized_error(&msg), "DELETE_FAILED", &msg)
        }
    }
}

// ── Batch VM Operations ─────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct BatchVmRequest {
    action: String,
    vms: Vec<BatchVmTarget>,
}

#[derive(Deserialize)]
pub(super) struct BatchVmTarget {
    namespace: String,
    name: String,
}

pub(super) async fn batch_vm_handler(
    State(state): State<SharedState>,
    Json(req): Json<BatchVmRequest>,
) -> impl IntoResponse {
    // Limit batch size to prevent K8s API abuse
    const MAX_BATCH_SIZE: usize = 50;
    if req.vms.len() > MAX_BATCH_SIZE {
        return err_json(
            400,
            "BATCH_TOO_LARGE",
            &format!(
                "Batch size {} exceeds maximum of {}",
                req.vms.len(),
                MAX_BATCH_SIZE
            ),
        );
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    let mut results = Vec::new();

    for vm in &req.vms {
        if !is_valid_k8s_name(&vm.name) || !is_valid_k8s_name(&vm.namespace) {
            results.push(serde_json::json!({
                "name": vm.name,
                "namespace": vm.namespace,
                "status": "error",
                "message": "Invalid Kubernetes name"
            }));
            continue;
        }

        let result: Result<(), anyhow::Error> = match req.action.as_str() {
            "start" => client.start_vm(&vm.namespace, &vm.name).await.map(|_| ()),
            "stop" => client.stop_vm(&vm.namespace, &vm.name).await.map(|_| ()),
            "restart" => client.restart_vm(&vm.namespace, &vm.name).await.map(|_| ()),
            "delete" => client.delete_vm(&vm.namespace, &vm.name).await,
            _ => {
                results.push(serde_json::json!({
                    "name": vm.name,
                    "namespace": vm.namespace,
                    "status": "error",
                    "message": format!("Unknown action: {}", req.action)
                }));
                continue;
            }
        };

        match result {
            Ok(()) => results.push(serde_json::json!({
                "name": vm.name,
                "namespace": vm.namespace,
                "status": "success",
                "message": format!("VM '{}' {}ed", vm.name, req.action)
            })),
            Err(e) => results.push(serde_json::json!({
                "name": vm.name,
                "namespace": vm.namespace,
                "status": "error",
                "message": sanitize_error(&e)
            })),
        }
    }

    let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms/batch");
    ok_json(&ApiResponse::success(
        &serde_json::json!({"results": results}),
        &ctx.request_id,
    ))
}

// ── Create VM Endpoint ────────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct CreateVmRequest {
    name: String,
    namespace: Option<String>,
    template: Option<String>,
    profile: Option<String>,
    cpus: Option<u32>,
    memory: Option<String>,
    disk_size: Option<String>,
    cloud_init: Option<String>,
    start: Option<bool>,
    /// When true (default), create a CiliumNetworkPolicy or Kubernetes NetworkPolicy so the VM's virt-launcher pods can egress broadly (internet-friendly under default-deny Cilium).
    #[serde(default = "default_allow_internet")]
    allow_internet: bool,
    /// `nocloud` (default) or `configdrive` for `cloudInitConfigDrive` + Cloudbase-Init.
    #[serde(default)]
    cloud_init_delivery: Option<String>,
    #[serde(default)]
    expose: Option<CreateVmExposeReq>,
    /// Boot this VM from a golden image: the root disk is cloned from the
    /// catalog rather than created blank. Without it, a Windows template
    /// boots an empty disk and there is nothing to install from.
    #[serde(default)]
    image: Option<CreateVmImageReq>,
    /// Kairon: install from this catalog ISO (`MachineImage` kind iso) onto a
    /// blank root disk of `disk_size` (default: the template's, else 60Gi).
    #[serde(default)]
    iso: Option<String>,
    /// Kairon: second CD-ROM with drivers (kind iso). Windows templates default
    /// to `virtio-win` when that image exists; `""` attaches none.
    #[serde(default)]
    driver_iso: Option<String>,
    /// Kairon: PVC (Filesystem mode, empty) to hold the root disk; seeded from
    /// `image` or left blank for an `iso` install.
    #[serde(default)]
    root_volume: Option<String>,
    /// Windows unattended setup: Secret (preferred) or ConfigMap holding
    /// `autounattend.xml`. Mounted as sysprep CD-ROM media.
    #[serde(default)]
    sysprep_secret: Option<String>,
    #[serde(default)]
    sysprep_config_map: Option<String>,
    /// Explicit GPU devices (KubeVirt `domain.devices.gpus`); wins over `gpu`.
    #[serde(default)]
    gpus: Option<Vec<CreateVmGpuReq>>,
    /// GPU shorthand: `{"gpu": {"count": 1}}` attaches N `nvidia.com/gpu` devices.
    #[serde(default)]
    gpu: Option<CreateVmGpuSugarReq>,
    /// Skip the "does any node advertise this GPU resource" preflight.
    #[serde(default)]
    force: bool,
}

#[derive(Deserialize)]
pub(super) struct CreateVmGpuReq {
    /// Device alias inside the VM spec (default `gpu<N>`).
    #[serde(default)]
    name: Option<String>,
    /// Device-plugin resource, e.g. `nvidia.com/gpu` or `nvidia.com/GRID_T4-2Q`.
    device_name: String,
}

#[derive(Deserialize)]
pub(super) struct CreateVmGpuSugarReq {
    #[serde(default = "default_gpu_count")]
    count: u32,
    /// Device-plugin resource (default `nvidia.com/gpu`).
    #[serde(default)]
    resource_name: Option<String>,
    /// vGPU mediated-device profile; `GRID_T4-2Q` → `nvidia.com/GRID_T4-2Q`.
    /// Requires a vGPU-configured cluster (Phase 2).
    #[serde(default)]
    vgpu_profile: Option<String>,
}

pub(super) fn default_gpu_count() -> u32 {
    1
}

/// Golden-image source for a new VM's root disk.
#[derive(Deserialize)]
pub(super) struct CreateVmImageReq {
    /// CDI `DataSource` name (default), or a source PVC name when `from_pvc`.
    name: String,
    /// Namespace holding the catalog (default: `vm-images`).
    #[serde(default)]
    namespace: Option<String>,
    /// Clone directly from a PVC instead of resolving a `DataSource`.
    #[serde(default)]
    from_pvc: bool,
    /// Storage class for the clone; defaults to the cluster default class.
    #[serde(default)]
    storage_class: Option<String>,
    /// `Filesystem` (default) or `Block` for the clone. Defaults to Filesystem to
    /// match golden images and get the fast native CSI clone.
    #[serde(default)]
    volume_mode: Option<String>,
    /// Auto-size the VM from the image's instancetype/preference labels
    /// (`inferFromVolume`). Default true; ignored if `profile`/`cpus` are given.
    #[serde(default = "default_true")]
    infer: bool,
}

#[cfg(feature = "kairon")]
use crate::api::handlers::machine_images::{KIND_DISK, KIND_ISO};

/// 400 unless `name` is a Kairon `MachineImage` of `kind`.
#[cfg(feature = "kairon")]
async fn check_kairon_catalog_image(
    state: &SharedState,
    name: &str,
    kind: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    use crate::api::handlers::machine_images::{catalog_kind, get_catalog_image};
    let client = state.read().await.kube_client.client();
    match get_catalog_image(&client, name).await {
        Ok(Some(img)) if catalog_kind(&img) == kind => Ok(()),
        Ok(Some(img)) => Err(err_json(
            400,
            "INVALID_IMAGE",
            &format!(
                "MachineImage '{name}' is kind {}, expected {kind}",
                catalog_kind(&img)
            ),
        )),
        Ok(None) => Err(err_json(
            400,
            "INVALID_IMAGE",
            &format!(
                "MachineImage '{name}' not found; publish it with POST /api/v1/machine-images"
            ),
        )),
        Err(e) => Err(err_json(500, "INTERNAL_ERROR", &sanitize_error(&e))),
    }
}

/// Kairon install media and PVC root disk: `iso` turns the root into a blank
/// disk with the ISO (and drivers) attached; `root_volume` puts the root disk
/// on a PVC that Kairon seeds.
#[cfg(feature = "kairon")]
async fn apply_kairon_install_media(
    state: &SharedState,
    req: &CreateVmRequest,
    config: &mut crate::config::VMConfig,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    use crate::config::{DiskConfig, DiskDeviceType, DiskSource};
    if !crate::api::vm_backend::is_kairon() {
        return Err(err_json(
            400,
            "KAIRON_REQUIRED",
            "iso, driver_iso and root_volume need the Kairon backend; on KubeVirt upload \
             install media with POST /api/v1/images/upload",
        ));
    }
    let catalog_cd = |name: &str, iso: &str| DiskConfig {
        name: name.into(),
        size: "0".into(),
        storage_class: None,
        boot_order: 0,
        source: DiskSource::GoldenImage {
            name: iso.into(),
            namespace: String::new(),
            from_pvc: false,
            storage_class: None,
            volume_mode: None,
        },
        device_type: DiskDeviceType::CDROM,
        bus: Some("sata".into()),
        cache: None,
        io: None,
    };
    if let Some(iso) = req.iso.as_deref() {
        if req.image.is_some() {
            return Err(err_json(
                400,
                "INVALID_IMAGE",
                "image and iso are mutually exclusive",
            ));
        }
        check_kairon_catalog_image(state, iso, KIND_ISO).await?;
        let Some(root) = config
            .disks
            .iter_mut()
            .find(|d| d.device_type == DiskDeviceType::Disk)
        else {
            return Err(err_json(
                400,
                "INVALID_IMAGE",
                "template has no root disk to install onto",
            ));
        };
        root.source = DiskSource::Blank;
        if let Some(ds) = &req.disk_size {
            root.size = ds.clone();
        }
        config.disks.push(catalog_cd("install", iso));

        let windows = req
            .template
            .as_deref()
            .is_some_and(|t| t.starts_with("windows"));
        let driver = match req.driver_iso.as_deref() {
            Some("") => None,
            Some(d) => Some((d, true)),
            None if windows => Some(("virtio-win", false)),
            None => None,
        };
        if let Some((d, explicit)) = driver {
            match check_kairon_catalog_image(state, d, KIND_ISO).await {
                Ok(()) => config.disks.push(catalog_cd("drivers", d)),
                Err(e) if explicit => return Err(e),
                Err(_) => {}
            }
        }
    } else if req.driver_iso.as_deref().is_some_and(|d| !d.is_empty()) {
        return Err(err_json(400, "INVALID_IMAGE", "driver_iso needs iso"));
    }
    if let Some(claim) = req.root_volume.as_deref().filter(|c| !c.is_empty()) {
        config.annotations.insert(
            crate::kairon::converter::ROOT_VOLUME_ANNOTATION.to_string(),
            claim.to_string(),
        );
    }
    Ok(())
}

#[cfg(not(feature = "kairon"))]
async fn apply_kairon_install_media(
    _state: &SharedState,
    _req: &CreateVmRequest,
    _config: &mut crate::config::VMConfig,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    Err(err_json(
        400,
        "KAIRON_REQUIRED",
        "iso, driver_iso and root_volume need a Kairon-enabled build",
    ))
}

/// Namespace holding the golden-image catalog when the caller doesn't say.
pub(super) const DEFAULT_IMAGE_NAMESPACE: &str = "vm-images";

/// Resolve the actual size of a golden image's backing PVC, so a caller
/// who omits `disk_size` doesn't inherit the template's container-disk
/// placeholder ("0" — see `add_container_disk`, "container disks don't
/// need size") as the clone's target size. CDI rejects a zero-or-less
/// storage request outright ("Storage size can't be equal or less than
/// zero"), and separately requires the clone target to be >= the source,
/// so matching the source's own size is the only value that always works.
pub(super) async fn resolve_golden_image_size(
    client: &kube::Client,
    name: &str,
    namespace: &str,
    from_pvc: bool,
) -> Option<String> {
    use kube::Api;
    use kube::api::{ApiResource, DynamicObject};
    let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), namespace);
    let (pvc_name, pvc_ns) = if from_pvc {
        (name.to_string(), namespace.to_string())
    } else {
        let ar = ApiResource {
            group: "cdi.kubevirt.io".to_string(),
            version: "v1beta1".to_string(),
            api_version: "cdi.kubevirt.io/v1beta1".to_string(),
            kind: "DataSource".to_string(),
            plural: "datasources".to_string(),
        };
        let ds_api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace, &ar);
        let ds = ds_api.get(name).await.ok()?;
        let src = ds.data.get("spec")?.get("source")?.get("pvc")?;
        let src_name = src.get("name")?.as_str()?.to_string();
        let src_ns = src
            .get("namespace")
            .and_then(|n| n.as_str())
            .unwrap_or(namespace)
            .to_string();
        (src_name, src_ns)
    };
    let pvc_api: Api<PersistentVolumeClaim> = if pvc_ns == namespace {
        pvc_api
    } else {
        Api::namespaced(client.clone(), &pvc_ns)
    };
    let pvc = pvc_api.get(&pvc_name).await.ok()?;
    Some(pvc.spec?.resources?.requests?.get("storage")?.0.clone())
}

pub(super) fn default_allow_internet() -> bool {
    true
}

#[derive(Deserialize)]
pub(super) struct CreateVmExposeReq {
    #[serde(default)]
    enabled: bool,
    service_type: Option<String>,
    #[serde(default)]
    ports: Vec<CreateVmExposePortReq>,
}

#[derive(Deserialize)]
pub(super) struct CreateVmExposePortReq {
    name: Option<String>,
    port: i32,
    target_port: Option<i32>,
    protocol: Option<String>,
    /// External node port for NodePort/LoadBalancer (must be 30000–32767).
    node_port: Option<i32>,
}

/// Maps dashboard/API expose payload to `VmExposeConfig` when enabled.
pub(super) fn vm_expose_from_api_req(ex: CreateVmExposeReq) -> Option<VmExposeConfig> {
    if !ex.enabled {
        return None;
    }
    let mut ports: Vec<VmExposePort> = ex
        .ports
        .into_iter()
        .map(|p| VmExposePort {
            name: p.name,
            port: p.port,
            target_port: p.target_port.unwrap_or(p.port),
            protocol: p.protocol.unwrap_or_else(|| "TCP".to_string()),
            node_port: p.node_port,
        })
        .collect();
    if ports.is_empty() {
        ports.push(VmExposePort {
            name: Some("ssh".to_string()),
            port: 22,
            target_port: 22,
            protocol: "TCP".to_string(),
            node_port: None,
        });
    }
    Some(VmExposeConfig {
        enabled: true,
        service_type: ex.service_type.unwrap_or_else(|| "ClusterIP".to_string()),
        ports,
    })
}

pub(super) async fn create_vm_handler(
    State(state): State<SharedState>,
    Json(req): Json<CreateVmRequest>,
) -> impl IntoResponse {
    if let Some(resp) = validate_k8s_params(&[("name", &req.name)]) {
        return resp;
    }

    let ns = req.namespace.as_deref().unwrap_or("default");
    if let Some(resp) = validate_k8s_params(&[("namespace", ns)]) {
        return resp;
    }

    // Start with template or blank config
    let mut builder = if let Some(ref tpl_name) = req.template {
        if let Some(tpl_config) = TEMPLATES.get(tpl_name) {
            let mut b = VMConfigBuilder::new(&req.name);
            b = b.namespace(ns);
            b = b.cpu(
                tpl_config.cpu.cores,
                tpl_config.cpu.sockets,
                tpl_config.cpu.threads,
            );
            b = b.memory(&tpl_config.memory.size);
            // Preserve disk types from template (containerDisk, blank, etc.)
            for disk in &tpl_config.disks {
                b = b.add_disk(disk.clone());
            }
            // Preserve network interfaces from template
            for iface in &tpl_config.interfaces {
                b = b.add_interface(iface.clone());
            }
            // Preserve cloud-init, features, clock from template
            if let Some(ref ci) = tpl_config.cloud_init {
                b = b.cloud_init(crate::templates::refresh_guestkit_cloud_init(&ci.user_data));
            }
            if tpl_config.enable_rng {
                b = b.enable_rng();
            }
            if let Some(ref clock) = tpl_config.clock {
                b = b.clock(clock.clone());
            }
            if let Some(ref features) = tpl_config.features {
                b = b.features(features.clone());
            }
            if let Some(ref firmware) = tpl_config.firmware {
                b = b.firmware(firmware.clone());
            }
            // Preserve labels from template
            b = b.labels(tpl_config.labels.clone());
            b
        } else {
            return err_json(
                400,
                "INVALID_TEMPLATE",
                &format!("Template '{}' not found", tpl_name),
            );
        }
    } else {
        let mut b = VMConfigBuilder::new(&req.name);
        b = b.namespace(ns);
        b = b.add_blank_disk("rootdisk", "10Gi", 1);
        b = b.add_pod_network("default");
        b
    };

    // Apply profile defaults (before manual overrides)
    if let Some(ref profile_name) = req.profile {
        if let Ok(profiles) = PROFILES.read() {
            if let Some(profile) = profiles.get(profile_name) {
                builder = builder.cpu(profile.cpu_cores, profile.cpu_sockets, profile.cpu_threads);
                builder = builder.memory(&profile.memory);
                // Profile disk size applied below via config.disks
            }
        }
    }

    // Apply manual overrides (take priority over profile)
    if let Some(cpus) = req.cpus {
        builder = builder.cpu(cpus, 1, 1);
    }
    if let Some(ref mem) = req.memory {
        builder = builder.memory(mem);
    }

    // Apply cloud-init
    if let Some(ref ci) = req.cloud_init {
        if !ci.is_empty() {
            builder = builder.merge_cloud_init(ci);
        }
    }

    let mut config = builder.build();

    // Override disk size if specified
    let mut disk_size_explicit = false;
    if let Some(ref ds) = req.disk_size {
        if let Some(disk) = config.disks.first_mut() {
            disk.size = ds.clone();
        }
        disk_size_explicit = true;
    } else if let Some(ref profile_name) = req.profile {
        // Apply profile disk size if no explicit override
        if let Ok(profiles) = PROFILES.read() {
            if let Some(profile) = profiles.get(profile_name) {
                if let Some(disk) = config.disks.first_mut() {
                    disk.size = profile.disk_size.clone();
                }
                disk_size_explicit = true;
            }
        }
    }

    // Golden image: repoint the root disk at the catalog so the VM boots a clone
    // of an installed OS instead of an empty disk. Applied after sizing, since the
    // clone must be at least as large as the source.
    if let Some(ref img) = req.image {
        if img.name.is_empty() {
            return err_json(400, "INVALID_IMAGE", "image.name cannot be empty");
        }
        let img_ns = img
            .namespace
            .clone()
            .unwrap_or_else(|| DEFAULT_IMAGE_NAMESPACE.to_string());
        // Most templates' root disk starts as a containerDisk, whose size
        // defaults to the placeholder "0" ("container disks don't need
        // size" — see add_container_disk). Left as-is, that "0" gets sent
        // to CDI verbatim as the clone's target size and is rejected
        // ("Storage size can't be equal or less than zero"). If the
        // caller didn't pin a size explicitly, infer it from the source
        // PVC instead of silently shipping an unusable "0" — CDI also
        // requires the clone target to be >= the source, so matching it
        // is the only value that's always valid.
        #[cfg(feature = "kairon")]
        if crate::api::vm_backend::is_kairon() {
            if let Err(resp) = check_kairon_catalog_image(&state, &img.name, KIND_DISK).await {
                return resp;
            }
            // Kairon sizes from the image (or its MachineImage defaults); a
            // template's size would be sent as diskSize and may be too small.
            if !disk_size_explicit {
                if let Some(disk) = config.disks.first_mut() {
                    disk.size = "0".into();
                }
            }
        }
        if !disk_size_explicit && !crate::api::vm_backend::is_kairon() {
            let client = { state.read().await.kube_client.client() };
            match resolve_golden_image_size(&client, &img.name, &img_ns, img.from_pvc).await {
                Some(size) => {
                    if let Some(disk) = config.disks.first_mut() {
                        disk.size = size;
                    }
                }
                None => {
                    return err_json(
                        400,
                        "INVALID_IMAGE",
                        &format!(
                            "could not resolve a size for image '{}' in namespace '{}' \
                             (DataSource/source PVC not found or not yet bound) — pass \
                             disk_size explicitly",
                            img.name, img_ns
                        ),
                    );
                }
            }
        }
        let Some(root) = config.disks.first_mut() else {
            return err_json(
                400,
                "INVALID_IMAGE",
                "template has no disks to apply a golden image to",
            );
        };
        root.source = DiskSource::GoldenImage {
            name: img.name.clone(),
            namespace: img
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_IMAGE_NAMESPACE.to_string()),
            from_pvc: img.from_pvc,
            storage_class: img.storage_class.clone(),
            volume_mode: img.volume_mode.clone(),
        };
        // A cloned disk boots on its own; an install-media CD-ROM must not
        // out-rank it or the VM reruns Setup on every boot.
        root.boot_order = 1;

        // Auto-size from the image's labels unless the caller pinned a size.
        // The published DataSource carries default-instancetype/preference; a VM
        // that infers from its root volume picks them up, so one image serves any
        // consumer without restating CPU/mem. Explicit profile/cpus/memory win.
        let root_name = root.name.clone();
        let sized_explicitly = req.profile.is_some() || req.cpus.is_some() || req.memory.is_some();
        if img.infer && !sized_explicitly {
            config.instancetype = Some(crate::config::VmMatcherRef {
                name: String::new(),
                kind: None,
                infer_from_volume: Some(root_name.clone()),
            });
            config.preference = Some(crate::config::VmMatcherRef {
                name: String::new(),
                kind: None,
                infer_from_volume: Some(root_name),
            });
        }
    }

    if req.iso.is_some() || req.root_volume.is_some() || req.driver_iso.is_some() {
        if let Err(resp) = apply_kairon_install_media(&state, &req, &mut config).await {
            return resp;
        }
    }

    // Windows unattended setup media.
    if req.sysprep_secret.is_some() || req.sysprep_config_map.is_some() {
        config.sysprep = Some(crate::config::SysprepConfig {
            secret: req.sysprep_secret.clone(),
            config_map: req.sysprep_config_map.clone(),
        });
    }

    if let Some(ref d) = req.cloud_init_delivery {
        if let Some(ref mut ci) = config.cloud_init {
            ci.delivery = match d.to_ascii_lowercase().as_str() {
                "configdrive" | "config_drive" => CloudInitDelivery::ConfigDrive,
                _ => CloudInitDelivery::NoCloud,
            };
        }
    }

    if let Some(ex) = req.expose {
        if let Some(cfg) = vm_expose_from_api_req(ex) {
            config.expose = Some(cfg);
        }
    }

    // GPU attachment: an explicit device list wins; the `gpu` shorthand
    // expands to N identical devices. The converter defaults KVM-hidden on
    // for NVIDIA resources so guest drivers load.
    if let Some(ref gpu_list) = req.gpus {
        for (i, g) in gpu_list.iter().enumerate() {
            if g.device_name.is_empty() {
                return err_json(400, "INVALID_GPU", "gpus[].device_name cannot be empty");
            }
            config.gpus.push(crate::config::VmGpuDevice {
                name: g.name.clone().unwrap_or_else(|| format!("gpu{i}")),
                device_name: g.device_name.clone(),
                virtual_gpu_options: None,
            });
        }
    } else if let Some(ref g) = req.gpu {
        let resource = match (&g.resource_name, &g.vgpu_profile) {
            (Some(r), _) => r.clone(),
            (None, Some(profile)) => format!("nvidia.com/{profile}"),
            (None, None) => "nvidia.com/gpu".to_string(),
        };
        for i in 0..g.count.max(1) {
            config.gpus.push(crate::config::VmGpuDevice {
                name: format!("gpu{i}"),
                device_name: resource.clone(),
                virtual_gpu_options: None,
            });
        }
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };

    // GPU preflight: a VM asking for a resource no node advertises would
    // sit Pending forever with an opaque scheduler message. Fail fast with
    // what IS available; `force: true` skips (e.g. node about to join).
    if !config.gpus.is_empty() && !req.force {
        use k8s_openapi::api::core::v1::Node;
        let node_api: kube::Api<Node> = kube::Api::all(client.client().clone());
        if let Ok(nodes) = node_api.list(&Default::default()).await {
            let advertised = |vm_only: bool| -> std::collections::BTreeSet<String> {
                nodes
                    .items
                    .iter()
                    .flat_map(|n| {
                        if vm_only {
                            crate::kube::gpu_inventory::node_vm_gpu_resources(n)
                        } else {
                            crate::kube::gpu_inventory::node_gpu_resources(n)
                        }
                    })
                    .filter(|r| r.allocatable > 0)
                    .map(|r| r.name)
                    .collect()
            };
            let available = advertised(true);
            let anywhere = advertised(false);
            let missing: Vec<&str> = config
                .gpus
                .iter()
                .map(|g| g.device_name.as_str())
                .filter(|d| !available.contains(*d))
                .collect();
            let container_only: Vec<&str> = missing
                .iter()
                .copied()
                .filter(|d| anywhere.contains(*d))
                .collect();
            if !container_only.is_empty() {
                return err_json(
                    422,
                    "GPU_RESERVED_FOR_CONTAINERS",
                    &format!(
                        "{} is only advertised on nodes whose NVIDIA GPU Operator workload is \
                         `container` (GPUs reserved for pods, e.g. Gryvia jobs); label a node \
                         nvidia.com/gpu.workload.config=vm-passthrough to give its GPUs to VMs, \
                         or pass \"force\": true to override",
                        container_only.join(", ")
                    ),
                );
            }
            if !missing.is_empty() {
                let avail_msg = if available.is_empty() {
                    "no node advertises any GPU resource — run \
                     scripts/cluster/enable-gpu-passthrough.sh or install a GPU device plugin"
                        .to_string()
                } else {
                    format!(
                        "available GPU resources: {}",
                        available.into_iter().collect::<Vec<_>>().join(", ")
                    )
                };
                return err_json(
                    422,
                    "GPU_RESOURCE_UNAVAILABLE",
                    &format!(
                        "no node advertises {} ({avail_msg}); pass \"force\": true to override",
                        missing.join(", ")
                    ),
                );
            }
        }
    }

    // Evaluate VeyronPolicy CRDs before creation
    if let Ok(violations) = check_policies(&client, &config).await {
        if !violations.is_empty() {
            let deny_violations: Vec<_> = violations
                .iter()
                .filter(|v| v.enforcement == "Deny")
                .collect();
            if !deny_violations.is_empty() {
                let messages: Vec<String> = deny_violations
                    .iter()
                    .map(|v| format!("[{}] {}", v.policy_name, v.message))
                    .collect();
                return err_json(403, "POLICY_VIOLATION", &messages.join("; "));
            }
            // Warn-level violations are logged but don't block creation
            for v in violations.iter().filter(|v| v.enforcement == "Warn") {
                log::warn!(
                    "Policy warning for VM '{}': [{}] {}",
                    req.name,
                    v.policy_name,
                    v.message
                );
            }
        }
    }

    let backend = VmBackend::for_client(&client);
    match backend.create(&config).await {
        Ok(_) => {
            // Auto-start if requested
            let started = if req.start.unwrap_or(false) {
                backend.start(ns, &req.name).await.is_ok()
            } else {
                false
            };
            // The egress policy selects virt-launcher pods, which only exist on KubeVirt.
            let internet = if req.allow_internet && backend.kind() == BackendKind::KubeVirt {
                match vm_internet::ensure_vm_internet_egress(&client.client(), ns, &req.name).await
                {
                    Ok(st) => serde_json::to_value(&st).unwrap_or(serde_json::Value::Null),
                    Err(e) => {
                        log::warn!(
                            "VM '{}' created but internet egress policy failed: {}",
                            req.name,
                            e
                        );
                        serde_json::json!({
                            "applied": false,
                            "error": sanitize_error(&e),
                        })
                    }
                }
            } else {
                serde_json::json!({ "applied": false, "skipped": true })
            };
            let ctx = req_ctx(HttpMethod::POST, "/api/v1/vms");
            ok_json(&ApiResponse::success(
                &serde_json::json!({
                    "message": format!("VM '{}' created{}", req.name, if started { " and started" } else { "" }),
                    "name": req.name,
                    "namespace": ns,
                    "started": started,
                    "internet": internet,
                }),
                &ctx.request_id,
            ))
        }
        Err(e) => {
            let msg = sanitize_error(&e);
            let msg_lower = msg.to_lowercase();
            if msg.contains("already exists") || msg.contains("conflict") {
                err_json(
                    409,
                    "VM_EXISTS",
                    &format!("VM '{}' already exists", req.name),
                )
            } else if msg_lower.contains("forbidden") || msg_lower.contains("exceeded quota") {
                err_json(403, "CREATE_FORBIDDEN", &msg)
            } else {
                err_json(500, "CREATE_FAILED", &msg)
            }
        }
    }
}

// ── Templates Endpoint ───────────────────────────────────────
