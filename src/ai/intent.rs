// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Intent to infrastructure: "a Postgres box with 8 GB and a GPU" → validated
//! `POST /api/v1/vms` bodies with a cost estimate, preflight results and YAML.
//! Creating always goes through a proposal.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use axum::{Extension, Json, extract::State, response::IntoResponse, response::Response};
use k8s_openapi::api::core::v1::{Namespace, Node, ResourceQuota};
use kube::Api;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};

use super::api::err;
use super::llm::OpenAiBackend;
use super::registry::{self, safe_name};
use crate::api::auth_context::{AuthContext, role_rank};
use crate::api::http_server::web::{ApiRole, SharedState};
use crate::templates::TEMPLATES;

const MAX_VMS: usize = 10;

#[derive(Deserialize, Default)]
pub struct IntentReq {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub namespace: Option<String>,
    /// Edited specs from a previous response; skips interpretation.
    #[serde(default)]
    pub specs: Option<Vec<Value>>,
    /// Save the specs as a proposal.
    #[serde(default)]
    pub submit: bool,
}

// ── Deterministic interpretation ──────────────────────────────────

static RE_CPU: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{1,3})\s*-?\s*(?:v?cpus?|cores?|vcores?)\b").unwrap());
static RE_MEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d{1,4}(?:\.\d+)?)\s*(gb|gib|g|mb|mib|tb|tib)\b(?:\s*(?:of\s+)?(ram|memory|mem|disk|storage|ssd|volume))?")
        .unwrap()
});
static RE_DISK_WORD_FIRST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:disk|storage|ssd|volume)\s*(?:of|:)?\s*(\d{1,5})\s*(gb|gib|g|tb|tib)\b")
        .unwrap()
});
static RE_GPU: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:(\d{1,2})\s*x?\s*)?(?:nvidia\s+)?(?:[a-z]\d{1,3}\s+)?gpus?\b").unwrap()
});
static RE_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:named|called|name)\s+([a-z0-9][a-z0-9-]{0,50})").unwrap());
static RE_NS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:namespace\s+([a-z0-9][a-z0-9-]{0,62})|in\s+(?:the\s+)?([a-z0-9][a-z0-9-]{0,62})\s+namespace)")
        .unwrap()
});
static RE_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{1,2}|two|three|four|five|six|seven|eight|nine|ten)\s+(?:[a-z0-9-]+\s+)?(?:vms?|servers?|machines?|nodes?|workers?|instances?|boxes)\b")
        .unwrap()
});

fn word_num(s: &str) -> Option<usize> {
    let n = match s {
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return s.parse().ok(),
    };
    Some(n)
}

fn to_gi(n: f64, unit: &str) -> f64 {
    match unit {
        "mb" | "mib" => n / 1024.0,
        "tb" | "tib" => n * 1024.0,
        _ => n,
    }
}

fn gi_str(gi: f64) -> String {
    if gi < 1.0 {
        format!("{}Mi", (gi * 1024.0).round() as u64)
    } else {
        format!("{}Gi", gi.round() as u64)
    }
}

/// Best template for the text: most matching name parts, then a sensible default.
pub fn pick_template(text: &str, templates: &[String]) -> Option<String> {
    let t = text.to_lowercase();
    let aliases = [
        ("red hat", "rhel"),
        ("win ", "windows "),
        ("server 2022", "windows-server-2022"),
        ("server 2019", "windows-server-2019"),
        ("server 2025", "windows-server-2025"),
    ];
    let mut text = format!(" {t} ");
    for (a, b) in aliases {
        if text.contains(a) {
            text.push_str(&format!(" {b} "));
        }
    }
    let mut best: Option<(usize, usize, &String)> = None;
    for name in templates {
        let parts: Vec<&str> = name.split(['-', '_']).filter(|p| !p.is_empty()).collect();
        if !text.contains(parts[0]) {
            continue;
        }
        let score = parts.iter().filter(|p| text.contains(**p)).count();
        // Prefer more matched parts, then fewer unmatched ones (the plain family template).
        let unmatched = parts.len() - score;
        let better = match best {
            None => true,
            Some((s, u, _)) => score > s || (score == s && unmatched < u),
        };
        if better {
            best = Some((score, unmatched, name));
        }
    }
    best.map(|(_, _, n)| n.clone())
}

