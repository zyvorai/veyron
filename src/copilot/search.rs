// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Cross-resource search (VMs, pods, nodes, PVCs, snapshots, templates, images) shared by `GET /api/v1/search`
//! and the agent's `search_resources` tool.
//!
//! Query syntax: free-text words (all must match name/namespace/labels) plus
//! optional filters `kind:vm|pod|node|pvc|snapshot|template|image`, `ns:<namespace>`, `status:<text>`,
//! `os:<text>`. Example: `kind:vm status:failed ns:prod web`.

use serde::Serialize;

use crate::kube::KubeClient;

pub const DEFAULT_LIMIT: usize = 50;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchQuery {
    pub words: Vec<String>,
    pub kinds: Vec<String>,
    pub namespace: Option<String>,
    pub status: Option<String>,
    pub os: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub status: String,
}

fn canonical_kind(k: &str) -> Option<&'static str> {
    match k {
        "vm" | "vms" | "virtualmachine" => Some("vm"),
        "pod" | "pods" => Some("pod"),
        "node" | "nodes" | "host" | "hosts" => Some("node"),
        "pvc" | "pvcs" | "disk" | "disks" | "volume" | "volumes" => Some("pvc"),
        "snapshot" | "snapshots" | "snap" | "snaps" => Some("snapshot"),
        "template" | "templates" => Some("template"),
        "image" | "images" => Some("image"),
        _ => None,
    }
}

pub fn parse_query(raw: &str) -> SearchQuery {
    let mut q = SearchQuery::default();
    for tok in raw.split_whitespace() {
        let lc = tok.to_lowercase();
        if let Some((key, val)) = lc.split_once(':') {
            if val.is_empty() {
                q.words.push(lc);
                continue;
            }
            match key {
                "kind" | "type" => {
                    for part in val.split(',') {
                        if let Some(k) = canonical_kind(part) {
                            if !q.kinds.iter().any(|x| x == k) {
                                q.kinds.push(k.to_string());
                            }
                        }
                    }
                    continue;
                }
                "ns" | "namespace" => {
                    q.namespace = Some(val.to_string());
                    continue;
                }
                "status" | "state" => {
                    q.status = Some(val.to_string());
                    continue;
                }
                "os" => {
                    q.os = Some(val.to_string());
                    continue;
                }
                _ => {}
            }
        }
        q.words.push(lc);
    }
    q
}

impl SearchQuery {
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
            && self.namespace.is_none()
            && self.status.is_none()
            && self.os.is_none()
            && self.kinds.is_empty()
    }

    fn wants_kind(&self, kind: &str) -> bool {
        self.kinds.is_empty() || self.kinds.iter().any(|k| k == kind)
    }
}

/// Pure matcher: `haystack_extra` carries label text (os, instancetype, ...).
fn matches(q: &SearchQuery, name: &str, ns: &str, status: &str, haystack_extra: &str) -> bool {
    let name_lc = name.to_lowercase();
    let ns_lc = ns.to_lowercase();
    let extra_lc = haystack_extra.to_lowercase();
    if let Some(want) = &q.namespace {
        if !ns_lc.contains(want.as_str()) {
            return false;
        }
    }
    if let Some(want) = &q.status {
        if !status.to_lowercase().contains(want.as_str()) {
            return false;
        }
    }
    if let Some(want) = &q.os {
        if !name_lc.contains(want.as_str()) && !extra_lc.contains(want.as_str()) {
            return false;
        }
    }
    q.words.iter().all(|w| {
        name_lc.contains(w.as_str()) || ns_lc.contains(w.as_str()) || extra_lc.contains(w.as_str())
    })
}

/// Label text for a Kairon Machine row: its image reference, when it has one.
fn machine_label_text(os: Option<&str>) -> String {
    match os.map(str::trim).filter(|o| !o.is_empty()) {
        Some(o) => format!("veyron.io/os={o}"),
        None => String::new(),
    }
}

