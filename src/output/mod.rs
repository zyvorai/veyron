// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

pub mod spinner;
pub mod table;

use crate::config::VMConfig;
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Yaml,
    Json,
    Table,
}

impl OutputFormat {
    pub fn parse_format(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "yaml" | "yml" => Some(Self::Yaml),
            "json" => Some(Self::Json),
            "table" => Some(Self::Table),
            _ => None,
        }
    }

    pub fn from_path(path: &str) -> Self {
        if path.ends_with(".json") {
            Self::Json
        } else {
            Self::Yaml
        }
    }
}

/// Serialize any value to YAML string.
///
/// ```
/// use veyron::to_yaml;
/// let yaml = to_yaml(&vec!["hello", "world"]).unwrap();
/// assert!(yaml.contains("hello"));
/// ```
pub fn to_yaml<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_yml::to_string(value)?)
}

/// Serialize any value to pretty-printed JSON string.
///
/// ```
/// use veyron::to_json;
/// let json = to_json(&vec!["hello", "world"]).unwrap();
/// assert!(json.contains("hello"));
/// ```
pub fn to_json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}

/// Formats a VMConfig to the specified format
pub fn format_output<T: Serialize>(value: &T, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Yaml | OutputFormat::Table => to_yaml(value),
        OutputFormat::Json => to_json(value),
    }
}

/// Pretty print VMConfig to stdout
pub fn print_vm_config(config: &VMConfig, format: OutputFormat) -> Result<()> {
    let output = format_output(config, format)?;
    println!("{}", output);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VMConfigBuilder;

    #[test]
    fn test_yaml_output() {
        let config = VMConfigBuilder::new("test")
            .namespace("default")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .build();

        let yaml = to_yaml(&config).unwrap();
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("namespace: default"));
    }

    #[test]
    fn test_json_output() {
        let config = VMConfigBuilder::new("test")
            .namespace("default")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .build();

        let json = to_json(&config).unwrap();
        assert!(json.contains("\"name\": \"test\""));
        assert!(json.contains("\"namespace\": \"default\""));
    }

    #[test]
    fn test_output_format_from_str() {
        assert!(matches!(
            OutputFormat::parse_format("yaml"),
            Some(OutputFormat::Yaml)
        ));
        assert!(matches!(
            OutputFormat::parse_format("json"),
            Some(OutputFormat::Json)
        ));
        assert!(matches!(
            OutputFormat::parse_format("table"),
            Some(OutputFormat::Table)
        ));
        assert!(OutputFormat::parse_format("invalid").is_none());
    }
}
