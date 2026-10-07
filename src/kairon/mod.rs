// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Kairon VM platform integration (`kairon.zyvor.dev`).
//!
//! Kairon runs real VMs on Kubernetes without a pod per VM: a `Machine`
//! object is desired state, `kairon-controller` places it, and `kairon-node`
//! runs it on FluxVM. Veyron reads and writes these CRDs through the Kubernetes
//! API and reaches the per-node relay (console, guest exec, logs) directly.

pub mod converter;
#[cfg(feature = "web")]
pub mod oci;
#[cfg(feature = "web")]
pub mod relay;
pub mod types;

pub use converter::{machine_from_config, parse_cloud_config, resolve_container_disk_image};
pub use types::*;

pub const GROUP: &str = "kairon.zyvor.dev";
pub const VERSION: &str = "v1beta1";
pub const API_VERSION: &str = "kairon.zyvor.dev/v1beta1";

/// Fork a running parent Machine's FluxVM runtime instead of booting fresh.
pub const ANNOTATION_FORK_FROM: &str = "kairon.zyvor.dev/fork-from";
pub const LABEL_FORKED_FROM: &str = "kairon.zyvor.dev/forked-from";
/// Stamped by kairon-controller with the node the Machine is assigned to.
pub const LABEL_ASSIGNED_NODE: &str = "kairon.zyvor.dev/assigned-node";
/// Label Veyron puts on every Machine it creates.
pub const LABEL_MANAGED_BY: &str = "app.kubernetes.io/managed-by";

pub const POWER_RUNNING: &str = "Running";
pub const POWER_STOPPED: &str = "Stopped";
pub const POWER_PAUSED: &str = "Paused";
pub const POWER_HALTED: &str = "Halted";

/// Valid `spec.powerState` values.
pub const POWER_STATES: &[&str] = &[POWER_RUNNING, POWER_STOPPED, POWER_PAUSED, POWER_HALTED];

/// Map a legacy KubeVirt-style run strategy (still accepted by the API and
/// CLI) onto a Kairon power state.
pub fn power_state_for_run_strategy(strategy: &str) -> Option<&'static str> {
    match strategy.trim().to_ascii_lowercase().as_str() {
        "always" | "rerunonfailure" | "running" | "once" => Some(POWER_RUNNING),
        "halted" | "manual" | "stopped" => Some(POWER_STOPPED),
        "paused" => Some(POWER_PAUSED),
        _ => None,
    }
}

impl Machine {
    pub fn name(&self) -> &str {
        self.metadata.name.as_deref().unwrap_or("")
    }

    pub fn namespace(&self) -> &str {
        self.metadata.namespace.as_deref().unwrap_or("default")
    }

    /// `status.phase`, or `"Unknown"` before the controller first reports.
    pub fn phase(&self) -> &str {
        self.status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .filter(|p| !p.is_empty())
            .unwrap_or("Unknown")
    }

    pub fn is_running(&self) -> bool {
        self.phase() == "Running"
    }

    /// `spec.powerState`, defaulting to `Running` the way kairon-controller does.
    pub fn power_state(&self) -> &str {
        self.spec
            .power_state
            .as_deref()
            .filter(|p| !p.is_empty())
            .unwrap_or(POWER_RUNNING)
    }

    pub fn node(&self) -> Option<&str> {
        self.status
            .as_ref()
            .and_then(|s| s.node_name.as_deref())
            .filter(|n| !n.is_empty())
            .or_else(|| self.spec.node_name.as_deref().filter(|n| !n.is_empty()))
    }

    pub fn runtime_id(&self) -> Option<&str> {
        self.status
            .as_ref()
            .and_then(|s| s.runtime_id.as_deref())
            .filter(|r| !r.is_empty())
    }

    pub fn guest_ips(&self) -> Vec<String> {
        let Some(st) = self.status.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        if let Some(ip) = st.guest_ip.as_deref().filter(|ip| !ip.is_empty()) {
            out.push(ip.to_string());
        }
        for ip in &st.guest_ips {
            if !ip.is_empty() && !out.contains(ip) {
                out.push(ip.clone());
            }
        }
        if out.is_empty() {
            if let Some(ip) = st
                .network
                .as_ref()
                .and_then(|n| n.get("guestIP"))
                .and_then(|v| v.as_str())
                .filter(|ip| !ip.is_empty())
            {
                out.push(ip.to_string());
            }
        }
        out
    }

    pub fn guest_ip(&self) -> Option<String> {
        self.guest_ips().into_iter().next()
    }

    /// vCPUs actually applied by kairon-node, else parsed from `spec.resources.cpu`.
    pub fn vcpus(&self) -> u32 {
        if let Some(n) = self.status.as_ref().and_then(|s| s.applied_vcpus) {
            if n > 0 {
                return n;
            }
        }
        parse_cpu(&self.spec.resources.cpu)
    }

    /// `spec.resources.memory` as written (e.g. `4Gi`).
    pub fn memory(&self) -> &str {
        &self.spec.resources.memory
    }

    pub fn memory_bytes(&self) -> u64 {
        if let Some(mib) = self.status.as_ref().and_then(|s| s.applied_memory_mib) {
            if mib > 0 {
                return mib * 1024 * 1024;
            }
        }
        parse_quantity_bytes(&self.spec.resources.memory).unwrap_or(0)
    }

