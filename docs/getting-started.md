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

- **Local account:** `admin`, with the password from `VEYRON_BOOTSTRAP_ADMIN_PASSWORD` (it defaults
  to the API key).
- **API key:** the value of the `veyron-api-key` Secret (`VEYRON_API_KEY`).

Change the bootstrap credentials before anyone else can reach the console. Production setups add
role-scoped keys and SSO ([api.md](api.md), [sso.md](sso.md)).

## 4. Create a VM

In the console: **VMs → New VM**, pick a template (Ubuntu, Debian, Fedora, Windows and more), size
it and start it. Open the console tab for an in-browser VNC session.

The same through the API:

```bash
export VEYRON_API_KEY=<key>
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