fn default_template(templates: &[String]) -> String {
    for t in ["ubuntu-24.04", "ubuntu-22.04", "ubuntu"] {
        if templates.iter().any(|x| x == t) {
            return t.to_string();
        }
    }
    templates
        .first()
        .cloned()
        .unwrap_or_else(|| "ubuntu-24.04".into())
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(40).collect()
}

fn short_id() -> String {
    use rand::Rng;
    let mut r = rand::thread_rng();
    (0..4)
        .map(|_| char::from(b"abcdefghjkmnpqrstuvwxyz23456789"[r.gen_range(0..31)]))
        .collect()
}

/// Turn a sentence into create-VM bodies without a language model.
pub fn interpret(text: &str, default_ns: &str, templates: &[String]) -> (Vec<Value>, Vec<String>) {
    let t = text.to_lowercase();
    let mut notes = Vec::new();

    let template = pick_template(&t, templates).unwrap_or_else(|| {
        let d = default_template(templates);
        notes.push(format!("No operating system mentioned; using {d}."));
        d
    });
    let windows = template.starts_with("windows");

    let workload = [
        ("postgres", "db"),
        ("mysql", "db"),
        ("mariadb", "db"),
        ("database", "db"),
        ("mongo", "db"),
        ("redis", "cache"),
        ("kafka", "kafka"),
        ("web", "web"),
        ("nginx", "web"),
        ("build", "build"),
        ("ci ", "ci"),
        ("jenkins", "ci"),
        ("k3s", "k3s"),
        ("kubernetes", "k8s"),
        ("dev", "dev"),
        ("llm", "llm"),
        ("inference", "infer"),
        ("training", "train"),
    ]
    .iter()
    .find(|(k, _)| t.contains(k))
    .map(|(_, v)| *v);

    // Size hints first; explicit numbers below override them.
    let (mut cpus, mut mem_gi): (u32, f64) = if windows { (2, 4.0) } else { (2, 2.0) };
    if t.contains("tiny") || t.contains("small") {
        (cpus, mem_gi) = if windows { (2, 4.0) } else { (1, 1.0) };
    } else if t.contains("large") || t.contains("big") || t.contains("beefy") {
        (cpus, mem_gi) = (8, 16.0);
    } else if t.contains("medium")
        || matches!(
            workload,
            Some("db" | "kafka" | "build" | "ci" | "k8s" | "llm" | "infer" | "train")
        )
    {
        (cpus, mem_gi) = (4, 8.0);
    }

    if let Some(c) = RE_CPU.captures(&t) {
        cpus = c[1].parse::<u32>().unwrap_or(cpus).clamp(1, 128);
    }
    let mut disk_gi: Option<f64> = None;
    let mut mem_set = false;
    for c in RE_MEM.captures_iter(&t) {
        let n: f64 = c[1].parse().unwrap_or(0.0);
        let gi = to_gi(n, &c[2]);
        match c.get(3).map(|m| m.as_str()) {
            Some("disk" | "storage" | "ssd" | "volume") => disk_gi = Some(gi),
            _ if !mem_set && gi <= 1024.0 => {
                mem_gi = gi;
                mem_set = true;
            }
            _ if disk_gi.is_none() => disk_gi = Some(gi),
            _ => {}
        }
    }
    if let Some(c) = RE_DISK_WORD_FIRST.captures(&t) {
        disk_gi = Some(to_gi(c[1].parse().unwrap_or(0.0), &c[2]));
        // "disk 100gb" was also counted as memory by RE_MEM; undo that if it was the only size.
        if mem_set && (mem_gi - disk_gi.unwrap_or(0.0)).abs() < f64::EPSILON {
            mem_gi = if windows { 4.0 } else { 2.0 };
        }
    }

    let gpu = RE_GPU.captures(&t).map(|c| {
        c.get(1)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .unwrap_or(1)
            .clamp(1, 8)
    });
    if gpu.is_some() && !mem_set && mem_gi < 16.0 {
        mem_gi = 16.0;
        cpus = cpus.max(4);
    }

    let allow_internet = ![
        "no internet",
        "offline",
        "air-gapped",
        "airgapped",
        "isolated",
        "no egress",
    ]
    .iter()
    .any(|k| t.contains(k));
    let start = !(t.contains("don't start") || t.contains("do not start") || t.contains("stopped"));

    let ns = RE_NS
        .captures(&t)
        .and_then(|c| c.get(1).or(c.get(2)).map(|m| m.as_str().to_string()))
        .unwrap_or_else(|| default_ns.to_string());

    let count = RE_COUNT
        .captures(&t)
        .and_then(|c| word_num(&c[1]))
        .unwrap_or(1)
        .clamp(1, MAX_VMS);

    let base = RE_NAME
        .captures(&t)
        .map(|c| slug(&c[1]))
        .unwrap_or_else(|| {
            let family = template.split('-').next().unwrap_or("vm");
            format!("{}-{}", workload.unwrap_or(family), short_id())
        });

    let specs = (1..=count)
        .map(|i| {
            let name = if count > 1 {
                format!("{base}-{i}")
            } else {
                base.clone()
            };
            let mut s = json!({
                "name": name,
                "namespace": ns,
                "template": template,
                "cpus": cpus,
                "memory": gi_str(mem_gi),
                "start": start,
                "allow_internet": allow_internet,
            });
            if let Some(d) = disk_gi {
                s["disk_size"] = json!(gi_str(d));
            }
            if let Some(n) = gpu {
                s["gpu"] = json!({"count": n});
            }
            s
        })
        .collect();
    (specs, notes)
}

