// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Agent sandboxes: short-lived, isolated VMs where AI agents run code.
//!
//! - **KubeVirt** (default): a warm pool of small Ubuntu VMs in their own namespace,
//!   claimed in seconds. Exec and file transfer go over the QEMU guest agent, so a
//!   sandbox needs no network at all; egress is denied unless the caller asks for it.
//! - **Kairon** (`--features kairon`, `VEYRON_VM_BACKEND=kairon`): Firecracker
//!   Machines, forked from a warm parent when `VEYRON_SANDBOX_KAIRON_PARENT` is set.
//!
//! Every sandbox has an owner and a TTL. Owners only see their own sandboxes (admins
//! see all); the leader tick deletes expired ones and keeps the pool topped up.

#[cfg(feature = "kairon")]
mod kairon;
mod kubevirt;

use std::time::Duration;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ai::registry::{ToolCtx, ToolKind, ToolOutput, ToolSpec, wrap_untrusted};
use crate::api::auth_context::AuthContext;
use crate::api::http_server::web::{ApiRole, SharedState};

pub const LABEL_SANDBOX: &str = "veyron.io/sandbox";
pub const LABEL_STATE: &str = "veyron.io/sandbox-state";
pub const ANN_OWNER: &str = "veyron.io/sandbox-owner";
pub const ANN_EXPIRES: &str = "veyron.io/sandbox-expires";
pub const ANN_INTERNET: &str = "veyron.io/sandbox-internet";
pub const ANN_CREATED: &str = "veyron.io/sandbox-created";
pub const STATE_WARM: &str = "warm";
pub const STATE_CLAIMED: &str = "claimed";

const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
const MAX_OUTPUT_CHARS: usize = 64 * 1024;

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

