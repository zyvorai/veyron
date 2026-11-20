// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Natural-language search: a question becomes a small filter the console runs
//! over rows it already has loaded, so answers are instant and never stale.

use std::sync::LazyLock;

use axum::{Json, response::IntoResponse, response::Response};
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};

use super::api::err;
use super::llm::OpenAiBackend;

/// Searchable fields per console resource (keys of the console's row objects).
pub const SCHEMA: &[(&str, &[(&str, &str)])] = &[
    (
        "vms",
        &[
            ("name", "string"),
            ("ns", "string"),
            (
                "status",
                "string: Running, Stopped, Paused, Starting, Migrating, Failed, Unknown",
            ),
            ("cpu", "number: vCPUs"),
            ("ram", "number: GiB of memory"),
            ("disk", "number: GB of disk"),
            ("host", "string: node name"),
            ("ip", "string, \"—\" when none"),
            ("os", "string: OS or template"),
        ],
    ),
    (
        "hosts",
        &[
            ("name", "string"),
            ("status", "string: Ready, NotReady, Cordoned"),
            ("cpu", "number: CPU use %"),
            ("mem", "number: memory use %"),
            ("vms", "number: VMs on the host"),
            ("kernel", "string"),
        ],
    ),
    (
        "pods",
        &[
            ("name", "string"),
            ("ns", "string"),
            (
                "status",
                "string: Running, Pending, Failed, Succeeded, CrashLoopBackOff",
            ),
            ("restarts", "number"),
            ("node", "string"),
            ("image", "string"),
        ],
    ),
    (
        "pvcs",
        &[
            ("name", "string"),
            ("ns", "string"),
            ("status", "string: Bound, Pending, Lost"),
            ("cls", "string: storage class"),
            ("size", "string, e.g. 20Gi"),
            ("used", "number: % used"),
        ],
    ),
    (
        "snapshots",
        &[
            ("name", "string"),
            ("ns", "string"),
            ("vm", "string: source VM"),
            ("status", "string: Ready, InProgress, Failed"),
        ],
    ),
];

const OPS: &[&str] = &[
    "eq",
    "ne",
    "contains",
    "not_contains",
    "gt",
    "gte",
    "lt",
    "lte",
    "in",
];

#[derive(Deserialize, Default)]
pub struct SearchReq {
    #[serde(default)]
    pub query: String,
}

fn fields(resource: &str) -> Option<&'static [(&'static str, &'static str)]> {
    SCHEMA.iter().find(|(r, _)| *r == resource).map(|(_, f)| *f)
}

fn is_number(resource: &str, field: &str) -> bool {
    fields(resource)
        .and_then(|f| f.iter().find(|(n, _)| *n == field))
        .is_some_and(|(_, t)| t.starts_with("number"))
}

/// Drop anything that is not in the schema so the console never evaluates junk.
pub fn validate(filter: &Value) -> Result<Value, String> {
    let resource = filter
        .get("resource")
        .and_then(|v| v.as_str())
        .unwrap_or("vms");
    let f = fields(resource).ok_or_else(|| format!("Cannot search {resource}"))?;
    let mut where_ = Vec::new();
    for c in filter
        .get("where")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let field = c.get("field").and_then(|v| v.as_str()).unwrap_or("");
        let op = c.get("op").and_then(|v| v.as_str()).unwrap_or("eq");
        if !f.iter().any(|(n, _)| *n == field) || !OPS.contains(&op) {
            continue;
        }
        let mut value = c.get("value").cloned().unwrap_or(Value::Null);
        if is_number(resource, field) && matches!(op, "gt" | "gte" | "lt" | "lte") {
            match value
                .as_f64()
                .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
            {
                Some(n) => value = json!(n),
                None => continue,
            }
        }
        where_.push(json!({"field": field, "op": op, "value": value}));
    }
    let sort = filter.get("sort").filter(|s| s.is_object()).and_then(|s| {
        let field = s.get("field")?.as_str()?;
        f.iter().any(|(n, _)| *n == field).then(|| {
            let dir = if s.get("dir").and_then(|d| d.as_str()) == Some("asc") {
                "asc"
            } else {
                "desc"
            };
            json!({"field": field, "dir": dir})
        })
    });
    let limit = filter
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(1, 500));
    Ok(json!({"resource": resource, "where": where_, "sort": sort, "limit": limit}))
}

