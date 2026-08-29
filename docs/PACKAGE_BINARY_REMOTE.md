# Package Veyron as a Linux binary (remote build)

Use this flow when you want to **ship a client deliverable** (tarball with `veyron` + optional `virtctl`) without giving them Kubernetes deploy scripts or container manifests.

**Policy:** customer tarballs are **binaries/artifacts only** — no git clone on the install host. See **`docs/CLIENT_BUNDLE_POLICY.md`** for all products (Rust/Go static vs Python `venv` bundles for h2kvm/forge).

The build runs on a **Linux amd64 machine** you control (build server, k3s node, CI runner). You download the archive and hand it to the client.

## What you get

After `./scripts/package-binary-remote.sh` completes, the **tar.gz** contains everything a customer needs:

```
veyron-0.2.0-linux-amd64/
  veyron                  # binary
  trial.token             # signed evaluation token (trial builds)
  virtctl                  # optional
  install.sh               # client on this machine
  install-cluster.sh       # Cilium + KubeVirt + CDI (cluster admin)
  apply-cluster-network.sh # Cilium egress bootstrap
  test-cluster.sh          # verify cluster prerequisites
  test-package.sh          # client smoke test
  CLUSTER_SETUP.txt        # flags and order of operations
  PREREQUISITES.txt        # checklist
  cluster/                 # prereq installer + bootstrap YAML
  veyron.env.example
  README.txt / QUICKSTART.txt
```

Checksum file: `veyron-0.2.0-linux-amd64.tar.gz.sha256`

With `--fetch`, the `.tar.gz` and checksum are also copied to **`dist/`** in your local repo.

### Customer install

**Cluster (once per Kubernetes cluster):**

```bash
export KUBECONFIG=/path/to/kubeconfig
./install-cluster.sh              # VEYRON_SKIP_CDI=1 etc. — see CLUSTER_SETUP.txt
# Deploy Veyron in-cluster (Helm/k8s from source repo)
./apply-cluster-network.sh        # if Cilium default-deny egress
./test-cluster.sh
```

**Client on this machine:**

```bash
tar xzf veyron-*-linux-amd64.tar.gz && cd veyron-*-linux-amd64
ls -l trial.token          # required for evaluation builds
./install.sh
nano veyron.env   # KUBECONFIG + API key
# optional: export VEYRON_TRIAL_TOKEN="$(cat trial.token)"
./test-package.sh
```

After the signed token expires: **sales@zyvor.dev** — see [LICENSING.md](LICENSING.md).

### Customer uninstall

```bash
./uninstall.sh --yes                  # stop + remove config
./uninstall.sh --yes --remove-dir     # also delete the extracted folder
./uninstall.sh --yes --keep-config    # stop only, keep veyron.env
```

## Prerequisites

### On the remote build host

The package script **installs build dependencies automatically** (like `deploy-remote.sh`), unless you pass **`--skip-deps`**.

| Requirement | Notes |
|-------------|--------|
| **Linux x86_64** | Same arch as the client binary |
| **podman** or **docker** | Installed via `dnf`/`apt` if missing; used for the musl static build |
| **SSH access** | Your laptop can `ssh user@host` with key auth |
| **Disk / RAM** | ~2 GB free disk; 4+ GB RAM recommended for Rust compile inside the image build |

Kubernetes on the build host is **not** required for packaging (only for running the binary later).

### On your laptop

- `rsync`, `ssh`, `scp`
- Veyron source checkout

## Build and download

From the Veyron repo root:

```bash
# Full build on remote + copy tarball to ./dist/
./scripts/package-binary-remote.sh <host> <user> --fetch
```

First run typically takes **10–15 minutes** (Rust compile inside the container build). Later runs:

```bash
# Re-package from an existing image (seconds)
./scripts/package-binary-remote.sh <host> <user> --reuse-image --fetch
```

### Options

| Flag | Effect |
|------|--------|
| `--fetch` | `scp` the tarball and `.sha256` into `./dist/` |
| `--reuse-image` | Skip `podman build` if `veyron-package:<version>` already exists |
| `--no-virtctl` | Smaller tarball without `virtctl` |
| `--skip-deps` | Do not auto-install podman/docker on the build host |

### Environment

| Variable | Default | Purpose |
|----------|---------|---------|
| `DEPLOY_HOST` / `DEPLOY_USER` | — / `operator` | When host/user omitted |
| `VEYRON_PACKAGE_DIR` | `~/veyron-dist` | Remote output directory |
| `VEYRON_PACKAGE_VERSION` | from `Cargo.toml` | Archive name version |
| `VEYRON_REMOTE_SKIP_SSH_CHECK=1` | off | Skip SSH preflight |

