// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Natural-language guardrails: a sentence becomes a VeyronPolicy whose CEL
//! rules use the operator's variables, dry-run against the VMs that exist now.
//! Saving goes through a proposal. Policies audit and warn; they never block.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use axum::{Extension, Json, extract::State, response::IntoResponse, response::Response};
use k8s_openapi::api::core::v1::{ConfigMap, Service};
use k8s_openapi::api::networking::v1::NetworkPolicy;
use kube::Api;
use kube::api::{ApiResource, DynamicObject, ListParams};
use kube::core::GroupVersionKind;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};

use super::api::err;
use super::cel::{Env, Program, Val};
use super::llm::OpenAiBackend;
use super::registry::{self, safe_name};
use crate::api::auth_context::{AuthContext, role_rank};
use crate::api::http_server::web::{ApiRole, SharedState};
use crate::operator_crds::PolicyLabelSelector;

/// CEL variables, matching `evaluateCEL` in operator/internal/controller/veyronpolicy_controller.go.
pub const VARIABLES: &[(&str, &str)] = &[
    ("cpu_cores", "uint: vCPU cores per socket"),
    ("cpu_sockets", "uint"),
    ("cpu_threads", "uint"),
    ("memory_size", "string, e.g. \"8Gi\""),
    ("memory_gib", "double: memory in GiB"),
    ("enable_tpm", "bool"),
    ("enable_rng", "bool"),
    ("has_cloud_init", "bool"),
    (
        "has_eviction_strategy",
        "bool: can live-migrate on node drain",
    ),
    ("has_firmware", "bool"),
    ("has_features", "bool"),
    ("num_disks", "uint"),
    ("num_interfaces", "uint"),
    ("template", "string: OS template name"),
    ("name", "string"),
    ("namespace", "string"),
    ("running", "bool"),
    ("rdp_exposed", "bool: a Service exposes RDP (3389)"),
    (
        "internet_egress",
        "bool: a per-VM internet egress policy exists",
    ),
    ("gpu_count", "int"),
    (
        "has_snapshot_schedule",
        "bool: an enabled snapshot schedule exists",
    ),
    ("labels", "map(string, string): VM labels"),
];

#[derive(Default)]
pub struct Facts {
    rdp: BTreeSet<(String, String)>,
    egress: BTreeSet<(String, String)>,
    schedules: BTreeSet<(String, String)>,
}

pub async fn facts(client: &kube::Client) -> Facts {
    let mut f = Facts::default();
    if let Ok(l) = Api::<Service>::all(client.clone())
        .list(&ListParams::default())
        .await
    {
        for s in l.items {
            let ns = s.metadata.namespace.clone().unwrap_or_default();
            let Some(spec) = s.spec else { continue };
            let sel = spec.selector.unwrap_or_default();
            let vm = [
                "kubevirt.io/vm",
                "vm.kubevirt.io/name",
                "kubevirt.io/domain",
            ]
            .iter()
            .find_map(|k| sel.get(*k).cloned());
            let rdp = spec.ports.unwrap_or_default().iter().any(|p| {
                p.port == 3389
                    || matches!(
                        &p.target_port,
                        Some(k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(3389))
                    )
            });
            if let (Some(vm), true) = (vm, rdp) {
                f.rdp.insert((ns, vm));
            }
        }
    }
    let mut egress_names: Vec<(String, String)> = Vec::new();
    if let Ok(l) = Api::<NetworkPolicy>::all(client.clone())
        .list(&ListParams::default())
        .await
    {
        egress_names.extend(l.items.into_iter().map(|n| {
            (
                n.metadata.namespace.unwrap_or_default(),
                n.metadata.name.unwrap_or_default(),
            )
        }));
    }
    let gvk = GroupVersionKind::gvk("cilium.io", "v2", "CiliumNetworkPolicy");
    let mut ar = ApiResource::from_gvk(&gvk);
    ar.plural = "ciliumnetworkpolicies".into();
    if let Ok(l) = Api::<DynamicObject>::all_with(client.clone(), &ar)
        .list(&ListParams::default())
        .await
    {
        egress_names.extend(l.items.into_iter().map(|n| {
            (
                n.metadata.namespace.unwrap_or_default(),
                n.metadata.name.unwrap_or_default(),
            )
        }));
    }
    for (ns, name) in egress_names {
        if let Some(vm) = name.strip_prefix("veyron-net-") {
            f.egress.insert((ns, vm.to_string()));
        }
    }
    let lp = ListParams::default().labels("veyron.io/type=snapshot-schedule");
    if let Ok(l) = Api::<ConfigMap>::all(client.clone()).list(&lp).await {
        for cm in l.items {
            let ns = cm.metadata.namespace.clone().unwrap_or_default();
            let rec: Value = cm
                .data
                .as_ref()
                .and_then(|d| d.get("schedule.json"))
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            if rec["enabled"].as_bool() == Some(true) {
                if let Some(vm) = rec["vm_name"].as_str() {
                    f.schedules.insert((ns, vm.to_string()));
                }
            }
        }
    }
    f
}

