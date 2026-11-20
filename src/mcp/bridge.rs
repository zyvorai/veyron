// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! `veyron mcp`: relay newline-delimited JSON-RPC between stdin/stdout and a
//! Veyron API's Streamable HTTP endpoint, for MCP clients that only speak stdio.
//! Logs go to stderr; stdout carries protocol messages only.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub fn endpoint(base: &str) -> String {
    let b = base.trim_end_matches('/');
    if b.ends_with("/mcp") {
        b.to_string()
    } else {
        format!("{b}/mcp")
    }
}

pub async fn run_stdio(url: &str, api_key: Option<&str>, insecure: bool) -> Result<()> {
    let key = api_key
        .filter(|k| !k.is_empty())
        .context("Set --api-key or VEYRON_API_KEY")?;
    let endpoint = endpoint(url);
    let http = reqwest::Client::builder()
        .danger_accept_invalid_certs(insecure)
        .timeout(std::time::Duration::from_secs(600))
        .build()?;
    eprintln!("veyron mcp: relaying stdio to {endpoint}");

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    let mut session: Option<String> = None;

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let id = serde_json::from_str::<Value>(line)
            .ok()
            .and_then(|v| v.get("id").cloned());

        let mut req = http
            .post(&endpoint)
            .header("x-api-key", key)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .body(line.to_string());
        if let Some(s) = &session {
            req = req.header("mcp-session-id", s);
        }

        let reply = match req.send().await {
            Ok(resp) => {
                if let Some(s) = resp
                    .headers()
                    .get("mcp-session-id")
                    .and_then(|v| v.to_str().ok())
                {
                    session = Some(s.to_string());
                }
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                if status == reqwest::StatusCode::ACCEPTED || body.trim().is_empty() {
                    None
                } else if status.is_success() {
                    Some(body.trim().to_string())
                } else {
                    id.as_ref().map(|id| {
                        error_line(
                            id,
                            &format!("Veyron returned HTTP {status}: {}", body.trim()),
                        )
                    })
                }
            }
            Err(e) => id
                .as_ref()
                .map(|id| error_line(id, &format!("Cannot reach Veyron: {e}"))),
        };
        if let Some(r) = reply {
            stdout.write_all(r.replace('\n', " ").as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

fn error_line(id: &Value, message: &str) -> String {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32603, "message": message}}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_appends_mcp_once() {
        assert_eq!(endpoint("https://h:5151"), "https://h:5151/mcp");
        assert_eq!(endpoint("https://h:5151/"), "https://h:5151/mcp");
        assert_eq!(endpoint("https://h:5151/mcp"), "https://h:5151/mcp");
    }
}
