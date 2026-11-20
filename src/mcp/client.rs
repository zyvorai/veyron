// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! MCP client: connect Veyron AI to external MCP servers (Streamable HTTP).
//!
//! Servers live in the ConfigMap `veyron-ai-mcp-servers` and their bearer tokens
//! in the Secret `veyron-ai-mcp-tokens` (one key per server). Remote tools appear
//! as `ext.<server>.<tool>`. A tool counts as read-only only when the server marks
//! it `readOnlyHint: true` or an admin lists it in `read_only_tools`; anything else
//! is hidden unless `allow_write` is set, and even then it only drafts a proposal
//! whose single step calls `POST /api/v1/ai/mcp-servers/:server/call` (Admin).

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::future::join_all;
use k8s_openapi::ByteString;
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::Api;
use kube::api::{ObjectMeta, Patch, PatchParams};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ai::api::err;
use crate::ai::proposals::{BlastRadius, DiffLine, Proposal, ProposalStep};
use crate::ai::registry::{self, ToolCtx, ToolKind, ToolOutput, ToolSpec};
use crate::api::auth_context::AuthContext;
use crate::api::http_server::web::{ApiRole, SharedState};

/// Sent on every outgoing request; a Veyron MCP server seeing it lists only its own tools.
pub const HOP_HEADER: &str = "x-veyron-mcp-client";
const CM_NAME: &str = "veyron-ai-mcp-servers";
const SECRET_NAME: &str = "veyron-ai-mcp-tokens";
const TYPE_LABEL: &str = "ai-mcp-servers";
const PROTOCOL_VERSION: &str = "2025-06-18";
const SPEC_TIMEOUT: Duration = Duration::from_secs(2);
const CALL_TIMEOUT: Duration = Duration::from_secs(30);
const CACHE_OK: Duration = Duration::from_secs(60);
const CACHE_ERR: Duration = Duration::from_secs(30);
const MAX_SERVERS: usize = 16;
const MAX_TOOLS_PER_SERVER: usize = 64;
const MAX_BODY: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServer {
    pub name: String,
    pub url: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Offer tools that are not marked read-only, as proposals approved by an Admin.
    #[serde(default)]
    pub allow_write: bool,
    /// Tools to treat as read-only even without `readOnlyHint`.
    #[serde(default)]
    pub read_only_tools: Vec<String>,
    #[serde(default = "yes")]
    pub tls_verify: bool,
    #[serde(default)]
    pub description: String,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone)]
struct RemoteTool {
    name: String,
    description: String,
    schema: Value,
    read_only: bool,
}

// ── Config storage ────────────────────────────────────────────────

async fn load_servers(client: &kube::Client, ns: &str) -> Vec<McpServer> {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), ns);
    api.get_opt(CM_NAME)
        .await
        .ok()
        .flatten()
        .and_then(|cm| cm.data?.get("servers.json").cloned())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

async fn load_tokens(client: &kube::Client, ns: &str) -> BTreeMap<String, String> {
    let api: Api<Secret> = Api::namespaced(client.clone(), ns);
    api.get_opt(SECRET_NAME)
        .await
        .ok()
        .flatten()
        .and_then(|s| s.data)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| String::from_utf8(v.0).ok().map(|t| (k, t)))
        .collect()
}

fn labels() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("veyron.io/type".to_string(), TYPE_LABEL.to_string()),
        (
            "app.kubernetes.io/managed-by".to_string(),
            "veyron".to_string(),
        ),
    ])
}

fn validate(s: &McpServer) -> Result<(), String> {
    let n = &s.name;
    if n.is_empty()
        || n.len() > 32
        || !n
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || n.starts_with('-')
        || n.ends_with('-')
    {
        return Err(format!(
            "'{n}': server names use lowercase letters, digits and dashes (max 32)"
        ));
    }
    let u = reqwest::Url::parse(&s.url).map_err(|e| format!("'{n}': bad URL: {e}"))?;
    if !matches!(u.scheme(), "http" | "https") || u.host_str().is_none() {
        return Err(format!("'{n}': URL must be http(s)://host/…"));
    }
    if !u.username().is_empty() || u.password().is_some() {
        return Err(format!(
            "'{n}': put credentials in the token field, not the URL"
        ));
    }
    Ok(())
}

// ── Streamable HTTP transport ─────────────────────────────────────

