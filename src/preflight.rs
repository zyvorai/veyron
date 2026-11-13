// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! `veyron preflight`: an offline check of the configuration `api-serve` will read.
//!
//! The checks are pure functions over a snapshot of the environment and a small file probe, so
//! they run without a cluster and are unit-tested without touching the process environment.
//! Secret values are never copied into a finding: messages name the variable and the problem only.

use serde::Serialize;
use std::collections::BTreeMap;

/// The lab default shipped by the deploy scripts and the bootstrap admin (`Admin@321`).
const DEFAULT_SECRET: &str = "Admin@321";
const MIN_KEY_LEN: usize = 16;
const MIN_JWT_SECRET_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: &'static str,
    pub status: Status,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn count(&self, s: Status) -> usize {
        self.findings.iter().filter(|f| f.status == s).count()
    }

    /// Process exit code: 1 on any failure, or on any warning when `strict`.
    pub fn exit_code(&self, strict: bool) -> i32 {
        if self.count(Status::Fail) > 0 || (strict && self.count(Status::Warn) > 0) {
            1
        } else {
            0
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for f in &self.findings {
            let tag = match f.status {
                Status::Ok => "ok  ",
                Status::Warn => "warn",
                Status::Fail => "FAIL",
            };
            out.push_str(&format!("[{tag}] {:<18} {}\n", f.id, f.message));
            if let Some(fix) = &f.fix {
                out.push_str(&format!("       {:<18} fix: {fix}\n", ""));
            }
        }
        out.push_str(&format!(
            "\n{} ok, {} warning(s), {} failure(s)\n",
            self.count(Status::Ok),
            self.count(Status::Warn),
            self.count(Status::Fail)
        ));
        out
    }
}

/// What a file probe found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileProbe {
    Missing,
    Unreadable,
    Text(String),
}

/// Everything the checks read. Build it with [`Inputs::from_process`] or by hand in tests.
pub struct Inputs {
    pub env: BTreeMap<String, String>,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
    pub probe: Box<dyn Fn(&str) -> FileProbe>,
}

impl Inputs {
    pub fn from_process(tls_cert: Option<String>, tls_key: Option<String>) -> Self {
        let env = std::env::vars()
            .filter(|(k, _)| {
                k.starts_with("VEYRON_") || k == "KUBECONFIG" || k == "KUBERNETES_SERVICE_HOST"
            })
            .collect();
        Self {
            env,
            tls_cert,
            tls_key,
            probe: Box::new(|p| match std::fs::read_to_string(p) {
                Ok(s) => FileProbe::Text(s),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileProbe::Missing,
                Err(_) => FileProbe::Unreadable,
            }),
        }
    }

    fn get(&self, k: &str) -> Option<&str> {
        self.env.get(k).map(|v| v.trim()).filter(|v| !v.is_empty())
    }
}

struct Out(Vec<Finding>);

impl Out {
    fn ok(&mut self, id: &'static str, msg: impl Into<String>) {
        self.push(id, Status::Ok, msg, None);
    }
    fn warn(&mut self, id: &'static str, msg: impl Into<String>, fix: impl Into<String>) {
        self.push(id, Status::Warn, msg, Some(fix.into()));
    }
    fn fail(&mut self, id: &'static str, msg: impl Into<String>, fix: impl Into<String>) {
        self.push(id, Status::Fail, msg, Some(fix.into()));
    }
    fn push(
        &mut self,
        id: &'static str,
        status: Status,
        msg: impl Into<String>,
        fix: Option<String>,
    ) {
        self.0.push(Finding {
            id,
            status,
            message: msg.into(),
            fix,
        });
    }
}

/// Run every check. Order is stable so output diffs cleanly.
pub fn run(inp: &Inputs) -> Report {
    let mut o = Out(Vec::new());
    check_api_key(inp, &mut o);
    check_api_keys(inp, &mut o);
    check_jwt(inp, &mut o);
    check_bootstrap(inp, &mut o);
    check_oidc(inp, &mut o);
    check_tls(inp, &mut o);
    check_namespace(inp, &mut o);
    check_urls(inp, &mut o);
    check_cors(inp, &mut o);
    check_webhook(inp, &mut o);
    check_insecure_flags(inp, &mut o);
    check_kubeconfig(inp, &mut o);
    Report { findings: o.0 }
}

