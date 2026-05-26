// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Ingress response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressResponse {
    pub name: String,
    pub namespace: String,
    pub class_name: Option<String>,
    pub hosts: Vec<String>,
    pub tls: bool,
    pub rules: Vec<IngressRule>,
    pub created_at: String,
}

/// Ingress rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRule {
    pub host: String,
    pub path: String,
    pub service_name: String,
    pub service_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngressQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/ingress", get(list_ingress))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_ingress(
    State(state): State<SharedState>,
    Query(query): Query<IngressQuery>,
) -> Json<Vec<IngressResponse>> {
    use k8s_openapi::api::networking::v1::Ingress;

    let s = state.read().await;
    let client = s.client().client();
    let api: kube::api::Api<Ingress> = match query.namespace.as_deref() {
        Some("all") => kube::api::Api::all(client),
        Some(ns) => kube::api::Api::namespaced(client, ns),
        None => kube::api::Api::namespaced(client, &s.namespace),
    };
    let ingresses = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<IngressResponse> = ingresses
        .items
        .iter()
        .map(|ing| {
            let spec = ing.spec.as_ref();

            let has_tls = spec
                .and_then(|s| s.tls.as_ref())
                .map(|t| !t.is_empty())
                .unwrap_or(false);

            let mut hosts = Vec::new();
            let mut rules = Vec::new();

            if let Some(spec_rules) = spec.and_then(|s| s.rules.as_ref()) {
                for rule in spec_rules {
                    let host = rule.host.clone().unwrap_or_default();
                    if !host.is_empty() {
                        hosts.push(host.clone());
                    }
                    if let Some(http) = &rule.http {
                        for path in &http.paths {
                            let (svc_name, svc_port) = path
                                .backend
                                .service
                                .as_ref()
                                .map(|svc| {
                                    let port = svc.port.as_ref().and_then(|p| p.number).unwrap_or(0)
                                        as u16;
                                    (svc.name.clone(), port)
                                })
                                .unwrap_or_default();

                            rules.push(IngressRule {
                                host: host.clone(),
                                path: path.path.clone().unwrap_or_else(|| "/".to_string()),
                                service_name: svc_name,
                                service_port: svc_port,
                            });
                        }
                    }
                }
            }

            IngressResponse {
                name: ing.metadata.name.clone().unwrap_or_default(),
                namespace: ing.metadata.namespace.clone().unwrap_or_default(),
                class_name: spec.and_then(|s| s.ingress_class_name.clone()),
                hosts,
                tls: has_tls,
                rules,
                created_at: ing
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            }
        })
        .collect();

    Json(results)
}
