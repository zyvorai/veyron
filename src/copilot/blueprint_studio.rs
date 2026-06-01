// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::yaml_build::YamlBuildRequest;
use crate::kube::KubeClient;
use crate::operator_crds::{CRDBlueprintVMSpec, VMRogueBlueprint, VMRogueBlueprintSpec};
use kube::api::{Api, PostParams};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct BlueprintSaveRequest {
    #[serde(flatten)]
    pub build: YamlBuildRequest,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub blueprint_name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlueprintSaveResponse {
    pub name: String,
    pub namespace: String,
    pub vm_count: usize,
    pub message: String,
}

pub fn spec_from_yaml_build(
    req: &YamlBuildRequest,
    description: Option<String>,
    tags: Vec<String>,
) -> VMRogueBlueprintSpec {
    let name = req.name.clone().unwrap_or_else(|| "copilot-vm".to_string());
    let template = req
        .template
        .clone()
        .unwrap_or_else(|| "ubuntu-22.04".to_string());
    VMRogueBlueprintSpec {
        description,
        vms: vec![CRDBlueprintVMSpec {
            name,
            template,
            profile: None,
            cpu: req.cpus,
            memory: req.memory.clone(),
            disk_size: req.disk_size.clone(),
            depends_on: Vec::new(),
            labels: Default::default(),
        }],
        tags,
    }
}

pub async fn save_blueprint(
    client: &KubeClient,
    namespace: &str,
    req: BlueprintSaveRequest,
) -> Result<BlueprintSaveResponse, String> {
    let spec = spec_from_yaml_build(&req.build, req.description.clone(), req.tags);
    let vm_count = spec.vms.len();
    let name = req
        .blueprint_name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("bp-{}", random_suffix()));
    let bp = VMRogueBlueprint::new(&name, spec);
    let api: Api<VMRogueBlueprint> = Api::namespaced(client.client().clone(), namespace);
    api.create(&PostParams::default(), &bp)
        .await
        .map_err(|e| format!("Could not create VMRogueBlueprint: {e}"))?;
    Ok(BlueprintSaveResponse {
        name: name.clone(),
        namespace: namespace.to_string(),
        vm_count,
        message: format!("Saved VMRogueBlueprint {namespace}/{name} ({vm_count} VM slot(s))"),
    })
}

fn random_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}").chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_from_build_maps_template_and_sizing() {
        let req = YamlBuildRequest {
            name: Some("db-1".into()),
            namespace: Some("apps".into()),
            template: Some("ubuntu-22.04".into()),
            cpus: Some(4),
            memory: Some("16Gi".into()),
            disk_size: Some("100Gi".into()),
            storage_class: None,
            network_attachment: None,
        };
        let spec = spec_from_yaml_build(&req, Some("Postgres tier".into()), vec!["copilot".into()]);
        assert_eq!(spec.vms.len(), 1);
        assert_eq!(spec.vms[0].name, "db-1");
        assert_eq!(spec.vms[0].template, "ubuntu-22.04");
        assert_eq!(spec.vms[0].cpu, Some(4));
        assert_eq!(spec.vms[0].memory.as_deref(), Some("16Gi"));
    }
}
