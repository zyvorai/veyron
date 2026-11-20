// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Resolve an OCI tag to the digest Kairon requires on `spec.image.digest`.
//!
//! Kairon keys its node image cache by digest and rejects an image source
//! without one, so a `repo:tag` reference from a template or API caller is
//! pinned here at create time (anonymous registry v2 token flow).

use anyhow::{Context, Result, anyhow, bail};
use std::time::Duration;

const ACCEPT: &str = "application/vnd.oci.image.index.v1+json, \
application/vnd.docker.distribution.manifest.list.v2+json, \
application/vnd.oci.image.manifest.v1+json, \
application/vnd.docker.distribution.manifest.v2+json";

/// Split `[registry/]repo[:tag]` into (registry host, repository, tag).
pub fn split_reference(image: &str) -> (String, String, String) {
    let image = image.split('@').next().unwrap_or(image);
    let (host, rest) = match image.split_once('/') {
        Some((h, r)) if h.contains('.') || h.contains(':') || h == "localhost" => {
            (h.to_string(), r.to_string())
        }
        _ => ("registry-1.docker.io".to_string(), image.to_string()),
    };
    let rest = if host == "registry-1.docker.io" && !rest.contains('/') {
        format!("library/{rest}")
    } else {
        rest
    };
    let (repo, tag) = match rest.rsplit_once(':') {
        Some((r, t)) if !t.contains('/') => (r.to_string(), t.to_string()),
        _ => (rest, "latest".to_string()),
    };
    (host, repo, tag)
}

/// Resolve `image` (`repo:tag` or `repo@sha256:…`) to its manifest digest.
pub async fn resolve_digest(image: &str) -> Result<String> {
    if let Some(d) = super::converter::oci_digest_of(image) {
        return Ok(d);
    }
    let (host, repo, tag) = split_reference(image);
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    let url = format!("https://{host}/v2/{repo}/manifests/{tag}");
    let mut resp = http
        .head(&url)
        .header("Accept", ACCEPT)
        .send()
        .await
        .with_context(|| format!("HEAD {url}"))?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        let challenge = resp
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let token = anonymous_token(&http, &challenge, &repo).await?;
        resp = http
            .head(&url)
            .header("Accept", ACCEPT)
            .bearer_auth(token)
            .send()
            .await
            .with_context(|| format!("HEAD {url}"))?;
    }
    if !resp.status().is_success() {
        bail!("registry returned {} for {image}", resp.status());
    }
    resp.headers()
        .get("docker-content-digest")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| anyhow!("registry did not return Docker-Content-Digest for {image}"))
}

async fn anonymous_token(http: &reqwest::Client, challenge: &str, repo: &str) -> Result<String> {
    let param = |key: &str| -> Option<String> {
        let needle = format!("{key}=\"");
        let start = challenge.find(&needle)? + needle.len();
        let end = challenge[start..].find('"')? + start;
        Some(challenge[start..end].to_string())
    };
    let realm = param("realm").ok_or_else(|| anyhow!("registry auth challenge has no realm"))?;
    let service = param("service").unwrap_or_default();
    let scope = param("scope").unwrap_or_else(|| format!("repository:{repo}:pull"));
    let v: serde_json::Value = http
        .get(&realm)
        .query(&[("service", service.as_str()), ("scope", scope.as_str())])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    v.get("token")
        .or_else(|| v.get("access_token"))
        .and_then(|t| t.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow!("registry token endpoint returned no token"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references() {
        assert_eq!(
            split_reference("quay.io/containerdisks/ubuntu:24.04"),
            (
                "quay.io".into(),
                "containerdisks/ubuntu".into(),
                "24.04".into()
            )
        );
        assert_eq!(
            split_reference("ubuntu"),
            (
                "registry-1.docker.io".into(),
                "library/ubuntu".into(),
                "latest".into()
            )
        );
        assert_eq!(
            split_reference("localhost:5000/disks/fedora"),
            (
                "localhost:5000".into(),
                "disks/fedora".into(),
                "latest".into()
            )
        );
    }
}