pub fn namespace() -> String {
    std::env::var("VEYRON_SANDBOX_NAMESPACE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "veyron-sandboxes".into())
}

pub fn pool_target() -> usize {
    env_u64("VEYRON_SANDBOX_POOL_SIZE", 1) as usize
}

pub fn max_per_owner() -> usize {
    env_u64("VEYRON_SANDBOX_MAX_PER_OWNER", 3) as usize
}

fn max_ttl_minutes() -> u64 {
    env_u64("VEYRON_SANDBOX_MAX_TTL_MINUTES", 24 * 60)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    KubeVirt,
    #[cfg(feature = "kairon")]
    Kairon,
}

fn backend() -> Backend {
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::selected() == crate::api::vm_backend::BackendKind::Kairon {
        return Backend::Kairon;
    }
    Backend::KubeVirt
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sandbox {
    pub id: String,
    pub namespace: String,
    pub owner: String,
    /// `ready` | `starting` | `failed` | `warm`
    pub status: String,
    pub backend: String,
    pub internet: bool,
    pub created_at: String,
    pub expires_at: String,
    pub vm: String,
    /// Egress has been blocked (only meaningful when `internet` is false).
    #[serde(default)]
    pub isolated: bool,
}

impl Sandbox {
    /// Unclaimed warm-pool VM (`warm` when ready, `warming` while booting).
    pub fn is_pool(&self) -> bool {
        self.status == "warm" || self.status == "warming"
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ExecResult {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
    pub timed_out: bool,
    pub truncated: bool,
}

/// Owners are compared without the "(via Veyron AI)" suffix, so the console, the
/// assistant and MCP all see the same sandboxes for one person.
pub fn owner_key(subject: &str) -> String {
    subject
        .trim_end_matches(" (via Veyron AI)")
        .trim()
        .to_string()
}

fn can_access(auth_role: &ApiRole, subject: &str, sb: &Sandbox) -> bool {
    *auth_role == ApiRole::Admin || owner_key(subject) == owner_key(&sb.owner)
}

fn new_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let s: String = (0..8)
        .map(|_| {
            let n = rng.gen_range(0..36u8);
            if n < 10 {
                (b'0' + n) as char
            } else {
                (b'a' + n - 10) as char
            }
        })
        .collect();
    format!("sbx-{s}")
}

// ── Backend dispatch ───────────────────────────────────────────────

async fn list_all(client: &kube::Client) -> anyhow::Result<Vec<Sandbox>> {
    match backend() {
        Backend::KubeVirt => kubevirt::list(client).await,
        #[cfg(feature = "kairon")]
        Backend::Kairon => kairon::list(client).await,
    }
}

async fn claim(
    client: &kube::Client,
    owner: &str,
    ttl_minutes: u64,
    internet: bool,
    wait: Duration,
) -> anyhow::Result<Sandbox> {
    match backend() {
        Backend::KubeVirt => kubevirt::claim(client, owner, ttl_minutes, internet, wait).await,
        #[cfg(feature = "kairon")]
        Backend::Kairon => kairon::claim(client, owner, ttl_minutes, internet, wait).await,
    }
}

async fn destroy(client: &kube::Client, sb: &Sandbox) -> anyhow::Result<()> {
    match backend() {
        Backend::KubeVirt => kubevirt::destroy(client, sb).await,
        #[cfg(feature = "kairon")]
        Backend::Kairon => kairon::destroy(client, sb).await,
    }
}

async fn exec(
    client: &kube::Client,
    sb: &Sandbox,
    command: &str,
    timeout_secs: u64,
) -> anyhow::Result<ExecResult> {
    let mut r = match backend() {
        Backend::KubeVirt => kubevirt::exec(client, sb, command, timeout_secs).await?,
        #[cfg(feature = "kairon")]
        Backend::Kairon => kairon::exec(client, sb, command, timeout_secs).await?,
    };
    for s in [&mut r.stdout, &mut r.stderr] {
        if s.len() > MAX_OUTPUT_CHARS {
            let cut = crate::ai::registry::char_floor(s, MAX_OUTPUT_CHARS);
            s.truncate(cut);
            r.truncated = true;
        }
    }
    Ok(r)
}

async fn write_file(
    client: &kube::Client,
    sb: &Sandbox,
    path: &str,
    data: &[u8],
) -> anyhow::Result<()> {
    match backend() {
        Backend::KubeVirt => kubevirt::write_file(client, sb, path, data).await,
        #[cfg(feature = "kairon")]
        Backend::Kairon => kairon::write_file(client, sb, path, data).await,
    }
}

async fn read_file(client: &kube::Client, sb: &Sandbox, path: &str) -> anyhow::Result<Vec<u8>> {
    match backend() {
        Backend::KubeVirt => kubevirt::read_file(client, sb, path, MAX_FILE_BYTES).await,
        #[cfg(feature = "kairon")]
        Backend::Kairon => kairon::read_file(client, sb, path, MAX_FILE_BYTES).await,
    }
}

pub fn backend_summary() -> Value {
    match backend() {
        Backend::KubeVirt => json!({
            "backend": "kubevirt",
            "namespace": namespace(),
            "pool_target": pool_target(),
            "typical_start": if pool_target() > 0 { "seconds (warm pool)" } else { "1–3 minutes (cold boot)" },
        }),
        #[cfg(feature = "kairon")]
        Backend::Kairon => json!({
            "backend": "kairon",
            "namespace": namespace(),
            "fork_parent": kairon::fork_parent(),
            "typical_start": if kairon::fork_parent().is_some() { "under a second (fork)" } else { "seconds (Firecracker)" },
        }),
    }
}

fn valid_path(p: &str) -> Result<String, String> {
    let p = p.trim();
    if !p.starts_with('/') || p.contains('\0') || p.len() > 1024 {
        return Err("path must be an absolute file path".into());
    }
    if p.split('/').any(|seg| seg == "..") {
        return Err("path must not contain '..'".into());
    }
    Ok(p.to_string())
}

// ── Core operations (shared by HTTP and tools) ─────────────────────

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CreateReq {
    #[serde(default)]
    pub ttl_minutes: Option<u64>,
    #[serde(default)]
    pub internet: bool,
    /// Wait for the sandbox to be ready (default true, up to 4 minutes).
    #[serde(default)]
    pub wait: Option<bool>,
}

#[derive(Debug)]
pub struct OpError(pub u16, pub String);

impl<E: std::fmt::Display> From<E> for OpError {
    fn from(e: E) -> Self {
        OpError(500, e.to_string())
    }
}

async fn visible(
    client: &kube::Client,
    role: &ApiRole,
    subject: &str,
) -> Result<Vec<Sandbox>, OpError> {
    let all = list_all(client).await?;
    Ok(all
        .into_iter()
        .filter(|s| !s.is_pool() && can_access(role, subject, s))
        .collect())
}

async fn find(
    client: &kube::Client,
    role: &ApiRole,
    subject: &str,
    id: &str,
) -> Result<Sandbox, OpError> {
    let all = list_all(client).await?;
    let sb = all
        .into_iter()
        .find(|s| s.id == id && !s.is_pool())
        .ok_or_else(|| OpError(404, format!("Sandbox {id} not found")))?;
    if !can_access(role, subject, &sb) {
        return Err(OpError(404, format!("Sandbox {id} not found")));
    }
    Ok(sb)
}

pub async fn op_create(
    client: &kube::Client,
    role: &ApiRole,
    subject: &str,
    req: &CreateReq,
) -> Result<Sandbox, OpError> {
    if *role == ApiRole::ReadOnly {
        return Err(OpError(
            403,
            "Creating sandboxes needs the write role".into(),
        ));
    }
    let owner = owner_key(subject);
    let mine = visible(client, &ApiRole::Write, &owner).await?;
    if mine.len() >= max_per_owner() {
        return Err(OpError(
            429,
            format!(
                "You already have {} sandboxes (limit {}). Destroy one first.",
                mine.len(),
                max_per_owner()
            ),
        ));
    }
    let ttl = req.ttl_minutes.unwrap_or(30).clamp(5, max_ttl_minutes());
    let wait = if req.wait.unwrap_or(true) {
        Duration::from_secs(env_u64("VEYRON_SANDBOX_WAIT_SECS", 240))
    } else {
        Duration::ZERO
    };
    note_use();
    Ok(claim(client, &owner, ttl, req.internet, wait).await?)
}

pub async fn op_exec(
    client: &kube::Client,
    role: &ApiRole,
    subject: &str,
    id: &str,
    command: &str,
    timeout_secs: u64,
) -> Result<ExecResult, OpError> {
    if command.trim().is_empty() {
        return Err(OpError(400, "command is required".into()));
    }
    let sb = find(client, role, subject, id).await?;
    if sb.status != "ready" {
        return Err(OpError(
            409,
            format!("Sandbox {id} is {}; try again shortly", sb.status),
        ));
    }
    Ok(exec(client, &sb, command, timeout_secs.clamp(1, 900)).await?)
}

// ── HTTP ────────────────────────────────────────────────────────────

/// Full-path routes for the long-lived router (sandbox boots and commands can
/// outlast the normal request timeout).
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/api/v1/sandboxes", get(list_handler).post(create_handler))
        .route(
            "/api/v1/sandboxes/:id",
            get(get_handler).delete(delete_handler),
        )
        .route("/api/v1/sandboxes/:id/exec", post(exec_handler))
        .route(
            "/api/v1/sandboxes/:id/files",
            get(read_handler).put(write_handler),
        )
        .with_state(state)
}

fn err(e: OpError) -> Response {
    let code = StatusCode::from_u16(e.0).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        code,
        Json(json!({"success": false, "status": e.0, "error": {"message": e.1}})),
    )
        .into_response()
}

fn ok(v: impl Serialize) -> Response {
    Json(json!({"success": true, "data": v})).into_response()
}

async fn client_of(state: &SharedState) -> kube::Client {
    state.read().await.kube_client.client()
}

async fn list_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
) -> Response {
    let client = client_of(&state).await;
    let all = match list_all(&client).await {
        Ok(v) => v,
        Err(e) => return err(e.into()),
    };
    let warm: Vec<&Sandbox> = all.iter().filter(|s| s.is_pool()).collect();
    let ready_warm = warm.iter().filter(|s| s.status == "warm").count();
    let items: Vec<&Sandbox> = all
        .iter()
        .filter(|s| !s.is_pool() && can_access(&auth.role, &auth.subject, s))
        .collect();
    let mut summary = backend_summary();
    summary["items"] = json!(items);
    summary["pool"] =
        json!({"target": pool_target(), "ready": ready_warm, "warming": warm.len() - ready_warm});
    summary["limit_per_owner"] = json!(max_per_owner());
    ok(summary)
}