// ── Validation ────────────────────────────────────────────────────

/// Keep only known create-VM fields with sane values.
pub fn sanitize(spec: &Value, default_ns: &str, templates: &[String]) -> Result<Value, String> {
    let name = safe_name(spec.get("name").and_then(|v| v.as_str()).unwrap_or(""))?;
    let ns = safe_name(
        spec.get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or(default_ns),
    )?;
    let mut out = json!({"name": name, "namespace": ns});
    let tpl = spec.get("template").and_then(|v| v.as_str()).unwrap_or("");
    let tpl = if templates.iter().any(|t| t == tpl) {
        tpl.to_string()
    } else {
        pick_template(tpl, templates).ok_or_else(|| format!("Unknown template '{tpl}'"))?
    };
    out["template"] = json!(tpl);
    if let Some(c) = spec.get("cpus").and_then(|v| v.as_u64()) {
        out["cpus"] = json!(c.clamp(1, 128));
    }
    for k in ["memory", "disk_size"] {
        if let Some(m) = spec.get(k).and_then(|v| v.as_str()) {
            let m = m
                .trim()
                .replace("GB", "Gi")
                .replace("gb", "Gi")
                .replace("MB", "Mi");
            if crate::utils::parse_memory_bytes(&m) == 0 {
                return Err(format!("Invalid {k} '{m}'"));
            }
            out[k] = json!(m);
        }
    }
    for k in ["profile", "cloud_init"] {
        if let Some(v) = spec
            .get(k)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            out[k] = json!(v);
        }
    }
    for k in ["start", "allow_internet"] {
        if let Some(v) = spec.get(k).and_then(|v| v.as_bool()) {
            out[k] = json!(v);
        }
    }
    if let Some(g) = spec.get("gpu").filter(|g| g.is_object()) {
        let count = g
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .clamp(1, 8);
        let mut gpu = json!({"count": count});
        if let Some(r) = g.get("resource_name").and_then(|v| v.as_str()) {
            gpu["resource_name"] = json!(r);
        }
        out["gpu"] = gpu;
    }
    Ok(out)
}

