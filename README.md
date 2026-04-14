# VMRogue

[![CI](https://github.com/ssahani/VMRogue/workflows/CI/badge.svg)](https://github.com/ssahani/VMRogue/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

> Rogue VM management for KubeVirt — forged in Rust.

A powerful, ergonomic, and extensible Rust CLI, library, and web dashboard to declaratively build, validate, visualize, and apply KubeVirt VMs.

## Features

### Innovative Features (Unique to VMRogue)
- **VM Snapshots & Backup** - Production-grade snapshot management for disaster recovery
- **8 VM Resource Profiles** - Pre-configured profiles (dev, prod, database, web, etc.)
- **Multi-VM Blueprints** - Deploy complete stacks (LAMP, Kubernetes, 3-tier, CI/CD)
- **Automated Health Checks** - Diagnostics with scoring and recommendations
- **Smart Recommendations** - AI-like resource suggestions based on workload
- **Dependency Management** - Automatic VM deployment ordering
- **Web Dashboard** - Real-time dashboard with VNC console, VM detail panels, security posture, node/pod views
- **REST API** - 30+ endpoints with OpenAPI spec, API key auth, rate limiting, and webhooks
- **VNC Console** - Browser-based VM console via noVNC with Ctrl+Alt+Del, fullscreen, reconnect

### Core Features
- **44 OS Templates** - Ubuntu, Fedora, CentOS, Debian, RHEL, AlmaLinux, Rocky, Alpine, Arch, Windows, and more
- **Flexible configuration** - YAML/JSON configuration files or CLI arguments
- **Built-in validation** - Validate VM configs before deployment
- **Cloud-init support** - Easy VM customization with cloud-init
- **CLI & Library** - Use as a command-line tool or Rust library
- **Full VM lifecycle** - Create, start, stop, restart, clone, export, delete
- **Interactive TUI** - Full terminal UI with ratatui for dashboard, VM management, and monitoring

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
git clone https://github.com/ssahani/VMRogue.git
cd VMRogue
cargo build --release
```

## Web Dashboard

VMRogue includes a built-in web dashboard with a dark industrial theme.

### Start the API server

```bash
# Set an API key (required)
export VMROGUE_API_KEY="your-secret-key"

# Start the server
vmrogue api-serve --port 5151 --host 0.0.0.0
```

Open `http://localhost:5151/dashboard` in your browser. The dashboard will prompt for the API key on first load (with a "Remember me" option).

### Dashboard features
- **Dashboard** - Real-time VM fleet overview with live CPU/memory utilization charts, real cluster capacity gauges (CPU, memory, nodes), VM summary, and event feed
- **VMs** - Full VM list with search/filter, clickable detail panels showing status, resources, network interfaces (IP/MAC), guest OS info, and security posture scoring
- **VNC Console** - Browser-based VM console via bundled noVNC with Ctrl+Alt+Del, fullscreen toggle, and reconnect
- **VM Creation** - Forge VMs from 43 OS templates with profile picker (8 profiles), CPU/memory/disk overrides, cloud-init editor, and auto-start
- **Snapshots** - Create, restore, and delete VM snapshots from the UI
- **Nodes** - Kubernetes node table with CPU/memory capacity, kubelet version, OS info
- **Pods** - Namespace pod listing with phase, node, IP, restarts, age
- **Events** - Cluster event feed with auto-refresh
- **Security** - Per-VM security posture scoring (Secure Boot, TPM, RNG, eviction strategy, resource limits)
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

The K8s deployment includes RBAC (ClusterRole for VMs, nodes, pods, VNC subresources, snapshots), a NodePort service on port 30151, and automatic API key secret creation. See `deploy/k8s.yaml` for the full manifest.

### Deploy to a remote server via SSH

```bash
./scripts/deploy-ssh.sh HOST sus
```

Builds a static musl binary, uploads via SCP, installs a systemd service, and starts it.

## Quick Start

### Smart VM Creation with Profiles

```bash
# Get recommendations for your workload
vmrogue recommend database

# Create an optimized database VM
vmrogue create prod-db --template ubuntu-22.04 --profile database

# Or create a development VM
vmrogue create dev-vm --template ubuntu --profile dev
```

### Deploy Complete Application Stacks

```bash
# List available blueprints
vmrogue blueprints

# Deploy a complete LAMP stack
vmrogue deploy lamp --prefix myapp --start

# Deploy a Kubernetes cluster (1 control plane + 2 workers)
vmrogue deploy k8s-cluster --prefix prod
```

### Create VMs from Templates (44 templates available)

```bash
# Create an Ubuntu VM
vmrogue create my-ubuntu --template ubuntu-22.04 --cpus 4 --memory 8Gi

# Create an AlmaLinux VM
vmrogue create my-alma --template almalinux-9

# Create with custom resources
vmrogue create my-vm --template fedora-40 --cpus 8 --memory 16Gi --disk-size 100Gi
```

### Check VM Health

```bash
# Run health check on VM
vmrogue health my-vm

# Get detailed diagnostics
vmrogue health my-vm --detailed
```

### List and use profiles

```bash
# List all resource profiles
vmrogue profiles
vmrogue profiles --details

# View specific profile
vmrogue profile database

# Get workload recommendations
vmrogue recommend database
vmrogue recommend web
```

### Work with blueprints

```bash
# List all blueprints
vmrogue blueprints
vmrogue blueprints --tag web

# View blueprint details
vmrogue blueprint lamp

# Deploy blueprint (dry run)
vmrogue deploy lamp --dry-run

# Deploy with custom prefix
vmrogue deploy lamp --prefix myapp --start
```

### List and use templates

```bash
# List all templates
vmrogue templates

# View template details
vmrogue template ubuntu-22.04
vmrogue template almalinux-9 --output json
```

### Generate a VM manifest

```bash
# Generate VMConfig format
vmrogue generate my-vm --template ubuntu --output vm.yaml

# Generate KubeVirt VirtualMachine CRD (ready to kubectl apply)
vmrogue generate web-server \
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
vmrogue create my-custom-vm --from-file examples/basic-vm.yaml
```

### Validate a configuration

```bash
vmrogue validate examples/ubuntu-cloud-init.yaml
```

### Manage VMs on Kubernetes

```bash
# Create VM on cluster with profile
vmrogue create production-db --template ubuntu-22.04 --profile database

# List VMs
vmrogue list
vmrogue list --all-namespaces

# Get VM details and status
vmrogue get production-db
vmrogue status production-db
vmrogue status production-db --watch  # Watch mode

# Health check
vmrogue health production-db
vmrogue health production-db --detailed

# Start/Stop/Restart VMs
vmrogue start production-db
vmrogue stop production-db
vmrogue restart production-db

# Clone VM
vmrogue clone production-db staging-db --start

# Export VM config
vmrogue export production-db --output prod-db.yaml

# Delete VM
vmrogue delete production-db
vmrogue delete production-db --yes  # Skip confirmation
```

## VM Resource Profiles

VMRogue includes **8 pre-configured profiles** optimized for different workloads:

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
vmrogue profiles              # List all profiles
vmrogue profile database      # View specific profile
```

## Multi-VM Blueprints

Deploy complete application stacks with **5 ready-to-use blueprints**:

| Blueprint | VMs | Description |
|-----------|-----|-------------|
| **lamp** | 2 | MySQL database + Apache web server |
| **k8s-cluster** | 3 | 1 control plane + 2 worker nodes |
| **3tier** | 3 | PostgreSQL + App Server + Nginx |
| **cicd** | 3 | GitLab + Jenkins + Artifact Registry |
| **dev-stack** | 3 | Database + Redis Cache + Workspace |

```bash
vmrogue blueprints            # List all blueprints
vmrogue blueprint lamp        # View blueprint details
vmrogue deploy lamp --start   # Deploy and start
```

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
vmrogue templates             # List all templates
vmrogue template ubuntu-22.04 # View template details
```

See [OS_TEMPLATES.md](OS_TEMPLATES.md) for complete catalog.

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
    user: vmrogue
    password: vmrogue
    chpasswd: { expire: False }
labels:
  app: my-app
```

## Application Configuration

VMRogue supports a layered configuration file for setting defaults:

```bash
# Create default config file
vmrogue config-init

# View current configuration
vmrogue config-show
```

**Config file locations** (higher priority wins):
1. CLI arguments (always win)
2. `~/.config/vmrogue/config.toml` (user)
3. `/etc/vmrogue/config.toml` (system-wide)
4. Built-in defaults

```toml
# ~/.config/vmrogue/config.toml
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

Use vmrogue as a library in your Rust projects:

```rust
use vmrogue::config::VMConfigBuilder;
use vmrogue::output::{to_yaml, OutputFormat};

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

### Current Status (v0.2.0)

**Core** - 44 OS templates, 8 resource profiles, 5 multi-VM blueprints, 169+ CLI commands, configuration validation, YAML/JSON output, cloud-init support.

**Kubernetes** - Full CRUD, VM lifecycle management, KubeVirt CRD conversion, health checks with scoring, multi-namespace support, PVC creation, batch operations, manifest generation for all disk and network types.

**Operations** - Custom profile/blueprint CRUD with filesystem persistence, cost estimation and budgets, VM snapshots and backups with scheduling, live migration with progress tracking and HA, interactive TUI with ratatui.

**Networking & Security** - IPAM, BGP, DNS, QoS, Cilium, network policies, RBAC, security scanning, compliance (PCI-DSS, HIPAA, SOC2, GDPR, NIST), secret management with zeroization.

**API & Dashboard** - REST API with 30+ endpoints, OpenAPI spec, API key auth, rate limiting, webhooks. Web dashboard with VNC console (bundled noVNC), VM detail panels, security posture scoring, node/pod views, search/filter, profile picker, cloud-init editor.

**Infrastructure** - Kubeconfig caching, crash-safe atomic persistence, SSRF-safe webhook delivery. Kubernetes deployment with RBAC and NodePort. SSH deploy scripts. Docker image (scratch-based, ~75MB) with bundled virtctl for VNC.

### Future Enhancements

- [ ] Profile auto-selection based on template
- [ ] ML-based optimization
- [ ] Pluggable template registry (local/remote)
- [ ] Terraform provider
- [ ] SR-IOV and OVN networking
- [ ] DataVolume CRD management (CDI)

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
vmrogue --verbose create my-vm --template ubuntu
```

### Build documentation

```bash
cargo doc --open
```

### Browse all 169 commands

```bash
vmrogue commands
```

## Usage Examples

### Create Development Environment
```bash
vmrogue recommend development
vmrogue create dev-vm --template ubuntu --profile dev
vmrogue start dev-vm
```

### Create Production Database
```bash
vmrogue recommend database
vmrogue create prod-db --template almalinux-9 --profile database
vmrogue health prod-db
vmrogue start prod-db
```

### Deploy Complete LAMP Stack
```bash
vmrogue blueprint lamp
vmrogue deploy lamp --prefix myapp --start
vmrogue list
```

### Deploy Kubernetes Cluster
```bash
vmrogue blueprint k8s-cluster
vmrogue deploy k8s-cluster --prefix prod --namespace kube-system
```

## Documentation

- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Development status and architecture
- **[SECURITY.md](SECURITY.md)** - Security policy and hardening details
- **[CHANGELOG.md](CHANGELOG.md)** - Release changelog
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - Quick reference card
- **[docs/OS_TEMPLATES.md](docs/OS_TEMPLATES.md)** - Full OS template catalog
- **[docs/SNAPSHOTS.md](docs/SNAPSHOTS.md)** - Snapshot and backup management
- **[docs/DISK_MANAGEMENT.md](docs/DISK_MANAGEMENT.md)** - Disk operations guide
- **[docs/NETWORK_MANAGEMENT.md](docs/NETWORK_MANAGEMENT.md)** - Network management guide
- **[docs/INTERACTIVE_TUI.md](docs/INTERACTIVE_TUI.md)** - Interactive TUI guide
- **[docs/ADVANCED_FEATURES.md](docs/ADVANCED_FEATURES.md)** - Advanced features guide

### Configuration Examples

See the `examples/` directory for more configuration examples:

- `basic-vm.yaml` - Simple VM configuration
- `ubuntu-cloud-init.yaml` - Ubuntu VM with cloud-init
- `demo_theme.rs` - Theme demonstration

## Security

VMRogue follows secure-by-default principles:

- **No `unsafe` code** - The entire codebase is safe Rust
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
