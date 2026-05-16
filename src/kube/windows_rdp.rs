//! Default Remote Desktop (TCP 3389) for Windows KubeVirt VMs.
//!
//! Merges masquerade interface port 3389 and optional cloud-init / config-drive userdata.

use serde_json::{json, Value};

pub const WINDOWS_RDP_PORT: i32 = 3389;

const RDP_USERDATA_PS1: &str = r#"#ps1_sysnative
Set-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Control\Terminal Server' -Name 'fDenyTSConnections' -Value 0
Enable-NetFirewallRule -DisplayGroup 'Remote Desktop'
Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Terminal Server\WinStations\RDP-Tcp' -Name 'UserAuthentication' -Value 0
"#;

pub fn should_apply_windows_rdp(vm: &Value, template_name_hint: Option<&str>) -> bool {
    if template_name_hint
        .map(|t| t.to_lowercase().contains("windows"))
        .unwrap_or(false)
    {
        return true;
    }
    is_windows_vm(vm)
}

pub fn is_windows_vm(vm: &Value) -> bool {
    if let Some(labels) = vm
        .get("metadata")
        .and_then(|m| m.get("labels"))
        .and_then(|l| l.as_object())
    {
        if labels
            .get("vmrogue.io/guest-os")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.eq_ignore_ascii_case("windows"))
        {
            return true;
        }
        for (k, v) in labels {
            let kl = k.to_lowercase();
            let vl = v.as_str().unwrap_or("").to_lowercase();
            if vl.contains("windows")
                && (kl.contains("os")
                    || kl.contains("template")
                    || kl.starts_with("kubevirt.io/")
                    || kl.starts_with("os.template.kubevirt.io/"))
            {
                return true;
            }
        }
        for key in [
            "vm.kubevirt.io/template",
            "vmrogue.io/template",
            "kubevirt.io/template",
        ] {
            if let Some(t) = labels.get(key).and_then(|v| v.as_str()) {
                if t.to_lowercase().contains("windows") {
                    return true;
                }
            }
        }
    }

    if let Some(os_type) = vm
        .pointer("/spec/template/spec/domain/os/type")
        .and_then(|t| t.as_str())
    {
        if os_type.eq_ignore_ascii_case("windows") {
            return true;
        }
    }

    if vm.pointer("/spec/template/spec/domain/devices/video")
        .and_then(|v| v.as_array())
        .is_some_and(|arr| arr.iter().any(|v| v.get("qxl").is_some()))
    {
        return true;
    }

    false
}

pub fn merge_windows_rdp_defaults(vm: &mut Value) {
    merge_rdp_interface_ports(vm);
    merge_rdp_userdata(vm);
    merge_rdp_metadata_labels(vm);
}

fn merge_rdp_metadata_labels(vm: &mut Value) {
    if !vm.get("metadata").map(|m| m.is_object()).unwrap_or(false) {
        vm["metadata"] = json!({});
    }
    let Some(meta) = vm.get_mut("metadata").and_then(|m| m.as_object_mut()) else {
        return;
    };
    let labels = meta
        .entry("labels")
        .or_insert_with(|| json!({}))
        .as_object_mut();
    let Some(labels) = labels else {
        return;
    };
    labels
        .entry("vmrogue.io/rdp".to_string())
        .or_insert(json!("enabled"));
}

fn merge_rdp_interface_ports(vm: &mut Value) {
    let Some(interfaces) = vm
        .pointer_mut("/spec/template/spec/domain/devices/interfaces")
        .and_then(|i| i.as_array_mut())
    else {
        return;
    };

    let rdp_port = json!({
        "name": "rdp",
        "port": WINDOWS_RDP_PORT,
        "protocol": "TCP"
    });

    for iface in interfaces.iter_mut() {
        let has_masquerade = iface.get("masquerade").is_some();
        let has_bridge = iface.get("bridge").is_some();
        if !has_masquerade && !has_bridge {
            continue;
        }
        let Some(iface_obj) = iface.as_object_mut() else {
            continue;
        };
        if !iface_obj.contains_key("ports") {
            iface_obj.insert("ports".to_string(), json!([]));
        }
        let Some(ports) = iface_obj.get_mut("ports").and_then(|p| p.as_array_mut()) else {
            continue;
        };
        let already = ports.iter().any(|p| {
            p.get("port")
                .and_then(|n| n.as_i64())
                .is_some_and(|n| n == i64::from(WINDOWS_RDP_PORT))
                || p.get("name")
                    .and_then(|n| n.as_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case("rdp"))
        });
        if !already {
            ports.push(rdp_port.clone());
        }
    }
}

fn merge_rdp_userdata(vm: &mut Value) {
    let Some(volumes) = vm
        .pointer_mut("/spec/template/spec/volumes")
        .and_then(|v| v.as_array_mut())
    else {
        return;
    };

    for vol in volumes.iter_mut() {
        let ci_key = if vol.get("cloudInitConfigDrive").is_some() {
            "cloudInitConfigDrive"
        } else if vol.get("cloudInitNoCloud").is_some() {
            "cloudInitNoCloud"
        } else {
            continue;
        };
        let Some(ci) = vol.get_mut(ci_key) else {
            continue;
        };
        let existing = ci
            .get("userData")
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        if existing.contains("fDenyTSConnections") {
            return;
        }
        let merged = if existing.trim().is_empty() {
            RDP_USERDATA_PS1.to_string()
        } else {
            format!("{}\n\n{}", existing.trim_end(), RDP_USERDATA_PS1)
        };
        ci["userData"] = json!(merged);
        return;
    }
}

pub fn vmrogue_rdp_service_name(vm_name: &str) -> String {
    const PREFIX: &str = "rdp-";
    let max_body = 63usize.saturating_sub(PREFIX.len());
    let slug: String = vm_name
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '-' => c,
            _ => '-',
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(max_body)
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if slug.is_empty() {
        format!("{PREFIX}vm")
    } else {
        format!("{PREFIX}{slug}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_rdp_port_on_masquerade() {
        let mut vm = json!({
            "spec": { "template": { "spec": {
                "domain": { "devices": {
                    "interfaces": [{ "name": "default", "masquerade": {} }]
                }}
            }}}
        });
        merge_windows_rdp_defaults(&mut vm);
        let ports = &vm["spec"]["template"]["spec"]["domain"]["devices"]["interfaces"][0]["ports"];
        assert!(ports.as_array().unwrap().iter().any(|p| p["port"] == 3389));
    }
}
