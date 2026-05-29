// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! In-guest filesystem usage via QEMU guest-agent (`guest-exec`).

use anyhow::Result;
use kube::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::guest_os::{detect_guest_os_family, GuestOsFamily};
use super::kubevirt_subresources::{vmi_guest_exec, wait_guest_exec};
use super::windows_rdp::{is_windows_guest, parse_guest_exec_response, vmi_guest_agent_connected};

const LINUX_DF_CMD: &str = "df -P";
const WINDOWS_FS_PS1: &str = r"Get-PSDrive -PSProvider FileSystem | ForEach-Object { '{0}|{1}|{2}|{3}' -f $_.Name,$_.Used,$_.Free,$_.Root }";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuestFilesystemMount {
    pub mount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<String>,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub avail_bytes: u64,
    pub use_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestFilesystemResponse {
    pub namespace: String,
    pub vm_name: String,
    pub vmi_name: String,
    pub guest_agent_connected: bool,
    pub os_family: String,
    pub mounts: Vec<GuestFilesystemMount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prometheus_mounts: Vec<GuestFilesystemMount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

pub fn linux_df_guest_exec_body() -> Value {
    json!({
        "path": "/bin/sh",
        "arg": ["-c", LINUX_DF_CMD],
        "capture-output": true
    })
}

pub fn windows_fs_guest_exec_body() -> Value {
    json!({
        "path": r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
        "arg": [
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            WINDOWS_FS_PS1
        ],
        "capture-output": true
    })
}

/// Parse POSIX `df -P` output (1K blocks).
pub fn parse_linux_df_output(stdout: &str) -> Vec<GuestFilesystemMount> {
    let mut mounts = Vec::new();
    for line in stdout.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        let filesystem = parts[0].to_string();
        let blocks_1k = parts[1].parse::<u64>().unwrap_or(0);
        let used_1k = parts[2].parse::<u64>().unwrap_or(0);
        let avail_1k = parts[3].parse::<u64>().unwrap_or(0);
        let capacity = parts[4].trim_end_matches('%');
        let mount = parts[5..].join(" ");
        let use_percent = capacity.parse::<f64>().unwrap_or_else(|_| {
            if blocks_1k > 0 {
                (used_1k as f64 / blocks_1k as f64) * 100.0
            } else {
                0.0
            }
        });
        let size_bytes = blocks_1k.saturating_mul(1024);
        let used_bytes = used_1k.saturating_mul(1024);
        let avail_bytes = avail_1k.saturating_mul(1024);
        mounts.push(GuestFilesystemMount {
            mount,
            filesystem: Some(filesystem),
            size_bytes,
            used_bytes,
            avail_bytes,
            use_percent,
        });
    }
    mounts
}

/// Parse Windows `Get-PSDrive` pipe output: `Name|Used|Free|Root`.
pub fn parse_windows_psdrive_output(stdout: &str) -> Vec<GuestFilesystemMount> {
    let mut mounts = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 4 {
            continue;
        }
        let name = parts[0].trim();
        let used = parts[1].trim().parse::<u64>().unwrap_or(0);
        let free = parts[2].trim().parse::<u64>().unwrap_or(0);
        let root = parts[3].trim();
        let size_bytes = used.saturating_add(free);
        let use_percent = if size_bytes > 0 {
            (used as f64 / size_bytes as f64) * 100.0
        } else {
            0.0
        };
        mounts.push(GuestFilesystemMount {
            mount: if root.is_empty() {
                format!("{name}:")
            } else {
                root.to_string()
            },
            filesystem: Some("NTFS".into()),
            size_bytes,
            used_bytes: used,
            avail_bytes: free,
            use_percent,
        });
    }
    mounts
}

