// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Catalog export: Rust templates/profiles → cluster VMTemplate/VMProfile CRD YAML.

use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::operator_crds::{VMProfile, VMProfileSpec, VeyronVMSpec, VMTemplate, VMTemplateSpec};
use crate::profiles::PROFILES;
use crate::templates::TEMPLATES;

const MANAGED_LABEL: &str = "veyron-catalog-export";

pub fn embedded_template_count() -> usize {
    TEMPLATES.list().len()
}

pub fn embedded_profile_count() -> usize {
    PROFILES.read().map(|p| p.list().len()).unwrap_or(0)
}

pub fn template_family(name: &str) -> String {
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

fn build_template_crd(name: &str) -> Result<VMTemplate> {
    let config = TEMPLATES
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("template missing: {name}"))?;
    let default_spec = VeyronVMSpec::from(&config);
    let family = template_family(name);
    let recommended = crate::profiles::ProfileManager::suggest_profile_for_template(name)
        .map(|p| vec![p])
        .unwrap_or_default();
    Ok(VMTemplate {
        metadata: kube::api::ObjectMeta {
            name: Some(name.to_string()),
            labels: Some(BTreeMap::from([(
                "veyron.io/managed-by".to_string(),
                MANAGED_LABEL.to_string(),
            )])),
            ..Default::default()
        },
        spec: VMTemplateSpec {
            description: Some(format!("Veyron built-in template: {name}")),
            tags: vec![family.clone()],
            family: Some(family),
            recommended_profiles: recommended,
            min_kubevirt_version: None,
            default: default_spec,
        },
        status: None,
    })
}

