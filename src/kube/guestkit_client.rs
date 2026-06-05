// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

//! Host-side GuestKit JSON-RPC client via virt-launcher pod exec.

use anyhow::{Context, Result};
use guestkit_agent_protocol::{read_line, write_line};
use k8s_openapi::api::core::v1::Pod;
use kube::api::{AttachParams, Api, ListParams};
use kube::Client;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

pub const GUESTKIT_BIN_HOST: &str = "/usr/local/bin/guestkit";
pub const GUEST_AGENT_CHANNEL: &str = "org.qemu.guest_agent.0";

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
    "find /run/libvirt /var/run/kubevirt-private /var/lib/libvirt -name 'org.qemu.guest_agent.0' 2>/dev/null | head -1"
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

async fn discover_libvirt_domain(pods: &Api<Pod>, pod_name: &str) -> Result<String> {
    let (stdout, stderr, _) = exec_capture(
        pods,
        pod_name,
        vec![
            "virsh".to_string(),
            "list".to_string(),
            "--name".to_string(),
        ],
    )
    .await?;
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("no libvirt domain in virt-launcher {pod_name}: {stderr}"))
}

fn jsonrpc_method_to_qga_execute(method: &str, params: &Value) -> Result<Value> {
    match method {
        "guestkit.getEvidence" => Ok(json!({ "execute": "guestkit-get-evidence" })),
        "guestkit.doctor" => Ok(json!({
            "execute": "guestkit-doctor",
            "arguments": params,
        })),
        "guestkit.getCapabilities" => Ok(json!({ "execute": "guestkit-get-capabilities" })),
        "guestkit.getVersion" => Ok(json!({ "execute": "guestkit-get-version" })),
        "guestkit.runFixPlan" => Ok(json!({
            "execute": "guestkit-run-fix-plan",
            "arguments": params,
        })),
        other => anyhow::bail!("unsupported GuestKit RPC method for KubeVirt: {other}"),
    }
}

fn parse_qga_response(stdout: &str, stderr: &str) -> Result<Value> {
    let combined = if stdout.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };
    if combined.is_empty() {
        anyhow::bail!("virsh qemu-agent-command returned no output");
    }
    let value: Value = serde_json::from_str(combined)
        .with_context(|| format!("parse virsh output: {combined}"))?;
    if let Some(ret) = value.get("return") {
        return Ok(ret.clone());
    }
    if let Some(err) = value.get("error") {
        anyhow::bail!("guest agent error: {err}");
    }
    Ok(value)
}

/// Invoke GuestKit JSON-RPC inside virt-launcher via `virsh qemu-agent-command`.
pub async fn guestkit_rpc_for_vm(
    client: Client,
    namespace: &str,
    vm_name: &str,
    method: &str,
    params: Value,
) -> Result<Value> {
    let pod_name = find_virt_launcher_pod(&client, namespace, vm_name).await?;
    let pods: Api<Pod> = Api::namespaced(client, namespace);
    let domain = discover_libvirt_domain(&pods, &pod_name).await?;
    let qga_cmd = jsonrpc_method_to_qga_execute(method, &params)?;
    let payload = serde_json::to_string(&qga_cmd)?;
    let (stdout, stderr, _) = exec_capture(
        &pods,
        &pod_name,
        vec![
            "virsh".to_string(),
            "qemu-agent-command".to_string(),
            domain,
            payload,
        ],
    )
    .await?;
    parse_qga_response(&stdout, &stderr)
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
