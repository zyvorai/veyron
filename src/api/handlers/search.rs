// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! `GET /api/v1/search?q=&limit=&namespace=` — cross-resource search
//! (VMs, pods, nodes, PVCs). Read-only; see `copilot::search` for the query syntax.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
#[cfg(feature = "web")]
use serde::Deserialize;
#[cfg(feature = "web")]
use serde::Serialize;

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::copilot::search::{self, SearchHit};

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub limit: Option<usize>,
    #[serde(flatten)]
    pub ns: DashboardNamespaceQuery,
}

#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub scope: String,
    pub count: usize,
    pub results: Vec<SearchHit>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/search", get(search_handler))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn search_handler(
    State(state): State<SharedState>,
    Query(p): Query<SearchParams>,
) -> Json<SearchResponse> {
    let query = p.q.clone().unwrap_or_default();
    let (client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let parsed = search::parse_query(&query);
    let scope = parsed
        .namespace
        .clone()
        .unwrap_or_else(|| namespace_scope::resolve_opt(p.ns.namespace.clone(), &default_ns));
    let results = search::search(
        &client,
        &scope,
        &parsed,
        p.limit.unwrap_or(search::DEFAULT_LIMIT),
    )
    .await;
    Json(SearchResponse {
        query,
        scope,
        count: results.len(),
        results,
    })
}