fn http_client(verify: bool, timeout: Duration) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout.min(Duration::from_secs(5)))
        .danger_accept_invalid_certs(!verify)
        .redirect(reqwest::redirect::Policy::none())
        .build()
}

/// Pick the JSON-RPC response with `id` out of a JSON or SSE body.
fn parse_rpc_body(content_type: &str, body: &str, id: u64) -> Result<Value, String> {
    let pick = |v: Value| -> Option<Value> {
        match v {
            Value::Array(items) => items
                .into_iter()
                .find(|m| m.get("id").and_then(Value::as_u64) == Some(id)),
            m if m.get("id").and_then(Value::as_u64) == Some(id) => Some(m),
            _ => None,
        }
    };
    let msg = if content_type.contains("text/event-stream") {
        let mut found = None;
        for event in body.replace("\r\n", "\n").split("\n\n") {
            let data: Vec<&str> = event
                .lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .map(|l| l.strip_prefix(' ').unwrap_or(l))
                .collect();
            if data.is_empty() {
                continue;
            }
            if let Some(m) = serde_json::from_str(&data.join("\n")).ok().and_then(pick) {
                found = Some(m);
                break;
            }
        }
        found
    } else {
        serde_json::from_str(body).ok().and_then(pick)
    };
    let msg = msg.ok_or_else(|| "no JSON-RPC response in the server's reply".to_string())?;
    if let Some(e) = msg.get("error") {
        let text = e.get("message").and_then(Value::as_str).unwrap_or("error");
        return Err(format!("server error: {text}"));
    }
    Ok(msg.get("result").cloned().unwrap_or(Value::Null))
}

struct Session {
    http: reqwest::Client,
    url: String,
    token: Option<String>,
    session_id: Option<String>,
    next_id: u64,
}

impl Session {
    async fn open(
        srv: &McpServer,
        token: Option<String>,
        timeout: Duration,
    ) -> Result<Self, String> {
        let http = http_client(srv.tls_verify, timeout).map_err(|e| e.to_string())?;
        let mut s = Self {
            http,
            url: srv.url.clone(),
            token,
            session_id: None,
            next_id: 1,
        };
        s.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "veyron", "version": env!("CARGO_PKG_VERSION")}
            }),
        )
        .await?;
        s.post(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .await?;
        Ok(s)
    }

    async fn post(&mut self, body: Value) -> Result<(String, String), String> {
        let mut req = self
            .http
            .post(&self.url)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", PROTOCOL_VERSION)
            .header(HOP_HEADER, "1")
            .json(&body);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        if let Some(sid) = &self.session_id {
            req = req.header("mcp-session-id", sid);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("cannot reach server: {e}"))?;
        let status = resp.status();
        if let Some(sid) = resp
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
        {
            self.session_id = Some(sid.to_string());
        }
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        if bytes.len() > MAX_BODY {
            return Err("server reply too large".into());
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(format!(
                "server refused the token (HTTP {})",
                status.as_u16()
            ));
        }
        if !status.is_success() {
            return Err(format!("HTTP {}", status.as_u16()));
        }
        Ok((ct, String::from_utf8_lossy(&bytes).into_owned()))
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let (ct, body) = self
            .post(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
            .await?;
        parse_rpc_body(&ct, &body, id)
    }
}

fn to_remote_tools(srv: &McpServer, result: &Value) -> Vec<RemoteTool> {
    result
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|t| {
            let name = t.get("name")?.as_str()?.to_string();
            if name.is_empty()
                || name.len() > 64
                || !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return None;
            }
            let hint = t
                .pointer("/annotations/readOnlyHint")
                .and_then(Value::as_bool)
                == Some(true);
            Some(RemoteTool {
                read_only: hint || srv.read_only_tools.contains(&name),
                description: t
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .chars()
                    .take(500)
                    .collect(),
                schema: t
                    .get("inputSchema")
                    .filter(|s| s.is_object())
                    .cloned()
                    .unwrap_or_else(|| json!({"type": "object", "properties": {}})),
                name,
            })
        })
        .take(MAX_TOOLS_PER_SERVER)
        .collect()
}

type CacheEntry = (Instant, Result<Vec<RemoteTool>, String>);
static TOOL_CACHE: Lazy<Mutex<HashMap<String, CacheEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn cache_key(srv: &McpServer) -> String {
    format!(
        "{}|{}|{}|{}",
        srv.name,
        srv.url,
        srv.tls_verify,
        srv.read_only_tools.join(",")
    )
}