fn label_text(labels: &Option<std::collections::BTreeMap<String, String>>) -> String {
    labels
        .as_ref()
        .map(|m| {
            m.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// `(name, namespace, status, label text)` for every VM in scope, from whichever
/// backend runs the VMs (Kairon Machines, or KubeVirt when explicitly selected).
/// Kairon rows carry the Machine's image reference as `veyron.io/os=<ref>`, so `os:` filters
/// match the image name (e.g. `windows-server-2022`), not KubeVirt OS labels.
async fn vm_rows(client: &KubeClient, scope: &str) -> Vec<(String, String, String, String)> {
    #[cfg(feature = "web")]
    if crate::api::vm_backend::is_kairon() {
        return crate::api::vm_backend::VmBackend::for_client(client)
            .list(scope)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|i| {
                let ns = if i.namespace.is_empty() {
                    "default".to_string()
                } else {
                    i.namespace
                };
                (i.name, ns, i.status, machine_label_text(i.os.as_deref()))
            })
            .collect();
    }
    client
        .list_vms_for_scope(scope)
        .await
        .into_iter()
        .map(|vm| {
            (
                vm.metadata.name.clone().unwrap_or_default(),
                vm.metadata
                    .namespace
                    .clone()
                    .unwrap_or_else(|| "default".into()),
                vm.status
                    .as_ref()
                    .and_then(|s| s.printable_status.clone())
                    .unwrap_or_else(|| "Unknown".into()),
                label_text(&vm.metadata.labels),
            )
        })
        .collect()
}

/// `(name, namespace, status, extra text)` for every snapshot in scope. Kairon lists disk
/// snapshots plus VM-state checkpoints; KubeVirt lists `VirtualMachineSnapshot`s. The extra
/// text carries the source VM so searching a VM name also finds its snapshots.
async fn snapshot_rows(client: &KubeClient, scope: &str) -> Vec<(String, String, String, String)> {
    #[cfg(all(feature = "web", feature = "kairon"))]
    if crate::api::vm_backend::is_kairon() {
        return crate::api::kairon_ops::list_snapshots(client, scope, None)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|s| {
                (
                    s.name,
                    s.namespace,
                    s.status,
                    format!("vm={} kind={}", s.vm_name, s.kind),
                )
            })
            .collect();
    }
    #[cfg(feature = "web")]
    {
        let raw = client.client();
        let namespaces = if scope == "all" {
            crate::api::handlers::namespace_scope::kubernetes_namespace_names(&raw).await
        } else {
            vec![scope.to_string()]
        };
        let mut out = Vec::new();
        for ns in namespaces {
            let manager = crate::snapshots::SnapshotManager::from_client(raw.clone(), &ns);
            for s in manager.list_all_snapshots().await.unwrap_or_default() {
                out.push((
                    s.name,
                    s.namespace,
                    s.status.to_string(),
                    format!("vm={}", s.vm_name),
                ));
            }
        }
        return out;
    }
    #[allow(unreachable_code)]
    {
        let _ = (client, scope);
        Vec::new()
    }
}

/// `(name, status, extra text)` for each Kairon catalog image (cluster-scoped). Empty on KubeVirt.
async fn image_rows(client: &KubeClient) -> Vec<(String, String, String)> {
    #[cfg(all(feature = "web", feature = "kairon"))]
    if crate::api::vm_backend::is_kairon() {
        use kube::api::{Api, ListParams};
        let list = Api::<crate::kairon::CatalogImage>::all(client.client())
            .list(&ListParams::default())
            .await
            .map(|l| l.items)
            .unwrap_or_default();
        return list
            .iter()
            .map(|m| {
                let j = crate::api::handlers::machine_images::catalog_json(m);
                let field = |k: &str| j[k].as_str().unwrap_or("").to_string();
                (
                    m.metadata.name.clone().unwrap_or_default(),
                    if m.spec.deprecated {
                        "Deprecated"
                    } else {
                        "Available"
                    }
                    .to_string(),
                    format!(
                        "os={} family={} version={} kind={}",
                        field("os"),
                        field("family"),
                        field("version"),
                        field("kind")
                    ),
                )
            })
            .collect();
    }
    let _ = client;
    Vec::new()
}

/// Run the search. `scope` is a namespace or `"all"`; a `ns:` filter narrows it.
pub async fn search(
    client: &KubeClient,
    scope: &str,
    q: &SearchQuery,
    limit: usize,
) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    if q.is_empty() {
        return hits;
    }
    let limit = limit.clamp(1, 200);

    if q.wants_kind("vm") {
        for (name, ns, status, labels) in vm_rows(client, scope).await {
            if matches(q, &name, &ns, &status, &labels) {
                hits.push(SearchHit {
                    kind: "vm".into(),
                    id: format!("{ns}/{name}"),
                    title: name,
                    subtitle: format!("{ns} · {status}"),
                    status,
                });
            }
        }
    }
    if q.wants_kind("pod") {
        for p in client.list_pods_for_scope(scope).await {
            let name = p.metadata.name.as_deref().unwrap_or("");
            let ns = p.metadata.namespace.as_deref().unwrap_or("default");
            let status = p
                .status
                .as_ref()
                .and_then(|s| s.phase.clone())
                .unwrap_or_else(|| "Unknown".into());
            if matches(q, name, ns, &status, &label_text(&p.metadata.labels)) {
                hits.push(SearchHit {
                    kind: "pod".into(),
                    id: format!("{ns}/{name}"),
                    title: name.into(),
                    subtitle: format!("{ns} · {status}"),
                    status,
                });
            }
        }
    }
    if q.wants_kind("node") && q.namespace.is_none() {
        for n in client.list_nodes().await.unwrap_or_default() {
            let name = n.metadata.name.as_deref().unwrap_or("");
            let ready = n
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .and_then(|cs| cs.iter().find(|c| c.type_ == "Ready"))
                .map(|c| c.status == "True")
                .unwrap_or(false);
            let status = if ready { "Ready" } else { "NotReady" }.to_string();
            if matches(q, name, "", &status, &label_text(&n.metadata.labels)) {
                hits.push(SearchHit {
                    kind: "node".into(),
                    id: name.into(),
                    title: name.into(),
                    subtitle: status.clone(),
                    status,
                });
            }
        }
    }
    if q.wants_kind("pvc") {
        for c in client.list_pvcs_for_scope(scope).await {
            let name = c.metadata.name.as_deref().unwrap_or("");
            let ns = c.metadata.namespace.as_deref().unwrap_or("default");
            let status = c
                .status
                .as_ref()
                .and_then(|s| s.phase.clone())
                .unwrap_or_else(|| "Unknown".into());
            if matches(q, name, ns, &status, &label_text(&c.metadata.labels)) {
                hits.push(SearchHit {
                    kind: "pvc".into(),
                    id: format!("{ns}/{name}"),
                    title: name.into(),
                    subtitle: format!("{ns} · {status}"),
                    status,
                });
            }
        }
    }
    if q.wants_kind("snapshot") {
        for (name, ns, status, extra) in snapshot_rows(client, scope).await {
            if matches(q, &name, &ns, &status, &extra) {
                hits.push(SearchHit {
                    kind: "snapshot".into(),
                    id: format!("{ns}/{name}"),
                    title: name,
                    subtitle: format!("{ns} · {status}"),
                    status,
                });
            }
        }
    }
    // Templates and images are cluster-wide catalogs: a namespace filter excludes them.
    if q.wants_kind("template") && q.namespace.is_none() {
        for name in crate::templates::TEMPLATES.list() {
            if matches(q, &name, "", "Available", "") {
                hits.push(SearchHit {
                    kind: "template".into(),
                    id: name.clone(),
                    title: name,
                    subtitle: "built-in template".into(),
                    status: "Available".into(),
                });
            }
        }
    }
    if q.wants_kind("image") && q.namespace.is_none() {
        for (name, status, extra) in image_rows(client).await {
            if matches(q, &name, "", &status, &extra) {
                hits.push(SearchHit {
                    kind: "image".into(),
                    id: name.clone(),
                    title: name,
                    subtitle: format!("catalog image · {status}"),
                    status,
                });
            }
        }
    }
    hits.truncate(limit);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_filters_and_words() {
        let q = parse_query("kind:vm,disk Status:Failed ns:prod web-01 os:windows");
        assert_eq!(q.kinds, vec!["vm", "pvc"]);
        assert_eq!(q.status.as_deref(), Some("failed"));
        assert_eq!(q.namespace.as_deref(), Some("prod"));
        assert_eq!(q.os.as_deref(), Some("windows"));
        assert_eq!(q.words, vec!["web-01"]);
    }

    #[test]
    fn unknown_prefix_and_empty_value_stay_free_text() {
        let q = parse_query("foo:bar kind: ");
        assert_eq!(q.words, vec!["foo:bar", "kind:"]);
        assert!(q.kinds.is_empty());
    }

    #[test]
    fn empty_query_is_empty() {
        assert!(parse_query("   ").is_empty());
        assert!(!parse_query("ns:x").is_empty());
    }

    #[test]
    fn matcher_requires_all_words_and_filters() {
        let q = parse_query("web ns:prod status:run");
        assert!(matches(&q, "web-01", "prod", "Running", ""));
        assert!(!matches(&q, "web-01", "dev", "Running", ""));
        assert!(!matches(&q, "web-01", "prod", "Stopped", ""));
        assert!(!matches(&q, "db-01", "prod", "Running", ""));
        let q = parse_query("os:windows");
        assert!(matches(
            &q,
            "vm1",
            "d",
            "Running",
            "kubevirt.io/os=windows2022"
        ));
        assert!(!matches(&q, "vm1", "d", "Running", "kubevirt.io/os=ubuntu"));
    }

    #[test]
    fn os_filter_matches_kairon_image_refs() {
        let q = parse_query("os:windows");
        let win = machine_label_text(Some("windows-server-2022"));
        assert_eq!(win, "veyron.io/os=windows-server-2022");
        assert!(matches(&q, "vm1", "d", "Running", &win));
        let ubu = machine_label_text(Some("ubuntu-24.04"));
        assert!(!matches(&q, "vm1", "d", "Running", &ubu));
        assert_eq!(machine_label_text(None), "");
        assert_eq!(machine_label_text(Some("  ")), "");
        assert!(!matches(
            &q,
            "vm1",
            "d",
            "Running",
            &machine_label_text(None)
        ));
    }

    #[test]
    fn new_kinds_parse_with_their_aliases() {
        let q = parse_query("kind:snaps,templates,images db");
        assert_eq!(q.kinds, vec!["snapshot", "template", "image"]);
        assert!(q.wants_kind("snapshot") && !q.wants_kind("vm"));
        assert_eq!(q.words, vec!["db"]);
        assert!(
            parse_query("db").wants_kind("template"),
            "default searches every kind"
        );
    }

    #[test]
    fn kind_filter_limits_kinds() {
        let q = parse_query("kind:node");
        assert!(q.wants_kind("node"));
        assert!(!q.wants_kind("vm"));
        assert!(parse_query("x").wants_kind("pod"));
    }
}
