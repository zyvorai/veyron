// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use crate::copilot::{
    CopilotAskRequest, CopilotResponse, YamlBuildRequest, alert_advisor, backup_advisor,
    catalog_advisor, cilium_advisor, compliance_advisor, copilot_ask, cost_advisor,
    cost_advisor_vm, drift_advisor, finalize_copilot, forecast_advisor, gitops_advisor,
    guest_filesystem_report, guest_inspector, integrations_advisor, migration_advisor,
    network_lens, node_advisor, observability_advisor, performance_advisor, quota_advisor,
    recommend_template, scheduling_explainer, scheduling_fleet_advisor, security_sentinel,
    security_sentinel_fleet, slo_advisor, storage_doctor, velero_dr_advisor, vm_doctor,
    yaml_preview,
};
use crate::kube::KubeClient;
use anyhow::Result;

pub async fn handle_ai_ask(query: &str, namespace: &str, vm_name: Option<&str>) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() {
        "default"
    } else {
        namespace
    };
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
    let scope = if namespace.is_empty() {
        "default"
    } else {
        namespace
    };
    let resp = finalize_copilot(backup_advisor(&client, scope).await).await;
    print_copilot(&resp, "text");
    Ok(())
}

pub async fn handle_ai_cost(namespace: &str, name: Option<&str>, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() {
        "default"
    } else {
        namespace
    };
    let resp = if let Some(vm) = name {
        finalize_copilot(cost_advisor_vm(&client, scope, vm).await).await
    } else {
        finalize_copilot(cost_advisor(&client, scope).await).await
    };
    print_copilot(&resp, output);
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

pub async fn handle_ai_filesystem(name: &str, namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let resp = finalize_copilot(guest_filesystem_report(&client, namespace, name).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_storage(namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() {
        "default"
    } else {
        namespace
    };
    let resp = finalize_copilot(storage_doctor(&client, scope).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_security(namespace: &str, name: Option<&str>, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() {
        "default"
    } else {
        namespace
    };
    let resp = if let Some(vm) = name {
        finalize_copilot(security_sentinel(&client, scope, vm).await).await
    } else {
        finalize_copilot(security_sentinel_fleet(&client, scope).await).await
    };
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_performance(namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() {
        "default"
    } else {
        namespace
    };
    let resp = finalize_copilot(performance_advisor(&client, scope).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_gitops(namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() {
        "default"
    } else {
        namespace
    };
    let resp = finalize_copilot(gitops_advisor(&client, scope).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_forecast(namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() {
        "default"
    } else {
        namespace
    };
    let resp = finalize_copilot(forecast_advisor(&client, scope).await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_integrations(output: &str) -> Result<()> {
    let resp = finalize_copilot(integrations_advisor().await).await;
    print_copilot(&resp, output);
    Ok(())
}

pub async fn handle_ai_pending(namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let scope = if namespace.is_empty() {
        "default"
    } else {
        namespace
    };
    let resp = finalize_copilot(scheduling_fleet_advisor(&client, scope).await).await;
    print_copilot(&resp, output);
    Ok(())
}

macro_rules! ai_scope_handler {
    ($name:ident, $advisor:ident) => {
        pub async fn $name(namespace: &str, output: &str) -> Result<()> {
            let client = KubeClient::new().await?;
            let scope = if namespace.is_empty() {
                "default"
            } else {
                namespace
            };
            let resp = finalize_copilot($advisor(&client, scope).await).await;
            print_copilot(&resp, output);
            Ok(())
        }
    };
}

ai_scope_handler!(handle_ai_compliance, compliance_advisor);
ai_scope_handler!(handle_ai_observability, observability_advisor);
ai_scope_handler!(handle_ai_cilium, cilium_advisor);
ai_scope_handler!(handle_ai_nodes, node_advisor);
ai_scope_handler!(handle_ai_drift, drift_advisor);
ai_scope_handler!(handle_ai_alerts, alert_advisor);
ai_scope_handler!(handle_ai_slo, slo_advisor);
ai_scope_handler!(handle_ai_migrations, migration_advisor);

pub async fn handle_ai_catalog(_namespace: &str, output: &str) -> Result<()> {
    let client = KubeClient::new().await?;
    let resp = finalize_copilot(catalog_advisor(&client).await).await;
    print_copilot(&resp, output);
    Ok(())
}

ai_scope_handler!(handle_ai_quotas, quota_advisor);
ai_scope_handler!(handle_ai_velero_dr, velero_dr_advisor);

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
