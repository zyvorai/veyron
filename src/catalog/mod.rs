// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Catalog export: Rust templates/profiles → cluster VMTemplate/VMProfile CRD YAML.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::operator_crds::{VMProfile, VMProfileSpec, VMRogueVMSpec, VMTemplate, VMTemplateSpec};
use crate::profiles::PROFILES;
use crate::templates::TEMPLATES;

fn template_family(name: &str) -> String {
    if name.starts_with("windows") {
        "windows".to_string()
    } else {
        "linux".to_string()
    }
}

fn write_yaml<T: serde::Serialize>(path: &Path, doc: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let yaml = serde_yml::to_string(doc).context("serialize catalog YAML")?;
    fs::write(path, yaml).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Export all built-in templates and profiles as cluster-scoped CRD manifests.
pub fn export_catalog(output_dir: &Path) -> Result<(usize, usize)> {
    let templates_dir = output_dir.join("templates");
    let profiles_dir = output_dir.join("profiles");
    fs::create_dir_all(&templates_dir)?;
    fs::create_dir_all(&profiles_dir)?;

    let mut template_count = 0;
    for name in TEMPLATES.list() {
        let config = TEMPLATES
            .get(&name)
            .ok_or_else(|| anyhow::anyhow!("template missing: {name}"))?;
        let default_spec = VMRogueVMSpec::from(&config);
        let family = template_family(&name);
        let recommended = crate::profiles::ProfileManager::suggest_profile_for_template(&name)
            .map(|p| vec![p])
            .unwrap_or_default();

        let doc = VMTemplate {
            metadata: kube::api::ObjectMeta {
                name: Some(name.clone()),
                labels: Some(BTreeMap::from([(
                    "vmrogue.io/managed-by".to_string(),
                    "vmrogue-catalog-export".to_string(),
                )])),
                ..Default::default()
            },
            spec: VMTemplateSpec {
                description: Some(format!("VMRogue built-in template: {name}")),
                tags: vec![family.clone()],
                family: Some(family),
                recommended_profiles: recommended,
                min_kubevirt_version: None,
                default: default_spec,
            },
            status: None,
        };

        write_yaml(&templates_dir.join(format!("{name}.yaml")), &doc)?;
        template_count += 1;
    }

    let profiles = PROFILES
        .read()
        .map_err(|e| anyhow::anyhow!("lock profiles: {e}"))?;
    let mut profile_count = 0;
    for profile in profiles.list() {
        let doc = VMProfile {
            metadata: kube::api::ObjectMeta {
                name: Some(profile.name.clone()),
                labels: Some(BTreeMap::from([(
                    "vmrogue.io/managed-by".to_string(),
                    "vmrogue-catalog-export".to_string(),
                )])),
                ..Default::default()
            },
            spec: VMProfileSpec {
                description: Some(profile.description.clone()),
                cores: profile.cpu_cores,
                sockets: profile.cpu_sockets,
                threads: profile.cpu_threads,
                memory: profile.memory.clone(),
                disk_size: profile.disk_size.clone(),
                use_cases: profile.use_cases.clone(),
                recommended_templates: profile.recommended_os.clone(),
            },
            status: None,
        };
        write_yaml(&profiles_dir.join(format!("{}.yaml", profile.name)), &doc)?;
        profile_count += 1;
    }

    Ok((template_count, profile_count))
}

/// Default output directory for catalog export.
pub fn default_output_dir() -> PathBuf {
    PathBuf::from("operator/config/catalog")
}

/// Summary row for cluster VMTemplate listing.
#[derive(Debug, Clone)]
pub struct ClusterTemplateSummary {
    pub name: String,
    pub family: Option<String>,
    pub description: Option<String>,
}

/// Summary row for cluster VMProfile listing.
#[derive(Debug, Clone)]
pub struct ClusterProfileSummary {
    pub name: String,
    pub cores: u32,
    pub memory: String,
    pub disk_size: String,
}

/// List VMTemplate CRDs from the connected cluster.
pub async fn list_cluster_templates() -> Result<Vec<ClusterTemplateSummary>> {
    use kube::{Api, Client, api::ListParams};

    let client = Client::try_default().await?;
    let api: Api<VMTemplate> = Api::all(client);
    let list = api.list(&ListParams::default().limit(500)).await?;
    let mut items: Vec<_> = list
        .items
        .into_iter()
        .map(|t| ClusterTemplateSummary {
            name: t.metadata.name.unwrap_or_default(),
            family: t.spec.family.clone(),
            description: t.spec.description.clone(),
        })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

/// List VMProfile CRDs from the connected cluster.
pub async fn list_cluster_profiles() -> Result<Vec<ClusterProfileSummary>> {
    use kube::{Api, Client, api::ListParams};

    let client = Client::try_default().await?;
    let api: Api<VMProfile> = Api::all(client);
    let list = api.list(&ListParams::default().limit(100)).await?;
    let mut items: Vec<_> = list
        .items
        .into_iter()
        .map(|p| ClusterProfileSummary {
            name: p.metadata.name.unwrap_or_default(),
            cores: p.spec.cores,
            memory: p.spec.memory.clone(),
            disk_size: p.spec.disk_size.clone(),
        })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn export_catalog_writes_files() {
        let dir = TempDir::new().unwrap();
        let (t, p) = export_catalog(dir.path()).unwrap();
        assert!(t >= 40);
        assert!(p >= 8);
        assert!(dir.path().join("templates/windows-2022.yaml").exists());
        assert!(dir.path().join("profiles/prod.yaml").exists());
    }
}