// ── Deterministic parser ─────────────────────────────────────────

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).unwrap());
    };
}

re!(
    RE_CMP,
    r"(more than|over|greater than|above|at least|>=|>|less than|under|below|fewer than|at most|<=|<|exactly|with)\s+(\d+(?:\.\d+)?)\s*(v?cpus?|cores?|gb|gib|g|%|percent|restarts?|vms?)?\s*(ram|memory|mem|of ram|of memory|disk|cpu|restarts?)?"
);
re!(
    RE_ON,
    r"\bon\s+(?:node|host)\s+([a-z0-9][a-z0-9.-]*)|\bon\s+([a-z0-9][a-z0-9-]*\d[a-z0-9.-]*)"
);
re!(
    RE_NS,
    r"\b(?:in\s+(?:the\s+)?namespace\s+|namespace\s+|in\s+(?:the\s+)?)([a-z0-9][a-z0-9-]*)(?:\s+namespace)?"
);
re!(
    RE_NAME,
    r#"\b(?:named|called|name contains|with)\s+["']?([a-z0-9][a-z0-9.-]*)["']?(?:\s+in (?:the|their) name)?"#
);
re!(RE_TOP, r"\b(?:top|first)\s+(\d{1,3})\b");
re!(
    RE_CLASS,
    r"\b(?:class|storage class|storageclass)\s+([a-z0-9][a-z0-9.-]*)"
);
re!(RE_OF_VM, r"\b(?:of|for)\s+(?:vm\s+)?([a-z0-9][a-z0-9-]*)");

fn resource_of(t: &str) -> &'static str {
    let first = |words: &[&str]| words.iter().filter_map(|w| t.find(w)).min();
    let hits = [
        ("snapshots", first(&["snapshot"])),
        ("pvcs", first(&["pvc", "volume", "disks"])),
        ("pods", first(&["pod"])),
        (
            "hosts",
            first(&["hosts", "nodes", "host ", "node "]).filter(|i| {
                // "vms on node x" is about VMs.
                !t[..*i].trim_end().ends_with(" on")
            }),
        ),
        (
            "vms",
            first(&["vm", "machine", "server", "instance", "guest"]),
        ),
    ];
    hits.iter()
        .filter_map(|(r, i)| i.map(|i| (i, *r)))
        .min()
        .map(|(_, r)| r)
        .unwrap_or("vms")
}

fn cond(field: &str, op: &str, value: Value) -> Value {
    json!({"field": field, "op": op, "value": value})
}