    pub fn resource_usage(&self) -> Option<&ResourceUsage> {
        self.status.as_ref().and_then(|s| s.resource_usage.as_ref())
    }

    pub fn message(&self) -> Option<&str> {
        self.status
            .as_ref()
            .and_then(|s| s.message.as_deref())
            .filter(|m| !m.is_empty())
    }

    pub fn condition(&self, type_: &str) -> Option<&MachineCondition> {
        self.status
            .as_ref()
            .and_then(|s| s.conditions.iter().find(|c| c.type_ == type_))
    }

    /// Human status for tables: the phase, or the requested power state while
    /// the controller has not reported yet.
    pub fn printable_status(&self) -> String {
        match self.phase() {
            "Unknown" => match self.power_state() {
                POWER_RUNNING => "Pending".to_string(),
                other => other.to_string(),
            },
            p => p.to_string(),
        }
    }

    /// Boot image reference for display (`oci`, `httpURL`, catalog name, or path).
    pub fn image_ref(&self) -> Option<String> {
        let img = &self.spec.image;
        if let Some(src) = img.source.as_ref() {
            if let Some(o) = src.oci.as_deref().filter(|s| !s.is_empty()) {
                return Some(o.to_string());
            }
            if let Some(u) = src.http_url.as_deref().filter(|s| !s.is_empty()) {
                return Some(u.to_string());
            }
        }
        img.image_ref
            .clone()
            .or_else(|| img.catalog_name.clone())
            .filter(|s| !s.is_empty())
            .or_else(|| img.path.clone().filter(|s| !s.is_empty()))
            .or_else(|| {
                self.spec
                    .volumes
                    .first()
                    .map(|v| format!("pvc/{}", v.claim_name))
            })
    }

    /// Every PVC claim this Machine references (boot volume, shares, data disks).
    pub fn claim_names(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .spec
            .volumes
            .iter()
            .map(|v| v.claim_name.clone())
            .chain(self.spec.disks.iter().map(|d| d.claim_name.clone()))
            .filter(|c| !c.is_empty())
            .collect();
        out.dedup();
        out
    }

    pub fn has_device_claims(&self) -> bool {
        !self.spec.device_claims.is_empty()
    }
}

/// Parse a Kubernetes CPU quantity (`"2"`, `"1500m"`) to whole vCPUs (min 1 when set).
pub fn parse_cpu(q: &str) -> u32 {
    let q = q.trim();
    if q.is_empty() {
        return 0;
    }
    if let Some(m) = q.strip_suffix('m') {
        return m
            .parse::<u64>()
            .map(|v| v.div_ceil(1000).max(1) as u32)
            .unwrap_or(0);
    }
    q.parse::<f64>()
        .map(|v| v.ceil().max(1.0) as u32)
        .unwrap_or(0)
}

/// Parse a Kubernetes memory quantity (`4Gi`, `512Mi`, `2G`, `1073741824`) to bytes.
pub fn parse_quantity_bytes(q: &str) -> Option<u64> {
    let q = q.trim();
    if q.is_empty() {
        return None;
    }
    let units: &[(&str, u64)] = &[
        ("Ki", 1 << 10),
        ("Mi", 1 << 20),
        ("Gi", 1 << 30),
        ("Ti", 1 << 40),
        ("Pi", 1 << 50),
        ("k", 1_000),
        ("K", 1_000),
        ("M", 1_000_000),
        ("G", 1_000_000_000),
        ("T", 1_000_000_000_000),
    ];
    for (suffix, mult) in units {
        if let Some(n) = q.strip_suffix(suffix) {
            return n
                .trim()
                .parse::<f64>()
                .ok()
                .map(|v| (v * *mult as f64) as u64);
        }
    }
    q.parse::<f64>().ok().map(|v| v as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantities() {
        assert_eq!(parse_cpu("2"), 2);
        assert_eq!(parse_cpu("1500m"), 2);
        assert_eq!(parse_cpu("250m"), 1);
        assert_eq!(parse_quantity_bytes("4Gi"), Some(4 << 30));
        assert_eq!(parse_quantity_bytes("512Mi"), Some(512 << 20));
        assert_eq!(parse_quantity_bytes("1G"), Some(1_000_000_000));
    }

    #[test]
    fn run_strategy_mapping() {
        assert_eq!(power_state_for_run_strategy("Always"), Some(POWER_RUNNING));
        assert_eq!(power_state_for_run_strategy("Halted"), Some(POWER_STOPPED));
        assert_eq!(power_state_for_run_strategy("Manual"), Some(POWER_STOPPED));
        assert_eq!(power_state_for_run_strategy("bogus"), None);
    }

    #[test]
    fn printable_status_before_controller_reports() {
        let mut m = Machine::new("a", MachineSpec::default());
        assert_eq!(m.printable_status(), "Pending");
        m.spec.power_state = Some(POWER_STOPPED.into());
        assert_eq!(m.printable_status(), "Stopped");
        m.status = Some(MachineStatus {
            phase: Some("Running".into()),
            guest_ip: Some("10.1.1.1".into()),
            ..Default::default()
        });
        assert_eq!(m.printable_status(), "Running");
        assert_eq!(m.guest_ip().as_deref(), Some("10.1.1.1"));
    }
}
