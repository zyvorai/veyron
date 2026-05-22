//! Default Remote Desktop (TCP 3389) for Windows KubeVirt VMs.
//!
//! Merges masquerade interface port 3389 and optional cloud-init / config-drive userdata.

use base64::Engine;
use kube::Client;
use serde::Serialize;
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

pub fn is_windows_guest(vm: Option<&Value>, vmi: Option<&Value>) -> bool {
    vm.is_some_and(is_windows_vm) || vmi.is_some_and(is_windows_vmi)
}

pub fn is_windows_vmi(vmi: &Value) -> bool {
    let info = vmi
        .pointer("/status/guestOSInfo")
        .or_else(|| vmi.get("guestOSInfo"));
    let Some(info) = info else {
        return false;
    };
    if info
        .get("id")
        .and_then(|v| v.as_str())
        .is_some_and(|id| {
            id.eq_ignore_ascii_case("mswindows")
                || id.eq_ignore_ascii_case("windows")
                || id.contains("win")
        })
    {
        return true;
    }
    for key in ["name", "prettyName", "version", "versionId"] {
        if let Some(s) = info.get(key).and_then(|v| v.as_str()) {
            if looks_like_windows_text(s) {
                return true;
            }
        }
    }
    false
}

fn looks_like_windows_text(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.contains("windows")
        || lower.contains("microsoft")
        || lower.starts_with("win ")
        || lower.contains(" win ")
}

pub fn vm_spec_has_rdp_port(vm: &Value) -> bool {
    vm.pointer("/spec/template/spec/domain/devices/interfaces")
        .and_then(|ifaces| ifaces.as_array())
        .is_some_and(|ifaces| {
            ifaces.iter().any(|iface| {
                iface
                    .get("ports")
                    .and_then(|ports| ports.as_array())
                    .is_some_and(|ports| {
                        ports.iter().any(|p| {
                            p.get("port")
                                .and_then(|n| n.as_i64())
                                .is_some_and(|n| n == i64::from(WINDOWS_RDP_PORT))
                                || p.get("name")
                                    .and_then(|n| n.as_str())
                                    .is_some_and(|n| n.eq_ignore_ascii_case("rdp"))
                        })
                    })
            })
        })
}

fn vm_name_suggests_windows(name: &str) -> bool {
    let lower = name.to_lowercase();
    if looks_like_windows_text(&lower) {
        return true;
    }
    if lower.contains("legacy") {
        return true;
    }
    lower.starts_with("win-")
        || lower.starts_with("win_")
        || lower.ends_with("-win")
        || lower.contains("-win-")
}

