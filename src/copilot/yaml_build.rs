// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse, ValidationItem};
use crate::config::VMConfig;
use crate::config::VMConfigBuilder;
use crate::kube::KubeClient;
use crate::kube::vm_config_to_kubevirt;
use crate::output::to_yaml;
use crate::templates::TEMPLATES;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct YamlBuildRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub cpus: Option<u32>,
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(default)]
    pub disk_size: Option<String>,
    #[serde(default)]
    pub storage_class: Option<String>,
    #[serde(default)]
    pub network_attachment: Option<String>,
}

pub fn parse_spec_from_query(query: &str) -> YamlBuildRequest {
    let lower = query.to_lowercase();
    let cpus = extract_u32_after(&lower, &["cpu", "vcpu", "vcpus", "cores"]);
    let memory = extract_memory(&lower);
    let disk = extract_disk_gb(&lower);

    let template = if lower.contains("windows 11") || lower.contains("windows-11") {
        Some("windows-11".into())
    } else if lower.contains("windows 10") {
        Some("windows-10".into())
    } else if lower.contains("windows") || lower.contains("server 2022") {
        Some("windows-2022".into())
    } else if lower.contains("ubuntu 24") {
        Some("ubuntu-24.04".into())
    } else if lower.contains("ubuntu") {
        Some("ubuntu-22.04".into())
    } else if lower.contains("postgres") {
        Some("ubuntu-22.04".into())
    } else {
        None
    };

    YamlBuildRequest {
        name: Some("copilot-vm".into()),
        namespace: None,
        template,
        cpus,
        memory,
        disk_size: disk,
        storage_class: None,
        network_attachment: None,
    }
}

pub async fn yaml_preview(
    client: &KubeClient,
    scope: &str,
    req: YamlBuildRequest,
) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue YAML Builder",
        "yaml_preview",
        "Generated VirtualMachine preview",
    );

    let name = req.name.as_deref().unwrap_or("copilot-vm");
    let namespace =
        req.namespace
            .as_deref()
            .unwrap_or(if scope == "all" { "default" } else { scope });

    let template_name = req.template.as_deref().unwrap_or("ubuntu-22.04");

    let mut config: VMConfig = if let Some(mut tpl) = TEMPLATES.get(template_name) {
        tpl.name = name.to_string();
        tpl.namespace = namespace.to_string();
        tpl
    } else {
        VMConfigBuilder::new(name)
            .namespace(namespace)
            .cpu(req.cpus.unwrap_or(2), 1, 1)
            .memory(req.memory.as_deref().unwrap_or("4Gi"))
            .add_blank_disk("rootdisk", req.disk_size.as_deref().unwrap_or("40Gi"), 1)
            .add_pod_network("default")
            .build()
    };

    if let Some(c) = req.cpus {
        config.cpu.cores = c;
    }
    if let Some(ref m) = req.memory {
        config.memory.size = m.clone();
    }
    if let Some(ref d) = req.disk_size {
        if let Some(disk) = config.disks.first_mut() {
            disk.size = d.clone();
        }
    }
    let validation = validate_against_cluster(client, &req, &config.memory.size, namespace).await;
    r.validation = Some(validation.clone());

    match vm_config_to_kubevirt(&config) {
        Ok(vm) => match to_yaml(&vm) {
            Ok(yaml) => {
                r.yaml_preview = Some(yaml);
                r.yaml_build = Some(req.clone());
                r.summary =
                    format!("Preview for {namespace}/{name} from template {template_name}.");
                let failed = validation.iter().any(|v| v.status == "error");
                if failed {
                    r.recommendations
                        .push("Fix validation errors before applying.".into());
                } else {
                    r.recommendations.push(
                        "Validation passed — review YAML then apply or save as blueprint.".into(),
                    );
                }
            }
            Err(e) => r.summary = format!("YAML serialize error: {e}"),
        },
        Err(e) => r.summary = format!("Could not build KubeVirt VM: {e}"),
    }

    r.actions = vec![
        CopilotAction {
            label: "Apply via Forge VM".into(),
            action: "forge_vm".into(),
            page: Some(template_name.into()),
        },
        CopilotAction {
            label: "Save as Blueprint".into(),
            action: "save_blueprint".into(),
            page: None,
        },
        CopilotAction {
            label: "Export GitOps".into(),
            action: "export_gitops".into(),
            page: None,
        },
    ];

    r
}

