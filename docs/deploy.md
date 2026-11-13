<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Deploy

Ways to run Veyron: the release binaries, a remote deploy script, Helm, or plain manifests, plus a standalone
client tarball. The container image `ghcr.io/zyvorai/veyron:0.4.1` (and `:latest`) is published and cosign-signed, but
the GHCR package is **private**, so pulls need `docker login ghcr.io` ([details](#container-image)).

## Release binaries

Each GitHub release (v0.4.1 is the latest) carries `veyron-linux-amd64`, `veyron-linux-arm64`,
`veyron-darwin-amd64`, `veyron-darwin-arm64`, `SHA256SUMS`, `SHA256SUMS.cosign.bundle` and an SPDX SBOM
(`veyron.spdx.json`). The console is embedded. Linux builds link glibc dynamically.

How the v0.4.1 release came about, and what it does and does not give you:

- **Built by the `Release` workflow.** The four binaries come from the tagged run (test and build jobs pass). In the
  first attempt the Docker job failed at the GHCR push, which at the time also skipped the release job, so the release page
  was first created by hand from the run's binaries plus a `SHA256SUMS` file. After the package access was fixed the
  workflow run was re-run: the image was pushed and the release assets above (including the cosign bundle and SBOM) were
  uploaded by the workflow's `release` job. Since PR #59 the `release` job needs only `build`, so a failing image push no
  longer blocks the GitHub release. (The v0.4.1 release notes text was written before the re-run and still says the cosign
  signature and SBOM are missing; the asset list is authoritative.)
- **`SHA256SUMS` is signed keyless with cosign** (GitHub Actions OIDC, via Sigstore). The checksum alone only catches a
  corrupted download; verifying the bundle also shows the file was produced by this repository's release workflow for
  that tag. The SBOM is SPDX JSON.
- **macOS binaries are not Developer ID signed or notarized.** The workflow has no signing step. `veyron-darwin-arm64`
  carries only the linker's ad-hoc signature and `veyron-darwin-amd64` is not signed at all (checked with `codesign -dv`
  on the first-uploaded v0.4.1 assets; not re-checked on the re-uploaded ones).
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

Optional signature check (needs [`cosign`](https://docs.sigstore.dev/cosign/); this was run against the v0.4.1 files and
printed `Verified OK`). Replace the tag in the identity for other releases:

```bash
curl -fsSLO https://github.com/zyvorai/veyron/releases/download/$V/SHA256SUMS.cosign.bundle
cosign verify-blob --bundle SHA256SUMS.cosign.bundle \
  --certificate-identity "https://github.com/zyvorai/veyron/.github/workflows/release.yml@refs/tags/$V" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com SHA256SUMS
```

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

> **Image caveat.** `ghcr.io/zyvorai/veyron:0.4.1` and `:latest` are published (`0.4.0` was not), but the `veyron` GHCR
> package is **private**: the cluster needs credentials to pull it (see [Container image](#container-image)). The chart's
> `image.tag` defaults to `latest`, which now means 0.4.1 and moves with each release; pin `--set image.tag=0.4.1` for
> reproducible installs. Without pull credentials, or to use your own registry, set `image.repository` / `image.tag`
> (and `imagePullSecrets`), or use the remote deploy script, which builds on a node.

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

`ghcr.io/zyvorai/veyron:0.4.1` and `:latest` are published (`0.4.0` was never published as an image). The `Release`
workflow (`.github/workflows/release.yml`) builds the image in its `docker` job with an SBOM and provenance attestation,
pushes `:<version>` and `:latest`, and signs the digest with keyless cosign. The v0.4.1 image was pushed by re-running
that job after the first attempt failed at the GHCR push (the `veyron` package did not yet grant this repository write
access). The `0.3.x` tags on GHCR are images from 2026-10-04, before the Kairon rewrite.

**The package is private.** Pulling needs authentication with a token that can read packages:

```bash
echo "$GHCR_TOKEN" | docker login ghcr.io -u <github-user> --password-stdin
docker pull ghcr.io/zyvorai/veyron:0.4.1
# in Kubernetes: a docker-registry Secret, referenced through the chart's imagePullSecrets
kubectl -n veyron-system create secret docker-registry ghcr-pull --docker-server=ghcr.io \
  --docker-username=<github-user> --docker-password="$GHCR_TOKEN"
helm upgrade --install veyron ./charts/veyron -n veyron-system --set image.tag=0.4.1 \
  --set 'imagePullSecrets[0].name=ghcr-pull' ...   # plus the auth values from the Helm section
```

(These are the standard Docker and Kubernetes forms; they were not run against this package for this page, and the image
signature was not verified here. The workflow signs `ghcr.io/zyvorai/veyron@<digest>` keyless, so `cosign verify` needs
the digest, the release workflow identity and the GitHub OIDC issuer.)

**For maintainers.** The workflow logs in with the built-in `GITHUB_TOKEN` (`packages: write` is granted in the
workflow), which works only while the package grants `zyvorai/veyron` the **Write** role (Packages, `veyron`, *Package
settings*, **Manage Actions access**). If the push fails with `permission_denied: write_package`, fix that and re-run the
failed `docker` job. Since PR #59 the GitHub Release job does not depend on the image job, so a failed push no longer
blocks the release. Making the image public is a separate choice in the same settings page; it has not been done.

**Without GHCR access**, build your own. The first route builds on a node and keeps the image there; the second mirrors
the workflow and pushes to a registry you choose:

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
- [ ] Pin `image.tag` (for example `0.4.1`) instead of the moving default `latest`, and give the cluster pull credentials for the private GHCR package, or use your own registry ([Container image](#container-image)).
- [ ] Keep RDP behind a VPN or gateway. Public RDP NodePorts are refused unless
      `VEYRON_ALLOW_PUBLIC_RDP=1`.
- [ ] Wire Prometheus and Alertmanager ([integrations.md](integrations.md)).
- [ ] Run `./scripts/customer-readiness.sh <host>`, the full go-live gate.
- [ ] Hold a commercial license for production use ([LICENSE](../LICENSE)).

### Preflight: `veyron preflight`

`veyron preflight` checks the configuration the API will read, without a cluster and without sending anything over the
network. Run it where `api-serve` runs (or with the same environment, for example `kubectl exec` into the pod or
`docker run --env-file`).

```bash
veyron preflight                       # human-readable; exit 1 on any failure
veyron preflight --strict              # warnings also exit 1 (CI / pre-deploy gate)
veyron preflight -o json               # machine-readable findings
veyron preflight --tls-cert c.pem --tls-key k.pem
```

It reports: a missing or default/short `VEYRON_API_KEY`; `VEYRON_API_KEYS` entries the server would silently
mishandle (no key, unknown role that falls back to `readonly`, duplicate names or values, a key containing `:` that is
cut short); a half-set JWT pair or a short JWT secret; the published default bootstrap admin password; partial OIDC
settings or a missing audience; TLS files that are absent or not PEM (expiry is not checked); an invalid
`VEYRON_NAMESPACE`; integration URLs that are not http(s), or plain `http://` to a non-cluster host while a token is
set; `*` in `VEYRON_CORS_ORIGINS` (it is not a wildcard); an unsigned plan webhook; risk-lowering flags that are on; and
a missing kubeconfig outside a cluster. Secret values are never printed. It is the configuration counterpart of
`veyron doctor`, which checks the cluster connection and KubeVirt. Findings are `ok`, `warn` or `fail`.

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