/// CEL variables for one KubeVirt VM (the API's view of what the operator sees).
pub fn vm_env(vm: &Value, f: &Facts) -> Env {
    let name = vm
        .pointer("/metadata/name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let ns = vm
        .pointer("/metadata/namespace")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let spec = vm
        .pointer("/spec/template/spec")
        .cloned()
        .unwrap_or_default();
    let dom = &spec["domain"];
    let devs = &dom["devices"];
    let n = |v: &Value, d: f64| Val::Num(v.as_f64().unwrap_or(d));
    let mem = dom
        .pointer("/memory/guest")
        .or_else(|| dom.pointer("/resources/requests/memory"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let count = |v: &Value| Val::Num(v.as_array().map(|a| a.len()).unwrap_or(0) as f64);
    let labels: BTreeMap<String, Val> = vm
        .pointer("/metadata/labels")
        .and_then(|l| l.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), Val::Str(s.to_string()))))
                .collect()
        })
        .unwrap_or_default();
    let template = [
        "veyron.io/template",
        "vm.kubevirt.io/template",
        "veyron.io/os",
    ]
    .iter()
    .find_map(|k| {
        vm.pointer("/metadata/labels")
            .and_then(|l| l.get(*k))
            .and_then(|v| v.as_str())
    })
    .or_else(|| {
        vm.pointer("/metadata/annotations")
            .and_then(|a| a.get("veyron.io/template"))
            .and_then(|v| v.as_str())
    })
    .unwrap_or_default()
    .to_string();
    let running = vm
        .pointer("/status/printableStatus")
        .and_then(|v| v.as_str())
        == Some("Running")
        || vm.pointer("/spec/running").and_then(|v| v.as_bool()) == Some(true)
        || vm.pointer("/spec/runStrategy").and_then(|v| v.as_str()) == Some("Always");
    let has_ci = spec["volumes"].as_array().is_some_and(|v| {
        v.iter()
            .any(|x| x.get("cloudInitNoCloud").is_some() || x.get("cloudInitConfigDrive").is_some())
    });
    let key = (ns.clone(), name.clone());
    Env::from([
        ("cpu_cores".into(), n(&dom["cpu"]["cores"], 1.0)),
        ("cpu_sockets".into(), n(&dom["cpu"]["sockets"], 1.0)),
        ("cpu_threads".into(), n(&dom["cpu"]["threads"], 1.0)),
        (
            "memory_gib".into(),
            Val::Num(if mem.is_empty() {
                0.0
            } else {
                crate::utils::parse_memory_gib(&mem)
            }),
        ),
        ("memory_size".into(), Val::Str(mem)),
        ("enable_tpm".into(), Val::Bool(devs.get("tpm").is_some())),
        ("enable_rng".into(), Val::Bool(devs.get("rng").is_some())),
        ("has_cloud_init".into(), Val::Bool(has_ci)),
        (
            "has_eviction_strategy".into(),
            Val::Bool(spec.get("evictionStrategy").is_some()),
        ),
        (
            "has_firmware".into(),
            Val::Bool(dom.get("firmware").is_some()),
        ),
        (
            "has_features".into(),
            Val::Bool(dom.get("features").is_some()),
        ),
        ("num_disks".into(), count(&devs["disks"])),
        ("num_interfaces".into(), count(&devs["interfaces"])),
        ("template".into(), Val::Str(template)),
        ("name".into(), Val::Str(name)),
        ("namespace".into(), Val::Str(ns)),
        ("running".into(), Val::Bool(running)),
        ("rdp_exposed".into(), Val::Bool(f.rdp.contains(&key))),
        ("internet_egress".into(), Val::Bool(f.egress.contains(&key))),
        ("gpu_count".into(), count(&devs["gpus"])),
        (
            "has_snapshot_schedule".into(),
            Val::Bool(f.schedules.contains(&key)),
        ),
        ("labels".into(), Val::Map(labels)),
    ])
}

