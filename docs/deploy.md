<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Deploy

Ways to run Veyron: the release binaries, a remote deploy script, Helm, or plain manifests, plus a standalone
client tarball.

## Release binaries

Each GitHub release (v0.4.0 is the latest) carries `veyron-linux-amd64`, `veyron-linux-arm64`,
`veyron-darwin-amd64`, `veyron-darwin-arm64` and `SHA256SUMS`. The console is embedded. Linux builds link
glibc dynamically; the darwin builds are only ad-hoc signed (no Developer ID, not notarized). v0.4.0 has no cosign bundle or SBOM, so
the checksum file is the only integrity check, and it comes from the same release page as the binaries.

```bash
V=v0.4.0; A=linux-amd64        # linux-arm64, darwin-amd64 or darwin-arm64
curl -fsSLO https://github.com/zyvorai/veyron/releases/download/$V/veyron-$A
curl -fsSLO https://github.com/zyvorai/veyron/releases/download/$V/SHA256SUMS
grep " veyron-$A\$" SHA256SUMS | (sha256sum -c - 2>/dev/null || shasum -a 256 -c -)   # expect "veyron-$A: OK"
chmod +x veyron-$A && sudo install -m 0755 veyron-$A /usr/local/bin/veyron
veyron --version               # veyron 0.4.0
```

Run the API and console with `veyron api-serve`:

| Flag | Meaning |
|---|---|
| `-p, --port` | Listen port (config default 8080; the in-cluster Deployment uses 5151) |
| `--host` | Bind address (config default `127.0.0.1`; the Deployment uses `0.0.0.0`) |
| `--tls`, `--tls-cert`, `--tls-key` | Serve HTTPS from a PEM certificate and key. Both paths are required with `--tls`. Without `--tls` the server speaks plain HTTP, so API keys cross the network in clear text |
| `--rate-limit` | Global requests per minute (default 60; `0` disables the limit). Health, console assets and high-frequency operator GET polling are exempt |
| `--auth` | Accepted for compatibility. The HTTP server always requires an API key, JWT or OIDC bearer; this flag does not change that |
| `--namespace` (global, `VEYRON_NAMESPACE`) | Namespace for Veyron's own state (the `veyron-users` Secret, settings ConfigMaps) |

