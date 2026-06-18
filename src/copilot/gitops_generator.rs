// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct GitOpsExportRequest {
    pub yaml: String,
    #[serde(default)]
    pub app_name: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitOpsExportResponse {
    pub app_name: String,
    pub namespace: String,
    pub files: BTreeMap<String, String>,
    pub runbook: String,
}

pub fn export_gitops(req: GitOpsExportRequest) -> GitOpsExportResponse {
    let app_name = sanitize_slug(
        req.app_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("veyron-app"),
    );
    let namespace = sanitize_slug(
        req.namespace
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("default"),
    );
    let vm_yaml = req.yaml.trim().to_string();

    let kustomization = format!(
        "apiVersion: kustomize.config.k8s.io/v1beta1\n\
kind: Kustomization\n\
namespace: {namespace}\n\
resources:\n\
  - namespace.yaml\n\
  - virtualmachine.yaml\n\
commonLabels:\n\
  app.kubernetes.io/name: {app_name}\n\
  app.kubernetes.io/managed-by: veyron-copilot\n"
    );

    let namespace_yaml = format!(
        "apiVersion: v1\n\
kind: Namespace\n\
metadata:\n\
  name: {namespace}\n\
  labels:\n\
    app.kubernetes.io/name: {app_name}\n"
    );

    let runbook = format!(
        "# {app_name} — Veyron GitOps runbook\n\n\
## Layout\n\n\
- `base/namespace.yaml` — target namespace\n\
- `base/virtualmachine.yaml` — KubeVirt VirtualMachine from Copilot YAML Builder\n\
- `base/kustomization.yaml` — Kustomize entrypoint\n\
- `overlays/prod/kustomization.yaml` — example overlay (same base; patch as needed)\n\n\
## Deploy\n\n\
```bash\n\
kubectl apply -k base/\n\
# or with Kustomize CLI:\n\
kubectl apply -k overlays/prod/\n\
```\n\n\
## Sync with Argo CD / Flux\n\n\
1. Commit this tree to your Git repo under `apps/{app_name}/`.\n\
2. Point Argo CD Application or Flux Kustomization at `overlays/prod`.\n\
3. Use Veyron dashboard **Integrations → GitOps** or `POST /api/v1/gitops/sync` when controllers are wired.\n\n\
## Safety\n\n\
- Take a KubeVirt snapshot before changing running VMs.\n\
- Review `virtualmachine.yaml` for storage class, networks, and cloud-init secrets.\n"
    );

    let overlay_kustomization = "apiVersion: kustomize.config.k8s.io/v1beta1\n\
kind: Kustomization\n\
resources:\n\
  - ../../base\n\
# patches:\n\
#   - path: vm-patch.yaml\n"
        .to_string();

    let mut files = BTreeMap::new();
    files.insert("base/kustomization.yaml".into(), kustomization);
    files.insert("base/namespace.yaml".into(), namespace_yaml);
    files.insert("base/virtualmachine.yaml".into(), vm_yaml);
    files.insert(
        "overlays/prod/kustomization.yaml".into(),
        overlay_kustomization,
    );
    files.insert("README.md".into(), runbook.clone());

    GitOpsExportResponse {
        app_name,
        namespace,
        files,
        runbook,
    }
}

fn sanitize_slug(s: &str) -> String {
    let lower = s.trim().to_lowercase();
    let mut out = String::new();
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            out.push(ch);
        } else if (ch.is_whitespace() || ch == '_') && !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "veyron-app".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_includes_kustomize_tree() {
        let resp = export_gitops(GitOpsExportRequest {
            yaml: "apiVersion: kubevirt.io/v1\nkind: VirtualMachine\n".into(),
            app_name: Some("My App".into()),
            namespace: Some("prod-ns".into()),
        });
        assert_eq!(resp.app_name, "my-app");
        assert_eq!(resp.namespace, "prod-ns");
        assert!(resp.files.contains_key("base/virtualmachine.yaml"));
        assert!(
            resp.files
                .get("base/kustomization.yaml")
                .unwrap()
                .contains("namespace: prod-ns")
        );
    }
}
