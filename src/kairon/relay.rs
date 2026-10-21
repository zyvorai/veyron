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

/// A template with neither `{node}` nor `{node_ip}` sends every Machine's relay
/// call to one fixed node, so VMs scheduled anywhere else answer "VM not found".
pub fn template_is_per_node(template: &str) -> bool {
    template.contains("{node_ip}") || template.contains("{node}")
}

fn warn_if_fixed_template(template: &str) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    if !template_is_per_node(template) {
        ONCE.call_once(|| {
            log::warn!(
                "VEYRON_KAIRON_NODE_URL={template} has no {{node_ip}}/{{node}} placeholder: relay calls (logs, snapshots, exec, consoles) all go to that one node and fail for Machines scheduled elsewhere"
            );
        });
    }
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
        warn_if_fixed_template(&template);
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
        warn_if_fixed_template(&template);
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

    /// Writes `data` to `path` in the guest over FluxVM's vsock agent
    /// (`POST /agent-file/put/{id}`); needs `spec.guestAgent.console`.
    pub async fn agent_put_file(&self, path: &str, data: &[u8], mode: Option<u32>) -> Result<()> {
        use base64::Engine as _;
        let body = serde_json::json!({
            "path": path,
            "contentBase64": base64::engine::general_purpose::STANDARD.encode(data),
            "mode": mode,
        });
        let _: serde_json::Value = self
            .post_json("agent-file/put", &body, Duration::from_secs(120))
            .await?;
        Ok(())
    }

    /// Reads `path` from the guest (`POST /agent-file/get/{id}`); returns bytes and mode.
    pub async fn agent_get_file(&self, path: &str) -> Result<(Vec<u8>, u32)> {
        use base64::Engine as _;
        let v: serde_json::Value = self
            .post_json(
                "agent-file/get",
                &serde_json::json!({ "path": path }),
                Duration::from_secs(120),
            )
            .await?;
        let b64 = v
            .get("contentBase64")
            .and_then(|x| x.as_str())
            .unwrap_or_default();
        let data = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .context("decode relay agent-file content")?;
        let mode = v.get("mode").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        Ok((data, mode))
    }

    /// Generic per-runtime relay call: `{base}/{path}/{runtime_id}/{segments..}?{query}`.
    /// Segments are percent-encoded. Non-2xx answers become errors carrying the
    /// relay's message; a 2xx response is returned unread so callers can stream it.
    pub async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        segments: &[&str],
        query: &str,
        body: Option<Vec<u8>>,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Response> {
        let mut url = self.url(path);
        for seg in segments {
            url.push('/');
            url.push_str(&urlencode(seg));
        }
        self.send(method, url, path, query, body, timeout).await
    }

    /// Node-scoped relay call with no runtime id (`{base}/{path}`), e.g. `capabilities`
    /// and `egress-check`.
    pub async fn call_node(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/{}", self.base_url, path.trim_matches('/'));
        let body = body.map(serde_json::to_vec).transpose()?;
        let resp = self
            .send(
                method,
                url,
                path,
                query,
                body,
                Some(Duration::from_secs(60)),
            )
            .await?;
        let bytes = resp.bytes().await?;
        if bytes.is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_slice(&bytes).with_context(|| format!("decode relay /{path} response"))
    }

    async fn send(
        &self,
        method: reqwest::Method,
        mut url: String,
        path: &str,
        query: &str,
        body: Option<Vec<u8>>,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Response> {
        if !query.is_empty() {
            url.push('?');
            url.push_str(query);
        }
        let mut builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(tls_insecure());
        if let Some(t) = timeout {
            builder = builder.timeout(t);
        }
        let mut req = builder
            .build()
            .context("build relay HTTP client")?
            .request(method, url)
            .bearer_auth(&self.token);
        if let Some(b) = body {
            req = req.header("content-type", "application/json").body(b);
        }
        let resp = req
            .send()
            .await
            .with_context(|| format!("kairon-node relay /{path}"))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!(
                "kairon-node relay /{path} returned {status}: {}",
                text.trim()
            );
        }
        Ok(resp)
    }

    /// [`Self::call`] decoded as JSON (empty body => `null`).
    pub async fn call_json(
        &self,
        method: reqwest::Method,
        path: &str,
        segments: &[&str],
        query: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let body = body.map(serde_json::to_vec).transpose()?;
        let resp = self
            .call(
                method,
                path,
                segments,
                query,
                body,
                Some(Duration::from_secs(120)),
            )
            .await?;
        let bytes = resp.bytes().await?;
        if bytes.is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_slice(&bytes).with_context(|| format!("decode relay /{path} response"))
    }

    /// Writes a guest file: FluxVM's vsock agent when the Machine has
    /// `spec.guestAgent.console`, otherwise base64 chunks through guest exec.
    pub async fn put_file(&self, path: &str, data: &[u8], mode: Option<u32>) -> Result<()> {
        match self.agent_put_file(path, data, mode).await {
            Err(e) if vsock_agent_missing(&e) => self.exec_put_file(path, data).await,
            r => r,
        }
    }

    /// Reads a guest file, with the same vsock-then-exec fallback as [`Self::put_file`].
    pub async fn get_file(&self, path: &str, max: usize) -> Result<(Vec<u8>, u32)> {
        match self.agent_get_file(path).await {
            Err(e) if vsock_agent_missing(&e) => Ok((self.exec_get_file(path, max).await?, 0)),
            r => r,
        }
    }

    /// File write over guest exec (`base64 -d >> path`), for guests without the vsock agent.
    pub async fn exec_put_file(&self, path: &str, data: &[u8]) -> Result<()> {
        use base64::Engine as _;
        const CHUNK: usize = 48 * 1024;
        let p = shell_quote(path);
        let first = self
            .exec(&ExecRequest::shell(
                &format!("mkdir -p \"$(dirname {p})\" && : > {p}"),
                30,
            ))
            .await?;
        if first.exit_code != 0 {
            bail!("cannot write {path}: {}", first.stderr.trim());
        }
        for chunk in data.chunks(CHUNK) {
            let b64 = base64::engine::general_purpose::STANDARD.encode(chunk);
            let r = self
                .exec(&ExecRequest::shell(
                    &format!("printf %s '{b64}' | base64 -d >> {p}"),
                    30,
                ))
                .await?;
            if r.exit_code != 0 {
                bail!("cannot write {path}: {}", r.stderr.trim());
            }
        }
        Ok(())
    }

    /// File read over guest exec (`base64 -w0`), refusing files larger than `max` bytes.
    pub async fn exec_get_file(&self, path: &str, max: usize) -> Result<Vec<u8>> {
        use base64::Engine as _;
        let p = shell_quote(path);
        let size = self
            .exec(&ExecRequest::shell(&format!("stat -c %s {p}"), 30))
            .await?;
        if size.exit_code != 0 {
            bail!("cannot read {path}: {}", size.stderr.trim());
        }
        if size.stdout.trim().parse::<usize>().unwrap_or(0) > max {
            bail!("File is larger than {} MiB", max >> 20);
        }
        let r = self
            .exec(&ExecRequest::shell(&format!("base64 -w0 {p}"), 60))
            .await?;
        if r.exit_code != 0 {
            bail!("cannot read {path}: {}", r.stderr.trim());
        }
        Ok(base64::engine::general_purpose::STANDARD.decode(r.stdout.trim())?)
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

fn vsock_agent_missing(e: &anyhow::Error) -> bool {
    format!("{e:#}").contains("guest agent is not enabled")
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
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
    fn fixed_template_is_not_per_node() {
        assert!(template_is_per_node(DEFAULT_RELAY_URL));
        assert!(template_is_per_node("https://{node}.relay.local:9443"));
        assert!(!template_is_per_node("https://80.79.5.173:8092"));
        // A fixed URL expands to the same target for every node.
        assert_eq!(
            expand_template("https://80.79.5.173:8092", "n1", "10.0.0.1"),
            expand_template("https://80.79.5.173:8092", "n2", "10.0.0.2"),
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

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn detects_missing_vsock_agent() {
        let e = anyhow!("relay returned 502: guest agent is not enabled for this VM");
        assert!(vsock_agent_missing(&e));
        assert!(!vsock_agent_missing(&anyhow!("connection refused")));
    }
}