pub fn parse(query: &str) -> Value {
    let t = format!(" {} ", query.to_lowercase());
    let resource = resource_of(&t);
    let mut w: Vec<Value> = Vec::new();
    let mut sort: Option<Value> = None;
    let mut limit: Option<u64> = None;

    // Status words.
    let statuses: &[(&str, &str)] = match resource {
        "hosts" => &[
            ("not ready", "NotReady"),
            ("notready", "NotReady"),
            ("cordoned", "Cordoned"),
            ("ready", "Ready"),
        ],
        "pods" => &[
            ("crashloop", "CrashLoopBackOff"),
            ("crashing", "CrashLoopBackOff"),
            ("pending", "Pending"),
            ("failed", "Failed"),
            ("completed", "Succeeded"),
            ("running", "Running"),
        ],
        "pvcs" => &[
            ("unbound", "Pending"),
            ("pending", "Pending"),
            ("lost", "Lost"),
            ("bound", "Bound"),
        ],
        "snapshots" => &[
            ("failed", "Failed"),
            ("in progress", "InProgress"),
            ("ready", "Ready"),
        ],
        _ => &[
            ("not running", "!Running"),
            ("stopped", "Stopped"),
            ("halted", "Stopped"),
            ("off ", "Stopped"),
            ("paused", "Paused"),
            ("failed", "Failed"),
            ("broken", "Failed"),
            ("migrating", "Migrating"),
            ("starting", "Starting"),
            ("pending", "Pending"),
            ("running", "Running"),
        ],
    };
    if let Some((_, s)) = statuses.iter().find(|(k, _)| t.contains(k)) {
        match s.strip_prefix('!') {
            Some(neg) => w.push(cond("status", "ne", json!(neg))),
            None => w.push(cond("status", "eq", json!(s))),
        }
    }

    // OS words (VMs).
    if resource == "vms" {
        for os in [
            "windows", "ubuntu", "debian", "rhel", "centos", "rocky", "alma", "fedora", "suse",
            "arch", "alpine",
        ] {
            if t.contains(os) {
                w.push(cond("os", "contains", json!(os)));
                break;
            }
        }
        if t.contains("no ip") || t.contains("without an ip") || t.contains("without ip") {
            w.push(cond("ip", "eq", json!("—")));
        }
    }

    // Numeric comparisons.
    for c in RE_CMP.captures_iter(&t) {
        let op = match &c[1] {
            "more than" | "over" | "greater than" | "above" | ">" => "gt",
            "at least" | ">=" => "gte",
            "less than" | "under" | "below" | "fewer than" | "<" => "lt",
            "at most" | "<=" => "lte",
            _ => "eq",
        };
        let n: f64 = c[2].parse().unwrap_or(0.0);
        let unit = c.get(3).map(|m| m.as_str()).unwrap_or("");
        let noun = c.get(4).map(|m| m.as_str()).unwrap_or("");
        let field = match (resource, unit, noun) {
            ("pods", u, n) if u.starts_with("restart") || n.starts_with("restart") => "restarts",
            ("vms", u, _) if u.contains("cpu") || u.starts_with("core") => "cpu",
            ("vms", _, n) if n.contains("disk") => "disk",
            ("vms", "gb" | "gib" | "g", _) => "ram",
            ("vms", _, n) if n.contains("ram") || n.contains("mem") => "ram",
            ("hosts", u, _) if u.starts_with("vm") => "vms",
            ("hosts", _, n) if n.contains("mem") || n.contains("ram") => "mem",
            ("hosts", "%" | "percent", _) => "cpu",
            ("pvcs", "%" | "percent", _) => "used",
            _ => continue,
        };
        if &c[1] == "with" && op == "eq" && unit.is_empty() && noun.is_empty() {
            continue;
        }
        w.push(cond(field, op, json!(n)));
    }

    // Placement and naming.
    if let Some(c) = RE_ON.captures(&t) {
        let node = c.get(1).or(c.get(2)).map(|m| m.as_str()).unwrap_or("");
        match resource {
            "vms" => w.push(cond("host", "contains", json!(node))),
            "pods" => w.push(cond("node", "contains", json!(node))),
            _ => {}
        }
    }
    if resource != "hosts" {
        if let Some(c) = RE_NS.captures(&t) {
            let ns = &c[1];
            let generic = [
                "the",
                "a",
                "my",
                "use",
                "progress",
                "total",
                "all",
                "production",
                "prod",
            ];
            let explicit = t.contains("namespace");
            if explicit
                || !generic.contains(&ns)
                    && c.get(0).is_some_and(|m| m.as_str().ends_with("namespace"))
            {
                w.push(cond("ns", "eq", json!(ns)));
            }
        }
    }
    if let Some(c) = RE_NAME.captures(&t) {
        let v = &c[1];
        let reserved = [
            "more", "less", "at", "over", "under", "no", "an", "a", "the", "gpu", "gpus",
        ];
        if !reserved.contains(&v) && !v.chars().all(|ch| ch.is_ascii_digit()) {
            w.push(cond("name", "contains", json!(v)));
        }
    }
    if resource == "pvcs" {
        if let Some(c) = RE_CLASS.captures(&t) {
            w.push(cond("cls", "contains", json!(&c[1])));
        }
    }
    if resource == "snapshots" {
        if let Some(c) = RE_OF_VM.captures(&t) {
            w.push(cond("vm", "contains", json!(&c[1])));
        }
    }
    if resource == "pods"
        && (t.contains("restarting") || t.contains("restarted"))
        && !w.iter().any(|c| c["field"] == "restarts")
    {
        w.push(cond("restarts", "gt", json!(0)));
    }

    // Ranking.
    let rank: &[(&str, &str, &str)] = &[
        ("most memory", "ram", "desc"),
        ("biggest", "ram", "desc"),
        ("largest", "ram", "desc"),
        ("smallest", "ram", "asc"),
        ("most cpu", "cpu", "desc"),
        ("most cores", "cpu", "desc"),
        ("busiest", "cpu", "desc"),
        ("most loaded", "cpu", "desc"),
        ("idle", "cpu", "asc"),
        ("most restarts", "restarts", "desc"),
        ("fullest", "used", "desc"),
        ("most vms", "vms", "desc"),
    ];
    for (k, f, d) in rank {
        if t.contains(k) && fields(resource).is_some_and(|fs| fs.iter().any(|(n, _)| n == f)) {
            sort = Some(json!({"field": f, "dir": d}));
            break;
        }
    }
    if let Some(c) = RE_TOP.captures(&t) {
        limit = c[1].parse().ok();
    }
    if sort.is_some()
        && limit.is_none()
        && (t.contains("most ") || t.contains("est "))
        && !t.contains(" all ")
    {
        limit = Some(10);
    }

    json!({"resource": resource, "where": w, "sort": sort, "limit": limit})
}

