// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// GitOps posture: VMRogue GitOps ConfigMaps, drift, Argo CD / Flux presence.
pub async fn gitops_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue GitOps Advisor",
        "gitops_advisor",
        "GitOps posture in workspace",
    );

    let k8s = client.client();
    let vms = client.list_vms_for_scope(scope).await;
    let namespaces: Vec<String> = if scope == "all" {
        let mut ns: Vec<String> = vms
            .iter()
            .filter_map(|v| v.metadata.namespace.clone())
            .collect();
        ns.sort();
        ns.dedup();
        if ns.is_empty() {
            vec!["default".into()]
        } else {
            ns
        }
    } else {
        vec![scope.to_string()]
    };

    let mut repos = 0usize;
    let mut drift_ns = Vec::new();

    for ns in &namespaces {
        let api: Api<ConfigMap> = Api::namespaced(k8s.clone(), ns);
        let lp = ListParams::default().labels("vmrogue.io/type=gitops");
        if let Ok(cms) = api.list(&lp).await {
            for cm in &cms.items {
                repos += 1;
                let data = cm.data.as_ref();
                let repo = data
                    .and_then(|d| d.get("repo_url"))
                    .map(String::as_str)
                    .unwrap_or("");
                let stored = data
                    .and_then(|d| d.get("vm_count"))
                    .and_then(|v| v.parse::<usize>().ok());
                let live = vms
                    .iter()
                    .filter(|v| v.metadata.namespace.as_deref() == Some(ns.as_str()))
                    .count();
                if let Some(stored) = stored {
                    if stored != live {
                        drift_ns.push(format!("{ns}: stored {stored} VMs, live {live}"));
                    }
                }
                if !repo.is_empty() {
                    r.evidence.push(format!("GitOps repo in {ns}: {repo}"));
                }
            }
        }
    }

    #[cfg(feature = "web")]
    {
        let (argo, _) = argo_apps(k8s.clone(), scope).await;
        if !argo.is_empty() {
            r.evidence
                .push(format!("Argo CD Applications: {}", argo.join(", ")));
        }
        let flux = flux_kustomizations(k8s.clone(), scope).await;
        if !flux.is_empty() {
            r.evidence
                .push(format!("Flux Kustomizations: {}", flux.join(", ")));
        }
    }

    let argo_url = std::env::var("VMROGUE_ARGOCD_URL")
        .ok()
        .filter(|u| !u.trim().is_empty());
    if argo_url.is_some() {
        r.evidence.push("VMROGUE_ARGOCD_URL is set — POST /gitops/sync can refresh apps".into());
    } else {
        r.recommendations.push(
            "Set VMROGUE_ARGOCD_URL + token to trigger Argo CD sync from the dashboard.".into(),
        );
    }

    for d in drift_ns.iter().take(6) {
        r.evidence.push(format!("Drift: {d}"));
    }

    r.summary = if repos == 0 {
        "No VMRogue GitOps ConfigMaps — export VMs or configure a repo in GitOps settings.".into()
    } else if drift_ns.is_empty() {
        format!("{repos} GitOps config(s); live VM counts match stored snapshots.")
    } else {
        format!("{repos} GitOps config(s); {} namespace(s) show VM count drift.", drift_ns.len())
    };

    r.actions = vec![
        CopilotAction {
            label: "Open GitOps".into(),
            action: "navigate".into(),
            page: Some("gitops".into()),
        },
        CopilotAction {
            label: "Export GitOps".into(),
            action: "navigate".into(),
            page: Some("gitops".into()),
        },
    ];
    r
}

#[cfg(feature = "web")]
async fn argo_apps(client: kube::Client, scope: &str) -> (Vec<String>, Option<String>) {
    use kube::api::{ApiResource, DynamicObject, ListParams};

    let ar = ApiResource {
        group: "argoproj.io".into(),
        version: "v1alpha1".into(),
        api_version: "argoproj.io/v1alpha1".into(),
        kind: "Application".into(),
        plural: "applications".into(),
    };
    let api: kube::Api<DynamicObject> = if scope == "all" {
        kube::Api::all_with(client, &ar)
    } else {
        kube::Api::namespaced_with(client, scope, &ar)
    };
    let Ok(list) = api.list(&ListParams::default().limit(20)).await else {
        return (vec![], None);
    };
    let names: Vec<String> = list
        .items
        .iter()
        .filter_map(|o| o.metadata.name.clone())
        .collect();
    (names, None)
}

#[cfg(feature = "web")]
async fn flux_kustomizations(client: kube::Client, scope: &str) -> Vec<String> {
    use kube::api::{ApiResource, DynamicObject, ListParams};

    let ar = ApiResource {
        group: "kustomize.toolkit.fluxcd.io".into(),
        version: "v1".into(),
        api_version: "kustomize.toolkit.fluxcd.io/v1".into(),
        kind: "Kustomization".into(),
        plural: "kustomizations".into(),
    };
    let api: kube::Api<DynamicObject> = if scope == "all" {
        kube::Api::all_with(client, &ar)
    } else {
        kube::Api::namespaced_with(client, scope, &ar)
    };
    api.list(&ListParams::default().limit(20))
        .await
        .ok()
        .map(|l| {
            l.items
                .iter()
                .filter_map(|o| o.metadata.name.clone())
                .collect()
        })
        .unwrap_or_default()
}
