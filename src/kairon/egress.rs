// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Per-VM outbound policy ("Network access") for Kairon Machines.
//!
//! A guest on QEMU user-mode networking has no host-visible interface; its traffic is
//! re-originated by the QEMU process, so FluxVM governs it with the `fluxvm_qemu_egress`
//! eBPF program on the VM's cgroup. That program is policy-driven: a mode plus
//! `cidr/proto/port` rules (`spec.network.egress` on the Machine). This module is the
//! Veyron side of that contract: validation identical to FluxVM's (so a bad policy is a
//! friendly 422 here instead of a failed apply on the node), "would this be allowed?"
//! answers, named presets resolved against Veyron's own reachable endpoints, and the
//! diagnostics the console shows.
//!
//! Two facts shape the presets:
//! * Cilium's socket load balancer rewrites a ClusterIP/NodePort destination to the backend
//!   pod address *before* the egress hook runs, so the filter sees `<pod ip>:5151`, not the
//!   node IP. Platform rules therefore cover the pod network as well as the node address.
//! * The kernel returns only the longest matching prefix, so a narrow rule must also carry
//!   the ports of every covering rule. FluxVM folds them and allows at most
//!   [`MAX_PORTS`] distinct `(proto, port)` per network; [`compile`] enforces the same limit.

use std::collections::{BTreeMap, BTreeSet};
use std::net::Ipv4Addr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Capacity of the node's rule table (`MAX_RULES` in FluxVM).
pub const MAX_RULES: usize = 256;
/// Distinct `(proto, port)` pairs one network may carry once covering rules are folded in.
pub const MAX_PORTS: usize = 4;