async fn remote_tools(
    srv: &McpServer,
    token: Option<String>,
    fresh: bool,
) -> Result<Vec<RemoteTool>, String> {
    let key = cache_key(srv);
    if !fresh {
        if let Some((at, res)) = TOOL_CACHE.lock().ok().and_then(|c| c.get(&key).cloned()) {
            let ttl = if res.is_ok() { CACHE_OK } else { CACHE_ERR };
            if at.elapsed() < ttl {
                return res;
            }
        }
    }
    let res = tokio::time::timeout(SPEC_TIMEOUT, async {
        let mut s = Session::open(srv, token, SPEC_TIMEOUT).await?;
        let r = s.request("tools/list", json!({})).await?;
        Ok::<_, String>(to_remote_tools(srv, &r))
    })
    .await
    .unwrap_or_else(|_| Err("timed out listing tools".into()));
    if let Ok(mut c) = TOOL_CACHE.lock() {
        c.insert(key, (Instant::now(), res.clone()));
    }
    res
}

async fn call_tool(
    srv: &McpServer,
    token: Option<String>,
    tool: &str,
    args: &Value,
) -> Result<Value, String> {
    tokio::time::timeout(CALL_TIMEOUT, async {
        let mut s = Session::open(srv, token, CALL_TIMEOUT).await?;
        s.request("tools/call", json!({"name": tool, "arguments": args}))
            .await
    })
    .await
    .unwrap_or_else(|_| Err("tool call timed out".into()))
}

// ── Registry integration ──────────────────────────────────────────

/// `ext.<server>.<tool>` (or `ext__<server>__<tool>`, the OpenAI-safe spelling).
pub fn parse_ext_name(name: &str) -> Option<(String, String)> {
    let (server, tool) = if let Some(rest) = name.strip_prefix("ext.") {
        rest.split_once('.')?
    } else {
        name.strip_prefix("ext__")?.split_once("__")?
    };
    (!server.is_empty() && !tool.is_empty()).then(|| (server.to_string(), tool.to_string()))
}

fn spec_for(srv: &McpServer, t: &RemoteTool) -> ToolSpec {
    let label = if srv.description.is_empty() {
        srv.name.clone()
    } else {
        srv.description.clone()
    };
    if t.read_only {
        ToolSpec {
            name: format!("ext.{}.{}", srv.name, t.name),
            description: format!("[MCP {label}] {}", t.description),
            kind: ToolKind::External,
            min_role: ApiRole::ReadOnly,
            parameters: t.schema.clone(),
        }
    } else {
        ToolSpec {
            name: format!("ext.{}.{}", srv.name, t.name),
            description: format!(
                "[MCP {label}] {} Changes an external system: drafts a proposal an Admin must approve.",
                t.description
            ),
            kind: ToolKind::Change,
            min_role: ApiRole::Admin,
            parameters: t.schema.clone(),
        }
    }
}

