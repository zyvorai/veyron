//! Guest OS family detection (Windows vs Linux) for KubeVirt VMs and VMIs.

use serde_json::Value;

use super::windows_rdp::{is_windows_vm, is_windows_vmi};

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
    if guest_os_label(vm, "linux") {
        return true;
    }
    if let Some(labels) = vm
        .get("metadata")
        .and_then(|m| m.get("labels"))
        .and_then(|l| l.as_object())
    {
        if labels.contains_key("hyper2kvm.io/os-distro") {
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
    if vm_has_cloud_init(vm) && !is_windows_vm(vm) {
        return true;
    }
    false
}

fn is_linux_vmi(vmi: &Value) -> bool {
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
        "vmrogue.io/guest-os",
        "hyper2kvm.io/guest-os",
    ] {
        if vm
            .get("metadata")
            .and_then(|m| m.get("labels"))
            .and_then(|l| l.get(key))
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.eq_ignore_ascii_case(family))
        {
            return true;
        }
    }
    false
}

fn vm_has_cloud_init(vm: &Value) -> bool {
    vm.pointer("/spec/template/spec/volumes")
        .and_then(|v| v.as_array())
        .is_some_and(|vols| {
            vols.iter().any(|vol| {
                vol.get("cloudInitNoCloud").is_some() || vol.get("cloudInitConfigDrive").is_some()
            })
        })
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
