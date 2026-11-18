// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! `POST /imports/validate`: dry-run preflight for a VM import.
//!
//! Everything here is a pure function over the request plus an optional [`ProbeOutcome`]; the
//! only I/O is the [`ImageProbe`] implementation, which tests replace with a fake. The checks
//! mirror `machine_for_import` but collect every problem instead of stopping at the first, and
//! add policy checks (URL host, credentials in the URL) and warnings that creation does not
//! enforce.

use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::imports::{FORMATS, normalize_digest, valid_name};

/// Request body. Every field is optional so a missing one becomes a reported error rather
/// than a 422 from the JSON extractor.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ValidateRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub cpus: Option<u32>,
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(default)]
    pub secure_boot: bool,
    #[serde(default)]
    pub tpm: bool,
    /// Guest OS hint such as `windows-2022`, `windows-11`, `ubuntu-24.04`. Optional.
    #[serde(default)]
    pub guest_os: Option<String>,
    /// Source VM name, used only for the Windows hint when `guest_os` is absent.
    #[serde(default)]
    pub source: Option<ValidateSource>,
    /// Ask Veyron to `HEAD` the URL (size estimate, reachability from Veyron, not the node).
    #[serde(default)]
    pub probe: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ValidateSource {
    #[serde(default)]
    pub vm: Option<String>,
}

/// What Veyron knows about the cluster that the request cannot tell it.
#[derive(Debug, Clone, Default)]
pub struct ClusterFacts {
    /// `KAIRON_IMAGE_CACHE_DIR` is set in Veyron's environment (best effort: Veyron cannot
    /// read the nodes' own settings).
    pub image_cache_configured: bool,
}

