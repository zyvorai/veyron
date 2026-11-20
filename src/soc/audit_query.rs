// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Query and export of the API audit trail (`category == "audit"` events in the SOC log).
//!
//! `GET /soc/events` can only filter by severity and category and cannot bound by time. This module
//! adds the filters an auditor actually needs (who, what, when, outcome, which VM or plan) and
//! three output formats. Everything here is pure so it is tested without a cluster.

use super::event::SecurityEvent;
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Hard cap on one response; the stored log never holds more than 500 events anyway.
pub const MAX_LIMIT: usize = 500;
pub const DEFAULT_LIMIT: usize = 200;

/// Raw query string parameters of `GET /soc/audit`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuditQuery {
    /// Exact actor (API key name, username, JWT subject), case-insensitive.
    pub actor: Option<String>,
    /// HTTP method (or agent action), case-insensitive.
    pub action: Option<String>,
    /// `success` or `failure`.
    pub outcome: Option<String>,
    /// Substring of the request path / target.
    pub path: Option<String>,
    pub namespace: Option<String>,
    pub vm: Option<String>,
    /// `veyron` (API calls) or `veyron-agent` (Copilot plan steps).
    pub source: Option<String>,
    /// Copilot plan id (label `plan_id`).
    pub plan_id: Option<String>,
    /// RFC 3339, inclusive.
    pub since: Option<String>,
    /// RFC 3339, exclusive.
    pub until: Option<String>,
    pub limit: Option<usize>,
    /// `desc` (default, newest first) or `asc`.
    pub order: Option<String>,
    /// `json` (default), `ndjson` or `csv`.
    pub format: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Ndjson,
    Csv,
}

/// A validated query.
#[derive(Debug, Clone)]
pub struct AuditFilter {
    q: AuditQuery,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    pub limit: usize,
    pub ascending: bool,
    pub format: Format,
}

impl AuditFilter {
    /// Validate a raw query. The error text is safe to return to the client.
    pub fn parse(q: AuditQuery) -> Result<Self, String> {
        let ts = |name: &str, v: &Option<String>| -> Result<Option<DateTime<Utc>>, String> {
            match v.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                None => Ok(None),
                Some(s) => DateTime::parse_from_rfc3339(s)
                    .map(|d| Some(d.with_timezone(&Utc)))
                    .map_err(|_| {
                        format!("{name} must be an RFC 3339 timestamp, e.g. 2026-11-01T00:00:00Z")
                    }),
            }
        };
        let since = ts("since", &q.since)?;
        let until = ts("until", &q.until)?;
        if let (Some(s), Some(u)) = (since, until) {
            if s >= u {
                return Err("since must be earlier than until".into());
            }
        }
        let limit = q.limit.unwrap_or(DEFAULT_LIMIT);
        if limit == 0 {
            return Err("limit must be at least 1".into());
        }
        let ascending = match q.order.as_deref().map(str::to_ascii_lowercase).as_deref() {
            None | Some("desc") => false,
            Some("asc") => true,
            Some(_) => return Err("order must be asc or desc".into()),
        };
        let format = match q.format.as_deref().map(str::to_ascii_lowercase).as_deref() {
            None | Some("json") => Format::Json,
            Some("ndjson") => Format::Ndjson,
            Some("csv") => Format::Csv,
            Some(_) => return Err("format must be json, ndjson or csv".into()),
        };
        match q.outcome.as_deref().map(str::to_ascii_lowercase).as_deref() {
            None | Some("success") | Some("failure") => {}
            Some(_) => return Err("outcome must be success or failure".into()),
        }
        Ok(Self {
            q,
            since,
            until,
            limit: limit.min(MAX_LIMIT),
            ascending,
            format,
        })
    }

    fn matches(&self, e: &SecurityEvent) -> bool {
        if e.category != "audit" {
            return false;
        }
        let eq = |want: &Option<String>, have: &str| {
            want.as_deref().is_none_or(|w| w.eq_ignore_ascii_case(have))
        };
        if !(eq(&self.q.actor, &e.actor)
            && eq(&self.q.action, &e.action)
            && eq(&self.q.outcome, &e.outcome)
            && eq(&self.q.namespace, &e.namespace)
            && eq(&self.q.source, &e.source))
        {
            return false;
        }
        if let Some(vm) = &self.q.vm {
            if !e
                .vm_name
                .as_deref()
                .is_some_and(|v| v.eq_ignore_ascii_case(vm))
            {
                return false;
            }
        }
        if let Some(p) = &self.q.path {
            if !e.target.contains(p.as_str()) {
                return false;
            }
        }
        if let Some(id) = &self.q.plan_id {
            if e.labels.get("plan_id") != Some(id) {
                return false;
            }
        }
        if self.since.is_some() || self.until.is_some() {
            // Events whose timestamp does not parse can never be placed in a window.
            let Ok(t) = DateTime::parse_from_rfc3339(&e.timestamp) else {
                return false;
            };
            let t = t.with_timezone(&Utc);
            if self.since.is_some_and(|s| t < s) || self.until.is_some_and(|u| t >= u) {
                return false;
            }
        }
        true
    }

    /// Filter, order and truncate. Returns `(matched_total, page)`.
    pub fn apply<'a>(&self, events: &'a [SecurityEvent]) -> (usize, Vec<&'a SecurityEvent>) {
        let mut hits: Vec<&SecurityEvent> = events.iter().filter(|e| self.matches(e)).collect();
        // RFC 3339 strings from one producer sort chronologically; parse for correctness across offsets.
        let key = |e: &&SecurityEvent| {
            DateTime::parse_from_rfc3339(&e.timestamp)
                .map(|d| d.timestamp_millis())
                .unwrap_or(i64::MIN)
        };
        hits.sort_by_key(key);
        if !self.ascending {
            hits.reverse();
        }
        let total = hits.len();
        hits.truncate(self.limit);
        (total, hits)
    }
}

