// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Guest OS family detection (Windows vs Linux) for KubeVirt VMs and VMIs.

use serde_json::Value;

pub use super::windows_rdp::{is_windows_vm, is_windows_vmi, vm_spec_has_rdp_port};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestOsFamily {
    Windows,
    Linux,
    Unknown,
}

impl GuestOsFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Unknown => "unknown",
        }
    }
}

pub fn detect_guest_os_family(vm: Option<&Value>, vmi: Option<&Value>) -> GuestOsFamily {
    if vm.is_some_and(is_windows_vm) || vmi.is_some_and(is_windows_vmi) {
        return GuestOsFamily::Windows;
    }
    if vm.is_some_and(is_linux_vm) || vmi.is_some_and(is_linux_vmi) {
        return GuestOsFamily::Linux;
    }
    GuestOsFamily::Unknown
}

fn is_linux_vm(vm: &Value) -> bool {
    if is_windows_vm(vm) {
        return false;
    }

    if guest_os_label(vm, "linux") {
        return true;
    }

    if let Some(distro) = hyper2kvm_os_distro(vm) {
        if looks_like_linux_distro(&distro) && !looks_like_windows_distro(&distro) {
            return true;
        }
    }

    if let Some(variant) = kubevirt_os_template_variant(vm) {
        if looks_like_linux_variant(&variant) && !looks_like_windows_variant(&variant) {
            return true;
        }
    }

    if let Some(os_type) = vm
        .pointer("/spec/template/spec/domain/os/type")
        .and_then(|t| t.as_str())
    {
        if os_type.eq_ignore_ascii_case("linux") {
            return true;
        }
    }

    if vm_has_linux_cloud_init(vm) {
        return true;
    }

    false
}

fn is_linux_vmi(vmi: &Value) -> bool {
    if is_windows_vmi(vmi) {
        return false;
    }

    let info = vmi
        .pointer("/status/guestOSInfo")
        .or_else(|| vmi.get("guestOSInfo"));
    let Some(info) = info else {
        return false;
    };

    for key in ["name", "prettyName"] {
        if let Some(s) = info.get(key).and_then(|v| v.as_str()) {
            if looks_like_linux_text(s) && !looks_like_windows_text(s) {
                return true;
            }
        }
    }
    false
}

fn guest_os_label(vm: &Value, family: &str) -> bool {
    for key in [
        "v9s.io/guest-os",
        "veyron.io/guest-os",
        "hyper2kvm.io/guest-os",
    ] {
        if metadata_str(vm, "labels", key).is_some_and(|v| v.eq_ignore_ascii_case(family)) {
            return true;
        }
        if metadata_str(vm, "annotations", key).is_some_and(|v| v.eq_ignore_ascii_case(family)) {
            return true;
        }
    }
    false
}

fn metadata_str(vm: &Value, section: &str, key: &str) -> Option<String> {
    vm.get("metadata")
        .and_then(|m| m.get(section))
        .and_then(|l| l.get(key))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn hyper2kvm_os_distro(vm: &Value) -> Option<String> {
    metadata_str(vm, "labels", "hyper2kvm.io/os-distro")
}

fn kubevirt_os_template_variant(vm: &Value) -> Option<String> {
    let labels = vm.get("metadata")?.get("labels")?.as_object()?;
    for (k, v) in labels {
        let prefix = "os.template.kubevirt.io/";
        if !k.starts_with(prefix) {
            continue;
        }
        let from_key = k.strip_prefix(prefix).unwrap_or("").trim();
        if !from_key.is_empty() && from_key != "variant" {
            return Some(from_key.to_string());
        }
        if let Some(s) = v.as_str().filter(|s| !s.is_empty() && *s != "true") {
            return Some(s.to_string());
        }
    }
    metadata_str(vm, "labels", "hyper2kvm.io/os-variant")
        .or_else(|| metadata_str(vm, "annotations", "hyper2kvm.io/os-variant"))
}

fn vm_has_linux_cloud_init(vm: &Value) -> bool {
    let Some(vols) = vm
        .pointer("/spec/template/spec/volumes")
        .and_then(|v| v.as_array())
    else {
        return false;
    };
    vols.iter().any(|vol| {
        if let Some(ci) = vol.get("cloudInitNoCloud") {
            return userdata_looks_like_linux_cloud_config(ci);
        }
        if let Some(ci) = vol.get("cloudInitConfigDrive") {
            let ud = ci.get("userData").and_then(|u| u.as_str()).unwrap_or("");
            if userdata_looks_like_windows(ud) {
                return false;
            }
            return userdata_looks_like_linux_cloud_config(ci);
        }
        false
    })
}

fn userdata_looks_like_windows(ud: &str) -> bool {
    let l = ud.to_lowercase();
    l.contains("ps1_sysnative") || l.contains("fdenytsconnections") || l.contains("terminal server")
}

fn userdata_looks_like_linux_cloud_config(ci: &Value) -> bool {
    let ud = ci.get("userData").and_then(|u| u.as_str()).unwrap_or("");
    if userdata_looks_like_windows(ud) {
        return false;
    }
    let trimmed = ud.trim_start();
    trimmed.starts_with("#cloud-config") || trimmed.starts_with("#!/bin/bash")
}

fn looks_like_linux_text(s: &str) -> bool {
    let lower = s.to_lowercase();
    [
        "linux", "rhel", "red hat", "ubuntu", "debian", "centos", "rocky", "alma", "fedora",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

fn looks_like_windows_text(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.contains("windows") || lower.contains("microsoft")
}

fn looks_like_linux_distro(distro: &str) -> bool {
    let l = distro.to_lowercase();
    l == "ubuntu"
        || l == "debian"
        || l == "centos"
        || l == "rhel"
        || l == "fedora"
        || l == "rocky"
        || l.starts_with("ubuntu")
}

fn looks_like_windows_distro(distro: &str) -> bool {
    let l = distro.to_lowercase();
    l.contains("windows") || l.contains("win")
}

fn looks_like_linux_variant(v: &str) -> bool {
    let l = v.to_lowercase();
    l.starts_with("ubuntu")
        || l.starts_with("rhel")
        || l.starts_with("centos")
        || l.starts_with("debian")
        || l.starts_with("fedora")
        || l == "linux"
}

fn looks_like_windows_variant(v: &str) -> bool {
    let l = v.to_lowercase();
    l.contains("win")
}