impl ClusterFacts {
    pub fn from_env() -> Self {
        Self {
            image_cache_configured: std::env::var("KAIRON_IMAGE_CACHE_DIR")
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub field: &'static str,
    pub code: &'static str,
    pub message: String,
}

fn issue(field: &'static str, code: &'static str, message: impl Into<String>) -> Issue {
    Issue {
        field,
        code,
        message: message.into(),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProbeOutcome {
    pub reachable: bool,
    pub status: Option<u16>,
    pub content_length: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Normalized {
    pub name: Option<String>,
    pub namespace: Option<String>,
    pub image_url: Option<String>,
    pub sha256: Option<String>,
    pub format: String,
    pub cpus: u32,
    pub memory: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidateReport {
    pub valid: bool,
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
    pub normalized: Normalized,
    /// Present only when a probe ran.
    pub probe: Option<ProbeOutcome>,
    /// Image size in bytes when the probe reported one.
    pub size_bytes: Option<u64>,
}

/// How a URL host is treated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostClass {
    Public,
    /// RFC 1918 / ULA: allowed (h2kvm serves from the LAN) but worth a note.
    Private,
    /// Never allowed: the node cannot meaningfully reach it, or it is a metadata endpoint.
    Blocked(&'static str),
}

pub fn classify_ip(ip: &IpAddr) -> HostClass {
    match ip {
        IpAddr::V4(v4) => {
            if v4.is_loopback() {
                HostClass::Blocked("loopback address")
            } else if v4.is_unspecified() {
                HostClass::Blocked("unspecified address")
            } else if v4.is_link_local() {
                HostClass::Blocked("link-local address (cloud metadata range)")
            } else if v4.is_multicast() || v4.is_broadcast() {
                HostClass::Blocked("multicast or broadcast address")
            } else if v4.is_private() {
                HostClass::Private
            } else {
                HostClass::Public
            }
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return classify_ip(&IpAddr::V4(v4));
            }
            let seg0 = v6.segments()[0];
            if v6.is_loopback() {
                HostClass::Blocked("loopback address")
            } else if v6.is_unspecified() {
                HostClass::Blocked("unspecified address")
            } else if (seg0 & 0xffc0) == 0xfe80 {
                HostClass::Blocked("link-local address")
            } else if v6.is_multicast() {
                HostClass::Blocked("multicast address")
            } else if (seg0 & 0xfe00) == 0xfc00 {
                HostClass::Private
            } else {
                HostClass::Public
            }
        }
    }
}

pub fn classify_host(host: &str) -> HostClass {
    let h = host.trim_matches(|c| c == '[' || c == ']');
    if let Ok(ip) = h.parse::<IpAddr>() {
        return classify_ip(&ip);
    }
    let lower = h.to_ascii_lowercase();
    let lower = lower.trim_end_matches('.');
    if lower == "localhost" || lower.ends_with(".localhost") || lower == "metadata.google.internal"
    {
        return HostClass::Blocked("loopback or metadata host name");
    }
    HostClass::Public
}

/// Parsed, policy-checked view of an `image_url`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlCheck {
    pub scheme: String,
    pub host: String,
    pub class: HostClass,
}

/// Parse `image_url` and apply the scheme/host/credential policy. `Err` is the error message.
pub fn check_url(raw: &str) -> Result<UrlCheck, String> {
    let raw = raw.trim();
    let url = reqwest::Url::parse(raw).map_err(|e| format!("image_url is not a valid URL: {e}"))?;
    let scheme = url.scheme().to_string();
    if scheme != "http" && scheme != "https" {
        return Err("image_url must be an http:// or https:// URL".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(
            "image_url must not embed credentials (they would be stored in an annotation)".into(),
        );
    }
    let host = url
        .host_str()
        .filter(|h| !h.is_empty())
        .ok_or("image_url has no host")?
        .to_string();
    let class = classify_host(&host);
    if let HostClass::Blocked(why) = &class {
        return Err(format!("image_url host {host:?} is not allowed: {why}"));
    }
    Ok(UrlCheck {
        scheme,
        host,
        class,
    })
}

fn is_windows(req: &ValidateRequest) -> bool {
    let hay = |s: &Option<String>| {
        s.as_deref()
            .map(|v| v.to_ascii_lowercase().contains("win"))
            .unwrap_or(false)
    };
    hay(&req.guest_os)
        || (req.guest_os.is_none() && hay(&req.source.as_ref().and_then(|s| s.vm.clone())))
}

fn needs_secure_boot_tpm(req: &ValidateRequest) -> bool {
    let os = req.guest_os.as_deref().unwrap_or("").to_ascii_lowercase();
    ["11", "2022", "2025"].iter().any(|v| os.contains(v))
}

/// The URL to probe, if the request has one that passes policy.
pub fn probe_target(req: &ValidateRequest) -> Option<String> {
    let url = req.image_url.as_deref()?.trim();
    check_url(url).ok().map(|_| url.to_string())
}

/// Pure validation. `probe` is the outcome of probing [`probe_target`], if one ran.
pub fn validate_import(
    req: &ValidateRequest,
    facts: &ClusterFacts,
    probe: Option<&ProbeOutcome>,
) -> ValidateReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut norm = Normalized::default();

    match req.name.as_deref() {
        None | Some("") => errors.push(issue("name", "REQUIRED", "name is required")),
        Some(n) if !valid_name(n) => errors.push(issue(
            "name",
            "INVALID_NAME",
            format!("name {n:?} must be a DNS-1123 label"),
        )),
        Some(n) => norm.name = Some(n.to_string()),
    }
    norm.namespace = req.namespace.clone().filter(|n| !n.is_empty());

    let mut check = None;
    match req.image_url.as_deref().map(str::trim) {
        None | Some("") => errors.push(issue("image_url", "REQUIRED", "image_url is required")),
        Some(u) => match check_url(u) {
            Ok(c) => {
                norm.image_url = Some(u.to_string());
                check = Some(c);
            }
            Err(e) => errors.push(issue("image_url", "INVALID_URL", e)),
        },
    }

    match req.sha256.as_deref() {
        None | Some("") => errors.push(issue("sha256", "REQUIRED", "sha256 is required")),
        Some(s) => match normalize_digest(s) {
            Some(d) => norm.sha256 = Some(d),
            None => errors.push(issue(
                "sha256",
                "INVALID_SHA256",
                "sha256 must be 64 hex characters, optionally prefixed with sha256:",
            )),
        },
    }

    norm.format = req
        .format
        .as_deref()
        .unwrap_or("qcow2")
        .to_ascii_lowercase();
    if !FORMATS.contains(&norm.format.as_str()) {
        errors.push(issue(
            "format",
            "INVALID_FORMAT",
            format!("format must be one of {}", FORMATS.join(", ")),
        ));
    }

    norm.cpus = req.cpus.unwrap_or(2);
    if !(1..=256).contains(&norm.cpus) {
        errors.push(issue(
            "cpus",
            "INVALID_CPUS",
            "cpus must be between 1 and 256",
        ));
    }
    norm.memory = req.memory.clone().unwrap_or_else(|| "4Gi".into());
    if norm.memory.parse::<u64>().is_ok() || !norm.memory.starts_with(|c: char| c.is_ascii_digit())
    {
        errors.push(issue(
            "memory",
            "INVALID_MEMORY",
            format!("memory {:?} must be a quantity such as 4Gi", norm.memory),
        ));
    }

    // Warnings.
    if let Some(c) = &check {
        if c.scheme == "http" {
            warnings.push(issue(
                "image_url",
                "PLAINTEXT_HTTP",
                "image_url uses http://; the SHA-256 guards the bytes but the transfer is unencrypted",
            ));
        }
        if c.class == HostClass::Private {
            warnings.push(issue(
                "image_url",
                "PRIVATE_HOST",
                format!(
                    "{} is a private address; it must be reachable from the Kairon node, not just from Veyron",
                    c.host
                ),
            ));
        }
    }
    if matches!(norm.format.as_str(), "ova" | "vmdk" | "vhd" | "vhdx") {
        warnings.push(issue(
            "format",
            "CONVERSION_ON_NODE",
            format!(
                "{} is converted on the node: expect extra scratch space and a longer first boot (h2kvm output is already qcow2)",
                norm.format
            ),
        ));
    }
    if is_windows(req) {
        if needs_secure_boot_tpm(req) && !(req.secure_boot && req.tpm) {
            warnings.push(issue(
                "secure_boot",
                "WINDOWS_SECURE_BOOT_TPM",
                "Windows 11 and Windows Server 2022 or later expect secure_boot and tpm to be enabled",
            ));
        } else if req.guest_os.is_none() && !(req.secure_boot && req.tpm) {
            warnings.push(issue(
                "secure_boot",
                "WINDOWS_SECURE_BOOT_TPM_HINT",
                "this looks like a Windows guest; set secure_boot and tpm if it is Windows 11 or Server 2022+ (set guest_os to silence this)",
            ));
        }
    }
    if !facts.image_cache_configured {
        warnings.push(issue(
            "image_url",
            "IMAGE_CACHE_UNCONFIRMED",
            "KAIRON_IMAGE_CACHE_DIR is not set in Veyron's environment; Veyron cannot confirm the nodes have an image cache, and imports need one",
        ));
    }

    let mut size_bytes = None;
    if let Some(p) = probe {
        if !p.reachable {
            warnings.push(issue(
                "image_url",
                "PROBE_UNREACHABLE",
                format!(
                    "Veyron could not fetch the URL ({}); the node may still reach it",
                    p.error
                        .clone()
                        .or_else(|| p.status.map(|s| format!("HTTP {s}")))
                        .unwrap_or_else(|| "no response".into())
                ),
            ));
        } else {
            size_bytes = p.content_length;
            if p.content_length.is_none() {
                warnings.push(issue(
                    "image_url",
                    "SIZE_UNKNOWN",
                    "the server did not report a Content-Length",
                ));
            }
            if matches!(p.status, Some(300..=399)) {
                warnings.push(issue(
                    "image_url",
                    "PROBE_REDIRECT",
                    "the URL redirects; Veyron did not follow it, so size is unknown",
                ));
            }
        }
    }

    ValidateReport {
        valid: errors.is_empty(),
        errors,
        warnings,
        normalized: norm,
        probe: probe.cloned(),
        size_bytes,
    }
}

/// Network probe, injectable so tests never touch the network.
pub trait ImageProbe: Send + Sync {
    fn probe<'a>(&'a self, url: &'a str)
    -> Pin<Box<dyn Future<Output = ProbeOutcome> + Send + 'a>>;
}

/// Real probe: resolves the host, refuses blocked addresses, pins the connection to the
/// resolved address (no DNS rebinding), does not follow redirects, 8 s timeout, `HEAD` only.
pub struct HttpProbe;

impl ImageProbe for HttpProbe {
    fn probe<'a>(
        &'a self,
        url: &'a str,
    ) -> Pin<Box<dyn Future<Output = ProbeOutcome> + Send + 'a>> {
        Box::pin(async move {
            let fail = |e: String| ProbeOutcome {
                reachable: false,
                status: None,
                content_length: None,
                error: Some(e),
            };
            let Ok(parsed) = reqwest::Url::parse(url) else {
                return fail("invalid URL".into());
            };
            let Some(host) = parsed.host_str().map(str::to_string) else {
                return fail("no host".into());
            };
            let port = parsed.port_or_known_default().unwrap_or(80);
            let addrs: Vec<_> = match tokio::net::lookup_host((host.as_str(), port)).await {
                Ok(a) => a.collect(),
                Err(e) => return fail(format!("DNS lookup failed: {e}")),
            };
            let Some(addr) = addrs.first().copied() else {
                return fail("host did not resolve".into());
            };
            if let Some(bad) = addrs.iter().find_map(|a| match classify_ip(&a.ip()) {
                HostClass::Blocked(why) => Some(why),
                _ => None,
            }) {
                return fail(format!("host resolves to a blocked address: {bad}"));
            }
            let client = match reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(8))
                .resolve(&host, addr)
                .build()
            {
                Ok(c) => c,
                Err(e) => return fail(e.to_string()),
            };
            match client.head(url).send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let content_length = resp
                        .headers()
                        .get(reqwest::header::CONTENT_LENGTH)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok());
                    ProbeOutcome {
                        reachable: status < 400 || status == 405,
                        status: Some(status),
                        content_length,
                        error: None,
                    }
                }
                Err(e) => fail(e.to_string()),
            }
        })
    }
}

