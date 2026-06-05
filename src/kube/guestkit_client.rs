// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

//! Host-side GuestKit JSON-RPC client via virt-launcher pod exec.

use anyhow::{Context, Result};
use guestkit_agent_protocol::{read_line, write_line};
use k8s_openapi::api::core::v1::Pod;
use kube::api::{AttachParams, Api, ListParams};
use kube::Client;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const GUESTKIT_BIN_HOST: &str = "/usr/local/bin/guestkit";
pub const GUESTKIT_BIN_LAUNCHER: &str = "/tmp/vmrogue-guestkit";
pub const GUEST_AGENT_CHANNEL: &str = "org.qemu.guest_agent.0";

static INSTALLED_PODS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// Resolve virt-launcher pod name for a running VM.
pub async fn find_virt_launcher_pod(
    client: &Client,
    namespace: &str,
    vm_name: &str,
) -> Result<String> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let lp = ListParams::default().labels(&format!("kubevirt.io/vm={vm_name}"));
    let list = pods.list(&lp).await.context("list virt-launcher pods")?;
    let items = list.items;
    let pod = items
        .iter()
        .find(|p| {
            p.metadata
                .name
                .as_deref()
                .is_some_and(|n| n.starts_with("virt-launcher-"))
        })
        .or_else(|| items.first())
        .context(format!(
            "no virt-launcher pod for VM {namespace}/{vm_name} — is it running?"
        ))?;
    pod.metadata
        .name
        .clone()
        .context("virt-launcher pod has no name")
}

/// Discover guest agent unix socket inside virt-launcher (test helper / offline parse).
pub fn discover_agent_socket_from_find_output(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && line.contains(GUEST_AGENT_CHANNEL))
        .map(str::to_string)
}

/// Shell snippet to locate the QGA-compatible channel socket in virt-launcher.
pub fn socket_discovery_shell() -> &'static str {
    "find /var/run/kubevirt-private /var/lib/libvirt -name 'org.qemu.guest_agent.0' 2>/dev/null | head -1"
}

async fn exec_capture(
    pods: &Api<Pod>,
    pod_name: &str,
    cmd: Vec<String>,
) -> Result<(String, String, i32)> {
    let ap = AttachParams::default().stdout(true).stderr(true);
    let mut attached = pods
        .exec(pod_name, cmd, &ap)
        .await
        .context("pod exec attach")?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut out) = attached.stdout() {
        out.read_to_end(&mut stdout).await?;
    }
    if let Some(mut err) = attached.stderr() {
        err.read_to_end(&mut stderr).await?;
    }
    attached.join().await.context("pod exec join")?;
    Ok((
        String::from_utf8_lossy(&stdout).to_string(),
        String::from_utf8_lossy(&stderr).to_string(),
        0,
    ))
}

async fn ensure_guestkit_binary_in_pod(pods: &Api<Pod>, pod_name: &str) -> Result<()> {
    {
        let guard = INSTALLED_PODS.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        if guard.contains(pod_name) {
            return Ok(());
        }
    }

    let (stdout, _, _) = exec_capture(
        pods,
        pod_name,
        vec![
            "sh".to_string(),
            "-c".to_string(),
            format!("test -x {GUESTKIT_BIN_LAUNCHER} && echo ok"),
        ],
    )
    .await?;
    if stdout.trim() == "ok" {
        let mut guard = INSTALLED_PODS.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        guard.insert(pod_name.to_string());
        return Ok(());
    }

    let binary = std::fs::read(guestkit_binary_path()).with_context(|| {
        format!(
            "read guestkit binary at {} — rebuild API image with guestkit bundled",
            guestkit_binary_path().display()
        )
    })?;

    let ap = AttachParams::default().stdin(true).stdout(true).stderr(true);
    let mut attached = pods
        .exec(
            pod_name,
            vec![
                "tee".to_string(),
                GUESTKIT_BIN_LAUNCHER.to_string(),
            ],
            &ap,
        )
        .await
        .context("pod exec tee guestkit binary")?;
    if let Some(mut stdin) = attached.stdin() {
        stdin.write_all(&binary).await?;
    }
    attached.join().await.context("pod exec tee join")?;

    exec_capture(
        pods,
        pod_name,
        vec![
            "chmod".to_string(),
            "+x".to_string(),
            GUESTKIT_BIN_LAUNCHER.to_string(),
        ],
    )
    .await?;

    let mut guard = INSTALLED_PODS.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
    guard.insert(pod_name.to_string());
    Ok(())
}

async fn discover_socket_in_pod(pods: &Api<Pod>, pod_name: &str) -> Result<String> {
    let (stdout, stderr, _) = exec_capture(
        pods,
        pod_name,
        vec!["sh".to_string(), "-c".to_string(), socket_discovery_shell().to_string()],
    )
    .await?;
    discover_agent_socket_from_find_output(&stdout).ok_or_else(|| {
        anyhow::anyhow!(
            "guest agent socket not found in virt-launcher {pod_name}: {stderr}{stdout}"
        )
    })
}

/// Invoke GuestKit JSON-RPC inside virt-launcher via bundled `guestkit agent-call`.
pub async fn guestkit_rpc_for_vm(
    client: Client,
    namespace: &str,
    vm_name: &str,
    method: &str,
    params: Value,
) -> Result<Value> {
    let pod_name = find_virt_launcher_pod(&client, namespace, vm_name).await?;
    let pods: Api<Pod> = Api::namespaced(client, namespace);
    ensure_guestkit_binary_in_pod(&pods, &pod_name).await?;
    let socket = discover_socket_in_pod(&pods, &pod_name).await?;
    let params_json = serde_json::to_string(&params)?;
    let (stdout, stderr, _) = exec_capture(
        &pods,
        &pod_name,
        vec![
            GUESTKIT_BIN_LAUNCHER.to_string(),
            "agent-call".to_string(),
            "--socket".to_string(),
            socket,
            "--method".to_string(),
            method.to_string(),
            "--params".to_string(),
            params_json,
        ],
    )
    .await?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        anyhow::bail!("guestkit agent-call returned empty output: {stderr}");
    }
    serde_json::from_str(trimmed).context("parse guestkit agent-call JSON output")
}

/// Direct unix-socket JSON-RPC (host-side libvirt socket on node or virt-launcher).
pub fn call_agent_socket(socket_path: &str, method: &str, params: Value) -> Result<Value> {
    use std::io::BufReader;
    use std::os::unix::net::UnixStream;
    let mut stream = UnixStream::connect(socket_path)
        .with_context(|| format!("connect to agent socket {socket_path}"))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(120)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });
    write_line(&mut stream, &serde_json::to_vec(&req)?).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut reader = BufReader::new(stream);
    let frame = read_line(&mut reader).map_err(|e| anyhow::anyhow!("{e}"))?;
    let resp: Value = serde_json::from_slice(&frame).context("parse agent frame")?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("agent RPC error: {err}");
    }
    Ok(resp.get("result").cloned().unwrap_or(Value::Null))
}

pub fn guestkit_binary_path() -> PathBuf {
    std::env::var("VMROGUE_GUESTKIT_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(GUESTKIT_BIN_HOST).to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_socket_from_find_output() {
        let out = "/var/run/kubevirt-private/abc/org.qemu.guest_agent.0\n";
        assert_eq!(
            discover_agent_socket_from_find_output(out),
            Some("/var/run/kubevirt-private/abc/org.qemu.guest_agent.0".into())
        );
    }
}
