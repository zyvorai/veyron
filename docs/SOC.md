# Veyron Security Operations (SOC)

Veyron SOC provides a normalized security event stream, built-in threat detections, optional SIEM push (Elastic ECS, Splunk HEC, Microsoft Sentinel, QRadar LEEF), read-only threat hunts, attack-surface inventory, and SOAR webhooks.

## Dashboard

Open **Security → SOC** in the classic navbar (or Finder → Operations → SOC). The page shows:

- Open detections with acknowledge actions
- SIEM exporter status (from environment variables)
- Security event stream (ConfigMap buffer + cluster collectors)
- Threat hunt (Elastic KQL or Splunk SPL when enabled)
- Attack surface (RDP NodePort, SSH expose, internet-facing signals)

## API routes

| Method | Route | Purpose |
|--------|-------|---------|
| GET | `/api/v1/soc/events` | List events (`?refresh=true` to collect from cluster) |
| POST | `/api/v1/soc/events` | Force sync collectors |
| GET | `/api/v1/soc/detections` | Evaluate rules and merge ack state |
| GET | `/api/v1/soc/detections/:id` | Single detection |
| POST | `/api/v1/soc/detections/:id/ack` | Acknowledge (`{"acked_by":"analyst"}`) |
| GET | `/api/v1/soc/export/status` | Per-backend push status |
| GET | `/api/v1/soc/hunts` | Saved hunts from ConfigMaps |
| POST | `/api/v1/soc/hunts/run` | Run Elastic/Splunk query (`backend`, `query`, `time_range`) |
| GET | `/api/v1/soc/attack-surface` | VM exposure inventory |
| GET | `/api/v1/soc/playbooks` | SOAR playbook definitions |
| POST | `/api/v1/soc/playbooks/trigger` | Fire playbook webhook |

Namespace scoping uses the same `?namespace=` query as other dashboard pages.

## Persistence

Events and detections are stored in labeled ConfigMaps in the API namespace:

- `veyron.io/type=soc-events` — rolling buffer (~500 events)
- `veyron.io/type=soc-detections` — merged detection state including acks

Collectors ingest Kubernetes Events, VM security findings, and API audit mutations (non-GET routes on VMs, snapshots, RDP/SSH expose, guest-agent, `/soc/*`).

## Detection rules (Phase 1)

| Rule ID | Signal |
|---------|--------|
| `rdp-public-expose` | RDP NodePort service on a VM |
| `ssh-nodeport-expose` | SSH expose via NodePort/LoadBalancer |
| `no-network-policy-ns` | Namespace with VMs but zero NetworkPolicies |
| `veyron-drift` | VeyronVM `status.driftDetected` |
| `privileged-vm-spec` | Privileged virt-launcher domain spec |
| `burst-failed-scheduling` | Burst of FailedScheduling events |

New open detections append to the event stream and optionally call `VEYRON_SOAR_WEBHOOK_URL`.

## SIEM environment variables

Set on the API Deployment or `veyron-integrations` Secret (see `deploy/k8s/optional-integrations.env.example.yaml`).

### Elastic (ECS bulk)

| Variable | Required | Notes |
|----------|----------|-------|
| `VEYRON_ELASTIC_URL` | yes | e.g. `https://elastic:9200` |
| `VEYRON_ELASTIC_API_KEY` | yes | API key or encoded key |
| `VEYRON_ELASTIC_INDEX` | no | default `veyron-security` |
| `VEYRON_ELASTIC_PIPELINE` | no | ingest pipeline name |
| `VEYRON_ELASTIC_HUNT_ENABLED` | no | `true` for dashboard/API hunts |

### Splunk (HEC)

| Variable | Required |
|----------|----------|
| `VEYRON_SPLUNK_HEC_URL` | yes |
| `VEYRON_SPLUNK_HEC_TOKEN` | yes |
| `VEYRON_SPLUNK_INDEX` | no |
| `VEYRON_SPLUNK_SOURCETYPE` | no |
| `VEYRON_SPLUNK_HUNT_ENABLED` | no (`true` for SPL hunts) |

### Microsoft Sentinel (DCE)

| Variable | Required |
|----------|----------|
| `VEYRON_SENTINEL_DCE_URL` | yes |
| `VEYRON_SENTINEL_TENANT_ID` | yes |
| `VEYRON_SENTINEL_CLIENT_ID` | yes |
| `VEYRON_SENTINEL_CLIENT_SECRET` | yes |

### QRadar (LEEF UDP)

| Variable | Required |
|----------|----------|
| `VEYRON_QRADAR_HOST` | yes |
| `VEYRON_QRADAR_PORT` | no (default 514) |

### SOAR

| Variable | Purpose |
|----------|---------|
| `VEYRON_SOAR_WEBHOOK_URL` | POST JSON on new detections and playbook triggers |

A background task pushes recent events every 30 seconds when exporters are configured. Detections refresh also pushes a small batch.

## Integrations page

**Integrations** includes probes for `elastic`, `splunk`, `sentinel`, `qradar`, and `soar` (same env vars). Use **SOC → Integrations** or configure the Secret and restart `veyron-api`.

## Smoke test

```bash
export VEYRON_API_KEY=CHANGE_ME
BASE=https://YOUR_NODE_IP:30151

curl -sk -H "X-API-Key: $VEYRON_API_KEY" "$BASE/api/v1/soc/detections?namespace=all"
curl -sk -H "X-API-Key: $VEYRON_API_KEY" "$BASE/api/v1/soc/events?limit=10&refresh=true"
curl -sk -H "X-API-Key: $VEYRON_API_KEY" "$BASE/api/v1/soc/export/status"
curl -sk -H "X-API-Key: $VEYRON_API_KEY" "$BASE/api/v1/soc/attack-surface?namespace=all"
```

## Licensing

Core detections and Elastic ECS export are available in the open-source build. Enterprise packaging may bundle additional SIEM connectors and managed content; see `docs/legal/LICENSING-MODEL.md`.
