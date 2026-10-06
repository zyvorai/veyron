// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! One-click model serving: an Ubuntu VM that runs an OpenAI-compatible server on
//! port 8000 behind a ClusterIP Service.
//!
//! - `runtime: llamacpp` — `llama-server` on CPU with a GGUF model (works without GPUs).
//! - `runtime: vllm` — NVIDIA driver + vLLM on a GPU VM (needs `gpu`).
//!
//! "Use for Veyron AI" stores the choice in the ConfigMap `veyron-ai-llm` and points
//! the agent at the in-cluster endpoint, so prompts never leave the cluster. Every
//! API replica re-reads that ConfigMap each minute ([`refresh_override`]).

use std::collections::BTreeMap;
use std::time::Duration;

use axum::{
    Extension, Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use futures_util::future::join_all;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::Api;
use kube::api::{ListParams, ObjectMeta, Patch, PatchParams};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::api::err;
use super::exec;
use super::registry::safe_name;
use crate::api::auth_context::AuthContext;
use crate::api::http_server::web::SharedState;
use crate::copilot::LlmConfig;
use crate::kube::types::VirtualMachine;

pub const MODEL_LABEL: &str = "veyron.io/ai-model";
const ANN_RUNTIME: &str = "veyron.io/ai-runtime";
const ANN_MODEL: &str = "veyron.io/ai-model-id";
const ANN_SERVED: &str = "veyron.io/ai-served-name";
const SETTING_CM: &str = "veyron-ai-llm";
const PORT: u16 = 8000;
const TEMPLATE: &str = "ubuntu-24.04";
const DEFAULT_GGUF: &str = "Qwen/Qwen2.5-0.5B-Instruct-GGUF/qwen2.5-0.5b-instruct-q4_k_m.gguf";
const DEFAULT_VLLM: &str = "Qwen/Qwen2.5-1.5B-Instruct";

#[derive(Debug, Deserialize)]
pub struct CreateModel {
    pub name: String,
    #[serde(default)]
    pub namespace: Option<String>,
    /// `llamacpp` (default) or `vllm`.
    #[serde(default)]
    pub runtime: Option<String>,
    /// llamacpp: `owner/repo/file.gguf` on Hugging Face or an https URL to a GGUF.
    /// vllm: a Hugging Face repo id.
    #[serde(default)]
    pub model: Option<String>,
    /// Name clients use in `"model"` (default: derived from the model).
    #[serde(default)]
    pub served_name: Option<String>,
    #[serde(default)]
    pub cpus: Option<u32>,
    #[serde(default)]
    pub memory: Option<String>,
    /// Scratch disk for weights and runtimes (ephemeral, re-downloaded on restart).
    #[serde(default)]
    pub disk_size: Option<String>,
    #[serde(default)]
    pub context: Option<u32>,
    /// `{count, resource_name}` like `POST /vms`; required for vllm.
    #[serde(default)]
    pub gpu: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
struct Plan {
    runtime: &'static str,
    model: String,
    served_name: String,
    download_url: Option<String>,
    cpus: u32,
    memory: String,
    disk: String,
    context: u32,
}

fn shell_safe(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 400
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/:?&=%+~".contains(c))
}

fn slug(s: &str) -> String {
    let base = s.rsplit('/').next().unwrap_or(s);
    let base = base.strip_suffix(".gguf").unwrap_or(base).to_lowercase();
    let base = base
        .split(['-', '_'])
        .filter(|p| {
            !matches!(
                *p,
                "q2" | "q3" | "q4" | "q5" | "q6" | "q8" | "k" | "m" | "s" | "l" | "0" | "gguf"
            )
        })
        .collect::<Vec<_>>()
        .join("-");
    let s: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').chars().take(60).collect::<String>();
    if s.is_empty() { "model".into() } else { s }
}

fn plan(req: &CreateModel) -> Result<Plan, String> {
    let runtime = match req.runtime.as_deref().unwrap_or("llamacpp") {
        "llamacpp" | "llama.cpp" | "llama-cpp" => "llamacpp",
        "vllm" => "vllm",
        other => return Err(format!("Unknown runtime '{other}' (llamacpp or vllm)")),
    };
    let model = req
        .model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or(if runtime == "vllm" {
            DEFAULT_VLLM
        } else {
            DEFAULT_GGUF
        })
        .to_string();
    if !shell_safe(&model) {
        return Err("model may only contain letters, digits and ._-/:?&=%+~".into());
    }
    let download_url = if runtime == "llamacpp" {
        if model.starts_with("https://") {
            Some(model.clone())
        } else {
            let parts: Vec<&str> = model.splitn(3, '/').collect();
            if parts.len() != 3 || !parts[2].ends_with(".gguf") {
                return Err(
                    "llamacpp model must be owner/repo/file.gguf or an https:// URL".into(),
                );
            }
            Some(format!(
                "https://huggingface.co/{}/{}/resolve/main/{}",
                parts[0], parts[1], parts[2]
            ))
        }
    } else {
        if model.contains("://") || model.split('/').count() != 2 {
            return Err(
                "vllm model must be a Hugging Face repo id like Qwen/Qwen2.5-1.5B-Instruct".into(),
            );
        }
        if req.gpu.is_none() {
            return Err(
                "vllm needs a GPU: add \"gpu\": {\"count\": 1}, or use runtime llamacpp on CPU"
                    .into(),
            );
        }
        None
    };
    let served_name = match req
        .served_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(s)
            if s.len() <= 100
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._-/".contains(c)) =>
        {
            s.to_string()
        }
        Some(_) => return Err("served_name may only contain letters, digits and ._-/".into()),
        None => slug(&model),
    };
    let (cpus, memory, disk) = if runtime == "vllm" {
        (8, "32Gi", "100Gi")
    } else {
        (4, "8Gi", "30Gi")
    };
    Ok(Plan {
        runtime,
        model,
        served_name,
        download_url,
        cpus: req.cpus.unwrap_or(cpus).clamp(1, 128),
        memory: req.memory.clone().unwrap_or_else(|| memory.into()),
        disk: req.disk_size.clone().unwrap_or_else(|| disk.into()),
        context: req
            .context
            .unwrap_or(if runtime == "vllm" { 8192 } else { 4096 })
            .clamp(512, 131_072),
    })
}

fn indent(s: &str, n: usize) -> String {
    let pad = " ".repeat(n);
    s.lines()
        .map(|l| format!("{pad}{l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

const MOUNT_DATA: &str = r#"D=/opt/veyron-ai
mkdir -p "$D"
if ! mountpoint -q "$D"; then
  if blkid -L veyron-ai >/dev/null 2>&1; then
    mount LABEL=veyron-ai "$D"
  else
    for d in $(lsblk -dpno NAME,TYPE | awk '$2=="disk"{print $1}'); do
      [ -n "$(lsblk -no FSTYPE "$d" | tr -d '[:space:]')" ] && continue
      [ "$(lsblk -bdno SIZE "$d")" -lt 5000000000 ] && continue
      mkfs.ext4 -q -F -L veyron-ai "$d" && mount "$d" "$D" && break
    done
  fi
fi"#;

fn setup_script(p: &Plan) -> String {
    let body = if p.runtime == "llamacpp" {
        r#"export DEBIAN_FRONTEND=noninteractive
command -v unzip >/dev/null || { apt-get update -q; apt-get install -yq unzip curl libgomp1; }
if [ ! -x "$D/llama/llama-server" ]; then
  url=$(curl -fsSL https://api.github.com/repos/ggml-org/llama.cpp/releases/latest | grep -o 'https://[^"]*bin-ubuntu-x64\.zip' | head -1 || true)
  if [ -n "$url" ] && curl -fL --retry 5 -o /tmp/llama.zip "$url"; then
    rm -rf "$D/llama-x" && mkdir -p "$D/llama-x" && unzip -q -o /tmp/llama.zip -d "$D/llama-x"
    bin=$(find "$D/llama-x" -name llama-server -type f | head -1)
    rm -rf "$D/llama" && mv "$(dirname "$bin")" "$D/llama"
  else
    apt-get install -yq build-essential cmake git
    rm -rf "$D/src" && git clone --depth 1 https://github.com/ggml-org/llama.cpp "$D/src"
    cmake -S "$D/src" -B "$D/src/build" -DLLAMA_CURL=OFF -DCMAKE_BUILD_TYPE=Release
    cmake --build "$D/src/build" -j "$(nproc)" --target llama-server
    rm -rf "$D/llama" && cp -r "$D/src/build/bin" "$D/llama"
  fi
fi
if [ ! -s "$D/model.gguf" ]; then
  curl -fL --retry 10 --retry-delay 5 -o "$D/model.gguf.part" "$MODEL_URL"
  mv "$D/model.gguf.part" "$D/model.gguf"
fi"#
    } else {
        r#"export DEBIAN_FRONTEND=noninteractive
if ! nvidia-smi >/dev/null 2>&1; then
  if [ ! -e "$D/.driver-installed" ]; then
    apt-get update -q
    apt-get install -yq ubuntu-drivers-common
    ubuntu-drivers install --gpgpu || apt-get install -yq nvidia-driver-550-server nvidia-utils-550-server
    touch "$D/.driver-installed"
    systemctl reboot
    exit 0
  fi
  echo "NVIDIA driver installed but no GPU visible" >&2
  exit 1
fi
if [ ! -x "$D/venv/bin/vllm" ]; then
  apt-get install -yq python3-venv python3-pip
  python3 -m venv "$D/venv"
  "$D/venv/bin/pip" install -q --upgrade pip
  TMPDIR="$D" "$D/venv/bin/pip" install -q vllm
fi"#
    };
    format!(
        "#!/bin/bash\nset -euo pipefail\n. /etc/veyron-model.env\n{MOUNT_DATA}\n{body}\ntouch \"$D/.ready\"\n"
    )
}

fn run_script(p: &Plan) -> &'static str {
    if p.runtime == "llamacpp" {
        r#"#!/bin/sh
. /etc/veyron-model.env
D=/opt/veyron-ai
export LD_LIBRARY_PATH="$D/llama"
exec "$D/llama/llama-server" --host 0.0.0.0 --port 8000 -m "$D/model.gguf" --alias "$SERVED_NAME" -c "$CTX" -t "$(nproc)" --jinja"#
    } else {
        r#"#!/bin/sh
. /etc/veyron-model.env
D=/opt/veyron-ai
export HF_HOME="$D/hf"
exec "$D/venv/bin/vllm" serve "$MODEL" --host 0.0.0.0 --port 8000 --served-model-name "$SERVED_NAME" --max-model-len "$CTX" --enable-auto-tool-choice --tool-call-parser hermes"#
    }
}

fn cloud_init(p: &Plan) -> String {
    let env = format!(
        "MODEL='{}'\nMODEL_URL='{}'\nSERVED_NAME='{}'\nCTX='{}'",
        p.model,
        p.download_url.clone().unwrap_or_default(),
        p.served_name,
        p.context
    );
    let setup_unit = "[Unit]\nDescription=Veyron model setup\nWants=network-online.target\nAfter=network-online.target\n[Service]\nType=oneshot\nRemainAfterExit=yes\nTimeoutStartSec=0\nExecStart=/usr/local/sbin/veyron-model-setup.sh\n[Install]\nWantedBy=multi-user.target";
    let run_unit = "[Unit]\nDescription=Veyron model server (OpenAI-compatible, port 8000)\nRequires=veyron-model-setup.service\nAfter=veyron-model-setup.service\n[Service]\nExecStart=/usr/local/sbin/veyron-model-run.sh\nRestart=always\nRestartSec=10\n[Install]\nWantedBy=multi-user.target";
    format!(
        "#cloud-config\nwrite_files:\n  - path: /etc/veyron-model.env\n    content: |\n{}\n  - path: /usr/local/sbin/veyron-model-setup.sh\n    permissions: '0755'\n    content: |\n{}\n  - path: /usr/local/sbin/veyron-model-run.sh\n    permissions: '0755'\n    content: |\n{}\n  - path: /etc/systemd/system/veyron-model-setup.service\n    content: |\n{}\n  - path: /etc/systemd/system/veyron-model.service\n    content: |\n{}\nruncmd:\n  - [ systemctl, daemon-reload ]\n  - [ systemctl, enable, veyron-model-setup.service, veyron-model.service ]\n  - [ systemctl, start, --no-block, veyron-model.service ]\n",
        indent(&env, 6),
        indent(&setup_script(p), 6),
        indent(run_script(p), 6),
        indent(setup_unit, 6),
        indent(run_unit, 6),
    )
}

pub fn endpoint(ns: &str, name: &str) -> String {
    format!(
        "http://{}.{ns}.svc:{PORT}/v1",
        crate::kube::vm_expose_service_name(name)
    )
}

fn kerr(status: u16, msg: impl AsRef<str>) -> Response {
    err(status, msg.as_ref())
}

// ── Handlers ──────────────────────────────────────────────────────

pub async fn create_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateModel>,
) -> Response {
    let name = match safe_name(req.name.trim()) {
        Ok(n) if n.len() <= 40 && !n.contains('.') => n,
        _ => return kerr(400, "name must be a DNS label of at most 40 characters"),
    };
    let ns = match safe_name(req.namespace.as_deref().unwrap_or("default")) {
        Ok(n) => n,
        Err(e) => return kerr(400, e),
    };
    let p = match plan(&req) {
        Ok(p) => p,
        Err(e) => return kerr(400, e),
    };
    let mut spec = json!({
        "name": name,
        "namespace": ns,
        "template": TEMPLATE,
        "cpus": p.cpus,
        "memory": p.memory,
        "cloud_init": cloud_init(&p),
        "start": false,
        "allow_internet": true,
        "expose": {"enabled": true, "service_type": "ClusterIP",
                   "ports": [{"name": "http", "port": PORT, "target_port": PORT}]},
    });
    if let Some(g) = &req.gpu {
        spec["gpu"] = g.clone();
    }
    let r = match exec::call(
        &auth.role,
        &auth.subject,
        "POST",
        "/api/v1/vms",
        Some(&spec),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return kerr(500, e.to_string()),
    };
    if !r.ok() {
        return kerr(r.status, r.error_message());
    }

    let client = state.read().await.kube_client.client();
    let vms: Api<VirtualMachine> = Api::namespaced(client, &ns);
    let mut notes = Vec::new();
    if let Err(e) = label_and_size(&vms, &name, &p).await {
        notes.push(format!("Could not set model labels or disk size: {e}"));
    }
    let start = exec::call(
        &auth.role,
        &auth.subject,
        "POST",
        &format!("/api/v1/vms/{ns}/{name}/start"),
        None,
    )
    .await;
    if !matches!(&start, Ok(s) if s.ok()) {
        notes.push("Created stopped; start it from Machines.".into());
    }
    Json(json!({"success": true, "data": {
        "name": name,
        "namespace": ns,
        "runtime": p.runtime,
        "model": p.model,
        "served_name": p.served_name,
        "endpoint": endpoint(&ns, &name),
        "status": "provisioning",
        "notes": notes,
        "message": if p.runtime == "vllm" {
            "Installing the NVIDIA driver (one reboot) and vLLM; first start takes 15-30 minutes."
        } else {
            "Downloading llama.cpp and the model; ready in a few minutes."
        },
    }}))
    .into_response()
}

async fn label_and_size(vms: &Api<VirtualMachine>, name: &str, p: &Plan) -> anyhow::Result<()> {
    let vm = vms.get(name).await?;
    let mut volumes = serde_json::to_value(&vm)?
        .pointer("/spec/template/spec/volumes")
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(arr) = volumes.as_array_mut() {
        for v in arr.iter_mut() {
            if v.get("name").and_then(Value::as_str) == Some("datadisk")
                && v.get("emptyDisk").is_some()
            {
                v["emptyDisk"]["capacity"] = json!(p.disk);
            }
        }
    }
    let mut patch = json!({
        "metadata": {
            "labels": {MODEL_LABEL: "true"},
            "annotations": {ANN_RUNTIME: p.runtime, ANN_MODEL: p.model, ANN_SERVED: p.served_name}
        }
    });
    if volumes.is_array() {
        patch["spec"] = json!({"template": {"spec": {"volumes": volumes}}});
    }
    vms.patch(name, &PatchParams::default(), &Patch::Merge(&patch))
        .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct ModelRow {
    name: String,
    namespace: String,
    runtime: String,
    model: String,
    served_name: String,
    vm_status: String,
    /// provisioning | installing | ready | stopped
    status: String,
    endpoint: String,
    models: Vec<String>,
    in_use: bool,
    created_at: Option<String>,
}

async fn probe(url: &str) -> Option<Vec<String>> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;
    let r = http.get(format!("{url}/models")).send().await.ok()?;
    if !r.status().is_success() {
        return None;
    }
    let v: Value = r.json().await.ok()?;
    Some(
        v.get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string))
            .collect(),
    )
}

async fn model_rows(client: &kube::Client) -> Vec<ModelRow> {
    let vms: Api<VirtualMachine> = Api::all(client.clone());
    let list = vms
        .list(&ListParams::default().labels(&format!("{MODEL_LABEL}=true")))
        .await
        .map(|l| l.items)
        .unwrap_or_default();
    let current = crate::copilot::runtime_override().map(|c| c.base_url);
    let probes = join_all(list.iter().map(|vm| {
        let ep = endpoint(
            vm.metadata.namespace.as_deref().unwrap_or("default"),
            vm.metadata.name.as_deref().unwrap_or(""),
        );
        async move { probe(&ep).await }
    }))
    .await;
    list.into_iter()
        .zip(probes)
        .map(|(vm, served)| {
            let ns = vm.metadata.namespace.clone().unwrap_or_default();
            let name = vm.metadata.name.clone().unwrap_or_default();
            let ann = vm.metadata.annotations.clone().unwrap_or_default();
            let get = |k: &str| ann.get(k).cloned().unwrap_or_default();
            let vm_status = vm
                .status
                .as_ref()
                .and_then(|s| s.printable_status.clone())
                .unwrap_or_default();
            let ep = endpoint(&ns, &name);
            let status = match (&served, vm_status.as_str()) {
                (Some(_), _) => "ready",
                (None, "Running") => "installing",
                (None, "Stopped" | "Halted") => "stopped",
                _ => "provisioning",
            };
            ModelRow {
                runtime: get(ANN_RUNTIME),
                model: get(ANN_MODEL),
                served_name: get(ANN_SERVED),
                vm_status,
                status: status.into(),
                in_use: current.as_deref() == Some(ep.as_str()),
                models: served.unwrap_or_default(),
                created_at: vm
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339()),
                endpoint: ep,
                name,
                namespace: ns,
            }
        })
        .collect()
}