fn build_profile_crd(profile: &crate::profiles::Profile) -> VMProfile {
    VMProfile {
        metadata: kube::api::ObjectMeta {
            name: Some(profile.name.clone()),
            labels: Some(BTreeMap::from([(
                "veyron.io/managed-by".to_string(),
                MANAGED_LABEL.to_string(),
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
    }
}

/// Upsert a cluster-scoped VMTemplate (create or replace on conflict).
pub async fn upsert_template(api: &kube::Api<VMTemplate>, tpl: VMTemplate) -> Result<()> {
    use kube::api::PostParams;

    let name = tpl
        .metadata
        .name
        .clone()
        .context("VMTemplate metadata.name required")?;
    match api.create(&PostParams::default(), &tpl).await {
        Ok(_) => Ok(()),
        Err(kube::Error::Api(a)) if a.code == 409 => {
            api.replace(&name, &PostParams::default(), &tpl).await?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Upsert a cluster-scoped VMProfile (create or replace on conflict).
pub async fn upsert_profile(api: &kube::Api<VMProfile>, prof: VMProfile) -> Result<()> {
    use kube::api::PostParams;

    let name = prof
        .metadata
        .name
        .clone()
        .context("VMProfile metadata.name required")?;
    match api.create(&PostParams::default(), &prof).await {
        Ok(_) => Ok(()),
        Err(kube::Error::Api(a)) if a.code == 409 => {
            api.replace(&name, &PostParams::default(), &prof).await?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogSyncResult {
    pub templates_synced: usize,
    pub profiles_synced: usize,
    pub message: String,
}

/// Push all built-in templates and profiles to the cluster (in-memory → CRD upsert).
pub async fn sync_embedded_catalog(client: &kube::Client) -> Result<CatalogSyncResult> {
    use kube::Api;

    let tpl_api: Api<VMTemplate> = Api::all(client.clone());
    let prof_api: Api<VMProfile> = Api::all(client.clone());

    let mut templates_synced = 0usize;
    for name in TEMPLATES.list() {
        upsert_template(&tpl_api, build_template_crd(&name)?).await?;
        templates_synced += 1;
    }

    let profile_list = {
        let profiles = PROFILES
            .read()
            .map_err(|e| anyhow::anyhow!("lock profiles: {e}"))?;
        profiles.list()
    };
    let mut profiles_synced = 0usize;
    for profile in profile_list {
        upsert_profile(&prof_api, build_profile_crd(&profile)).await?;
        profiles_synced += 1;
    }

    Ok(CatalogSyncResult {
        templates_synced,
        profiles_synced,
        message: format!(
            "Synced {templates_synced} VMTemplate and {profiles_synced} VMProfile CRDs (create or replace)"
        ),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogStatus {
    pub crds_available: bool,
    pub embedded_templates: usize,
    pub embedded_profiles: usize,
    pub cluster_templates: usize,
    pub cluster_profiles: usize,
    pub in_sync: bool,
    pub missing_templates: Vec<String>,
    pub missing_profiles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Compare embedded catalog with cluster VMTemplate/VMProfile CRDs.
pub async fn catalog_status(client: &kube::Client) -> CatalogStatus {
    let embedded_templates = embedded_template_count();
    let embedded_profiles = embedded_profile_count();

    let cluster_templates = match list_cluster_templates_on(client).await {
        Ok(list) => list,
        Err(_) => {
            return CatalogStatus {
                crds_available: false,
                embedded_templates,
                embedded_profiles,
                cluster_templates: 0,
                cluster_profiles: 0,
                in_sync: false,
                missing_templates: vec![],
                missing_profiles: vec![],
                message: Some(
                    "veyron.io VMTemplate/VMProfile CRDs not installed — install operator CRDs first."
                        .to_string(),
                ),
            };
        }
    };

    let cluster_profiles = match list_cluster_profiles_on(client).await {
        Ok(list) => list,
        Err(e) => {
            return CatalogStatus {
                crds_available: false,
                embedded_templates,
                embedded_profiles,
                cluster_templates: cluster_templates.len(),
                cluster_profiles: 0,
                in_sync: false,
                missing_templates: vec![],
                missing_profiles: vec![],
                message: Some(format!("Could not list VMProfile CRDs: {e}")),
            };
        }
    };

    let cluster_tpl_names: BTreeSet<_> =
        cluster_templates.iter().map(|t| t.name.as_str()).collect();
    let cluster_prof_names: BTreeSet<_> =
        cluster_profiles.iter().map(|p| p.name.as_str()).collect();

    let missing_templates: Vec<String> = TEMPLATES
        .list()
        .into_iter()
        .filter(|n| !cluster_tpl_names.contains(n.as_str()))
        .collect();
    let missing_profiles: Vec<String> = PROFILES
        .read()
        .map(|p| {
            p.list()
                .into_iter()
                .map(|prof| prof.name)
                .filter(|name| !cluster_prof_names.contains(name.as_str()))
                .collect()
        })
        .unwrap_or_default();

    let in_sync = missing_templates.is_empty() && missing_profiles.is_empty();
    let message = if in_sync {
        Some("Cluster catalog matches all embedded templates and profiles.".to_string())
    } else {
        Some(format!(
            "Missing {} template(s) and {} profile(s) — run catalog sync.",
            missing_templates.len(),
            missing_profiles.len()
        ))
    };

    CatalogStatus {
        crds_available: true,
        embedded_templates,
        embedded_profiles,
        cluster_templates: cluster_templates.len(),
        cluster_profiles: cluster_profiles.len(),
        in_sync,
        missing_templates,
        missing_profiles,
        message,
    }
}

async fn list_cluster_templates_on(client: &kube::Client) -> Result<Vec<ClusterTemplateSummary>> {
    use kube::{Api, api::ListParams};

    let api: Api<VMTemplate> = Api::all(client.clone());
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

async fn list_cluster_profiles_on(client: &kube::Client) -> Result<Vec<ClusterProfileSummary>> {
    use kube::{Api, api::ListParams};

    let api: Api<VMProfile> = Api::all(client.clone());
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

/// Export all built-in templates and profiles as cluster-scoped CRD manifests.
pub fn export_catalog(output_dir: &Path) -> Result<(usize, usize)> {
    let templates_dir = output_dir.join("templates");
    let profiles_dir = output_dir.join("profiles");
    fs::create_dir_all(&templates_dir)?;
    fs::create_dir_all(&profiles_dir)?;

    let mut template_count = 0;
    for name in TEMPLATES.list() {
        let doc = build_template_crd(&name)?;
        write_yaml(&templates_dir.join(format!("{name}.yaml")), &doc)?;
        template_count += 1;
    }

    let profiles = PROFILES
        .read()
        .map_err(|e| anyhow::anyhow!("lock profiles: {e}"))?;
    let mut profile_count = 0;
    for profile in profiles.list() {
        let doc = build_profile_crd(&profile);
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