/// Check rules compile and only use known variables. Returns (good rules, errors).
pub fn check_rules(policy: &Value) -> (Vec<(String, Program, String)>, Vec<String>) {
    let known: BTreeSet<&str> = VARIABLES.iter().map(|(n, _)| *n).collect();
    let mut ok = Vec::new();
    let mut errors = Vec::new();
    for r in policy["rules"].as_array().cloned().unwrap_or_default() {
        let name = r["name"].as_str().unwrap_or("rule").to_string();
        let cond = r["condition"].as_str().unwrap_or_default();
        match Program::compile(cond) {
            Ok(p) => {
                let unknown: Vec<String> = p
                    .variables()
                    .into_iter()
                    .filter(|v| !known.contains(v.as_str()))
                    .collect();
                if unknown.is_empty() {
                    ok.push((
                        name,
                        p,
                        r["message"].as_str().unwrap_or_default().to_string(),
                    ));
                } else {
                    errors.push(format!(
                        "Rule {name}: unknown variable {}",
                        unknown.join(", ")
                    ));
                }
            }
            Err(e) => errors.push(format!("Rule {name}: {e}")),
        }
    }
    (ok, errors)
}

/// Evaluate the policy against every VM; an evaluation error counts as a violation, like the operator.
pub async fn dry_run(client: &kube::Client, policy: &Value) -> Value {
    let (rules, mut errors) = check_rules(policy);
    let selector: Option<PolicyLabelSelector> = policy
        .get("selector")
        .cloned()
        .and_then(|s| serde_json::from_value(s).ok());
    let vms = Api::<crate::kube::types::VirtualMachine>::all(client.clone())
        .list(&ListParams::default())
        .await
        .map(|l| l.items)
        .unwrap_or_default();
    let f = facts(client).await;
    let sandbox_ns = crate::sandbox::namespace();
    let mut checked = 0;
    let mut violations = Vec::new();
    let mut eval_errors = BTreeSet::new();
    for vm in vms {
        if vm.metadata.namespace.as_deref() == Some(sandbox_ns.as_str()) {
            continue;
        }
        let labels = vm.metadata.labels.clone().unwrap_or_default();
        if selector.as_ref().is_some_and(|s| !s.matches(&labels)) {
            continue;
        }
        checked += 1;
        let v = serde_json::to_value(&vm).unwrap_or_default();
        let env = vm_env(&v, &f);
        let id = format!(
            "{}/{}",
            vm.metadata.namespace.clone().unwrap_or_default(),
            vm.metadata.name.clone().unwrap_or_default()
        );
        for (name, prog, msg) in &rules {
            match prog.eval_bool(&env) {
                Ok(true) => {}
                Ok(false) => violations.push(json!({"vm": id, "rule": name, "message": msg})),
                Err(e) => {
                    eval_errors.insert(format!("Rule {name}: {e}"));
                    violations.push(json!({"vm": id, "rule": name, "message": format!("{msg} (could not evaluate: {e})")}));
                }
            }
        }
    }
    errors.extend(eval_errors.into_iter().take(5));
    json!({"violations": violations, "checked": checked, "errors": errors})
}

