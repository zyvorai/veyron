// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Pluggable local/remote template registry (VEYRON_TEMPLATE_REGISTRY).

use crate::config::VMConfig;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TemplateRegistryEntry {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Path to VMConfig YAML/JSON relative to registry file or absolute.
    pub path: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TemplateRegistry {
    #[serde(default)]
    pub templates: Vec<TemplateRegistryEntry>,
}

impl TemplateRegistry {
    pub fn registry_path() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("VEYRON_TEMPLATE_REGISTRY") {
            if !p.trim().is_empty() {
                return Some(PathBuf::from(p));
            }
        }
        dirs::config_dir().map(|d| d.join("veyron").join("templates.toml"))
    }

    pub fn load() -> Result<Option<Self>> {
        let Some(path) = Self::registry_path() else {
            return Ok(None);
        };
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path).context("read template registry")?;
        let reg: Self = toml::from_str(&content).context("parse template registry")?;
        Ok(Some(reg))
    }

    pub fn resolve_config(&self, name: &str, base: &Path) -> Result<Option<VMConfig>> {
        let Some(entry) = self.templates.iter().find(|t| t.name == name) else {
            return Ok(None);
        };
        let path = Path::new(&entry.path);
        let full = if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.parent().unwrap_or(base).join(path)
        };
        let content = std::fs::read_to_string(&full)
            .with_context(|| format!("read template file {}", full.display()))?;
        let config: VMConfig = if full.extension().is_some_and(|e| e == "json") {
            serde_json::from_str(&content)?
        } else {
            serde_yml::from_str(&content)?
        };
        Ok(Some(config))
    }

    pub fn list_by_tags(&self) -> HashMap<String, Vec<String>> {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        for t in &self.templates {
            if t.tags.is_empty() {
                m.entry("custom".to_string())
                    .or_default()
                    .push(t.name.clone());
            } else {
                for tag in &t.tags {
                    m.entry(tag.clone()).or_default().push(t.name.clone());
                }
            }
        }
        for v in m.values_mut() {
            v.sort();
        }
        m
    }
}
