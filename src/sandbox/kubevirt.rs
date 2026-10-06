// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! KubeVirt sandboxes: a warm pool of small Ubuntu VMs, claimed by relabeling.
//!
//! Pool VMs boot with internet so cloud-init can install the guest agent. Claiming
//! one without internet removes its allow policy and adds a Cilium `egressDeny`
//! (deny wins over the cluster-wide virt-launcher allow); the namespace also has a
//! deny-all NetworkPolicy for non-Cilium CNIs. Exec and files use the guest agent,
//! never the network.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use base64::Engine;
use k8s_openapi::api::core::v1::Namespace;
use k8s_openapi::api::networking::v1::NetworkPolicy;
use kube::api::{
    Api, ApiResource, DeleteParams, DynamicObject, GroupVersionKind, ListParams, Patch,
    PatchParams, PostParams,
};
use serde_json::{Value, json};

use super::{
    ANN_CREATED, ANN_EXPIRES, ANN_INTERNET, ANN_OWNER, ExecResult, LABEL_SANDBOX, LABEL_STATE,
    STATE_CLAIMED, STATE_WARM, Sandbox, namespace, new_id,
};
use crate::kube::guestkit_client::{QgaSession, decode_guest_exec_status};

const ANN_ISOLATED: &str = "veyron.io/sandbox-isolated";
const ISOLATION_POLICY: &str = "veyron-sandbox-isolation";
const CHUNK: usize = 48 * 1024;
/// Pool VMs that never get a guest agent (bad image, no egress) are replaced.
const STUCK_AFTER: Duration = Duration::from_secs(20 * 60);

fn ar(kind: &str, plural: &str, group: &str, version: &str) -> ApiResource {
    let mut ar = ApiResource::from_gvk(&GroupVersionKind::gvk(group, version, kind));
    ar.plural = plural.into();
    ar
}

fn vm_api(client: &kube::Client) -> Api<DynamicObject> {
    Api::namespaced_with(
        client.clone(),
        &namespace(),
        &ar("VirtualMachine", "virtualmachines", "kubevirt.io", "v1"),
    )
}

fn vmi_api(client: &kube::Client) -> Api<DynamicObject> {
    Api::namespaced_with(
        client.clone(),
        &namespace(),
        &ar(
            "VirtualMachineInstance",
            "virtualmachineinstances",
            "kubevirt.io",
            "v1",
        ),
    )
}

fn cnp_api(client: &kube::Client) -> Api<DynamicObject> {
    Api::namespaced_with(
        client.clone(),
        &namespace(),
        &ar(
            "CiliumNetworkPolicy",
            "ciliumnetworkpolicies",
            "cilium.io",
            "v2",
        ),
    )
}

fn deny_policy_name(vm: &str) -> String {
    format!("veyron-sbx-deny-{vm}")
}