// ── Drafting ─────────────────────────────────────────────────────

static RE_NUM_CPU: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(at most|no more than|max(?:imum)?|up to|at least|min(?:imum)?|more than|less than|fewer than)\s+(\d+)\s*(?:v?cpus?|cores?)").unwrap()
});
static RE_NUM_MEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(at most|no more than|max(?:imum)?|up to|at least|min(?:imum)?|more than|less than)\s+(\d+)\s*(?:gb|gib|g)\b").unwrap()
});
static RE_NUM_GPU: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(at most|no more than|max(?:imum)?|up to)\s+(\d+)\s*gpus?").unwrap()
});
static RE_LABEL_SEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:labell?ed|with label|label)\s+([a-z0-9./_-]+)\s*[=:]\s*([a-z0-9._-]+)").unwrap()
});
static RE_REQ_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:must|should|need to|require[sd]?)\s+(?:have|carry)\s+(?:an?\s+)?([a-z0-9./_-]+)\s+label").unwrap()
});

fn bound_op(word: &str) -> &'static str {
    match word {
        "at least" | "min" | "minimum" => ">=",
        "more than" => ">",
        "less than" | "fewer than" => "<",
        _ => "<=",
    }
}

fn rule(name: &str, condition: String, message: String) -> Value {
    json!({"name": name, "condition": condition, "message": message})
}

/// Turn common guardrail phrases into rules without a language model.
pub fn draft_rules(text: &str) -> Value {
    let t = text.to_lowercase();
    let neg = |w: &str| {
        t.split(['.', ';', ',']).any(|clause| {
            clause.contains(w)
                && [
                    "no ", "not ", "never", "without", "block", "disallow", "forbid", "public",
                ]
                .iter()
                .any(|n| clause.contains(n))
        })
    };
    let mut rules = Vec::new();
    if t.contains("snapshot") {
        rules.push(rule(
            "snapshot-schedule",
            "has_snapshot_schedule".into(),
            "VM has no enabled snapshot schedule".into(),
        ));
    }
    if t.contains("rdp") && neg("rdp") {
        rules.push(rule(
            "no-rdp-exposure",
            "!rdp_exposed".into(),
            "RDP (3389) is exposed through a Service".into(),
        ));
    }
    if (t.contains("internet") || t.contains("egress")) && (neg("internet") || neg("egress")) {
        rules.push(rule(
            "no-internet-egress",
            "!internet_egress".into(),
            "VM has an internet egress policy".into(),
        ));
    }
    if t.contains("tpm") {
        rules.push(rule(
            "tpm",
            "enable_tpm".into(),
            "TPM is not enabled".into(),
        ));
    }
    if t.contains("cloud-init") || t.contains("cloud init") {
        rules.push(rule(
            "cloud-init",
            "has_cloud_init".into(),
            "VM has no cloud-init".into(),
        ));
    }
    if t.contains("evict") || t.contains("live migrat") || t.contains("migratable") {
        rules.push(rule(
            "eviction-strategy",
            "has_eviction_strategy".into(),
            "VM cannot live-migrate on node drain".into(),
        ));
    }
    if let Some(c) = RE_NUM_CPU.captures(&t) {
        let op = bound_op(&c[1]);
        rules.push(rule(
            "cpu-limit",
            format!("cpu_cores {op} {}", &c[2]),
            format!("CPU cores must be {op} {}", &c[2]),
        ));
    }
    if let Some(c) = RE_NUM_MEM.captures(&t) {
        let op = bound_op(&c[1]);
        rules.push(rule(
            "memory-limit",
            format!("memory_gib {op} {}.0", &c[2]),
            format!("Memory must be {op} {} GiB", &c[2]),
        ));
    }
    if let Some(c) = RE_NUM_GPU.captures(&t) {
        rules.push(rule(
            "gpu-limit",
            format!("gpu_count <= {}", &c[2]),
            format!("At most {} GPUs per VM", &c[2]),
        ));
    } else if t.contains("gpu") && neg("gpu") {
        rules.push(rule(
            "no-gpu",
            "gpu_count == 0".into(),
            "GPUs are not allowed".into(),
        ));
    }
    if let Some(c) = RE_REQ_LABEL.captures(&t) {
        let k = &c[1];
        rules.push(rule(
            &format!("label-{}", k.replace(['/', '.', '_'], "-")),
            format!("'{k}' in labels"),
            format!("VM is missing the {k} label"),
        ));
    }

    let mut policy = json!({
        "description": text.trim(),
        "enabled": true,
        "enforcementAction": if t.contains("warn") { "Warn" } else { "Audit" },
        "severity": if t.contains("critical") || t.contains("never") { "High" } else { "Medium" },
        "rules": rules,
    });
    if let Some(c) = RE_LABEL_SEL.captures(&t) {
        policy["selector"] = json!({"matchLabels": {&c[1]: &c[2]}});
    }
    policy
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
        if out.len() >= 40 {
            break;
        }
    }
    out.trim_matches('-').to_string()
}

