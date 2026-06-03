# VMRogue Security Operations (SOC)

VMRogue SOC provides a normalized security event stream, built-in threat detections, optional SIEM push (Elastic ECS, Splunk HEC, Microsoft Sentinel, QRadar LEEF), read-only threat hunts, attack-surface inventory, and SOAR webhooks.

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

- `vmrogue.io/type=soc-events` — rolling buffer (~500 events)
- `vmrogue.io/type=soc-detections` — merged detection state including acks

Collectors ingest Kubernetes Events, VM security findings, and API audit mutations (non-GET routes on VMs, snapshots, RDP/SSH expose, guest-agent, `/soc/*`).

## Detection rules (Phase 1)

| Rule ID | Signal |
|---------|--------|
| `rdp-public-expose` | RDP NodePort service on a VM |
| `ssh-nodeport-expose` | SSH expose via NodePort/LoadBalancer |
| `no-network-policy-ns` | Namespace with VMs but zero NetworkPolicies |
| `vmrogue-drift` | VMRogueVM `status.driftDetected` |
| `privileged-vm-spec` | Privileged virt-launcher domain spec |
| `burst-failed-scheduling` | Burst of FailedScheduling events |

New open detections append to the event stream and optionally call `VMROGUE_SOAR_WEBHOOK_URL`.

## SIEM environment variables

Set on the API Deployment or `vmrogue-integrations` Secret (see `deploy/k8s/optional-integrations.env.example.yaml`).

### Elastic (ECS bulk)

| Variable | Required | Notes |
|----------|----------|-------|
| `VMROGUE_ELASTIC_URL` | yes | e.g. `https://elastic:9200` |
| `VMROGUE_ELASTIC_API_KEY` | yes | API key or encoded key |
| `VMROGUE_ELASTIC_INDEX` | no | default `vmrogue-security` |
| `VMROGUE_ELASTIC_PIPELINE` | no | ingest pipeline name |
| `VMROGUE_ELASTIC_HUNT_ENABLED` | no | `true` for dashboard/API hunts |

### Splunk (HEC)

| Variable | Required |
|----------|----------|
| `VMROGUE_SPLUNK_HEC_URL` | yes |
| `VMROGUE_SPLUNK_HEC_TOKEN` | yes |
| `VMROGUE_SPLUNK_INDEX` | no |
| `VMROGUE_SPLUNK_SOURCETYPE` | no |
| `VMROGUE_SPLUNK_HUNT_ENABLED` | no (`true` for SPL hunts) |

### Microsoft Sentinel (DCE)

| Variable | Required |
|----------|----------|
| `VMROGUE_SENTINEL_DCE_URL` | yes |
| `VMROGUE_SENTINEL_TENANT_ID` | yes |
| `VMROGUE_SENTINEL_CLIENT_ID` | yes |
| `VMROGUE_SENTINEL_CLIENT_SECRET` | yes |

### QRadar (LEEF UDP)

| Variable | Required |
|----------|----------|
| `VMROGUE_QRADAR_HOST` | yes |
| `VMROGUE_QRADAR_PORT` | no (default 514) |

### SOAR

| Variable | Purpose |
|----------|---------|
| `VMROGUE_SOAR_WEBHOOK_URL` | POST JSON on new detections and playbook triggers |

A background task pushes recent events every 30 seconds when exporters are configured. Detections refresh also pushes a small batch.

## Integrations page

**Integrations** includes probes for `elastic`, `splunk`, `sentinel`, `qradar`, and `soar` (same env vars). Use **SOC → Integrations** or configure the Secret and restart `vmrogue-api`.

## Smoke test

```bash
export VMROGUE_API_KEY=Admin@321
BASE=https://212.8.252.194:30151

curl -sk -H "X-API-Key: $VMROGUE_API_KEY" "$BASE/api/v1/soc/detections?namespace=all"
curl -sk -H "X-API-Key: $VMROGUE_API_KEY" "$BASE/api/v1/soc/events?limit=10&refresh=true"
curl -sk -H "X-API-Key: $VMROGUE_API_KEY" "$BASE/api/v1/soc/export/status"
curl -sk -H "X-API-Key: $VMROGUE_API_KEY" "$BASE/api/v1/soc/attack-surface?namespace=all"
```

## Licensing

Core detections and Elastic ECS export are available in the open-source build. Enterprise packaging may bundle additional SIEM connectors and managed content; see `docs/legal/LICENSING-MODEL.md`.