pub async fn list_handler(State(state): State<SharedState>) -> Response {
    let client = state.read().await.kube_client.client();
    Json(json!({"success": true, "data": model_rows(&client).await})).into_response()
}

pub async fn delete_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Path((ns, name)): Path<(String, String)>,
) -> Response {
    let (ns, name) = match (safe_name(&ns), safe_name(&name)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return kerr(400, "invalid name"),
    };
    let (client, ai_ns) = {
        let s = state.read().await;
        (s.kube_client.client(), super::ai_namespace(&s.namespace))
    };
    let vms: Api<VirtualMachine> = Api::namespaced(client.clone(), &ns);
    match vms.get_opt(&name).await {
        Ok(Some(vm))
            if vm
                .metadata
                .labels
                .as_ref()
                .and_then(|l| l.get(MODEL_LABEL))
                .is_some() => {}
        Ok(_) => return kerr(404, "No model server with that name"),
        Err(e) => return kerr(500, e.to_string()),
    }
    let r = match exec::call(
        &auth.role,
        &auth.subject,
        "DELETE",
        &format!("/api/v1/vms/{ns}/{name}"),
        None,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return kerr(500, e.to_string()),
    };
    if !r.ok() {
        return kerr(r.status, r.error_message());
    }
    let mut cleared = false;
    if let Some(s) = load_setting(&client, &ai_ns).await {
        if s.namespace == ns && s.name == name {
            let _ = Api::<ConfigMap>::namespaced(client.clone(), &ai_ns)
                .delete(SETTING_CM, &Default::default())
                .await;
            apply(None, String::new());
            cleared = true;
        }
    }
    Json(json!({"success": true, "data": {"deleted": true, "llm_setting_cleared": cleared}}))
        .into_response()
}