pub fn describe(f: &Value) -> String {
    let res = f["resource"].as_str().unwrap_or("vms");
    let label = match res {
        "vms" => "VMs",
        "hosts" => "hosts",
        "pods" => "pods",
        "pvcs" => "volumes",
        _ => "snapshots",
    };
    let conds: Vec<String> = f["where"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|c| {
                    let op = match c["op"].as_str().unwrap_or("") {
                        "eq" => "is",
                        "ne" => "is not",
                        "contains" => "contains",
                        "not_contains" => "does not contain",
                        "gt" => ">",
                        "gte" => "≥",
                        "lt" => "<",
                        "lte" => "≤",
                        o => o,
                    };
                    let v = c["value"]
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| c["value"].to_string());
                    format!("{} {op} {v}", c["field"].as_str().unwrap_or(""))
                })
                .collect()
        })
        .unwrap_or_default();
    let mut s = if conds.is_empty() {
        format!("All {label}")
    } else {
        format!("{} where {}", capitalize(label), conds.join(" and "))
    };
    if let Some(field) = f.pointer("/sort/field").and_then(|v| v.as_str()) {
        let dir = if f.pointer("/sort/dir").and_then(|v| v.as_str()) == Some("asc") {
            "lowest"
        } else {
            "highest"
        };
        s.push_str(&format!(", {dir} {field} first"));
    }
    if let Some(n) = f["limit"].as_u64() {
        s.push_str(&format!(", top {n}"));
    }
    s
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

