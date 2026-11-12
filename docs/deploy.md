<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Deploy

Ways to run Veyron: the release binaries, a remote deploy script, Helm, or plain manifests, plus a standalone
client tarball. **There is no published container image for 0.4.x** ([details](#container-image)): in-cluster installs
today build the image from source on a node.

## Release binaries

Each GitHub release (v0.4.1 is the latest) carries `veyron-linux-amd64`, `veyron-linux-arm64`,
`veyron-darwin-amd64`, `veyron-darwin-arm64` and `SHA256SUMS`. The console is embedded. Linux builds link
glibc dynamically.

What the v0.4.0 and v0.4.1 releases are, exactly:

- **Built by CI, published by hand.** The four binaries come from the tagged `Release` workflow run (its test and
  build jobs pass). The same workflow's Docker job fails at the GHCR push, and the GitHub Release job needs the Docker
  job, so the workflow never creates the release itself. The release page was created manually from the workflow's
  binaries plus a `SHA256SUMS` file.
- **No signatures.** There is no cosign bundle for `SHA256SUMS` and no SBOM (the workflow would produce both, but only
  after the Docker job succeeds), so the checksum file is the only integrity check, and it comes from the same release
  page as the binaries. It detects a corrupted download, not a tampered release.
- **macOS binaries are not Developer ID signed or notarized.** Checked on the v0.4.1 assets with `codesign -dv`:
  `veyron-darwin-arm64` carries only the linker's ad-hoc signature and `veyron-darwin-amd64` is not signed at all.
  A binary saved from a browser is quarantined by Gatekeeper; clear it with `xattr -d com.apple.quarantine veyron-darwin-*`
  (`curl` downloads are not quarantined).

```bash
V=v0.4.1; A=linux-amd64        # linux-arm64, darwin-amd64 or darwin-arm64
curl -fsSLO https://github.com/zyvorai/veyron/releases/download/$V/veyron-$A
curl -fsSLO https://github.com/zyvorai/veyron/releases/download/$V/SHA256SUMS
grep " veyron-$A\$" SHA256SUMS | (sha256sum -c - 2>/dev/null || shasum -a 256 -c -)   # expect "veyron-$A: OK"
chmod +x veyron-$A && sudo install -m 0755 veyron-$A /usr/local/bin/veyron
veyron --version               # veyron 0.4.1
```

The `grep` keeps the check to the one file you downloaded. `shasum -a 256 -c SHA256SUMS` on its own (the form the release
notes use) also works, but reports `FAILED open or read` for every binary that is not in the current directory, so use it
only after downloading all four. `sha256sum` (GNU) and `shasum -a 256` (macOS, and Perl `shasum` on Linux) both read
the same file format.

Run the API and console with `veyron api-serve`:

| Flag | Meaning |
|---|---|
| `-p, --port` | Listen port (config default 8080; the in-cluster Deployment uses 5151) |
| `--host` | Bind address (config default `127.0.0.1`; the Deployment uses `0.0.0.0`) |
| `--tls`, `--tls-cert`, `--tls-key` | Serve HTTPS from a PEM certificate and key. Both paths are required with `--tls`. Without `--tls` the server speaks plain HTTP, so API keys cross the network in clear text |
| `--rate-limit` | Requests per minute for the whole server, one shared bucket (default 60; `0` disables the limit). Exempt: health, OIDC config/token and `GET` polling of the console's read routes; the console shell and assets are not exempt ([limits](api.md#limits-worth-knowing)) |
| `--auth` | Accepted for compatibility (`none`, `api-key`, `bearer`, `basic`, `oauth2` or `mtls`; anything else is an error) and only echoed in the startup banner. The HTTP server always requires an API key, JWT or OIDC bearer; this flag does not change that |

Two more settings belong to the `veyron` command rather than `api-serve`, so they go **before** the subcommand:
`veyron --namespace veyron-system api-serve ...` (or `VEYRON_NAMESPACE`, default `default`) picks the namespace for
Veyron's own state (the `veyron-users` Secret, SOC and settings ConfigMaps), and `--kubeconfig` (or `KUBECONFIG`)
the cluster. `--verbose` and `--no-color` are accepted anywhere.

Defaults for `--port`, `--host`, `--tls`, `--tls-cert`, `--tls-key`, `--auth` and `--rate-limit` can also come from the
`[api]` section of `/etc/veyron/config.toml` and `~/.config/veyron/config.toml` (create one with `veyron config-init`,
inspect with `veyron config-show`); flags override the user file, which overrides the system file. `veyron api-routes`
and `veyron api-spec` print the route table and the OpenAPI document without a server.

With `--tls` the server also opens a plain-HTTP redirect listener (port 80 when the HTTPS port is 443, otherwise `port - 1`, for example 5150)
that sends browsers to HTTPS (the target host is `VEYRON_PUBLIC_HOST` when set; otherwise the request's `Host` header, accepted only if it is a well-formed hostname or IP, else `400`. Set `VEYRON_PUBLIC_HOST` in any deployment reachable by untrusted clients).

Settings come from the environment (the same variables the manifests set):

| Variable | Notes |
|---|---|
| `VEYRON_API_KEY` | **Required.** The primary key, role `admin`. When it is unset every request answers `503 AUTH_NOT_CONFIGURED`, even if `VEYRON_API_KEYS`, JWT or OIDC are set |
| `VEYRON_API_KEYS` | More keys as `name:key:role,...` (`admin`, `write`, `readonly`; `read` also works; a missing or unknown role means `readonly`). Names and keys must not contain `:` or `,` |
| `VEYRON_JWT_SECRET`, `VEYRON_JWT_ISSUER`, `VEYRON_JWT_ROLE_CLAIM` | HS256 secret and issuer for session tokens (console sign-in) and for your own JWTs; role claim defaults to `role`. Without the first two, `POST /auth/login` answers `503` and the bootstrap admin is skipped. Login tokens last 24 h and are revoked when the user is deleted or their password is reset |
| `VEYRON_BOOTSTRAP_ADMIN`, `_USER`, `_PASSWORD`, `_SYNC` | First console account, see [getting-started.md](getting-started.md#default-credentials) |
| `VEYRON_OIDC_*` | Single sign-on, see [sso.md](sso.md). `VEYRON_OIDC_AUDIENCE` (optional) makes the token's `aud` mandatory |
| `VEYRON_PUBLIC_HOST` | Public hostname used by the HTTP-to-HTTPS redirect listener (with `--tls`); the `Host` header is then ignored |
| `VEYRON_IMAGE_STORE_MAX_BYTES` | Size cap for one image-store upload or capture (default 64 GiB; `0` disables the cap). Larger uploads answer `413 UPLOAD_TOO_LARGE` |
| `VEYRON_KUBEVIRT_WS_INSECURE_TLS` | `1` skips verification of the Kubernetes API certificate on the legacy KubeVirt VNC/serial proxy (verified against the cluster CA by default) |
| `VEYRON_CORS_ORIGINS` | Comma-separated extra origins. Unset means same-origin only |
| `VEYRON_VM_BACKEND` | Unset, `auto` or `kairon`: Kairon. `kubevirt`: legacy |
| `VEYRON_WS_METRICS_MAX` | Concurrent `/ws/metrics` streams (default 64); further upgrades answer `503` |
| `VEYRON_HTTP_REQUEST_TIMEOUT_SECS` | Per-request timeout for ordinary routes (default 30) |
| `VEYRON_ALLOW_PUBLIC_RDP` | `1` lets `PUT /vms/:ns/:name/rdp-expose` publish RDP on a NodePort or LoadBalancer (refused with `403` otherwise) |
| `VEYRON_NAMESPACE`, `VEYRON_CONFIG`, `KUBECONFIG` | Same as `--namespace`, `--config` and `--kubeconfig`. Outside a pod the kubeconfig is used; in a pod the ServiceAccount is |

Needs a reachable Kubernetes API and, for VM routes, Kairon's CRDs (see [Kairon settings](#kairon-settings)).
Only the in-cluster Deployment (sections below) comes with the image-store volume and the relay Secret wired up.

The `veyron` CLI's VM commands (`create`, `list`, `start`, ...) manage KubeVirt `VirtualMachine`s only in
v0.4.1; they do not manage Kairon Machines.

## 1. Remote deploy script (fastest)

Syncs the source to a cluster node over SSH, builds the images there (layer cache on by default), imports them
into the node runtime (k3s, containerd, microk8s, minikube), and applies [`deploy/k8s.yaml`](../deploy/k8s.yaml) in
`veyron-system`. It wraps `scripts/deploy-all-remote.sh`, which also builds and deploys the legacy Go operator
(`veyron-operator`), its `veyron.io` CRDs, the template/profile catalog and a NATS Deployment. That is harmless on
Kairon, which does not use them. The API image is tagged `docker.io/library/veyron:<Cargo.toml version>` with
`imagePullPolicy: Never`, so it only exists on that node; on a multi-node cluster pin the Deployment to it or push the
image to a registry yourself.

```bash
./scripts/deploy-remote.sh <host> <user>             # full build and deploy
./scripts/deploy-remote.sh <host> <user> --quick     # re-apply manifests only
./scripts/deploy-remote.sh <host> <user> --with-oidc # also write the veyron-oidc Secret
```

Credentials: the script reuses `VEYRON_API_KEY` / `VEYRON_BOOTSTRAP_ADMIN_PASSWORD` / `VEYRON_JWT_SECRET` from your
environment, else the values in the existing `veyron-api-key` Secret, else the lab defaults (JWT secret: always random).
It then deletes and recreates that Secret and **saves the result in plain text to
`~/.config/veyron/hosts/<host>.env` on the machine that ran the script** (mode 600; the test scripts read it).
Rotate by exporting new values and redeploying.

Useful environment variables:

| Variable | Effect |
|---|---|
| `VEYRON_DEPLOY_NO_CACHE=1` | Clean image rebuild |
| `VEYRON_SKIP_GUESTKIT_PREP=1` | Skip re-syncing the GuestKit build context |
| `VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP=1` | Don't apply the Cilium egress policies |
| `VEYRON_INSTALL_CDI=1` | Install CDI too (legacy KubeVirt only; Kairon needs none; without it the CDI step is skipped, and `VEYRON_SKIP_CDI=1` skips it even then) |
| `DEPLOY_SSH_TIMEOUT`, `DEPLOY_SSH_PORT` | SSH settings |

The API serves HTTPS on `hostPort` 5151 and NodePort 30151 with a self-signed certificate made by an
init container (new on every pod start), plus the HTTP-to-HTTPS redirect on `hostPort` 5150 and NodePort 30150. Then
check it:

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

> **Image caveat.** `ghcr.io/zyvorai/veyron:0.4.0` and `:0.4.1` are **not published** (see [Container image](#container-image)).
> The chart's `image.tag` defaults to `latest`, which is the older 2026-10-04 image from before
> the Kairon rewrite, and the chart's `appVersion` (0.4.1) does not change that. Do not pin `image.tag=0.4.0` or `0.4.1`:
> the pull fails. Build and push the image yourself and set `image.repository` and `image.tag`
> (`--set image.repository=... --set image.tag=...`), or use the remote deploy script, which builds on a node.

```bash
helm upgrade --install veyron ./charts/veyron -n veyron-system --create-namespace \
  --set image.repository=registry.example.com/veyron --set image.tag=0.4.1 \
  --set auth.apiKey="$(openssl rand -hex 24)" \
  --set auth.adminPassword="$(openssl rand -base64 18)"
```

Key values in [`charts/veyron/values.yaml`](../charts/veyron/values.yaml) (full table in the
[chart README](../charts/veyron/README.md)): `image.*`, `service.type` / `nodePort`, `tls.existingSecret`,
`auth.*` (API key, admin user and password, extra keys, JWT), `oidc.*` or `oidc.existingSecret`,
`rateLimit.perMinute`, `imageStore.*`, `networkPolicy.enabled`, `rbac.create`, `ai.*`.

Without overrides the chart installs the lab defaults: console login `admin` / `Admin@321` and API
key `Admin@321` (see [getting-started.md](getting-started.md#default-credentials)). **Always set
your own `auth.apiKey` and `auth.adminPassword` in production.** The chart defaults are public. An existing
`veyron-api-key` Secret keeps its values across upgrades unless you set new ones.

Things the chart does not do for you:

- **The kairon-node relay Secret is not wired.** `deploy/k8s.yaml` loads the optional `veyron-kairon` Secret; the chart
  does not. Put `VEYRON_KAIRON_NODE_URL`, `KAIRON_NODE_CONSOLE_TOKEN` (and, for a self-signed relay,
  `VEYRON_KAIRON_NODE_TLS_INSECURE=1`) in a Secret and name it in `integrations.existingSecret` (or `oidc.existingSecret`):
  both are loaded as `envFrom`, so any other variable from this page can travel the same way
  (`VEYRON_PUBLIC_HOST`, `VEYRON_OIDC_AUDIENCE`, `VEYRON_IMAGE_STORE_MAX_BYTES`, `VEYRON_WS_METRICS_MAX`, ...).
- **The NetworkPolicy is tight.** With `networkPolicy.enabled` (the default) the pod may receive only on the API port
  and send only to DNS (53) and the Kubernetes API (443, 6443). Calls to the kairon-node relay (8090), Prometheus,
  an LLM endpoint, Netra and the like are blocked. Add your own policy for them or set `networkPolicy.enabled=false`.
- **Veyron's own state goes to `api.namespace`**, which defaults to `default` (it sets `VEYRON_NAMESPACE`), not to the
  release namespace. The `veyron-users` Secret and the SOC and settings ConfigMaps live there.
- **Keep `replicaCount` at 1** unless clients use session affinity: console tickets, rate limits and the login limiter
  are held in each pod's memory.
- **`ai.maxToolRounds` (2) and `ai.timeoutSecs` (30) are always exported** and override the code defaults (8 and 6
  rounds; a 300 s timeout when an in-cluster model is selected). Set them to `0` to leave the variables unset.
  (Read from the templates; not tested on a cluster.)
- The image-store PVC is named `<release fullname>-image-store` (`veyron-api-image-store` with the default
  `fullnameOverride`), is kept on `helm uninstall`, and `imageStore.persistence=false` falls back to an `emptyDir`
  that loses uploads on restart.

## 3. Plain manifests

```bash
kubectl create namespace veyron-system
KEY=$(openssl rand -hex 16)
kubectl -n veyron-system create secret generic veyron-api-key \
  --from-literal=api-key="$KEY" --from-literal=admin-password="$KEY" \
  --from-literal=jwt-secret="$(openssl rand -hex 32)" --from-literal=jwt-issuer=veyron
sed -e 's#image: docker.io/library/veyron:.*#image: registry.example.com/veyron:0.4.1#' \
    -e 's#imagePullPolicy: Never#imagePullPolicy: IfNotPresent#' deploy/k8s.yaml | kubectl apply -f -
```

`deploy/k8s.yaml` does not work as shipped: it references the `veyron-api-key` Secret (create it first) and the image
`docker.io/library/veyron:0.2.2` with `imagePullPolicy: Never`, a placeholder the deploy scripts replace. The `sed` above substitutes
your own image and lets the kubelet pull it (drop the second expression if the image is already on the node). The manifest creates the
namespace, ServiceAccount, ClusterRole and binding, a 50 GiB `veyron-image-store` PVC and a NodePort Service (30151
for HTTPS, 30150 for the redirect), and loads these Secrets if present: `veyron-integrations`, `veyron-atlas-auth`,
`veyron-kryton-auth`, `veyron-oidc`, `veyron-kairon` and `veyron-rbac-keys`. `deploy/kustomize/` wraps the same file.

## Container image

`ghcr.io/zyvorai/veyron:0.4.0` and `:0.4.1` do not exist. The `Release` workflow (`.github/workflows/release.yml`) builds
the image in its `docker` job, but pushing to GHCR fails because the `veyron` package grants this repository no write
access, and the `release` job lists `docker` in `needs`, so it is skipped. (The workflow log shows the failure in the
"Build and push" step; the exact denial message was not captured here.) The tags `0.3.x` and `latest` on GHCR are
images from 2026-10-04, before the Kairon rewrite.

**Fix for maintainers** (needs admin rights on the package):

1. Open the `veyron` package under the `zyvorai` organization on GitHub (Packages, `veyron`, *Package settings*).
2. Under **Manage Actions access**, add the repository `zyvorai/veyron` with the **Write** role.
3. Re-run the failed `docker` job of the tag's workflow run, or push a new tag. The workflow logs in with the built-in
   `GITHUB_TOKEN` (`packages: write` is already granted in the workflow), so no personal access token is needed.
   Re-running the whole workflow would also run the `release` job against a release page that already exists; check
   what it does to the hand-published assets first. This path has not been tried.

**Until then**, build your own. The first route builds on a node and keeps the image there; the second mirrors the
workflow and pushes to a registry you choose:

```bash
./scripts/deploy-remote.sh <host> <user>          # builds docker.io/library/veyron:<version> on the node

# or, anywhere with Docker (GUESTKIT_REF is pinned in .github/workflows/ci.yml)
git clone https://github.com/zyvorai/zyvor-guestkit guestkit && git -C guestkit checkout <GUESTKIT_REF>
docker build -t registry.example.com/veyron:0.4.1 . && docker push registry.example.com/veyron:0.4.1
```

(The second route was read from the Dockerfile and the workflow, not run for this page.)

## RBAC

The API runs as ServiceAccount `veyron` with ClusterRole `veyron`. It is deliberately broad, so one
admin key can drive VMs, storage, snapshots and network policy. Three files define it and must stay
in sync: `deploy/k8s.yaml`, `charts/veyron/templates/clusterrole.yaml` and
`scripts/deploy-k8s-remote.sh`. For locked-down clusters, trim the rules and bind a read-only
ClusterRole for dashboards.

## Production checklist

- [ ] Replace the default API key and admin password (`Admin@321`); use `VEYRON_API_KEYS` for role-scoped keys. Give
      automation the least role it needs: since 0.4.1 only `admin` approves a `VeyronAction` or edits a `VeyronPolicy`.
- [ ] Put a real certificate in `tls.existingSecret`, and set `VEYRON_PUBLIC_HOST` so the HTTP redirect never
      echoes a client-supplied `Host`.
- [ ] Turn on SSO ([sso.md](sso.md)) and set `VEYRON_OIDC_AUDIENCE`.
- [ ] Behind a reverse proxy, remember that the login limiter and rate limiter see the proxy's address.
- [ ] Use an image you built and pushed, not the Helm default `latest` ([Container image](#container-image)).
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