async fn create_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    body: Option<Json<CreateReq>>,
) -> Response {
    let req = body.map(|b| b.0).unwrap_or_default();
    let client = client_of(&state).await;
    match op_create(&client, &auth.role, &auth.subject, &req).await {
        Ok(sb) => (
            StatusCode::CREATED,
            Json(json!({"success": true, "data": sb})),
        )
            .into_response(),
        Err(e) => err(e),
    }
}

async fn get_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Response {
    let client = client_of(&state).await;
    match find(&client, &auth.role, &auth.subject, &id).await {
        Ok(sb) => ok(sb),
        Err(e) => err(e),
    }
}

async fn delete_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Response {
    let client = client_of(&state).await;
    let sb = match find(&client, &auth.role, &auth.subject, &id).await {
        Ok(sb) => sb,
        Err(e) => return err(e),
    };
    match destroy(&client, &sb).await {
        Ok(()) => ok(json!({"id": id, "deleted": true})),
        Err(e) => err(e.into()),
    }
}

#[derive(Deserialize)]
struct ExecReq {
    command: String,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

async fn exec_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<ExecReq>,
) -> Response {
    let client = client_of(&state).await;
    match op_exec(
        &client,
        &auth.role,
        &auth.subject,
        &id,
        &req.command,
        req.timeout_secs.unwrap_or(60),
    )
    .await
    {
        Ok(r) => ok(r),
        Err(e) => err(e),
    }
}

