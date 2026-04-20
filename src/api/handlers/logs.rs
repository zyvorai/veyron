#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use k8s_openapi::api::core::v1::Pod;
#[cfg(feature = "web")]
use kube::{Api, api::ListParams};

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub message: String,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Log query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryParams {
    pub namespace: Option<String>,
    pub level: Option<String>,
    pub source: Option<String>,
    #[serde(alias = "q")]
    pub search: Option<String>,
    pub limit: Option<u32>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub vm: Option<String>,
    pub tail: Option<i64>,
    pub since: Option<i64>,
    pub container: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/logs", get(list_logs))
        .route("/logs/query", get(query_logs))
        .with_state(state)
}

/// Dashboard-friendly log bundle (virt-launcher pod logs + simple counters).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDashboardResponse {
    pub error_count: u32,
    pub warn_count: u32,
    pub info_count: u32,
    pub total_1h: u32,
    pub lines: Vec<LogLineDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLineDto {
    pub ts: String,
    pub level: String,
    pub source: String,
    pub msg: String,
}

/// Parse a single log line into a LogEntry.
///
/// Tries to extract a leading RFC 3339 timestamp and an optional level token
/// (INFO, WARN, ERROR, DEBUG, TRACE) from the beginning of the line. Everything
/// remaining becomes the message.
#[cfg(feature = "web")]
fn parse_log_line(line: &str, source: &str) -> LogEntry {
    let mut timestamp = String::new();
    let mut level = "INFO".to_string();
    let mut message = line.to_string();

    // Try to extract a leading timestamp (e.g. "2024-01-15T12:00:00Z ...")
    if line.len() > 20 {
        let maybe_ts = &line[..std::cmp::min(35, line.len())];
        if let Some(space_idx) = maybe_ts.find(' ') {
            let ts_candidate = &line[..space_idx];
            if ts_candidate.contains('T') || ts_candidate.contains(':') {
                timestamp = ts_candidate.to_string();
                let rest = line[space_idx..].trim_start();
                message = rest.to_string();
            }
        }
    }

    // Try to extract a log level token from the remaining message
    let level_tokens = ["ERROR", "WARN", "WARNING", "INFO", "DEBUG", "TRACE"];
    for token in &level_tokens {
        if message.starts_with(token) {
            level = if *token == "WARNING" {
                "WARN".to_string()
            } else {
                token.to_string()
            };
            message = message[token.len()..]
                .trim_start_matches(&[' ', ':', '-'][..])
                .trim_start()
                .to_string();
            break;
        }
    }

    LogEntry {
        timestamp,
        level,
        source: source.to_string(),
        message,
        metadata: std::collections::HashMap::new(),
    }
}

/// Fetch logs from virt-launcher pods (`kubevirt.io/domain` label).
#[cfg(feature = "web")]
async fn fetch_pod_logs(
    state: &SharedState,
    namespace_scope: &str,
    vm_filter: Option<&str>,
    tail_lines: Option<i64>,
    since_seconds: Option<i64>,
    container: Option<&str>,
) -> Vec<LogEntry> {
    let s = state.read().await;
    let client = s.client().client();

    let pods: Api<Pod> = if namespace_scope == "all" {
        Api::all(client.clone())
    } else {
        Api::namespaced(client.clone(), namespace_scope)
    };
    let lp = ListParams::default().labels("kubevirt.io/domain");

    let pod_list = match pods.list(&lp).await {
        Ok(list) => list,
        Err(_) => return vec![],
    };

    let mut entries = Vec::new();

    for pod in &pod_list.items {
        let pod_name = pod.metadata.name.clone().unwrap_or_default();

        // If a VM filter is specified, check the kubevirt.io/domain label
        if let Some(vm) = vm_filter {
            let domain = pod
                .metadata
                .labels
                .as_ref()
                .and_then(|l| l.get("kubevirt.io/domain"))
                .map(|s| s.as_str())
                .unwrap_or("");
            if domain != vm {
                continue;
            }
        }

        let tail = tail_lines.unwrap_or(100).min(10_000);

        let mut log_params = kube::api::LogParams {
            tail_lines: Some(tail),
            ..Default::default()
        };

        if let Some(since) = since_seconds.filter(|&s| s > 0) {
            log_params.since_seconds = Some(since);
        }

        if let Some(c) = container {
            log_params.container = Some(c.to_string());
        }

        let log_text = match pods.logs(&pod_name, &log_params).await {
            Ok(text) => text,
            Err(_) => continue,
        };

        for line in log_text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            entries.push(parse_log_line(line, &pod_name));
        }
    }

    entries
}

#[cfg(feature = "web")]
async fn list_logs(
    State(state): State<SharedState>,
    Query(params): Query<LogQueryParams>,
) -> Json<LogDashboardResponse> {
    let ns = {
        let s = state.read().await;
        params
            .namespace
            .clone()
            .unwrap_or_else(|| s.namespace.clone())
    };
    let mut entries = fetch_pod_logs(&state, &ns, None, Some(500), None, None).await;
    apply_log_filters(&mut entries, &params);
    Json(build_log_dashboard(entries))
}

#[cfg(feature = "web")]
async fn query_logs(
    State(state): State<SharedState>,
    Query(params): Query<LogQueryParams>,
) -> Json<Vec<LogEntry>> {
    let ns = {
        let s = state.read().await;
        params
            .namespace
            .clone()
            .unwrap_or_else(|| s.namespace.clone())
    };
    let tail = params.tail.or(params.limit.map(|l| l as i64));
    let mut entries = fetch_pod_logs(
        &state,
        &ns,
        params.vm.as_deref(),
        tail,
        params.since,
        params.container.as_deref(),
    )
    .await;

    apply_log_filters(&mut entries, &params);

    Json(entries)
}

#[cfg(feature = "web")]
fn apply_log_filters(entries: &mut Vec<LogEntry>, params: &LogQueryParams) {
    if let Some(ref level_filter) = params.level {
        let uf = level_filter.to_uppercase();
        entries.retain(|e| e.level == uf);
    }

    if let Some(ref source_filter) = params.source {
        entries.retain(|e| e.source.contains(source_filter.as_str()));
    }

    if let Some(ref search) = params.search {
        let s = search.to_lowercase();
        entries.retain(|e| e.message.to_lowercase().contains(&s));
    }
}

#[cfg(feature = "web")]
fn entry_to_dto(e: LogEntry) -> LogLineDto {
    LogLineDto {
        ts: if e.timestamp.is_empty() {
            "-".to_string()
        } else {
            e.timestamp
        },
        level: e.level,
        source: e.source,
        msg: e.message,
    }
}

#[cfg(feature = "web")]
fn build_log_dashboard(entries: Vec<LogEntry>) -> LogDashboardResponse {
    let error_count = entries.iter().filter(|e| e.level == "ERROR").count() as u32;
    let warn_count = entries.iter().filter(|e| e.level == "WARN").count() as u32;
    let info_count = entries.iter().filter(|e| e.level == "INFO").count() as u32;
    let total_1h = entries.len() as u32;
    let lines: Vec<LogLineDto> = entries.into_iter().map(entry_to_dto).collect();

    LogDashboardResponse {
        error_count,
        warn_count,
        info_count,
        total_1h,
        lines,
    }
}
