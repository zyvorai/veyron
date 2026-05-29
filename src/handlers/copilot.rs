// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use crate::copilot::{
    CopilotAskRequest, CopilotResponse, YamlBuildRequest, backup_advisor, copilot_ask,
    finalize_copilot, guest_inspector, network_lens, recommend_template, scheduling_explainer,
    security_sentinel, security_sentinel_fleet, storage_doctor, vm_doctor, yaml_preview,
};
use crate::kube::KubeClient;
use anyhow::Result;

pub async fn handle_ai_ask(query: &str, namespace: &str, vm_name: Option<&str>) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() { "default" } else { namespace };
    let resp = finalize_copilot(
        copilot_ask(
            &client,
            scope,
            &CopilotAskRequest {
                query: query.to_string(),
                namespace: Some(namespace.to_string()),
                vm_name: vm_name.map(str::to_string),
            },
        )
        .await,
    )
    .await;
    print_copilot(&resp, "text");
    Ok(())
}

pub async fn handle_ai_doctor(name: &str, namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let resp = finalize_copilot(vm_doctor(&client, namespace, name).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_scheduling(name: &str, namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let resp = finalize_copilot(scheduling_explainer(&client, namespace, name).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_explain(message: &str, namespace: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let resp = finalize_copilot(
        copilot_ask(
            &client,
            namespace,
            &CopilotAskRequest {
                query: message.to_string(),
                namespace: None,
                vm_name: None,
            },
        )
        .await,
    )
    .await;
    print_copilot(&resp, "text");
    Ok(())
}

pub async fn handle_ai_yaml(
    namespace: &str,
    name: Option<&str>,
    template: Option<&str>,
    cpus: Option<u32>,
    memory: Option<&str>,
    disk: Option<&str>,
    output: &str,
) -> Result<()> {
    let client = KubeClient::new().await?;
    let req = YamlBuildRequest {
        name: name.map(str::to_string),
        namespace: Some(namespace.to_string()),
        template: template.map(str::to_string),
        cpus,
        memory: memory.map(str::to_string),
        disk_size: disk.map(str::to_string),
        storage_class: None,
        network_attachment: None,
    };
    let resp = finalize_copilot(yaml_preview(&client, namespace, req).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_recommend(description: &str) -> Result<()> {
    let resp = recommend_template(description);
    print_copilot(&resp, "text");
    Ok(())
}

pub async fn handle_ai_backup(namespace: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() { "default" } else { namespace };
    let resp = finalize_copilot(backup_advisor(&client, scope).await).await;
    print_copilot(&resp, "text");
    Ok(())
}

pub async fn handle_ai_network(name: &str, namespace: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let resp = finalize_copilot(network_lens(&client, namespace, name).await).await;
    print_copilot(&resp, "text");
    Ok(())
}

pub async fn handle_ai_guest(name: &str, namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let resp = finalize_copilot(guest_inspector(&client, namespace, name).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_storage(namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() { "default" } else { namespace };
    let resp = finalize_copilot(storage_doctor(&client, scope).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_security(
    namespace: &str,
    name: Option<&str>,
    output: &str,
) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() { "default" } else { namespace };
    let resp = if let Some(vm) = name {
        finalize_copilot(security_sentinel(&client, scope, vm).await).await
    } else {
        finalize_copilot(security_sentinel_fleet(&client, scope).await).await
    };
    print_copilot(&resp, output);
    Ok(())
}

fn print_copilot(resp: &CopilotResponse, output: &str) {
    if output == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(resp).unwrap_or_else(|_| "{}".into())
        );
        return;
    }
    println!("[{}] {}", resp.module, resp.title);
    if !resp.summary.is_empty() {
        println!("\n{}", resp.summary);
    }
    if let Some(score) = resp.health_score {
        println!("\nHealth score: {score}/100");
    }
    if !resp.evidence.is_empty() {
        println!("\nEvidence:");
        for e in &resp.evidence {
            println!("  - {e}");
        }
    }
    if let Some(ref validation) = resp.validation {
        println!("\nValidation:");
        for v in validation {
            println!("  [{}] {} — {}", v.status, v.check, v.detail);
        }
    }
    if let Some(ref yaml) = resp.yaml_preview {
        println!("\n--- YAML preview ---\n{yaml}");
    }
    if !resp.recommendations.is_empty() {
        println!("\nRecommended fixes:");
        for (i, r) in resp.recommendations.iter().enumerate() {
            println!("  {}. {r}", i + 1);
        }
    }
    if !resp.actions.is_empty() {
        println!("\nActions:");
        for a in &resp.actions {
            println!("  - {}", a.label);
        }
    }
}