// ── "Use for Veyron AI" ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct LlmSetting {
    /// Model server VM; empty for a custom endpoint.
    #[serde(default)]
    namespace: String,
    #[serde(default)]
    name: String,
    base_url: String,
    model: String,
    set_by: String,
    set_at: String,
}

impl LlmSetting {
    fn custom(&self) -> bool {
        self.name.is_empty()
    }
}

const KEY_SECRET: &str = "veyron-ai-llm";

async fn load_key(client: &kube::Client, ns: &str) -> String {
    Api::<k8s_openapi::api::core::v1::Secret>::namespaced(client.clone(), ns)
        .get_opt(KEY_SECRET)
        .await
        .ok()
        .flatten()
        .and_then(|s| s.data?.get("api_key").cloned())
        .and_then(|b| String::from_utf8(b.0).ok())
        .unwrap_or_default()
}

async fn save_key(client: &kube::Client, ns: &str, key: Option<&str>) -> anyhow::Result<()> {
    use k8s_openapi::api::core::v1::Secret;
    let api: Api<Secret> = Api::namespaced(client.clone(), ns);
    match key.filter(|k| !k.is_empty()) {
        None => {
            let _ = api.delete(KEY_SECRET, &Default::default()).await;
        }
        Some(k) => {
            let secret = Secret {
                metadata: ObjectMeta {
                    name: Some(KEY_SECRET.into()),
                    labels: Some(BTreeMap::from([(
                        "veyron.io/type".to_string(),
                        "ai-llm-setting".to_string(),
                    )])),
                    ..Default::default()
                },
                string_data: Some(BTreeMap::from([("api_key".to_string(), k.to_string())])),
                type_: Some("Opaque".into()),
                ..Default::default()
            };
            api.patch(
                KEY_SECRET,
                &PatchParams::apply("veyron-ai").force(),
                &Patch::Apply(&secret),
            )
            .await?;
        }
    }
    Ok(())
}