#[derive(Deserialize)]
struct PathQuery {
    path: String,
}

async fn read_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Query(q): Query<PathQuery>,
) -> Response {
    use base64::Engine;
    let path = match valid_path(&q.path) {
        Ok(p) => p,
        Err(e) => return err(OpError(400, e)),
    };
    let client = client_of(&state).await;
    let sb = match find(&client, &auth.role, &auth.subject, &id).await {
        Ok(sb) => sb,
        Err(e) => return err(e),
    };
    match read_file(&client, &sb, &path).await {
        Ok(bytes) => ok(json!({
            "path": path,
            "size": bytes.len(),
            "content_b64": base64::engine::general_purpose::STANDARD.encode(&bytes),
        })),
        Err(e) => err(e.into()),
    }
}

#[derive(Deserialize)]
struct WriteReq {
    path: String,
    #[serde(default)]
    content_b64: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

fn decode_content(content_b64: Option<&str>, content: Option<&str>) -> Result<Vec<u8>, OpError> {
    use base64::Engine;
    let bytes = match (content_b64, content) {
        (Some(b), _) => base64::engine::general_purpose::STANDARD
            .decode(b.trim())
            .map_err(|e| OpError(400, format!("content_b64 is not valid base64: {e}")))?,
        (None, Some(t)) => t.as_bytes().to_vec(),
        (None, None) => return Err(OpError(400, "content or content_b64 is required".into())),
    };
    if bytes.len() > MAX_FILE_BYTES {
        return Err(OpError(
            413,
            format!("Files are limited to {} MiB", MAX_FILE_BYTES >> 20),
        ));
    }
    Ok(bytes)
}

async fn write_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<WriteReq>,
) -> Response {
    let path = match valid_path(&req.path) {
        Ok(p) => p,
        Err(e) => return err(OpError(400, e)),
    };
    let bytes = match decode_content(req.content_b64.as_deref(), req.content.as_deref()) {
        Ok(b) => b,
        Err(e) => return err(e),
    };
    let client = client_of(&state).await;
    let sb = match find(&client, &auth.role, &auth.subject, &id).await {
        Ok(sb) => sb,
        Err(e) => return err(e),
    };
    match write_file(&client, &sb, &path, &bytes).await {
        Ok(()) => ok(json!({"path": path, "size": bytes.len()})),
        Err(e) => err(e.into()),
    }
}