### Manual download

```bash
scp user@<host>:~/veyron-dist/veyron-0.2.0-linux-amd64.tar.gz .
scp user@<host>:~/veyron-dist/veyron-0.2.0-linux-amd64.tar.gz.sha256 .
sha256sum -c veyron-0.2.0-linux-amd64.tar.gz.sha256
```

## Give the package to a client

**Option A — direct hand-off:**

1. Send **`veyron-<version>-linux-amd64.tar.gz`** and the **`.sha256`** file (or verify before sending).
2. Include a **kubeconfig** (or instructions to use their own) with rights to manage KubeVirt VMs.
3. Point them to the **Client install** section below.

**Option B — public download link:** `./scripts/publish-customer-release.sh [--trial]` uploads the
tarball to the **public** [zyvorai/veyron-releases](https://github.com/zyvorai/veyron-releases)
repo (this source repo is private — a release published *here* is unreachable by customers).
Add `--trial` to publish the 30-day evaluation build instead — see `docs/LICENSING.md`.

Do **not** rely on deploy scripts for client installs unless you are also operating their cluster.

## What is inside the tarball

Every client bundle includes:

| File | Purpose |
|------|---------|
| **`install.sh`** | **Start here** — deps + config + verify in one command |
| **`uninstall.sh`** | Remove install from the customer machine (`--remove-dir` deletes folder) |
| **`QUICKSTART.txt`** | Short numbered steps for copy-paste |
| **`README.txt`** | Full install, deploy, test, troubleshooting |
| **`install-client-deps.sh`** | OS/runtime packages on the client host |
| **`test-package.sh`** | Smoke test after install |
| **`*.env.example`** | Environment template |

## Client install and run

On a **Linux x86_64** machine that can reach the Kubernetes API:

```bash
tar xzf veyron-0.2.0-linux-amd64.tar.gz
cd veyron-0.2.0-linux-amd64
sha256sum -c ../veyron-0.2.0-linux-amd64.tar.gz.sha256
./install-client-deps.sh          # optional
cp veyron.env.example veyron.env
# Edit veyron.env: KUBECONFIG, VEYRON_API_KEY

set -a && source veyron.env && set +a
./veyron api-serve --host 0.0.0.0 --port 5151
./test-package.sh
```

Open **`http://<server-ip>:5151/dashboard`**, enter the API key.

### CLI-only (no long-running server)

```bash
export KUBECONFIG=/path/to/kubeconfig
./veyron list --namespace all
./veyron doctor
```

### HTTPS

```bash
./veyron api-serve --host 0.0.0.0 --port 5151 \
  --tls --tls-cert /etc/veyron/tls.crt --tls-key /etc/veyron/tls.key
```

### systemd (optional)

```ini
[Unit]
Description=Veyron API
After=network.target

[Service]
Type=simple
EnvironmentFile=/etc/veyron/env
WorkingDirectory=/opt/veyron
ExecStart=/opt/veyron/veyron api-serve --host 0.0.0.0 --port 5151
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

## Client prerequisites (cluster side)

The binary is **not** a hypervisor. The client still needs:

- Kubernetes with **KubeVirt** (and CDI if they use DataVolumes / imports)
- A **kubeconfig** whose user/service account can manage VMs (see `deploy/k8s.yaml` `ClusterRole` for the full API feature set)
- Network: client host → Kubernetes API; users → Veyron API port (e.g. `5151`)

## Compare: binary package vs `deploy-remote.sh`

| Approach | Best for |
|----------|----------|
| **`package-binary-remote.sh`** | Hand off a tarball; client runs API on bastion/VM; no in-cluster install |
| **`deploy-remote.sh`** | You operate the cluster; API in `veyron-system` on NodePort (e.g. `30151`) |

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
export VEYRON_API_KEY=Admin@321
./scripts/verify-veyron-remote.sh <api-host> <port>
```

For in-cluster NodePort on k3s, port is often **30151** (HTTPS).

## Related docs

- [README.md](../README.md) — project overview
- [WINDOWS_KUBEVIRT_PRODUCTION.md](WINDOWS_KUBEVIRT_PRODUCTION.md) — Windows RDP / guest-agent
- [DEVELOPER_VM_ACCESS.md](DEVELOPER_VM_ACCESS.md) — SSH / virtctl access
