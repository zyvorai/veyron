# Package VMRogue as a Linux binary (remote build)

Use this flow when you want to **ship a client deliverable** (tarball with `vmrogue` + optional `virtctl`) without giving them Kubernetes deploy scripts or container manifests.

The build runs on a **Linux amd64 machine** you control (build server, k3s node, CI runner). You download the archive and hand it to the client.

## What you get

After `./scripts/package-binary-remote.sh` completes, the remote host has:

```
~/vmrogue-dist/
  vmrogue-0.2.0-linux-amd64/
    vmrogue              # static Linux amd64 binary (~26 MB)
    virtctl              # KubeVirt CLI helper (from image build)
    vmrogue.env.example
    README.txt
  vmrogue-0.2.0-linux-amd64.tar.gz
  vmrogue-0.2.0-linux-amd64.tar.gz.sha256
```

With `--fetch`, the `.tar.gz` and checksum are also copied to **`dist/`** in your local repo.

## Prerequisites

### On the remote build host

| Requirement | Notes |
|-------------|--------|
| **Linux x86_64** | Same arch as the client binary |
| **podman** or **docker** | Used to run the multi-stage `Dockerfile` (musl static binary) |
| **SSH access** | Your laptop can `ssh user@host` with key auth |
| **Disk / RAM** | ~2 GB free disk; 4+ GB RAM recommended for Rust compile inside the image build |

Kubernetes on the build host is **not** required for packaging (only for running the binary later).

### On your laptop

- `rsync`, `ssh`, `scp`
- VMRogue source checkout

## Build and download

From the VMRogue repo root:

```bash
# Full build on remote + copy tarball to ./dist/
./scripts/package-binary-remote.sh HOST sus --fetch
```

First run typically takes **10–15 minutes** (Rust compile inside the container build). Later runs:

```bash
# Re-package from an existing image (seconds)
./scripts/package-binary-remote.sh HOST sus --reuse-image --fetch
```

### Options

| Flag | Effect |
|------|--------|
| `--fetch` | `scp` the tarball and `.sha256` into `./dist/` |
| `--reuse-image` | Skip `podman build` if `vmrogue-package:<version>` already exists |
| `--no-virtctl` | Smaller tarball without `virtctl` |

### Environment

| Variable | Default | Purpose |
|----------|---------|---------|
| `DEPLOY_HOST` / `DEPLOY_USER` | — / `sus` | When host/user omitted |
| `VMROGUE_PACKAGE_DIR` | `~/vmrogue-dist` | Remote output directory |
| `VMROGUE_PACKAGE_VERSION` | from `Cargo.toml` | Archive name version |
| `VMROGUE_REMOTE_SKIP_SSH_CHECK=1` | off | Skip SSH preflight |

### Manual download

```bash
scp sus@HOST:~/vmrogue-dist/vmrogue-0.2.0-linux-amd64.tar.gz .
scp sus@HOST:~/vmrogue-dist/vmrogue-0.2.0-linux-amd64.tar.gz.sha256 .
sha256sum -c vmrogue-0.2.0-linux-amd64.tar.gz.sha256
```

## Give the package to a client

1. Send **`vmrogue-<version>-linux-amd64.tar.gz`** and the **`.sha256`** file (or verify before sending).
2. Include a **kubeconfig** (or instructions to use their own) with rights to manage KubeVirt VMs.
3. Point them to the **Client install** section below.

Do **not** rely on deploy scripts for client installs unless you are also operating their cluster.

## Client install and run

On a **Linux x86_64** machine that can reach the Kubernetes API:

```bash
tar xzf vmrogue-0.2.0-linux-amd64.tar.gz
cd vmrogue-0.2.0-linux-amd64
cp vmrogue.env.example vmrogue.env
# Edit vmrogue.env: KUBECONFIG, VMROGUE_API_KEY

set -a && source vmrogue.env && set +a
./vmrogue api-serve --host 0.0.0.0 --port 5151
```

Open **`http://<server-ip>:5151/dashboard`**, enter the API key.

### CLI-only (no long-running server)

```bash
export KUBECONFIG=/path/to/kubeconfig
./vmrogue list --namespace all
./vmrogue doctor
```

### HTTPS

```bash
./vmrogue api-serve --host 0.0.0.0 --port 5151 \
  --tls --tls-cert /etc/vmrogue/tls.crt --tls-key /etc/vmrogue/tls.key
```

### systemd (optional)

```ini
[Unit]
Description=VMRogue API
After=network.target

[Service]
Type=simple
EnvironmentFile=/etc/vmrogue/env
WorkingDirectory=/opt/vmrogue
ExecStart=/opt/vmrogue/vmrogue api-serve --host 0.0.0.0 --port 5151
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

## Client prerequisites (cluster side)

The binary is **not** a hypervisor. The client still needs:

- Kubernetes with **KubeVirt** (and CDI if they use DataVolumes / imports)
- A **kubeconfig** whose user/service account can manage VMs (see `deploy/k8s.yaml` `ClusterRole` for the full API feature set)
- Network: client host → Kubernetes API; users → VMRogue API port (e.g. `5151`)

## Compare: binary package vs `deploy-remote.sh`

| Approach | Best for |
|----------|----------|
| **`package-binary-remote.sh`** | Hand off a tarball; client runs API on bastion/VM; no in-cluster install |
| **`deploy-remote.sh`** | You operate the cluster; API in `vmrogue-system` on NodePort (e.g. `30151`) |

You can use **both**: deploy in-cluster for production, and package a binary for admins on a jump host.

## Troubleshooting

| Problem | What to check |
|---------|----------------|
| SSH fails | Keys, firewall, `DEPLOY_SSH_TIMEOUT` |
| `podman: command not found` | Install podman or docker on build host |
| Build OOM / killed | More RAM or swap on build host |
| Client `connection refused` to API | Firewall on port `5151`, `--host 0.0.0.0` |
| Dashboard loads but VMs empty | `KUBECONFIG`, RBAC, KubeVirt installed |
| `guest-exec` / RDP in guest fails | Cluster must expose guest-exec subresource; API identity needs RBAC (see `CLAUDE.md`) |

## Verify after packaging

On your laptop (against a running API, in-cluster or client-hosted):

```bash
export VMROGUE_API_KEY=CHANGE_ME
./scripts/verify-vmrogue-remote.sh <api-host> <port>
```

For in-cluster NodePort on k3s, port is often **30151** (HTTPS).

## Related docs

- [README.md](../README.md) — project overview
- [WINDOWS_KUBEVIRT_PRODUCTION.md](WINDOWS_KUBEVIRT_PRODUCTION.md) — Windows RDP / guest-agent
- [DEVELOPER_VM_ACCESS.md](DEVELOPER_VM_ACCESS.md) — SSH / virtctl access
