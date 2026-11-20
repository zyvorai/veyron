// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Optional OpenAI-compatible LLM layer for Copilot.
//!
//! `VEYRON_AI_URL`, `VEYRON_AI_API_KEY`, `VEYRON_AI_MODEL` (default `gpt-4o-mini`).
//! OpenRouter (Zyra): also accepts `OPENROUTER_API_KEY`, `ANTHROPIC_AUTH_TOKEN` (`sk-or-v1-…`),
//! `ANTHROPIC_BASE_URL` / `OPENROUTER_API_URL`, and `ANTHROPIC_MODEL` / `OPENROUTER_MODEL`.
//! `VEYRON_AI_MODE`: `off` | `paraphrase` | `routing` | `agent`.

use super::intent::{self, CopilotIntent};
use super::tools::{ToolInvokeArgs, parse_tool_args, tool_definitions};
use super::{CopilotChatMessage, CopilotResponse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiMode {
    Off,
    Paraphrase,
    Routing,
    Agent,
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClassifiedIntent {
    pub intent: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub vm_name: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ParaphraseOut {
    summary: Option<String>,
    recommendations: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ChatResponse {
    choices: Option<Vec<ChatChoice>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ChatChoice {
    message: Option<ChatMessageOut>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ChatMessageOut {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallOut>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ToolCallOut {
    id: Option<String>,
    function: Option<ToolFunctionOut>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ToolFunctionOut {
    name: Option<String>,
    arguments: Option<String>,
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn is_openrouter_key(key: &str) -> bool {
    key.starts_with("sk-or-")
}

fn normalize_openai_base_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    // Block non-http(s) schemes and private/metadata hosts to prevent SSRF via env vars.
    let lower = trimmed.to_lowercase();
    let safe = (lower.starts_with("https://") || lower.starts_with("http://"))
        && !lower.contains("169.254.")
        && !lower.contains("metadata.")
        && !lower.contains("localhost")
        && !lower.starts_with("http://127.");
    if !safe {
        log::warn!("LLM base URL blocked (unsafe scheme or host): {trimmed}");
        return "https://api.openai.com/v1".to_string();
    }
    let mut base = trimmed.to_string();
    if base.contains("openrouter.ai/api") && !base.ends_with("/v1") {
        base.push_str("/v1");
    }
    base
}

/// Strip CR/LF characters that would allow HTTP header injection.
fn sanitize_header_value(v: &str) -> String {
    v.chars().filter(|c| *c != '\r' && *c != '\n').collect()
}

/// `VEYRON_AI_PROVIDER=anthropic` selects Anthropic's native Messages API instead of the
/// OpenAI-compatible `/chat/completions` format (anything else keeps OpenAI-compatible).
fn provider_env_is_anthropic() -> bool {
    env_nonempty("VEYRON_AI_PROVIDER").is_some_and(|p| p.eq_ignore_ascii_case("anthropic"))
}

fn resolve_api_key() -> Option<String> {
    env_nonempty("VEYRON_AI_API_KEY")
        .or_else(|| env_nonempty("ANTHROPIC_API_KEY").filter(|_| provider_env_is_anthropic()))
        .or_else(|| env_nonempty("OPENROUTER_API_KEY"))
        .or_else(|| env_nonempty("ANTHROPIC_AUTH_TOKEN").filter(|k| is_openrouter_key(k)))
}

fn resolve_base_url(key: &str) -> String {
    if provider_env_is_anthropic() || key.starts_with("sk-ant-") {
        // Same SSRF screen as the OpenAI path; a blocked URL falls back to Anthropic, not OpenAI.
        return match env_nonempty("VEYRON_AI_URL").map(|u| normalize_openai_base_url(&u)) {
            Some(u) if u != "https://api.openai.com/v1" => u,
            _ => ANTHROPIC_DEFAULT_BASE.to_string(),
        };
    }
    if let Some(url) = env_nonempty("VEYRON_AI_URL")
        .or_else(|| env_nonempty("OPENROUTER_API_URL"))
        .or_else(|| env_nonempty("OPENROUTER_BASE_URL"))
        .or_else(|| env_nonempty("ANTHROPIC_BASE_URL"))
    {
        return normalize_openai_base_url(&url);
    }
    if is_openrouter_key(key) {
        return "https://openrouter.ai/api/v1".to_string();
    }
    "https://api.openai.com/v1".to_string()
}

fn resolve_model(key: &str) -> String {
    env_nonempty("VEYRON_AI_MODEL")
        .or_else(|| env_nonempty("OPENROUTER_MODEL"))
        .or_else(|| env_nonempty("ANTHROPIC_MODEL"))
        .unwrap_or_else(|| {
            if provider_env_is_anthropic() || key.starts_with("sk-ant-") {
                ANTHROPIC_DEFAULT_MODEL.into()
            } else if is_openrouter_key(key) {
                "openrouter/free".into()
            } else {
                "gpt-4o-mini".into()
            }
        })
}

static RUNTIME_OVERRIDE: once_cell::sync::Lazy<std::sync::RwLock<Option<LlmConfig>>> =
    once_cell::sync::Lazy::new(|| std::sync::RwLock::new(None));

/// Point Veyron AI at a different endpoint at runtime (e.g. a model served in-cluster).
/// `None` reverts to the `VEYRON_AI_*` environment.
pub fn set_runtime_override(cfg: Option<LlmConfig>) {
    if let Ok(mut g) = RUNTIME_OVERRIDE.write() {
        *g = cfg;
    }
}

pub fn runtime_override() -> Option<LlmConfig> {
    RUNTIME_OVERRIDE.read().ok().and_then(|g| g.clone())
}

pub fn llm_config() -> Option<LlmConfig> {
    runtime_override().or_else(env_llm_config)
}

/// The LLM configured through environment variables, ignoring any in-cluster override.
pub fn env_llm_config() -> Option<LlmConfig> {
    // Self-hosted OpenAI-compatible servers (vLLM, llama.cpp, Ollama) usually need no key.
    let key = resolve_api_key().or_else(|| env_nonempty("VEYRON_AI_URL").map(|_| String::new()))?;
    let base_url = resolve_base_url(&key);
    let model = resolve_model(&key);
    Some(LlmConfig {
        base_url,
        api_key: key,
        model,
    })
}

pub fn ai_mode() -> AiMode {
    let explicit = std::env::var("VEYRON_AI_MODE")
        .ok()
        .map(|s| s.to_lowercase());
    match explicit.as_deref() {
        Some("off") | Some("0") | Some("false") => AiMode::Off,
        Some("routing") => AiMode::Routing,
        Some("agent") => AiMode::Agent,
        Some("paraphrase") => AiMode::Paraphrase,
        Some(_) => AiMode::Paraphrase,
        None => {
            if llm_config().is_some() {
                AiMode::Paraphrase
            } else {
                AiMode::Off
            }
        }
    }
}

pub fn max_tool_rounds() -> u32 {
    std::env::var("VEYRON_AI_MAX_TOOL_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(6)
}

/// In-cluster models usually run on CPU, where reading a long prompt alone can take minutes.
const IN_CLUSTER_TIMEOUT_SECS: u64 = 300;

pub fn timeout_secs() -> u64 {
    std::env::var("VEYRON_AI_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            if runtime_override().is_some() {
                IN_CLUSTER_TIMEOUT_SECS
            } else {
                30
            }
        })
}

pub fn ai_rate_limit_per_min() -> u64 {
    std::env::var("VEYRON_AI_RATE_LIMIT_PER_MIN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
}

pub fn status_snapshot() -> super::CopilotStatusResponse {
    let cfg = llm_config();
    super::CopilotStatusResponse {
        llm_configured: cfg.is_some(),
        mode: format!("{:?}", ai_mode()).to_lowercase(),
        model: cfg.as_ref().map(|c| c.model.clone()),
    }
}

/// Rewrite summary and recommendations when LLM is configured and mode allows paraphrase.
#[cfg(feature = "web")]
pub async fn maybe_enhance_response(mut resp: CopilotResponse) -> CopilotResponse {
    if !matches!(
        ai_mode(),
        AiMode::Paraphrase | AiMode::Routing | AiMode::Agent
    ) {
        return resp;
    }
    let Some(cfg) = llm_config() else {
        return resp;
    };
    if resp.summary.is_empty() && resp.recommendations.is_empty() {
        return resp;
    }

    let prompt = format!(
        "You are Veyron Copilot for KubeVirt. Rewrite ONLY the summary and recommendations \
         in plain language for an operator. Do not invent cluster facts. Keep the same meaning.\n\n\
         Module: {}\nTitle: {}\nSummary: {}\nRecommendations:\n{}\n\n\
         Reply as JSON: {{\"summary\":\"...\",\"recommendations\":[\"...\"]}}",
        resp.module,
        resp.title,
        resp.summary,
        resp.recommendations.join("\n")
    );

    match chat_json(&cfg, &prompt).await {
        Ok((summary, recs)) => {
            if !summary.trim().is_empty() {
                resp.summary = summary;
            }
            if !recs.is_empty() {
                resp.recommendations = recs;
            }
        }
        Err(e) => log::debug!("VEYRON_AI paraphrase skipped: {e}"),
    }
    resp
}

#[cfg(not(feature = "web"))]
pub async fn maybe_enhance_response(resp: CopilotResponse) -> CopilotResponse {
    resp
}

/// LLM intent classification with keyword fallback handled by caller.
#[cfg(feature = "web")]
pub async fn classify_intent(
    query: &str,
    history: &[CopilotChatMessage],
) -> Option<ClassifiedIntent> {
    let cfg = llm_config()?;
    let history_text: String = history
        .iter()
        .rev()
        .take(6)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Classify this Veyron Copilot question. Reply JSON only: \
         {{\"intent\":\"vm_doctor|scheduling_explainer|backup_advisor|cost_advisor|network_lens|\
         guest_inspector|storage_doctor|security_sentinel|performance_advisor|quota_advisor|\
         catalog_advisor|velero_dr_advisor|list_unhealthy_vms|explain_error|general_help\",\
         \"namespace\":null,\"vm_name\":null,\"confidence\":0.9}}\n\n\
         History:\n{history_text}\n\nQuestion: {query}"
    );

    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            {"role": "system", "content": "Respond with valid JSON only."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.1
    });

    let text = post_chat(&cfg, &body).await.ok()?;
    parse_classified(&text)
}

#[cfg(not(feature = "web"))]
pub async fn classify_intent(
    _query: &str,
    _history: &[CopilotChatMessage],
) -> Option<ClassifiedIntent> {
    None
}

pub fn classified_to_intent(c: &ClassifiedIntent) -> CopilotIntent {
    match c.intent.as_str() {
        "vm_doctor" => CopilotIntent::VmDoctor,
        "scheduling_explainer" => CopilotIntent::SchedulingExplainer,
        "list_unhealthy_vms" => CopilotIntent::UnhealthyFleet,
        "backup_advisor" => CopilotIntent::BackupAdvisor,
        "cost_advisor" => CopilotIntent::CostAdvisor,
        "network_lens" => CopilotIntent::NetworkLens,
        "guest_inspector" => CopilotIntent::GuestInspector,
        "storage_doctor" => CopilotIntent::StorageDoctor,
        "security_sentinel" => CopilotIntent::SecuritySentinel,
        "performance_advisor" => CopilotIntent::PerformanceAdvisor,
        "quota_advisor" => CopilotIntent::QuotaAdvisor,
        "catalog_advisor" => CopilotIntent::CatalogAdvisor,
        "velero_dr_advisor" => CopilotIntent::VeleroDrAdvisor,
        "explain_error" => CopilotIntent::ErrorExplainer,
        _ => CopilotIntent::GeneralHelp,
    }
}

#[derive(Debug, Clone)]
pub struct AgentToolCall {
    pub name: String,
    pub args: ToolInvokeArgs,
}

/// One step of the agent loop: either a final answer or a batch of tool calls.
#[derive(Debug, Clone)]
pub enum AgentStep {
    Final(String),
    Calls {
        /// The assistant message (with `tool_calls`) to append to the transcript
        /// before the matching `tool` result messages.
        assistant: serde_json::Value,
        /// `(tool_call_id, call)` pairs.
        calls: Vec<(String, AgentToolCall)>,
    },
}

const AGENT_SYSTEM_PROMPT: &str = "You are Veyron Copilot for KubeVirt. Use tools to fetch cluster facts, \
    call several tools across turns when needed, and then answer concisely from the tool results. \
    Never invent evidence; if a tool returns nothing, say so. Prefer specific tools over guessing.";

/// Build the initial transcript (system + recent history + user query).
pub fn agent_messages(query: &str, history: &[CopilotChatMessage]) -> Vec<serde_json::Value> {
    let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({
        "role": "system",
        "content": AGENT_SYSTEM_PROMPT
    })];
    for m in history.iter().rev().take(8).rev() {
        messages.push(serde_json::json!({"role": m.role, "content": m.content}));
    }
    messages.push(serde_json::json!({"role": "user", "content": query}));
    messages
}

/// Run one LLM turn over `messages` with tools enabled.
/// Returns `None` when no LLM is configured or the request fails.
#[cfg(feature = "web")]
pub async fn agent_step(messages: &[serde_json::Value]) -> Option<AgentStep> {
    let cfg = llm_config()?;
    let tools: Vec<serde_json::Value> = tool_definitions()
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters
                }
            })
        })
        .collect();

    let body = serde_json::json!({
        "model": cfg.model,
        "messages": messages,
        "tools": tools,
        "tool_choice": "auto",
        "temperature": 0.2
    });

    let resp = post_chat_raw(&cfg, &body).await.ok()?;
    let choice = resp.choices?.into_iter().next()?;
    let msg = choice.message?;
    let raw_calls = msg.tool_calls.unwrap_or_default();
    let mut calls = Vec::new();
    let mut wire_calls = Vec::new();
    for (i, tc) in raw_calls.into_iter().enumerate() {
        let Some(func) = tc.function else { continue };
        let Some(name) = func.name else { continue };
        let raw_args = func.arguments.unwrap_or_default();
        let id = tc.id.unwrap_or_else(|| format!("call_{i}"));
        wire_calls.push(serde_json::json!({
            "id": id,
            "type": "function",
            "function": {"name": name, "arguments": raw_args}
        }));
        calls.push((
            id,
            AgentToolCall {
                name,
                args: parse_tool_args(&raw_args),
            },
        ));
    }
    if calls.is_empty() {
        return Some(AgentStep::Final(msg.content.unwrap_or_default()));
    }
    Some(AgentStep::Calls {
        assistant: serde_json::json!({
            "role": "assistant",
            "content": msg.content,
            "tool_calls": wire_calls
        }),
        calls,
    })
}

#[cfg(not(feature = "web"))]
pub async fn agent_step(_messages: &[serde_json::Value]) -> Option<AgentStep> {
    None
}

#[cfg(feature = "web")]
async fn chat_json(cfg: &LlmConfig, prompt: &str) -> anyhow::Result<(String, Vec<String>)> {
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            {"role": "system", "content": "Respond with valid JSON only."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.2
    });
    let text = post_chat(cfg, &body).await?;
    let parsed: ParaphraseOut = parse_json_text(&text).unwrap_or(ParaphraseOut {
        summary: Some(text),
        recommendations: None,
    });
    Ok((
        parsed.summary.unwrap_or_default(),
        parsed.recommendations.unwrap_or_default(),
    ))
}

#[cfg(feature = "web")]
async fn post_chat(cfg: &LlmConfig, body: &serde_json::Value) -> anyhow::Result<String> {
    let chat: ChatResponse = post_chat_raw(cfg, body).await?;
    Ok(chat
        .choices
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .unwrap_or_default())
}

/// Optional attribution headers some gateways (OpenRouter) use.
pub fn extra_headers() -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if let Some(referer) =
        env_nonempty("VEYRON_AI_HTTP_REFERER").or_else(|| env_nonempty("OPENROUTER_HTTP_REFERER"))
    {
        out.push(("HTTP-Referer", sanitize_header_value(&referer)));
    }
    if let Some(title) =
        env_nonempty("VEYRON_AI_APP_TITLE").or_else(|| env_nonempty("OPENROUTER_APP_TITLE"))
    {
        out.push(("X-Title", sanitize_header_value(&title)));
    }
    out
}

