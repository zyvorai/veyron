# Veyron Administration & Configuration

> Part of the [Veyron Handbook](README.md) · see also
> [Product Guide](product-guide.md) · [FAQ](faq.md) ·
> [Troubleshooting](troubleshooting.md)

---

## Deployment models

Veyron runs as a single container/pod (the Rust API server) plus, optionally, the
Go operator. Choose one of the following.

| Model | How | When |
|-------|-----|------|
| **Helm (recommended)** | `charts/veyron` | Production; NodePort or Ingress front door. |
| **Operator Helm** | `charts/veyron-operator` | GitOps via `veyron.io` CRDs. |
| **Monitoring add-on** | `charts/veyron-monitoring` | Prometheus/Grafana dashboards + rules. |
| **Raw manifest** | `kubectl apply -f deploy/k8s.yaml` | Minimal/air-gapped install. |
| **Local build & run** | `cargo build --release && veyron api-serve` | Development against a kubeconfig. |
| **Scripted** | `scripts/deploy-k8s.sh`, `scripts/deploy-remote.sh`, `scripts/deploy-all-remote.sh` | Build+deploy locally or to a remote host over SSH. |

### Helm

```bash
helm upgrade --install veyron charts/veyron \
  --namespace veyron-system --create-namespace \
  --set image.tag=0.2.0 \
  --set auth.apiKey="$(openssl rand -hex 24)" \
  --set service.type=NodePort \
  --set service.nodePort=30151
```

Key chart defaults (`charts/veyron/values.yaml`): image
`ghcr.io/ssahani/veyron:latest`; resource names `veyron-api` (Deployment/Service),
`veyron` (ServiceAccount/ClusterRole); `service.type=NodePort`, `service.port=5151`,
`service.nodePort=30151`; `api.host=0.0.0.0`, `api.port=5151`,
`api.namespace=default`; `tls.enabled=true` with `tls.autoGenerate=true`;
`auth.secretName=veyron-api-key`; `rateLimit.perMinute=60`; PodDisruptionBudget and
a NetworkPolicy enabled; RBAC ClusterRole `veyron`.

### Scripted deploy

`scripts/deploy-k8s.sh` auto-detects the container runtime (docker/podman/nerdctl)
and K8s distro (k8s/k3s/kind/minikube), builds the image, and applies
`deploy/k8s.yaml`. Defaults: registry `ghcr.io/ssahani/veyron`, version from
`Cargo.toml`, namespace `veyron-system`.

```bash
./scripts/deploy-k8s.sh all       # build + deploy (default)
./scripts/deploy-k8s.sh status    # deployment status
./scripts/deploy-k8s.sh logs      # tail pod logs
./scripts/deploy-k8s.sh delete    # remove everything
```

Remote and packaging helpers: `scripts/deploy-remote.sh`,
`scripts/deploy-all-remote.sh`, `scripts/package-binary-remote.sh` (see
[PACKAGE_BINARY_REMOTE.md](../PACKAGE_BINARY_REMOTE.md)), and post-deploy checks
`scripts/verify-veyron-remote.sh HOST [30151]` and
`scripts/test-vm-daily-ops-remote.sh HOST [30151]`.

---

## Ports

| Where | Setting | Default | Notes |
|-------|---------|---------|-------|
| Local dev | `AppConfig.api.port` | **8080** | `veyron api-serve` with no flags; host `127.0.0.1`, TLS off. |
| Container | Dockerfile `CMD` | **5151** | `api-serve --port 5151 --host 0.0.0.0`. |
| Redirect | auto | **5150** | HTTP→HTTPS redirect on `port − 1` when TLS enabled (port 80 → 443 when port is 443). |
| Service | chart `service.nodePort` | **30151** | HTTPS NodePort → container 5151. |
| Service | `deploy/k8s.yaml` | **30150** | HTTP NodePort (port 80 → 5150 redirect). |

Override precedence for the port: `veyron api-serve --port` **>** `config.toml`
`[api] port` **>** built-in default (8080). In-container the value comes from the
Dockerfile `CMD` / chart `api.port`.

---

## Configuration file (`config.toml`)

Layered resolution (later layers win, then CLI flags):

1. Compiled defaults
2. `/etc/veyron/config.toml` (system-wide)
3. `~/.config/veyron/config.toml` (user)
4. CLI flags / environment

Manage it with `veyron config-init` (writes the user file; `--force` to overwrite)
and `veyron config-show` (`--path` to print the path only). The user file is
written `0600`.

