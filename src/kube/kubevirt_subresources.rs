// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! KubeVirt `subresources.kubevirt.io/v1` helpers (guest-exec, …).

use anyhow::Result;
use kube::Client;
use serde_json::Value;
use std::time::Duration;

fn segment(s: &str) -> String {
    s.to_string()
}

fn format_kube_error(e: &kube::Error) -> String {
    match e {
        kube::Error::Api(resp) => {
            if !resp.message.is_empty() {
                format!("{} ({})", resp.message, resp.reason)
            } else {
                format!("{}: {} (code {})", resp.reason, resp.status, resp.code)
            }
        }
        other => format!("{other}"),
    }
}

const SUB: &str = "/apis/subresources.kubevirt.io/v1";

/// Freeze a running VMI in place (`PUT .../pause`).
pub async fn vmi_pause(client: Client, namespace: &str, vmi: &str) -> Result<()> {
    vmi_imperative_subresource(client, namespace, vmi, "pause").await
}

/// Resume a paused VMI (`PUT .../unpause`).
pub async fn vmi_unpause(client: Client, namespace: &str, vmi: &str) -> Result<()> {
    vmi_imperative_subresource(client, namespace, vmi, "unpause").await
}

async fn vmi_imperative_subresource(
    client: Client,
    namespace: &str,
    vmi: &str,
    action: &str,
) -> Result<()> {
    let uri = format!(
        "{SUB}/namespaces/{}/virtualmachineinstances/{}/{action}",
        segment(namespace),
        segment(vmi)
    );
    let req = http::Request::builder()
        .method(http::Method::PUT)
        .uri(&uri)
        .body(Vec::new())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let text = client
        .request_text(req)
        .await
        .map_err(|e| anyhow::anyhow!("{}", format_kube_error(&e)))?;
    if text.trim().is_empty() {
        return Ok(());
    }
    let _status: Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("pause/unpause response parse failed: {e}"))?;
    Ok(())
}

/// Run a command in the guest via QEMU guest-agent (`guest-exec`).
pub async fn vmi_guest_exec(
    client: Client,
    namespace: &str,
    vmi: &str,
    body: Value,
) -> Result<Value> {
    let uri = format!(
        "{SUB}/namespaces/{}/virtualmachineinstances/{}/guest-exec",
        segment(namespace),
        segment(vmi)
    );
    let payload = serde_json::to_vec(&body)?;
    let mut last_404 = false;

    for method in [http::Method::PUT, http::Method::POST] {
        let req = http::Request::builder()
            .method(method)
            .uri(&uri)
            .header("Content-Type", "application/json")
            .body(payload.clone())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        match client.request::<Value>(req).await {
            Ok(v) => return Ok(v),
            Err(kube::Error::Api(ref err)) if err.code == 404 => {
                last_404 = true;
                continue;
            }
            Err(e) => return Err(anyhow::anyhow!("{}", format_kube_error(&e))),
        }
    }

    Err(anyhow::anyhow!(
        "guest-exec is not available on this cluster API{}",
        if last_404 {
            " (subresource returned HTTP 404 for both PUT and POST)"
        } else {
            ""
        }
    ))
}

/// Poll async `guest-exec` completion via `guest-exec-status` (when the cluster exposes it).
pub async fn vmi_guest_exec_status(
    client: Client,
    namespace: &str,
    vmi: &str,
    pid: i64,
) -> Result<Value> {
    let uri = format!(
        "{SUB}/namespaces/{}/virtualmachineinstances/{}/guest-exec-status?pid={pid}",
        segment(namespace),
        segment(vmi)
    );
    let req = http::Request::builder()
        .method(http::Method::GET)
        .uri(&uri)
        .body(Vec::new())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    client
        .request::<Value>(req)
        .await
        .map_err(|e| anyhow::anyhow!("{}", format_kube_error(&e)))
}

/// Wait for a `guest-exec` PID to finish; returns the status payload when exit data is present.
pub async fn wait_guest_exec(
    client: Client,
    namespace: &str,
    vmi: &str,
    pid: i64,
    max_attempts: u32,
    interval: Duration,
) -> Result<Option<Value>> {
    for _ in 0..max_attempts {
        match vmi_guest_exec_status(client.clone(), namespace, vmi, pid).await {
            Ok(status) => {
                let has_exit = status.get("exit-code").is_some()
                    || status.get("exitcode").is_some()
                    || status.get("out-data").is_some()
                    || status.get("out_data").is_some()
                    || status.get("err-data").is_some()
                    || status.get("err_data").is_some();
                if has_exit {
                    return Ok(Some(status));
                }
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("404") || msg.contains("NotFound") {
                    return Ok(None);
                }
                return Err(e);
            }
        }
        tokio::time::sleep(interval).await;
    }
    Ok(None)
}
