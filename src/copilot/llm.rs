// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Optional OpenAI-compatible paraphrase layer for Copilot responses.
//!
//! Set `VMROGUE_AI_URL` (e.g. `https://api.openai.com/v1`), `VMROGUE_AI_API_KEY`, and
//! optionally `VMROGUE_AI_MODEL` (default `gpt-4o-mini`). Facts in `evidence`, `validation`,
//! and `yaml_preview` are never sent for rewriting — only summary and recommendations.

use super::CopilotResponse;

struct LlmConfig {
    base_url: String,
    api_key: String,
    model: String,
}

fn llm_config() -> Option<LlmConfig> {
    let base = std::env::var("VMROGUE_AI_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let key = std::env::var("VMROGUE_AI_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let model = std::env::var("VMROGUE_AI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
    Some(LlmConfig {
        base_url: base.trim_end_matches('/').to_string(),
        api_key: key,
        model,
    })
}

/// Rewrite summary and recommendations in friendlier language when an LLM backend is configured.
#[cfg(feature = "web")]
pub async fn maybe_enhance_response(mut resp: CopilotResponse) -> CopilotResponse {
    let Some(cfg) = llm_config() else {
        return resp;
    };
    if resp.summary.is_empty() && resp.recommendations.is_empty() {
        return resp;
    }

    let prompt = format!(
        "You are VMRogue Copilot for KubeVirt. Rewrite ONLY the summary and recommendations \
         in plain language for an operator. Do not invent cluster facts. Keep the same meaning.\n\n\
         Module: {}\nTitle: {}\nSummary: {}\nRecommendations:\n{}\n\n\
         Reply as JSON: {{\"summary\":\"...\",\"recommendations\":[\"...\"]}}",
        resp.module,
        resp.title,
        resp.summary,
        resp.recommendations.join("\n")
    );

    match paraphrase_openai_compatible(&cfg, &prompt).await {
        Ok((summary, recs)) => {
            if !summary.trim().is_empty() {
                resp.summary = summary;
            }
            if !recs.is_empty() {
                resp.recommendations = recs;
            }
        }
        Err(e) => {
            log::debug!("VMROGUE_AI paraphrase skipped: {e}");
        }
    }
    resp
}

#[cfg(not(feature = "web"))]
pub async fn maybe_enhance_response(resp: CopilotResponse) -> CopilotResponse {
    resp
}

#[cfg(feature = "web")]
async fn paraphrase_openai_compatible(
    cfg: &LlmConfig,
    prompt: &str,
) -> anyhow::Result<(String, Vec<String>)> {
    use serde::Deserialize;

    #[derive(serde::Serialize)]
    struct ChatRequest<'a> {
        model: &'a str,
        messages: Vec<ChatMessage<'a>>,
        temperature: f32,
    }

    #[derive(serde::Serialize)]
    struct ChatMessage<'a> {
        role: &'a str,
        content: &'a str,
    }

    #[derive(Deserialize)]
    struct ChatResponse {
        choices: Option<Vec<ChatChoice>>,
    }

    #[derive(Deserialize)]
    struct ChatChoice {
        message: Option<ChatMessageOut>,
    }

    #[derive(Deserialize)]
    struct ChatMessageOut {
        content: Option<String>,
    }

    #[derive(Deserialize)]
    struct ParaphraseOut {
        summary: Option<String>,
        recommendations: Option<Vec<String>>,
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .build()?;

    let url = if cfg.base_url.contains("/chat/completions") {
        cfg.base_url.clone()
    } else {
        format!("{}/chat/completions", cfg.base_url)
    };

    let body = ChatRequest {
        model: &cfg.model,
        messages: vec![
            ChatMessage {
                role: "system",
                content: "Respond with valid JSON only.",
            },
            ChatMessage {
                role: "user",
                content: prompt,
            },
        ],
        temperature: 0.2,
    };

    let resp = client
        .post(url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("LLM HTTP {}", resp.status());
    }
    let chat: ChatResponse = resp.json().await?;
    let text = chat
        .choices
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .unwrap_or_default();

    let parsed: ParaphraseOut = serde_json::from_str(text.trim())
        .or_else(|_| {
            let stripped = text
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            serde_json::from_str(stripped)
        })
        .unwrap_or(ParaphraseOut {
            summary: Some(text),
            recommendations: None,
        });

    Ok((
        parsed.summary.unwrap_or_default(),
        parsed.recommendations.unwrap_or_default(),
    ))
}
