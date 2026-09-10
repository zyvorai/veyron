# Admin Basics (Veyron)

## Ports

| Port | Service |
|------|---------|
| **8080** | Local HTTP API + dashboard (`veyron api-serve` / `serve`) |
| **5151** | Container / Helm HTTPS API + dashboard |
| **5150** / **30150** | HTTP→HTTPS redirect (chart) |
| **30151** | Common HTTPS NodePort for dashboard / API |

## Auth

- `VEYRON_API_KEY` (admin) or `VEYRON_API_KEYS="name:key:role,..."`.
- OIDC PKCE via `VEYRON_OIDC_*` (Keycloak/Auth0/Okta) — set `VEYRON_OIDC_GROUP_ADMIN` /
  `VEYRON_OIDC_GROUP_WRITE` to your IdP's group names (defaults: `veyron-admins,cluster-admins`
  / `veyron-write,veyron-editors`). Confidential IdP clients (Keycloak's default) also need
  `VEYRON_OIDC_CLIENT_SECRET`. Full walkthrough: [Setting Up SSO](sso-setup.md).
- WebSocket consoles use one-shot tickets: `POST /api/v1/ws/ticket`.
- Shell HTML (`/`, `/console`, `/console/assets/*`) and health/OIDC bootstrap paths are auth-exempt.
- The console's login screen takes username/password only (`POST /api/v1/auth/login`); API keys
  and OIDC are for API/CLI clients and programmatic flows — see [Getting Started](getting-started.md).

### Default credentials

On first install, if you don't set `auth.apiKey` / `auth.adminPassword`, the chart's
`veyron-api-key` Secret defaults both to **`CHANGE_ME`** — **change these before any
internet-facing or production install.**

`auth.jwtSecret` is different: it's the token *signing* key, not a login credential — a
predictable default there would let anyone forge a valid session token, not just guess a
password. It's auto-generated (random, 64 chars) on first install and preserved across
upgrades; never set it to a fixed value checked into source control.

Retrieve the generated values:

```bash
kubectl -n veyron get secret veyron-api-key -o jsonpath='{.data.api-key}' | base64 -d; echo
kubectl -n veyron get secret veyron-api-key -o jsonpath='{.data.admin-password}' | base64 -d; echo
```

Set your own at install time:

```bash
helm upgrade --install veyron charts/veyron -n veyron \
  --set auth.apiKey="a-real-key" --set auth.adminPassword="a-real-password"
# jwtSecret: leave unset to keep the auto-generated value, or override explicitly if needed
```

## Install sketch

```bash
# Local
veyron api-serve   # or: veyron serve

# Helm (example)
helm upgrade --install veyron charts/veyron -n veyron --create-namespace
```

### From a binary bundle (no git clone, no build tools)

If you're a customer, you'll receive **`veyron-<version>-linux-amd64.tar.gz`** directly
from us (email **sales@zyvor.dev** if you need it re-sent). It ships the Helm chart plus
a script that builds a runtime image from the bundle's own binaries — no source tree, no
cargo:

```bash
tar xzf veyron-*-linux-amd64.tar.gz && cd veyron-*-linux-amd64
./install-cluster.sh                                       # KubeVirt/CDI/Cilium prereqs
./install-to-kubernetes.sh --registry <your-registry>/veyron --push
./apply-cluster-network.sh
```

`install-to-kubernetes.sh --help` shows namespace/release/dry-run options, and a local
single-node cluster (kind/k3s/minikube) sharing the build daemon's image store can skip
`--push` entirely.

### Just evaluating? Grab the 30-day trial bundle

Download from **[github.com/zyvorai/veyron-releases](https://github.com/zyvorai/veyron-releases/releases)**
— same tarball, plus a signed `trial.token` next to the binary, no purchase needed to try
it. After 30 days the binary stops starting until you
[contact sales@zyvor.dev](mailto:sales@zyvor.dev) for a full, unrestricted license.

OpenAPI: `/api/openapi.json`. Route dump: `veyron api-routes`.

## Related

- [Getting Started](getting-started.md)
- [Using the Console](using-the-console.md)

