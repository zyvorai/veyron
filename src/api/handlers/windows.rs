// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Windows golden-image setup wizard API.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::Query,
    routing::get,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct WindowsSetupQuery {
    #[serde(default = "default_template")]
    pub template: String,
    pub vm_name: Option<String>,
    pub namespace: Option<String>,
}

fn default_template() -> String {
    "windows-2022".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsSetupStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub doc_anchor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsSetupPlan {
    pub template: String,
    pub vm_name: String,
    pub namespace: String,
    pub cloud_init_delivery: String,
    pub steps: Vec<WindowsSetupStep>,
    pub sample_vmroguevm_yaml: String,
    pub docs: Vec<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new().route("/windows/setup-plan", get(get_setup_plan))
}

#[cfg(feature = "web")]
async fn get_setup_plan(Query(q): Query<WindowsSetupQuery>) -> Json<WindowsSetupPlan> {
    let vm_name = q.vm_name.unwrap_or_else(|| "win-vm".to_string());
    let namespace = q.namespace.unwrap_or_else(|| "default".to_string());

    let steps = vec![
        WindowsSetupStep {
            id: "packer".into(),
            title: "Build golden QCOW2 with Packer".into(),
            description: "Audit Mode, VirtIO drivers, Cloudbase-Init, Sysprep.".into(),
            doc_anchor: Some("packer".into()),
        },
        WindowsSetupStep {
            id: "cdi".into(),
            title: "Import image via CDI DataVolume".into(),
            description: "Upload or HTTP import your generalized Windows image into a PVC.".into(),
            doc_anchor: Some("cdi".into()),
        },
        WindowsSetupStep {
            id: "configdrive".into(),
            title: "Use cloudInitConfigDrive for Cloudbase-Init".into(),
            description: "Set cloud_init delivery to configdrive in VMConfig.".into(),
            doc_anchor: Some("cloudInitConfigDrive".into()),
        },
        WindowsSetupStep {
            id: "rdp".into(),
            title: "Expose RDP via API".into(),
            description: "PUT /api/v1/vms/{ns}/{name}/rdp-expose; POST guest-agent/enable-rdp.".into(),
            doc_anchor: Some("rdp-expose".into()),
        },
    ];

    let sample = format!(
        "apiVersion: vmrogue.io/v1alpha1\nkind: VMRogueVM\nmetadata:\n  name: {vm_name}\n  namespace: {namespace}\nspec:\n  template: {}\n  cpu:\n    cores: 4\n  memory:\n    size: 8Gi\n  cloudInit:\n    delivery: configdrive\n",
        q.template
    );

    Json(WindowsSetupPlan {
        template: q.template.clone(),
        vm_name,
        namespace,
        cloud_init_delivery: "configdrive".to_string(),
        steps,
        sample_vmroguevm_yaml: sample,
        docs: vec![
            "docs/WINDOWS_KUBEVIRT_PRODUCTION.md".into(),
            "docs/WINDOWS_PACKER_GITOPS_PIPELINE.md".into(),
            "examples/windows-kubevirt-gitops/".into(),
        ],
    })
}