const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_DEFAULT_BASE: &str = "https://api.anthropic.com";
const ANTHROPIC_DEFAULT_MODEL: &str = "claude-sonnet-5-5";
const ANTHROPIC_DEFAULT_MAX_TOKENS: u64 = 2048;

/// Whether `cfg` should be spoken to with Anthropic's Messages API: an Anthropic URL or
/// `sk-ant-` key, or `VEYRON_AI_PROVIDER=anthropic` when no in-cluster override is active.
pub fn is_anthropic(cfg: &LlmConfig) -> bool {
    cfg.base_url.contains("anthropic.com")
        || cfg.api_key.starts_with("sk-ant-")
        || (provider_env_is_anthropic() && runtime_override().is_none())
}

fn anthropic_messages_url(base: &str) -> String {
    let b = base.trim_end_matches('/');
    if b.ends_with("/v1/messages") {
        b.to_string()
    } else if b.ends_with("/v1") {
        format!("{b}/messages")
    } else {
        format!("{b}/v1/messages")
    }
}

fn append_blocks(msgs: &mut Vec<serde_json::Value>, role: &str, blocks: Vec<serde_json::Value>) {
    if blocks.is_empty() {
        return;
    }
    // The API wants alternating roles: fold consecutive same-role messages together
    // (e.g. several tool results answering one assistant turn become one user message).
    if let Some(last) = msgs.last_mut() {
        if last["role"] == role {
            if let Some(arr) = last["content"].as_array_mut() {
                arr.extend(blocks);
                return;
            }
        }
    }
    msgs.push(serde_json::json!({"role": role, "content": blocks}));
}

