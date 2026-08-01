# Veyron customer-readiness — ⛔ **NO-GO**

**NO-GO** — 32 passed, 1 failed, 5 warnings, 9 skipped (ceph_present_health_not_exposed:1, inconclusive:1, no_guest_agent:1, no_pvc_backed_image:2, no_ro_key:1, no_write_key:1, single_node:2)

## Cluster

| Field | Value |
|---|---|
| Host | `212.8.248.187:30151` |
| Nodes | 1 |
| KubeVirt | v1.8.4 ✓ |
| CDI | v1.65.0 ✓ |
| Ceph | present |
| TLS issuer | CN=veyron, O=veyron |
| P1 acceptance gate | passed |

## Phases

| Phase | Pass | Fail | Warn | Skip |
|---|---|---|---|---|
| P0-node | 1 | 0 | 0 | 0 |
| P1-gate | 1 | 0 | 0 | 0 |
| setup | 0 | 0 | 1 | 0 |
| P2-smoke | 1 | 0 | 0 | 0 |
| P3 · Auth & TLS posture | 1 | 0 | 2 | 0 |
| P3 · RBAC enforcement matrix (auth_context.rs) | 0 | 0 | 1 | 2 |
| P3 · Namespace scoping | 3 | 0 | 0 | 0 |
| P3 · Cross-namespace clone RBAC | 0 | 0 | 1 | 1 |
| P4 · Snapshot round-trip | 6 | 0 | 0 | 2 |
| P4 · DR export → apply → verify | 2 | 0 | 0 | 0 |
| P4 · Orphan reclaim | 2 | 0 | 0 | 0 |
| P4 · Velero off-cluster backup | 1 | 0 | 0 | 0 |
| P5 · Live migration | 0 | 0 | 0 | 2 |
| P5 · Self-healing | 4 | 0 | 0 | 0 |
| P5 · Ceph / storage health | 0 | 0 | 0 | 1 |
| P6 · Upgrade path | 4 | 0 | 0 | 0 |
| P6 · Monitoring actually scrapes | 2 | 0 | 0 | 0 |
| P6 · Day-2 hotplug seen by guest | 0 | 0 | 0 | 1 |
| P6 · Version drift vs versions.env | 2 | 0 | 0 | 0 |
| P7-lifecycle | 1 | 0 | 0 | 0 |
| P8-dashboard | 1 | 0 | 0 | 0 |
| P9-windows | 0 | 1 | 0 | 0 |

## ⛔ Blocking failures (must fix before go-live)

- **[P9-windows]** P9 Windows golden image (exit 1) — see /Users/ssahani/tt/veyron/readiness-report-20260722-r2/logs/p9-windows.log

## ⚠️ Warnings (advisory — do not block, but review)

- [setup] dedicated namespace unavailable — tests run in 'default' (resources still name-scoped veyron-readiness-* and cleaned up)
- [P3 · Auth & TLS posture] Default API key 'Admin@321' in use — rotate via VEYRON_API_KEY / veyron-api-key Secret before go-live
- [P3 · Auth & TLS posture] Self-signed / dev TLS cert (issuer: CN=veyron, O=veyron) — install a trusted cert before go-live
- [P3 · RBAC enforcement matrix (auth_context.rs)] RBAC enforcement matrix not exercised — supply VEYRON_API_KEYS with readonly+write+admin roles for full coverage
- [P3 · Cross-namespace clone RBAC] could not apply cloner RoleBinding (manual step)

## ○ Skipped (not applicable)

- [P3 · RBAC enforcement matrix (auth_context.rs)] RBAC read-only key denials _(reason: no_ro_key:set VEYRON_RO_KEY)_
- [P3 · RBAC enforcement matrix (auth_context.rs)] RBAC write key admin-gate denials _(reason: no_write_key:set VEYRON_WRITE_KEY)_
- [P3 · Cross-namespace clone RBAC] cross-ns clone denial _(reason: inconclusive:{"status":500,"success":false,"data":null,"error":{"code":"CREATE_FAILED","message":"Namespace not found — create the )_
- [P4 · Snapshot round-trip] in-guest data round-trip _(reason: no_pvc_backed_image:set VEYRON_READY_DATASOURCE)_
- [P4 · Snapshot round-trip] post-restore data verification _(reason: no_pvc_backed_image)_
- [P5 · Live migration] live migration (node A→B, IP unchanged, guest connected) _(reason: single_node)_
- [P5 · Live migration] cordon/drain reschedule _(reason: single_node)_
- [P5 · Ceph / storage health] Ceph health _(reason: ceph_present_health_not_exposed:check ceph -s on the cluster)_
- [P6 · Day-2 hotplug seen by guest] day-2 hotplug guest verification _(reason: no_guest_agent)_

---

_Verdict rule: **GO** iff zero failures **and** the P1 acceptance gate passed. Warnings and skips never block. Multi-node checks (live migration, drain) SKIP on single-node clusters — re-run against a ≥2-node cluster to exercise them._