pub fn is_windows_vm(vm: &Value) -> bool {
    if let Some(name) = vm
        .get("metadata")
        .and_then(|m| m.get("name"))
        .and_then(|n| n.as_str())
    {
        if vm_name_suggests_windows(name) {
            return true;
        }
    }

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

        if labels
            .get("vmrogue.io/rdp")
            .and_then(|v| v.as_str())
            .is_some_and(|v| !v.is_empty() && !v.eq_ignore_ascii_case("false"))
        {
            return true;
        }

        if labels
            .get("hyper2kvm.io/migrated")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v == "true" || v.eq_ignore_ascii_case("yes"))
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

    if let Some(ann) = vm
        .get("metadata")
        .and_then(|m| m.get("annotations"))
        .and_then(|a| a.as_object())
    {
        for (k, v) in ann {
            let kl = k.to_lowercase();
            let vl = v.as_str().unwrap_or("").to_lowercase();
            if vl.contains("windows") && (kl.contains("os") || kl.contains("guest")) {
                return true;
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

    if vm.pointer("/spec/template/spec/domain/features/hyperv").is_some() {
        return true;
    }

    if vm.pointer("/spec/template/spec/domain/clock/timer")
        .and_then(|t| t.as_array())
        .is_some_and(|timers| {
            timers.iter().any(|timer| {
                timer
                    .get("name")
                    .and_then(|n| n.as_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case("HypervClock"))
            })
        })
    {
        return true;
    }

    if vm_spec_has_rdp_port(vm) {
        return true;
    }

    if let Some(volumes) = vm
        .pointer("/spec/template/spec/volumes")
        .and_then(|v| v.as_array())
    {
        for vol in volumes {
            let path = vol
                .pointer("/containerDisk/image")
                .or_else(|| vol.pointer("/dataVolume/name"))
                .or_else(|| vol.pointer("/persistentVolumeClaim/claimName"));
            if let Some(s) = path.and_then(|p| p.as_str()) {
                if s.to_lowercase().contains("windows") {
                    return true;
                }
            }
            for ci_key in ["cloudInitConfigDrive", "cloudInitNoCloud"] {
                if let Some(ud) = vol
                    .get(ci_key)
                    .and_then(|ci| ci.get("userData"))
                    .and_then(|u| u.as_str())
                {
                    let ud_l = ud.to_lowercase();
                    if ud_l.contains("ps1_sysnative")
                        || ud_l.contains("terminal server")
                        || ud_l.contains("fdenytsconnections")
                    {
                        return true;
                    }
                }
            }
        }
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

/// Inline PowerShell for `guest-exec` (same actions as [`RDP_USERDATA_PS1`]).
pub const WINDOWS_ENABLE_RDP_PS1_INLINE: &str = "Set-ItemProperty -Path 'HKLM:\\System\\CurrentControlSet\\Control\\Terminal Server' -Name 'fDenyTSConnections' -Value 0; Enable-NetFirewallRule -DisplayGroup 'Remote Desktop'; Set-ItemProperty -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Terminal Server\\WinStations\\RDP-Tcp' -Name 'UserAuthentication' -Value 0";

/// Inline PowerShell to disable incoming Remote Desktop in the guest.
pub const WINDOWS_DISABLE_RDP_PS1_INLINE: &str = "Set-ItemProperty -Path 'HKLM:\\System\\CurrentControlSet\\Control\\Terminal Server' -Name 'fDenyTSConnections' -Value 1; Disable-NetFirewallRule -DisplayGroup 'Remote Desktop'";

fn windows_rdp_guest_exec_body(ps1: &str) -> Value {
    json!({
        "path": r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
        "arg": [
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            ps1
        ],
        "capture-output": true
    })
}

/// KubeVirt `guest-exec` body to enable Remote Desktop inside a running Windows guest.
pub fn windows_enable_rdp_guest_exec_body() -> Value {
    windows_rdp_guest_exec_body(WINDOWS_ENABLE_RDP_PS1_INLINE)
}

/// KubeVirt `guest-exec` body to disable Remote Desktop inside a running Windows guest.
pub fn windows_disable_rdp_guest_exec_body() -> Value {
    windows_rdp_guest_exec_body(WINDOWS_DISABLE_RDP_PS1_INLINE)
}

#[derive(Debug, Serialize)]
pub struct RdpGuestAgentResponse {
    pub success: bool,
    pub guest_agent_connected: bool,
    pub is_windows_vm: bool,
    pub message: String,
    #[serde(rename = "guest_exec")]
    pub guest_exec: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

fn guest_exec_field_text(v: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let Some(raw) = v.get(*key).and_then(|x| x.as_str()).filter(|s| !s.is_empty()) else {
            continue;
        };
        let text = base64::engine::general_purpose::STANDARD
            .decode(raw.as_bytes())
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_else(|| raw.to_string());
        return Some(text);
    }
    None
}

fn guest_exec_exit_code(v: &Value) -> Option<i64> {
    v.get("exit-code")
        .or_else(|| v.get("exitcode"))
        .and_then(|x| x.as_i64())
}

/// Decode stdout/stderr and exit code from a `guest-exec` response.
pub fn parse_guest_exec_response(raw: &Value) -> (Option<i64>, Option<String>, Option<String>) {
    let exit = guest_exec_exit_code(raw);
    let stdout = guest_exec_field_text(raw, &["out-data", "out_data", "stdout"]);
    let stderr = guest_exec_field_text(raw, &["err-data", "err_data", "stderr"]);
    (exit, stdout, stderr)
}

/// QEMU guest agent connected per KubeVirt `AgentConnected` condition on a VMI JSON value.
pub fn vmi_guest_agent_connected(vmi: &Value) -> bool {
    vmi.pointer("/status/conditions")
        .or_else(|| vmi.get("conditions"))
        .and_then(|c| c.as_array())
        .is_some_and(|conds| {
            conds.iter().any(|c| {
                c.get("type").and_then(|t| t.as_str()) == Some("AgentConnected")
                    && c.get("status").and_then(|s| s.as_str()) == Some("True")
            })
        })
}

/// Enable or disable Windows Remote Desktop via QEMU guest-agent (`guest-exec`).
pub async fn set_rdp_via_guest_agent(
    client: Client,
    namespace: &str,
    _vm_name: &str,
    vmi_name: &str,
    vmi_json: &Value,
    vm_json: &Value,
    enable: bool,
) -> anyhow::Result<RdpGuestAgentResponse> {
    if !vmi_guest_agent_connected(vmi_json) {
        anyhow::bail!("QEMU guest agent is not connected (AgentConnected condition is not True)");
    }

    let is_windows = is_windows_guest(Some(vm_json), Some(vmi_json));
    let body = if enable {
        windows_enable_rdp_guest_exec_body()
    } else {
        windows_disable_rdp_guest_exec_body()
    };
    let mut guest_exec =
        super::kubevirt_subresources::vmi_guest_exec(client.clone(), namespace, vmi_name, body).await?;

    let mut exit_code;
    let mut stdout;
    let mut stderr;
    (exit_code, stdout, stderr) = parse_guest_exec_response(&guest_exec);
    let pid_only = guest_exec.get("pid").is_some() && exit_code.is_none() && stdout.is_none();

    if pid_only {
        if let Some(pid) = guest_exec.get("pid").and_then(|p| p.as_i64()) {
            if let Ok(Some(status)) = super::kubevirt_subresources::wait_guest_exec(
                client.clone(),
                namespace,
                vmi_name,
                pid,
                30,
                std::time::Duration::from_millis(500),
            )
            .await
            {
                guest_exec = status;
                (exit_code, stdout, stderr) = parse_guest_exec_response(&guest_exec);
            }
        }
    }

    let still_async = guest_exec.get("pid").is_some() && exit_code.is_none() && stdout.is_none();
    let success = !still_async && exit_code.unwrap_or(0) == 0;

    let message = if still_async {
        if enable {
            "Guest exec started (async). If RDP still fails, retry or enable Remote Desktop via VNC."
        } else {
            "Guest exec started (async). Remote Desktop disable may still be in progress."
        }
        .to_string()
    } else if success {
        if enable {
            "Remote Desktop registry and firewall rules updated in the guest.".to_string()
        } else {
            "Remote Desktop disabled in the guest (registry and firewall rules).".to_string()
        }
    } else {
        format!(
            "Guest exec finished with exit code {:?}. {}",
            exit_code,
            stderr.as_deref().unwrap_or("See guest_exec output.")
        )
    };

    Ok(RdpGuestAgentResponse {
        success,
        guest_agent_connected: true,
        is_windows_vm: is_windows,
        message,
        guest_exec,
        exit_code,
        stdout,
        stderr,
    })
}

/// Back-compat alias for [`RdpGuestAgentResponse`].
pub type EnableRdpGuestAgentResponse = RdpGuestAgentResponse;

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
    use serde_json::json;

    #[test]
    fn detects_hyper2kvm_migrated_legacy_vm() {
        let vm = json!({
            "metadata": {
                "name": "2025legacy-67790c6f",
                "labels": { "hyper2kvm.io/migrated": "true" }
            },
            "spec": { "template": { "spec": {
                "domain": { "devices": {
                    "interfaces": [{ "name": "default", "masquerade": {} }]
                }}
            }}}
        });
        assert!(is_windows_vm(&vm));
    }

    #[test]
    fn detects_windows_from_guest_os_info() {
        let vmi = json!({
            "status": {
                "guestOSInfo": {
                    "id": "mswindows",
                    "prettyName": "Windows Server 2025 Datacenter Evaluation"
                }
            }
        });
        assert!(is_windows_vmi(&vmi));
        assert!(is_windows_guest(None, Some(&vmi)));
    }

    #[test]
    fn enable_rdp_guest_exec_body_uses_powershell() {
        let body = windows_enable_rdp_guest_exec_body();
        assert!(
            body["path"]
                .as_str()
                .unwrap()
                .to_ascii_lowercase()
                .contains("powershell")
        );
        let joined = body["arg"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("fDenyTSConnections"));
        assert!(joined.contains("-Value 0"));
    }

    #[test]
    fn disable_rdp_guest_exec_body_denies_connections() {
        let joined = windows_disable_rdp_guest_exec_body()["arg"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("fDenyTSConnections"));
        assert!(joined.contains("-Value 1"));
        assert!(joined.contains("Disable-NetFirewallRule"));
    }

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
