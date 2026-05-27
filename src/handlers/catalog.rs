// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::tui::colors::cli as color;

pub fn handle_catalog_export(output: Option<String>) -> Result<()> {
    let out = output
        .map(PathBuf::from)
        .unwrap_or_else(crate::catalog::default_output_dir);
    let (templates, profiles) = crate::catalog::export_catalog(&out)?;
    println!(
        "{}",
        color::success(&format!(
            "Exported {templates} VMTemplate and {profiles} VMProfile manifests to {}",
            out.display()
        ))
    );
    Ok(())
}

pub async fn handle_catalog_list(templates: bool, profiles: bool) -> Result<()> {
    let show_templates = templates || !profiles;
    let show_profiles = profiles || !templates;

    if show_templates {
        match crate::catalog::list_cluster_templates().await {
            Ok(list) if list.is_empty() => {
                println!(
                    "{}",
                    color::info("No VMTemplate CRDs in cluster (run: vmrogue catalog sync)")
                );
            }
            Ok(list) => {
                println!("{}", color::header("Cluster VMTemplate catalog:"));
                for t in list {
                    let family = t.family.as_deref().unwrap_or("other");
                    let desc = t.description.as_deref().unwrap_or("");
                    if desc.is_empty() {
                        println!(
                            "  {} {} {}",
                            color::value("•"),
                            color::label(&t.name),
                            color::info(&format!("({family})"))
                        );
                    } else {
                        println!(
                            "  {} {} {} — {}",
                            color::value("•"),
                            color::label(&t.name),
                            color::info(&format!("({family})")),
                            desc
                        );
                    }
                }
            }
            Err(e) => {
                return Err(e.context(
                    "list VMTemplate CRDs (install operator CRDs and run vmrogue catalog sync)",
                ));
            }
        }
    }

    if show_profiles {
        if show_templates {
            println!();
        }
        match crate::catalog::list_cluster_profiles().await {
            Ok(list) if list.is_empty() => {
                println!(
                    "{}",
                    color::info("No VMProfile CRDs in cluster (run: vmrogue catalog sync)")
                );
            }
            Ok(list) => {
                println!("{}", color::header("Cluster VMProfile catalog:"));
                for p in list {
                    println!(
                        "  {} {} — {} CPU, {}, disk {}",
                        color::value("•"),
                        color::label(&p.name),
                        p.cores,
                        p.memory,
                        p.disk_size
                    );
                }
            }
            Err(e) => {
                return Err(
                    e.context("list VMProfile CRDs (install operator CRDs and catalog sync)"),
                );
            }
        }
    }
    Ok(())
}

async fn upsert_template(
    api: &kube::Api<crate::operator_crds::VMTemplate>,
    tpl: crate::operator_crds::VMTemplate,
) -> Result<()> {
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

async fn upsert_profile(
    api: &kube::Api<crate::operator_crds::VMProfile>,
    prof: crate::operator_crds::VMProfile,
) -> Result<()> {
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

pub async fn handle_catalog_sync(_namespace: String) -> Result<()> {
    use kube::{Api, Client};

    let dir = crate::catalog::default_output_dir();
    let templates_dir = dir.join("templates");
    let profiles_dir = dir.join("profiles");
    if !templates_dir.is_dir() || !profiles_dir.is_dir() {
        anyhow::bail!(
            "catalog not exported — run: vmrogue catalog export (or ./scripts/generate-catalog-crds.sh)"
        );
    }

    let client = Client::try_default().await?;
    let tpl_api: Api<crate::operator_crds::VMTemplate> = Api::all(client.clone());
    let prof_api: Api<crate::operator_crds::VMProfile> = Api::all(client);

    let mut synced = 0usize;
    for entry in std::fs::read_dir(&templates_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "yaml") {
            let content = std::fs::read_to_string(&path)?;
            let tpl: crate::operator_crds::VMTemplate = serde_yml::from_str(&content)?;
            upsert_template(&tpl_api, tpl).await?;
            synced += 1;
        }
    }
    for entry in std::fs::read_dir(&profiles_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "yaml") {
            let content = std::fs::read_to_string(&path)?;
            let prof: crate::operator_crds::VMProfile = serde_yml::from_str(&content)?;
            upsert_profile(&prof_api, prof).await?;
            synced += 1;
        }
    }
    println!(
        "{}",
        color::success(&format!("Synced {synced} catalog CRDs to cluster (create or replace)"))
    );
    Ok(())
}
