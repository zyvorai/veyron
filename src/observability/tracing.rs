// Distributed Tracing - Trace collection and correlation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCollector {
    pub traces: Vec<Trace>,
    pub max_traces: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub trace_id: String,
    pub spans: Vec<Span>,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub service: String,
    pub status: TraceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub span_id: String,
    pub parent_id: Option<String>,
    pub operation: String,
    pub service: String,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub status: SpanStatus,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TraceStatus {
    Ok,
    Error,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpanStatus {
    Ok,
    Error,
    Timeout,
}

impl TraceCollector {
    pub fn new(max_traces: usize) -> Self {
        Self {
            traces: Vec::new(),
            max_traces,
        }
    }

    pub fn add_trace(&mut self, trace: Trace) {
        self.traces.insert(0, trace);
        self.traces.truncate(self.max_traces);
    }

    pub fn get_trace(&self, id: &str) -> Option<&Trace> {
        self.traces.iter().find(|t| t.trace_id == id)
    }
    pub fn recent(&self, limit: usize) -> Vec<&Trace> {
        self.traces.iter().take(limit).collect()
    }

    pub fn error_traces(&self) -> Vec<&Trace> {
        self.traces
            .iter()
            .filter(|t| t.status == TraceStatus::Error)
            .collect()
    }

    pub fn slow_traces(&self, threshold_ms: u64) -> Vec<&Trace> {
        self.traces
            .iter()
            .filter(|t| t.duration_ms > threshold_ms)
            .collect()
    }

    pub fn by_service(&self, service: &str) -> Vec<&Trace> {
        self.traces
            .iter()
            .filter(|t| t.service == service)
            .collect()
    }

    pub fn avg_latency(&self) -> f64 {
        if self.traces.is_empty() {
            return 0.0;
        }
        self.traces
            .iter()
            .map(|t| t.duration_ms as f64)
            .sum::<f64>()
            / self.traces.len() as f64
    }
}

impl Default for TraceCollector {
    fn default() -> Self {
        Self::new(1000)
    }
}