/// Tools from every enabled server; slow or broken servers are skipped (2 s budget).
pub async fn external_tool_specs(kube: &crate::kube::KubeClient, ns: &str) -> Vec<ToolSpec> {
    let client = kube.client();
    let servers: Vec<McpServer> = load_servers(&client, ns)
        .await
        .into_iter()
        .filter(|s| s.enabled)
        .collect();
    if servers.is_empty() {
        return Vec::new();
    }
    let tokens = load_tokens(&client, ns).await;
    let lists = join_all(
        servers
            .iter()
            .map(|s| remote_tools(s, tokens.get(&s.name).cloned(), false)),
    )
    .await;
    servers
        .iter()
        .zip(lists)
        .flat_map(|(srv, res)| {
            res.unwrap_or_default()
                .into_iter()
                .filter(|t| t.read_only || srv.allow_write)
                .map(|t| spec_for(srv, &t))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn out_err(msg: impl Into<String>) -> ToolOutput {
    ToolOutput {
        ok: false,
        content: json!({"error": msg.into()}),
        proposal: None,
    }
}

fn result_is_error(r: &Value) -> bool {
    r.get("isError").and_then(Value::as_bool) == Some(true)
}

/// Prefer `structuredContent`; otherwise join the text blocks.
fn result_payload(r: &Value) -> Value {
    if let Some(s) = r.get("structuredContent") {
        return s.clone();
    }
    let blocks = r
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let texts: Vec<Value> = blocks
        .iter()
        .map(|b| match b.get("type").and_then(Value::as_str) {
            Some("text") => {
                let t = b.get("text").and_then(Value::as_str).unwrap_or("");
                serde_json::from_str::<Value>(t).unwrap_or_else(|_| json!(t))
            }
            Some(other) => json!({"type": other, "note": "non-text content omitted"}),
            None => b.clone(),
        })
        .collect();
    match texts.len() {
        0 => Value::Null,
        1 => texts.into_iter().next().unwrap_or(Value::Null),
        _ => Value::Array(texts),
    }
}

pub async fn invoke_external(ctx: &ToolCtx, name: &str, args: &Value) -> ToolOutput {
    let Some((server, tool)) = parse_ext_name(name) else {
        return out_err(format!("Unknown tool {name}"));
    };
    let client = ctx.kube.client();
    let Some(srv) = load_servers(&client, &ctx.api_namespace)
        .await
        .into_iter()
        .find(|s| s.name == server && s.enabled)
    else {
        return out_err(format!(
            "MCP server '{server}' is not configured or is disabled"
        ));
    };
    let token = load_tokens(&client, &ctx.api_namespace)
        .await
        .get(&srv.name)
        .cloned();
    let tools = match remote_tools(&srv, token.clone(), false).await {
        Ok(t) => t,
        Err(e) => return out_err(format!("MCP server '{server}': {e}")),
    };
    let Some(rt) = tools.into_iter().find(|t| t.name == tool) else {
        return out_err(format!("MCP server '{server}' has no tool '{tool}'"));
    };
    let args = if args.is_object() {
        args.clone()
    } else {
        json!({})
    };

    if rt.read_only {
        return match call_tool(&srv, token, &tool, &args).await {
            Ok(r) => ToolOutput {
                ok: !result_is_error(&r),
                content: registry::wrap_untrusted(registry::compact(result_payload(&r))),
                proposal: None,
            },
            Err(e) => out_err(format!("MCP server '{server}': {e}")),
        };
    }
    if !srv.allow_write {
        return out_err(format!(
            "'{tool}' on MCP server '{server}' can change things, and write tools are disabled for this server"
        ));
    }
    registry::submit_proposal(ctx, write_proposal(ctx, &srv, &tool, &args)).await
}

fn write_proposal(ctx: &ToolCtx, srv: &McpServer, tool: &str, args: &Value) -> Proposal {
    let mut p = Proposal::new(
        format!("Run {tool} on MCP server {}", srv.name),
        &ctx.source,
        &ctx.subject,
    );
    p.rationale = format!("An AI agent asked to call the external tool '{tool}'.");
    let shown: String = args.to_string().chars().take(400).collect();
    p.steps.push(ProposalStep {
        method: "POST".into(),
        path: format!("/api/v1/ai/mcp-servers/{}/call", srv.name),
        body: Some(json!({"tool": tool, "arguments": args})),
        summary: format!("Call {tool} on {}", srv.name),
        min_role: "admin".into(),
    });
    p.diff.push(DiffLine {
        target: format!("mcp://{}", srv.name),
        field: tool.to_string(),
        before: String::new(),
        after: shown,
    });
    p.blast_radius = BlastRadius {
        reversible: false,
        notes: vec![format!(
            "Runs on an external system ({}). Veyron cannot verify or roll it back.",
            reqwest::Url::parse(&srv.url)
                .ok()
                .and_then(|u| u.host_str().map(str::to_string))
                .unwrap_or_default()
        )],
        ..Default::default()
    };
    p
}

// ── HTTP handlers ─────────────────────────────────────────────────

async fn namespace(state: &SharedState) -> (crate::kube::KubeClient, String) {
    let s = state.read().await;
    (s.kube_client.clone(), crate::ai::ai_namespace(&s.namespace))
}

/// Servers with token presence and a live tool listing (Admin).
pub async fn list_servers_handler(State(state): State<SharedState>) -> Response {
    let (kube, ns) = namespace(&state).await;
    let client = kube.client();
    let servers = load_servers(&client, &ns).await;
    let tokens = load_tokens(&client, &ns).await;
    let probes = join_all(servers.iter().map(|s| async {
        if s.enabled {
            Some(remote_tools(s, tokens.get(&s.name).cloned(), true).await)
        } else {
            None
        }
    }))
    .await;
    let data: Vec<Value> = servers
        .iter()
        .zip(probes)
        .map(|(s, probe)| {
            let mut v = serde_json::to_value(s).unwrap_or_default();
            v["has_token"] = json!(tokens.contains_key(&s.name));
            match probe {
                Some(Ok(tools)) => {
                    v["connected"] = json!(true);
                    v["tools"] = json!(
                        tools
                            .iter()
                            .map(|t| json!({
                                "name": t.name,
                                "read_only": t.read_only,
                                "offered": t.read_only || s.allow_write,
                            }))
                            .collect::<Vec<_>>()
                    );
                }
                Some(Err(e)) => {
                    v["connected"] = json!(false);
                    v["error"] = json!(e);
                }
                None => v["connected"] = json!(false),
            }
            v
        })
        .collect();
    Json(json!({"success": true, "data": data})).into_response()
}

#[derive(Debug, Deserialize)]
pub struct ServerInput {
    #[serde(flatten)]
    server: McpServer,
    /// Absent keeps the stored token, `""` removes it, anything else replaces it.
    #[serde(default)]
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PutServers {
    servers: Vec<ServerInput>,
}

/// Replace the server list (Admin).
pub async fn put_servers_handler(
    State(state): State<SharedState>,
    Json(req): Json<PutServers>,
) -> Response {
    if req.servers.len() > MAX_SERVERS {
        return err(
            StatusCode::BAD_REQUEST.as_u16(),
            &format!("At most {MAX_SERVERS} MCP servers"),
        );
    }
    let mut seen = std::collections::HashSet::new();
    for s in &req.servers {
        if let Err(e) = validate(&s.server) {
            return err(StatusCode::BAD_REQUEST.as_u16(), &e);
        }
        if !seen.insert(s.server.name.clone()) {
            return err(
                StatusCode::BAD_REQUEST.as_u16(),
                &format!("Duplicate server name '{}'", s.server.name),
            );
        }
    }
    let (kube, ns) = namespace(&state).await;
    let client = kube.client();

    let mut tokens = load_tokens(&client, &ns).await;
    tokens.retain(|k, _| seen.contains(k));
    for s in &req.servers {
        match s.token.as_deref().map(str::trim) {
            Some("") => {
                tokens.remove(&s.server.name);
            }
            Some(t) => {
                tokens.insert(s.server.name.clone(), t.to_string());
            }
            None => {}
        }
    }
    let servers: Vec<McpServer> = req.servers.into_iter().map(|s| s.server).collect();
    let pp = PatchParams::apply("veyron-ai").force();

    let secret = Secret {
        metadata: ObjectMeta {
            name: Some(SECRET_NAME.into()),
            labels: Some(labels()),
            ..Default::default()
        },
        data: Some(
            tokens
                .into_iter()
                .map(|(k, v)| (k, ByteString(v.into_bytes())))
                .collect(),
        ),
        type_: Some("Opaque".into()),
        ..Default::default()
    };
    let sapi: Api<Secret> = Api::namespaced(client.clone(), &ns);
    if let Err(e) = sapi.patch(SECRET_NAME, &pp, &Patch::Apply(&secret)).await {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            &format!("Could not save tokens: {e}"),
        );
    }

    let cm = ConfigMap {
        metadata: ObjectMeta {
            name: Some(CM_NAME.into()),
            labels: Some(labels()),
            ..Default::default()
        },
        data: Some(BTreeMap::from([(
            "servers.json".to_string(),
            serde_json::to_string_pretty(&servers).unwrap_or_else(|_| "[]".into()),
        )])),
        ..Default::default()
    };
    let capi: Api<ConfigMap> = Api::namespaced(client, &ns);
    if let Err(e) = capi.patch(CM_NAME, &pp, &Patch::Apply(&cm)).await {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            &format!("Could not save servers: {e}"),
        );
    }
    if let Ok(mut c) = TOOL_CACHE.lock() {
        c.clear();
    }
    list_servers_handler(State(state)).await
}

#[derive(Debug, Deserialize)]
pub struct CallReq {
    tool: String,
    #[serde(default)]
    arguments: Value,
}

/// Run any tool on a configured server (Admin; the execution step of write proposals).
pub async fn call_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Path(server): Path<String>,
    Json(req): Json<CallReq>,
) -> Response {
    let (kube, ns) = namespace(&state).await;
    let client = kube.client();
    let Some(srv) = load_servers(&client, &ns)
        .await
        .into_iter()
        .find(|s| s.name == server && s.enabled)
    else {
        return err(
            StatusCode::NOT_FOUND.as_u16(),
            &format!("MCP server '{server}' is not configured or is disabled"),
        );
    };
    let token = load_tokens(&client, &ns).await.get(&srv.name).cloned();
    let args = if req.arguments.is_object() {
        req.arguments
    } else {
        json!({})
    };
    log::info!(
        "MCP external tool call by {}: {server}.{}",
        auth.subject,
        req.tool
    );
    match call_tool(&srv, token, &req.tool, &args).await {
        Ok(r) if result_is_error(&r) => err(
            StatusCode::BAD_GATEWAY.as_u16(),
            &format!("{} reported an error: {}", server, result_payload(&r)),
        ),
        Ok(r) => Json(json!({"success": true, "data": result_payload(&r)})).into_response(),
        Err(e) => err(
            StatusCode::BAD_GATEWAY.as_u16(),
            &format!("MCP server '{server}': {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srv(name: &str, url: &str) -> McpServer {
        McpServer {
            name: name.into(),
            url: url.into(),
            enabled: true,
            allow_write: false,
            read_only_tools: vec!["lookup".into()],
            tls_verify: true,
            description: String::new(),
        }
    }

    #[test]
    fn ext_names_parse_in_both_spellings() {
        assert_eq!(
            parse_ext_name("ext.github.list_issues"),
            Some(("github".into(), "list_issues".into()))
        );
        assert_eq!(
            parse_ext_name("ext__github__list_issues"),
            Some(("github".into(), "list_issues".into()))
        );
        assert_eq!(parse_ext_name("ext.github"), None);
        assert_eq!(parse_ext_name("list_vms"), None);
    }

    #[test]
    fn server_validation() {
        assert!(validate(&srv("github", "https://mcp.example.com/mcp")).is_ok());
        assert!(validate(&srv("Bad_Name", "https://x")).is_err());
        assert!(validate(&srv("ok", "ftp://x")).is_err());
        assert!(validate(&srv("ok", "https://u:p@x/mcp")).is_err());
    }

    #[test]
    fn rpc_bodies_json_and_sse() {
        let j = r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[]}}"#;
        assert_eq!(
            parse_rpc_body("application/json", j, 2).unwrap(),
            json!({"tools": []})
        );
        let sse = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n\nevent: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":3,\"result\":{\"ok\":1}}\n\n";
        assert_eq!(
            parse_rpc_body("text/event-stream", sse, 3).unwrap(),
            json!({"ok": 1})
        );
        let e = r#"{"jsonrpc":"2.0","id":4,"error":{"code":-32601,"message":"nope"}}"#;
        assert!(
            parse_rpc_body("application/json", e, 4)
                .unwrap_err()
                .contains("nope")
        );
    }

    #[test]
    fn tools_are_read_only_only_when_marked() {
        let s = srv("x", "https://x/mcp");
        let tools = to_remote_tools(
            &s,
            &json!({"tools": [
                {"name": "get_thing", "annotations": {"readOnlyHint": true}},
                {"name": "lookup"},
                {"name": "delete_thing", "inputSchema": {"type": "object"}},
                {"name": "bad name"}
            ]}),
        );
        let ro: Vec<_> = tools
            .iter()
            .map(|t| (t.name.as_str(), t.read_only))
            .collect();
        assert_eq!(
            ro,
            vec![
                ("get_thing", true),
                ("lookup", true),
                ("delete_thing", false)
            ]
        );
        let spec = spec_for(&s, &tools[2]);
        assert!(spec.kind == ToolKind::Change && spec.min_role == ApiRole::Admin);
    }

    #[test]
    fn payload_prefers_structured_then_text() {
        assert_eq!(
            result_payload(&json!({"structuredContent": {"a": 1}})),
            json!({"a": 1})
        );
        assert_eq!(
            result_payload(&json!({"content": [{"type": "text", "text": "{\"b\":2}"}]})),
            json!({"b": 2})
        );
        assert_eq!(
            result_payload(&json!({"content": [{"type": "text", "text": "hi"}]})),
            json!("hi")
        );
    }
}
