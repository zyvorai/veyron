// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Client for kairon-node's per-node relay (`internal/consoleproxy`).
//!
//! kairon-node runs with `hostNetwork: true` and serves the relay on the
//! node's own address (default port 8090), authenticated by the shared
//! `kairon-console-token` bearer. It is the only path to a Machine's VNC and
//! text console, QEMU guest-agent exec, guest firewall and runtime logs.
//!
//! Configuration:
//! - `KAIRON_NODE_CONSOLE_TOKEN` (or `VEYRON_KAIRON_CONSOLE_TOKEN`) — required.
//! - `VEYRON_KAIRON_NODE_URL` — URL template, default `http://{node_ip}:8090`.
//!   `{node}` expands to the node name, `{node_ip}` to its InternalIP.
//! - `VEYRON_KAIRON_NODE_TLS_INSECURE=1` — skip relay TLS verification
//!   (only meaningful with an `https://` template).

use super::Machine;
use anyhow::{Context, Result, anyhow, bail};
use k8s_openapi::api::core::v1::Node;
use kube::{Api, Client};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const DEFAULT_RELAY_URL: &str = "http://{node_ip}:8090";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub powershell: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

impl ExecRequest {
    pub fn shell(cmd: &str, timeout_seconds: u64) -> Self {
        Self {
            path: Some("/bin/sh".into()),
            args: vec!["-c".into(), cmd.into()],
            powershell: None,
            timeout_seconds: Some(timeout_seconds),
        }
    }

