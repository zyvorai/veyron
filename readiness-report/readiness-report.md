# Veyron customer-readiness — ⛔ **NO-GO**

**NO-GO** — 1 passed, 1 failed, 0 warnings, 0 skipped

## Cluster

| Field | Value |
|---|---|
| Host | `HOST:30151` |
| Nodes | 0 |
| KubeVirt | v1.9.0 ⚠️ drift (baseline v1.8.4) |
| CDI | v1.66.0 ⚠️ drift (baseline v1.65.0) |
| Ceph | absent |
| TLS issuer | CN=veyron, O=veyron |
| P1 acceptance gate | FAILED |

## Phases

| Phase | Pass | Fail | Warn | Skip |
|---|---|---|---|---|
| P0-node | 1 | 0 | 0 | 0 |
| P1-gate | 0 | 1 | 0 | 0 |

## ⛔ Blocking failures (must fix before go-live)

- **[P1-gate]** P1 acceptance gate (exit 1) — see /Users/ssahani/tt/veyron/readiness-report/logs/p1-gate.log

---

_Verdict rule: **GO** iff zero failures **and** the P1 acceptance gate passed. Warnings and skips never block. Multi-node checks (live migration, drain) SKIP on single-node clusters — re-run against a ≥2-node cluster to exercise them._