async fn parse_llm(backend: &OpenAiBackend, query: &str) -> anyhow::Result<Value> {
    let schema: Vec<String> = SCHEMA
        .iter()
        .map(|(r, f)| {
            format!(
                "{r}: {}",
                f.iter()
                    .map(|(n, t)| format!("{n} ({t})"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    let system = format!(
        "Translate a question about infrastructure into a filter over a table.\n\
         Return {{\"resource\": one of vms|hosts|pods|pvcs|snapshots, \
         \"where\": [{{\"field\", \"op\", \"value\"}}], \"sort\": {{\"field\", \"dir\": \"asc\"|\"desc\"}} or null, \"limit\": number or null}}.\n\
         Ops: {}. String matching is case-insensitive.\nFields:\n{}",
        OPS.join(", "),
        schema.join("\n")
    );
    backend.complete_json(&system, query).await
}

pub async fn search_handler(Json(req): Json<SearchReq>) -> Response {
    let q = req.query.trim();
    if q.is_empty() {
        return err(400, "Ask a question");
    }
    if q.len() > 500 {
        return err(400, "Question is too long");
    }
    let (raw, source) = match OpenAiBackend::from_env() {
        Some(b) => match parse_llm(&b, q).await {
            Ok(v) => (v, "llm"),
            Err(_) => (parse(q), "rules"),
        },
        None => (parse(q), "rules"),
    };
    let filter = match validate(&raw) {
        Ok(f) => f,
        Err(_) => validate(&parse(q)).unwrap_or_else(|_| json!({"resource": "vms", "where": []})),
    };
    let explanation = describe(&filter);
    Json(json!({"success": true, "data": {"filter": filter, "source": source, "explanation": explanation}}))
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(f: &Value, field: &str, op: &str, value: Value) -> bool {
        f["where"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["field"] == field && c["op"] == op && c["value"] == value)
    }

    #[test]
    fn vms_by_status_os_and_size() {
        let f = parse("stopped windows VMs with more than 8 GB RAM");
        assert_eq!(f["resource"], "vms");
        assert!(has(&f, "status", "eq", json!("Stopped")));
        assert!(has(&f, "os", "contains", json!("windows")));
        assert!(has(&f, "ram", "gt", json!(8.0)));
    }

    #[test]
    fn vms_on_a_node_are_vms_not_hosts() {
        let f = parse("vms on node worker-2 with at least 4 cores");
        assert_eq!(f["resource"], "vms");
        assert!(has(&f, "host", "contains", json!("worker-2")));
        assert!(has(&f, "cpu", "gte", json!(4.0)));
    }

    #[test]
    fn pods_restarting_in_namespace() {
        let f = parse("pods restarting in namespace kubevirt");
        assert_eq!(f["resource"], "pods");
        assert!(has(&f, "restarts", "gt", json!(0)));
        assert!(has(&f, "ns", "eq", json!("kubevirt")));
    }

    #[test]
    fn ranking_and_limits() {
        let f = parse("top 5 busiest hosts");
        assert_eq!(f["resource"], "hosts");
        assert_eq!(f["sort"]["field"], "cpu");
        assert_eq!(f["limit"], 5);
        let g = parse("which vms use the most memory");
        assert_eq!(g["sort"]["field"], "ram");
        assert_eq!(g["limit"], 10);
    }

    #[test]
    fn validate_drops_unknown_fields_and_ops() {
        let v = validate(&json!({
            "resource": "vms",
            "where": [
                {"field": "ram", "op": "gt", "value": "8"},
                {"field": "secret", "op": "eq", "value": 1},
                {"field": "name", "op": "regex", "value": ".*"}
            ],
            "sort": {"field": "nope"}
        }))
        .unwrap();
        assert_eq!(v["where"].as_array().unwrap().len(), 1);
        assert_eq!(v["where"][0]["value"], json!(8.0));
        assert!(v["sort"].is_null());
        assert!(validate(&json!({"resource": "secrets"})).is_err());
    }

    #[test]
    fn describes_filters_in_words() {
        let f = parse("running ubuntu vms");
        assert_eq!(
            describe(&f),
            "VMs where status is Running and os contains ubuntu"
        );
    }
}