/// Admin-supplied endpoint: http(s), no credentials, no link-local/metadata hosts.
fn valid_custom_url(u: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(u.trim()).map_err(|e| format!("Bad URL: {e}"))?;
    let host = url.host_str().unwrap_or("").to_lowercase();
    if !matches!(url.scheme(), "http" | "https") || host.is_empty() {
        return Err("URL must be http(s)://host/…".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Put the key in the API key field, not the URL".into());
    }
    if host.starts_with("169.254.") || host.starts_with("metadata") || host == "[fd00:ec2::254]" {
        return Err("Metadata endpoints are not allowed".into());
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

async fn load_setting(client: &kube::Client, ns: &str) -> Option<LlmSetting> {
    Api::<ConfigMap>::namespaced(client.clone(), ns)
        .get_opt(SETTING_CM)
        .await
        .ok()
        .flatten()
        .and_then(|cm| cm.data?.get("setting.json").cloned())
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn apply(setting: Option<&LlmSetting>, api_key: String) {
    crate::copilot::set_runtime_override(setting.map(|s| LlmConfig {
        base_url: s.base_url.clone(),
        api_key,
        model: s.model.clone(),
    }));
}

/// Sync the in-cluster model choice from its ConfigMap (every replica, each minute).
pub async fn refresh_override(client: &kube::Client, api_namespace: &str) {
    let ns = super::ai_namespace(api_namespace);
    match Api::<ConfigMap>::namespaced(client.clone(), &ns)
        .get_opt(SETTING_CM)
        .await
    {
        Ok(Some(_)) => {
            let s = load_setting(client, &ns).await;
            let key = match &s {
                Some(s) if s.custom() => load_key(client, &ns).await,
                _ => String::new(),
            };
            apply(s.as_ref(), key);
        }
        Ok(None) => apply(None, String::new()),
        Err(_) => {}
    }
}

fn setting_view(s: Option<&LlmSetting>) -> Value {
    let env = crate::copilot::env_llm_config().is_some();
    match s {
        Some(s) if s.custom() => json!({
            "source": "custom",
            "model": s.model,
            "base_url": s.base_url,
            "set_by": s.set_by,
            "set_at": s.set_at,
            "env_configured": env,
        }),
        Some(s) => json!({
            "source": "in-cluster",
            "model": s.model,
            "base_url": s.base_url,
            "ref": {"namespace": s.namespace, "name": s.name},
            "set_by": s.set_by,
            "set_at": s.set_at,
            "env_configured": env,
        }),
        None => json!({
            "source": if env { "env" } else { "none" },
            "model": crate::copilot::llm_config().map(|c| c.model),
            "env_configured": env,
        }),
    }
}

pub async fn get_llm_setting(State(state): State<SharedState>) -> Response {
    let (client, ns) = {
        let s = state.read().await;
        (s.kube_client.client(), super::ai_namespace(&s.namespace))
    };
    let s = load_setting(&client, &ns).await;
    Json(json!({"success": true, "data": setting_view(s.as_ref())})).into_response()
}

#[derive(Debug, Deserialize)]
pub struct PutSetting {
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    name: Option<String>,
    /// Custom OpenAI-compatible endpoint instead of a model server.
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    clear: bool,
}

/// Point Veyron AI at an in-cluster model server, or back to the env config (Admin).
pub async fn put_llm_setting(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<PutSetting>,
) -> Response {
    let (client, ai_ns) = {
        let s = state.read().await;
        (s.kube_client.client(), super::ai_namespace(&s.namespace))
    };
    let cms: Api<ConfigMap> = Api::namespaced(client.clone(), &ai_ns);
    if req.clear {
        let _ = cms.delete(SETTING_CM, &Default::default()).await;
        let _ = save_key(&client, &ai_ns, None).await;
        apply(None, String::new());
        return Json(json!({"success": true, "data": setting_view(None)})).into_response();
    }
    let now = chrono::Utc::now().to_rfc3339();
    let (setting, key) = if let Some(url) = req.url.as_deref().filter(|u| !u.trim().is_empty()) {
        let base_url = match valid_custom_url(url) {
            Ok(u) => u,
            Err(e) => return kerr(400, e),
        };
        let model = req
            .model
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .unwrap_or("gpt-4.1-mini");
        let key = req.api_key.clone().unwrap_or_default();
        if let Err(e) = save_key(&client, &ai_ns, Some(&key)).await {
            return kerr(500, format!("Could not save the API key: {e}"));
        }
        let s = LlmSetting {
            namespace: String::new(),
            name: String::new(),
            base_url,
            model: model.to_string(),
            set_by: auth.subject.clone(),
            set_at: now,
        };
        (s, key)
    } else {
        let (Some(ns), Some(name)) = (req.namespace.as_deref(), req.name.as_deref()) else {
            return kerr(
                400,
                "Send {namespace, name} of a model server, {url, model, api_key}, or {clear: true}",
            );
        };
        let row = model_rows(&client)
            .await
            .into_iter()
            .find(|r| r.namespace == ns && r.name == name);
        let Some(row) = row else {
            return kerr(404, "No model server with that name");
        };
        if row.status != "ready" {
            return kerr(
                409,
                format!("{name} is {} — wait until it is ready", row.status),
            );
        }
        let _ = save_key(&client, &ai_ns, None).await;
        let model = row
            .models
            .first()
            .cloned()
            .unwrap_or(row.served_name.clone());
        let s = LlmSetting {
            namespace: row.namespace,
            name: row.name,
            base_url: row.endpoint,
            model,
            set_by: auth.subject.clone(),
            set_at: now,
        };
        (s, String::new())
    };
    let cm = ConfigMap {
        metadata: ObjectMeta {
            name: Some(SETTING_CM.into()),
            labels: Some(BTreeMap::from([(
                "veyron.io/type".to_string(),
                "ai-llm-setting".to_string(),
            )])),
            ..Default::default()
        },
        data: Some(BTreeMap::from([(
            "setting.json".to_string(),
            serde_json::to_string(&setting).unwrap_or_default(),
        )])),
        ..Default::default()
    };
    if let Err(e) = cms
        .patch(
            SETTING_CM,
            &PatchParams::apply("veyron-ai").force(),
            &Patch::Apply(&cm),
        )
        .await
    {
        return kerr(500, format!("Could not save the setting: {e}"));
    }
    apply(Some(&setting), key);
    Json(json!({"success": true, "data": setting_view(Some(&setting))})).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(runtime: &str, model: Option<&str>, gpu: bool) -> CreateModel {
        CreateModel {
            name: "llm".into(),
            namespace: None,
            runtime: Some(runtime.into()),
            model: model.map(str::to_string),
            served_name: None,
            cpus: None,
            memory: None,
            disk_size: None,
            context: None,
            gpu: gpu.then(|| json!({"count": 1})),
        }
    }

    #[test]
    fn llamacpp_defaults_and_hf_urls() {
        let p = plan(&req("llamacpp", None, false)).unwrap();
        assert_eq!(p.runtime, "llamacpp");
        assert_eq!(
            p.download_url.as_deref(),
            Some(
                "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf"
            )
        );
        assert_eq!(p.served_name, "qwen2.5-0.5b-instruct");
        assert_eq!((p.cpus, p.memory.as_str()), (4, "8Gi"));
        assert!(plan(&req("llamacpp", Some("owner/repo"), false)).is_err());
        assert!(plan(&req("llamacpp", Some("a/b/c.gguf'; rm -rf /"), false)).is_err());
    }

    #[test]
    fn vllm_needs_gpu_and_repo_id() {
        assert!(plan(&req("vllm", None, false)).unwrap_err().contains("GPU"));
        let p = plan(&req("vllm", Some("meta-llama/Llama-3.1-8B-Instruct"), true)).unwrap();
        assert_eq!(p.served_name, "llama-3.1-8b-instruct");
        assert!(p.download_url.is_none());
        assert!(plan(&req("vllm", Some("https://x/y"), true)).is_err());
        assert!(plan(&req("ollama", None, false)).is_err());
    }

    #[test]
    fn cloud_init_is_valid_yaml_and_merges_with_guestkit() {
        for p in [
            plan(&req("llamacpp", None, false)).unwrap(),
            plan(&req("vllm", None, true)).unwrap(),
        ] {
            let ci = cloud_init(&p);
            let v: serde_yml::Value = serde_yml::from_str(&ci).unwrap();
            assert_eq!(v["write_files"].as_sequence().unwrap().len(), 5);
            let tpl = crate::templates::TemplateManager::new();
            let base = tpl
                .get(TEMPLATE)
                .unwrap()
                .cloud_init
                .clone()
                .unwrap()
                .user_data;
            let merged = crate::templates::merge_cloud_init(&base, &ci);
            assert!(merged.contains("guestkit-agent") && merged.contains("veyron-model.service"));
            let mv: serde_yml::Value = serde_yml::from_str(&merged).unwrap();
            assert!(mv["write_files"].as_sequence().unwrap().len() >= 7);
        }
    }

    #[test]
    fn custom_urls_are_checked() {
        assert_eq!(
            valid_custom_url("https://api.openai.com/v1/").unwrap(),
            "https://api.openai.com/v1"
        );
        assert!(valid_custom_url("http://169.254.169.254/latest").is_err());
        assert!(valid_custom_url("https://u:p@x/v1").is_err());
        assert!(valid_custom_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn endpoint_uses_expose_service() {
        assert_eq!(endpoint("ai", "llm"), "http://llm-veyron-xp.ai.svc:8000/v1");
    }
}
