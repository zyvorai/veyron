// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub fn dns_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        && value.as_bytes()[0] != b'-'
        && value.as_bytes()[value.len() - 1] != b'-'
}

/// Exact integer resource parsing. Unsupported/scientific/fractional quantities fail closed.
pub fn quantity(value: &str, cpu: bool) -> Result<u64, String> {
    let units: &[(&str, u64)] = if cpu {
        &[("m", 1), ("", 1000)]
    } else {
        &[
            ("Ki", 1024),
            ("Mi", 1024 * 1024),
            ("Gi", 1024 * 1024 * 1024),
            ("Ti", 1024u64.pow(4)),
            ("K", 1000),
            ("M", 1000000),
            ("G", 1000000000),
            ("", 1),
        ]
    };
    for (suffix, multiplier) in units {
        if let Some(number) = value.strip_suffix(suffix)
            && !number.is_empty()
            && number.bytes().all(|c| c.is_ascii_digit())
        {
            return number
                .parse::<u64>()
                .ok()
                .and_then(|n| n.checked_mul(*multiplier))
                .ok_or_else(|| format!("quantity overflow: {value}"));
        }
    }
    Err(format!("unsupported quantity: {value}"))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capacity {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    #[serde(default)]
    pub gpu: u64,
}
impl Capacity {
    pub fn fits(&self, request: &Self) -> bool {
        self.cpu_millis >= request.cpu_millis
            && self.memory_bytes >= request.memory_bytes
            && self.gpu >= request.gpu
    }
    pub fn subtract(&mut self, request: &Self) {
        self.cpu_millis = self.cpu_millis.saturating_sub(request.cpu_millis);
        self.memory_bytes = self.memory_bytes.saturating_sub(request.memory_bytes);
        self.gpu = self.gpu.saturating_sub(request.gpu);
    }
    pub fn reserve_percent(&mut self, percent: u8) {
        let keep = u64::from(100 - percent);
        self.cpu_millis = (u128::from(self.cpu_millis) * u128::from(keep) / 100) as u64;
        self.memory_bytes = (u128::from(self.memory_bytes) * u128::from(keep) / 100) as u64;
        self.gpu = (u128::from(self.gpu) * u128::from(keep) / 100) as u64; // conservative: fractional GPU never allocates
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementNode {
    pub name: String,
    pub available: Capacity,
    pub ready: bool,
    pub schedulable: bool,
    #[serde(default)]
    pub tainted: bool,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementRequest {
    pub resources: Capacity,
    #[serde(default)]
    pub node_selector: BTreeMap<String, String>,
    #[serde(default)]
    pub excluded_nodes: BTreeSet<String>,
    #[serde(default = "default_headroom")]
    pub headroom_percent: u8,
}
fn default_headroom() -> u8 {
    10
}
#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub node: String,
    pub eligible: bool,
    pub reasons: Vec<String>,
    pub remaining: Capacity,
}

pub fn placement(
    nodes: &[PlacementNode],
    req: &PlacementRequest,
) -> Result<Vec<Candidate>, String> {
    if req.headroom_percent >= 100
        || req.resources.cpu_millis == 0
        || req.resources.memory_bytes == 0
    {
        return Err("CPU/memory must be positive; headroom must be below 100".into());
    }
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for n in nodes {
        if n.name.is_empty() || !seen.insert(&n.name) {
            return Err("node names must be unique and nonempty".into());
        }
        let mut reasons = Vec::new();
        if !n.ready {
            reasons.push("node is not Ready".into());
        }
        if !n.schedulable {
            reasons.push("node is cordoned".into());
        }
        if n.tainted {
            reasons.push("node has an untolerated scheduling taint".into());
        }
        if req.excluded_nodes.contains(&n.name) {
            reasons.push("node excluded by policy".into());
        }
        if req
            .node_selector
            .iter()
            .any(|(k, v)| n.labels.get(k) != Some(v))
        {
            reasons.push("node selector mismatch".into());
        }
        let mut remaining = n.available.clone();
        remaining.reserve_percent(req.headroom_percent);
        if !remaining.fits(&req.resources) {
            reasons.push("insufficient capacity after headroom".into());
        }
        let eligible = reasons.is_empty();
        if eligible {
            remaining.subtract(&req.resources);
        }
        result.push(Candidate {
            node: n.name.clone(),
            eligible,
            reasons,
            remaining,
        });
    }
    result.sort_by(|a, b| {
        b.eligible
            .cmp(&a.eligible)
            .then(a.remaining.memory_bytes.cmp(&b.remaining.memory_bytes))
            .then(a.node.cmp(&b.node))
    });
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceVm {
    pub name: String,
    pub resources: Capacity,
    pub migration_ready: bool,
    #[serde(default)]
    pub local_storage: bool,
    #[serde(default)]
    pub whole_gpu: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceRequest {
    pub source_node: String,
    pub vms: Vec<MaintenanceVm>,
    pub nodes: Vec<PlacementNode>,
    #[serde(default = "default_headroom")]
    pub headroom_percent: u8,
}
#[derive(Debug, Clone, Serialize)]
pub struct Move {
    pub vm: String,
    pub destination: Option<String>,
    pub blockers: Vec<String>,
}
pub fn maintenance(req: &MaintenanceRequest) -> Result<Vec<Move>, String> {
    if req.source_node.is_empty()
        || req.vms.len() > 1000
        || req.nodes.len() > 1000
        || req.headroom_percent >= 100
    {
        return Err("invalid maintenance bounds".into());
    }
    let mut nodes = req.nodes.clone();
    for node in &mut nodes {
        node.available.reserve_percent(req.headroom_percent);
    }
    let mut names = BTreeSet::new();
    let mut moves = Vec::new();
    let mut vms = req.vms.clone();
    vms.sort_by(|a, b| {
        b.resources
            .memory_bytes
            .cmp(&a.resources.memory_bytes)
            .then(a.name.cmp(&b.name))
    });
    for vm in vms {
        if !dns_label(&vm.name) || !names.insert(vm.name.clone()) {
            return Err("VM names must be unique DNS labels".into());
        }
        let mut blockers = Vec::new();
        if !vm.migration_ready {
            blockers.push("migration readiness evidence missing".into());
        }
        if vm.local_storage {
            blockers.push("local storage blocks live migration".into());
        }
        if vm.whole_gpu {
            blockers.push("whole GPU assignment blocks live migration".into());
        }
        let p = placement(
            &nodes,
            &PlacementRequest {
                resources: vm.resources.clone(),
                node_selector: BTreeMap::new(),
                excluded_nodes: BTreeSet::from([req.source_node.clone()]),
                headroom_percent: 0,
            },
        )?;
        let destination = if blockers.is_empty() {
            p.iter().find(|c| c.eligible).map(|c| c.node.clone())
        } else {
            None
        };
        if let Some(ref dest) = destination {
            nodes
                .iter_mut()
                .find(|n| &n.name == dest)
                .expect("candidate node")
                .available
                .subtract(&vm.resources);
        } else if blockers.is_empty() {
            blockers.push("no destination capacity".into());
        }
        moves.push(Move {
            vm: vm.name,
            destination,
            blockers,
        });
    }
    Ok(moves)
}

#[derive(Debug, Clone, Serialize)]
pub struct Assessment {
    pub ready: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub evidence_only: bool,
}
fn assessment(checks: &[(bool, &str)], warnings: &[&str]) -> Assessment {
    let blockers = checks
        .iter()
        .filter(|(ok, _)| !ok)
        .map(|(_, why)| why.to_string())
        .collect::<Vec<_>>();
    Assessment {
        ready: blockers.is_empty(),
        blockers,
        warnings: warnings.iter().map(|s| s.to_string()).collect(),
        evidence_only: true,
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationEvidence {
    pub conversion_complete: bool,
    pub virtio_drivers: bool,
    pub boot_verified: bool,
    pub network_mapped: bool,
    pub storage_mapped: bool,
    pub source_fenced: bool,
    pub application_verified: bool,
}
pub fn migration(e: &MigrationEvidence) -> Assessment {
    assessment(
        &[
            (e.conversion_complete, "h2kvm conversion not verified"),
            (e.virtio_drivers, "VirtIO drivers not verified"),
            (e.boot_verified, "target boot not verified"),
            (e.network_mapped, "network mapping not verified"),
            (e.storage_mapped, "storage mapping not verified"),
            (e.source_fenced, "source fencing not verified"),
            (e.application_verified, "application validation missing"),
        ],
        &["Evidence supplied by caller; this endpoint does not convert disks or execute cutover."],
    )
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GpuEvidence {
    pub device_advertised: bool,
    pub iommu: bool,
    pub guest_driver: bool,
    pub license_valid: bool,
    pub isolation_verified: bool,
    pub whole_gpu: bool,
    pub live_migration_requested: bool,
}
pub fn gpu(e: &GpuEvidence) -> Assessment {
    assessment(
        &[
            (e.device_advertised, "GPU resource not advertised"),
            (e.iommu, "IOMMU evidence missing"),
            (e.guest_driver, "guest driver evidence missing"),
            (e.license_valid, "license evidence missing"),
            (e.isolation_verified, "isolation evidence missing"),
            (
                !(e.whole_gpu && e.live_migration_requested),
                "whole GPU live migration unsupported",
            ),
        ],
        &["No hardware probes or driver installation are performed."],
    )
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryEvidence {
    pub snapshot_at: i64,
    pub incident_at: i64,
    pub recovered_at: i64,
    pub now: i64,
    pub target_rpo_seconds: u64,
    pub target_rto_seconds: u64,
    pub snapshot_ready: bool,
    pub isolated: bool,
    pub source_fenced: bool,
    pub boot_verified: bool,
    pub application_verified: bool,
}
pub fn recovery(e: &RecoveryEvidence) -> Assessment {
    let timestamps = e.snapshot_at >= 0
        && e.snapshot_at <= e.incident_at
        && e.incident_at <= e.recovered_at
        && e.recovered_at <= e.now;
    let rpo = e
        .incident_at
        .checked_sub(e.snapshot_at)
        .and_then(|n| u64::try_from(n).ok());
    let rto = e
        .recovered_at
        .checked_sub(e.incident_at)
        .and_then(|n| u64::try_from(n).ok());
    assessment(
        &[
            (timestamps, "invalid or future recovery timestamps"),
            (
                rpo.is_some_and(|n| n <= e.target_rpo_seconds),
                "RPO target missed",
            ),
            (
                rto.is_some_and(|n| n <= e.target_rto_seconds),
                "RTO target missed",
            ),
            (e.snapshot_ready, "snapshot not ready"),
            (e.isolated, "recovery isolation missing"),
            (e.source_fenced, "source fencing missing"),
            (e.boot_verified, "restored boot not verified"),
            (e.application_verified, "application health not verified"),
        ],
        &["Snapshot readiness is not proof of successful recovery; evidence is caller supplied."],
    )
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCluster {
    pub context: String,
    pub control_plane_id: String,
    pub ready: bool,
    pub storage_replication_verified: bool,
    pub isolated_recovery_verified: bool,
}
pub fn fleet(clusters: &[FleetCluster]) -> Assessment {
    let mut contexts = BTreeSet::new();
    let mut planes = BTreeSet::new();
    let unique = clusters.iter().all(|c| {
        !c.context.is_empty()
            && !c.control_plane_id.is_empty()
            && contexts.insert(&c.context)
            && planes.insert(&c.control_plane_id)
    });
    assessment(
        &[
            (clusters.len() >= 2, "two distinct clusters required"),
            (clusters.len() <= 100, "fleet exceeds 100 clusters"),
            (
                unique,
                "contexts/control planes must be unique and nonempty",
            ),
            (
                clusters.iter().all(|c| c.ready),
                "one or more clusters not ready",
            ),
            (
                clusters.iter().all(|c| c.storage_replication_verified),
                "replication not verified",
            ),
            (
                clusters.iter().all(|c| c.isolated_recovery_verified),
                "isolated recovery not verified",
            ),
        ],
        &["No cross-cluster connectivity, replication or failover is executed."],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn size(cpu: u64, mem: u64) -> Capacity {
        Capacity {
            cpu_millis: cpu,
            memory_bytes: mem,
            gpu: 0,
        }
    }
    fn node(name: &str) -> PlacementNode {
        PlacementNode {
            name: name.into(),
            available: size(8000, 16000),
            ready: true,
            schedulable: true,
            tainted: false,
            labels: BTreeMap::new(),
        }
    }
    #[test]
    fn zero_headroom_preserves_small_resources() {
        let mut r = Capacity {
            cpu_millis: 1,
            memory_bytes: 1,
            gpu: 1,
        };
        r.reserve_percent(0);
        assert_eq!(r.gpu, 1);
        assert_eq!(r.cpu_millis, 1);
        r.reserve_percent(10);
        assert_eq!(r.gpu, 0);
    }
    #[test]
    fn quantities_are_exact_and_bounded() {
        assert_eq!(quantity("1500m", true).unwrap(), 1500);
        assert_eq!(quantity("2Gi", false).unwrap(), 2147483648);
        assert!(quantity("1.5", true).is_err());
        assert!(quantity("18446744073709551615Gi", false).is_err());
    }
    #[test]
    fn dns_labels_reject_traversal() {
        for name in ["", "all", "../foo", "Foo", "-foo", "foo-"] {
            if name != "all" {
                assert!(!dns_label(name));
            }
        }
        assert!(dns_label("web-1"));
    }
    #[test]
    fn placement_reports_every_blocker() {
        let mut n = node("a");
        n.ready = false;
        n.tainted = true;
        let result = placement(
            &[n],
            &PlacementRequest {
                resources: size(9000, 20000),
                headroom_percent: 10,
                node_selector: BTreeMap::new(),
                excluded_nodes: BTreeSet::new(),
            },
        )
        .unwrap();
        assert!(!result[0].eligible);
        assert_eq!(result[0].reasons.len(), 3);
    }
    #[test]
    fn evacuation_does_not_double_book() {
        let req = MaintenanceRequest {
            source_node: "source".into(),
            nodes: vec![node("target")],
            headroom_percent: 0,
            vms: vec![
                MaintenanceVm {
                    name: "a".into(),
                    resources: size(6000, 10000),
                    migration_ready: true,
                    local_storage: false,
                    whole_gpu: false,
                },
                MaintenanceVm {
                    name: "b".into(),
                    resources: size(6000, 10000),
                    migration_ready: true,
                    local_storage: false,
                    whole_gpu: false,
                },
            ],
        };
        let result = maintenance(&req).unwrap();
        assert!(result[0].destination.is_some());
        assert!(result[1].destination.is_none());
    }
    #[test]
    fn recovery_needs_application_and_ordered_timestamps() {
        let mut e = RecoveryEvidence {
            snapshot_at: 100,
            incident_at: 200,
            recovered_at: 250,
            now: 300,
            target_rpo_seconds: 100,
            target_rto_seconds: 50,
            snapshot_ready: true,
            isolated: true,
            source_fenced: true,
            boot_verified: true,
            application_verified: false,
        };
        assert!(!recovery(&e).ready);
        e.application_verified = true;
        assert!(recovery(&e).ready);
        e.recovered_at = 400;
        assert!(!recovery(&e).ready);
    }
    #[test]
    fn gpu_cannot_claim_passthrough_migration() {
        assert!(
            !gpu(&GpuEvidence {
                device_advertised: true,
                iommu: true,
                guest_driver: true,
                license_valid: true,
                isolation_verified: true,
                whole_gpu: true,
                live_migration_requested: true
            })
            .ready
        );
    }
    #[test]
    fn duplicate_fleet_context_is_not_two_clusters() {
        let c = FleetCluster {
            context: "a".into(),
            control_plane_id: "b".into(),
            ready: true,
            storage_replication_verified: true,
            isolated_recovery_verified: true,
        };
        assert!(!fleet(&[c]).ready);
    }
    #[test]
    fn migration_requires_fencing() {
        assert!(
            !migration(&MigrationEvidence {
                conversion_complete: true,
                virtio_drivers: true,
                boot_verified: true,
                network_mapped: true,
                storage_mapped: true,
                source_fenced: false,
                application_verified: true
            })
            .ready
        );
    }
}
