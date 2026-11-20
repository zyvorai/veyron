// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use super::SocExportStatus;
use crate::soc::event::SecurityEvent;

static LAST_ERROR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static EVENTS_PUSHED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct QRadarConfig {
    pub syslog_host: String,
    pub syslog_port: u16,
    pub leef_vendor: String,
    pub leef_product: String,
}

pub fn config() -> Option<QRadarConfig> {
    // `VEYRON_QRADAR_HOST`/`_PORT` are the names documented in docs/soc.md.
    let env_nonempty = |names: &[&str]| {
        names
            .iter()
            .filter_map(|n| std::env::var(n).ok())
            .find(|s| !s.trim().is_empty())
    };
    let host = env_nonempty(&["VEYRON_QRADAR_SYSLOG_HOST", "VEYRON_QRADAR_HOST"])?;
    let port = env_nonempty(&["VEYRON_QRADAR_SYSLOG_PORT", "VEYRON_QRADAR_PORT"])
        .and_then(|p| p.trim().parse().ok())
        .unwrap_or(514);
    let leef_vendor = std::env::var("VEYRON_QRADAR_LEEF_VENDOR").unwrap_or_else(|_| "Zyvor".into());
    let leef_product =
        std::env::var("VEYRON_QRADAR_LEEF_PRODUCT").unwrap_or_else(|_| "Veyron".into());
    Some(QRadarConfig {
        syslog_host: host,
        syslog_port: port,
        leef_vendor,
        leef_product,
    })
}

pub fn status() -> SocExportStatus {
    let configured = config().is_some();
    SocExportStatus {
        backend: "qradar".to_string(),
        configured,
        last_push_ok: configured && LAST_ERROR.lock().ok().and_then(|g| g.clone()).is_none(),
        last_error: LAST_ERROR.lock().ok().and_then(|g| g.clone()),
        events_pushed: EVENTS_PUSHED.load(std::sync::atomic::Ordering::Relaxed),
    }
}

/// Neutralise characters that would split a LEEF record or forge fields:
/// tab (field delimiter), CR/LF (record boundary) and `|` (header delimiter).
fn leef_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' | '\r' | '\n' | '|' => ' ',
            c => c,
        })
        .collect()
}

fn to_leef(ev: &SecurityEvent, cfg: &QRadarConfig) -> String {
    format!(
        "LEEF:2.0|{}|{}|1.0|{}|devTime={}\tsev={}\tsrc={}\tusrName={}\tmsg={}",
        leef_escape(&cfg.leef_vendor),
        leef_escape(&cfg.leef_product),
        leef_escape(&ev.category),
        leef_escape(&ev.timestamp),
        leef_escape(&ev.severity),
        leef_escape(&ev.source),
        leef_escape(&ev.actor),
        leef_escape(&ev.message)
    )
}

#[cfg(feature = "web")]
pub async fn push_events(events: &[SecurityEvent]) -> anyhow::Result<u32> {
    let cfg = match config() {
        Some(c) => c,
        None => return Ok(0),
    };
    let addr = format!("{}:{}", cfg.syslog_host, cfg.syslog_port);
    let mut pushed = 0u32;
    for ev in events {
        let line = to_leef(ev, &cfg);
        match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
            Ok(sock) => {
                if sock.send_to(line.as_bytes(), &addr).await.is_ok() {
                    pushed += 1;
                }
            }
            Err(e) => {
                if let Ok(mut g) = LAST_ERROR.lock() {
                    *g = Some(e.to_string());
                }
            }
        }
    }
    if pushed > 0 {
        if let Ok(mut g) = LAST_ERROR.lock() {
            *g = None;
        }
        EVENTS_PUSHED.fetch_add(pushed as u64, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(pushed)
}

#[cfg(feature = "web")]
pub async fn probe() -> bool {
    config().is_some()
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    fn ev(actor: &str, message: &str) -> SecurityEvent {
        serde_json::from_value(serde_json::json!({
            "id": "1", "timestamp": "t", "severity": "high", "category": "auth",
            "source": "api", "actor": actor, "target": "", "namespace": "ns",
            "action": "x", "outcome": "ok", "message": message
        }))
        .unwrap()
    }

    #[test]
    fn leef_neutralises_record_and_field_delimiters() {
        let cfg = QRadarConfig {
            syslog_host: "h".into(),
            syslog_port: 514,
            leef_vendor: "Zyvor".into(),
            leef_product: "Veyron".into(),
        };
        let line = to_leef(&ev("a|b\nc", "m\r\nsev=low\tx"), &cfg);
        assert!(!line.contains('\n') && !line.contains('\r'));
        assert_eq!(line.matches('\t').count(), 4);
        assert_eq!(line.matches('|').count(), 5);
    }
}
