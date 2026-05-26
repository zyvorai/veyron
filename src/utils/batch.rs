// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use crate::config::VMConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Batch VM creation configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Default namespace for all VMs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// List of VMs to create
    pub vms: Vec<VMConfig>,
}

impl BatchConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;

        let file_path = std::path::Path::new(path);
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let config: BatchConfig = match ext {
            "json" => serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?,
            "yaml" | "yml" => {
                serde_yml::from_str(&content).map_err(|e| anyhow::anyhow!("Invalid YAML: {}", e))?
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported file format '{}'. Use .json, .yaml, or .yml",
                    ext
                ));
            }
        };

        Ok(config)
    }

    pub fn apply_namespace(&mut self, namespace: &str) {
        for vm in &mut self.vms {
            if vm.namespace.is_empty() {
                vm.namespace = namespace.to_string();
            }
        }

        if self.namespace.is_none() {
            self.namespace = Some(namespace.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config() {
        let yaml = r#"
namespace: production
vms:
  - name: web-1
    namespace: production
    cpu:
      cores: 2
      sockets: 1
      threads: 1
    memory:
      size: 4Gi
    disks:
      - name: rootdisk
        size: 20Gi
        boot_order: 1
        source:
          type: blank
    interfaces:
      - name: default
        network: default
        model: virtio
        network_type: pod
  - name: web-2
    namespace: production
    cpu:
      cores: 2
      sockets: 1
      threads: 1
    memory:
      size: 4Gi
    disks:
      - name: rootdisk
        size: 20Gi
        boot_order: 1
        source:
          type: blank
    interfaces:
      - name: default
        network: default
        model: virtio
        network_type: pod
"#;

        let config: BatchConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.namespace, Some("production".to_string()));
        assert_eq!(config.vms.len(), 2);
        assert_eq!(config.vms[0].name, "web-1");
        assert_eq!(config.vms[1].name, "web-2");
    }
}
