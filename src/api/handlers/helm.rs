// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use k8s_openapi::api::core::v1::Secret;

/// Helm release response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmReleaseResponse {
    pub name: String,
    pub namespace: String,
    pub chart: String,
    pub chart_version: String,
    pub app_version: String,
    pub status: String,
    pub revision: u32,
    pub updated_at: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/helm/releases", get(list_helm_releases))
        .with_state(state)
}

/// Discover Helm releases by listing Secrets labeled with `owner=helm`.
#[cfg(feature = "web")]
async fn list_helm_releases(State(state): State<SharedState>) -> Json<Vec<HelmReleaseResponse>> {
    let s = state.read().await;
    let api: kube::api::Api<Secret> = kube::api::Api::all(s.client().client());
    let params = kube::api::ListParams::default().labels("owner=helm");
    let secrets = match api.list(&params).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let mut releases: std::collections::HashMap<String, HelmReleaseResponse> =
        std::collections::HashMap::new();

    for secret in &secrets.items {
        let labels = secret.metadata.labels.as_ref();
        let annotations = secret.metadata.annotations.as_ref();
        let name = labels
            .and_then(|l| l.get("name"))
            .cloned()
            .or_else(|| {
                annotations
                    .and_then(|a| a.get("meta.helm.sh/release-name"))
                    .cloned()
            })
            .or_else(|| secret.metadata.name.as_deref().and_then(parse_release_name))
            .unwrap_or_default();
        let status = labels
            .and_then(|l| l.get("status"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let version: u32 = labels
            .and_then(|l| l.get("version"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let (chart, chart_version) = labels
            .and_then(|l| l.get("helm.sh/chart").or_else(|| l.get("chart")))
            .map(|v| parse_chart_label(v))
            .unwrap_or_else(|| (String::new(), String::new()));
        let app_version = labels
            .and_then(|l| l.get("app.kubernetes.io/version"))
            .cloned()
            .or_else(|| {
                annotations
                    .and_then(|a| a.get("app.kubernetes.io/version"))
                    .cloned()
            })
            .unwrap_or_default();

        // Keep only the latest revision per release name
        let entry = releases
            .entry(name.clone())
            .or_insert_with(|| HelmReleaseResponse {
                name: name.clone(),
                namespace: secret.metadata.namespace.clone().unwrap_or_default(),
                chart: chart.clone(),
                chart_version: chart_version.clone(),
                app_version: app_version.clone(),
                status: status.clone(),
                revision: 0,
                updated_at: secret
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            });

        if version > entry.revision {
            entry.revision = version;
            entry.status = status;
            entry.chart = chart.clone();
            entry.chart_version = chart_version.clone();
            entry.app_version = app_version.clone();
            entry.updated_at = secret
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default();
        }
    }

    Json(releases.into_values().collect())
}

#[cfg(feature = "web")]
fn parse_release_name(secret_name: &str) -> Option<String> {
    let remainder = secret_name.strip_prefix("sh.helm.release.v1.")?;
    let (name, _) = remainder.rsplit_once(".v")?;
    Some(name.to_string())
}

#[cfg(feature = "web")]
fn parse_chart_label(label: &str) -> (String, String) {
    if let Some((name, version)) = label.rsplit_once('-') {
        if version
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            return (name.to_string(), version.to_string());
        }
    }
    (label.to_string(), String::new())
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    #[test]
    fn parse_release_name_from_helm_secret() {
        assert_eq!(
            parse_release_name("sh.helm.release.v1.vmrogue.v3"),
            Some("vmrogue".to_string())
        );
    }

    #[test]
    fn parse_chart_label_splits_chart_and_version() {
        assert_eq!(
            parse_chart_label("vmrogue-0.2.0"),
            ("vmrogue".to_string(), "0.2.0".to_string())
        );
        assert_eq!(
            parse_chart_label("custom-chart"),
            ("custom-chart".to_string(), String::new())
        );
    }
}