async fn validate_against_cluster(
    client: &KubeClient,
    req: &YamlBuildRequest,
    memory: &str,
    _namespace: &str,
) -> Vec<ValidationItem> {
    let mut items = Vec::new();

    if let Some(ref sc) = req.storage_class {
        match client.list_storage_classes().await {
            Ok(classes) => {
                let names: Vec<_> = classes
                    .iter()
                    .filter_map(|c| c.metadata.name.clone())
                    .collect();
                if names.iter().any(|n| n == sc) {
                    items.push(ValidationItem {
                        check: "StorageClass".into(),
                        status: "ok".into(),
                        detail: format!("{sc} exists"),
                    });
                } else {
                    items.push(ValidationItem {
                        check: "StorageClass".into(),
                        status: "error".into(),
                        detail: format!("{sc} not found — available: {}", names.join(", ")),
                    });
                }
            }
            Err(e) => items.push(ValidationItem {
                check: "StorageClass".into(),
                status: "warn".into(),
                detail: format!("Could not list StorageClasses: {e}"),
            }),
        }
    } else {
        items.push(ValidationItem {
            check: "StorageClass".into(),
            status: "ok".into(),
            detail: "Using cluster default (not specified in request)".into(),
        });
    }

    if let Some(ref nad) = req.network_attachment {
        items.push(ValidationItem {
            check: "NetworkAttachmentDefinition".into(),
            status: "warn".into(),
            detail: format!("Verify NAD {nad} exists (Multus) before apply"),
        });
    } else {
        items.push(ValidationItem {
            check: "Network".into(),
            status: "ok".into(),
            detail: "Default pod network (masquerade) — no NAD required".into(),
        });
    }

    let mem_gi = parse_gi(memory);
    if let Ok(nodes) = client.list_nodes().await {
        let mut max_mem = 0.0f64;
        for n in &nodes {
            if let Some(alloc) = n
                .status
                .as_ref()
                .and_then(|s| s.allocatable.as_ref())
                .and_then(|a| a.get("memory"))
            {
                max_mem = max_mem.max(parse_k8s_memory(&alloc.0));
            }
        }
        if mem_gi > 0.0 && max_mem > 0.0 && mem_gi > max_mem {
            items.push(ValidationItem {
                check: "Memory capacity".into(),
                status: "error".into(),
                detail: format!(
                    "Requested {memory} exceeds largest node allocatable (~{max_mem:.1} Gi)"
                ),
            });
        } else {
            items.push(ValidationItem {
                check: "Memory capacity".into(),
                status: "ok".into(),
                detail: format!("Requested {memory} fits within node allocatable range"),
            });
        }
    }

    let tpl = req.template.as_deref().unwrap_or("ubuntu-22.04");
    if TEMPLATES.get(tpl).is_some() {
        items.push(ValidationItem {
            check: "Template".into(),
            status: "ok".into(),
            detail: format!("Embedded template {tpl} available"),
        });
    } else {
        items.push(ValidationItem {
            check: "Template".into(),
            status: "warn".into(),
            detail: format!("{tpl} not in embedded library — using defaults"),
        });
    }

    items
}

fn extract_u32_after(lower: &str, keys: &[&str]) -> Option<u32> {
    for key in keys {
        if let Some(idx) = lower.find(key) {
            let rest = &lower[idx + key.len()..];
            for tok in rest.split_whitespace() {
                if let Ok(n) = tok.trim_matches(|c: char| !c.is_ascii_digit()).parse() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn extract_memory(lower: &str) -> Option<String> {
    if let Some(idx) = lower.find("32gb") {
        let _ = idx;
        return Some("32Gi".into());
    }
    if lower.contains("32 gb") || lower.contains("32gb") {
        return Some("32Gi".into());
    }
    if lower.contains("16 gb") || lower.contains("16gb") {
        return Some("16Gi".into());
    }
    if lower.contains("8 gb") || lower.contains("8gb") {
        return Some("8Gi".into());
    }
    for tok in lower.split_whitespace() {
        if tok.ends_with("gi") || tok.ends_with("gb") {
            return Some(tok.replace("gb", "Gi"));
        }
    }
    None
}

fn extract_disk_gb(lower: &str) -> Option<String> {
    if lower.contains("500gb") || lower.contains("500 gb") {
        return Some("500Gi".into());
    }
    if lower.contains("250gb") || lower.contains("250 gb") {
        return Some("250Gi".into());
    }
    if lower.contains("40gb") || lower.contains("40 gb") {
        return Some("40Gi".into());
    }
    None
}

fn parse_gi(mem: &str) -> f64 {
    let m = mem.trim().to_lowercase();
    if let Some(n) = m.strip_suffix("gi") {
        n.parse().unwrap_or(0.0)
    } else if let Some(n) = m.strip_suffix("gb") {
        n.parse().unwrap_or(0.0)
    } else {
        0.0
    }
}

fn parse_k8s_memory(s: &str) -> f64 {
    let m = s.trim().to_lowercase();
    if let Some(n) = m.strip_suffix("ki") {
        n.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
    } else if let Some(n) = m.strip_suffix("mi") {
        n.parse::<f64>().unwrap_or(0.0) / 1024.0
    } else if let Some(n) = m.strip_suffix("gi") {
        n.parse().unwrap_or(0.0)
    } else {
        0.0
    }
}