// ── AI tools ────────────────────────────────────────────────────────

fn sandbox_tool(name: &str, description: &str, params: Value) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        description: description.into(),
        kind: ToolKind::Sandbox,
        min_role: ApiRole::Write,
        parameters: params,
    }
}

pub fn tool_specs() -> Vec<ToolSpec> {
    let id = json!({"type": "string", "description": "Sandbox id from sandbox_create"});
    vec![
        sandbox_tool(
            "sandbox_create",
            "Start an isolated, disposable Linux VM (Ubuntu, Python 3) for running code. No internet unless asked. Deleted automatically after ttl_minutes.",
            json!({"type": "object", "properties": {
                "ttl_minutes": {"type": "integer", "minimum": 5, "maximum": 1440, "default": 30},
                "internet": {"type": "boolean", "default": false}
            }}),
        ),
        sandbox_tool(
            "sandbox_exec",
            "Run a bash command in your sandbox. Returns stdout, stderr and the exit code.",
            json!({"type": "object", "properties": {
                "sandbox_id": id,
                "command": {"type": "string"},
                "timeout_secs": {"type": "integer", "minimum": 1, "maximum": 900, "default": 60}
            }, "required": ["sandbox_id", "command"]}),
        ),
        sandbox_tool(
            "sandbox_write_file",
            "Write a file in your sandbox (text in `content`, or binary as base64 in `content_b64`). Up to 4 MiB.",
            json!({"type": "object", "properties": {
                "sandbox_id": id,
                "path": {"type": "string", "description": "Absolute path, e.g. /root/main.py"},
                "content": {"type": "string"},
                "content_b64": {"type": "string"}
            }, "required": ["sandbox_id", "path"]}),
        ),
        sandbox_tool(
            "sandbox_read_file",
            "Read a file from your sandbox (up to 4 MiB). Text comes back as `content`, binary as `content_b64`.",
            json!({"type": "object", "properties": {
                "sandbox_id": id,
                "path": {"type": "string"}
            }, "required": ["sandbox_id", "path"]}),
        ),
        sandbox_tool(
            "sandbox_destroy",
            "Delete your sandbox now.",
            json!({"type": "object", "properties": {"sandbox_id": id}, "required": ["sandbox_id"]}),
        ),
        sandbox_tool(
            "sandbox_list",
            "Your running sandboxes and when they expire.",
            json!({"type": "object", "properties": {}}),
        ),
    ]
}

fn out(ok: bool, content: Value) -> ToolOutput {
    ToolOutput {
        ok,
        content,
        proposal: None,
    }
}

fn fail(e: OpError) -> ToolOutput {
    out(false, json!({"error": e.1, "status": e.0}))
}

fn s_arg<'a>(args: &'a Value, k: &str) -> Option<&'a str> {
    args.get(k).and_then(Value::as_str)
}