fn spec_resources(spec: &Value) -> (u32, f64, f64, u32) {
    let tpl = spec["template"].as_str().and_then(|t| TEMPLATES.get(t));
    let cpus = spec["cpus"]
        .as_u64()
        .map(|c| c as u32)
        .or_else(|| {
            tpl.as_ref()
                .map(|c| c.cpu.cores.max(1) * c.cpu.sockets.max(1))
        })
        .unwrap_or(1);
    let mem = spec["memory"]
        .as_str()
        .map(crate::utils::parse_memory_gib)
        .or_else(|| {
            tpl.as_ref()
                .map(|c| crate::utils::parse_memory_gib(&c.memory.size))
        })
        .unwrap_or(2.0);
    let disk = spec["disk_size"]
        .as_str()
        .map(crate::utils::parse_memory_gib)
        .unwrap_or(20.0);
    let gpu = spec
        .pointer("/gpu/count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    (cpus, mem, disk, gpu)
}

pub fn cost(spec: &Value) -> Value {
    let (cpus, mem, disk, gpu) = spec_resources(spec);
    let calc = crate::cost::CostCalculator::default();
    let monthly = calc.estimate_monthly_cost(cpus, mem.ceil() as u32, disk.ceil() as u32);
    json!({
        "monthly": (monthly * 100.0).round() / 100.0,
        "hourly": (monthly / 730.0 * 1000.0).round() / 1000.0,
        "currency": "USD",
        "basis": format!("{cpus} vCPU, {} memory, {} disk at the configured cost rates", gi_str(mem), gi_str(disk)),
        "excludes": if gpu > 0 { Some("GPU time is not priced by the cost rates") } else { None },
    })
}

/// Render the KubeVirt VirtualMachine the API would create.
pub fn yaml(spec: &Value) -> Option<String> {
    let mut cfg = TEMPLATES.get(spec["template"].as_str()?)?;
    cfg.name = spec["name"].as_str()?.to_string();
    cfg.namespace = spec["namespace"].as_str()?.to_string();
    let (cpus, _, _, gpu) = spec_resources(spec);
    if spec["cpus"].is_u64() {
        cfg.cpu.cores = cpus;
        cfg.cpu.sockets = 1;
        cfg.cpu.threads = 1;
    }
    if let Some(m) = spec["memory"].as_str() {
        cfg.memory.size = m.to_string();
    }
    if gpu > 0 {
        let res = spec
            .pointer("/gpu/resource_name")
            .and_then(|v| v.as_str())
            .unwrap_or("nvidia.com/gpu");
        cfg.gpus = (0..gpu)
            .map(|i| crate::config::VmGpuDevice {
                name: format!("gpu{i}"),
                device_name: res.to_string(),
                virtual_gpu_options: None,
            })
            .collect();
    }
    let vm = crate::kube::converter::vm_config_to_kubevirt(&cfg).ok()?;
    serde_yml::to_string(&vm).ok()
}

fn check(id: &str, ok: bool, level: &str, message: String) -> Value {
    json!({"check": id, "ok": ok, "level": if ok { "ok" } else { level }, "message": message})
}

/// What would stop this VM from being created or scheduled.
pub async fn preflight(client: &kube::Client, spec: &Value, nodes: &[Node]) -> Vec<Value> {
    let mut out = Vec::new();
    let ns = spec["namespace"].as_str().unwrap_or("default");
    let name = spec["name"].as_str().unwrap_or("");
    let (cpus, mem, _, gpu) = spec_resources(spec);

    let vms: Api<crate::kube::types::VirtualMachine> = Api::namespaced(client.clone(), ns);
    match vms.get_opt(name).await {
        Ok(Some(_)) => out.push(check(
            "name",
            false,
            "error",
            format!("A VM named {ns}/{name} already exists"),
        )),
        Ok(None) => out.push(check("name", true, "", format!("{ns}/{name} is free"))),
        Err(e) => out.push(check(
            "name",
            false,
            "warn",
            format!("Could not check the name: {e}"),
        )),
    }

    let nsapi: Api<Namespace> = Api::all(client.clone());
    match nsapi.get_opt(ns).await {
        Ok(Some(_)) => out.push(check(
            "namespace",
            true,
            "",
            format!("Namespace {ns} exists"),
        )),
        Ok(None) => out.push(check(
            "namespace",
            false,
            "error",
            format!("Namespace {ns} does not exist"),
        )),
        Err(_) => {}
    }

    let alloc = |n: &Node, k: &str| -> Option<String> {
        n.status
            .as_ref()?
            .allocatable
            .as_ref()?
            .get(k)
            .map(|q| q.0.clone())
    };
    let schedulable: Vec<&Node> = nodes
        .iter()
        .filter(|n| {
            !n.spec
                .as_ref()
                .and_then(|s| s.unschedulable)
                .unwrap_or(false)
        })
        .collect();
    let kvm = schedulable
        .iter()
        .filter(|n| alloc(n, "devices.kubevirt.io/kvm").is_some_and(|v| v != "0"))
        .count();
    out.push(check(
        "kvm",
        kvm > 0,
        "error",
        if kvm > 0 {
            format!("{kvm} schedulable node(s) offer KVM")
        } else {
            "No schedulable node advertises devices.kubevirt.io/kvm".into()
        },
    ));
    let fits = schedulable
        .iter()
        .filter(|n| {
            let c = alloc(n, "cpu")
                .map(|v| crate::utils::parse_cpu_nanocores(&v))
                .unwrap_or(0) as f64
                / 1e9;
            let m = alloc(n, "memory")
                .map(|v| crate::utils::parse_memory_gib(&v))
                .unwrap_or(0.0);
            c >= cpus as f64 && m >= mem
        })
        .count();
    out.push(check(
        "capacity",
        fits > 0,
        "error",
        if fits > 0 {
            format!(
                "{fits} node(s) are big enough for {cpus} vCPU / {}",
                gi_str(mem)
            )
        } else {
            format!("No node has {cpus} vCPU and {} allocatable", gi_str(mem))
        },
    ));

    if gpu > 0 {
        let want = spec
            .pointer("/gpu/resource_name")
            .and_then(|v| v.as_str())
            .unwrap_or("nvidia.com/gpu");
        let available: BTreeSet<String> = nodes
            .iter()
            .flat_map(crate::kube::gpu_inventory::node_gpu_resources)
            .filter(|r| r.allocatable > 0)
            .map(|r| r.name)
            .collect();
        out.push(check(
            "gpu",
            available.contains(want),
            "error",
            if available.contains(want) {
                format!("{want} is available")
            } else if available.is_empty() {
                "No node advertises a GPU; creation would be refused (422)".into()
            } else {
                format!(
                    "{want} is not advertised; available: {}",
                    available.into_iter().collect::<Vec<_>>().join(", ")
                )
            },
        ));
    }

    let rq: Api<ResourceQuota> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = rq.list(&Default::default()).await {
        for q in list.items {
            let qn = q.metadata.name.clone().unwrap_or_default();
            let (Some(hard), Some(used)) = (
                q.status.as_ref().and_then(|s| s.hard.clone()),
                q.status.as_ref().and_then(|s| s.used.clone()),
            ) else {
                continue;
            };
            let left = |k: &str, parse: &dyn Fn(&str) -> f64| -> Option<f64> {
                Some(parse(&hard.get(k)?.0) - used.get(k).map(|v| parse(&v.0)).unwrap_or(0.0))
            };
            let cpu_left = left("requests.cpu", &|s| {
                crate::utils::parse_cpu_nanocores(s) as f64 / 1e9
            })
            .or_else(|| {
                left("limits.cpu", &|s| {
                    crate::utils::parse_cpu_nanocores(s) as f64 / 1e9
                })
            });
            let mem_left = left("requests.memory", &|s| crate::utils::parse_memory_gib(s))
                .or_else(|| left("limits.memory", &|s| crate::utils::parse_memory_gib(s)));
            let short =
                cpu_left.is_some_and(|c| c < cpus as f64) || mem_left.is_some_and(|m| m < mem);
            out.push(check(
                "quota",
                !short,
                "error",
                format!(
                    "Quota {qn}: {} vCPU and {} left",
                    cpu_left
                        .map(|c| format!("{c:.1}"))
                        .unwrap_or_else(|| "unlimited".into()),
                    mem_left
                        .map(gi_str)
                        .unwrap_or_else(|| "unlimited memory".into())
                ),
            ));
        }
    }
    out
}

// ── LLM interpretation ───────────────────────────────────────────

async fn interpret_llm(
    backend: &OpenAiBackend,
    text: &str,
    default_ns: &str,
    templates: &[String],
) -> anyhow::Result<(Vec<Value>, String)> {
    let system = format!(
        "You turn a request for virtual machines into Veyron create-VM bodies.\n\
         Return {{\"vms\": [..], \"explanation\": \"one or two sentences\"}}. At most {MAX_VMS} VMs.\n\
         Each VM: name (DNS label), namespace (default \"{default_ns}\"), template (one of the list), \
         cpus (integer), memory (e.g. \"8Gi\"), optional disk_size (\"100Gi\"), optional gpu {{\"count\": N}}, \
         optional cloud_init (#cloud-config text for packages the workload needs), start (bool), allow_internet (bool).\n\
         Size for the workload described; prefer the newest matching OS.\n\
         Templates: {}",
        templates.join(", ")
    );
    let v = backend.complete_json(&system, text).await?;
    let vms = v
        .get("vms")
        .and_then(|x| x.as_array())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("model returned no vms"))?;
    let explanation = v
        .get("explanation")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string();
    Ok((vms, explanation))
}

