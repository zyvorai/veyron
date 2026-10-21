// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! OCI container Machines (`spec.sandbox.oci`): run an image in its own
//! lightweight VM on a FluxVM node and read back the process state
//! (`status.container`). Kairon backend only.
//!
//! FluxVM's container track runs on macOS hosts only and has no exec/files
//! yet, so this is a job/service API, not an agent-sandbox backend: new
//! containers are pinned to `kubernetes.io/os=darwin` unless the caller
//! sets `node_selector`.

// The Err side is a ready-to-send HTTP response, returned straight to the client.
#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use kube::api::{Api, ApiResource, DeleteParams, DynamicObject, ListParams, PostParams};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::http_server::web::SharedState;
use crate::kairon::{LABEL_MANAGED_BY, Machine, OciHealthCheck, OciSpec, SandboxSpec};

const LABEL_KIND: &str = "veyron.io/kind";
const KIND_CONTAINER: &str = "container";

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/containers", get(list).post(create))
        .route("/containers/:ns/:name", get(show).delete(remove))
        .with_state(state)
}

fn err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "error": msg.into() }))).into_response()
}

fn kube_err(e: kube::Error) -> Response {
    match &e {
        kube::Error::Api(ae) => {
            let code = match ae.code {
                404 => StatusCode::NOT_FOUND,
                409 => StatusCode::CONFLICT,
                400 | 403 | 422 => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::BAD_GATEWAY,
            };
            err(code, ae.message.clone())
        }
        _ => err(StatusCode::BAD_GATEWAY, e.to_string()),
    }
}

async fn client(state: &SharedState) -> Result<kube::Client, Response> {
    if !crate::api::vm_backend::is_kairon() {
        return Err(err(
            StatusCode::NOT_IMPLEMENTED,
            "container sandboxes require the Kairon backend",
        ));
    }
    Ok(state.read().await.kube_client.client())
}

#[derive(Debug, Deserialize)]
pub struct CreateContainer {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    pub image: String,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub entrypoint: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub writable_root: Option<bool>,
    #[serde(default)]
    pub exit_policy: Option<String>,
    /// `host:guest` TCP pairs; host ports are 1024+.
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub restart: Option<String>,
    #[serde(default)]
    pub max_restarts: Option<u32>,
    #[serde(default)]
    pub health_check: Option<OciHealthCheck>,
    #[serde(default)]
    pub offline: Option<bool>,
    #[serde(default)]
    pub allow_hosts: Vec<String>,
    /// `tiny` | `small` | `standard`.
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub http_proxy_ports: Vec<u16>,
    #[serde(default)]
    pub cpus: Option<u32>,
    /// Kubernetes quantity, at least 512Mi.
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(default)]
    pub node_selector: BTreeMap<String, String>,
}

fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 63
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

/// Mirrors Kairon's admission rules so callers get a clear 422 up front.
pub fn validate(r: &CreateContainer) -> Result<(), String> {
    if r.image.trim().is_empty() {
        return Err("image is required".into());
    }
    if let Some(n) = &r.name
        && !valid_name(n)
    {
        return Err("name must be a lowercase DNS label".into());
    }
    if let Some(p) = &r.platform
        && !matches!(p.as_str(), "linux/arm64" | "linux/amd64")
    {
        return Err("platform must be linux/arm64 or linux/amd64".into());
    }
    if let Some(p) = &r.restart
        && !matches!(p.as_str(), "no" | "on-failure" | "always")
    {
        return Err("restart must be no, on-failure or always".into());
    }
    if let Some(p) = &r.exit_policy
        && !matches!(p.as_str(), "keep" | "poweroff")
    {
        return Err("exit_policy must be keep or poweroff".into());
    }
    if let Some(p) = &r.profile
        && !matches!(p.as_str(), "tiny" | "small" | "standard")
    {
        return Err("profile must be tiny, small or standard".into());
    }
    if let Some(e) = r
        .env
        .iter()
        .find(|e| !e.contains('=') || e.starts_with('='))
    {
        return Err(format!("env entry '{e}' must be KEY=value"));
    }
    if !r.ports.is_empty() && (r.offline == Some(true) || !r.allow_hosts.is_empty()) {
        return Err("ports cannot be combined with offline or allow_hosts".into());
    }
    for p in &r.ports {
        let host = p
            .split(':')
            .next()
            .and_then(|h| h.parse::<u32>().ok())
            .ok_or_else(|| format!("port '{p}' must look like host:guest"))?;
        if !p.contains(':') || !(1024..=65535).contains(&host) {
            return Err(format!("port '{p}': host port must be 1024-65535"));
        }
    }
    if let Some(h) = &r.health_check
        && h.command.is_empty()
    {
        return Err("health_check.command is required".into());
    }
    if r.cpus == Some(0) {
        return Err("cpus must be at least 1".into());
    }
    Ok(())
}

