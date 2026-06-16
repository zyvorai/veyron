# Veyron (Veyron)

[![CI](https://github.com/ssahani/Veyron/workflows/CI/badge.svg)](https://github.com/ssahani/Veyron/actions)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)

> **Veyron** by Zyvor — Kubernetes-native VM command center. Rogue VM management for KubeVirt — forged in Rust.

A powerful, ergonomic, and extensible Rust CLI, library, and web dashboard to declaratively build, validate, visualize, and apply KubeVirt VMs.

## Features

### Innovative Features (Unique to Veyron)
- **VM Snapshots & Backup** - Production-grade snapshot management via real VirtualMachineSnapshot CRDs
- **8 VM Resource Profiles** - Pre-configured profiles (dev, prod, database, web, etc.)
- **Multi-VM Blueprints** - Deploy complete stacks (LAMP, Kubernetes, 3-tier, CI/CD)
- **Automated Health Checks** - Diagnostics with scoring and recommendations; health API with real K8s connectivity checks, KubeVirt availability, and process uptime
- **Smart Recommendations** - AI-like resource suggestions based on workload
- **Dependency Management** - Automatic VM deployment ordering
- **Web Dashboard** — **Veyron** (Mission Control, Fleet Command, Template Foundry, ConsoleHub, VM Capsule, Stack Health, Event Intelligence, Security Posture) plus 40+ advanced pages via Finder
- **REST API** - 49 endpoints with OpenAPI spec, JWT Bearer auth, multi-key RBAC, rate limiting, and webhooks
- **VNC Console** - Browser-based VM console via direct K8s API WebSocket (no virtctl timeout) with Ctrl+Alt+Del, fullscreen, reconnect
- **Real Kubernetes Metrics** - CPU/memory utilization from Kubernetes Metrics Server (no fake data)
- **Policy Enforcement** - VeyronPolicy CRDs evaluated on VM creation; Deny violations block the request
- **Batch VM Operations** - Start, stop, restart, or delete multiple VMs in a single API call
- **WebSocket Metrics Streaming** - Live cluster metrics pushed every 5 seconds over WebSocket
- **GitOps Export** - `veyron gitops-export` exports VMs as VeyronVM CRD YAML manifests
- **Cost Budget Alerts** - Define per-namespace cost budgets via ConfigMaps with POST/GET API
- **Multi-Cluster Management** - Kubeconfig context discovery with per-cluster sync
- **DR Cross-Cluster Replication** - Export disaster recovery manifests for cross-cluster VM replication
- **Automation Rule Engine** - Execute automation rules with real K8s API calls (start/stop/restart/snapshot/delete)
- **Network Topology** - Real VMI interface data (IP, MAC, interface name) for topology views
- **Topology/Dependency Graph** - Real node-to-VM placement graph from cluster state

### Core Features
- **44 OS Templates** - Ubuntu, Fedora, CentOS, Debian, RHEL, AlmaLinux, Rocky, Alpine, Arch, Windows, and more
- **Flexible configuration** - YAML/JSON configuration files or CLI arguments
- **Built-in validation** - Validate VM configs before deployment
- **Cloud-init support** - Easy VM customization with cloud-init
- **CLI & Library** - Use as a command-line tool or Rust library
- **Full VM lifecycle** - Create, start, stop, restart, clone, export, delete
- **Interactive TUI** - Full terminal UI with ratatui for dashboard, VM management, and monitoring
- **Structured API Errors** - VeyronError with typed variants (NotImplemented, Unauthorized, Forbidden, ResourceConflict, ServiceUnavailable, KubeError) and proper HTTP status codes
- **Real Pod Logs** - Logs API fetches real output from virt-launcher pods
- **Storage & Disk Management** - List real StorageClasses, PVCs, and KubeVirt-labeled disks; expand disks via PVC patch
- **Migration Management** - List, create, and cancel VirtualMachineInstanceMigration CRDs
- **Helm Charts** - Production-ready Helm charts for Veyron API and Veyron Operator
- **Kustomize Overlays** - Dev and prod environment overlays
- **Prometheus Integration** - ServiceMonitor, 6 alert rules, and Grafana dashboard
- **Veyron Operator** - 8 action types, CEL policy expressions, Prometheus metrics
- **Persistent Audit Trail** - Disk-backed audit log for all operations
- **Local Secrets Encryption** - Key expansion and integrity tag for at-rest secret protection

## Architecture

Veyron is two cooperating deliverables:

1. **Rust `veyron`** — CLI, library, and (by default) HTTPS **API + embedded dashboard**. Uses `kube-rs` against the cluster API; can create KubeVirt VMs directly from `VMConfig` / YAML.
2. **Go Veyron Operator (`operator/`)** — **In-cluster** controller-runtime **operator** for **GitOps-style CRs** (`VeyronVM`, blueprints, policies, insights, actions under `veyron.io/v1alpha1`). The main loop turns each **`VeyronVM`** into a **KubeVirt `VirtualMachine`**, keeps it updated, and writes CR **status** (see `operator/internal/controller/veyronvm_controller.go`). Deploy it with **`charts/veyron-operator`** or the scripts that build both API and operator images (`./scripts/deploy-remote.sh` / `deploy-all-remote.sh`).

For day-to-day imperative use you only need the Rust binary; for **declarative CR-driven** VMs, install the operator and apply `VeyronVM` manifests (e.g. from `veyron gitops-export`).

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
git clone https://github.com/ssahani/Veyron.git
cd Veyron
cargo build --release
```

## Web Dashboard

Veyron includes a built-in web dashboard with a dark industrial theme.

### Start the API server

```bash
# Set API keys with RBAC roles (admin/write/readonly)
export VEYRON_API_KEYS="admin:supersecret,write:devkey,readonly:viewkey"

# Or use a single API key (legacy)
export VEYRON_API_KEY="your-secret-key"

# Start the server
veyron api-serve --port 5151 --host 0.0.0.0
```

Optional **snapshot schedule** tuning (built-in cron snapshots use a Kubernetes Lease so only one replica runs the worker when the API is scaled horizontally):

```bash
export VEYRON_SCHEDULER_LEASE_NAMESPACE=veyron-system   # Lease object namespace (defaults to VM default ns)
export VEYRON_SCHEDULER_LEASE_DISABLED=1                   # Skip lease — use only single-replica / dev
```

Open `http://localhost:5151/dashboard` in your browser. The dashboard will prompt for the API key on first load (with a "Remember me" option). JWT Bearer tokens with HMAC-SHA256 signature verification are also supported.

### Dashboard features
- **Dashboard** - Real-time VM fleet overview with live CPU/memory utilization charts (from Kubernetes Metrics Server), real cluster capacity gauges (CPU, memory, nodes), VM summary, and event feed
- **VMs** - Full VM list with search/filter, clickable detail panels showing status, resources, network interfaces (IP/MAC), guest OS info, security posture scoring, and **SSH / remote access** hints (TCP `ssh` and `virtctl ssh` with copy buttons; namespace-aware list). See [docs/DEVELOPER_VM_ACCESS.md](docs/DEVELOPER_VM_ACCESS.md).
- **Batch Operations** - Select multiple VMs and start, stop, restart, or delete them in a single action
- **VNC Console** - Browser-based VM console via bundled noVNC with Ctrl+Alt+Del, fullscreen toggle, and reconnect
- **VM Creation** - Forge VMs from 44 OS templates with profile picker (8 profiles), CPU/memory/disk overrides, cloud-init editor, and auto-start; policy enforcement blocks Deny violations
- **Cost Dashboard** - Per-namespace cost breakdown with forecast chart and budget alert management
- **Snapshots** - Create, restore, and delete VM snapshots from the UI (backed by real VirtualMachineSnapshot CRDs)
- **Nodes** - Kubernetes node table with CPU/memory capacity, kubelet version, OS info
- **Pods** - Namespace pod listing with phase, node, IP, restarts, age
- **Events** - Cluster event feed with auto-refresh
- **Security** - Per-VM security posture scoring (Secure Boot, TPM, RNG, eviction strategy, resource limits) with findings table
- **SOC** - Security Operations page: threat detections, normalized event stream, SIEM export (Elastic/Splunk/Sentinel/QRadar), threat hunts, attack-surface inventory — see [docs/SOC.md](docs/SOC.md)
- **Monitoring** - Prometheus/Grafana detection with health status and recommendations
- **Workloads** - Deployment, StatefulSet, and DaemonSet listing with replica status
- **WebSocket Metrics** - Live cluster metrics streamed over WebSocket (`/api/v1/ws/metrics`) at 5-second intervals
- **Configurable refresh** - 5s/10s/30s/off auto-refresh saved to localStorage

### Deploy to Kubernetes

```bash
# Build container image and deploy
./scripts/deploy-k8s.sh deploy

# Or step by step
./scripts/deploy-k8s.sh build    # Build container image
./scripts/deploy-k8s.sh deploy   # Apply K8s manifests
./scripts/deploy-k8s.sh status   # Check deployment
./scripts/deploy-k8s.sh logs     # Tail pod logs
./scripts/deploy-k8s.sh delete   # Remove everything
```

The K8s deployment includes RBAC (ClusterRole for VMs, nodes, pods, VNC subresources, snapshots), a NodePort service, and automatic API key secret creation. The pod uses `hostPort: 5151` so the dashboard is always reachable at `http://<node-ip>:5151` — a constant, predictable port with no NodePort range indirection. See `deploy/k8s.yaml` for the full manifest.

### Deploy to a remote Kubernetes cluster via SSH

```bash
# Full API + operator deployment (build, import, CRDs/RBAC, deploy, verify).
# Replace HOST and USER with the SSH target that can reach kubectl and your builder (podman/docker).
./scripts/deploy-remote.sh HOST USER

# Equivalent explicit script
./scripts/deploy-all-remote.sh HOST USER

# Fast redeploy: skip image rebuild/import
./scripts/deploy-all-remote.sh HOST USER --quick

# Deploy tuning (default: incremental layer cache, parallel image builds)
# VEYRON_DEPLOY_NO_CACHE=1          # full rebuild (slow)
# VEYRON_DEPLOY_DIAGNOSTICS=1       # cluster-wide pod list before rsync
# VEYRON_SKIP_GUESTKIT_PREP=1       # skip local guestkit rsync

# API-only deployment helper
./scripts/deploy-k8s-remote.sh HOST USER
```

### Ship a Linux binary to clients (no deploy scripts)

Build a **static linux/amd64 tarball** on a remote machine (podman/docker), then download and hand it to the client:

```bash
# Build on remote + fetch to ./dist/
./scripts/package-binary-remote.sh HOST USER --fetch

# Fast repackage when the image already exists
./scripts/package-binary-remote.sh HOST USER --reuse-image --fetch
```

See **[docs/PACKAGE_BINARY_REMOTE.md](docs/PACKAGE_BINARY_REMOTE.md)** for client install, systemd, and troubleshooting.

`deploy-all-remote.sh` now runs with detailed, timestamped logs and phase timings. During image builds it streams full remote output (including Cargo `Compiling ...` lines), so long Rust builds do not look stuck.

By default, full preflight diagnostics are **skipped** (faster on large clusters). Set `VEYRON_DEPLOY_DIAGNOSTICS=1` to print remote OS/kernel, CPU, memory, disk, cluster flavor, and a cluster-wide pod list before rsync.

Runtime import handling auto-detects `k3s`, `rke2`, `microk8s`, `kind`, `minikube`, `containerd`, and `docker-desktop` paths.

### Deploy with Helm

```bash
# Install Veyron API server
helm install veyron ./charts/veyron \
  --set apiKeys="admin:supersecret" \
  --namespace veyron-system --create-namespace

# Install Veyron Operator
helm install veyron-operator ./charts/veyron-operator \
  --namespace veyron-system

# Optional: bundled Prometheus + Grafana + Alertmanager (kube-prometheus-stack)
./scripts/install-veyron-monitoring.sh monitoring
# Uses chart charts/veyron-monitoring; set GRAFANA_ADMIN_PASSWORD for a non-default Grafana admin password.
```

The Veyron and operator Helm charts include RBAC, TLS, and NetworkPolicy. For clusters **without** an existing Prometheus stack, `charts/veyron-monitoring` installs kube-prometheus-stack plus Veyron ServiceMonitors, PrometheusRules, and a Grafana dashboard. The standalone manifests under `deploy/monitoring/` remain useful when you already run kube-prometheus-stack and only need Veyron scrape rules and the dashboard ConfigMap.

### Deploy with Kustomize

```bash
# Development environment
kubectl apply -k deploy/kustomize/overlays/dev

# Production environment
kubectl apply -k deploy/kustomize/overlays/prod
```

### Quick image builds

Use `Dockerfile.deploy` for rapid image builds with virtctl included:

```bash
docker build -f Dockerfile.deploy -t veyron:latest .
```

### Key API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/vms` | GET | List all VMs |
| `/api/v1/vms/batch` | POST | Batch start/stop/restart/delete multiple VMs |
| `/api/v1/ws/metrics` | WS | WebSocket stream of real cluster metrics (5s interval) |
| `/api/v1/costs/budgets` | GET/POST | View and set per-namespace cost budgets |
| `/api/v1/health` | GET | Health check with K8s connectivity, KubeVirt status, uptime |
| `/api/v1/snapshots` | GET/POST | List and create VirtualMachineSnapshot CRDs |
| `/api/v1/migrations` | GET/POST | List and create VirtualMachineInstanceMigration CRDs |
| `/api/v1/storage` | GET | List real StorageClasses and PVCs |
| `/api/v1/metrics` | GET | Real CPU/memory metrics from Kubernetes Metrics Server |
| `/api/v1/logs/{vm}` | GET | Fetch real pod logs from virt-launcher pods |
| `/api/v1/topology` | GET | Node-to-VM placement dependency graph |
| `/api/v1/network` | GET | Network topology from real VMI interfaces |
| `/api/v1/rbac` | GET | ClusterRoles and ClusterRoleBindings |
| `/api/v1/quotas` | GET | ResourceQuotas with CPU/memory limits |
| `/api/v1/workloads` | GET | Deployments, StatefulSets, DaemonSets |
| `/api/v1/hpa` | GET | HorizontalPodAutoscalers with replica status |
| `/api/v1/helm` | GET | Helm releases discovered from Secrets |
| `/api/v1/compliance` | GET | CIS benchmark compliance checking |
| `/api/v1/audit` | GET | Persistent audit trail |

All API handlers are implemented against Kubernetes (and ConfigMaps where persistence is used). Analytics routes (costs, forecasting, etc.) use documented heuristics, not cloud billing APIs. See `docs/FEATURE_MATRIX.md`.

## Quick Start

### Smart VM Creation with Profiles

```bash
# Get recommendations for your workload
veyron recommend database

# Create an optimized database VM
veyron create prod-db --template ubuntu-22.04 --profile database

# Or create a development VM
veyron create dev-vm --template ubuntu --profile dev
```

### Deploy Complete Application Stacks

```bash
# List available blueprints
veyron blueprints

# Deploy a complete LAMP stack
veyron deploy lamp --prefix myapp --start

# Deploy a Kubernetes cluster (1 control plane + 2 workers)
veyron deploy k8s-cluster --prefix prod
```

### Create VMs from Templates (44 templates available)

```bash
# Create an Ubuntu VM
veyron create my-ubuntu --template ubuntu-22.04 --cpus 4 --memory 8Gi

# Create an AlmaLinux VM
veyron create my-alma --template almalinux-9

# Create with custom resources
veyron create my-vm --template fedora-40 --cpus 8 --memory 16Gi --disk-size 100Gi
```

### Check VM Health

```bash
# Run health check on VM
veyron health my-vm

# Get detailed diagnostics
veyron health my-vm --detailed
```

### List and use profiles

```bash
# List all resource profiles
veyron profiles
veyron profiles --details

# View specific profile
veyron profile database

# Get workload recommendations
veyron recommend database
veyron recommend web
```

### Work with blueprints

```bash
# List all blueprints
veyron blueprints
veyron blueprints --tag web

# View blueprint details
veyron blueprint lamp

# Deploy blueprint (dry run)
veyron deploy lamp --dry-run

# Deploy with custom prefix
veyron deploy lamp --prefix myapp --start
```

### List and use templates

```bash
# List all templates
veyron templates

# View template details
veyron template ubuntu-22.04
veyron template almalinux-9 --output json
```

### Generate a VM manifest

```bash
# Generate VMConfig format
veyron generate my-vm --template ubuntu --output vm.yaml

# Generate KubeVirt VirtualMachine CRD (ready to kubectl apply)
veyron generate web-server \
  --template ubuntu \
  --cpus 8 \
  --memory 16Gi \
  --disk-size 100Gi \
  --kubevirt \
  --output webserver.yaml

# Apply to cluster
kubectl apply -f webserver.yaml
```

### Create from configuration file

```bash
veyron create my-custom-vm --from-file examples/basic-vm.yaml
```

### Validate a configuration

```bash
veyron validate examples/ubuntu-cloud-init.yaml
```

### Manage VMs on Kubernetes

```bash
# Create VM on cluster with profile
veyron create production-db --template ubuntu-22.04 --profile database

# List VMs
veyron list
veyron list --all-namespaces
veyron list-json                   # JSON Lines output for scripting

# Get VM details and status
veyron get production-db
veyron status production-db
veyron status production-db --watch  # Watch mode

# Health check
veyron health production-db
veyron health production-db --detailed

# Start/Stop/Restart/Pause VMs
veyron start production-db
veyron stop production-db
veyron restart production-db
veyron pause production-db
veyron unpause production-db

# Resize VM resources
veyron resize production-db --cpus 8 --memory 32Gi

# Clone VM
veyron clone production-db staging-db --start

# Export VM config
veyron export production-db --output prod-db.yaml

# Import VM from KubeVirt YAML manifest
veyron import prod-db.yaml

# Delete VM
veyron delete production-db
veyron delete production-db --yes  # Skip confirmation
```

### Cluster Operations

```bash
# List cluster nodes with status, roles, resources
veyron nodes

# List pods with status, node, IP, restarts
veyron pods

# List Kubernetes events
veyron events

# Cluster capacity analysis
veyron capacity

# Optimal node placement recommendation
veyron placement my-vm --strategy spread

# Search VMs with natural language
veyron search "running vms in production"

# AI-assisted VM diagnostics
veyron troubleshoot my-vm
```

### Multi-Cluster & GitOps

```bash
# List configured clusters
veyron clusters-list

# Discover clusters from kubeconfig
veyron clusters-discover

# Compare local manifests to live cluster state
veyron gitops-diff --namespace production

# Check GitOps sync status
veyron gitops-status
```

### GitOps Export

```bash
# Export all VMs in a namespace as VeyronVM CRD YAML manifests
veyron gitops-export --namespace production --output gitops/

# Export a single VM
veyron gitops-export my-vm --output gitops/my-vm.yaml
```

### Batch VM Operations

```bash
# Start multiple VMs at once via the API
curl -X POST http://localhost:5151/api/v1/vms/batch \
  -H "X-API-Key: $VEYRON_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"action": "start", "vms": ["vm-1", "vm-2", "vm-3"]}'

# Supported actions: start, stop, restart, delete
```

### Cost Budgets

```bash
# Set a cost budget for a namespace
curl -X POST http://localhost:5151/api/v1/costs/budgets \
  -H "X-API-Key: $VEYRON_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"namespace": "production", "monthly_budget": 500.0}'

# View cost budgets
curl http://localhost:5151/api/v1/costs/budgets \
  -H "X-API-Key: $VEYRON_API_KEY"
```

## VM Resource Profiles

Veyron includes **8 pre-configured profiles** optimized for different workloads:

| Profile | CPU | Memory | Disk | Best For |
|---------|-----|--------|------|----------|
| **minimal** | 1 | 512Mi | 5Gi | DNS, jump hosts, monitoring agents |
| **dev** | 1 | 2Gi | 10Gi | Development, testing, learning |
| **test** | 2 | 4Gi | 20Gi | CI/CD pipelines, integration testing |
| **web** | 4 | 8Gi | 40Gi | Nginx, Apache, static sites |
| **prod** | 4 | 8Gi | 40Gi | Production workloads, web apps |
| **database** | 6 | 16Gi | 200Gi | PostgreSQL, MySQL, MongoDB |
| **microservice** | 2 | 4Gi | 20Gi | Container runtime, K8s nodes |
| **high-perf** | 8 | 16Gi | 100Gi | ML, data processing, high traffic |

```bash
veyron profiles              # List all profiles
veyron profile database      # View specific profile
```

## Multi-VM Blueprints

Deploy complete application stacks with **12 ready-to-use blueprints** (5 Linux stacks + 7 Windows enterprise roles):

| Blueprint | VMs | Description |
|-----------|-----|-------------|
| **lamp** | 2 | MySQL database + Apache web server |
| **k8s-cluster** | 3 | 1 control plane + 2 worker nodes |
| **3tier** | 3 | PostgreSQL + App Server + Nginx |
| **cicd** | 3 | GitLab + Jenkins + Artifact Registry |
| **dev-stack** | 3 | Database + Redis Cache + Workspace |
| **windows-jumpbox** | 1 | Windows Server 2022 jump host |
| **windows-ad** | 1 | Domain controller |
| **windows-ad-member** | 2 | DC + member server (ordered) |
| **windows-rds** | 1 | RDS session host |
| **windows-iis** | 1 | IIS web server |
| **windows-sql** | 1 | SQL Server host (database profile) |
| **windows-dev** | 1 | Windows 11 dev workstation |

```bash
veyron blueprints            # List all blueprints
veyron blueprints --tag windows
veyron blueprint lamp        # View blueprint details
veyron deploy lamp --start   # Deploy and start
veyron catalog export        # Export VMTemplate/VMProfile CRD YAML
veyron catalog sync          # Apply catalog to cluster
```

See [docs/TEMPLATE_CATALOG.md](docs/TEMPLATE_CATALOG.md) for operator template resolution and Windows secret refs.

## OS Templates (44 Available)

### Linux Distributions
- **Ubuntu**: 18.04, 20.04, 22.04, 24.04, latest
- **Fedora**: 38, 39, 40, latest
- **CentOS**: stream9, 7, latest
- **Debian**: 11, 12, latest
- **RHEL**: 8, 9, latest
- **AlmaLinux**: 8, 9, latest
- **Rocky Linux**: 8, 9, latest
- **OpenSUSE**: leap, tumbleweed, latest
- **Alpine**: 3.18, latest
- **Arch**: latest
- **Oracle Linux**: 8, 9, latest

### BSD & Container-Optimized
- **FreeBSD**: 13, 14, latest
- **Flatcar**: stable
- **Talos**: latest

### Windows
- **Windows**: 2k19, 2k22, 10, 11, latest

```bash
veyron templates             # List all templates
veyron template ubuntu-22.04 # View template details
```

See [docs/OS_TEMPLATES.md](docs/OS_TEMPLATES.md) for complete catalog.

## Configuration File Format

Create a YAML or JSON file with your VM configuration:

```yaml
name: my-vm
namespace: default
cpu:
  cores: 4
  sockets: 1
  threads: 1
memory:
  size: 8Gi
disks:
  - name: rootdisk
    size: 40Gi
    boot_order: 1
    storage_class: standard
    source:
      type: Blank
interfaces:
  - name: default
    network: default
    model: virtio
    network_type: Pod
cloud_init:
  user_data: |
    #cloud-config
    user: veyron
    password: veyron
    chpasswd: { expire: False }
labels:
  app: my-app
```

## Application Configuration

Veyron supports a layered configuration file for setting defaults:

```bash
# Create default config file
veyron config-init

# View current configuration
veyron config-show
```

**Config file locations** (higher priority wins):
1. CLI arguments (always win)
2. `~/.config/veyron/config.toml` (user)
3. `/etc/veyron/config.toml` (system-wide)
4. Built-in defaults

```toml
# ~/.config/veyron/config.toml
namespace = "production"
kubeconfig = "/home/user/.kube/production"

[logging]
level = "info"           # error, warn, info, debug, trace
format = "text"          # text, json

[api]
port = 8080
host = "0.0.0.0"
tls = false
auth = "none"            # none, api-key, bearer, basic, oauth2, mtls
rate_limit = 60          # requests per minute

[output]
format = "table"         # table, yaml, json
color = true

[tui]
refresh_interval = 5     # seconds
interactive = false
```

## Library Usage

Use veyron as a library in your Rust projects:

```rust
use veyron::config::VMConfigBuilder;
use veyron::output::{to_yaml, OutputFormat};

fn main() -> anyhow::Result<()> {
    let config = VMConfigBuilder::new("my-vm")
        .namespace("production")
        .cpu(4, 1, 1)
        .memory("8Gi")
        .add_blank_disk("rootdisk", "40Gi", 1)
        .add_pod_network("default")
        .label("app", "webserver")
        .build();

    let yaml = to_yaml(&config)?;
    println!("{}", yaml);

    Ok(())
}
```

## Roadmap

### Current Status

**Core** - 44 OS templates, 8 resource profiles, 12 multi-VM blueprints (incl. Windows AD/RDS/IIS/SQL), VMTemplate/VMProfile catalog CRDs, 169+ CLI commands

**Kubernetes** - Full CRUD, VM lifecycle management (including pause/unpause and resize), KubeVirt CRD conversion, health checks with scoring, multi-namespace support, PVC creation, batch operations, manifest generation for all disk and network types. Handlers return live cluster data; some dashboard analytics endpoints use estimates (see `docs/FEATURE_MATRIX.md`).

**TUI** - All 31 TUI views render live data from the cluster. Interactive terminal UI with ratatui covering VM management, security dashboard, cost analytics, compliance, performance profiling, RBAC visualization, migration wizard, and more.

**Operations** - Custom profile/blueprint CRUD with filesystem persistence, cost estimation and budgets (with ConfigMap-backed budget alerts), VM snapshots and backups with scheduling, live migration with progress tracking and HA, GitOps export/diff/status, automation rule execution engine with 8 action types.

**Multi-Cluster & DR** - Multi-cluster management via kubeconfig context discovery with per-cluster sync. DR cross-cluster VM replication with `export_dr_manifests`. CLI commands for cluster listing and discovery.

**Policy & Automation** - VeyronPolicy CRD enforcement on VM creation (blocks Deny violations). Veyron Operator with CEL policy expressions, 8 action types (CreateSnapshot, DeleteVM, Migrate, SendNotification, and 4 more), and Prometheus metrics. Automation rules with real K8s API calls.

**Networking & Security** - IPAM, BGP, DNS, QoS, Cilium, network policies, RBAC, security scanning, compliance (PCI-DSS, HIPAA, SOC2, GDPR, NIST), secret management with zeroization and local encryption. Network topology from real VMI interfaces (IP, MAC, interface name). JWT Bearer token auth with HMAC-SHA256 signature verification. Multi-key RBAC (admin/write/readonly) via `VEYRON_API_KEYS`. Persistent audit trail.

**API & Dashboard** - REST API with 49 endpoints, OpenAPI spec, JWT auth, multi-key RBAC, rate limiting, webhooks. Web dashboard with 15 pages: Dashboard, VMs, Snapshots, Nodes, Pods, Events, Cost, VNC Console, Security, Monitoring, Workloads, and more. VNC console uses direct K8s API WebSocket (no virtctl timeout). Batch VM operations, WebSocket metrics streaming, cost budget management.

**Infrastructure** - Helm charts for veyron and veyron-operator. Kustomize overlays for dev/prod. Prometheus ServiceMonitor with 6 alert rules. Grafana dashboard with 10 panels. Kubeconfig caching, crash-safe atomic persistence, SSRF-safe webhook delivery. Kubernetes deployment with RBAC and NodePort. Docker image with Dockerfile.deploy for quick builds. CI integration tests with Kind + KubeVirt.

### Product boundary

Veyron operates KubeVirt on Kubernetes (lifecycle, policy, observability). **Cross-hypervisor migration** (VMware, Hyper-V, virt-v2v) belongs in [HyperSDK](https://zyvor.dev/hypersdk). See [docs/VMROGUE_HYPERSDK_BOUNDARY.md](docs/VMROGUE_HYPERSDK_BOUNDARY.md).

### Future Enhancements

- [x] Profile auto-selection based on template — `veyron create --template` via `ProfileManager::suggest_profile_for_template`
- [ ] ML-based optimization *(partial: Prometheus-backed forecasting when `VEYRON_PROMETHEUS_URL` is set; see [docs/OPTIONAL_INTEGRATIONS.md](docs/OPTIONAL_INTEGRATIONS.md))*
- [x] Pluggable template registry — `~/.config/veyron/templates.toml` or `VMROGUE_TEMPLATE_REGISTRY`
- [x] Terraform provider — see [`terraform-provider-veyron/`](terraform-provider-veyron/)
- [x] SR-IOV networking — VM config, converter, and CLI support
- [x] OVN networking — `NetworkType::Ovn` + `add_ovn_network()` builder
- [x] DataVolume CRD management (CDI) — `GET /api/v1/images/catalog`, `POST /api/v1/images/import`

## Development

A `Makefile` is provided for common tasks:

```bash
make help       # Show all commands
make test       # Run all tests
make clippy     # Run linter
make lint       # Format check + clippy
make ci         # Full CI pipeline locally
make release    # Build optimized binary (11MB)
make install    # Install to ~/.cargo/bin
make tui        # Launch interactive TUI
```

### Run with debug logging

```bash
veyron --verbose create my-vm --template ubuntu
```

### Build documentation

```bash
cargo doc --open
```

### Browse all 169 commands

```bash
veyron commands
```

## Usage Examples

### Create Development Environment
```bash
veyron recommend development
veyron create dev-vm --template ubuntu --profile dev
veyron start dev-vm
```

### Create Production Database
```bash
veyron recommend database
veyron create prod-db --template almalinux-9 --profile database
veyron health prod-db
veyron start prod-db
```

### Deploy Complete LAMP Stack
```bash
veyron blueprint lamp
veyron deploy lamp --prefix myapp --start
veyron list
```

### Deploy Kubernetes Cluster
```bash
veyron blueprint k8s-cluster
veyron deploy k8s-cluster --prefix prod --namespace kube-system
```

## Documentation

- **[docs/README.md](docs/README.md)** - Documentation index (quick map of all guides)
- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Development status and architecture
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - How to contribute and run CI locally
- **[SECURITY.md](SECURITY.md)** - Security policy and hardening details
- **[CHANGELOG.md](CHANGELOG.md)** - Release changelog
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - Quick reference card
- **[docs/OS_TEMPLATES.md](docs/OS_TEMPLATES.md)** - Full OS template catalog
- **[docs/SNAPSHOTS.md](docs/SNAPSHOTS.md)** - Snapshot and backup management
- **[docs/DISK_MANAGEMENT.md](docs/DISK_MANAGEMENT.md)** - Disk operations guide
- **[docs/NETWORK_MANAGEMENT.md](docs/NETWORK_MANAGEMENT.md)** - Network management guide
- **[docs/DEVELOPER_VM_ACCESS.md](docs/DEVELOPER_VM_ACCESS.md)** - Developer SSH / `virtctl` access to KubeVirt VMs
- **[docs/WINDOWS_KUBEVIRT_PRODUCTION.md](docs/WINDOWS_KUBEVIRT_PRODUCTION.md)** - Windows golden images, Cloudbase-Init, Sysprep, production VM YAML, and **automation** (Packer/GitOps/CDI vs Veyron `cloud_init` NoCloud) ([printable summary](docs/client-presentations/06-windows-kubevirt-production.html))
- **[docs/WINDOWS_PACKER_GITOPS_PIPELINE.md](docs/WINDOWS_PACKER_GITOPS_PIPELINE.md)** - Packer + QEMU → QCOW2 → CDI → Kustomize / **`veyron generate --kubevirt`** + **`cloudInitConfigDrive`** patch
- **`examples/windows-kubevirt-gitops/`** - Sample DataVolume + VM manifests and `scripts/patch_kubevirt_configdrive.py`
- **[docs/INTERACTIVE_TUI.md](docs/INTERACTIVE_TUI.md)** - Interactive TUI guide
- **[docs/ADVANCED_FEATURES.md](docs/ADVANCED_FEATURES.md)** - Advanced features guide

### Configuration Examples

See the `examples/` directory for more configuration examples:

- `basic-vm.yaml` - Simple VM configuration
- `ubuntu-cloud-init.yaml` - Ubuntu VM with cloud-init
- `demo_theme.rs` - Theme demonstration

## Security

Veyron follows secure-by-default principles:

- **No `unsafe` code** - The entire codebase is safe Rust
- **JWT Bearer token auth** - HMAC-SHA256 signature verification for API authentication
- **Multi-key RBAC** - Admin, write, and readonly roles via `VEYRON_API_KEYS` environment variable
- **Persistent audit trail** - All operations logged to disk for forensic review
- **Local secrets encryption** - At-rest encryption with key expansion and integrity tag
- **Structured error types** - VeyronError variants ensure internal details are never leaked
- **Secret zeroization** - Secrets are cleared from memory on drop, rotate, and revoke via the `zeroize` crate
- **SSRF prevention** - Webhook URLs validated against private/internal IPs with DNS rebinding protection
- **Crash-safe persistence** - Atomic writes with fsync and unique temp file names; no `/tmp` fallback
- **Restrictive file permissions** - Persisted data created with mode `0600` on Unix
- **CORS disabled by default** - API server requires explicit origin configuration
- **TLS certificate validation enforced** - Cannot be bypassed via configuration
- **Path traversal protection** - Profile/blueprint names are sanitized before filesystem access
- **Error sanitization** - Internal error details are never leaked to API clients
- **Input validation** - PAM usernames, cron expressions, and API parameters are validated at boundaries
- **Arithmetic safety** - Saturating/checked arithmetic prevents integer overflow throughout
- **No panics in production paths** - All `.unwrap()` / `.expect()` calls verified safe or replaced with error handling
- **Connection pooling** - Kubeconfig cached and reused; HTTP API shares Kubernetes client connections

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under MIT OR Apache-2.0.

## Acknowledgments

- [KubeVirt](https://kubevirt.io/) - Kubernetes Virtualization API
- [kube-rs](https://github.com/kube-rs/kube) - Kubernetes client for Rust