/// Translate the agent's OpenAI-format chat body into an Anthropic Messages request.
pub(crate) fn to_anthropic_request(body: &serde_json::Value) -> serde_json::Value {
    use serde_json::{Value, json};
    let text_of = |m: &Value| m["content"].as_str().unwrap_or("").to_string();
    let mut system: Vec<String> = Vec::new();
    let mut msgs: Vec<Value> = Vec::new();
    for m in body["messages"].as_array().into_iter().flatten() {
        match m["role"].as_str().unwrap_or("user") {
            "system" => {
                let t = text_of(m);
                if !t.is_empty() {
                    system.push(t);
                }
            }
            "assistant" => {
                let mut blocks = Vec::new();
                let t = text_of(m);
                if !t.is_empty() {
                    blocks.push(json!({"type": "text", "text": t}));
                }
                for tc in m["tool_calls"].as_array().into_iter().flatten() {
                    let args = tc["function"]["arguments"].as_str().unwrap_or("{}");
                    let input: Value = serde_json::from_str(args).unwrap_or_else(|_| json!({}));
                    blocks.push(json!({
                        "type": "tool_use",
                        "id": tc["id"],
                        "name": tc["function"]["name"],
                        "input": input
                    }));
                }
                append_blocks(&mut msgs, "assistant", blocks);
            }
            "tool" => append_blocks(
                &mut msgs,
                "user",
                vec![json!({
                    "type": "tool_result",
                    "tool_use_id": m["tool_call_id"],
                    "content": text_of(m)
                })],
            ),
            _ => {
                let t = text_of(m);
                if !t.is_empty() {
                    append_blocks(&mut msgs, "user", vec![json!({"type": "text", "text": t})]);
                }
            }
        }
    }
    let max_tokens = std::env::var("VEYRON_AI_MAX_TOKENS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(ANTHROPIC_DEFAULT_MAX_TOKENS);
    let mut req = json!({
        "model": body["model"],
        "max_tokens": max_tokens,
        "messages": msgs,
    });
    if !system.is_empty() {
        req["system"] = json!(system.join("\n\n"));
    }
    if let Some(t) = body.get("temperature").filter(|t| t.is_number()) {
        req["temperature"] = t.clone();
    }
    let tools: Vec<Value> = body["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|t| {
            json!({
                "name": t["function"]["name"],
                "description": t["function"]["description"],
                "input_schema": t["function"]["parameters"]
            })
        })
        .collect();
    if !tools.is_empty() {
        req["tools"] = json!(tools);
        req["tool_choice"] = json!({"type": "auto"});
    }
    req
}