fn template_name() -> String {
    std::env::var("VEYRON_SANDBOX_TEMPLATE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "ubuntu-24.04".into())
}

fn is_404(e: &kube::Error) -> bool {
    matches!(e, kube::Error::Api(ae) if ae.code == 404)
}

fn ignore_404_409(r: Result<(), kube::Error>) -> Result<(), kube::Error> {
    match r {
        Err(kube::Error::Api(ae)) if ae.code == 404 || ae.code == 409 => Ok(()),
        other => other,
    }
}

pub async fn ensure_namespace(client: &kube::Client) -> Result<()> {
    let ns = namespace();
    let ns_api: Api<Namespace> = Api::all(client.clone());
    if ns_api.get_opt(&ns).await?.is_none() {
        let obj: Namespace = serde_json::from_value(json!({
            "metadata": {"name": ns, "labels": {"veyron.io/sandbox-namespace": "true"}}
        }))?;
        ignore_404_409(
            ns_api
                .create(&PostParams::default(), &obj)
                .await
                .map(|_| ()),
        )
        .context("create sandbox namespace")?;
    }
    // Egress only: with kube-proxy, DNS replies reach the pod from a CIDR identity
    // that an ingress default-deny drops, which breaks DNS even for sandboxes
    // whose per-VM policy allows the internet.
    let np_api: Api<NetworkPolicy> = Api::namespaced(client.clone(), &ns);
    let np: NetworkPolicy = serde_json::from_value(json!({
        "apiVersion": "networking.k8s.io/v1",
        "kind": "NetworkPolicy",
        "metadata": {"name": ISOLATION_POLICY, "labels": {LABEL_SANDBOX: "true"}},
        "spec": {"podSelector": {}, "policyTypes": ["Egress"], "egress": []}
    }))?;
    np_api
        .patch(
            ISOLATION_POLICY,
            &PatchParams::apply("veyron-sandbox").force(),
            &Patch::Apply(&np),
        )
        .await
        .context("apply sandbox isolation NetworkPolicy")?;
    Ok(())
}

pub fn cloud_init(name: &str) -> String {
    format!(
        "#cloud-config\n\
         hostname: {name}\n\
         ssh_pwauth: false\n\
         disable_root: true\n\
         packages:\n  - qemu-guest-agent\n\
         runcmd:\n  - [systemctl, enable, --now, qemu-guest-agent]\n  - [mkdir, -p, /workspace]\n"
    )
}

async fn create_vm(
    client: &kube::Client,
    name: &str,
    state: &str,
    annotations: BTreeMap<String, String>,
) -> Result<()> {
    let tpl = template_name();
    let mut cfg = crate::templates::TEMPLATES
        .get(&tpl)
        .with_context(|| format!("sandbox template '{tpl}' not found"))?;
    cfg.name = name.to_string();
    cfg.namespace = namespace();
    cfg.cpu.cores = 1;
    cfg.cpu.sockets = 1;
    cfg.cpu.threads = 1;
    cfg.memory.size = std::env::var("VEYRON_SANDBOX_MEMORY").unwrap_or_else(|_| "1Gi".into());
    cfg.cloud_init = Some(crate::config::CloudInitConfig {
        user_data: cloud_init(name),
        network_data: None,
        delivery: crate::config::CloudInitDelivery::NoCloud,
    });
    cfg.run_strategy = Some("Always".into());
    cfg.allow_internet = true;
    cfg.expose = None;
    cfg.labels.insert(LABEL_SANDBOX.into(), "true".into());
    cfg.labels.insert(LABEL_STATE.into(), state.into());
    for (k, v) in annotations {
        cfg.annotations.insert(k, v);
    }
    let vm = crate::kube::converter::vm_config_to_kubevirt(&cfg)?;
    let api: Api<crate::kube::types::VirtualMachine> =
        Api::namespaced(client.clone(), &namespace());
    api.create(&PostParams::default(), &vm)
        .await
        .with_context(|| format!("create sandbox VM {name}"))?;
    crate::kube::vm_internet::ensure_vm_internet_egress(client, &namespace(), name).await?;
    Ok(())
}

struct VmiInfo {
    phase: String,
    agent: bool,
}

async fn vmi_index(client: &kube::Client) -> BTreeMap<String, VmiInfo> {
    let mut out = BTreeMap::new();
    let Ok(list) = vmi_api(client).list(&ListParams::default()).await else {
        return out;
    };
    for o in list.items {
        let Some(name) = o.metadata.name.clone() else {
            continue;
        };
        let status = o.data.get("status");
        let phase = status
            .and_then(|s| s.get("phase"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let agent = status
            .and_then(|s| s.get("conditions"))
            .and_then(Value::as_array)
            .is_some_and(|cs| {
                cs.iter().any(|c| {
                    c.get("type").and_then(Value::as_str) == Some("AgentConnected")
                        && c.get("status").and_then(Value::as_str) == Some("True")
                })
            });
        out.insert(name, VmiInfo { phase, agent });
    }
    out
}

fn ann(o: &DynamicObject, k: &str) -> String {
    o.metadata
        .annotations
        .as_ref()
        .and_then(|a| a.get(k))
        .cloned()
        .unwrap_or_default()
}

fn to_sandbox(o: &DynamicObject, vmis: &BTreeMap<String, VmiInfo>) -> Sandbox {
    let name = o.metadata.name.clone().unwrap_or_default();
    let state = o
        .metadata
        .labels
        .as_ref()
        .and_then(|l| l.get(LABEL_STATE))
        .cloned()
        .unwrap_or_default();
    let vmi = vmis.get(&name);
    let agent = vmi.is_some_and(|v| v.agent);
    let failed = vmi.is_some_and(|v| v.phase == "Failed");
    let status = match (state.as_str(), agent, failed) {
        (STATE_WARM, true, _) => "warm",
        (STATE_WARM, false, _) => "warming",
        (_, _, true) => "failed",
        (_, true, _) => "ready",
        _ => "starting",
    };
    let created = {
        let a = ann(o, ANN_CREATED);
        if a.is_empty() {
            o.metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default()
        } else {
            a
        }
    };
    Sandbox {
        id: name.clone(),
        namespace: namespace(),
        owner: ann(o, ANN_OWNER),
        status: status.into(),
        backend: "kubevirt".into(),
        internet: ann(o, ANN_INTERNET) == "true",
        created_at: created,
        expires_at: ann(o, ANN_EXPIRES),
        vm: name,
        isolated: ann(o, ANN_ISOLATED) == "true",
    }
}

pub async fn list(client: &kube::Client) -> Result<Vec<Sandbox>> {
    let lp = ListParams::default().labels(&format!("{LABEL_SANDBOX}=true"));
    let vms = match vm_api(client).list(&lp).await {
        Ok(l) => l.items,
        Err(e) if is_404(&e) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let vmis = vmi_index(client).await;
    Ok(vms.iter().map(|o| to_sandbox(o, &vmis)).collect())
}

fn claim_annotations(owner: &str, ttl_minutes: u64, internet: bool) -> BTreeMap<String, String> {
    let now = chrono::Utc::now();
    BTreeMap::from([
        (ANN_OWNER.to_string(), owner.to_string()),
        (
            ANN_EXPIRES.to_string(),
            (now + chrono::Duration::minutes(ttl_minutes as i64)).to_rfc3339(),
        ),
        (ANN_INTERNET.to_string(), internet.to_string()),
        (ANN_CREATED.to_string(), now.to_rfc3339()),
    ])
}

/// Relabel a warm VM as claimed; the resourceVersion guard makes concurrent claims safe.
async fn try_claim(
    client: &kube::Client,
    name: &str,
    annotations: &BTreeMap<String, String>,
) -> Result<bool> {
    let api = vm_api(client);
    let Some(cur) = api.get_opt(name).await? else {
        return Ok(false);
    };
    let still_warm = cur
        .metadata
        .labels
        .as_ref()
        .and_then(|l| l.get(LABEL_STATE))
        .is_some_and(|s| s == STATE_WARM);
    if !still_warm {
        return Ok(false);
    }
    let patch = json!({
        "metadata": {
            "resourceVersion": cur.metadata.resource_version,
            "labels": {LABEL_STATE: STATE_CLAIMED},
            "annotations": annotations,
        }
    });
    match api
        .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
    {
        Ok(_) => Ok(true),
        Err(kube::Error::Api(ae)) if ae.code == 409 => Ok(false),
        Err(e) => Err(e.into()),
    }
}

async fn get(client: &kube::Client, name: &str) -> Result<Option<Sandbox>> {
    let Some(o) = vm_api(client).get_opt(name).await? else {
        return Ok(None);
    };
    let vmis = vmi_index(client).await;
    Ok(Some(to_sandbox(&o, &vmis)))
}

pub async fn claim(
    client: &kube::Client,
    owner: &str,
    ttl_minutes: u64,
    internet: bool,
    wait: Duration,
) -> Result<Sandbox> {
    ensure_namespace(client).await?;
    let annotations = claim_annotations(owner, ttl_minutes, internet);
    let all = list(client).await?;
    let mut claimed = None;
    for w in all.iter().filter(|s| s.status == "warm") {
        if try_claim(client, &w.id, &annotations).await? {
            claimed = Some(w.id.clone());
            break;
        }
    }
    let name = match claimed {
        Some(n) => n,
        None => {
            let n = new_id();
            create_vm(client, &n, STATE_CLAIMED, annotations).await?;
            n
        }
    };

    let deadline = Instant::now() + wait;
    loop {
        let sb = get(client, &name)
            .await?
            .with_context(|| format!("sandbox {name} disappeared"))?;
        if sb.status == "ready" {
            if !internet {
                set_internet(client, &name, false).await?;
            }
            return get(client, &name).await?.context("sandbox disappeared");
        }
        if sb.status == "failed" {
            bail!("sandbox {name} failed to boot");
        }
        if Instant::now() >= deadline {
            return Ok(sb);
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

/// Allow or block guest egress for one sandbox VM.
pub async fn set_internet(client: &kube::Client, vm: &str, on: bool) -> Result<()> {
    let ns = namespace();
    let deny = deny_policy_name(vm);
    if on {
        crate::kube::vm_internet::ensure_vm_internet_egress(client, &ns, vm).await?;
        let _ = ignore_404_409(
            cnp_api(client)
                .delete(&deny, &DeleteParams::default())
                .await
                .map(|_| ()),
        );
    } else {
        crate::kube::vm_internet::remove_vm_internet_egress(client, &ns, vm).await?;
        let body = json!({
            "apiVersion": "cilium.io/v2",
            "kind": "CiliumNetworkPolicy",
            "metadata": {"name": deny, "namespace": ns, "labels": {LABEL_SANDBOX: "true"}},
            "spec": {
                "endpointSelector": {"matchLabels": {"kubevirt.io/vm": vm}},
                "egressDeny": [{"toEntities": ["world", "cluster"]}]
            }
        });
        let obj: DynamicObject = serde_json::from_value(body)?;
        if let Err(e) = ignore_404_409(
            cnp_api(client)
                .create(&PostParams::default(), &obj)
                .await
                .map(|_| ()),
        ) {
            log::debug!(
                "sandbox {vm}: no Cilium deny policy ({e}); relying on NetworkPolicy isolation"
            );
        }
    }
    let patch = json!({"metadata": {"annotations": {ANN_ISOLATED: (!on).to_string(), ANN_INTERNET: on.to_string()}}});
    vm_api(client)
        .patch(vm, &PatchParams::default(), &Patch::Merge(&patch))
        .await?;
    Ok(())
}

pub async fn destroy(client: &kube::Client, sb: &Sandbox) -> Result<()> {
    match vm_api(client)
        .delete(&sb.vm, &DeleteParams::background())
        .await
    {
        Ok(_) => {}
        Err(e) if is_404(&e) => {}
        Err(e) => return Err(e.into()),
    }
    let _ = crate::kube::vm_internet::remove_vm_internet_egress(client, &namespace(), &sb.vm).await;
    let _ = cnp_api(client)
        .delete(&deny_policy_name(&sb.vm), &DeleteParams::default())
        .await;
    Ok(())
}

async fn ensure_isolated(client: &kube::Client, sb: &Sandbox) -> Result<()> {
    if !sb.internet && !sb.isolated {
        set_internet(client, &sb.vm, false).await?;
    }
    Ok(())
}

async fn run(
    s: &QgaSession,
    command: &str,
    timeout_secs: u64,
    input: Option<&[u8]>,
) -> Result<ExecResult> {
    let started = Instant::now();
    let mut args = json!({
        "path": "/usr/bin/timeout",
        "arg": ["-k", "5", format!("{timeout_secs}s"), "/bin/bash", "-lc",
                format!("cd /workspace 2>/dev/null || cd /root; {command}")],
        "env": ["HOME=/root", "LANG=C.UTF-8",
                "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],
        "capture-output": true
    });
    if let Some(data) = input {
        args["input-data"] = json!(base64::engine::general_purpose::STANDARD.encode(data));
    }
    let r = s
        .execute(&json!({"execute": "guest-exec", "arguments": args}), 30)
        .await?;
    let pid = r
        .get("pid")
        .and_then(Value::as_i64)
        .context("guest agent returned no pid")?;
    let deadline = started + Duration::from_secs(timeout_secs + 20);
    let mut wait = Duration::from_millis(150);
    loop {
        let st = s
            .execute(
                &json!({"execute": "guest-exec-status", "arguments": {"pid": pid}}),
                30,
            )
            .await?;
        let d = decode_guest_exec_status(&st);
        if d.exited {
            let code = d.exit_code.unwrap_or(-1);
            return Ok(ExecResult {
                exit_code: code,
                timed_out: code == 124
                    || (code == 137 && started.elapsed().as_secs() >= timeout_secs),
                stdout: d.stdout,
                stderr: d.stderr,
                truncated: d.truncated,
                duration_ms: started.elapsed().as_millis(),
            });
        }
        if Instant::now() >= deadline {
            return Ok(ExecResult {
                exit_code: -1,
                timed_out: true,
                stderr: format!("Command did not finish within {timeout_secs}s"),
                duration_ms: started.elapsed().as_millis(),
                ..Default::default()
            });
        }
        tokio::time::sleep(wait).await;
        wait = (wait * 2).min(Duration::from_secs(1));
    }
}

pub async fn exec(
    client: &kube::Client,
    sb: &Sandbox,
    command: &str,
    timeout_secs: u64,
) -> Result<ExecResult> {
    ensure_isolated(client, sb).await?;
    let s = QgaSession::open(client.clone(), &sb.namespace, &sb.vm).await?;
    run(&s, command, timeout_secs, None).await
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub async fn write_file(
    client: &kube::Client,
    sb: &Sandbox,
    path: &str,
    data: &[u8],
) -> Result<()> {
    ensure_isolated(client, sb).await?;
    let s = QgaSession::open(client.clone(), &sb.namespace, &sb.vm).await?;
    if let Some(dir) = std::path::Path::new(path).parent().and_then(|p| p.to_str()) {
        if !dir.is_empty() && dir != "/" {
            run(&s, &format!("mkdir -p {}", shell_quote(dir)), 30, None).await?;
        }
    }
    let handle = s
        .execute(
            &json!({"execute": "guest-file-open", "arguments": {"path": path, "mode": "w"}}),
            30,
        )
        .await?;
    let handle = handle
        .as_i64()
        .context("guest-file-open returned no handle")?;
    let result: Result<()> = async {
        for chunk in data.chunks(CHUNK) {
            let b64 = base64::engine::general_purpose::STANDARD.encode(chunk);
            s.execute(
                &json!({"execute": "guest-file-write", "arguments": {"handle": handle, "buf-b64": b64}}),
                30,
            )
            .await?;
        }
        Ok(())
    }
    .await;
    let _ = s
        .execute(
            &json!({"execute": "guest-file-close", "arguments": {"handle": handle}}),
            30,
        )
        .await;
    result
}

pub async fn read_file(
    client: &kube::Client,
    sb: &Sandbox,
    path: &str,
    max: usize,
) -> Result<Vec<u8>> {
    let s = QgaSession::open(client.clone(), &sb.namespace, &sb.vm).await?;
    let handle = s
        .execute(
            &json!({"execute": "guest-file-open", "arguments": {"path": path, "mode": "r"}}),
            30,
        )
        .await?;
    let handle = handle
        .as_i64()
        .context("guest-file-open returned no handle")?;
    let result: Result<Vec<u8>> = async {
        let mut out = Vec::new();
        loop {
            let r = s
                .execute(
                    &json!({"execute": "guest-file-read", "arguments": {"handle": handle, "count": CHUNK}}),
                    30,
                )
                .await?;
            if let Some(b) = r.get("buf-b64").and_then(Value::as_str) {
                out.extend(base64::engine::general_purpose::STANDARD.decode(b)?);
            }
            if out.len() > max {
                bail!("File is larger than {} MiB", max >> 20);
            }
            let eof = r.get("eof").and_then(Value::as_bool).unwrap_or(true);
            let count = r.get("count").and_then(Value::as_u64).unwrap_or(0);
            if eof || count == 0 {
                return Ok(out);
            }
        }
    }
    .await;
    let _ = s
        .execute(
            &json!({"execute": "guest-file-close", "arguments": {"handle": handle}}),
            30,
        )
        .await;
    result
}

fn age(sb: &Sandbox) -> Duration {
    chrono::DateTime::parse_from_rfc3339(&sb.created_at)
        .ok()
        .and_then(|t| {
            (chrono::Utc::now() - t.with_timezone(&chrono::Utc))
                .to_std()
                .ok()
        })
        .unwrap_or_default()
}

/// Keep `target` warm VMs, replace stuck ones, and finish isolating claimed sandboxes.
pub async fn reconcile_pool(
    client: &kube::Client,
    all: &[Sandbox],
    warm: usize,
    target: usize,
) -> Result<()> {
    if target == 0 && all.is_empty() {
        return Ok(());
    }
    for sb in all.iter().filter(|s| s.status == "ready") {
        if let Err(e) = ensure_isolated(client, sb).await {
            log::warn!("isolating sandbox {}: {e}", sb.id);
        }
    }
    let mut warm = warm;
    for sb in all
        .iter()
        .filter(|s| s.status == "warming" && age(s) > STUCK_AFTER)
    {
        log::warn!("sandbox pool VM {} never became ready; replacing it", sb.id);
        destroy(client, sb).await?;
        warm = warm.saturating_sub(1);
    }
    if warm > target {
        let mut extra: Vec<&Sandbox> = all.iter().filter(|s| s.is_pool()).collect();
        extra.sort_by_key(|s| s.status == "warm");
        for sb in extra.into_iter().take(warm - target) {
            destroy(client, sb).await?;
        }
    } else if warm < target {
        ensure_namespace(client).await?;
        for _ in 0..(target - warm).min(2) {
            let id = new_id();
            create_vm(
                client,
                &id,
                STATE_WARM,
                BTreeMap::from([(ANN_CREATED.to_string(), chrono::Utc::now().to_rfc3339())]),
            )
            .await?;
            log::info!("sandbox pool: started {id}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_init_installs_guest_agent() {
        let ci = cloud_init("sbx-1");
        assert!(ci.starts_with("#cloud-config"));
        assert!(ci.contains("qemu-guest-agent"));
        assert!(ci.contains("hostname: sbx-1"));
    }

    #[test]
    fn quoting_survives_single_quotes() {
        assert_eq!(shell_quote("/tmp/it's"), "'/tmp/it'\\''s'");
    }

    #[test]
    fn status_reflects_pool_state_and_agent() {
        let mk = |state: &str| -> DynamicObject {
            serde_json::from_value(json!({
                "apiVersion": "kubevirt.io/v1", "kind": "VirtualMachine",
                "metadata": {"name": "sbx-a", "labels": {LABEL_STATE: state},
                             "annotations": {ANN_OWNER: "alice", ANN_INTERNET: "false"}}
            }))
            .unwrap()
        };
        let mut vmis = BTreeMap::new();
        assert_eq!(to_sandbox(&mk(STATE_WARM), &vmis).status, "warming");
        assert_eq!(to_sandbox(&mk(STATE_CLAIMED), &vmis).status, "starting");
        vmis.insert(
            "sbx-a".to_string(),
            VmiInfo {
                phase: "Running".into(),
                agent: true,
            },
        );
        assert_eq!(to_sandbox(&mk(STATE_WARM), &vmis).status, "warm");
        let sb = to_sandbox(&mk(STATE_CLAIMED), &vmis);
        assert_eq!(sb.status, "ready");
        assert_eq!(sb.owner, "alice");
        assert!(!sb.internet);
    }

    #[test]
    fn exec_status_keeps_streams_apart() {
        let st = json!({"exited": true, "exitcode": 2, "out-data": "b2s=", "err-data": "Ym9vbQ=="});
        let d = decode_guest_exec_status(&st);
        assert_eq!(d.exit_code, Some(2));
        assert_eq!(d.stdout, "ok");
        assert_eq!(d.stderr, "boom");
        assert!(d.exited);
        assert!(!decode_guest_exec_status(&json!({"exited": false})).exited);
    }
}