fn weak(key: &str) -> Option<&'static str> {
    if key == DEFAULT_SECRET {
        Some("is the published lab default")
    } else if key.len() < MIN_KEY_LEN {
        Some("is shorter than 16 characters")
    } else {
        None
    }
}

fn check_api_key(inp: &Inputs, o: &mut Out) {
    match inp.get("VEYRON_API_KEY") {
        None => o.fail(
            "api-key",
            "VEYRON_API_KEY is not set, so the API rejects every request",
            "set VEYRON_API_KEY (Helm: auth.apiKey) to a random value of 16+ characters, e.g. `openssl rand -hex 24`",
        ),
        Some(k) => match weak(k) {
            Some(why) => o.warn(
                "api-key",
                format!("VEYRON_API_KEY {why}; it is the admin credential"),
                "replace it with a random value of 16+ characters",
            ),
            None => o.ok("api-key", "VEYRON_API_KEY is set and not obviously weak"),
        },
    }
}

/// One parsed `VEYRON_API_KEYS` entry, mirroring the server's parser (`name:key[:role]`).
struct KeyEntry {
    name: String,
    key: String,
    role: Option<String>,
}

fn parse_api_keys(raw: &str) -> (Vec<KeyEntry>, Vec<String>) {
    let mut ok = Vec::new();
    let mut skipped = Vec::new();
    for (i, entry) in raw.split(',').enumerate() {
        let parts: Vec<&str> = entry.trim().split(':').collect();
        if parts.len() >= 2 {
            ok.push(KeyEntry {
                name: parts[0].to_string(),
                key: parts[1].to_string(),
                role: parts.get(2).map(|s| s.to_string()),
            });
        } else {
            skipped.push(format!(
                "#{} ({})",
                i + 1,
                if entry.trim().is_empty() {
                    "empty"
                } else {
                    "no ':'"
                }
            ));
        }
    }
    (ok, skipped)
}

fn check_api_keys(inp: &Inputs, o: &mut Out) {
    let Some(raw) = inp.get("VEYRON_API_KEYS") else {
        o.ok(
            "api-keys",
            "VEYRON_API_KEYS is not set (only the primary key is used)",
        );
        return;
    };
    let (entries, skipped) = parse_api_keys(raw);
    let mut bad = false;
    if !skipped.is_empty() {
        bad = true;
        o.warn(
            "api-keys",
            format!(
                "entries without a key are silently ignored: {}",
                skipped.join(", ")
            ),
            "use the format name:key:role,name2:key2:role",
        );
    }
    let mut seen_names = BTreeMap::new();
    let mut seen_keys = BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        let label = format!("entry #{} ('{}')", i + 1, e.name);
        match e.role.as_deref() {
            Some("admin") | Some("write") | Some("readonly") | Some("read") => {}
            other => {
                bad = true;
                o.warn(
                    "api-keys",
                    format!(
                        "{label} has {} role and silently becomes readonly",
                        other
                            .map(|r| format!("unknown '{r}'"))
                            .unwrap_or_else(|| "no".into())
                    ),
                    "set the role to admin, write or readonly",
                );
            }
        }
        if let Some(why) = weak(&e.key) {
            bad = true;
            o.warn(
                "api-keys",
                format!("{label} key {why}"),
                "use a random value of 16+ characters",
            );
        }
        if e.name.is_empty() {
            bad = true;
            o.warn(
                "api-keys",
                format!("{label} has an empty name, so audit entries cannot tell its caller apart"),
                "give every key a name",
            );
        }
        if seen_names.insert(e.name.clone(), ()).is_some() && !e.name.is_empty() {
            bad = true;
            o.warn(
                "api-keys",
                format!("{label}: the name is used more than once, so audit entries are ambiguous"),
                "give every key a unique name",
            );
        }
        if seen_keys.insert(e.key.clone(), ()).is_some()
            || inp.get("VEYRON_API_KEY") == Some(e.key.as_str())
        {
            bad = true;
            o.warn("api-keys", format!("{label} repeats another key's value; the first match wins, so its role may not be the one you intended"), "give each key its own value");
        }
    }
    // A key containing ':' is cut at the first ':' by the parser.
    if raw.split(',').any(|e| e.trim().split(':').count() > 3) {
        bad = true;
        o.warn(
            "api-keys",
            "an entry has more than three ':'-separated parts; everything after the role is dropped (a key containing ':' is truncated)",
            "keys cannot contain ':' or ','; regenerate them as hex or base64url",
        );
    }
    if !bad {
        o.ok(
            "api-keys",
            format!(
                "{} extra key(s) parse cleanly with known roles",
                entries.len()
            ),
        );
    }
}