pub async fn invoke_tool(ctx: &ToolCtx, name: &str, args: &Value) -> ToolOutput {
    use base64::Engine;
    let client = ctx.kube.client();
    let id = s_arg(args, "sandbox_id").unwrap_or("").trim();
    match name {
        "sandbox_create" => {
            let req = CreateReq {
                ttl_minutes: args.get("ttl_minutes").and_then(Value::as_u64),
                internet: args
                    .get("internet")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                wait: Some(true),
            };
            match op_create(&client, &ctx.role, &ctx.subject, &req).await {
                Ok(sb) => out(
                    true,
                    json!({
                        "sandbox_id": sb.id, "status": sb.status, "expires_at": sb.expires_at,
                        "internet": sb.internet,
                        "hint": if sb.status == "ready" { "Run commands with sandbox_exec." } else { "Still booting; retry sandbox_exec in a minute." }
                    }),
                ),
                Err(e) => fail(e),
            }
        }
        "sandbox_exec" => {
            let cmd = s_arg(args, "command").unwrap_or("");
            let t = args
                .get("timeout_secs")
                .and_then(Value::as_u64)
                .unwrap_or(60);
            match op_exec(&client, &ctx.role, &ctx.subject, id, cmd, t).await {
                Ok(r) => out(r.exit_code == 0, wrap_untrusted(json!(r))),
                Err(e) => fail(e),
            }
        }
        "sandbox_write_file" => {
            let path = match valid_path(s_arg(args, "path").unwrap_or("")) {
                Ok(p) => p,
                Err(e) => return fail(OpError(400, e)),
            };
            let bytes = match decode_content(s_arg(args, "content_b64"), s_arg(args, "content")) {
                Ok(b) => b,
                Err(e) => return fail(e),
            };
            let sb = match find(&client, &ctx.role, &ctx.subject, id).await {
                Ok(sb) => sb,
                Err(e) => return fail(e),
            };
            match write_file(&client, &sb, &path, &bytes).await {
                Ok(()) => out(true, json!({"path": path, "size": bytes.len()})),
                Err(e) => fail(e.into()),
            }
        }
        "sandbox_read_file" => {
            let path = match valid_path(s_arg(args, "path").unwrap_or("")) {
                Ok(p) => p,
                Err(e) => return fail(OpError(400, e)),
            };
            let sb = match find(&client, &ctx.role, &ctx.subject, id).await {
                Ok(sb) => sb,
                Err(e) => return fail(e),
            };
            match read_file(&client, &sb, &path).await {
                Ok(bytes) => {
                    let body = match String::from_utf8(bytes.clone()) {
                        Ok(text) if text.len() <= MAX_OUTPUT_CHARS => {
                            json!({"path": path, "size": bytes.len(), "content": text})
                        }
                        _ => json!({"path": path, "size": bytes.len(),
                            "content_b64": base64::engine::general_purpose::STANDARD.encode(&bytes)}),
                    };
                    out(true, wrap_untrusted(body))
                }
                Err(e) => fail(e.into()),
            }
        }
        "sandbox_destroy" => {
            let sb = match find(&client, &ctx.role, &ctx.subject, id).await {
                Ok(sb) => sb,
                Err(e) => return fail(e),
            };
            match destroy(&client, &sb).await {
                Ok(()) => out(true, json!({"sandbox_id": id, "deleted": true})),
                Err(e) => fail(e.into()),
            }
        }
        "sandbox_list" => match visible(&client, &ctx.role, &ctx.subject).await {
            Ok(v) => out(true, json!(v)),
            Err(e) => fail(e),
        },
        _ => fail(OpError(400, format!("Unknown tool {name}"))),
    }
}

// ── Leader tick: reap expired sandboxes, refill the warm pool ───────

static LAST_USE: once_cell::sync::Lazy<std::sync::Mutex<Option<std::time::Instant>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));

fn note_use() {
    if let Ok(mut g) = LAST_USE.lock() {
        *g = Some(std::time::Instant::now());
    }
}