/// Validate and, when `req.probe` is set, probe through `prober`.
pub async fn run_validate(
    req: &ValidateRequest,
    facts: &ClusterFacts,
    prober: &dyn ImageProbe,
) -> ValidateReport {
    let outcome = match (req.probe, probe_target(req)) {
        (true, Some(url)) => Some(prober.probe(&url).await),
        _ => None,
    };
    validate_import(req, facts, outcome.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn req() -> ValidateRequest {
        serde_json::from_value(json!({
            "name": "web01",
            "image_url": "https://files.example/web01.qcow2",
            "sha256": "ab".repeat(32),
        }))
        .unwrap()
    }

    fn facts() -> ClusterFacts {
        ClusterFacts {
            image_cache_configured: true,
        }
    }

    fn codes(v: &[Issue]) -> Vec<&'static str> {
        v.iter().map(|i| i.code).collect()
    }

    struct Fake(ProbeOutcome);
    impl ImageProbe for Fake {
        fn probe<'a>(
            &'a self,
            _url: &'a str,
        ) -> Pin<Box<dyn Future<Output = ProbeOutcome> + Send + 'a>> {
            let o = self.0.clone();
            Box::pin(async move { o })
        }
    }

    #[test]
    fn clean_request_is_valid_and_normalized() {
        let r = validate_import(&req(), &facts(), None);
        assert!(r.valid, "{:?}", r.errors);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(r.normalized.format, "qcow2");
        assert_eq!(r.normalized.cpus, 2);
        assert_eq!(r.normalized.memory, "4Gi");
        assert_eq!(
            r.normalized.sha256.as_deref(),
            Some(format!("sha256:{}", "ab".repeat(32)).as_str())
        );
    }

    #[test]
    fn collects_every_error_not_just_the_first() {
        let r: ValidateRequest = serde_json::from_value(json!({
            "name": "Web_01", "image_url": "ftp://x/y", "sha256": "dead",
            "format": "iso", "cpus": 0, "memory": "4096"
        }))
        .unwrap();
        let v = validate_import(&r, &facts(), None);
        assert!(!v.valid);
        assert_eq!(
            codes(&v.errors),
            [
                "INVALID_NAME",
                "INVALID_URL",
                "INVALID_SHA256",
                "INVALID_FORMAT",
                "INVALID_CPUS",
                "INVALID_MEMORY"
            ]
        );
    }

    #[test]
    fn empty_body_reports_required_fields() {
        let v = validate_import(&ValidateRequest::default(), &facts(), None);
        assert_eq!(codes(&v.errors), ["REQUIRED", "REQUIRED", "REQUIRED"]);
    }

    #[test]
    fn url_policy() {
        for bad in [
            "http://127.0.0.1/x.qcow2",
            "http://localhost:8099/x",
            "http://169.254.169.254/latest",
            "http://[::1]/x",
            "http://[fe80::1]/x",
            "http://[::ffff:127.0.0.1]/x",
            "http://0.0.0.0/x",
            "https://user:pw@files.example/x",
            "file:///tmp/x",
        ] {
            assert!(check_url(bad).is_err(), "{bad} should be rejected");
        }
        assert_eq!(
            check_url("http://10.0.0.5:8099/a").unwrap().class,
            HostClass::Private
        );
        assert_eq!(
            check_url("https://files.example/a").unwrap().class,
            HostClass::Public
        );
    }

    #[test]
    fn warns_on_http_private_and_conversion() {
        let mut r = req();
        r.image_url = Some("http://10.0.0.5:8099/a.vmdk".into());
        r.format = Some("VMDK".into());
        let v = validate_import(&r, &facts(), None);
        assert!(v.valid);
        assert_eq!(
            codes(&v.warnings),
            ["PLAINTEXT_HTTP", "PRIVATE_HOST", "CONVERSION_ON_NODE"]
        );
    }

    #[test]
    fn windows_hints() {
        let mut r = req();
        r.guest_os = Some("windows-2022".into());
        assert_eq!(
            codes(&validate_import(&r, &facts(), None).warnings),
            ["WINDOWS_SECURE_BOOT_TPM"]
        );
        r.secure_boot = true;
        r.tpm = true;
        assert!(validate_import(&r, &facts(), None).warnings.is_empty());
        // Older Windows with an explicit guest_os needs no hint.
        r.secure_boot = false;
        r.tpm = false;
        r.guest_os = Some("windows-2019".into());
        assert!(validate_import(&r, &facts(), None).warnings.is_empty());
        // Name heuristic only when guest_os is absent.
        let mut r = req();
        r.source = Some(ValidateSource {
            vm: Some("dc1/WIN-APP01".into()),
        });
        assert_eq!(
            codes(&validate_import(&r, &facts(), None).warnings),
            ["WINDOWS_SECURE_BOOT_TPM_HINT"]
        );
    }

    #[test]
    fn warns_when_image_cache_unconfirmed() {
        let v = validate_import(&req(), &ClusterFacts::default(), None);
        assert_eq!(codes(&v.warnings), ["IMAGE_CACHE_UNCONFIRMED"]);
    }

    #[test]
    fn probe_outcomes_become_size_and_warnings() {
        let ok = ProbeOutcome {
            reachable: true,
            status: Some(200),
            content_length: Some(490_000_000),
            error: None,
        };
        let v = validate_import(&req(), &facts(), Some(&ok));
        assert_eq!(v.size_bytes, Some(490_000_000));
        assert!(v.warnings.is_empty());

        let no_len = ProbeOutcome {
            content_length: None,
            ..ok.clone()
        };
        assert_eq!(
            codes(&validate_import(&req(), &facts(), Some(&no_len)).warnings),
            ["SIZE_UNKNOWN"]
        );
        let down = ProbeOutcome {
            reachable: false,
            status: Some(404),
            ..Default::default()
        };
        let v = validate_import(&req(), &facts(), Some(&down));
        assert!(v.valid, "an unreachable probe is a warning, not an error");
        assert_eq!(codes(&v.warnings), ["PROBE_UNREACHABLE"]);
    }

    #[tokio::test]
    async fn probe_runs_only_when_requested_and_url_passes_policy() {
        let fake = Fake(ProbeOutcome {
            reachable: true,
            status: Some(200),
            content_length: Some(7),
            error: None,
        });
        // probe not requested
        let v = run_validate(&req(), &facts(), &fake).await;
        assert!(v.probe.is_none());
        // requested
        let mut r = req();
        r.probe = true;
        let v = run_validate(&r, &facts(), &fake).await;
        assert_eq!(v.size_bytes, Some(7));
        // blocked URL is never probed
        r.image_url = Some("http://169.254.169.254/x".into());
        let v = run_validate(&r, &facts(), &fake).await;
        assert!(v.probe.is_none());
        assert_eq!(codes(&v.errors), ["INVALID_URL"]);
    }

    #[test]
    fn a_valid_report_means_machine_for_import_succeeds() {
        use crate::api::handlers::imports::{ImportRequest, machine_for_import};
        let r = req();
        assert!(validate_import(&r, &facts(), None).valid);
        let ir: ImportRequest = serde_json::from_value(json!({
            "name": r.name, "image_url": r.image_url, "sha256": r.sha256
        }))
        .unwrap();
        assert!(machine_for_import(&ir, "prod").is_ok());
    }
}