fn check_jwt(inp: &Inputs, o: &mut Out) {
    match (inp.get("VEYRON_JWT_SECRET"), inp.get("VEYRON_JWT_ISSUER")) {
        (None, None) => o.ok("jwt", "JWT login is not configured (console password login is off)"),
        (Some(_), None) | (None, Some(_)) => o.fail(
            "jwt",
            "only one of VEYRON_JWT_SECRET / VEYRON_JWT_ISSUER is set; JWT auth and console login stay disabled",
            "set both",
        ),
        (Some(s), Some(_)) => {
            if s.len() < MIN_JWT_SECRET_LEN || s == DEFAULT_SECRET {
                o.warn(
                    "jwt",
                    "VEYRON_JWT_SECRET is shorter than 32 characters; anyone who guesses it can mint admin tokens",
                    "use `openssl rand -hex 32`",
                );
            } else {
                o.ok("jwt", "JWT secret and issuer are set");
            }
        }
    }
}

fn check_bootstrap(inp: &Inputs, o: &mut Out) {
    let pw = inp
        .get("VEYRON_BOOTSTRAP_ADMIN_PASSWORD")
        .or_else(|| inp.get("VEYRON_API_KEY"));
    if inp.get("VEYRON_JWT_SECRET").is_some() && pw == Some(DEFAULT_SECRET) {
        o.warn(
            "bootstrap-admin",
            "the console bootstrap admin password is the published default",
            "set VEYRON_BOOTSTRAP_ADMIN_PASSWORD, or change the password after first login",
        );
    } else {
        o.ok(
            "bootstrap-admin",
            "no default bootstrap admin password in the environment",
        );
    }
}

fn check_oidc(inp: &Inputs, o: &mut Out) {
    let any = [
        "VEYRON_OIDC_CLIENT_ID",
        "VEYRON_OIDC_ISSUER",
        "VEYRON_OIDC_AUTHORIZATION_URL",
        "VEYRON_OIDC_JWKS_URL",
        "VEYRON_OIDC_USERINFO_URL",
    ]
    .iter()
    .any(|k| inp.get(k).is_some());
    if !any {
        o.ok("oidc", "OIDC is not configured");
        return;
    }
    let mut problems = Vec::new();
    if inp.get("VEYRON_OIDC_CLIENT_ID").is_none() {
        problems.push("VEYRON_OIDC_CLIENT_ID is missing (OIDC login stays off)");
    }
    if inp.get("VEYRON_OIDC_ISSUER").is_none() && inp.get("VEYRON_OIDC_AUTHORIZATION_URL").is_none()
    {
        problems.push("neither VEYRON_OIDC_ISSUER nor VEYRON_OIDC_AUTHORIZATION_URL is set");
    }
    if inp.get("VEYRON_OIDC_AUDIENCE").is_none() {
        problems.push("VEYRON_OIDC_AUDIENCE is unset, so the token audience is not checked");
    }
    if problems.is_empty() {
        o.ok("oidc", "OIDC client, issuer and audience are set");
    } else {
        o.warn("oidc", problems.join("; "), "see docs/sso.md");
    }
}