const CSV_COLUMNS: [&str; 11] = [
    "timestamp",
    "id",
    "source",
    "actor",
    "action",
    "outcome",
    "target",
    "namespace",
    "vm",
    "plan_id",
    "message",
];

/// One CSV field. Quotes when needed and defuses spreadsheet formula injection: a value that starts
/// with `=`, `+`, `-`, `@`, tab or CR is prefixed with `'` (actors and paths are attacker-controlled).
pub fn csv_field(v: &str) -> String {
    let mut s = v.to_string();
    if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        s.insert(0, '\'');
    }
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

pub fn to_csv(events: &[&SecurityEvent]) -> String {
    let mut out = CSV_COLUMNS.join(",");
    out.push('\n');
    for e in events {
        let row = [
            e.timestamp.as_str(),
            e.id.as_str(),
            e.source.as_str(),
            e.actor.as_str(),
            e.action.as_str(),
            e.outcome.as_str(),
            e.target.as_str(),
            e.namespace.as_str(),
            e.vm_name.as_deref().unwrap_or(""),
            e.labels.get("plan_id").map(String::as_str).unwrap_or(""),
            e.message.as_str(),
        ];
        out.push_str(
            &row.iter()
                .map(|f| csv_field(f))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

/// One JSON object per line (serde escapes newlines, so a line is always one event).
pub fn to_ndjson(events: &[&SecurityEvent]) -> String {
    let mut out = String::new();
    for e in events {
        if let Ok(line) = serde_json::to_string(e) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts: &str, actor: &str, method: &str, path: &str, outcome: &str) -> SecurityEvent {
        let mut e = SecurityEvent::new(
            "audit",
            "low",
            "veyron",
            format!("{method} {path} — {outcome}"),
        )
        .with_actor(actor)
        .with_action(method)
        .with_target(path)
        .with_outcome(outcome);
        e.timestamp = ts.to_string();
        e
    }

    fn sample() -> Vec<SecurityEvent> {
        let mut agent = ev(
            "2026-11-03T10:00:00Z",
            "alice",
            "snapshot",
            "ns1/vm1",
            "success",
        );
        agent.source = "veyron-agent".into();
        agent.labels.insert("plan_id".into(), "plan-7".into());
        vec![
            ev(
                "2026-11-01T10:00:00Z",
                "ci",
                "POST",
                "/api/v1/vms/ns1/vm1/start",
                "success",
            ),
            ev(
                "2026-11-02T10:00:00Z",
                "bob",
                "DELETE",
                "/api/v1/vms/ns2/vm9",
                "failure",
            ),
            agent,
            SecurityEvent::new("network", "high", "k8s", "not an audit event"),
        ]
    }

    fn q(f: impl FnOnce(&mut AuditQuery)) -> AuditFilter {
        let mut q = AuditQuery::default();
        f(&mut q);
        AuditFilter::parse(q).unwrap()
    }

    #[test]
    fn only_audit_events_and_newest_first_by_default() {
        let evs = sample();
        let (total, hits) = q(|_| {}).apply(&evs);
        assert_eq!(total, 3);
        assert_eq!(hits[0].actor, "alice");
        assert_eq!(hits[2].actor, "ci");
        let (_, asc) = q(|q| q.order = Some("ASC".into())).apply(&evs);
        assert_eq!(asc[0].actor, "ci");
    }

    #[test]
    fn filters_combine_with_and() {
        let evs = sample();
        let (t, h) = q(|q| q.actor = Some("BOB".into())).apply(&evs);
        assert_eq!((t, h.len()), (1, 1));
        let (t, _) = q(|q| q.outcome = Some("failure".into())).apply(&evs);
        assert_eq!(t, 1);
        let (t, _) = q(|q| q.path = Some("/vms/ns1".into())).apply(&evs);
        assert_eq!(t, 1);
        let (t, _) = q(|q| {
            q.actor = Some("bob".into());
            q.outcome = Some("success".into());
        })
        .apply(&evs);
        assert_eq!(t, 0);
        let (t, h) = q(|q| q.plan_id = Some("plan-7".into())).apply(&evs);
        assert_eq!((t, h[0].source.as_str()), (1, "veyron-agent"));
        let (t, _) = q(|q| q.source = Some("veyron".into())).apply(&evs);
        assert_eq!(t, 2);
    }

    #[test]
    fn vm_filter_uses_the_parsed_vm_name() {
        let mut evs = sample();
        evs[0].vm_name = Some("vm1".into());
        let (t, _) = q(|q| q.vm = Some("VM1".into())).apply(&evs);
        assert_eq!(t, 1);
    }

    #[test]
    fn time_window_is_since_inclusive_until_exclusive() {
        let evs = sample();
        let (t, _) = q(|q| q.since = Some("2026-11-02T10:00:00Z".into())).apply(&evs);
        assert_eq!(t, 2);
        let (t, _) = q(|q| q.until = Some("2026-11-02T10:00:00Z".into())).apply(&evs);
        assert_eq!(t, 1);
        // offsets are honoured: 11:00+01:00 is 10:00Z
        let (t, _) = q(|q| q.since = Some("2026-11-03T11:00:00+01:00".into())).apply(&evs);
        assert_eq!(t, 1);
    }

    #[test]
    fn unparseable_timestamps_are_excluded_from_windows_only() {
        let mut evs = sample();
        evs[0].timestamp = "garbage".into();
        assert_eq!(q(|_| {}).apply(&evs).0, 3);
        assert_eq!(
            q(|q| q.since = Some("2000-01-01T00:00:00Z".into()))
                .apply(&evs)
                .0,
            2
        );
    }

    #[test]
    fn limit_truncates_but_total_counts_all() {
        let evs = sample();
        let (t, h) = q(|q| q.limit = Some(1)).apply(&evs);
        assert_eq!((t, h.len()), (3, 1));
        assert_eq!(q(|q| q.limit = Some(10_000)).limit, MAX_LIMIT);
    }

    #[test]
    fn bad_input_is_rejected_with_a_message() {
        let bad = |f: fn(&mut AuditQuery)| {
            let mut q = AuditQuery::default();
            f(&mut q);
            AuditFilter::parse(q).unwrap_err()
        };
        assert!(bad(|q| q.since = Some("yesterday".into())).contains("RFC 3339"));
        assert!(bad(|q| q.until = Some("2026-13-45".into())).contains("until"));
        assert!(bad(|q| q.limit = Some(0)).contains("limit"));
        assert!(bad(|q| q.order = Some("sideways".into())).contains("order"));
        assert!(bad(|q| q.format = Some("xml".into())).contains("format"));
        assert!(bad(|q| q.outcome = Some("maybe".into())).contains("outcome"));
        assert!(
            bad(|q| {
                q.since = Some("2026-11-02T00:00:00Z".into());
                q.until = Some("2026-11-01T00:00:00Z".into());
            })
            .contains("earlier")
        );
    }

    #[test]
    fn csv_neutralizes_formulas_and_quotes_specials() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(csv_field("+1"), "'+1");
        assert_eq!(csv_field("-2"), "'-2");
        assert_eq!(csv_field("@SUM(A1)"), "'@SUM(A1)");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("line\nbreak"), "\"line\nbreak\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn csv_has_header_and_one_row_per_event() {
        let mut evs = sample();
        evs[0].actor = "=cmd|' /C calc'!A0".into();
        let (_, hits) = q(|q| q.order = Some("asc".into())).apply(&evs);
        let csv = to_csv(&hits);
        let lines: Vec<_> = csv.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].starts_with("timestamp,id,source,actor"));
        assert!(lines[1].contains("'=cmd|' /C calc'!A0"));
        assert!(!lines[1].contains(",=cmd"));
    }

    #[test]
    fn ndjson_is_one_event_per_line_even_with_newlines_in_fields() {
        let mut evs = sample();
        evs[0].message = "a\nb".into();
        let (_, hits) = q(|_| {}).apply(&evs);
        let nd = to_ndjson(&hits);
        assert_eq!(nd.lines().count(), 3);
        for l in nd.lines() {
            serde_json::from_str::<serde_json::Value>(l).unwrap();
        }
    }
}