/// Clamp what the model or user sent to the fields the CRD accepts.
pub fn normalize(mut p: Value, default_ns: &str) -> Value {
    let rules: Vec<Value> = p["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|r| {
            let cond = r["condition"].as_str()?.trim().to_string();
            (!cond.is_empty()).then(|| {
                let name = slug(r["name"].as_str().unwrap_or("rule"));
                json!({
                    "name": if name.is_empty() { "rule".to_string() } else { name },
                    "condition": cond,
                    "message": r["message"].as_str().unwrap_or("Policy rule violated"),
                })
            })
        })
        .collect();
    let action = match p["enforcementAction"].as_str() {
        Some("Warn" | "Deny") => "Warn",
        _ => "Audit",
    };
    let sev = match p["severity"].as_str().map(|s| s.to_lowercase()).as_deref() {
        Some("critical") => "Critical",
        Some("high") => "High",
        Some("low") => "Low",
        Some("info") => "Info",
        _ => "Medium",
    };
    let desc = p["description"].as_str().unwrap_or_default().to_string();
    let name = p["name"]
        .as_str()
        .map(slug)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let s = slug(&desc);
            if s.is_empty() {
                "ai-policy".into()
            } else {
                format!(
                    "ai-{}",
                    s.chars().take(30).collect::<String>().trim_end_matches('-')
                )
            }
        });
    let ns = p["namespace"]
        .as_str()
        .and_then(|n| safe_name(n).ok())
        .unwrap_or_else(|| default_ns.to_string());
    let mut out = json!({
        "name": name,
        "namespace": ns,
        "description": desc,
        "enabled": p["enabled"].as_bool().unwrap_or(true),
        "enforcementAction": action,
        "severity": sev,
        "rules": rules,
    });
    if let Some(sel) = p
        .get_mut("selector")
        .filter(|s| s.is_object())
        .map(|s| s.take())
    {
        if let Ok(s) = serde_json::from_value::<PolicyLabelSelector>(sel) {
            if s != PolicyLabelSelector::default() {
                out["selector"] = serde_json::to_value(s).unwrap_or_default();
            }
        }
    }
    if let Some(fw) = p["framework"].as_str() {
        out["framework"] = json!(fw);
    }
    out
}

