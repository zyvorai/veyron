<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Deploy

Three ways to run Veyron, plus a standalone client tarball.

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
  `VEYRON_KAIRON_NODE_URL` (for example `http://<node-ip>:8090`) and `KAIRON_NODE_CONSOLE_TOKEN` in
  the optional `veyron-kairon` Secret; `deploy/k8s.yaml` loads it when present. Add
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
