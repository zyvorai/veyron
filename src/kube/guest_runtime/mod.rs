// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

//! Guest runtime abstraction: KubeVirt guest-exec (QGA-compatible) and GuestKit JSON-RPC.

mod kubevirt;
mod types;

pub use types::*;

use crate::kube::guest_os::{detect_guest_os_family, GuestOsFamily};
use crate::kube::guestkit_client;
use crate::kube::windows_rdp::{is_windows_guest, vmi_guest_agent_connected};
use anyhow::{Context, Result};
use guestkit::evidence::EvidenceSnapshot;
use kube::Client;
use serde_json::{json, Value};
use std::env;

/// Preferred guest runtime backend from environment.
pub fn guest_runtime_mode_from_env() -> GuestRuntimeMode {
    match env::var("VMROGUE_GUEST_RUNTIME")
        .unwrap_or_else(|_| "auto".into())
        .to_lowercase()
        .as_str()
    {
        "guestkit" | "guest-kit" => GuestRuntimeMode::GuestKit,
        "qga" | "qemu" | "qemu-ga" => GuestRuntimeMode::QemuGa,
        _ => GuestRuntimeMode::Auto,
    }
}

/// Guest runtime connected per KubeVirt `AgentConnected` condition.
pub fn vmi_guest_runtime_connected(vmi: &Value) -> bool {
    vmi_guest_agent_connected(vmi)
}

/// Detect runtime kind from guest-info version string (via guest-exec when connected).
pub async fn detect_runtime_kind(
    client: Client,
    namespace: &str,
    vmi_name: &str,
    vm_json: Option<&Value>,
    vmi_json: Option<&Value>,
) -> GuestRuntimeKind {
    if is_windows_guest(vm_json, vmi_json) {
        return GuestRuntimeKind::QemuGa;
    }
    match guest_runtime_mode_from_env() {
        GuestRuntimeMode::GuestKit => return GuestRuntimeKind::GuestKit,
        GuestRuntimeMode::QemuGa => return GuestRuntimeKind::QemuGa,
        GuestRuntimeMode::Auto => {}
    }
    if let Some(vmi) = vmi_json {
        if !vmi_guest_runtime_connected(vmi) {
            return GuestRuntimeKind::Unknown;
        }
    } else {
        return GuestRuntimeKind::Unknown;
    }
    if let Ok(raw) = kubevirt::guest_info(client.clone(), namespace, vmi_name).await {
        let commands = raw
            .pointer("/return/supported_commands")
            .or_else(|| raw.get("supported_commands"))
            .and_then(|v| v.as_array());
        if let Some(list) = commands {
            if list.iter().any(|c| c.as_str() == Some("guestkit-get-evidence")) {
                return GuestRuntimeKind::GuestKit;
            }
        }
        if let Some(version) = raw
            .get("version")
            .or_else(|| raw.pointer("/return/version"))
            .and_then(|v| v.as_str())
        {
            if version.starts_with("guestkit-") {
                return GuestRuntimeKind::GuestKit;
            }
            if version.contains("qemu") {
                return GuestRuntimeKind::QemuGa;
            }
        }
    }
    if detect_guest_os_family(vm_json, vmi_json) == GuestOsFamily::Linux {
        GuestRuntimeKind::GuestKit
    } else {
        GuestRuntimeKind::Unknown
    }
}

pub struct GuestContext {
    pub client: Client,
    pub namespace: String,
    pub vm_name: String,
    pub vmi_name: String,
    pub vm_json: Value,
    pub vmi_json: Value,
    pub runtime_kind: GuestRuntimeKind,
}

impl GuestContext {
    pub async fn resolve(client: Client, namespace: &str, vm_name: &str) -> Result<Self> {
        let vmi_name = vm_name.to_string();
        let vm_json = json!({});
        let vmi_json = json!({});
        let runtime_kind =
            detect_runtime_kind(client.clone(), namespace, &vmi_name, None, None).await;
        Ok(Self {
            client,
            namespace: namespace.to_string(),
            vm_name: vm_name.to_string(),
            vmi_name,
            vm_json,
            vmi_json,
            runtime_kind,
        })
    }

    pub fn with_json(
        client: Client,
        namespace: &str,
        vm_name: &str,
        vmi_name: &str,
        vm_json: Value,
        vmi_json: Value,
    ) -> Self {
        Self {
            client,
            namespace: namespace.to_string(),
            vm_name: vm_name.to_string(),
            vmi_name: vmi_name.to_string(),
            vm_json,
            vmi_json,
            runtime_kind: GuestRuntimeKind::Unknown,
        }
    }

    pub async fn resolve_runtime_kind(&mut self) -> GuestRuntimeKind {
        self.runtime_kind = detect_runtime_kind(
            self.client.clone(),
            &self.namespace,
            &self.vmi_name,
            Some(&self.vm_json),
            Some(&self.vmi_json),
        )
        .await;
        self.runtime_kind
    }

    pub fn connected(&self) -> bool {
        vmi_guest_runtime_connected(&self.vmi_json)
    }
}

pub async fn guest_exec(
    ctx: &GuestContext,
    body: Value,
) -> Result<Value> {
    kubevirt::vmi_guest_exec(
        ctx.client.clone(),
        &ctx.namespace,
        &ctx.vmi_name,
        body,
    )
    .await
}

pub async fn guestkit_evidence(ctx: &GuestContext) -> Result<EvidenceSnapshot> {
    let value = guestkit_client::guestkit_rpc_for_vmi(
        ctx.client.clone(),
        &ctx.namespace,
        &ctx.vmi_name,
        "guestkit.getEvidence",
        json!({}),
    )
    .await?;
    serde_json::from_value(value).context("decode EvidenceSnapshot")
}

pub async fn guestkit_doctor(ctx: &GuestContext, target: &str) -> Result<Value> {
    guestkit_client::guestkit_rpc_for_vmi(
        ctx.client.clone(),
        &ctx.namespace,
        &ctx.vmi_name,
        "guestkit.doctor",
        json!({ "target": target }),
    )
    .await
}

pub async fn guestkit_fix_plan(ctx: &GuestContext, plan: Value) -> Result<Value> {
    guestkit_client::guestkit_rpc_for_vmi(
        ctx.client.clone(),
        &ctx.namespace,
        &ctx.vmi_name,
        "guestkit.runFixPlan",
        plan,
    )
    .await
}

pub async fn guestkit_capabilities(ctx: &GuestContext) -> Result<Value> {
    guestkit_client::guestkit_rpc_for_vmi(
        ctx.client.clone(),
        &ctx.namespace,
        &ctx.vmi_name,
        "guestkit.getCapabilities",
        json!({}),
    )
    .await
}

pub async fn guestkit_version(ctx: &GuestContext) -> Result<Value> {
    guestkit_client::guestkit_rpc_for_vmi(
        ctx.client.clone(),
        &ctx.namespace,
        &ctx.vmi_name,
        "guestkit.getVersion",
        json!({}),
    )
    .await
}

pub fn runtime_kind_label(kind: GuestRuntimeKind) -> &'static str {
    match kind {
        GuestRuntimeKind::GuestKit => "guestkit",
        GuestRuntimeKind::QemuGa => "qemu-ga",
        GuestRuntimeKind::Unknown => "unknown",
    }
}