fn pem_has(text: &str, needles: &[&str]) -> bool {
    needles
        .iter()
        .any(|n| text.contains(&format!("-----BEGIN {n}-----")))
}

fn check_tls(inp: &Inputs, o: &mut Out) {
    match (&inp.tls_cert, &inp.tls_key) {
        (None, None) => {
            o.ok("tls", "no --tls-cert/--tls-key given (a self-signed certificate is generated when TLS is on)");
        }
        (Some(_), None) | (None, Some(_)) => o.fail(
            "tls",
            "only one of the TLS certificate and key is set",
            "pass both --tls-cert and --tls-key",
        ),
        (Some(c), Some(k)) => {
            let mut ok = true;
            for (what, path, needles) in [
                ("certificate", c, &["CERTIFICATE"][..]),
                (
                    "key",
                    k,
                    &["PRIVATE KEY", "RSA PRIVATE KEY", "EC PRIVATE KEY"][..],
                ),
            ] {
                match (inp.probe)(path) {
                    FileProbe::Missing => {
                        ok = false;
                        o.fail(
                            "tls",
                            format!("TLS {what} file '{path}' does not exist"),
                            "check the path and the volume mount",
                        );
                    }
                    FileProbe::Unreadable => {
                        ok = false;
                        o.fail(
                            "tls",
                            format!("TLS {what} file '{path}' is not readable"),
                            "check file permissions",
                        );
                    }
                    FileProbe::Text(t) if !pem_has(&t, needles) => {
                        ok = false;
                        o.fail(
                            "tls",
                            format!("TLS {what} file '{path}' is not PEM"),
                            "provide a PEM-encoded file",
                        );
                    }
                    FileProbe::Text(_) => {}
                }
            }
            if ok {
                o.ok(
                    "tls",
                    "TLS certificate and key files exist and look like PEM (expiry is not checked)",
                );
            }
        }
    }
}

