// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use anyhow::Result;
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
                    color::info("No VMTemplate CRDs in cluster (run: veyron catalog sync)")
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
                    "list VMTemplate CRDs (install operator CRDs and run veyron catalog sync)",
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
                    color::info("No VMProfile CRDs in cluster (run: veyron catalog sync)")
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
                    e.context("list VMProfile CRDs (install operator CRDs and catalog sync)")
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
    crate::catalog::upsert_template(api, tpl).await
}

async fn upsert_profile(
    api: &kube::Api<crate::operator_crds::VMProfile>,
    prof: crate::operator_crds::VMProfile,
) -> Result<()> {
    crate::catalog::upsert_profile(api, prof).await
}

pub async fn handle_catalog_sync(_namespace: String) -> Result<()> {
    use kube::Api;

    let dir = crate::catalog::default_output_dir();
    let templates_dir = dir.join("templates");
    let profiles_dir = dir.join("profiles");
    if !templates_dir.is_dir() || !profiles_dir.is_dir() {
        anyhow::bail!(
            "catalog not exported — run: veyron catalog export (or ./scripts/generate-catalog-crds.sh)"
        );
    }

    let client = crate::kube::get_client().await?;
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
        color::success(&format!(
            "Synced {synced} catalog CRDs to cluster (create or replace)"
        ))
    );
    Ok(())
}