/// Env names that suggest a credential; Machine objects are readable by anyone
/// who can read Machines, and Kairon has no secret reference field yet.
pub fn credential_warnings(env: &[String]) -> Vec<String> {
    env.iter()
        .filter_map(|e| e.split('=').next())
        .filter(|k| {
            let k = k.to_ascii_uppercase();
            [
                "SECRET", "TOKEN", "PASSWORD", "PASSWD", "API_KEY", "PRIVATE",
            ]
            .iter()
            .any(|w| k.contains(w))
        })
        .map(|k| format!("env {k} looks like a credential; Machine objects are not secret"))
        .collect()
}

/// The Machine body. Built as JSON because `spec.image` must be absent.
pub fn machine_object(r: &CreateContainer, name: &str, ns: &str) -> Result<Value, String> {
    let oci = OciSpec {
        image: r.image.clone(),
        platform: r.platform.clone(),
        command: r.command.clone(),
        entrypoint: r.entrypoint.clone(),
        env: r.env.clone(),
        workdir: r.workdir.clone(),
        user: r.user.clone(),
        writable_root: r.writable_root,
        exit_policy: r.exit_policy.clone(),
        ports: r.ports.clone(),
        restart: r.restart.clone(),
        max_restarts: r.max_restarts,
        health_check: r.health_check.clone(),
        offline: r.offline,
        allow_hosts: r.allow_hosts.clone(),
        profile: r.profile.clone(),
        extra: Default::default(),
    };
    let sandbox = SandboxSpec {
        template_name: None,
        http_proxy_ports: r.http_proxy_ports.clone(),
        oci: Some(oci),
        extra: Default::default(),
    };
    let mut spec = json!({ "sandbox": sandbox });
    let mut resources = serde_json::Map::new();
    if let Some(c) = r.cpus {
        resources.insert("cpu".into(), json!(c.to_string()));
    }
    if let Some(m) = &r.memory {
        resources.insert("memory".into(), json!(m));
    }
    if !resources.is_empty() {
        spec["resources"] = Value::Object(resources);
    }
    let mut selector = r.node_selector.clone();
    selector
        .entry("kubernetes.io/os".into())
        .or_insert_with(|| "darwin".into());
    spec["placement"] = json!({ "nodeSelector": selector });
    Ok(json!({
        "apiVersion": "kairon.zyvor.dev/v1beta1",
        "kind": "Machine",
        "metadata": {
            "name": name,
            "namespace": ns,
            "labels": {LABEL_MANAGED_BY: "veyron", LABEL_KIND: KIND_CONTAINER},
        },
        "spec": spec,
    }))
}

fn summarize(m: &Machine) -> Value {
    let st = m.status.as_ref();
    let c = st.and_then(|s| s.container.as_ref());
    let oci = m.spec.sandbox.as_ref().and_then(|s| s.oci.as_ref());
    let exited = c.and_then(|c| c.exit_code).is_some();
    json!({
        "name": m.name(),
        "namespace": m.namespace(),
        "image": oci.map(|o| o.image.clone()),
        "phase": m.phase(),
        "node": st.and_then(|s| s.node_name.clone()),
        "message": m.message(),
        "exited": exited,
        "exit_code": c.and_then(|c| c.exit_code),
        "restarts": c.and_then(|c| c.restarts),
        "health": c.and_then(|c| c.health.clone()),
        "init_error": c.and_then(|c| c.init_error.clone()),
        "ports": oci.map(|o| o.ports.clone()).unwrap_or_default(),
        "guest_ip": st.and_then(|s| s.guest_ip.clone()),
    })
}

async fn create(State(state): State<SharedState>, Json(req): Json<CreateContainer>) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    if let Err(m) = validate(&req) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, m);
    }
    let ns = req.namespace.clone().unwrap_or_else(|| "default".into());
    if !valid_name(&ns) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "invalid namespace");
    }
    let name = req.name.clone().unwrap_or_else(|| {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let id: String = (0..8)
            .map(|_| char::from_digit(rng.gen_range(0..36u32), 36).unwrap_or('x'))
            .collect();
        format!("ctr-{id}")
    });
    let body = match machine_object(&req, &name, &ns) {
        Ok(b) => b,
        Err(m) => return err(StatusCode::UNPROCESSABLE_ENTITY, m),
    };
    let obj: DynamicObject = match serde_json::from_value(body) {
        Ok(o) => o,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let api: Api<DynamicObject> = Api::namespaced_with(c, &ns, &ApiResource::erase::<Machine>(&()));
    match api.create(&PostParams::default(), &obj).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({
                "name": name,
                "namespace": ns,
                "warnings": credential_warnings(&req.env),
            })),
        )
            .into_response(),
        Err(e) => kube_err(e),
    }
}