With `--tls` the server also opens a plain-HTTP redirect listener (port 80 when the HTTPS port is 443, otherwise `port - 1`, for example 5150)
that sends browsers to HTTPS (the target host is `VEYRON_PUBLIC_HOST` when set; otherwise the request's `Host` header, accepted only if it is a well-formed hostname or IP, else `400`. Set `VEYRON_PUBLIC_HOST` in any deployment reachable by untrusted clients).

Settings come from the environment (the same variables the manifests set):

| Variable | Notes |
|---|---|
| `VEYRON_API_KEY` | **Required.** The primary key, role `admin`. When it is unset every request answers `503 AUTH_NOT_CONFIGURED`, even if `VEYRON_API_KEYS`, JWT or OIDC are set |
| `VEYRON_API_KEYS` | More keys as `name:key:role,...` (`admin`, `write`, `readonly`; a missing or unknown role means `readonly`). Keys must not contain `:` or `,` |
| `VEYRON_JWT_SECRET`, `VEYRON_JWT_ISSUER`, `VEYRON_JWT_ROLE_CLAIM` | HS256 secret and issuer for session tokens (console sign-in) and for your own JWTs; role claim defaults to `role` |
| `VEYRON_BOOTSTRAP_ADMIN`, `_USER`, `_PASSWORD`, `_SYNC` | First console account, see [getting-started.md](getting-started.md#default-credentials) |
| `VEYRON_OIDC_*` | Single sign-on, see [sso.md](sso.md) |
| `VEYRON_PUBLIC_HOST` | Public hostname used by the HTTP-to-HTTPS redirect listener (with `--tls`); the `Host` header is then ignored |
| `VEYRON_IMAGE_STORE_MAX_BYTES` | Size cap for one image-store upload or capture (default 64 GiB; `0` disables the cap). Larger uploads answer `413 UPLOAD_TOO_LARGE` |
| `VEYRON_KUBEVIRT_WS_INSECURE_TLS` | `1` skips verification of the Kubernetes API certificate on the legacy KubeVirt VNC/serial proxy (verified against the cluster CA by default) |
| `VEYRON_CORS_ORIGINS` | Comma-separated extra origins. Unset means same-origin only |
| `VEYRON_VM_BACKEND` | Unset, `auto` or `kairon`: Kairon. `kubevirt`: legacy |
| `VEYRON_HTTP_REQUEST_TIMEOUT_SECS` | Per-request timeout for ordinary routes (default 30) |
| `KUBECONFIG` | Cluster access outside a pod (in a pod the ServiceAccount is used) |

Needs a reachable Kubernetes API and, for VM routes, Kairon's CRDs (see [Kairon settings](#kairon-settings)).
Only the in-cluster Deployment (sections below) comes with the image-store volume and the relay Secret wired up.

The `veyron` CLI's VM commands (`create`, `list`, `start`, ...) manage KubeVirt `VirtualMachine`s only in
v0.4.0; they do not manage Kairon Machines.

## 1. Remote deploy script (fastest)

Syncs the source to a cluster node over SSH, builds the API image there (layer cache on by
default), imports it into the node runtime (k3s or plain containerd) and applies
[`deploy/k8s.yaml`](../deploy/k8s.yaml) in `veyron-system`:

```bash
./scripts/deploy-remote.sh <host> <user>             # full build and deploy
./scripts/deploy-remote.sh <host> <user> --quick     # re-apply manifests only
./scripts/deploy-remote.sh <host> <user> --with-oidc # also write the veyron-oidc Secret
```

Useful environment variables:

| Variable | Effect |
|---|---|
| `VEYRON_DEPLOY_NO_CACHE=1` | Clean image rebuild |
| `VEYRON_SKIP_GUESTKIT_PREP=1` | Skip re-syncing the GuestKit build context |
| `VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP=1` | Don't apply the Cilium egress policies |
| `VEYRON_INSTALL_CDI=1` | Install CDI too (legacy KubeVirt only; Kairon needs none) |
| `DEPLOY_SSH_TIMEOUT`, `DEPLOY_SSH_PORT` | SSH settings |

The API serves HTTPS on `hostPort` 5151 and NodePort 30151 with a self-signed certificate made by an
init container. Then check it:

```bash
./scripts/verify-veyron-remote.sh <host>
./scripts/preflight-veyron-remote.sh <host>      # grades cluster capabilities, exits non-zero on failure
VEYRON_API_KEY=... ./scripts/console-audit.sh --host <host>   # layout + console errors on every console page
VEYRON_API_KEY=... ./scripts/test-ai-remote.sh <host>        # Veyron AI end-to-end
```

On an existing cluster, `scripts/cluster/adapt-existing-cluster.sh` reports what Veyron needs
(dry run by default). `--apply` fixes it in place, including two settings that are easy to miss:

- **Cilium socket LB `hostns-only`**: without it, legacy KubeVirt guests can't reach cluster DNS, so guest
  agents never connect and sandboxes never become ready. Restarts the Cilium DaemonSet.
- **Terminated-pod GC threshold** (1000): the k3s default of 12500 lets dead pods pile up and fill
  the disk. Restarts k3s.

## Kairon settings

Veyron runs VMs on Kairon. It needs no KubeVirt and no CDI, but it does need:

- **The kairon-node relay** for consoles, guest exec, logs and VM-state snapshots. Put
  `VEYRON_KAIRON_NODE_URL` (for example `http://{node_ip}:8090`) and `KAIRON_NODE_CONSOLE_TOKEN` in
  the optional `veyron-kairon` Secret; `deploy/k8s.yaml` loads it when present. Write `{node_ip}`
  (or `{node}`) literally: Veyron fills it in with the address of the node a VM runs on. A fixed
  address sends every VM's logs, snapshots, exec and consoles to that one node, so VMs scheduled
  on any other node answer "VM not found" (Veyron logs a warning at the first such call). Add
  `VEYRON_KAIRON_NODE_TLS_INSECURE=1` if the relay uses a self-signed certificate.
- **An image cache on every Kairon node** (`KAIRON_IMAGE_CACHE_DIR`, writable by kairon-node). VMs
  booted from templates or imports download their disk into it.
- **Kairon `v1beta1` CRDs** (current Kairon `main`). An older Kairon that serves only `v1alpha1`
  makes every VM call fail with "Resource not found"; upgrade Kairon first.

- **An image store volume** for console/API uploads (`PUT /api/v1/image-store/:name`).
  `deploy/k8s.yaml` and `deploy-k8s-remote.sh` create a 50 GiB `veyron-image-store` PVC on the
  default StorageClass; in Helm, `imageStore.persistence`, `size`, `storageClass` and
  `existingClaim`. Nodes download uploads from the API's NodePort (self-signed TLS, so
  `insecureSkipTLSVerify` is set and the SHA-256 guards the bytes). Point them elsewhere with
  `VEYRON_IMAGE_STORE_PUBLIC_URL` (add `VEYRON_IMAGE_STORE_INSECURE_TLS=1` for a self-signed
  certificate), and move the store with `VEYRON_IMAGE_STORE_DIR`.
- **Writable PV directories for `root_volume`.** kairon-node seeds a PVC root disk by writing
  `disk.img` into the PV's hostPath/local directory, and FluxVM must be allowed to open it. With the
  stock `ProtectSystem=strict` unit, add the PV root to kairon-node's `ReadWritePaths` and to
  FluxVM's `allowed_image_dirs`, or seeding fails with a read-only filesystem error.
- **Kairon with `MachineImage` and install-media support** (current Kairon and FluxVM `main`) for the
  image catalog, ISO installs and PVC root disks.

`VEYRON_VM_BACKEND` defaults to Kairon. Set `kubevirt` only to keep managing an existing KubeVirt
cluster; Veyron never switches to it on its own.

## 2. Helm

> **Image caveat.** `ghcr.io/zyvorai/veyron:0.4.0` is **not published** (the GHCR package refuses writes from the
> release workflow). The chart's `image.tag` defaults to `latest`, which is the older 2026-10-04 image from before
> the Kairon rewrite, and the chart's `appVersion` (0.4.0) does not change that. Do not pin `image.tag=0.4.0`:
> the pull fails. Use the remote deploy script (it builds the image from source on a node), or build and push
> the image yourself and set `image.repository` and `image.tag` (`--set image.repository=... --set image.tag=...`).

```bash
helm upgrade --install veyron ./charts/veyron -n veyron-system --create-namespace \
  --set auth.apiKey="$(openssl rand -hex 24)" \
  --set auth.adminPassword="$(openssl rand -base64 18)"
```

Key values in [`charts/veyron/values.yaml`](../charts/veyron/values.yaml): `image.tag`,
`service.type` / `nodePort`, `tls.existingSecret`, `auth.*` (API key, admin user, extra keys),
`oidc.*` or `oidc.existingSecret`, `rateLimit.perMinute`, `networkPolicy.enabled`, `rbac.create`.

Without overrides the chart installs the lab defaults: console login `admin` / `Admin@321` and API
key `Admin@321` (see [getting-started.md](getting-started.md#default-credentials)). **Always set
your own `auth.apiKey` and `auth.adminPassword` in production.** The chart defaults are public.

## 3. Plain manifests

```bash
kubectl apply -f deploy/k8s.yaml
```

## RBAC

The API runs as ServiceAccount `veyron` with ClusterRole `veyron`. It is deliberately broad, so one
admin key can drive VMs, storage, snapshots and network policy. Three files define it and must stay
in sync: `deploy/k8s.yaml`, `charts/veyron/templates/clusterrole.yaml` and
`scripts/deploy-k8s-remote.sh`. For locked-down clusters, trim the rules and bind a read-only
ClusterRole for dashboards.

## Production checklist

- [ ] Replace the default API key and admin password (`Admin@321`); use `VEYRON_API_KEYS` for role-scoped keys.
- [ ] Put a real certificate in `tls.existingSecret`.
- [ ] Turn on SSO ([sso.md](sso.md)).
- [ ] Keep RDP behind a VPN or gateway. Public RDP NodePorts are refused unless
      `VEYRON_ALLOW_PUBLIC_RDP=1`.
- [ ] Wire Prometheus and Alertmanager ([integrations.md](integrations.md)).
- [ ] Run `./scripts/customer-readiness.sh <host>`, the full go-live gate.
- [ ] Hold a commercial license for production use ([LICENSE](../LICENSE)).

## Client tarball

A static linux/amd64 build with install scripts, built on a remote Linux host:

```bash
./scripts/package-binary-remote.sh <host> <user> --fetch   # writes dist/veyron-<ver>-linux-amd64.tar.gz
```

The tarball contains the `veyron` binary, `install.sh`, `uninstall.sh`, cluster helper scripts,
`START_HERE.txt`, `HELP.txt`, `README.md`, `docs/`, `LICENSE` and `NOTICE`.

### Evaluation builds

`--trial` builds with the `trial` feature and bundles a signed `trial.token` (Ed25519). The binary
checks the token at startup: when it is missing, invalid or expired the binary stops, and in the
last 7 days it prints a countdown. Deleting local state cannot reset a trial. The token is read from
`VEYRON_TRIAL_TOKEN`, then `trial.token` next to the binary, then
`~/.config/veyron/trial.token`. In-cluster images are never built with `trial`.

```bash
./scripts/package-binary-remote.sh <host> <user> --trial --fetch
```

Tokens are minted with `cargo run --features trial --bin trial-tool -- issue --who <licensee> --days 30`.
The signing key never leaves Zyvor. To extend an evaluation, contact [sales@zyvor.dev](mailto:sales@zyvor.dev).