```toml
namespace = "default"          # default -n / VEYRON_NAMESPACE
# kubeconfig = "/path/to/kubeconfig"   # overrides KUBECONFIG

[logging]
level  = "info"                # error | warn | info | debug | trace
format = "text"                # text | json
# file = "/var/log/veyron.log"

[api]
port            = 8080
host            = "127.0.0.1"
tls             = false
# tls_cert      = "/etc/veyron/tls/cert.pem"
# tls_key       = "/etc/veyron/tls/key.pem"
auth            = "none"        # none | api-key | bearer | basic | oauth2 | mtls
rate_limit      = 60            # requests/min, 0 = disabled
cors            = true
cors_origins    = []            # e.g. ["https://dashboard.example.com"]
request_timeout = 30            # seconds

[output]
format     = "table"           # table | yaml | json
color      = true
timestamps = false

[tui]
refresh_interval = 5
interactive      = false
splash           = true
```

---

## Environment variable reference

Environment variables override file config for the API server and enable optional
integrations. All are read from `std::env`; unset means the feature is disabled.

### Core

| Variable | Purpose |
|----------|---------|
| `VEYRON_NAMESPACE` | Default namespace (overrides `[api] namespace` / `-n`). |
| `VEYRON_CONFIG` | Alternate config file path. |
| `KUBECONFIG` | Kubeconfig path (standard k8s var; `config.toml` `kubeconfig` overrides it). |

### API & authentication

| Variable | Purpose |
|----------|---------|
| `VEYRON_API_KEY` | **Primary API key** (admin role). If unset, the API rejects all requests (`503 AUTH_NOT_CONFIGURED`). |
| `VEYRON_API_KEYS` | Additional keys with roles: `name:key:role,...` where role ∈ `admin` \| `write` \| `readonly`. |
| `VEYRON_CORS_ORIGINS` | Comma-separated allowed CORS origins. |
| `VEYRON_HTTP_REQUEST_TIMEOUT_SECS` | Upstream HTTP request timeout. |
| `VEYRON_API_NODE_HOST` / `VEYRON_API_NODE_PORT` | External node host/port used to build console & expose URLs (e.g. `30151`). |
| `VEYRON_API_CLUSTER_IP` / `VEYRON_CLUSTER_DNS` | Cluster IP / DNS used for internal URL generation. |

### SSO / OIDC / JWT