// ── Handler ───────────────────────────────────────────────────────

pub async fn intent_handler(
    State(state): State<SharedState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<IntentReq>,
) -> Response {
    let ctx = super::tool_ctx(&state, &auth, "intent").await;
    let default_ns = req
        .namespace
        .clone()
        .filter(|n| !n.is_empty() && n != "all")
        .unwrap_or_else(|| "default".into());
    let templates = TEMPLATES.list();

    let (raw, source, explanation, mut notes) = if let Some(specs) = req.specs.clone() {
        (specs, "edited", String::new(), Vec::new())
    } else if req.text.trim().is_empty() {
        return err(400, "Describe the VM you want");
    } else {
        let llm = match OpenAiBackend::from_env() {
            Some(b) => interpret_llm(&b, &req.text, &default_ns, &templates)
                .await
                .ok(),
            None => None,
        };
        match llm {
            Some((v, e)) if !v.is_empty() => (v, "llm", e, Vec::new()),
            _ => {
                let (v, n) = interpret(&req.text, &default_ns, &templates);
                (v, "rules", String::new(), n)
            }
        }
    };

    let mut specs = Vec::new();
    for s in raw.iter().take(MAX_VMS) {
        match sanitize(s, &default_ns, &templates) {
            Ok(v) => specs.push(v),
            Err(e) => notes.push(e),
        }
    }
    if specs.is_empty() {
        return err(
            422,
            &format!("Could not build a VM from that: {}", notes.join("; ")),
        );
    }

    let client = ctx.kube.client();
    let nodes = Api::<Node>::all(client.clone())
        .list(&Default::default())
        .await
        .map(|l| l.items)
        .unwrap_or_default();
    let mut items = Vec::new();
    let mut total = 0.0;
    for s in &specs {
        let c = cost(s);
        total += c["monthly"].as_f64().unwrap_or(0.0);
        let pf = preflight(&client, s, &nodes).await;
        let blocked = pf.iter().any(|c| c["level"] == "error");
        items.push(json!({"spec": s, "cost": c, "preflight": pf, "ok": !blocked, "yaml": yaml(s)}));
    }

    let mut data = json!({
        "source": source,
        "explanation": if explanation.is_empty() { summary(&specs) } else { explanation },
        "notes": notes,
        "items": items,
        "total_monthly": (total * 100.0).round() / 100.0,
    });

    if req.submit {
        if role_rank(&auth.role) < role_rank(&ApiRole::Write) {
            return err(403, "Creating VMs needs the write role");
        }
        match proposal(&ctx, &specs, &req.text, total).await {
            Ok(p) => data["proposal"] = json!(p),
            Err(e) => return err(422, &e),
        }
    }
    Json(json!({"success": true, "data": data})).into_response()
}