/// Translate an Anthropic Messages response into the OpenAI-shaped `ChatResponse` the
/// rest of the Copilot reads.
fn from_anthropic_response(v: &serde_json::Value) -> anyhow::Result<ChatResponse> {
    use serde_json::json;
    let mut text = String::new();
    let mut calls = Vec::new();
    for b in v["content"].as_array().into_iter().flatten() {
        match b["type"].as_str() {
            Some("text") => text.push_str(b["text"].as_str().unwrap_or("")),
            Some("tool_use") => calls.push(json!({
                "id": b["id"],
                "type": "function",
                "function": {"name": b["name"], "arguments": b["input"].to_string()}
            })),
            _ => {}
        }
    }
    let mut message = json!({"content": if text.is_empty() { json!(null) } else { json!(text) }});
    if !calls.is_empty() {
        message["tool_calls"] = json!(calls);
    }
    Ok(serde_json::from_value(
        json!({"choices": [{"message": message}]}),
    )?)
}

#[cfg(feature = "web")]
async fn post_anthropic(cfg: &LlmConfig, body: &serde_json::Value) -> anyhow::Result<ChatResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs()))
        .build()?;
    let base = if cfg.base_url.is_empty() {
        ANTHROPIC_DEFAULT_BASE
    } else {
        cfg.base_url.as_str()
    };
    let resp = client
        .post(anthropic_messages_url(base))
        .header("x-api-key", &cfg.api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .json(&to_anthropic_request(body))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("LLM HTTP {}", resp.status());
    }
    from_anthropic_response(&resp.json().await?)
}

