// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

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
    let host = std::env::var("VEYRON_QRADAR_SYSLOG_HOST")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let port = std::env::var("VEYRON_QRADAR_SYSLOG_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(514);
    let leef_vendor = std::env::var("VEYRON_QRADAR_LEEF_VENDOR")
        .unwrap_or_else(|_| "Zyvor".into());
    let leef_product = std::env::var("VEYRON_QRADAR_LEEF_PRODUCT")
        .unwrap_or_else(|_| "VMRogue".into());
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

fn to_leef(ev: &SecurityEvent, cfg: &QRadarConfig) -> String {
    format!(
        "LEEF:2.0|{}|{}|1.0|{}|devTime={}\tsev={}\tsrc={}\tusrName={}\tmsg={}",
        cfg.leef_vendor,
        cfg.leef_product,
        ev.category,
        ev.timestamp,
        ev.severity,
        ev.source,
        ev.actor,
        ev.message.replace('\t', " ")
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