#[derive(Deserialize)]
struct NsQuery {
    namespace: Option<String>,
}

async fn list(State(state): State<SharedState>, Query(q): Query<NsQuery>) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let api: Api<Machine> = match q.namespace.as_deref() {
        None | Some("") | Some("all") => Api::all(c),
        Some(ns) => Api::namespaced(c, ns),
    };
    let lp = ListParams::default().labels(&format!("{LABEL_KIND}={KIND_CONTAINER}"));
    match api.list(&lp).await {
        Ok(l) => Json(json!({ "items": l.items.iter().map(summarize).collect::<Vec<_>>() }))
            .into_response(),
        Err(e) => kube_err(e),
    }
}

async fn show(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    match Api::<Machine>::namespaced(c, &ns).get(&name).await {
        Ok(m)
            if m.spec
                .sandbox
                .as_ref()
                .and_then(|s| s.oci.as_ref())
                .is_some() =>
        {
            Json(summarize(&m)).into_response()
        }
        Ok(_) => err(StatusCode::NOT_FOUND, "not a container Machine"),
        Err(e) => kube_err(e),
    }
}

async fn remove(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> Response {
    let c = match client(&state).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let api: Api<Machine> = Api::namespaced(c, &ns);
    // Refuse to delete ordinary VMs through the container route.
    match api.get(&name).await {
        Ok(m)
            if m.spec
                .sandbox
                .as_ref()
                .and_then(|s| s.oci.as_ref())
                .is_some() => {}
        Ok(_) => return err(StatusCode::NOT_FOUND, "not a container Machine"),
        Err(e) => return kube_err(e),
    }
    match api.delete(&name, &DeleteParams::background()).await {
        Ok(_) => Json(json!({ "deleted": name })).into_response(),
        Err(e) => kube_err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(v: Value) -> CreateContainer {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn machine_has_no_spec_image_and_defaults_to_darwin() {
        let r = req(
            json!({"image": "alpine:3.22", "command": ["sh", "-c", "exit 3"],
                           "restart": "no", "memory": "512Mi"}),
        );
        validate(&r).unwrap();
        let obj = machine_object(&r, "c1", "default").unwrap();
        assert!(obj["spec"].get("image").is_none());
        assert_eq!(
            obj["spec"]["placement"]["nodeSelector"]["kubernetes.io/os"],
            "darwin"
        );
        assert_eq!(obj["spec"]["sandbox"]["oci"]["image"], "alpine:3.22");
        assert_eq!(obj["metadata"]["labels"]["veyron.io/kind"], "container");
        // The result round-trips through the typed Machine.
        let m: Machine = serde_json::from_value(obj).unwrap();
        assert_eq!(
            m.spec.sandbox.unwrap().oci.unwrap().command,
            vec!["sh", "-c", "exit 3"]
        );
    }

    #[test]
    fn caller_node_selector_wins() {
        let r = req(json!({"image": "x", "node_selector": {"kubernetes.io/os": "linux"}}));
        let obj = machine_object(&r, "c", "default").unwrap();
        assert_eq!(
            obj["spec"]["placement"]["nodeSelector"]["kubernetes.io/os"],
            "linux"
        );
    }

    #[test]
    fn validation_mirrors_admission_rules() {
        assert!(validate(&req(json!({"image": ""}))).is_err());
        assert!(validate(&req(json!({"image": "a", "restart": "sometimes"}))).is_err());
        assert!(validate(&req(json!({"image": "a", "platform": "windows/amd64"}))).is_err());
        assert!(validate(&req(json!({"image": "a", "env": ["NOEQUALS"]}))).is_err());
        assert!(validate(&req(json!({"image": "a", "ports": ["80:80"]}))).is_err());
        assert!(
            validate(&req(
                json!({"image": "a", "ports": ["8080:80"], "offline": true})
            ))
            .is_err()
        );
        assert!(
            validate(&req(
                json!({"image": "a", "ports": ["8080:80"], "allow_hosts": ["x.com"]})
            ))
            .is_err()
        );
        assert!(validate(&req(json!({"image": "a", "ports": ["8080:80"]}))).is_ok());
        assert!(validate(&req(json!({"image": "a", "health_check": {"command": []}}))).is_err());
        assert!(validate(&req(json!({"image": "a", "name": "Bad_Name"}))).is_err());
    }

    #[test]
    fn flags_credential_looking_env() {
        let w = credential_warnings(&[
            "API_KEY=abc".into(),
            "MODE=fast".into(),
            "db_password=x".into(),
        ]);
        assert_eq!(w.len(), 2);
    }
}
