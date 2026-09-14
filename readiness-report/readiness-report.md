# Veyron customer-readiness — ⛔ **NO-GO**

**NO-GO** — 30 passed, 1 failed, 3 warnings, 14 skipped (ceph_present_health_not_exposed:1, console_check_missing:1, no_prometheus_or_metric:1, no_pvc_backed_image:2, not_requested:1, nproc_unreadable:1, opt_in_required:5, single_node:2)

## Cluster

| Field | Value |
|---|---|
| Host | `80.79.5.173:30151` |
| Nodes | 1 |
| KubeVirt | v1.9.0 ✓ |
| CDI | v1.66.0 ✓ |
| Ceph | present |
| TLS issuer | CN=veyron, O=veyron |
| P1 acceptance gate | passed |

## Phases

| Phase | Pass | Fail | Warn | Skip |
|---|---|---|---|---|
| P0-node | 1 | 0 | 0 | 0 |
| P1-gate | 1 | 0 | 0 | 0 |
| setup | 0 | 0 | 1 | 0 |
| P2-smoke | 0 | 1 | 0 | 0 |
| P3 · Auth & TLS posture | 1 | 0 | 2 | 0 |
| P3 · RBAC enforcement matrix (auth_context.rs) | 8 | 0 | 0 | 0 |
| P3 · Namespace scoping | 3 | 0 | 0 | 0 |
| P3 · Cross-namespace clone RBAC | 0 | 0 | 0 | 1 |
| P4 · Snapshot round-trip | 6 | 0 | 0 | 2 |
| P4 · DR export → apply → verify | 2 | 0 | 0 | 0 |
| P4 · Orphan reclaim | 1 | 0 | 0 | 1 |
| P4 · Velero off-cluster backup | 0 | 0 | 0 | 1 |
| P5 · Live migration | 0 | 0 | 0 | 2 |
| P5 · Self-healing | 1 | 0 | 0 | 1 |
| P5 · Ceph / storage health | 0 | 0 | 0 | 1 |
| P6 · Upgrade path | 1 | 0 | 0 | 1 |
| P6 · Monitoring actually scrapes | 1 | 0 | 0 | 1 |
| P6 · Day-2 hotplug seen by guest | 1 | 0 | 0 | 1 |
| P6 · Version drift vs versions.env | 2 | 0 | 0 | 0 |
| P7-lifecycle | 1 | 0 | 0 | 0 |
| P8-dashboard | 0 | 0 | 0 | 1 |
| P9-windows | 0 | 0 | 0 | 1 |

## ⛔ Blocking failures (must fix before go-live)

- **[P2-smoke]** P2 read-only smoke (exit 1) — see /Users/ssahani/tt/veyron/readiness-report/logs/p2-smoke.log

## ⚠️ Warnings (advisory — do not block, but review)

- [setup] dedicated namespace unavailable — tests run in 'default' (resources still name-scoped veyron-readiness-* and cleaned up)
- [P3 · Auth & TLS posture] Default API key 'Admin@321' in use — rotate via VEYRON_API_KEY / veyron-api-key Secret before go-live
- [P3 · Auth & TLS posture] Self-signed / dev TLS cert (issuer: CN=veyron, O=veyron) — install a trusted cert before go-live

## ○ Skipped (not applicable)

- [P3 · Cross-namespace clone RBAC] cross-ns golden-image clone RBAC (denied→allowed) _(reason: opt_in_required:VEYRON_READY_ALLOW_CLONE_RBAC)_
- [P4 · Snapshot round-trip] in-guest data round-trip _(reason: no_pvc_backed_image:set VEYRON_READY_DATASOURCE)_
- [P4 · Snapshot round-trip] post-restore data verification _(reason: no_pvc_backed_image)_
- [P4 · Orphan reclaim] orphan reclaim real delete (confirm=true) _(reason: opt_in_required:VEYRON_READY_ALLOW_ORPHAN_DELETE)_
- [P4 · Velero off-cluster backup] Velero backup→restore round-trip _(reason: opt_in_required:VEYRON_READY_ALLOW_VELERO)_
- [P5 · Live migration] live migration (node A→B, IP unchanged, guest connected) _(reason: single_node)_
- [P5 · Live migration] cordon/drain reschedule _(reason: single_node)_
- [P5 · Self-healing] real self-healing (kill test VMI → assert recovery) _(reason: opt_in_required:VEYRON_READY_ALLOW_HEAL)_
- [P5 · Ceph / storage health] Ceph health _(reason: ceph_present_health_not_exposed:check ceph -s on the cluster)_
- [P6 · Upgrade path] real upgrade no-op bounce (assert VM survives) _(reason: opt_in_required:VEYRON_READY_ALLOW_UPGRADE)_
- [P6 · Monitoring actually scrapes] monitoring scrape _(reason: no_prometheus_or_metric:HTTP 200)_
- [P6 · Day-2 hotplug seen by guest] hotplug guest CPU visibility _(reason: nproc_unreadable)_
- [P8-dashboard] dashboard console sweep _(reason: console_check_missing)_
- [P9-windows] Windows golden-image E2E _(reason: not_requested:pass --full)_

---

_Verdict rule: **GO** iff zero failures **and** the P1 acceptance gate passed. Warnings and skips never block. Multi-node checks (live migration, drain) SKIP on single-node clusters — re-run against a ≥2-node cluster to exercise them._
