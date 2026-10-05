// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//
// Security Operations Center — normalized events, detections, SIEM export, hunts, ASM.

pub mod asm;
pub mod collect;
pub mod detections;
pub mod event;
pub mod export;
pub mod hunts;
pub mod playbooks;
pub mod store;

pub use detections::SocDetection;
pub use event::SecurityEvent;

/// Shared client for SIEM export and SOAR webhooks: one connection pool, and a hung
/// endpoint fails after 30s instead of stalling the collector forever.
#[cfg(feature = "web")]
pub fn http_client() -> &'static reqwest::Client {
    static CLIENT: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    });
    &CLIENT
}
