<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Security operations (SOC)

Veyron watches its own fleet. It normalizes security events, runs built-in detections, pushes to
your SIEM, runs threat hunts, maps the attack surface and fires SOAR webhooks.

## What it detects

| Rule | Signal |
|---|---|
| `rdp-public-expose` | A VM's RDP is published on a NodePort |
| `ssh-nodeport-expose` | SSH exposed through a NodePort or LoadBalancer |
| `no-network-policy-ns` | A namespace with VMs and no NetworkPolicy |
| `veyron-drift` | A VM drifted from its declared spec |
| `privileged-vm-spec` | A privileged VM runtime spec |
| `burst-failed-scheduling` | A burst of scheduling failures |

Events come from Kubernetes Events, VM security findings and the API's own audit trail. New detections join the
event stream and can call `VEYRON_SOAR_WEBHOOK_URL`.

**What the audit trail records.** Every non-`GET` call whose path touches `/vms/`, `/snapshots`, `/expose` (and
`/rdp-expose`), `/guest-agent`, `/soc/`, `/ai/proposals`, `/ai/models` or `/sandboxes`, written after authentication
succeeds (a request rejected with `401` or `403` by the role check is not recorded). Each entry carries the method,
the path, success or failure (any non-2xx answer), and the actor: the API key's name, the local username, or the
JWT/OIDC `sub` or email. Copilot plan steps are recorded separately with the plan id and prompt
([COPILOT_AGENT.md](COPILOT_AGENT.md#audit)). The `GET /audit/trail` route is something else: it lists Kubernetes
Events.

**Bounds (0.4.1).** The log lives in one ConfigMap, and a ConfigMap tops out at 1 MiB. Request paths are cut at 256
bytes and actors at 128, and the stored log keeps the newest 500 events that fit in 900 KB, dropping the oldest first,
so a client sending very long URLs can no longer fill the ConfigMap and make later audit writes fail silently.

## API

| Method | Route | Role | Purpose |
|---|---|---|---|
| GET | `/api/v1/soc/events` | read | Event stream, newest first (`?refresh=true` collects now, `severity`, `category`, `limit` up to 500, default 200) |
| GET | `/api/v1/soc/audit` | read | Query and export the API audit trail: filter by `actor`, `action` (HTTP method), `outcome`, `path` (substring), `namespace`, `vm`, `source`, `plan_id`, `since` (inclusive) and `until` (exclusive) as RFC 3339, `order=asc\|desc`, `limit` (default 200, max 500), and `format=json\|ndjson\|csv` |
| POST | `/api/v1/soc/events` | write | Collect now and store the new events |
| GET | `/api/v1/soc/detections` | read | Evaluate rules, include acknowledgements. Not a pure read: it saves the result, adds open detections to the event log, pushes recent events to the SIEM and fires the SOAR webhook for new ones, whatever the caller's role |
| GET | `/api/v1/soc/detections/:id` | read | One detection |
| POST | `/api/v1/soc/detections/:id/ack` | write | Acknowledge (`{"acked_by":"analyst"}`) |
| GET | `/api/v1/soc/export/status` | read | Status per SIEM backend |
| GET | `/api/v1/soc/hunts` | read | Saved hunts (ConfigMaps `veyron.io/type=soc-hunt`) |
| POST | `/api/v1/soc/hunts/run` | write | Hunt: `{"backend":"elastic"\|"splunk","query":"...","time_range":"24h"}` |
| GET | `/api/v1/soc/attack-surface` | read | VM exposure inventory |
| GET | `/api/v1/soc/playbooks` | read | SOAR playbooks (ConfigMaps `veyron.io/type=soc-playbook`; the `VEYRON_SOAR_WEBHOOK_URL` one when none exist) |
| POST | `/api/v1/soc/playbooks/trigger` | admin | Fire a playbook |

### Querying and exporting the audit trail

`GET /api/v1/soc/audit` reads the stored log (it never collects or writes) and returns only audit events. Filters
combine with AND; text filters other than `path` match exactly and ignore case. Bad input answers `400` with an
`error` message.

```bash
# Everything one key did to VMs in November, as CSV for a spreadsheet or evidence bundle
curl -sk -H "Authorization: Bearer $KEY" \
  "https://veyron:5151/api/v1/soc/audit?actor=ci&path=/vms/&since=2026-11-01T00:00:00Z&until=2026-12-01T00:00:00Z&format=csv" > audit.csv

# Failed calls, newest first, as JSON: {"events": [...], "matched": 12, "returned": 12}
curl -sk -H "Authorization: Bearer $KEY" "https://veyron:5151/api/v1/soc/audit?outcome=failure"
```

`format=ndjson` gives one event per line for log shippers; `csv` has the columns `timestamp,id,source,actor,action,outcome,
target,namespace,vm,plan_id,message`. CSV fields that start with `=`, `+`, `-`, `@`, tab or CR are prefixed with `'`
so an attacker-chosen path or actor cannot become a spreadsheet formula. `matched` counts every hit; `returned` is
what fit under `limit`. The window is bounded by the stored log (newest 500 events, see the bounds above), so for
long retention ship events to a SIEM ([SIEM export](#siem-export)). Here `?namespace=` filters on the event's
namespace; it does not pick a different log.

State lives in labeled ConfigMaps in Veyron's own namespace (`VEYRON_NAMESPACE`; Helm `api.namespace`):
`veyron-soc-events` (`veyron.io/type=soc-events`, the rolling buffer above) and `veyron-soc-detections`
(`veyron.io/type=soc-detections`). Most routes accept `?namespace=` to scope the cluster scan.

## SIEM export

When an exporter is configured, a background task pushes recent events every 30 seconds.

| Backend | Variables |
|---|---|
| Elastic (ECS bulk) | `VEYRON_ELASTIC_URL`, `VEYRON_ELASTIC_API_KEY`, optional `_INDEX`, `_PIPELINE`. Hunts also need `VEYRON_ELASTIC_HUNT_ENABLED=true` |
| Splunk (HEC) | `VEYRON_SPLUNK_HEC_URL`, `VEYRON_SPLUNK_HEC_TOKEN`, optional `_INDEX`, `_SOURCETYPE` (default `veyron:security`). Hunts also need `VEYRON_SPLUNK_REST_URL` (the search endpoint; the same token is used) |
| Microsoft Sentinel | `VEYRON_SENTINEL_DCE_URL`, `_DCR_RULE`, `_TENANT_ID`, `_CLIENT_ID`, `_CLIENT_SECRET`, optional `_STREAM` (default `Custom-VeyronSecurity`) |
| QRadar (LEEF over UDP) | `VEYRON_QRADAR_HOST` (alias `_SYSLOG_HOST`), optional `VEYRON_QRADAR_PORT` (514, alias `_SYSLOG_PORT`), `_LEEF_VENDOR` (`Zyvor`), `_LEEF_PRODUCT` (`Veyron`) |
| SOAR | `VEYRON_SOAR_WEBHOOK_URL` |

Set them on the API Deployment or in the `veyron-integrations` Secret
(template: [`deploy/k8s/optional-integrations.env.example.yaml`](../deploy/k8s/optional-integrations.env.example.yaml)).

## Smoke test

```bash
BASE=https://<node-ip>:30151; H="X-API-Key: $VEYRON_API_KEY"
curl -sk -H "$H" "$BASE/api/v1/soc/detections?namespace=all"
curl -sk -H "$H" "$BASE/api/v1/soc/export/status"
```