async fn run_guest_exec_collect(
    client: Client,
    namespace: &str,
    vmi_name: &str,
    body: Value,
) -> Result<(Option<i64>, Option<String>, Option<String>)> {
    let mut guest_exec = vmi_guest_exec(client.clone(), namespace, vmi_name, body).await?;
    let (mut exit_code, mut stdout, mut stderr) = parse_guest_exec_response(&guest_exec);
    let pid_only = guest_exec.get("pid").is_some() && exit_code.is_none() && stdout.is_none();
    if pid_only {
        if let Some(pid) = guest_exec.get("pid").and_then(|p| p.as_i64()) {
            if let Ok(Some(status)) = wait_guest_exec(
                client,
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
    Ok((exit_code, stdout, stderr))
}

/// Collect in-guest mount usage for a running VMI with guest agent connected.
pub async fn collect_guest_filesystem(
    client: Client,
    namespace: &str,
    vm_name: &str,
    vmi_name: &str,
    vm_json: &Value,
    vmi_json: &Value,
) -> Result<GuestFilesystemResponse> {
    let family = detect_guest_os_family(Some(vm_json), Some(vmi_json));
    let agent_up = vmi_guest_agent_connected(vmi_json);
    if !agent_up {
        anyhow::bail!("QEMU guest agent is not connected (AgentConnected condition is not True)");
    }

    let windows = is_windows_guest(Some(vm_json), Some(vmi_json))
        || family == GuestOsFamily::Windows;
    let body = if windows {
        windows_fs_guest_exec_body()
    } else {
        linux_df_guest_exec_body()
    };

    let (exit_code, stdout, stderr) =
        run_guest_exec_collect(client, namespace, vmi_name, body).await?;

    let mounts = stdout
        .as_deref()
        .map(|out| {
            if windows {
                parse_windows_psdrive_output(out)
            } else {
                parse_linux_df_output(out)
            }
        })
        .unwrap_or_default();

    let success = exit_code.unwrap_or(0) == 0 && !mounts.is_empty();
    let message = if success {
        Some(format!("Collected {} mount(s) from guest.", mounts.len()))
    } else if exit_code.unwrap_or(0) != 0 {
        Some(format!(
            "Guest exec exit {:?}: {}",
            exit_code,
            stderr.as_deref().unwrap_or("no stderr")
        ))
    } else {
        Some("Guest exec returned no mount lines.".into())
    };

    Ok(GuestFilesystemResponse {
        namespace: namespace.to_string(),
        vm_name: vm_name.to_string(),
        vmi_name: vmi_name.to_string(),
        guest_agent_connected: true,
        os_family: family.as_str().to_string(),
        mounts,
        prometheus_mounts: Vec::new(),
        sources: vec!["guest-exec".into()],
        message,
        exit_code,
        stderr,
    })
}

/// Format a mount row for copilot evidence lines.
pub fn format_mount_evidence(m: &GuestFilesystemMount) -> String {
    format!(
        "{} — {:.1}% used ({}/{} bytes free)",
        m.mount,
        m.use_percent,
        m.used_bytes,
        m.avail_bytes
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_linux_df() {
        let out = "Filesystem     1024-blocks     Used Available Capacity Mounted on\n\
/dev/vda1        26214400  1310720  24903680       5% /\n\
tmpfs             1048576        0   1048576       0% /dev/shm\n";
        let mounts = parse_linux_df_output(out);
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].mount, "/");
        assert!((mounts[0].use_percent - 5.0).abs() < 0.01);
        assert_eq!(mounts[0].size_bytes, 26214400 * 1024);
    }

    #[test]
    fn parse_windows_psdrive() {
        let out = "C|50000000000|15000000000|C:\\\nD|10000000000|9000000000|D:\\\n";
        let mounts = parse_windows_psdrive_output(out);
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].mount, "C:\\");
        assert_eq!(mounts[0].size_bytes, 65_000_000_000);
        assert!((mounts[0].use_percent - (50_000_000_000.0 / 65_000_000_000.0 * 100.0)).abs() < 0.01);
    }
}