#[cfg(feature = "web")]
async fn post_chat_raw(cfg: &LlmConfig, body: &serde_json::Value) -> anyhow::Result<ChatResponse> {
    if is_anthropic(cfg) {
        return post_anthropic(cfg, body).await;
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs()))
        .build()?;
    let url = if cfg.base_url.contains("/chat/completions") {
        cfg.base_url.clone()
    } else {
        format!("{}/chat/completions", cfg.base_url)
    };
    let mut req = client.post(url);
    if !cfg.api_key.is_empty() {
        req = req.bearer_auth(&cfg.api_key);
    }
    if let Some(referer) =
        env_nonempty("VEYRON_AI_HTTP_REFERER").or_else(|| env_nonempty("OPENROUTER_HTTP_REFERER"))
    {
        req = req.header("HTTP-Referer", sanitize_header_value(&referer));
    }
    if let Some(title) =
        env_nonempty("VEYRON_AI_APP_TITLE").or_else(|| env_nonempty("OPENROUTER_APP_TITLE"))
    {
        req = req.header("X-Title", sanitize_header_value(&title));
    }
    let resp = req.json(body).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("LLM HTTP {}", resp.status());
    }
    Ok(resp.json().await?)
}

#[cfg(feature = "web")]
fn parse_classified(text: &str) -> Option<ClassifiedIntent> {
    serde_json::from_str(text.trim())
        .ok()
        .or_else(|| parse_json_text(text))
}