/// Annotation on a namespace naming the default preset for new VMs.
pub const ANN_DEFAULT_PRESET: &str = "veyron.io/default-egress-preset";
/// Preset applied to new Linux VMs when neither the namespace nor the environment names one.
pub const BUILTIN_DEFAULT_PRESET: &str = "platform-packages";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum EgressMode {
    /// Only loopback leaves the VM's QEMU process. FluxVM's default.
    #[default]
    Loopback,
    Allowlist,
    /// Allow everything and record what an allow-list would have refused.
    Audit,
    Open,
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum EgressProto {
    #[default]
    Any,
    Tcp,
    Udp,
}

/// One allowed destination. `port: None` allows every port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EgressRule {
    pub cidr: String,
    #[serde(default)]
    pub proto: EgressProto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl EgressRule {
    pub fn new(cidr: &str, proto: EgressProto, port: Option<u16>, note: &str) -> Self {
        Self {
            cidr: cidr.into(),
            proto,
            port,
            note: (!note.is_empty()).then(|| note.to_string()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct EgressPolicy {
    pub mode: EgressMode,
    pub rules: Vec<EgressRule>,
}

// ---- CIDR helpers --------------------------------------------------------------------

fn mask(len: u8) -> u32 {
    if len == 0 { 0 } else { u32::MAX << (32 - len) }
}

/// Parse `a.b.c.d[/len]`; host bits are cleared.
pub fn parse_cidr(s: &str) -> Result<(u32, u8), String> {
    let (ip, len) = match s.split_once('/') {
        Some((ip, len)) => (
            ip,
            len.trim()
                .parse::<u8>()
                .map_err(|_| format!("{s:?}: the prefix length is not a number"))?,
        ),
        None => (s, 32),
    };
    if len > 32 {
        return Err(format!("{s:?}: the prefix length is greater than 32"));
    }
    let ip: Ipv4Addr = ip.trim().parse().map_err(|_| {
        format!("{s:?} is not an IPv4 address or CIDR (IPv6 rules are not supported yet)")
    })?;
    Ok((u32::from(ip) & mask(len), len))
}

fn fmt_cidr(addr: u32, len: u8) -> String {
    format!("{}/{}", Ipv4Addr::from(addr), len)
}

fn covers(outer: (u8, u32), inner: (u8, u32)) -> bool {
    outer.0 <= inner.0 && (inner.1 & mask(outer.0)) == outer.1
}

/// Ranges the "public internet" presets use: everything except private, loopback, link-local,
/// carrier-grade NAT and multicast/reserved space.
pub fn public_ranges() -> Vec<String> {
    const EXCLUDED: &[(&str, u8)] = &[
        ("0.0.0.0", 8),
        ("10.0.0.0", 8),
        ("100.64.0.0", 10),
        ("127.0.0.0", 8),
        ("169.254.0.0", 16),
        ("172.16.0.0", 12),
        ("192.168.0.0", 16),
        ("224.0.0.0", 3),
    ];
    let mut ranges: Vec<(u32, u8)> = vec![(0, 0)];
    for (ip, len) in EXCLUDED {
        let ex_addr = u32::from(ip.parse::<Ipv4Addr>().expect("static address"));
        let ex = (*len, ex_addr);
        let mut next = Vec::new();
        // Subtract `ex` from each range by splitting ranges that contain it.
        let mut work = ranges;
        while let Some((addr, plen)) = work.pop() {
            let r = (plen, addr);
            if covers(ex, r) {
                continue; // wholly excluded
            }
            if !covers(r, ex) {
                next.push((addr, plen)); // disjoint
                continue;
            }
            // r strictly contains ex: split into halves.
            let half = plen + 1;
            work.push((addr, half));
            work.push((addr | (1u32 << (32 - half)), half));
        }
        next.sort();
        ranges = next;
    }
    ranges
        .into_iter()
        .map(|(a, l)| fmt_cidr(a, l))
        .collect::<Vec<_>>()
}

// ---- validation and folding -----------------------------------------------------------

struct Parsed {
    prefix: u8,
    addr: u32,
    proto: EgressProto,
    port: Option<u16>,
}

fn parse_rules(rules: &[EgressRule]) -> Result<Vec<Parsed>, String> {
    rules
        .iter()
        .map(|r| {
            let (addr, prefix) = parse_cidr(&r.cidr)?;
            if r.port == Some(0) {
                return Err(format!(
                    "rule {:?}: 0 is not a port; leave the port out to allow every port",
                    r.cidr
                ));
            }
            if r.port.is_some() && r.proto == EgressProto::Any {
                return Err(format!(
                    "rule {:?}: a port needs a protocol (tcp or udp)",
                    r.cidr
                ));
            }
            Ok(Parsed {
                prefix,
                addr,
                proto: r.proto,
                port: r.port,
            })
        })
        .collect()
}

/// What the rules for one network allow: every port (`true`), or these `(proto, port)` pairs.
type Ports = (bool, BTreeSet<(EgressProto, u16)>);

/// One entry of the node's rule table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compiled {
    pub prefix: u8,
    pub addr: u32,
    /// `None` = any port and protocol.
    pub ports: Option<Vec<(EgressProto, u16)>>,
}

/// Fold rules into the table FluxVM will program (see the module docs). Errors match what
/// the node would answer, in words an operator can act on.
pub fn compile(rules: &[EgressRule]) -> Result<Vec<Compiled>, String> {
    let parsed = parse_rules(rules)?;
    let mut own: BTreeMap<(u8, u32), Ports> = BTreeMap::new();
    for p in &parsed {
        let e = own.entry((p.prefix, p.addr)).or_default();
        match p.port {
            None => e.0 = true,
            Some(port) => {
                e.1.insert((p.proto, port));
            }
        }
    }
    let mut out = Vec::new();
    for &key in own.keys() {
        let mut any = false;
        let mut ports = BTreeSet::new();
        for (&other, (oany, oports)) in own.iter() {
            if covers(other, key) {
                any |= *oany;
                ports.extend(oports.iter().copied());
            }
        }
        if any {
            out.push(Compiled {
                prefix: key.0,
                addr: key.1,
                ports: None,
            });
            continue;
        }
        if ports.len() > MAX_PORTS {
            return Err(format!(
                "{} would need {} different ports once the rules of the networks around it are included; at most {MAX_PORTS} are supported per network. Allow every port for it, or use fewer distinct ports",
                fmt_cidr(key.1, key.0),
                ports.len()
            ));
        }
        out.push(Compiled {
            prefix: key.0,
            addr: key.1,
            ports: Some(ports.into_iter().collect()),
        });
    }
    Ok(out)
}

/// Check a policy the way FluxVM will.
pub fn validate(policy: &EgressPolicy) -> Result<(), String> {
    if policy.rules.len() > MAX_RULES {
        return Err(format!(
            "{} rules is over the limit of {MAX_RULES}; use wider networks",
            policy.rules.len()
        ));
    }
    for r in &policy.rules {
        if let Some(n) = &r.note {
            if n.len() > 200 {
                return Err("a rule note is limited to 200 characters".into());
            }
        }
    }
    compile(&policy.rules).map(|_| ())
}

// ---- explain --------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Verdict {
    pub dst: String,
    pub proto: EgressProto,
    pub port: u16,
    pub allowed: bool,
    /// The rule that grants it, when one does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<EgressRule>,
    pub reason: String,
    /// The one rule that would allow it, when it is refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<EgressRule>,
}

/// Would `policy` let a guest reach `ip:port` over `proto`?
pub fn explain(policy: &EgressPolicy, ip: Ipv4Addr, proto: EgressProto, port: u16) -> Verdict {
    let mut v = Verdict {
        dst: ip.to_string(),
        proto,
        port,
        allowed: false,
        rule: None,
        reason: String::new(),
        fix: None,
    };
    if ip.is_loopback() {
        v.allowed = true;
        v.reason = "loopback is always allowed".into();
        return v;
    }
    match policy.mode {
        EgressMode::Open => {
            v.allowed = true;
            v.reason = "the VM is open: every destination is allowed".into();
            return v;
        }
        EgressMode::Audit => {
            v.allowed = true;
            v.reason =
                "learn mode allows everything and records what an allow-list would refuse".into();
            return v;
        }
        EgressMode::Loopback | EgressMode::Allowlist => {}
    }
    let ipn = u32::from(ip);
    let entries = compile(&policy.rules).unwrap_or_default();
    let best = entries
        .iter()
        .filter(|e| covers((e.prefix, e.addr), (32, ipn)))
        .max_by_key(|e| e.prefix);
    let permitted = best.is_some_and(|e| match &e.ports {
        None => true,
        Some(p) => p
            .iter()
            .any(|(pp, port2)| *port2 == port && (*pp == EgressProto::Any || *pp == proto)),
    });
    if permitted {
        v.allowed = true;
        v.rule = policy
            .rules
            .iter()
            .filter(|r| {
                parse_cidr(&r.cidr).is_ok_and(|(a, l)| covers((l, a), (32, ipn)))
                    && r.port.is_none_or(|p| p == port)
                    && (r.proto == EgressProto::Any || r.proto == proto)
            })
            .max_by_key(|r| parse_cidr(&r.cidr).map(|(_, l)| l).unwrap_or(0))
            .cloned();
        v.reason = match &v.rule {
            Some(r) => format!("allowed by {}", describe_rule(r)),
            None => "allowed by a covering rule".into(),
        };
        return v;
    }
    v.reason = if policy.mode == EgressMode::Loopback {
        "the VM is isolated: only loopback is allowed".into()
    } else if best.is_some() {
        format!(
            "{} is allowed, but not on {} port {port}",
            best.map(|e| fmt_cidr(e.addr, e.prefix)).unwrap_or_default(),
            proto_name(proto)
        )
    } else {
        "no rule covers this destination".into()
    };
    let fix_proto = if proto == EgressProto::Any {
        EgressProto::Tcp
    } else {
        proto
    };
    let mut fix = EgressRule::new(
        &format!("{ip}/32"),
        fix_proto,
        Some(port),
        "added from explain",
    );
    // The fix must itself be valid once folded into the policy.
    let mut trial = policy.clone();
    trial.mode = EgressMode::Allowlist;
    trial.rules.push(fix.clone());
    if validate(&trial).is_err() {
        fix.port = None;
        fix.proto = EgressProto::Any;
    }
    v.fix = Some(fix);
    v
}

fn proto_name(p: EgressProto) -> &'static str {
    match p {
        EgressProto::Any => "any protocol",
        EgressProto::Tcp => "tcp",
        EgressProto::Udp => "udp",
    }
}

pub fn describe_rule(r: &EgressRule) -> String {
    match r.port {
        Some(p) => format!("{} {}/{}", r.cidr, proto_name(r.proto), p),
        None => format!("{} (all ports)", r.cidr),
    }
}

// ---- platform endpoints and presets ----------------------------------------------------

/// Where a guest can reach Veyron, as the egress filter will see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformEndpoints {
    /// Pod network CIDR. Cilium rewrites ClusterIP/NodePort to a pod address first.
    pub pod_cidr: String,
    pub node_ip: Option<String>,
    pub node_port: u16,
    pub cluster_ip: Option<String>,
    pub api_port: u16,
    pub cluster_dns: Option<String>,
    /// Where `pod_cidr` came from, so the console can say when it is a guess.
    pub pod_cidr_source: &'static str,
}

impl PlatformEndpoints {
    pub fn from_env(env: &dyn Fn(&str) -> Option<String>) -> Self {
        let get = |k: &str| {
            env(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let (pod_cidr, source) = match (get("VEYRON_POD_CIDR"), get("POD_IP")) {
            (Some(c), _) if parse_cidr(&c).is_ok() => (c, "VEYRON_POD_CIDR"),
            (_, Some(ip)) if ip.parse::<Ipv4Addr>().is_ok() => {
                let (a, _) = parse_cidr(&format!("{ip}/16")).unwrap_or((0, 16));
                (fmt_cidr(a, 16), "pod ip /16")
            }
            _ => ("10.0.0.0/8".to_string(), "default (private 10/8)"),
        };
        let valid_ip = |s: Option<String>| s.filter(|v| v.parse::<Ipv4Addr>().is_ok());
        Self {
            pod_cidr,
            node_ip: valid_ip(get("VEYRON_API_NODE_HOST")),
            node_port: get("VEYRON_API_NODE_PORT")
                .and_then(|p| p.parse().ok())
                .unwrap_or(30151),
            cluster_ip: valid_ip(
                get("VEYRON_API_CLUSTER_IP").or_else(|| get("VEYRON_API_SERVICE_HOST")),
            ),
            api_port: get("VEYRON_API_PORT")
                .and_then(|p| p.parse().ok())
                .unwrap_or(5151),
            cluster_dns: valid_ip(get("VEYRON_CLUSTER_DNS").or_else(|| Some("10.43.0.10".into()))),
            pod_cidr_source: source,
        }
    }

    /// Rules that let the guest agent reach Veyron: the pod network on the API port (where
    /// Cilium sends the traffic), the service and node addresses, and cluster DNS.
    pub fn rules(&self) -> Vec<EgressRule> {
        let mut r = vec![EgressRule::new(
            &self.pod_cidr,
            EgressProto::Tcp,
            Some(self.api_port),
            "Veyron API (pod network)",
        )];
        if let Some(ip) = &self.cluster_ip {
            r.push(EgressRule::new(
                &format!("{ip}/32"),
                EgressProto::Tcp,
                Some(443),
                "Veyron service",
            ));
            r.push(EgressRule::new(
                &format!("{ip}/32"),
                EgressProto::Tcp,
                Some(self.api_port),
                "Veyron service",
            ));
        }
        if let Some(ip) = &self.node_ip {
            r.push(EgressRule::new(
                &format!("{ip}/32"),
                EgressProto::Tcp,
                Some(self.node_port),
                "Veyron NodePort",
            ));
        }
        if let Some(ip) = &self.cluster_dns {
            r.push(EgressRule::new(
                &format!("{ip}/32"),
                EgressProto::Udp,
                Some(53),
                "cluster DNS",
            ));
            r.push(EgressRule::new(
                &format!("{ip}/32"),
                EgressProto::Tcp,
                Some(53),
                "cluster DNS",
            ));
        }
        r
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Preset {
    pub id: &'static str,
    pub title: &'static str,
    pub description: String,
    /// `none` | `low` | `medium` | `high`
    pub risk: &'static str,
    pub mode: EgressMode,
    pub rules: Vec<EgressRule>,
    /// Things the preset leans on, shown as chips ("Veyron API", "Canonical archive").
    pub needs: Vec<&'static str>,
}

impl Preset {
    pub fn policy(&self) -> EgressPolicy {
        EgressPolicy {
            mode: self.mode,
            rules: self.rules.clone(),
        }
    }
}

const CANONICAL: &[&str] = &["91.189.88.0/21", "185.125.188.0/22"];

fn is_ubuntu(template: Option<&str>) -> bool {
    template.is_some_and(|t| t.to_ascii_lowercase().starts_with("ubuntu"))
}

fn web_rules(ranges: &[String], note: &str) -> Vec<EgressRule> {
    let mut r = Vec::new();
    for c in ranges {
        r.push(EgressRule::new(c, EgressProto::Tcp, Some(80), note));
        r.push(EgressRule::new(c, EgressProto::Tcp, Some(443), note));
    }
    r.push(EgressRule::new(
        "0.0.0.0/0",
        EgressProto::Udp,
        Some(53),
        "DNS",
    ));
    r
}

/// All presets, resolved for this cluster. `template` picks the OS package sources.
pub fn presets(ep: &PlatformEndpoints, template: Option<&str>) -> Vec<Preset> {
    let platform = ep.rules();
    let join = |extra: Vec<EgressRule>| {
        let mut v = platform.clone();
        v.extend(extra);
        v
    };
    let (pkg_rules, pkg_desc, pkg_needs): (Vec<EgressRule>, String, Vec<&'static str>) =
        if is_ubuntu(template) {
            let ranges: Vec<String> = CANONICAL.iter().map(|s| s.to_string()).collect();
            (
                web_rules(&ranges, "Canonical archive"),
                "Veyron plus archive.ubuntu.com and security.ubuntu.com. Country mirrors \
                 (xx.archive.ubuntu.com) live elsewhere: use Web access or Learn mode for those."
                    .into(),
                vec!["Veyron API", "Canonical archive", "DNS"],
            )
        } else {
            (
                web_rules(&public_ranges(), "public web"),
                "Veyron plus the public internet on web ports, because this OS's mirrors sit \
                 on CDNs that have no stable address list."
                    .into(),
                vec!["Veyron API", "public web", "DNS"],
            )
        };
    vec![
        Preset {
            id: "isolated",
            title: "Isolated",
            description: "Nothing leaves the VM. The guest agent cannot connect either, so \
                          exec, files and metrics are unavailable."
                .into(),
            risk: "none",
            mode: EgressMode::Loopback,
            rules: vec![],
            needs: vec![],
        },
        Preset {
            id: "platform",
            title: "Platform only",
            description: "Only Veyron: the guest agent, exec, files and metrics work, and \
                          nothing else is reachable."
                .into(),
            risk: "low",
            mode: EgressMode::Allowlist,
            rules: platform.clone(),
            needs: vec!["Veyron API", "cluster DNS"],
        },
        Preset {
            id: "platform-packages",
            title: "Platform + package mirrors",
            description: pkg_desc,
            risk: "low",
            mode: EgressMode::Allowlist,
            rules: join(pkg_rules),
            needs: pkg_needs,
        },
        Preset {
            id: "web",
            title: "Web access",
            description: "Veyron plus any public address on 80 and 443, and DNS. Private \
                          networks stay unreachable."
                .into(),
            risk: "medium",
            mode: EgressMode::Allowlist,
            rules: join(web_rules(&public_ranges(), "public web")),
            needs: vec!["Veyron API", "public web", "DNS"],
        },
        Preset {
            id: "private",
            title: "Private networks",
            description: "Veyron plus your private networks (10/8, 172.16/12, 192.168/16) on \
                          every port. No internet."
                .into(),
            risk: "medium",
            mode: EgressMode::Allowlist,
            rules: join(vec![
                EgressRule::new("10.0.0.0/8", EgressProto::Any, None, "private"),
                EgressRule::new("172.16.0.0/12", EgressProto::Any, None, "private"),
                EgressRule::new("192.168.0.0/16", EgressProto::Any, None, "private"),
            ]),
            needs: vec!["Veyron API", "private networks"],
        },
        Preset {
            id: "learn",
            title: "Learn mode",
            description: "Everything is allowed and recorded. Run the workload, then turn \
                          what it reached into an allow-list with one click."
                .into(),
            risk: "medium",
            mode: EgressMode::Audit,
            rules: vec![],
            needs: vec![],
        },
        Preset {
            id: "open",
            title: "Open",
            description: "No restrictions. Use for trusted workloads, or for an hour with a \
                          timer."
                .into(),
            risk: "high",
            mode: EgressMode::Open,
            rules: vec![],
            needs: vec![],
        },
    ]
}

pub fn preset(id: &str, ep: &PlatformEndpoints, template: Option<&str>) -> Result<Preset, String> {
    presets(ep, template)
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| {
            let ids: Vec<_> = presets(ep, template).iter().map(|p| p.id).collect();
            format!(
                "unknown egress preset {id:?}; use one of {}",
                ids.join(", ")
            )
        })
}

/// The preset new VMs get: namespace annotation, then `VEYRON_VM_DEFAULT_EGRESS_PRESET`, then
/// the built-in. `none` (or `off`) means "do not set a policy; inherit the node default".
pub fn default_preset_id(
    annotations: &BTreeMap<String, String>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let v = annotations
        .get(ANN_DEFAULT_PRESET)
        .cloned()
        .or_else(|| env("VEYRON_VM_DEFAULT_EGRESS_PRESET"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| BUILTIN_DEFAULT_PRESET.to_string());
    match v.as_str() {
        "none" | "off" | "inherit" => None,
        _ => Some(v),
    }
}

/// What `POST /vms` accepts: a preset, explicit rules, or both (rules are added to the preset).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkAccessRequest {
    pub preset: Option<String>,
    pub mode: Option<EgressMode>,
    pub rules: Vec<EgressRule>,
}

impl NetworkAccessRequest {
    pub fn resolve(
        &self,
        ep: &PlatformEndpoints,
        template: Option<&str>,
    ) -> Result<EgressPolicy, String> {
        let mut policy = match &self.preset {
            Some(id) => preset(id, ep, template)?.policy(),
            None => EgressPolicy {
                mode: EgressMode::Allowlist,
                rules: vec![],
            },
        };
        for r in &self.rules {
            if !policy.rules.contains(r) {
                policy.rules.push(r.clone());
            }
        }
        if let Some(m) = self.mode {
            policy.mode = m;
        }
        validate(&policy)?;
        Ok(policy)
    }
}

// ---- policy for a new VM ---------------------------------------------------------------

/// Annotation carrying `network_access` from `POST /vms` to the backend (removed on create).
pub const ANN_NETWORK_ACCESS: &str = "veyron.io/network-access";
/// Annotation recording the template a VM came from (package presets depend on the OS).
pub const ANN_TEMPLATE: &str = "veyron.io/template";

/// The policy a new VM starts with. An explicit request always wins. Otherwise only a Linux
/// VM on user-mode networking gets the default preset: that is the case where the guest would
/// otherwise reach nothing, not even Veyron, so the agent could never connect. Bridged guests
/// (`tap`, `macvtap`), Windows, ISO installs and VMs of unknown origin are left to the node.
pub fn initial_policy(
    user_mode: bool,
    template: Option<&str>,
    explicit: Option<&NetworkAccessRequest>,
    ns_annotations: &BTreeMap<String, String>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<EgressPolicy>, String> {
    let ep = PlatformEndpoints::from_env(env);
    if let Some(req) = explicit {
        return req.resolve(&ep, template).map(Some);
    }
    let Some(t) = template else { return Ok(None) };
    if !user_mode || t.to_ascii_lowercase().starts_with("windows") {
        return Ok(None);
    }
    let Some(id) = default_preset_id(ns_annotations, env) else {
        return Ok(None);
    };
    // A misspelt default must not block VM creation: fall back to the built-in.
    let p = preset(&id, &ep, Some(t)).or_else(|_| preset(BUILTIN_DEFAULT_PRESET, &ep, Some(t)))?;
    Ok(Some(p.policy()))
}

/// Set `spec.network.egress` on a Machine about to be created (see [`initial_policy`]), and
/// strip the transient annotation. A policy already on the spec is kept.
pub fn apply_to_machine(
    machine: &mut crate::kairon::Machine,
    ns_annotations: &BTreeMap<String, String>,
) -> anyhow::Result<()> {
    let explicit_raw = machine
        .metadata
        .annotations
        .as_mut()
        .and_then(|a| a.remove(ANN_NETWORK_ACCESS));
    if machine
        .spec
        .network
        .as_ref()
        .is_some_and(|n| n.egress.is_some())
    {
        return Ok(());
    }
    let explicit: Option<NetworkAccessRequest> = explicit_raw
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?;
    let template = machine
        .metadata
        .annotations
        .as_ref()
        .and_then(|a| a.get(ANN_TEMPLATE))
        .cloned();
    let user_mode = machine
        .spec
        .network
        .as_ref()
        .and_then(|n| n.mode.as_deref())
        .is_none_or(|m| m == "user");
    let policy = initial_policy(
        user_mode,
        template.as_deref(),
        explicit.as_ref(),
        ns_annotations,
        &|k| std::env::var(k).ok().filter(|v| !v.is_empty()),
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    if let Some(p) = policy {
        machine
            .spec
            .network
            .get_or_insert_with(Default::default)
            .egress = Some(p);
    }
    Ok(())
}

// ---- fleet selectors and timers --------------------------------------------------------

/// Which VMs a bulk change applies to. At least one criterion is required unless `all`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Selector {
    pub all: bool,
    pub namespaces: Vec<String>,
    pub names: Vec<String>,
    pub labels: BTreeMap<String, String>,
}

impl Selector {
    pub fn is_empty(&self) -> bool {
        !self.all && self.namespaces.is_empty() && self.names.is_empty() && self.labels.is_empty()
    }

    /// Every given criterion must hold (namespaces, names and labels narrow each other).
    pub fn matches(&self, ns: &str, name: &str, labels: &BTreeMap<String, String>) -> bool {
        if self.is_empty() {
            return false;
        }
        (self.namespaces.is_empty() || self.namespaces.iter().any(|n| n == ns))
            && (self.names.is_empty() || self.names.iter().any(|n| n == name))
            && self.labels.iter().all(|(k, v)| labels.get(k) == Some(v))
    }
}

/// `"30m"`, `"2h"`, `"1d"`, `"90s"` or bare seconds, between one minute and thirty days.
pub fn parse_duration(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let (num, unit) = match s.char_indices().find(|(_, c)| !c.is_ascii_digit()) {
        Some((i, _)) => s.split_at(i),
        None => (s, "s"),
    };
    let n: u64 = num
        .parse()
        .map_err(|_| format!("{s:?} is not a duration; use 30m, 2h, 1d or seconds"))?;
    let secs = match unit.trim() {
        "s" | "" => n,
        "m" => n * 60,
        "h" => n * 3600,
        "d" => n * 86_400,
        _ => {
            return Err(format!(
                "{s:?} is not a duration; use 30m, 2h, 1d or seconds"
            ));
        }
    };
    if !(60..=30 * 86_400).contains(&secs) {
        return Err("a timed rule must last between 1 minute and 30 days".into());
    }
    Ok(secs)
}

// ---- diagnostics -----------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Problem {
    pub code: &'static str,
    /// `info` | `warn` | `error`
    pub severity: &'static str,
    pub message: String,
    /// A ready-made fix the console applies with one click: `{preset, mode, rules}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Value>,
}

pub struct ProblemInput<'a> {
    pub desired: &'a EgressPolicy,
    /// What the node reports it is enforcing (absent when the VM has no policy maps).
    pub programmed: Option<&'a EgressPolicy>,
    pub running: bool,
    pub live_known: bool,
    pub agent_connected: Option<bool>,
    pub blocked_flows: usize,
}

fn same_policy(a: &EgressPolicy, b: &EgressPolicy) -> bool {
    let norm = |p: &EgressPolicy| {
        let t: BTreeSet<(Vec<u8>, EgressProto, Option<u16>)> = compile(&p.rules)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|c| {
                let key = [&c.addr.to_be_bytes()[..], &[c.prefix]].concat();
                match c.ports {
                    None => vec![(key, EgressProto::Any, None)],
                    Some(ps) => ps
                        .into_iter()
                        .map(|(pr, po)| (key.clone(), pr, Some(po)))
                        .collect(),
                }
            })
            .collect();
        (p.mode, t)
    };
    norm(a) == norm(b)
}

pub fn detect_problems(
    i: &ProblemInput<'_>,
    ep: &PlatformEndpoints,
    template: Option<&str>,
) -> Vec<Problem> {
    let mut out = Vec::new();
    let api_ip = ep
        .pod_cidr
        .split('/')
        .next()
        .and_then(|s| s.parse::<Ipv4Addr>().ok());
    // The agent connects to the pod network on the API port; check the first usable address.
    let reaches_platform = api_ip.is_none_or(|ip| {
        let probe = Ipv4Addr::from(u32::from(ip).wrapping_add(1));
        explain(i.desired, probe, EgressProto::Tcp, ep.api_port).allowed
    });
    if i.agent_connected == Some(false)
        && matches!(i.desired.mode, EgressMode::Loopback | EgressMode::Allowlist)
        && !reaches_platform
    {
        let fix = preset(BUILTIN_DEFAULT_PRESET, ep, template)
            .ok()
            .map(|p| json!({"preset": p.id, "mode": p.mode, "rules": p.rules}));
        out.push(Problem {
            code: "agent_blocked",
            severity: "error",
            message:
                "The guest agent cannot connect: this VM's network access does not allow Veyron. \
                      Exec, files and metrics need it."
                    .into(),
            fix,
        });
    }
    if i.running && i.live_known {
        match i.programmed {
            None => out.push(Problem {
                code: "not_enforced",
                severity: "warn",
                message:
                    "The node is not enforcing a policy for this VM (it started before policy \
                          support). Restart the VM to apply it."
                        .into(),
                fix: None,
            }),
            Some(p) if !same_policy(p, i.desired) => out.push(Problem {
                code: "drift",
                severity: "warn",
                message: "The node is enforcing a different policy than the one saved for this VM."
                    .into(),
                fix: Some(json!({"mode": i.desired.mode, "rules": i.desired.rules})),
            }),
            _ => {}
        }
    }
    if i.blocked_flows > 0 {
        out.push(Problem {
            code: "blocked_flows",
            severity: "info",
            message: format!(
                "{} destination(s) were refused. Review the suggestions to allow the ones you expect.",
                i.blocked_flows
            ),
            fix: None,
        });
    }
    if i.desired.mode == EgressMode::Open {
        out.push(Problem {
            code: "open",
            severity: "warn",
            message:
                "This VM can reach anything. Prefer an allow-list, or open it for a limited time."
                    .into(),
            fix: None,
        });
    }
    out
}

/// Merge suggested rules into a policy as an allow-list (used by the "apply suggestions" step).
pub fn merge_suggestions(policy: &EgressPolicy, suggested: &[EgressRule]) -> EgressPolicy {
    let mut p = policy.clone();
    for r in suggested {
        if !p.rules.contains(r) {
            p.rules.push(r.clone());
        }
    }
    p.mode = EgressMode::Allowlist;
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| {
            pairs
                .iter()
                .find(|(a, _)| *a == k)
                .map(|(_, v)| v.to_string())
        }
    }

    fn ep() -> PlatformEndpoints {
        PlatformEndpoints::from_env(&env_of(&[
            ("POD_IP", "10.0.128.254"),
            ("VEYRON_API_NODE_HOST", "175.110.122.71"),
            ("VEYRON_API_CLUSTER_IP", "10.43.147.184"),
        ]))
    }

    fn rule(c: &str, p: EgressProto, port: Option<u16>) -> EgressRule {
        EgressRule::new(c, p, port, "")
    }

    #[test]
    fn cidr_parsing() {
        assert_eq!(parse_cidr("10.1.2.3/8").unwrap(), (0x0a00_0000, 8));
        assert_eq!(parse_cidr("1.2.3.4").unwrap(), (0x0102_0304, 32));
        assert!(parse_cidr("10.0.0.0/33").is_err());
        assert!(parse_cidr("fe80::/10").is_err());
        assert!(parse_cidr("x").is_err());
    }

    #[test]
    fn public_ranges_exclude_private_space() {
        let r = public_ranges();
        assert!(r.len() < 64, "{} ranges", r.len());
        let covered = |ip: &str| {
            let ip = u32::from(ip.parse::<Ipv4Addr>().unwrap());
            r.iter().any(|c| {
                let (a, l) = parse_cidr(c).unwrap();
                covers((l, a), (32, ip))
            })
        };
        for public in [
            "8.8.8.8",
            "91.189.91.82",
            "172.32.0.1",
            "172.15.255.255",
            "175.110.122.71",
            "100.63.255.255",
            "100.128.0.1",
            "223.255.255.255",
        ] {
            assert!(covered(public), "{public} should be public");
        }
        for private in [
            "10.1.1.1",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "127.0.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "224.0.0.1",
            "0.1.2.3",
        ] {
            assert!(!covered(private), "{private} should be excluded");
        }
    }

    #[test]
    fn validation_matches_the_node() {
        let ok = EgressPolicy {
            mode: EgressMode::Allowlist,
            rules: vec![rule("10.0.0.0/8", EgressProto::Tcp, Some(5151))],
        };
        assert!(validate(&ok).is_ok());
        let no_proto = EgressPolicy {
            mode: EgressMode::Allowlist,
            rules: vec![rule("10.0.0.0/8", EgressProto::Any, Some(443))],
        };
        assert!(
            validate(&no_proto)
                .unwrap_err()
                .contains("needs a protocol")
        );
        let zero = EgressPolicy {
            mode: EgressMode::Allowlist,
            rules: vec![rule("10.0.0.0/8", EgressProto::Tcp, Some(0))],
        };
        assert!(validate(&zero).is_err());
        let many = EgressPolicy {
            mode: EgressMode::Allowlist,
            rules: (1..=5)
                .map(|p| rule("10.0.0.0/8", EgressProto::Tcp, Some(8000 + p)))
                .collect(),
        };
        assert!(validate(&many).unwrap_err().contains("at most 4"));
    }

    #[test]
    fn folding_matches_longest_prefix_semantics() {
        let c = compile(&[
            rule("10.0.0.0/8", EgressProto::Tcp, Some(443)),
            rule("10.1.0.0/16", EgressProto::Tcp, Some(22)),
        ])
        .unwrap();
        let narrow = c.iter().find(|e| e.prefix == 16).unwrap();
        assert_eq!(
            narrow.ports,
            Some(vec![(EgressProto::Tcp, 22), (EgressProto::Tcp, 443)])
        );
        let policy = EgressPolicy {
            mode: EgressMode::Allowlist,
            rules: vec![
                rule("10.0.0.0/8", EgressProto::Tcp, Some(443)),
                rule("10.1.0.0/16", EgressProto::Tcp, Some(22)),
            ],
        };
        // The narrow rule must not hide the broad rule's port.
        assert!(explain(&policy, "10.1.2.3".parse().unwrap(), EgressProto::Tcp, 443).allowed);
        assert!(explain(&policy, "10.1.2.3".parse().unwrap(), EgressProto::Tcp, 22).allowed);
        assert!(!explain(&policy, "10.2.2.3".parse().unwrap(), EgressProto::Tcp, 22).allowed);
    }

    #[test]
    fn explain_names_the_rule_and_the_fix() {
        let p = EgressPolicy {
            mode: EgressMode::Allowlist,
            rules: vec![EgressRule::new(
                "10.0.0.0/8",
                EgressProto::Tcp,
                Some(5151),
                "Veyron API",
            )],
        };
        let ok = explain(&p, "10.0.128.254".parse().unwrap(), EgressProto::Tcp, 5151);
        assert!(ok.allowed);
        assert_eq!(ok.rule.unwrap().note.as_deref(), Some("Veyron API"));
        let no = explain(&p, "93.184.216.34".parse().unwrap(), EgressProto::Tcp, 443);
        assert!(!no.allowed);
        assert_eq!(no.fix.unwrap().cidr, "93.184.216.34/32");
        let wrong_port = explain(&p, "10.0.128.254".parse().unwrap(), EgressProto::Tcp, 22);
        assert!(!wrong_port.allowed);
        assert!(
            wrong_port.reason.contains("not on tcp port 22"),
            "{}",
            wrong_port.reason
        );
    }

    #[test]
    fn explain_modes() {
        let ip = "8.8.8.8".parse().unwrap();
        let mut p = EgressPolicy::default();
        assert!(!explain(&p, ip, EgressProto::Tcp, 443).allowed);
        assert!(explain(&p, "127.0.0.1".parse().unwrap(), EgressProto::Tcp, 1).allowed);
        p.mode = EgressMode::Open;
        assert!(explain(&p, ip, EgressProto::Tcp, 443).allowed);
        p.mode = EgressMode::Audit;
        assert!(explain(&p, ip, EgressProto::Tcp, 443).allowed);
    }

    #[test]
    fn explain_fix_widens_when_the_port_will_not_fit() {
        // 10.0.0.0/8 already carries four ports; a fifth on a /32 inside it must widen.
        let p = EgressPolicy {
            mode: EgressMode::Allowlist,
            rules: (1..=4)
                .map(|n| rule("10.0.0.0/8", EgressProto::Tcp, Some(8000 + n)))
                .collect(),
        };
        let v = explain(&p, "10.1.1.1".parse().unwrap(), EgressProto::Tcp, 9000);
        let fix = v.fix.unwrap();
        assert_eq!(fix.port, None);
    }

    #[test]
    fn every_preset_is_valid_for_both_private_and_public_node_addresses() {
        for node in ["175.110.122.71", "192.168.1.10"] {
            let ep = PlatformEndpoints::from_env(&|k| match k {
                "VEYRON_API_NODE_HOST" => Some(node.to_string()),
                "VEYRON_API_CLUSTER_IP" => Some("10.43.147.184".into()),
                "POD_IP" => Some("10.0.128.254".into()),
                _ => None,
            });
            for template in [None, Some("ubuntu-24.04"), Some("debian-13")] {
                for p in presets(&ep, template) {
                    validate(&p.policy())
                        .unwrap_or_else(|e| panic!("{} / {node} / {template:?}: {e}", p.id));
                    assert!(p.rules.len() <= MAX_RULES);
                }
            }
        }
    }

    #[test]
    fn platform_presets_let_the_agent_reach_the_pod_network() {
        let ep = ep();
        assert_eq!(ep.pod_cidr, "10.0.0.0/16");
        for id in ["platform", "platform-packages", "web", "private"] {
            let p = preset(id, &ep, Some("ubuntu-24.04")).unwrap().policy();
            // Cilium rewrites the NodePort to the pod address: that is what the filter sees.
            assert!(
                explain(&p, "10.0.128.254".parse().unwrap(), EgressProto::Tcp, 5151).allowed,
                "{id}"
            );
            assert!(
                explain(
                    &p,
                    "175.110.122.71".parse().unwrap(),
                    EgressProto::Tcp,
                    30151
                )
                .allowed,
                "{id}"
            );
        }
        let iso = preset("isolated", &ep, None).unwrap().policy();
        assert!(
            !explain(
                &iso,
                "10.0.128.254".parse().unwrap(),
                EgressProto::Tcp,
                5151
            )
            .allowed
        );
    }

    #[test]
    fn package_presets_are_template_aware() {
        let ep = ep();
        let ub = preset("platform-packages", &ep, Some("ubuntu-24.04"))
            .unwrap()
            .policy();
        assert!(explain(&ub, "91.189.91.82".parse().unwrap(), EgressProto::Tcp, 80).allowed);
        assert!(!explain(&ub, "8.8.8.8".parse().unwrap(), EgressProto::Tcp, 443).allowed);
        let deb = preset("platform-packages", &ep, Some("debian-13"))
            .unwrap()
            .policy();
        assert!(explain(&deb, "151.101.0.1".parse().unwrap(), EgressProto::Tcp, 443).allowed);
        assert!(!explain(&deb, "10.5.5.5".parse().unwrap(), EgressProto::Tcp, 443).allowed);
        assert!(!explain(&deb, "151.101.0.1".parse().unwrap(), EgressProto::Tcp, 22).allowed);
    }

    #[test]
    fn web_blocks_private_ranges_and_private_blocks_the_internet() {
        let ep = ep();
        let web = preset("web", &ep, None).unwrap().policy();
        assert!(!explain(&web, "192.168.1.1".parse().unwrap(), EgressProto::Tcp, 443).allowed);
        assert!(explain(&web, "8.8.8.8".parse().unwrap(), EgressProto::Tcp, 443).allowed);
        assert!(explain(&web, "9.9.9.9".parse().unwrap(), EgressProto::Udp, 53).allowed);
        let private = preset("private", &ep, None).unwrap().policy();
        assert!(
            explain(
                &private,
                "192.168.1.1".parse().unwrap(),
                EgressProto::Tcp,
                22
            )
            .allowed
        );
        assert!(!explain(&private, "8.8.8.8".parse().unwrap(), EgressProto::Tcp, 443).allowed);
    }

    #[test]
    fn unknown_preset_lists_the_valid_ones() {
        let e = preset("nope", &ep(), None).unwrap_err();
        assert!(
            e.contains("platform-packages") && e.contains("isolated"),
            "{e}"
        );
    }

    #[test]
    fn endpoints_fall_back_sensibly() {
        let none = PlatformEndpoints::from_env(&|_| None);
        assert_eq!(none.pod_cidr, "10.0.0.0/8");
        assert_eq!(none.node_port, 30151);
        assert!(none.node_ip.is_none());
        let explicit = PlatformEndpoints::from_env(&env_of(&[("VEYRON_POD_CIDR", "10.42.0.0/16")]));
        assert_eq!(explicit.pod_cidr, "10.42.0.0/16");
        let bad = PlatformEndpoints::from_env(&env_of(&[
            ("VEYRON_POD_CIDR", "junk"),
            ("POD_IP", "10.9.8.7"),
        ]));
        assert_eq!(bad.pod_cidr, "10.9.0.0/16");
    }

    #[test]
    fn default_preset_resolution() {
        let none = |_: &str| None;
        assert_eq!(
            default_preset_id(&BTreeMap::new(), &none).as_deref(),
            Some("platform-packages")
        );
        let ann = BTreeMap::from([(ANN_DEFAULT_PRESET.to_string(), "web".to_string())]);
        let env = |k: &str| (k == "VEYRON_VM_DEFAULT_EGRESS_PRESET").then(|| "open".to_string());
        assert_eq!(default_preset_id(&ann, &env).as_deref(), Some("web"));
        assert_eq!(
            default_preset_id(&BTreeMap::new(), &env).as_deref(),
            Some("open")
        );
        let off = |k: &str| (k == "VEYRON_VM_DEFAULT_EGRESS_PRESET").then(|| "none".to_string());
        assert_eq!(default_preset_id(&BTreeMap::new(), &off), None);
    }

    #[test]
    fn network_access_request_resolves_and_validates() {
        let ep = ep();
        let req = NetworkAccessRequest {
            preset: Some("platform".into()),
            mode: None,
            rules: vec![rule("203.0.113.0/24", EgressProto::Tcp, Some(8443))],
        };
        let p = req.resolve(&ep, None).unwrap();
        assert!(explain(&p, "203.0.113.9".parse().unwrap(), EgressProto::Tcp, 8443).allowed);
        let bad = NetworkAccessRequest {
            preset: Some("platform".into()),
            mode: None,
            rules: vec![rule("junk", EgressProto::Tcp, Some(1))],
        };
        assert!(bad.resolve(&ep, None).is_err());
        let rules_only = NetworkAccessRequest {
            preset: None,
            mode: None,
            rules: vec![rule("10.0.0.0/8", EgressProto::Tcp, Some(1))],
        };
        assert_eq!(
            rules_only.resolve(&ep, None).unwrap().mode,
            EgressMode::Allowlist
        );
        let json: NetworkAccessRequest = serde_json::from_value(json!({"preset": "web"})).unwrap();
        assert!(json.resolve(&ep, None).is_ok());
        assert!(serde_json::from_value::<NetworkAccessRequest>(json!({"presets": "web"})).is_err());
    }

    #[test]
    fn new_vms_get_the_default_only_when_it_is_the_case_that_needs_it() {
        let no_env = |_: &str| None;
        let none = BTreeMap::new();
        // Linux on user-mode networking: the default preset.
        let p = initial_policy(true, Some("ubuntu-24.04"), None, &none, &no_env)
            .unwrap()
            .unwrap();
        assert_eq!(p.mode, EgressMode::Allowlist);
        // Bridged, Windows, and unknown-origin VMs are left to the node.
        assert!(
            initial_policy(false, Some("ubuntu-24.04"), None, &none, &no_env)
                .unwrap()
                .is_none()
        );
        assert!(
            initial_policy(true, Some("windows-11"), None, &none, &no_env)
                .unwrap()
                .is_none()
        );
        assert!(
            initial_policy(true, None, None, &none, &no_env)
                .unwrap()
                .is_none()
        );
        // An explicit request wins, even for bridged VMs.
        let req = NetworkAccessRequest {
            preset: Some("isolated".into()),
            ..Default::default()
        };
        let p = initial_policy(false, None, Some(&req), &none, &no_env)
            .unwrap()
            .unwrap();
        assert_eq!(p.mode, EgressMode::Loopback);
        // "none" turns the default off; a misspelt default falls back instead of failing creation.
        let off = BTreeMap::from([(ANN_DEFAULT_PRESET.to_string(), "none".to_string())]);
        assert!(
            initial_policy(true, Some("ubuntu-24.04"), None, &off, &no_env)
                .unwrap()
                .is_none()
        );
        let typo = BTreeMap::from([(ANN_DEFAULT_PRESET.to_string(), "wbe".to_string())]);
        assert!(
            initial_policy(true, Some("ubuntu-24.04"), None, &typo, &no_env)
                .unwrap()
                .is_some()
        );
        // A bad explicit request is an error, not a silent default.
        let bad = NetworkAccessRequest {
            preset: Some("wbe".into()),
            ..Default::default()
        };
        assert!(initial_policy(true, None, Some(&bad), &none, &no_env).is_err());
    }

    #[test]
    fn apply_to_machine_sets_the_spec_and_strips_the_transient_annotation() {
        use crate::kairon::{Machine, MachineSpec};
        let mut m = Machine::new("vm", MachineSpec::default());
        m.metadata.annotations = Some(BTreeMap::from([
            (ANN_TEMPLATE.to_string(), "ubuntu-24.04".to_string()),
            (
                ANN_NETWORK_ACCESS.to_string(),
                r#"{"preset":"learn"}"#.to_string(),
            ),
        ]));
        apply_to_machine(&mut m, &BTreeMap::new()).unwrap();
        let egress = m.spec.network.as_ref().unwrap().egress.as_ref().unwrap();
        assert_eq!(egress.mode, EgressMode::Audit);
        assert!(
            !m.metadata
                .annotations
                .as_ref()
                .unwrap()
                .contains_key(ANN_NETWORK_ACCESS)
        );
        assert!(
            m.metadata
                .annotations
                .as_ref()
                .unwrap()
                .contains_key(ANN_TEMPLATE)
        );
        // A spec that already has a policy is kept.
        let before = m.spec.network.clone();
        apply_to_machine(&mut m, &BTreeMap::new()).unwrap();
        assert_eq!(m.spec.network, before);
        // Garbage in the annotation is an error rather than a silently open VM.
        let mut bad = Machine::new("vm2", MachineSpec::default());
        bad.metadata.annotations = Some(BTreeMap::from([(
            ANN_NETWORK_ACCESS.to_string(),
            "not json".to_string(),
        )]));
        assert!(apply_to_machine(&mut bad, &BTreeMap::new()).is_err());
    }

    #[test]
    fn selectors() {
        let labels = BTreeMap::from([("team".to_string(), "a".to_string())]);
        let none = Selector::default();
        assert!(
            !none.matches("ns", "vm", &labels),
            "empty selector matches nothing"
        );
        let all = Selector {
            all: true,
            ..Default::default()
        };
        assert!(all.matches("ns", "vm", &labels));
        let ns = Selector {
            namespaces: vec!["ns".into()],
            ..Default::default()
        };
        assert!(ns.matches("ns", "vm", &labels) && !ns.matches("other", "vm", &labels));
        let both = Selector {
            namespaces: vec!["ns".into()],
            labels: labels.clone(),
            ..Default::default()
        };
        assert!(both.matches("ns", "vm", &labels));
        assert!(!both.matches("ns", "vm", &BTreeMap::new()));
        let name = Selector {
            names: vec!["x".into()],
            ..Default::default()
        };
        assert!(!name.matches("ns", "vm", &labels));
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("1h").unwrap(), 3600);
        assert_eq!(parse_duration("90m").unwrap(), 5400);
        assert_eq!(parse_duration("600").unwrap(), 600);
        assert_eq!(parse_duration("1d").unwrap(), 86_400);
        assert!(parse_duration("10s").is_err());
        assert!(parse_duration("31d").is_err());
        assert!(parse_duration("soon").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn problems_surface_a_one_click_fix_for_a_blocked_agent() {
        let ep = ep();
        let desired = EgressPolicy::default();
        let probs = detect_problems(
            &ProblemInput {
                desired: &desired,
                programmed: Some(&desired),
                running: true,
                live_known: true,
                agent_connected: Some(false),
                blocked_flows: 3,
            },
            &ep,
            Some("ubuntu-24.04"),
        );
        let blocked = probs.iter().find(|p| p.code == "agent_blocked").unwrap();
        assert_eq!(blocked.fix.as_ref().unwrap()["preset"], "platform-packages");
        assert!(probs.iter().any(|p| p.code == "blocked_flows"));
        // A connected agent means no agent problem.
        let probs = detect_problems(
            &ProblemInput {
                desired: &desired,
                programmed: Some(&desired),
                running: true,
                live_known: true,
                agent_connected: Some(true),
                blocked_flows: 0,
            },
            &ep,
            None,
        );
        assert!(probs.is_empty(), "{probs:?}");
    }

    #[test]
    fn drift_and_unenforced_are_detected() {
        let ep = ep();
        let desired = preset("platform", &ep, None).unwrap().policy();
        let live = EgressPolicy::default();
        let mk = |programmed| {
            detect_problems(
                &ProblemInput {
                    desired: &desired,
                    programmed,
                    running: true,
                    live_known: true,
                    agent_connected: Some(true),
                    blocked_flows: 0,
                },
                &ep,
                None,
            )
        };
        assert!(mk(Some(&live)).iter().any(|p| p.code == "drift"));
        assert!(mk(None).iter().any(|p| p.code == "not_enforced"));
        assert!(mk(Some(&desired)).is_empty());
        // The same rules in a different order are not drift.
        let mut reordered = desired.clone();
        reordered.rules.reverse();
        assert!(mk(Some(&reordered)).is_empty());
        // A stopped VM or an unreachable node reports nothing it cannot know.
        let stopped = detect_problems(
            &ProblemInput {
                desired: &desired,
                programmed: None,
                running: false,
                live_known: true,
                agent_connected: None,
                blocked_flows: 0,
            },
            &ep,
            None,
        );
        assert!(stopped.is_empty());
    }

    #[test]
    fn merge_suggestions_switches_to_allowlist_without_duplicates() {
        let base = EgressPolicy {
            mode: EgressMode::Audit,
            rules: vec![rule("10.0.0.0/8", EgressProto::Tcp, Some(5151))],
        };
        let sugg = vec![
            rule("10.0.0.0/8", EgressProto::Tcp, Some(5151)),
            rule("1.2.3.4/32", EgressProto::Tcp, Some(443)),
        ];
        let m = merge_suggestions(&base, &sugg);
        assert_eq!(m.mode, EgressMode::Allowlist);
        assert_eq!(m.rules.len(), 2);
    }

    #[test]
    fn serde_roundtrip_matches_the_node_contract() {
        let p: EgressPolicy = serde_json::from_value(json!({
            "mode": "allowlist",
            "rules": [{"cidr": "10.0.0.0/8", "proto": "tcp", "port": 5151, "note": "api"}]
        }))
        .unwrap();
        assert_eq!(p.mode, EgressMode::Allowlist);
        assert_eq!(p.rules[0].proto, EgressProto::Tcp);
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["rules"][0]["port"], 5151);
        assert!(serde_json::from_value::<EgressPolicy>(json!({"mode": "loose"})).is_err());
    }
}
