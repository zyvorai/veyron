// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::dispatch::dispatch_intent;
use super::llm::{self, ai_mode, max_tool_rounds, AiMode};
use super::tools::invoke_tool;
use super::{
    finalize_copilot, general_help, intent, CopilotAskRequest, CopilotChatMessage,
    CopilotChatRequest, CopilotChatResponse, CopilotResponse, CopilotToolTrace,
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
        if let Some(calls) = llm::agent_tool_calls(query, history).await {
            let mut last: Option<CopilotResponse> = None;
            for (i, call) in calls.iter().enumerate() {
                if i as u32 >= max_tool_rounds() {
                    break;
                }
                tool_trace.push(CopilotToolTrace {
                    tool: call.name.clone(),
                    args: serde_json::to_value(&call.args).unwrap_or_default(),
                });
                if let Some(resp) =
                    invoke_tool(&call.name, &call.args, client, scope, query).await
                {
                    last = Some(resp);
                }
            }
            if let Some(r) = last {
                r
            } else {
                run_keyword_path(client, scope, &mut ask, history).await
            }
        } else {
            run_keyword_path(client, scope, &mut ask, history).await
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