fn parse_json_text<T: serde::de::DeserializeOwned>(text: &str) -> Option<T> {
    let stripped = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(stripped).ok()
}

/// Resolve intent using LLM routing when enabled, else keywords.
pub async fn resolve_intent(query: &str, history: &[CopilotChatMessage]) -> CopilotIntent {
    if matches!(ai_mode(), AiMode::Routing | AiMode::Agent) {
        if let Some(c) = classify_intent(query, history).await {
            if c.confidence.unwrap_or(0.5) >= 0.4 {
                return classified_to_intent(&c);
            }
        }
    }
    intent::detect_intent(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_rate_limit_default_when_unset() {
        if std::env::var("VEYRON_AI_RATE_LIMIT_PER_MIN").is_err() {
            assert_eq!(ai_rate_limit_per_min(), 20);
        }
    }

    #[test]
    fn max_tool_rounds_default() {
        if std::env::var("VEYRON_AI_MAX_TOOL_ROUNDS").is_err() {
            assert_eq!(max_tool_rounds(), 6);
        }
    }

    #[test]
    fn normalize_openrouter_base_appends_v1() {
        assert_eq!(
            normalize_openai_base_url("https://openrouter.ai/api"),
            "https://openrouter.ai/api/v1"
        );
        assert_eq!(
            normalize_openai_base_url("https://openrouter.ai/api/v1/"),
            "https://openrouter.ai/api/v1"
        );
    }

    #[test]
    fn anthropic_request_translates_the_agent_transcript() {
        use serde_json::json;
        let body = json!({
            "model": "claude-sonnet-5-5",
            "temperature": 0.2,
            "tool_choice": "auto",
            "tools": [{"type": "function", "function": {
                "name": "propose_vm_action", "description": "d",
                "parameters": {"type": "object", "properties": {}}}}],
            "messages": [
                {"role": "system", "content": "be brief"},
                {"role": "user", "content": "restart web-1"},
                {"role": "assistant", "content": null, "tool_calls": [
                    {"id": "t1", "type": "function", "function": {"name": "a", "arguments": "{\"x\":1}"}},
                    {"id": "t2", "type": "function", "function": {"name": "b", "arguments": "not json"}}]},
                {"role": "tool", "tool_call_id": "t1", "content": "r1"},
                {"role": "tool", "tool_call_id": "t2", "content": "r2"}
            ]
        });
        let r = to_anthropic_request(&body);
        assert_eq!(r["model"], "claude-sonnet-5-5");
        assert_eq!(r["system"], "be brief");
        assert!(r["max_tokens"].as_u64().unwrap() > 0);
        assert_eq!(r["tool_choice"], json!({"type": "auto"}));
        assert_eq!(r["tools"][0]["name"], "propose_vm_action");
        assert!(r["tools"][0]["input_schema"].is_object());
        let m = r["messages"].as_array().unwrap();
        assert_eq!(
            m.len(),
            3,
            "user, assistant(tool_use x2), user(tool_result x2)"
        );
        assert_eq!(m[0]["role"], "user");
        assert_eq!(m[1]["role"], "assistant");
        assert_eq!(
            m[1]["content"][0],
            json!({"type": "tool_use", "id": "t1", "name": "a", "input": {"x": 1}})
        );
        assert_eq!(
            m[1]["content"][1]["input"],
            json!({}),
            "unparseable args become an empty object"
        );
        assert_eq!(m[2]["role"], "user");
        assert_eq!(m[2]["content"][0]["type"], "tool_result");
        assert_eq!(m[2]["content"][1]["tool_use_id"], "t2");
    }

    #[test]
    fn anthropic_response_becomes_an_openai_shaped_choice() {
        use serde_json::json;
        let r = from_anthropic_response(&json!({"content": [
            {"type": "text", "text": "Let me check. "},
            {"type": "tool_use", "id": "toolu_1", "name": "propose_vm_action",
             "input": {"action": "snapshot", "names": ["db-1"]}}]}))
        .unwrap();
        let msg = r.choices.unwrap().remove(0).message.unwrap();
        assert_eq!(msg.content.as_deref(), Some("Let me check. "));
        let call = msg.tool_calls.unwrap().remove(0);
        assert_eq!(call.id.as_deref(), Some("toolu_1"));
        let f = call.function.unwrap();
        assert_eq!(f.name.as_deref(), Some("propose_vm_action"));
        let args = parse_tool_args(&f.arguments.unwrap());
        assert_eq!(args.action.as_deref(), Some("snapshot"));
        assert_eq!(args.names, vec!["db-1"]);

        let plain =
            from_anthropic_response(&json!({"content": [{"type": "text", "text": "done"}]}))
                .unwrap();
        let m = plain.choices.unwrap().remove(0).message.unwrap();
        assert_eq!(m.content.as_deref(), Some("done"));
        assert!(m.tool_calls.is_none());
    }

    #[test]
    fn anthropic_url_and_detection() {
        assert_eq!(
            anthropic_messages_url("https://api.anthropic.com"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            anthropic_messages_url("https://gw.example/v1/"),
            "https://gw.example/v1/messages"
        );
        assert_eq!(
            anthropic_messages_url("https://gw.example/v1/messages"),
            "https://gw.example/v1/messages"
        );
        let cfg = |url: &str, key: &str| LlmConfig {
            base_url: url.into(),
            api_key: key.into(),
            model: "m".into(),
        };
        assert!(is_anthropic(&cfg("https://api.anthropic.com", "k")));
        assert!(is_anthropic(&cfg(
            "https://api.openai.com/v1",
            "sk-ant-abc"
        )));
        assert!(!is_anthropic(&cfg(
            "https://api.openai.com/v1",
            "sk-proj-abc"
        )));
        assert!(!is_anthropic(&cfg("http://llm.svc:8080/v1", "")));
    }

    /// The real HTTP path against a mock Anthropic server: URL, auth headers, the translated
    /// request body, and the tool_use reply coming back as an OpenAI-shaped tool call.
    #[cfg(feature = "web")]
    #[tokio::test]
    async fn anthropic_wire_roundtrip_against_a_mock_server() {
        use axum::{Json, Router, http::HeaderMap, routing::post};
        use serde_json::{Value, json};
        use std::sync::{Arc, Mutex};

        let seen: Arc<Mutex<Option<(String, String, Value)>>> = Arc::new(Mutex::new(None));
        let sink = seen.clone();
        let app = Router::new().route(
            "/v1/messages",
            post(move |headers: HeaderMap, Json(body): Json<Value>| {
                let sink = sink.clone();
                async move {
                    let h = |k: &str| {
                        headers
                            .get(k)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("")
                            .to_string()
                    };
                    *sink.lock().unwrap() = Some((h("x-api-key"), h("anthropic-version"), body));
                    Json(json!({"content": [
                        {"type": "tool_use", "id": "toolu_9", "name": "propose_vm_action",
                         "input": {"action": "restart", "namespace": "prod", "names": ["web-1"]}}
                    ]}))
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let cfg = LlmConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            api_key: "sk-ant-test-key".into(),
            model: "claude-sonnet-5-5".into(),
        };
        assert!(is_anthropic(&cfg));
        let body = json!({
            "model": cfg.model,
            "messages": [
                {"role": "system", "content": "sys"},
                {"role": "user", "content": "restart web-1"}],
            "tools": [{"type": "function", "function": {
                "name": "propose_vm_action", "description": "d", "parameters": {"type": "object"}}}]
        });
        let resp = post_chat_raw(&cfg, &body).await.expect("mock reply");

        let (key, version, sent) = seen.lock().unwrap().clone().expect("request seen");
        assert_eq!(key, "sk-ant-test-key");
        assert_eq!(version, "2023-06-01");
        assert_eq!(sent["model"], "claude-sonnet-5-5");
        assert_eq!(sent["system"], "sys");
        assert_eq!(sent["messages"][0]["content"][0]["text"], "restart web-1");
        assert_eq!(sent["tools"][0]["name"], "propose_vm_action");

        let call = resp
            .choices
            .unwrap()
            .remove(0)
            .message
            .unwrap()
            .tool_calls
            .unwrap()
            .remove(0);
        assert_eq!(call.id.as_deref(), Some("toolu_9"));
        let args = parse_tool_args(&call.function.unwrap().arguments.unwrap());
        assert_eq!(args.action.as_deref(), Some("restart"));
        assert_eq!(args.namespace.as_deref(), Some("prod"));
        assert_eq!(args.names, vec!["web-1"]);
    }

    #[test]
    fn is_openrouter_key_prefix() {
        assert!(is_openrouter_key("sk-or-v1-abc"));
        assert!(!is_openrouter_key("sk-abc"));
    }
}