fn summary(specs: &[Value]) -> String {
    let s = &specs[0];
    let (cpus, mem, _, gpu) = spec_resources(s);
    let what = format!(
        "{} with {cpus} vCPU and {}{}",
        s["template"].as_str().unwrap_or("VM"),
        gi_str(mem),
        if gpu > 0 {
            format!(" and {gpu} GPU")
        } else {
            String::new()
        }
    );
    if specs.len() > 1 {
        format!(
            "{} VMs, each {what}, in {}.",
            specs.len(),
            s["namespace"].as_str().unwrap_or("")
        )
    } else {
        format!(
            "{}/{}: {what}.",
            s["namespace"].as_str().unwrap_or(""),
            s["name"].as_str().unwrap_or("")
        )
    }
}

async fn proposal(
    ctx: &registry::ToolCtx,
    specs: &[Value],
    text: &str,
    monthly: f64,
) -> Result<super::proposals::Proposal, String> {
    let mut merged: Option<super::proposals::Proposal> = None;
    for s in specs {
        let p = registry::draft_change(ctx, "create_vm", &json!({"spec": s}))
            .await
            .map_err(|e| e.to_string())?;
        match merged.as_mut() {
            None => merged = Some(p),
            Some(m) => {
                m.steps.extend(p.steps);
                m.diff.extend(p.diff);
                m.verify.extend(p.verify);
                m.rollback.extend(p.rollback);
                m.blast_radius.vms.extend(p.blast_radius.vms);
                for n in p.blast_radius.namespaces {
                    if !m.blast_radius.namespaces.contains(&n) {
                        m.blast_radius.namespaces.push(n);
                    }
                }
            }
        }
    }
    let mut p = merged.ok_or("nothing to create")?;
    if specs.len() > 1 {
        p.title = format!("Create {} VMs", specs.len());
    }
    p.rationale = if text.is_empty() {
        summary(specs)
    } else {
        format!("Requested: \"{}\"", text.trim())
    };
    p.blast_radius.monthly_cost_delta = Some((monthly * 100.0).round() / 100.0);
    let out = registry::submit_proposal(ctx, p).await;
    out.proposal.ok_or_else(|| out.content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tpls() -> Vec<String> {
        [
            "ubuntu-24.04",
            "ubuntu-22.04",
            "debian-12",
            "rhel-9",
            "windows-11",
            "windows-server-2022",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    #[test]
    fn picks_templates_by_os_words() {
        let t = tpls();
        assert_eq!(
            pick_template("an ubuntu 22.04 box", &t).as_deref(),
            Some("ubuntu-22.04")
        );
        assert_eq!(
            pick_template("windows server 2022 for AD", &t).as_deref(),
            Some("windows-server-2022")
        );
        assert_eq!(pick_template("Red Hat 9", &t).as_deref(), Some("rhel-9"));
        assert_eq!(pick_template("something", &t), None);
    }

    #[test]
    fn interprets_sizes_gpu_count_and_namespace() {
        let (v, _) = interpret(
            "three postgres servers on debian with 4 cores, 16GB RAM and a 200gb disk in the data namespace",
            "default",
            &tpls(),
        );
        assert_eq!(v.len(), 3);
        assert_eq!(v[0]["template"], "debian-12");
        assert_eq!(v[0]["cpus"], 4);
        assert_eq!(v[0]["memory"], "16Gi");
        assert_eq!(v[0]["disk_size"], "200Gi");
        assert_eq!(v[0]["namespace"], "data");
        assert!(v[0]["name"].as_str().unwrap().starts_with("db-"));

        let (g, notes) = interpret(
            "a box with 2 GPUs named trainer, no internet",
            "ml",
            &tpls(),
        );
        assert_eq!(g[0]["name"], "trainer");
        assert_eq!(g[0]["gpu"]["count"], 2);
        assert_eq!(g[0]["allow_internet"], false);
        assert_eq!(g[0]["memory"], "16Gi");
        assert!(!notes.is_empty(), "default OS is called out");
    }

    #[test]
    fn sanitize_rejects_bad_names_and_maps_templates() {
        let t = tpls();
        assert!(sanitize(&json!({"name": "../x"}), "default", &t).is_err());
        let s = sanitize(
            &json!({"name": "a", "template": "ubuntu 24.04", "memory": "8GB", "evil": 1}),
            "d",
            &t,
        )
        .unwrap();
        assert_eq!(s["template"], "ubuntu-24.04");
        assert_eq!(s["memory"], "8Gi");
        assert!(s.get("evil").is_none());
    }

    #[test]
    fn cost_and_yaml_are_rendered() {
        let s = json!({"name": "a", "namespace": "default", "template": "ubuntu-24.04", "cpus": 2, "memory": "4Gi"});
        assert!(cost(&s)["monthly"].as_f64().unwrap() > 0.0);
        if TEMPLATES.exists("ubuntu-24.04") {
            let y = yaml(&s).unwrap();
            assert!(y.contains("VirtualMachine"));
        }
    }
}
