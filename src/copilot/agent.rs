// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use super::dispatch::dispatch_intent;
use super::llm::{self, AiMode, ai_mode, max_tool_rounds};
use super::tools::invoke_tool;
use super::{
    CopilotAskRequest, CopilotChatMessage, CopilotChatRequest, CopilotChatResponse,
    CopilotResponse, CopilotToolTrace, finalize_copilot, general_help, intent,
};
use crate::kube::KubeClient;

pub async fn copilot_chat(
    client: &KubeClient,
    scope: &str,
    req: &CopilotChatRequest,
) -> CopilotChatResponse {
    let query_raw = req.query();
    let query = query_raw.trim();
    if query.is_empty() {
        let copilot = general_help();
        return CopilotChatResponse {
            message: CopilotChatMessage {
                role: "assistant".into(),
                content: copilot.summary.clone(),
                module: Some(copilot.module.clone()),
            },
            copilot,
            tool_trace: vec![],
            mode: format!("{:?}", ai_mode()).to_lowercase(),
        };
    }

    let history = &req.messages;
    let mut tool_trace = Vec::new();
    let mut ask = CopilotAskRequest {
        query: query.to_string(),
        namespace: req.namespace.clone(),
        vm_name: req.vm_name.clone(),
    };

    let copilot = if ai_mode() == AiMode::Agent {
        match run_agent_loop(client, scope, query, history, &mut tool_trace).await {
            Some(r) => r,
            None => run_keyword_path(client, scope, &mut ask, history).await,
        }
    } else {
        run_keyword_path(client, scope, &mut ask, history).await
    };

    let content = if copilot.summary.is_empty() {
        copilot.title.clone()
    } else {
        copilot.summary.clone()
    };

    CopilotChatResponse {
        message: CopilotChatMessage {
            role: "assistant".into(),
            content,
            module: Some(copilot.module.clone()),
        },
        copilot: finalize_copilot(copilot).await,
        tool_trace,
        mode: format!("{:?}", ai_mode()).to_lowercase(),
    }
}

/// Multi-turn agent loop: each tool result is fed back to the model as an
/// observation until it answers or `max_tool_rounds()` is reached.
/// Returns `None` when no tool ran (caller falls back to the keyword path).
async fn run_agent_loop(
    client: &KubeClient,
    scope: &str,
    query: &str,
    history: &[CopilotChatMessage],
    tool_trace: &mut Vec<CopilotToolTrace>,
) -> Option<CopilotResponse> {
    let mut messages = llm::agent_messages(query, history);
    let mut last: Option<CopilotResponse> = None;
    let mut final_text: Option<String> = None;

    for _ in 0..max_tool_rounds() {
        match llm::agent_step(&messages).await? {
            llm::AgentStep::Final(text) => {
                final_text = Some(text);
                break;
            }
            llm::AgentStep::Calls { assistant, calls } => {
                messages.push(assistant);
                for (id, call) in calls {
                    tool_trace.push(CopilotToolTrace {
                        tool: call.name.clone(),
                        args: serde_json::to_value(&call.args).unwrap_or_default(),
                    });
                    let resp = invoke_tool(&call.name, &call.args, client, scope, query).await;
                    let observation = match &resp {
                        Some(r) => tool_observation(r),
                        None => format!("unknown tool '{}'", call.name),
                    };
                    messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": id,
                        "content": observation
                    }));
                    if resp.is_some() {
                        last = resp;
                    }
                }
            }
        }
    }

    let mut resp = last?;
    if let Some(text) = final_text.filter(|t| !t.trim().is_empty()) {
        resp.summary = text;
    }
    Some(resp)
}

/// Compact, bounded text view of a tool result for the model.
fn tool_observation(r: &CopilotResponse) -> String {
    const MAX_CHARS: usize = 4000;
    let mut out = format!("{}\n{}", r.title, r.summary);
    if let Some(score) = r.health_score {
        out.push_str(&format!("\nhealth_score: {score}"));
    }
    for e in r.evidence.iter().take(20) {
        out.push_str(&format!("\n- {e}"));
    }
    for rec in r.recommendations.iter().take(10) {
        out.push_str(&format!("\nrecommendation: {rec}"));
    }
    if out.chars().count() > MAX_CHARS {
        out = out.chars().take(MAX_CHARS).collect::<String>() + "…";
    }
    out
}

async fn run_keyword_path(
    client: &KubeClient,
    scope: &str,
    ask: &mut CopilotAskRequest,
    history: &[CopilotChatMessage],
) -> CopilotResponse {
    if matches!(ai_mode(), AiMode::Routing | AiMode::Agent) {
        if let Some(c) = llm::classify_intent(&ask.query, history).await {
            if c.confidence.unwrap_or(0.5) >= 0.4 {
                if c.namespace.is_some() {
                    ask.namespace = c.namespace.clone();
                }
                if c.vm_name.is_some() {
                    ask.vm_name = c.vm_name.clone();
                }
                return dispatch_intent(client, scope, llm::classified_to_intent(&c), ask).await;
            }
        }
    }
    let intent = intent::detect_intent(&ask.query);
    dispatch_intent(client, scope, intent, ask).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_is_bounded_and_includes_evidence() {
        let mut r = CopilotResponse::new("doctor", "vm_doctor", "VM x");
        r.summary = "degraded".into();
        r.evidence = (0..50).map(|i| format!("event {i}")).collect();
        let o = tool_observation(&r);
        assert!(o.contains("degraded"));
        assert!(o.contains("event 19"));
        assert!(!o.contains("event 20"));
        r.summary = "x".repeat(10_000);
        assert!(tool_observation(&r).chars().count() <= 4001);
    }
}
