// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Kairon sandboxes: Firecracker Machines, forked from a running parent when
//! `VEYRON_SANDBOX_KAIRON_PARENT` (a Machine name in the sandbox namespace) is set,
//! which starts in well under a second. Exec goes through the kairon-node relay.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use base64::Engine;
use kube::api::{
    Api, ApiResource, DeleteParams, DynamicObject, ListParams, Patch, PatchParams, PostParams,
};
use kube::core::GroupVersionKind;

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
    let idle_suspended = !ann("kairon.zyvor.dev/idle-suspended").is_empty();
    let status = match m.phase() {
        "Running" if m.runtime_id().is_some() => "ready",
        "Failed" => "failed",
        _ if idle_suspended || m.power_state() == crate::kairon::POWER_PAUSED => "paused",
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
        parent: m
            .metadata
            .labels
            .as_ref()
            .and_then(|l| l.get(crate::kairon::LABEL_FORKED_FROM))
            .cloned()
            .unwrap_or_default(),
        idle_suspended,
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

/// `spec.lifecycle` for a new sandbox: enabled when `VEYRON_SANDBOX_IDLE_SECS` is set
/// (idle action from `VEYRON_SANDBOX_IDLE_ACTION`, default `Pause`). Wake-on-connect is
/// always on so a paused sandbox resumes on its next exec; the deadline mirrors the TTL.
pub(super) fn lifecycle_spec(ttl_minutes: u64) -> Option<crate::kairon::LifecycleSpec> {
    let idle = std::env::var("VEYRON_SANDBOX_IDLE_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|n| *n > 0)?;
    let action = std::env::var("VEYRON_SANDBOX_IDLE_ACTION")
        .ok()
        .filter(|a| {
            matches!(
                a.as_str(),
                "Pause" | "Hibernate" | "Halt" | "Stop" | "Delete"
            )
        })
        .unwrap_or_else(|| "Pause".into());
    Some(crate::kairon::LifecycleSpec {
        idle_timeout_seconds: Some(idle.max(60)),
        idle_action: Some(action),
        idle_cpu_millicores: None,
        max_lifetime_seconds: Some(ttl_minutes * 60),
        wake_on_connect: Some(true),
        wake_timeout_seconds: Some(60),
    })
}

/// Annotation recording the `MachineClaim` a pool-backed sandbox came from.
pub const ANN_CLAIM: &str = "veyron.io/sandbox-claim";

/// Kairon `MachinePool` to claim sandboxes from (paused warm members resume in
/// milliseconds). Unset keeps the create-a-Machine path.
fn pool_name() -> Option<String> {
    std::env::var("VEYRON_SANDBOX_KAIRON_POOL")
        .ok()
        .filter(|p| !p.is_empty())
}

fn claim_api(client: &kube::Client) -> Api<DynamicObject> {
    let gvk = GroupVersionKind::gvk("kairon.zyvor.dev", "v1beta1", "MachineClaim");
    Api::namespaced_with(
        client.clone(),
        &namespace(),
        &ApiResource::from_gvk_with_plural(&gvk, "machineclaims"),
    )
}

async fn claim_from_pool(
    client: &kube::Client,
    pool: &str,
    owner: &str,
    ttl_minutes: u64,
    internet: bool,
    wait: Duration,
) -> Result<Sandbox> {
    let claim_name = new_id();
    let obj: DynamicObject = serde_json::from_value(serde_json::json!({
        "apiVersion": "kairon.zyvor.dev/v1beta1",
        "kind": "MachineClaim",
        "metadata": {"name": claim_name, "namespace": namespace(),
                     "labels": {LABEL_MANAGED_BY: "veyron"}},
        "spec": {"poolName": pool, "ttlSeconds": ttl_minutes * 60},
    }))?;
    let claims = claim_api(client);
    claims
        .create(&PostParams::default(), &obj)
        .await
        .with_context(|| format!("create MachineClaim {claim_name} on pool {pool}"))?;

    let deadline = Instant::now() + wait.max(Duration::from_secs(5));
    let machine_name = loop {
        let cur = claims.get(&claim_name).await?;
        let st = cur.data.get("status");
        if let Some(m) = st
            .and_then(|s| s.get("machineName"))
            .and_then(|v| v.as_str())
            .filter(|m| !m.is_empty())
        {
            break m.to_string();
        }
        if Instant::now() >= deadline {
            let _ = claims
                .delete(&claim_name, &DeleteParams::background())
                .await;
            bail!("pool {pool} did not bind a Machine to claim {claim_name} in time");
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    };

    let now = chrono::Utc::now();
    let patch = serde_json::json!({"metadata": {
        "labels": {LABEL_SANDBOX: "true", LABEL_STATE: STATE_CLAIMED, LABEL_MANAGED_BY: "veyron"},
        "annotations": {
            ANN_OWNER: owner,
            ANN_EXPIRES: (now + chrono::Duration::minutes(ttl_minutes as i64)).to_rfc3339(),
            ANN_INTERNET: internet.to_string(),
            ANN_CREATED: now.to_rfc3339(),
            ANN_CLAIM: claim_name,
        }
    }});
    api(client)
        .patch(
            &machine_name,
            &PatchParams::default(),
            &Patch::Merge(&patch),
        )
        .await
        .with_context(|| format!("label claimed Machine {machine_name} as a sandbox"))?;
    Ok(to_sandbox(&api(client).get(&machine_name).await?))
}

pub async fn claim(
    client: &kube::Client,
    owner: &str,
    ttl_minutes: u64,
    internet: bool,
    wait: Duration,
) -> Result<Sandbox> {
    if let Some(pool) = pool_name() {
        return claim_from_pool(client, &pool, owner, ttl_minutes, internet, wait).await;
    }
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
    m.spec.lifecycle = lifecycle_spec(ttl_minutes);
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
    // A pool-claimed sandbox is released through its claim so the pool refills.
    if let Ok(Some(m)) = api(client).get_opt(&sb.vm).await {
        if let Some(claim) = m
            .metadata
            .annotations
            .as_ref()
            .and_then(|a| a.get(ANN_CLAIM))
        {
            let _ = claim_api(client)
                .delete(claim, &DeleteParams::background())
                .await;
        }
    }
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