/// The pool only stays warm while sandboxes are in use (or `VEYRON_SANDBOX_POOL_ALWAYS=1`),
/// so a cluster that never uses them does not pay for an idle VM.
fn pool_active(live: usize) -> bool {
    if matches!(
        std::env::var("VEYRON_SANDBOX_POOL_ALWAYS").as_deref(),
        Ok("1") | Ok("true")
    ) {
        return true;
    }
    if live > 0 {
        return true;
    }
    LAST_USE
        .lock()
        .ok()
        .and_then(|g| *g)
        .is_some_and(|t| t.elapsed() < Duration::from_secs(6 * 3600))
}

pub fn expired(sb: &Sandbox, now: chrono::DateTime<chrono::Utc>) -> bool {
    chrono::DateTime::parse_from_rfc3339(&sb.expires_at)
        .map(|t| t.with_timezone(&chrono::Utc) <= now)
        .unwrap_or(false)
}

pub async fn tick(state: &SharedState) {
    let client = client_of(state).await;
    let all = match list_all(&client).await {
        Ok(v) => v,
        Err(e) => {
            log::debug!("sandbox tick: {e}");
            return;
        }
    };
    let now = chrono::Utc::now();
    let mut live = 0;
    for sb in &all {
        if !sb.is_pool() && expired(sb, now) {
            log::info!("sandbox {} expired; deleting", sb.id);
            if let Err(e) = destroy(&client, sb).await {
                log::warn!("deleting expired sandbox {}: {e}", sb.id);
            }
        } else if !sb.is_pool() {
            live += 1;
        }
    }
    if backend() == Backend::KubeVirt {
        let warm = all.iter().filter(|s| s.is_pool()).count();
        let target = if pool_active(live) { pool_target() } else { 0 };
        if let Err(e) = kubevirt::reconcile_pool(&client, &all, warm, target).await {
            log::warn!("sandbox pool: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_key_ignores_ai_suffix() {
        assert_eq!(owner_key("alice (via Veyron AI)"), "alice");
        assert_eq!(owner_key("alice"), "alice");
    }

    #[test]
    fn access_is_owner_or_admin() {
        let sb = Sandbox {
            owner: "alice".into(),
            ..Default::default()
        };
        assert!(can_access(&ApiRole::Write, "alice (via Veyron AI)", &sb));
        assert!(!can_access(&ApiRole::Write, "bob", &sb));
        assert!(can_access(&ApiRole::Admin, "bob", &sb));
    }

    #[test]
    fn paths_must_be_absolute_without_dotdot() {
        assert!(valid_path("/root/a.py").is_ok());
        assert!(valid_path("root/a.py").is_err());
        assert!(valid_path("/root/../etc/shadow").is_err());
    }

    #[test]
    fn content_decodes_text_and_base64() {
        assert_eq!(decode_content(None, Some("hi")).unwrap(), b"hi");
        assert_eq!(decode_content(Some("aGk="), None).unwrap(), b"hi");
        assert!(decode_content(Some("!!"), None).is_err());
        assert!(decode_content(None, None).is_err());
    }

    #[test]
    fn expiry_is_checked_against_now() {
        let now = chrono::Utc::now();
        let mut sb = Sandbox {
            expires_at: (now - chrono::Duration::minutes(1)).to_rfc3339(),
            ..Default::default()
        };
        assert!(expired(&sb, now));
        sb.expires_at = (now + chrono::Duration::minutes(5)).to_rfc3339();
        assert!(!expired(&sb, now));
    }

    #[test]
    fn ids_look_like_sandbox_names() {
        let id = new_id();
        assert!(id.starts_with("sbx-") && id.len() == 12);
        assert!(crate::ai::registry::safe_name(&id).is_ok());
    }

    #[test]
    fn tools_are_sandbox_kind_and_need_write() {
        for t in tool_specs() {
            assert_eq!(t.kind, ToolKind::Sandbox);
            assert_eq!(t.min_role, ApiRole::Write);
        }
    }
}
