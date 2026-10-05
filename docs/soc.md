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

Events come from Kubernetes Events, VM security findings and the API's own audit trail (every
mutating call on VMs, snapshots, expose routes and guest commands). New detections join the event
stream and can call `VEYRON_SOAR_WEBHOOK_URL`.

## API

| Method | Route | Purpose |
|---|---|---|
| GET | `/api/v1/soc/events` | Event stream (`?refresh=true` collects now) |
| GET | `/api/v1/soc/detections` | Evaluate rules, include acknowledgements |
| POST | `/api/v1/soc/detections/:id/ack` | Acknowledge (`{"acked_by":"analyst"}`) |
| GET | `/api/v1/soc/export/status` | Status per SIEM backend |
| POST | `/api/v1/soc/hunts/run` | Elastic KQL or Splunk SPL hunt |
| GET | `/api/v1/soc/attack-surface` | VM exposure inventory |
| GET | `/api/v1/soc/playbooks` | SOAR playbooks |
| POST | `/api/v1/soc/playbooks/trigger` | Fire a playbook (admin) |

State lives in labeled ConfigMaps: `veyron.io/type=soc-events`, a rolling buffer of about 500
events, and `veyron.io/type=soc-detections`.

## SIEM export

When an exporter is configured, a background task pushes recent events every 30 seconds.

| Backend | Variables |
|---|---|
| Elastic (ECS bulk) | `VEYRON_ELASTIC_URL`, `VEYRON_ELASTIC_API_KEY`, optional `_INDEX`, `_PIPELINE`, `_HUNT_ENABLED` |
| Splunk (HEC) | `VEYRON_SPLUNK_HEC_URL`, `VEYRON_SPLUNK_HEC_TOKEN`, optional `_INDEX`, `_SOURCETYPE`, `_HUNT_ENABLED` |
| Microsoft Sentinel | `VEYRON_SENTINEL_DCE_URL`, `_TENANT_ID`, `_CLIENT_ID`, `_CLIENT_SECRET` |
| QRadar (LEEF over UDP) | `VEYRON_QRADAR_HOST`, optional `VEYRON_QRADAR_PORT` (514) |
| SOAR | `VEYRON_SOAR_WEBHOOK_URL` |

Set them on the API Deployment or in the `veyron-integrations` Secret
(template: [`deploy/k8s/optional-integrations.env.example.yaml`](../deploy/k8s/optional-integrations.env.example.yaml)).

## Smoke test

```bash
BASE=https://<node-ip>:30151; H="X-API-Key: $VEYRON_API_KEY"
curl -sk -H "$H" "$BASE/api/v1/soc/detections?namespace=all"
curl -sk -H "$H" "$BASE/api/v1/soc/export/status"
```
