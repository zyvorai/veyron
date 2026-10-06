<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Getting started

From an empty Kubernetes cluster to a running VM in the Veyron console.

## What you need

- A Kubernetes cluster (k3s works well) whose VM hosts are Linux with `/dev/kvm`.
- [Kairon](https://github.com/zyvorai/kairon) installed in `kairon-system`, with FluxVM on each VM host.
- `kubectl` and `helm` on your workstation, and SSH access to one cluster node for the remote
  deploy script.

Check a host before you start:

```bash
./scripts/cluster/preflight-node.sh <host> <user>     # /dev/kvm, virt-host-validate, swap
```

## 1. Install Kairon

```bash
kubectl label node <vm-host> kairon.zyvor.dev/capable=true
helm upgrade --install kairon oci://ghcr.io/zyvorai/charts/kairon \
  -n kairon-system --create-namespace
```

## 2. Deploy Veyron

The remote script syncs the source to a node, builds the images there and applies the manifests in
`veyron-system`:

```bash
./scripts/deploy-remote.sh <host> <user>
```

Or use the Helm chart in [`charts/veyron`](../charts/veyron) (see [deploy.md](deploy.md)).

## 3. Sign in

Open `https://<node-ip>:30151/console`. The certificate is self-signed on first deploy.

![Sign in](assets/console-login.png)

### Default credentials

| Field    | Value       |
|----------|-------------|
| Username | `admin`     |
| Password | `Admin@321` |
| API key  | `Admin@321` |

These are lab defaults, the same pair as [Netra](https://github.com/zyvorai/netra). The deploy
scripts and the Helm chart use them unless you override them:

- **Console password:** `VEYRON_BOOTSTRAP_ADMIN_PASSWORD` (Helm: `auth.adminPassword`), stored as
  `admin-password` in the `veyron-api-key` Secret.
- **API key:** `VEYRON_API_KEY` (Helm: `auth.apiKey`), stored as `api-key` in the same Secret.
- A redeploy keeps whatever the existing Secret holds, so a rotated key survives upgrades.
- The JWT signing secret is always random.

The login screen shows the host you are connecting to ("Connecting to `<host>`"), so you can check
you are on the right cluster before signing in.

Change the defaults before anyone else can reach the console. Production setups add
role-scoped keys and SSO ([api.md](api.md), [sso.md](sso.md)).

## 4. Create a VM

In the console: **VMs → New VM**, pick a template (Ubuntu, Debian, Fedora, Windows and more), size
it and start it. Open the console tab for an in-browser VNC session. Click the VM's row for its
details panel: guest health, disks, snapshots and migrations. To snapshot it on a schedule, use
**Storage & Network → Snapshot schedules → New** with a cron such as `0 2 * * *` (UTC).

The same through the API:

```bash
export VEYRON_API_KEY=Admin@321   # or your own key
curl -sk -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -X POST https://<node-ip>:30151/api/v1/vms \
  -d '{"name":"demo","namespace":"default","template":"ubuntu-24.04","cpus":2,"memory":"4Gi"}'
```

Or the CLI:

```bash
veyron create demo --template ubuntu-24.04
veyron list
```

## 5. Check the deployment

```bash
./scripts/verify-veyron-remote.sh <host>                          # API smoke test
VEYRON_API_KEY=<key> ./scripts/test-vm-daily-ops-remote.sh <host>  # VM lifecycle end to end
```

Next: [architecture.md](architecture.md) for how it works, [deploy.md](deploy.md) for production
settings.
