// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Optional OpenAI-compatible LLM layer for Copilot.
//!
//! `VMROGUE_AI_URL`, `VMROGUE_AI_API_KEY`, `VMROGUE_AI_MODEL` (default `gpt-4o-mini`).
//! OpenRouter (Zeus): also accepts `OPENROUTER_API_KEY`, `ANTHROPIC_AUTH_TOKEN` (`sk-or-v1-…`),
//! `ANTHROPIC_BASE_URL` / `OPENROUTER_API_URL`, and `ANTHROPIC_MODEL` / `OPENROUTER_MODEL`.
//! `VMROGUE_AI_MODE`: `off` | `paraphrase` | `routing` | `agent`.

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
    _id: Option<String>,
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
    let mut base = url.trim().trim_end_matches('/').to_string();
    if base.contains("openrouter.ai/api") && !base.ends_with("/v1") {
        base.push_str("/v1");
    }
    base
}

fn resolve_api_key() -> Option<String> {
    env_nonempty("VMROGUE_AI_API_KEY")
        .or_else(|| env_nonempty("OPENROUTER_API_KEY"))
        .or_else(|| {
            env_nonempty("ANTHROPIC_AUTH_TOKEN").filter(|k| is_openrouter_key(k))
        })
}

fn resolve_base_url(key: &str) -> String {
    if let Some(url) = env_nonempty("VMROGUE_AI_URL")
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
    env_nonempty("VMROGUE_AI_MODEL")
        .or_else(|| env_nonempty("OPENROUTER_MODEL"))
        .or_else(|| env_nonempty("ANTHROPIC_MODEL"))
        .unwrap_or_else(|| {
            if is_openrouter_key(key) {
                "openrouter/free".into()
            } else {
                "gpt-4o-mini".into()
            }
        })
}

pub fn llm_config() -> Option<LlmConfig> {
    let key = resolve_api_key()?;
    let base_url = resolve_base_url(&key);
    let model = resolve_model(&key);
    Some(LlmConfig {
        base_url,
        api_key: key,
        model,
    })
}

pub fn ai_mode() -> AiMode {
    let explicit = std::env::var("VMROGUE_AI_MODE")
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
    std::env::var("VMROGUE_AI_MAX_TOOL_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2)
}

pub fn timeout_secs() -> u64 {
    std::env::var("VMROGUE_AI_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)
}

pub fn ai_rate_limit_per_min() -> u64 {
    std::env::var("VMROGUE_AI_RATE_LIMIT_PER_MIN")
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
        Err(e) => log::debug!("VMROGUE_AI paraphrase skipped: {e}"),
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

/// Run one LLM turn with tools; returns tool calls if the model requested them.
#[cfg(feature = "web")]
pub async fn agent_tool_calls(
    query: &str,
    history: &[CopilotChatMessage],
) -> Option<Vec<AgentToolCall>> {
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

    let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({
        "role": "system",
        "content": "You are Veyron Copilot for KubeVirt. Use tools to fetch cluster facts. \
                    Never invent evidence. Prefer specific tools over guessing."
    })];
    for m in history.iter().rev().take(8).rev() {
        messages.push(serde_json::json!({
            "role": m.role,
            "content": m.content
        }));
    }
    messages.push(serde_json::json!({"role": "user", "content": query}));

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
    let calls = msg.tool_calls.unwrap_or_default();
    if calls.is_empty() {
        return None;
    }
    Some(
        calls
            .into_iter()
            .filter_map(|tc| {
                let func = tc.function?;
                let name = func.name?;
                let args = parse_tool_args(&func.arguments.unwrap_or_default());
                Some(AgentToolCall { name, args })
            })
            .collect(),
    )
}

#[cfg(not(feature = "web"))]
pub async fn agent_tool_calls(
    _query: &str,
    _history: &[CopilotChatMessage],
) -> Option<Vec<AgentToolCall>> {
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

#[cfg(feature = "web")]
async fn post_chat_raw(cfg: &LlmConfig, body: &serde_json::Value) -> anyhow::Result<ChatResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs()))
        .build()?;
    let url = if cfg.base_url.contains("/chat/completions") {
        cfg.base_url.clone()
    } else {
        format!("{}/chat/completions", cfg.base_url)
    };
    let mut req = client.post(url).bearer_auth(&cfg.api_key);
    if let Some(referer) = env_nonempty("VMROGUE_AI_HTTP_REFERER")
        .or_else(|| env_nonempty("OPENROUTER_HTTP_REFERER"))
    {
        req = req.header("HTTP-Referer", referer);
    }
    if let Some(title) =
        env_nonempty("VMROGUE_AI_APP_TITLE").or_else(|| env_nonempty("OPENROUTER_APP_TITLE"))
    {
        req = req.header("X-Title", title);
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
        if std::env::var("VMROGUE_AI_RATE_LIMIT_PER_MIN").is_err() {
            assert_eq!(ai_rate_limit_per_min(), 20);
        }
    }

    #[test]
    fn max_tool_rounds_default() {
        if std::env::var("VMROGUE_AI_MAX_TOOL_ROUNDS").is_err() {
            assert_eq!(max_tool_rounds(), 2);
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
    fn is_openrouter_key_prefix() {
        assert!(is_openrouter_key("sk-or-v1-abc"));
        assert!(!is_openrouter_key("sk-abc"));
    }
}