| Variable | Purpose |
|----------|---------|
| `VEYRON_OIDC_ISSUER` / `VEYRON_OIDC_CLIENT_ID` | OIDC provider + client. |
| `VEYRON_OIDC_CLIENT_SECRET` | **Required for confidential IdP clients** (Keycloak's default). Sent in the server-side token exchange; omit for a public/PKCE-only client. Missing it makes login fail silently — the browser-side redirect looks fine, the token exchange doesn't. |
| `VEYRON_OIDC_AUTHORIZATION_URL` / `VEYRON_OIDC_TOKEN_URL` / `VEYRON_OIDC_USERINFO_URL` / `VEYRON_OIDC_JWKS_URL` | OIDC endpoints. |
| `VEYRON_OIDC_REDIRECT_URI` / `VEYRON_OIDC_ROLE_CLAIM` | Redirect + role-claim mapping. |
| `VEYRON_OIDC_GROUP_ADMIN` / `VEYRON_OIDC_GROUP_WRITE` | Comma-separated, case-insensitive IdP group names mapped to `admin`/`write` (exact match). Defaults: `veyron-admins,cluster-admins` / `veyron-write,veyron-editors`. |
| `VEYRON_JWT_SECRET` / `VEYRON_JWT_ISSUER` / `VEYRON_JWT_ROLE_CLAIM` | JWT bearer verification. |

Deploy without leaking secrets through `--set`/CLI history: `./scripts/deploy-remote.sh <host> <user> --with-oidc` (writes a `veyron-oidc` Secret from the `VEYRON_OIDC_*` env vars in your shell). See [OIDC/SSO](../OIDC_SSO.md) for the full SSO setup, including a worked Keycloak walkthrough and the gotchas found live-testing against a real IdP (confidential-client secret, `sslRequired`, exact redirect URI, scope requests).

### AI (Ask Zyra / copilots)

| Variable | Purpose |
|----------|---------|
| `VEYRON_AI_MODE` | Enable/select AI mode. |
| `VEYRON_AI_URL` / `VEYRON_AI_API_KEY` / `VEYRON_AI_MODEL` | LLM endpoint, key, model. |
| `VEYRON_AI_APP_TITLE` / `VEYRON_AI_HTTP_REFERER` | OpenRouter-style headers. |
| `VEYRON_AI_MAX_TOOL_ROUNDS` / `VEYRON_AI_TIMEOUT_SECS` / `VEYRON_AI_RATE_LIMIT_PER_MIN` | Tool-loop / timeout / rate limits. |

See [Veyron AI](../VEYRON_AI.md).

### Observability & cost

| Variable | Purpose |
|----------|---------|
| `VEYRON_PROMETHEUS_URL` (+ `_EXTERNAL_URL`) | Metrics backend. |
| `VEYRON_LOKI_URL` / `VEYRON_LOKI_TOKEN` | Logs. |
| `VEYRON_JAEGER_QUERY_URL` (+ `_EXTERNAL_URL`) / `VEYRON_TEMPO_QUERY_URL` | Tracing. |
| `VEYRON_ALERTMANAGER_URL` / `_TOKEN` / `_EXTERNAL_URL` | Alerts. |
| `VEYRON_GRAFANA_URL` / `_EXTERNAL_URL` | Grafana deep links. |
| `VEYRON_COST_BACKEND` / `VEYRON_OPENCOST_URL` / `_TOKEN` | Cost reporting. |

### Security & GitOps

| Variable | Purpose |
|----------|---------|
| `VEYRON_TRIVY_URL` | Trivy server for image scanning. |
| `VEYRON_ARGOCD_URL` / `_TOKEN` / `_DEFAULT_APP` / `_EXTERNAL_URL` | Argo CD integration. |
| `VEYRON_FLUX_DEFAULT_KUSTOMIZATION` | Flux Kustomization default. |

### SIEM / SOC export

| Variable(s) | Purpose |
|-------------|---------|
| `VEYRON_SPLUNK_HEC_URL` / `_HEC_TOKEN` / `_INDEX` / `_SOURCETYPE` / `_REST_URL` | Splunk HEC export. |
| `VEYRON_QRADAR_SYSLOG_HOST` / `_SYSLOG_PORT` / `_LEEF_VENDOR` / `_LEEF_PRODUCT` | QRadar LEEF syslog. |
| `VEYRON_SENTINEL_DCE_URL` / `_DCR_RULE` / `_STREAM` / `_TENANT_ID` / `_CLIENT_ID` / `_CLIENT_SECRET` | Microsoft Sentinel. |
| `VEYRON_ELASTIC_URL` / `_API_KEY` / `_INDEX` / `_PIPELINE` / `_HUNT_ENABLED` | Elastic. |
| `VEYRON_SOAR_WEBHOOK_URL` | Generic SOAR webhook. |

See [SOC](../SOC.md).

### Notifications

| Variable(s) | Purpose |
|-------------|---------|
| `VEYRON_SLACK_WEBHOOK_URL` | Slack alerts. |
| `VEYRON_PAGERDUTY_ROUTING_KEY` | PagerDuty events. |
| `VEYRON_EMAIL_RELAY_URL` / `_RELAY_TOKEN` / `_FROM` / `_TEST_TO` | Email relay. |
| `VEYRON_SMS_WEBHOOK_URL` / `_WEBHOOK_TOKEN` / `_FROM` | SMS webhook. |

### Guest / migration / templates / scheduler

| Variable | Purpose |
|----------|---------|
| `VEYRON_GUESTKIT_BINARY` / `VEYRON_GUESTKIT_BINARY_URL` | GuestKit binary path / download URL. |
| `VEYRON_GUEST_RUNTIME` / `VEYRON_EMIT_GUEST_AGENT_CHANNELS` | Guest runtime + agent channel emission. |
| `VEYRON_VIRTIO_WIN_CONTAINER_DISK` | virtio-win container disk for Windows guests. |
| `VEYRON_TEMPLATE_REGISTRY` | OS template registry override. |
| `VEYRON_SCHEDULER_LEASE_DISABLED` / `VEYRON_SCHEDULER_LEASE_NAMESPACE` | Snapshot scheduler HA lease. |
| `VEYRON_PACKETWOLF_URL` / `_EXTERNAL_URL` / `_API_KEY` | packetwolf network-intel wiring. |

A ready-to-edit env template lives at
`deploy/k8s/optional-integrations.env.example.yaml`; see also
[Optional integrations](../OPTIONAL_INTEGRATIONS.md).

---

## Authentication

The web/REST API is protected by an **API-key gate** (see
`src/api/http_server.rs`):

- Set `VEYRON_API_KEY` (loaded as the `primary` key with **admin** role). Without
  it, every request returns `503 AUTH_NOT_CONFIGURED`.
- Add more keys via `VEYRON_API_KEYS` (`name:key:role`), roles `admin` / `write` /
  `readonly`.
- Clients present the key as `X-API-Key: <key>`, `Authorization: Bearer <key>`, or
  `?token=<key>`. Comparison is constant-time. The dashboard stores the key in
  `localStorage` and sends `X-API-Key` on every fetch.
- **WebSocket consoles** (VNC/serial/RDP) and the metrics stream use a short-lived
  ticket from `GET /api/v1/ws/ticket`, passed as `?ticket=`.
- CORS preflight (`OPTIONS`) is allowed without auth.
- Additional mechanisms: OIDC/SSO (`VEYRON_OIDC_*`, endpoints under
  `/api/v1/auth/oidc/*`), JWT bearer (`VEYRON_JWT_*`), and a PAM auth module.
- `veyron api-serve --auth` selects the method: `none`, `api-key`, `bearer`,
  `basic`, `oauth2`, `mtls`.

Manage keys/webhooks from the CLI: `veyron api-key-list|api-key-create|api-key-delete`,
`veyron webhook-list|webhook-create|webhook-delete`.

---

## TLS / certificates

- Enable with `[api] tls = true` + `tls_cert`/`tls_key`, or `veyron api-serve --tls
  --tls-cert … --tls-key …`.
- The server uses rustls (ring). When TLS is on, an HTTP→HTTPS redirect listener
  starts on `port − 1` (or port 80 when the main port is 443).
- In Helm, `tls.enabled=true` with `tls.autoGenerate=true` provisions a
  self-signed cert (an init step in `deploy/k8s.yaml` uses `alpine/openssl`); supply
  your own via `tls.existingSecret`.

---

## KubeVirt prerequisites

Veyron manages KubeVirt objects and needs:

1. A Kubernetes cluster with **KubeVirt** installed (`VirtualMachine`,
   `VirtualMachineInstance`, `VirtualMachineInstanceMigration`,
   `VirtualMachineSnapshot` CRDs available).
2. **CDI** (Containerized Data Importer) for disk imports / DataVolumes — helper
   `scripts/ensure-cdi-remote.sh`.
3. A working **kubeconfig** (`KUBECONFIG` or `config.toml` `kubeconfig`) with RBAC
   to read/write those resources. In-cluster, the chart's ServiceAccount +
   ClusterRole `veyron` provides this.
4. For live migration: a migration-capable storage/network setup (shared or
   migratable volumes).
5. For Windows guests: the virtio-win container disk (`VEYRON_VIRTIO_WIN_CONTAINER_DISK`);
   see [Windows on KubeVirt](../WINDOWS_KUBEVIRT_PRODUCTION.md).

---

## Building from source

Requires Rust ≥ 1.85 (edition 2024). GuestKit is a vendored submodule.

```bash
git clone https://github.com/ssahani/Veyron.git && cd Veyron
git submodule update --init --recursive     # brings in guestkit/
cargo build --release                        # default `web` feature on
./target/release/veyron --version

# Build without the web/API surface (CLI + TUI only)
cargo build --release --no-default-features

# Container image
docker build -t ghcr.io/ssahani/veyron:dev .
```

The Go operator builds separately under `operator/` (see its `Makefile`).

---

## Production checklist

- [ ] KubeVirt + CDI installed and healthy on the target cluster.
- [ ] `VEYRON_API_KEY` set to a strong random value (Helm `auth.apiKey` /
      `auth.secretName`); never ship the sample `CHANGE_ME`.
- [ ] TLS enabled (`tls.enabled=true`) with a real cert (`tls.existingSecret`) or
      accepted self-signed via `autoGenerate`.
- [ ] `rate_limit` and `cors_origins` scoped to your dashboard origin(s).
- [ ] Ingress/NodePort exposure decided; `VEYRON_API_NODE_HOST`/`_NODE_PORT` set so
      console/expose URLs resolve externally.
- [ ] SSO configured (`VEYRON_OIDC_*`) if you need per-user identity beyond API keys.
- [ ] Observability wired (`VEYRON_PROMETHEUS_URL`, Loki, etc.) and the
      `veyron-monitoring` chart installed for dashboards/alerts.
- [ ] RBAC ClusterRole reviewed; NetworkPolicy enabled.
- [ ] PodDisruptionBudget kept enabled; snapshot scheduler lease left on for HA.
- [ ] Post-deploy smoke test: `VEYRON_API_KEY=… scripts/verify-veyron-remote.sh HOST 30151`.
