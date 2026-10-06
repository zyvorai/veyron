// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Kairon sandboxes: Firecracker Machines, forked from a running parent when
//! `VEYRON_SANDBOX_KAIRON_PARENT` (a Machine name in the sandbox namespace) is set,
//! which starts in well under a second. Exec goes through the kairon-node relay.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use base64::Engine;
use kube::api::{Api, DeleteParams, ListParams, PostParams};

use super::{
    ANN_CREATED, ANN_EXPIRES, ANN_INTERNET, ANN_OWNER, ExecResult, LABEL_SANDBOX, LABEL_STATE,
    STATE_CLAIMED, Sandbox, namespace, new_id,
};
use crate::kairon::relay::{ExecRequest, RelayTarget};
use crate::kairon::{ANNOTATION_FORK_FROM, LABEL_MANAGED_BY, Machine, RuntimeSpec};

const CHUNK: usize = 48 * 1024;

pub fn fork_parent() -> Option<String> {
    std::env::var("VEYRON_SANDBOX_KAIRON_PARENT")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn api(client: &kube::Client) -> Api<Machine> {
    Api::namespaced(client.clone(), &namespace())
}

fn to_sandbox(m: &Machine) -> Sandbox {
    let ann = |k: &str| {
        m.metadata
            .annotations
            .as_ref()
            .and_then(|a| a.get(k))
            .cloned()
            .unwrap_or_default()
    };
    let status = match m.phase() {
        "Running" if m.runtime_id().is_some() => "ready",
        "Failed" => "failed",
        _ => "starting",
    };
    Sandbox {
        id: m.name().to_string(),
        namespace: m.namespace().to_string(),
        owner: ann(ANN_OWNER),
        status: status.into(),
        backend: if fork_parent().is_some() {
            "kairon-fork".into()
        } else {
            "kairon-firecracker".into()
        },
        internet: ann(ANN_INTERNET) == "true",
        created_at: ann(ANN_CREATED),
        expires_at: ann(ANN_EXPIRES),
        vm: m.name().to_string(),
        isolated: true,
    }
}

pub async fn list(client: &kube::Client) -> Result<Vec<Sandbox>> {
    let lp = ListParams::default().labels(&format!("{LABEL_SANDBOX}=true"));
    match api(client).list(&lp).await {
        Ok(l) => Ok(l.items.iter().map(to_sandbox).collect()),
        Err(kube::Error::Api(ae)) if ae.code == 404 => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}

pub async fn claim(
    client: &kube::Client,
    owner: &str,
    ttl_minutes: u64,
    internet: bool,
    wait: Duration,
) -> Result<Sandbox> {
    let name = new_id();
    let tpl = std::env::var("VEYRON_SANDBOX_TEMPLATE").unwrap_or_else(|_| "ubuntu-24.04".into());
    let mut cfg = crate::templates::TEMPLATES
        .get(&tpl)
        .with_context(|| format!("sandbox template '{tpl}' not found"))?;
    cfg.name = name.clone();
    cfg.namespace = namespace();
    cfg.cpu.cores = 1;
    cfg.cpu.sockets = 1;
    cfg.cpu.threads = 1;
    cfg.memory.size = std::env::var("VEYRON_SANDBOX_MEMORY").unwrap_or_else(|_| "1Gi".into());
    cfg.cloud_init = None;
    let mut m = crate::kairon::machine_from_config(&cfg)?;
    m.spec.runtime = Some(RuntimeSpec {
        backend: Some("firecracker".into()),
        kernel: None,
    });
    let now = chrono::Utc::now();
    let mut annotations = BTreeMap::from([
        (ANN_OWNER.to_string(), owner.to_string()),
        (
            ANN_EXPIRES.to_string(),
            (now + chrono::Duration::minutes(ttl_minutes as i64)).to_rfc3339(),
        ),
        (ANN_INTERNET.to_string(), internet.to_string()),
        (ANN_CREATED.to_string(), now.to_rfc3339()),
    ]);
    if let Some(parent) = fork_parent() {
        annotations.insert(ANNOTATION_FORK_FROM.to_string(), parent);
    }
    m.metadata.namespace = Some(namespace());
    m.metadata.annotations = Some(annotations);
    m.metadata.labels = Some(BTreeMap::from([
        (LABEL_SANDBOX.to_string(), "true".to_string()),
        (LABEL_STATE.to_string(), STATE_CLAIMED.to_string()),
        (LABEL_MANAGED_BY.to_string(), "veyron".to_string()),
    ]));
    api(client)
        .create(&PostParams::default(), &m)
        .await
        .with_context(|| format!("create sandbox Machine {name}"))?;

    let deadline = Instant::now() + wait;
    loop {
        let cur = api(client).get(&name).await?;
        let sb = to_sandbox(&cur);
        if sb.status != "starting" || Instant::now() >= deadline {
            if sb.status == "failed" {
                bail!(
                    "sandbox {name} failed to start: {}",
                    cur.message().unwrap_or("unknown")
                );
            }
            return Ok(sb);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

pub async fn destroy(client: &kube::Client, sb: &Sandbox) -> Result<()> {
    match api(client)
        .delete(&sb.vm, &DeleteParams::background())
        .await
    {
        Ok(_) => Ok(()),
        Err(kube::Error::Api(ae)) if ae.code == 404 => Ok(()),
        Err(e) => Err(e.into()),
    }
}

async fn relay(client: &kube::Client, sb: &Sandbox) -> Result<RelayTarget> {
    let m = api(client).get(&sb.vm).await?;
    RelayTarget::for_machine(client, &m).await
}

pub async fn exec(
    client: &kube::Client,
    sb: &Sandbox,
    command: &str,
    timeout_secs: u64,
) -> Result<ExecResult> {
    let started = Instant::now();
    let r = relay(client, sb)
        .await?
        .exec(&ExecRequest::shell(
            &format!("cd /workspace 2>/dev/null || cd /root; {command}"),
            timeout_secs,
        ))
        .await?;
    Ok(ExecResult {
        exit_code: r.exit_code,
        timed_out: r.exit_code == 124,
        stdout: r.stdout,
        stderr: r.stderr,
        truncated: false,
        duration_ms: started.elapsed().as_millis(),
    })
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub async fn write_file(
    client: &kube::Client,
    sb: &Sandbox,
    path: &str,
    data: &[u8],
) -> Result<()> {
    let target = relay(client, sb).await?;
    let p = quote(path);
    let first = target
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
        let r = target
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

pub async fn read_file(
    client: &kube::Client,
    sb: &Sandbox,
    path: &str,
    max: usize,
) -> Result<Vec<u8>> {
    let target = relay(client, sb).await?;
    let p = quote(path);
    let size = target
        .exec(&ExecRequest::shell(&format!("stat -c %s {p}"), 30))
        .await?;
    if size.exit_code != 0 {
        bail!("cannot read {path}: {}", size.stderr.trim());
    }
    if size.stdout.trim().parse::<usize>().unwrap_or(0) > max {
        bail!("File is larger than {} MiB", max >> 20);
    }
    let r = target
        .exec(&ExecRequest::shell(&format!("base64 -w0 {p}"), 60))
        .await?;
    if r.exit_code != 0 {
        bail!("cannot read {path}: {}", r.stderr.trim());
    }
    Ok(base64::engine::general_purpose::STANDARD.decode(r.stdout.trim())?)
}