async fn draft_llm(backend: &OpenAiBackend, text: &str) -> anyhow::Result<(Value, String)> {
    let vars: Vec<String> = VARIABLES
        .iter()
        .map(|(n, d)| format!("{n} ({d})"))
        .collect();
    let system = format!(
        "You write VeyronPolicy guardrails for KubeVirt VMs. Each rule is a CEL expression that is TRUE when \
         the VM is compliant. Use only these variables:\n{}\n\
         Numbers compare across int/uint/double. A missing map key is an error, so guard label lookups: \
         `'team' in labels && labels['team'] != ''`. Scope with a Kubernetes label selector when the sentence names \
         a group of VMs (e.g. production → {{\"matchLabels\": {{\"env\": \"production\"}}}}).\n\
         Return {{\"name\": dns-label, \"description\": text, \"severity\": Critical|High|Medium|Low, \
         \"enforcementAction\": Audit|Warn, \"selector\": {{..}} or null, \
         \"rules\": [{{\"name\": dns-label, \"condition\": CEL, \"message\": shown on violation}}], \
         \"explanation\": one or two sentences}}.",
        vars.join("\n")
    );
    let v = backend.complete_json(&system, text).await?;
    let explanation = v["explanation"].as_str().unwrap_or_default().to_string();
    Ok((v, explanation))
}

#[derive(Deserialize, Default)]
pub struct DraftReq {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub namespace: Option<String>,
}

async fn policy_ns(state: &SharedState, requested: Option<&str>) -> String {
    match requested.filter(|n| !n.is_empty() && *n != "all") {
        Some(n) => n.to_string(),
        None => state.read().await.namespace.clone(),
    }
}

fn scope_note(ns: &str) -> String {
    format!(
        "The dry run checks every VM the API can see. Once saved in {ns}, the operator re-checks VeyronVM resources in that namespace every two minutes and records violations; nothing is blocked."
    )
}

pub async fn draft_handler(
    State(state): State<SharedState>,
    Json(req): Json<DraftReq>,
) -> Response {
    let text = req.description.trim();
    if text.is_empty() {
        return err(400, "Describe the rule");
    }
    let ns = policy_ns(&state, req.namespace.as_deref()).await;
    let (raw, explanation, source) = match OpenAiBackend::from_env() {
        Some(b) => match draft_llm(&b, text).await {
            Ok((v, e)) => (v, e, "llm"),
            Err(_) => (draft_rules(text), String::new(), "rules"),
        },
        None => (draft_rules(text), String::new(), "rules"),
    };
    let mut policy = normalize(raw, &ns);
    if policy["description"].as_str().is_none_or(str::is_empty) {
        policy["description"] = json!(text);
    }
    if policy["rules"].as_array().is_none_or(|r| r.is_empty()) {
        return err(
            422,
            if source == "rules" {
                "No rule recognized. Without a language model, mention snapshots, RDP, internet egress, TPM, cloud-init, eviction, CPU/memory/GPU limits or a required label."
            } else {
                "The model returned no usable rules; try rephrasing."
            },
        );
    }
    let client = state.read().await.kube_client.client();
    let preview = dry_run(&client, &policy).await;
    let explanation = if explanation.is_empty() {
        format!(
            "{} rule{} drafted from your sentence. {}",
            policy["rules"].as_array().map(|r| r.len()).unwrap_or(0),
            if policy["rules"].as_array().map(|r| r.len()).unwrap_or(0) == 1 {
                ""
            } else {
                "s"
            },
            scope_note(&ns)
        )
    } else {
        format!("{explanation} {}", scope_note(&ns))
    };
    Json(json!({"success": true, "data": {"policy": policy, "preview": preview, "explanation": explanation, "source": source}}))
        .into_response()
}

#[derive(Deserialize, Default)]
pub struct PreviewReq {
    pub policy: Value,
    #[serde(default)]
    pub submit: bool,
}

