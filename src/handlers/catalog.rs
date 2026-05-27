// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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

pub async fn handle_catalog_sync(namespace: String) -> Result<()> {
    use kube::{Api, Client, api::PostParams};

    let dir = crate::catalog::default_output_dir();
    let client = Client::try_default().await?;

    let mut synced = 0usize;
    for entry in std::fs::read_dir(dir.join("templates"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "yaml") {
            let content = std::fs::read_to_string(&path)?;
            let tpl: crate::operator_crds::VMTemplate = serde_yml::from_str(&content)?;
            let api: Api<crate::operator_crds::VMTemplate> = Api::all(client.clone());
            let _ = api
                .create(&PostParams::default(), &tpl)
                .await
                .or_else(|e| {
                    if e.to_string().contains("AlreadyExists") {
                        Ok(tpl)
                    } else {
                        Err(e)
                    }
                })?;
            synced += 1;
        }
    }
    for entry in std::fs::read_dir(dir.join("profiles"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "yaml") {
            let content = std::fs::read_to_string(&path)?;
            let prof: crate::operator_crds::VMProfile = serde_yml::from_str(&content)?;
            let api: Api<crate::operator_crds::VMProfile> = Api::all(client.clone());
            let _ = api
                .create(&PostParams::default(), &prof)
                .await
                .or_else(|e| {
                    if e.to_string().contains("AlreadyExists") {
                        Ok(prof)
                    } else {
                        Err(e)
                    }
                })?;
            synced += 1;
        }
    }
    println!(
        "{}",
        color::success(&format!(
            "Synced {synced} catalog CRDs to cluster (namespace context: {namespace})"
        ))
    );
    Ok(())
}