    pub fn powershell(script: &str, timeout_seconds: u64) -> Self {
        Self {
            powershell: Some(script.into()),
            timeout_seconds: Some(timeout_seconds),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecResult {
    #[serde(default)]
    pub exit_code: i64,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
}

pub fn relay_token() -> Option<String> {
    std::env::var("KAIRON_NODE_CONSOLE_TOKEN")
        .or_else(|_| std::env::var("VEYRON_KAIRON_CONSOLE_TOKEN"))
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

pub fn relay_configured() -> bool {
    relay_token().is_some()
}

fn relay_template() -> String {
    std::env::var("VEYRON_KAIRON_NODE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_RELAY_URL.to_string())
}

pub fn tls_insecure() -> bool {
    matches!(
        std::env::var("VEYRON_KAIRON_NODE_TLS_INSECURE").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Expand the relay URL template for a node.
pub fn expand_template(template: &str, node: &str, node_ip: &str) -> String {
    template
        .replace("{node_ip}", node_ip)
        .replace("{node}", node)
        .trim_end_matches('/')
        .to_string()
}

async fn node_internal_ip(client: &Client, node: &str) -> Result<String> {
    let n = Api::<Node>::all(client.clone())
        .get(node)
        .await
        .with_context(|| format!("get node {node}"))?;
    let addrs = n.status.and_then(|s| s.addresses).unwrap_or_default();
    addrs
        .iter()
        .find(|a| a.type_ == "InternalIP")
        .or_else(|| addrs.iter().find(|a| a.type_ == "ExternalIP"))
        .map(|a| a.address.clone())
        .ok_or_else(|| anyhow!("node {node} has no InternalIP/ExternalIP address"))
}

/// Relay target for one running Machine.
#[derive(Debug, Clone)]
pub struct RelayTarget {
    pub base_url: String,
    pub runtime_id: String,
    pub token: String,
}

impl RelayTarget {
    pub async fn for_machine(client: &Client, m: &Machine) -> Result<Self> {
        let token = relay_token().ok_or_else(|| {
            anyhow!("kairon-node relay is not configured: set KAIRON_NODE_CONSOLE_TOKEN from the kairon-console-token Secret")
        })?;
        let runtime_id = m
            .runtime_id()
            .ok_or_else(|| {
                anyhow!(
                    "machine {}/{} has no runtime yet (phase {})",
                    m.namespace(),
                    m.name(),
                    m.phase()
                )
            })?
            .to_string();
        let node = m
            .node()
            .ok_or_else(|| {
                anyhow!(
                    "machine {}/{} is not scheduled to a node",
                    m.namespace(),
                    m.name()
                )
            })?
            .to_string();
        let template = relay_template();
        let node_ip = if template.contains("{node_ip}") {
            node_internal_ip(client, &node).await?
        } else {
            String::new()
        };
        Ok(Self {
            base_url: expand_template(&template, &node, &node_ip),
            runtime_id,
            token,
        })
    }

    /// Target for node-level relay routes that aren't about one runtime
    /// (e.g. backup downloads).
    pub async fn for_node(client: &Client, node: &str) -> Result<Self> {
        let token = relay_token().ok_or_else(|| {
            anyhow!("kairon-node relay is not configured: set KAIRON_NODE_CONSOLE_TOKEN from the kairon-console-token Secret")
        })?;
        let template = relay_template();
        let node_ip = if template.contains("{node_ip}") {
            node_internal_ip(client, node).await?
        } else {
            String::new()
        };
        Ok(Self {
            base_url: expand_template(&template, node, &node_ip),
            runtime_id: String::new(),
            token,
        })
    }

    /// Stream a FluxVM backup's root qcow2 (`GET /backup-root/{name}`). No
    /// overall timeout: the body is the whole disk.
    pub async fn backup_root(&self, backup: &str) -> Result<reqwest::Response> {
        let resp = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(tls_insecure())
            .build()
            .context("build relay HTTP client")?
            .get(format!(
                "{}/backup-root/{}",
                self.base_url,
                urlencode(backup)
            ))
            .bearer_auth(&self.token)
            .send()
            .await
            .context("kairon-node relay backup-root")?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!(
                "kairon-node relay /backup-root returned {status}: {}",
                text.trim()
            );
        }
        Ok(resp)
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}/{}",
            self.base_url,
            path.trim_matches('/'),
            urlencode(&self.runtime_id)
        )
    }

    /// `ws(s)://…/console/{runtimeID}` (VNC) or `…/text-console/{runtimeID}`.
    pub fn websocket_url(&self, kind: &str) -> String {
        let http = self.url(kind);
        if let Some(rest) = http.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = http.strip_prefix("http://") {
            format!("ws://{rest}")
        } else {
            http
        }
    }

    fn http(&self, timeout: Duration) -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(timeout)
            .danger_accept_invalid_certs(tls_insecure())
            .build()
            .context("build relay HTTP client")
    }

    async fn post_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &impl Serialize,
        timeout: Duration,
    ) -> Result<T> {
        let resp = self
            .http(timeout)?
            .post(self.url(path))
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
            .with_context(|| format!("kairon-node relay POST /{path}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!(
                "kairon-node relay /{path} returned {status}: {}",
                text.trim()
            );
        }
        serde_json::from_str(&text).with_context(|| format!("decode relay /{path} response"))
    }

    /// QEMU guest-agent `guest-exec` (Linux path+args, or Windows PowerShell).
    pub async fn exec(&self, req: &ExecRequest) -> Result<ExecResult> {
        let secs = req.timeout_seconds.unwrap_or(60) + 15;
        self.post_json("exec", req, Duration::from_secs(secs)).await
    }

    pub async fn firewall_open(&self, name: &str, port: u16, protocol: &str) -> Result<ExecResult> {
        let body = serde_json::json!({"name": name, "port": port, "protocol": protocol});
        self.post_json("qga-firewall/open", &body, Duration::from_secs(90))
            .await
    }

    pub async fn firewall_close(&self, name: &str) -> Result<ExecResult> {
        let body = serde_json::json!({"name": name});
        self.post_json("qga-firewall/close", &body, Duration::from_secs(90))
            .await
    }

    /// Hypervisor-level checkpoint (RAM + CPU + device state) tagged `tag`; the
    /// Machine keeps running.
    pub async fn vm_snapshot(&self, tag: &str) -> Result<()> {
        let body = serde_json::json!({ "tag": tag });
        let _: serde_json::Value = self
            .post_json("vm-snapshot", &body, Duration::from_secs(330))
            .await?;
        Ok(())
    }

    /// Stops the Machine and starts it back from checkpoint `tag`.
    pub async fn vm_restore_snapshot(&self, tag: &str) -> Result<()> {
        let body = serde_json::json!({ "tag": tag });
        let _: serde_json::Value = self
            .post_json("vm-restore-snapshot", &body, Duration::from_secs(330))
            .await?;
        Ok(())
    }

    pub async fn fsfreeze_status(&self) -> Result<String> {
        let resp = self
            .http(Duration::from_secs(30))?
            .get(self.url("qga-fsfreeze-status"))
            .bearer_auth(&self.token)
            .send()
            .await
            .context("kairon-node relay fsfreeze status")?
            .error_for_status()?;
        let v: serde_json::Value = resp.json().await?;
        Ok(v.get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown")
            .to_string())
    }

    /// Runtime (FluxVM/QEMU) logs; `query` is passed through (e.g. `tail=500`).
    pub async fn logs(&self, query: &str) -> Result<String> {
        let mut url = self.url("logs");
        if !query.is_empty() {
            url = format!("{url}?{query}");
        }
        let resp = self
            .http(Duration::from_secs(30))?
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await
            .context("kairon-node relay logs")?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!("kairon-node relay /logs returned {status}: {}", text.trim());
        }
        Ok(text)
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_expansion() {
        assert_eq!(
            expand_template(DEFAULT_RELAY_URL, "n1", "10.0.0.7"),
            "http://10.0.0.7:8090"
        );
        assert_eq!(
            expand_template("https://{node}.relay.local:9443/", "worker-2", ""),
            "https://worker-2.relay.local:9443"
        );
    }

    #[test]
    fn websocket_urls() {
        let t = RelayTarget {
            base_url: "https://10.0.0.7:8090".into(),
            runtime_id: "vm/1".into(),
            token: "t".into(),
        };
        assert_eq!(
            t.websocket_url("console"),
            "wss://10.0.0.7:8090/console/vm%2F1"
        );
        let t = RelayTarget {
            base_url: "http://10.0.0.7:8090".into(),
            ..t
        };
        assert_eq!(
            t.websocket_url("text-console"),
            "ws://10.0.0.7:8090/text-console/vm%2F1"
        );
    }

    #[test]
    fn exec_request_shapes() {
        let v = serde_json::to_value(ExecRequest::shell("uptime", 30)).unwrap();
        assert_eq!(v["path"], "/bin/sh");
        assert_eq!(v["args"][1], "uptime");
        assert_eq!(v["timeoutSeconds"], 30);
        let v = serde_json::to_value(ExecRequest::powershell("Get-Date", 10)).unwrap();
        assert_eq!(v["powershell"], "Get-Date");
        assert!(v.get("path").is_none());
    }
}