pub async fn preview_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<PreviewReq>,
) -> Response {
    if !req.policy.is_object() {
        return err(400, "policy object is required");
    }
    let ns = policy_ns(&state, req.policy["namespace"].as_str()).await;
    let policy = normalize(req.policy, &ns);
    let (_, errors) = check_rules(&policy);
    if !req.submit {
        let client = state.read().await.kube_client.client();
        let preview = dry_run(&client, &policy).await;
        return Json(json!({"success": true, "data": {"policy": policy, "preview": preview}}))
            .into_response();
    }
    if role_rank(&auth.role) < role_rank(&ApiRole::Write) {
        return err(403, "Saving a policy needs the write role");
    }
    if !errors.is_empty() {
        return err(422, &errors.join("; "));
    }
    let ctx = super::tool_ctx(&state, &auth, "policy").await;
    let p = match registry::draft_change(&ctx, "create_policy", &json!({"policy": policy})).await {
        Ok(p) => p,
        Err(e) => return err(422, &e.to_string()),
    };
    let out = registry::submit_proposal(&ctx, p).await;
    match out.proposal {
        Some(p) => Json(json!({"success": true, "data": {"proposal": p}})).into_response(),
        None => err(500, &out.content.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_from_common_phrases() {
        let p = draft_rules(
            "Production VMs labeled env=production must have a snapshot schedule and no public RDP, at most 8 cores",
        );
        let conds: Vec<&str> = p["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["condition"].as_str().unwrap())
            .collect();
        assert!(conds.contains(&"has_snapshot_schedule"));
        assert!(conds.contains(&"!rdp_exposed"));
        assert!(conds.contains(&"cpu_cores <= 8"));
        assert_eq!(p["selector"]["matchLabels"]["env"], "production");
    }

    #[test]
    fn required_label_and_no_gpu() {
        let p = draft_rules("every vm must have an owner label; no GPUs");
        let conds: Vec<&str> = p["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["condition"].as_str().unwrap())
            .collect();
        assert!(conds.contains(&"'owner' in labels"));
        assert!(conds.contains(&"gpu_count == 0"));
    }

    #[test]
    fn normalize_clamps_to_audit_or_warn_and_names() {
        let p = normalize(
            json!({"description": "No RDP!", "enforcementAction": "Deny", "severity": "critical", "rules": [{"name": "No RDP", "condition": "!rdp_exposed"}, {"name": "x", "condition": ""}]}),
            "veyron-system",
        );
        assert_eq!(p["enforcementAction"], "Warn");
        assert_eq!(p["severity"], "Critical");
        assert_eq!(p["name"], "ai-no-rdp");
        assert_eq!(p["namespace"], "veyron-system");
        assert_eq!(p["rules"].as_array().unwrap().len(), 1);
        assert_eq!(p["rules"][0]["name"], "no-rdp");
    }

    #[test]
    fn check_rules_flags_unknown_variables_and_syntax() {
        let (ok, errs) = check_rules(&json!({"rules": [
            {"name": "a", "condition": "cpu_cores <= 8"},
            {"name": "b", "condition": "spec.cpu > 2"},
            {"name": "c", "condition": "cpu_cores <="}
        ]}));
        assert_eq!(ok.len(), 1);
        assert_eq!(errs.len(), 2);
    }

    #[test]
    fn vm_env_reads_kubevirt_spec_and_facts() {
        let vm = json!({
            "metadata": {"name": "w", "namespace": "p", "labels": {"env": "prod"}},
            "spec": {"runStrategy": "Always", "template": {"spec": {
                "evictionStrategy": "LiveMigrate",
                "domain": {"cpu": {"cores": 4}, "memory": {"guest": "8Gi"}, "devices": {"tpm": {}, "disks": [{}, {}], "gpus": [{}]}},
                "volumes": [{"cloudInitNoCloud": {}}]
            }}}
        });
        let mut f = Facts::default();
        f.rdp.insert(("p".into(), "w".into()));
        let env = vm_env(&vm, &f);
        for (expr, want) in [
            ("cpu_cores == 4 && memory_gib == 8.0", true),
            (
                "enable_tpm && has_cloud_init && has_eviction_strategy && running",
                true,
            ),
            ("num_disks == 2 && gpu_count == 1", true),
            (
                "rdp_exposed && !internet_egress && !has_snapshot_schedule",
                true,
            ),
            ("labels['env'] == 'prod'", true),
        ] {
            assert_eq!(
                Program::compile(expr).unwrap().eval_bool(&env).unwrap(),
                want,
                "{expr}"
            );
        }
    }
}