fn dns_label(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 63
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

fn check_namespace(inp: &Inputs, o: &mut Out) {
    match inp.get("VEYRON_NAMESPACE") {
        None => o.ok(
            "namespace",
            "VEYRON_NAMESPACE is not set (the --namespace default applies)",
        ),
        Some(ns) if dns_label(ns) => o.ok(
            "namespace",
            format!("VEYRON_NAMESPACE '{ns}' is a valid namespace name"),
        ),
        Some(ns) => o.fail(
            "namespace",
            format!("VEYRON_NAMESPACE '{ns}' is not a valid Kubernetes namespace name"),
            "use lowercase letters, digits and '-' (max 63)",
        ),
    }
}

/// Integration endpoints that must be http(s) URLs; token pairs warn when sent in cleartext.
const URL_VARS: &[(&str, Option<&str>)] = &[
    ("VEYRON_PROMETHEUS_URL", None),
    ("VEYRON_GRAFANA_URL", None),
    ("VEYRON_ALERTMANAGER_URL", Some("VEYRON_ALERTMANAGER_TOKEN")),
    ("VEYRON_LOKI_URL", Some("VEYRON_LOKI_TOKEN")),
    ("VEYRON_ARGOCD_URL", Some("VEYRON_ARGOCD_TOKEN")),
    ("VEYRON_ATLAS_URL", Some("VEYRON_ATLAS_TOKEN")),
    ("VEYRON_KRYTON_URL", Some("VEYRON_KRYTON_TOKEN")),
    ("VEYRON_OPENCOST_URL", Some("VEYRON_OPENCOST_TOKEN")),
    ("VEYRON_ELASTIC_URL", Some("VEYRON_ELASTIC_API_KEY")),
    ("VEYRON_SPLUNK_HEC_URL", Some("VEYRON_SPLUNK_HEC_TOKEN")),
    ("VEYRON_SOAR_WEBHOOK_URL", None),
    ("VEYRON_SLACK_WEBHOOK_URL", None),
    (
        "VEYRON_KAIRON_NODE_URL",
        Some("VEYRON_KAIRON_CONSOLE_TOKEN"),
    ),
    ("VEYRON_AI_URL", Some("VEYRON_AI_API_KEY")),
];

fn url_scheme_host(u: &str) -> Option<(String, bool)> {
    let lower = u.trim().to_lowercase();
    let (scheme, rest) = lower.split_once("://")?;
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or("");
    let bare = host
        .trim_start_matches('[')
        .split([']', ':'])
        .next()
        .unwrap_or("");
    if bare.is_empty() {
        return None;
    }
    let local = bare == "localhost"
        || bare == "::1"
        || bare.starts_with("127.")
        || bare.ends_with(".svc")
        || bare.contains(".svc.")
        || !bare.contains('.');
    Some((scheme.to_string(), local))
}

fn check_urls(inp: &Inputs, o: &mut Out) {
    let mut configured = 0;
    let mut clean = true;
    for (var, token) in URL_VARS {
        let Some(v) = inp.get(var) else { continue };
        configured += 1;
        match url_scheme_host(v) {
            None => {
                clean = false;
                o.fail(
                    "urls",
                    format!("{var} is not a valid http(s) URL"),
                    "use e.g. https://host:port",
                );
            }
            Some((scheme, local)) => {
                if scheme == "http" && !local && token.is_some_and(|t| inp.get(t).is_some()) {
                    clean = false;
                    o.warn(
                        "urls",
                        format!("{var} uses plain http to a non-cluster host while a credential is set; it crosses the network in cleartext"),
                        "use https",
                    );
                }
            }
        }
    }
    if clean {
        o.ok(
            "urls",
            format!("{configured} integration URL(s) set, all valid"),
        );
    }
}

fn check_cors(inp: &Inputs, o: &mut Out) {
    match inp.get("VEYRON_CORS_ORIGINS") {
        None => o.ok("cors", "VEYRON_CORS_ORIGINS is not set (same-origin only)"),
        Some(v) if v.split(',').any(|s| s.trim() == "*") => o.warn(
            "cors",
            "VEYRON_CORS_ORIGINS contains '*', which is matched as a literal origin and is not a wildcard",
            "list the exact origins, e.g. https://console.example.com",
        ),
        Some(v) => {
            let bad: Vec<_> = v.split(',').map(str::trim).filter(|s| !s.is_empty() && url_scheme_host(s).is_none()).collect();
            if bad.is_empty() {
                o.ok("cors", "VEYRON_CORS_ORIGINS entries look like origins");
            } else {
                o.warn("cors", format!("{} CORS origin(s) are not http(s) origins and will never match", bad.len()), "use scheme://host[:port]");
            }
        }
    }
}

fn check_webhook(inp: &Inputs, o: &mut Out) {
    if inp.get("VEYRON_AI_PLAN_WEBHOOK_URL").is_some()
        && inp.get("VEYRON_AI_PLAN_WEBHOOK_SECRET").is_none()
    {
        o.warn(
            "plan-webhook",
            "VEYRON_AI_PLAN_WEBHOOK_URL is set without VEYRON_AI_PLAN_WEBHOOK_SECRET, so deliveries are unsigned",
            "set VEYRON_AI_PLAN_WEBHOOK_SECRET so receivers can verify the signature",
        );
    } else {
        o.ok("plan-webhook", "plan webhook is off or signed");
    }
}

fn flag_on(v: Option<&str>) -> bool {
    matches!(v, Some("1") | Some("true") | Some("yes"))
}

fn check_insecure_flags(inp: &Inputs, o: &mut Out) {
    let mut any = false;
    for (var, why) in [
        (
            "VEYRON_KAIRON_NODE_TLS_INSECURE",
            "disables TLS verification towards kairon-node",
        ),
        (
            "VEYRON_KUBEVIRT_WS_INSECURE_TLS",
            "disables TLS verification on the KubeVirt console WebSocket",
        ),
        (
            "VEYRON_IMAGE_STORE_INSECURE_TLS",
            "disables TLS verification for the image store",
        ),
        (
            "VEYRON_MCP_ALLOW_SELF_APPROVE",
            "lets a write key approve its own agent plans over MCP",
        ),
        (
            "VEYRON_ALLOW_PUBLIC_RDP",
            "allows RDP on NodePort/LoadBalancer",
        ),
    ] {
        if flag_on(inp.get(var)) {
            any = true;
            o.warn(
                "risk-flags",
                format!("{var} is on: {why}"),
                format!("unset {var} unless you need it"),
            );
        }
    }
    if !any {
        o.ok("risk-flags", "no risk-lowering flags are enabled");
    }
}

fn check_kubeconfig(inp: &Inputs, o: &mut Out) {
    if inp.get("KUBERNETES_SERVICE_HOST").is_some() {
        o.ok("kubeconfig", "running in a cluster (service account)");
        return;
    }
    let paths: Vec<String> = match inp.get("KUBECONFIG") {
        Some(v) => v
            .split(':')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
        None => dirs::home_dir()
            .map(|h| {
                vec![
                    h.join(".kube")
                        .join("config")
                        .to_string_lossy()
                        .into_owned(),
                ]
            })
            .unwrap_or_default(),
    };
    if paths
        .iter()
        .any(|p| matches!((inp.probe)(p), FileProbe::Text(_)))
    {
        o.ok("kubeconfig", "a kubeconfig file is readable");
    } else {
        o.warn(
            "kubeconfig",
            "no readable kubeconfig and not running in a cluster; commands that talk to Kubernetes will fail",
            "set KUBECONFIG or pass --kubeconfig",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(env: &[(&str, &str)]) -> Inputs {
        Inputs {
            env: env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            tls_cert: None,
            tls_key: None,
            probe: Box::new(|_| FileProbe::Missing),
        }
    }

    fn status(r: &Report, id: &str) -> Vec<Status> {
        r.findings
            .iter()
            .filter(|f| f.id == id)
            .map(|f| f.status)
            .collect()
    }

    const GOOD: &[(&str, &str)] = &[
        ("VEYRON_API_KEY", "0123456789abcdef0123456789abcdef"),
        ("KUBERNETES_SERVICE_HOST", "10.0.0.1"),
    ];

    #[test]
    fn clean_environment_passes() {
        let r = run(&inputs(GOOD));
        assert_eq!(r.count(Status::Fail), 0, "{}", r.to_text());
        assert_eq!(r.count(Status::Warn), 0, "{}", r.to_text());
        assert_eq!(r.exit_code(true), 0);
    }

    #[test]
    fn missing_primary_key_fails_and_default_warns() {
        let r = run(&inputs(&[]));
        assert_eq!(status(&r, "api-key"), vec![Status::Fail]);
        assert_eq!(r.exit_code(false), 1);
        let r = run(&inputs(&[("VEYRON_API_KEY", "Admin@321")]));
        assert_eq!(status(&r, "api-key"), vec![Status::Warn]);
        assert_eq!(r.exit_code(false), 0);
        assert_eq!(r.exit_code(true), 1);
    }

    #[test]
    fn secrets_are_never_echoed() {
        let secret = "s3cr3t-value-that-is-long-enough-xyz";
        let r = run(&inputs(&[
            ("VEYRON_API_KEY", "short"),
            (
                "VEYRON_API_KEYS",
                &format!("ci:{secret}:bogus,x:short:admin"),
            ),
            ("VEYRON_JWT_SECRET", "tiny"),
            ("VEYRON_JWT_ISSUER", "i"),
        ]));
        let text = r.to_text() + &serde_json::to_string(&r).unwrap();
        assert!(!text.contains(secret) && !text.contains("short:") && !text.contains("tiny"));
    }

    #[test]
    fn api_keys_parser_mismatches_are_reported() {
        let r = run(&inputs(&[
            ("VEYRON_API_KEY", "0123456789abcdef0123456789abcdef"),
            (
                "VEYRON_API_KEYS",
                "a:0123456789abcdef9999:bogus,,a:0123456789abcdef9999:admin,b",
            ),
        ]));
        let msgs: Vec<_> = r
            .findings
            .iter()
            .filter(|f| f.id == "api-keys")
            .map(|f| f.message.clone())
            .collect();
        assert!(
            msgs.iter().any(|m| m.contains("silently becomes readonly")),
            "{msgs:?}"
        );
        assert!(
            msgs.iter().any(|m| m.contains("silently ignored")),
            "{msgs:?}"
        );
        assert!(
            msgs.iter()
                .any(|m| m.contains("name is used more than once")),
            "{msgs:?}"
        );
        assert!(
            msgs.iter().any(|m| m.contains("repeats another key")),
            "{msgs:?}"
        );
    }

    #[test]
    fn key_with_colon_is_flagged_as_truncated() {
        let r = run(&inputs(&[(
            "VEYRON_API_KEYS",
            "a:0123456789:abcdef0123:admin",
        )]));
        assert!(
            r.findings
                .iter()
                .any(|f| f.id == "api-keys" && f.message.contains("truncated"))
        );
    }

    #[test]
    fn jwt_needs_both_halves() {
        assert_eq!(
            status(&run(&inputs(&[("VEYRON_JWT_SECRET", "x")])), "jwt"),
            vec![Status::Fail]
        );
        let long = "k".repeat(40);
        let r = run(&inputs(&[
            ("VEYRON_JWT_SECRET", &long),
            ("VEYRON_JWT_ISSUER", "veyron"),
        ]));
        assert_eq!(status(&r, "jwt"), vec![Status::Ok]);
    }

    #[test]
    fn default_bootstrap_password_only_matters_with_login() {
        let r = run(&inputs(&[("VEYRON_API_KEY", "Admin@321")]));
        assert_eq!(status(&r, "bootstrap-admin"), vec![Status::Ok]);
        let r = run(&inputs(&[
            ("VEYRON_API_KEY", "Admin@321"),
            ("VEYRON_JWT_SECRET", &"k".repeat(40)),
            ("VEYRON_JWT_ISSUER", "v"),
        ]));
        assert_eq!(status(&r, "bootstrap-admin"), vec![Status::Warn]);
    }

    #[test]
    fn oidc_partial_config_warns() {
        let r = run(&inputs(&[("VEYRON_OIDC_ISSUER", "https://idp")]));
        assert_eq!(status(&r, "oidc"), vec![Status::Warn]);
        let r = run(&inputs(&[
            ("VEYRON_OIDC_ISSUER", "https://idp"),
            ("VEYRON_OIDC_CLIENT_ID", "c"),
            ("VEYRON_OIDC_AUDIENCE", "veyron"),
        ]));
        assert_eq!(status(&r, "oidc"), vec![Status::Ok]);
    }

    #[test]
    fn tls_files_are_probed() {
        let mut i = inputs(GOOD);
        i.tls_cert = Some("/c.pem".into());
        assert_eq!(status(&run(&i), "tls"), vec![Status::Fail]); // key missing
        i.tls_key = Some("/k.pem".into());
        assert_eq!(status(&run(&i), "tls"), vec![Status::Fail, Status::Fail]); // both absent
        i.probe = Box::new(|p| {
            FileProbe::Text(if p == "/c.pem" {
                "-----BEGIN CERTIFICATE-----\nx\n".into()
            } else {
                "-----BEGIN PRIVATE KEY-----\nx\n".into()
            })
        });
        assert_eq!(status(&run(&i), "tls"), vec![Status::Ok]);
        i.probe = Box::new(|_| FileProbe::Text("garbage".into()));
        assert_eq!(status(&run(&i), "tls"), vec![Status::Fail, Status::Fail]);
    }

    #[test]
    fn namespace_validation() {
        assert_eq!(
            status(
                &run(&inputs(&[("VEYRON_NAMESPACE", "veyron-system")])),
                "namespace"
            ),
            vec![Status::Ok]
        );
        for bad in ["Veyron", "-a", "a_b", &"a".repeat(64)] {
            assert_eq!(
                status(&run(&inputs(&[("VEYRON_NAMESPACE", bad)])), "namespace"),
                vec![Status::Fail],
                "{bad}"
            );
        }
    }

    #[test]
    fn urls_are_validated_and_cleartext_credentials_flagged() {
        let r = run(&inputs(&[("VEYRON_LOKI_URL", "loki.example.com")]));
        assert_eq!(status(&r, "urls"), vec![Status::Fail]);
        let r = run(&inputs(&[
            ("VEYRON_LOKI_URL", "http://loki.example.com:3100"),
            ("VEYRON_LOKI_TOKEN", "t"),
        ]));
        assert_eq!(status(&r, "urls"), vec![Status::Warn]);
        // in-cluster http and https are fine; no token means nothing to leak
        let r = run(&inputs(&[
            ("VEYRON_LOKI_URL", "http://loki.monitoring.svc:3100"),
            ("VEYRON_LOKI_TOKEN", "t"),
            ("VEYRON_GRAFANA_URL", "http://grafana.example.com"),
            ("VEYRON_ARGOCD_URL", "https://argo.example.com"),
            ("VEYRON_ARGOCD_TOKEN", "t"),
        ]));
        assert_eq!(status(&r, "urls"), vec![Status::Ok]);
    }

    #[test]
    fn userinfo_in_url_does_not_confuse_host_detection() {
        let (s, local) = url_scheme_host("http://user:pw@evil.example.com/x").unwrap();
        assert_eq!(s, "http");
        assert!(!local);
    }

    #[test]
    fn cors_wildcard_warns() {
        let r = run(&inputs(&[(
            "VEYRON_CORS_ORIGINS",
            "https://a.example.com, *",
        )]));
        assert_eq!(status(&r, "cors"), vec![Status::Warn]);
        let r = run(&inputs(&[("VEYRON_CORS_ORIGINS", "https://a.example.com")]));
        assert_eq!(status(&r, "cors"), vec![Status::Ok]);
    }

    #[test]
    fn unsigned_webhook_warns() {
        let r = run(&inputs(&[(
            "VEYRON_AI_PLAN_WEBHOOK_URL",
            "https://h.example.com",
        )]));
        assert_eq!(status(&r, "plan-webhook"), vec![Status::Warn]);
        let r = run(&inputs(&[
            ("VEYRON_AI_PLAN_WEBHOOK_URL", "https://h"),
            ("VEYRON_AI_PLAN_WEBHOOK_SECRET", "s"),
        ]));
        assert_eq!(status(&r, "plan-webhook"), vec![Status::Ok]);
    }

    #[test]
    fn risk_flags_follow_the_servers_truthiness() {
        let r = run(&inputs(&[
            ("VEYRON_ALLOW_PUBLIC_RDP", "yes"),
            ("VEYRON_KAIRON_NODE_TLS_INSECURE", "true"),
        ]));
        assert_eq!(status(&r, "risk-flags"), vec![Status::Warn, Status::Warn]);
        let r = run(&inputs(&[("VEYRON_ALLOW_PUBLIC_RDP", "0")]));
        assert_eq!(status(&r, "risk-flags"), vec![Status::Ok]);
    }

    #[test]
    fn kubeconfig_probe() {
        let mut i = inputs(&[("KUBECONFIG", "/a:/b")]);
        assert_eq!(status(&run(&i), "kubeconfig"), vec![Status::Warn]);
        i.probe = Box::new(|p| {
            if p == "/b" {
                FileProbe::Text("x".into())
            } else {
                FileProbe::Missing
            }
        });
        assert_eq!(status(&run(&i), "kubeconfig"), vec![Status::Ok]);
    }

    #[test]
    fn json_report_shape() {
        let r = run(&inputs(&[]));
        let v: serde_json::Value = serde_json::to_value(&r).unwrap();
        assert!(v["findings"][0]["id"].is_string());
        assert_eq!(v["findings"][0]["status"], "fail");
    }
}
