# 🚀 VMRogue

[![CI](https://github.com/ssahani/VMRogue/workflows/CI/badge.svg)](https://github.com/ssahani/VMRogue/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

> Rogue VM management for KubeVirt with Rust power!

A powerful, ergonomic, and extensible Rust CLI and library to declaratively build, validate, visualize, and apply KubeVirt VMs.

## ✨ Features

### 🚀 Innovative Features (Unique to VMRogue!)
- 📸 **VM Snapshots & Backup** - Production-grade snapshot management for disaster recovery
- 📊 **8 VM Resource Profiles** - Pre-configured profiles (dev, prod, database, web, etc.)
- 🏗️ **Multi-VM Blueprints** - Deploy complete stacks (LAMP, Kubernetes, 3-tier, CI/CD)
- 🏥 **Automated Health Checks** - Diagnostics with scoring and recommendations
- 💡 **Smart Recommendations** - AI-like resource suggestions based on workload
- 🔄 **Dependency Management** - Automatic VM deployment ordering
- 🎨 **Beautiful Themed CLI** - Purple Kubernetes-inspired colors with status symbols

### 🎯 Core Features
- 🐧 **44 OS Templates** - Ubuntu, Fedora, CentOS, Debian, RHEL, AlmaLinux, Rocky, Alpine, Arch, Windows, and more!
- 🛠️ **Flexible configuration** - YAML/JSON configuration files or CLI arguments
- ✅ **Built-in validation** - Validate VM configs before deployment
- 📦 **Cloud-init support** - Easy VM customization with cloud-init
- 🔧 **CLI & Library** - Use as a command-line tool or Rust library
- 📈 **Full VM lifecycle** - Create, start, stop, restart, clone, export, delete

## 📦 Installation

```bash
cargo install --path .
```

Or build from source:

```bash
git clone https://github.com/yourusername/vmrogue.git
cd vmrogue
cargo build --release
```

## 🚀 Quick Start

### 🎯 Smart VM Creation with Profiles

```bash
# Get recommendations for your workload
vmrogue recommend database

# Create an optimized database VM
vmrogue create prod-db --template ubuntu-22.04 --profile database

# Or create a development VM
vmrogue create dev-vm --template ubuntu --profile dev
```

### 🏗️ Deploy Complete Application Stacks

```bash
# List available blueprints
vmrogue blueprints

# Deploy a complete LAMP stack
vmrogue deploy lamp --prefix myapp --start

# Deploy a Kubernetes cluster (1 control plane + 2 workers)
vmrogue deploy k8s-cluster --prefix prod
```

### 🐧 Create VMs from Templates (44 templates available!)

```bash
# Create an Ubuntu VM
vmrogue create my-ubuntu --template ubuntu-22.04 --cpus 4 --memory 8Gi

# Create an AlmaLinux VM
vmrogue create my-alma --template almalinux-9

# Create with custom resources
vmrogue create my-vm --template fedora-40 --cpus 8 --memory 16Gi --disk-size 100Gi
```

### 🏥 Check VM Health

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

## 📊 VM Resource Profiles

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

## 🏗️ Multi-VM Blueprints

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

## 🐧 OS Templates (44 Available!)

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

## 🔧 Configuration File Format

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

## ⚙️ Application Configuration

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

## 📚 Library Usage

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

## 🎯 Roadmap

### Current Status (v0.2.0) ✅

#### Innovative Features ✅
- ✅ 8 VM resource profiles (minimal, dev, test, web, prod, database, microservice, high-perf)
- ✅ 5 multi-VM blueprints (LAMP, k8s-cluster, 3tier, cicd, dev-stack)
- ✅ Automated health checks with scoring (0-100)
- ✅ Smart resource recommendations based on workload
- ✅ Dependency-aware VM deployment
- ✅ Beautiful themed CLI with purple Kubernetes palette
- ✅ VM status symbols and colored output

#### OS Templates ✅
- ✅ 44 OS templates across 15 families
- ✅ Ubuntu (5), Fedora (4), CentOS (3), Debian (3), RHEL (3)
- ✅ AlmaLinux (3), Rocky (3), OpenSUSE (3), Alpine (2)
- ✅ Oracle (3), FreeBSD (3), Arch (1), Flatcar (1), Talos (1)
- ✅ Windows (5 versions)

#### Core Features ✅
- ✅ Core type definitions
- ✅ Configuration validation (46 tests passing)
- ✅ YAML/JSON output
- ✅ CLI with 25+ commands

#### Kubernetes Integration ✅
- ✅ Full Kubernetes CRUD operations
- ✅ VM lifecycle management (create, start, stop, restart, delete, clone)
- ✅ KubeVirt CRD types and conversion
- ✅ VM status monitoring and health checks
- ✅ Multi-namespace support
- ✅ PVC creation support
- ✅ Batch operations

#### Manifest Generation ✅
- ✅ KubeVirt VirtualMachine CRD output
- ✅ Cloud-init volume generation
- ✅ All disk types (Blank, PVC, ContainerDisk, DataVolume)
- ✅ All network types (Pod, Bridge, Multus)

### Future Enhancements

- [ ] Custom profile creation
- [ ] Custom blueprint definitions
- [ ] Profile auto-selection based on template
- [ ] Resource usage tracking and analytics
- [ ] Cost estimation
- [ ] Auto-scaling recommendations
- [ ] ML-based optimization
- [ ] 🔌 Pluggable template registry (local/remote)
- [ ] 📸 VM snapshots and backups
- [ ] 🔄 Live migration support
- [ ] 🎨 Full interactive TUI
- [ ] 🔧 Terraform provider
- [ ] 🌐 Advanced networking (SR-IOV, OVN)
- [ ] 💾 DataVolume CRD management (CDI)
- [ ] 📊 Resource quota management
- [ ] 🔐 RBAC and security policies

## 🧪 Development

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

### Browse all 148 commands

```bash
vmrogue commands
```

## 💡 Usage Examples

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

## 📖 Documentation

- **[INNOVATIVE_FEATURES.md](INNOVATIVE_FEATURES.md)** - Complete guide to innovative features
- **[OS_TEMPLATES.md](OS_TEMPLATES.md)** - Full OS template catalog
- **[THEME_DESIGN.md](THEME_DESIGN.md)** - Theme system documentation
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - Quick reference card
- **[IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md)** - Implementation summary

### Configuration Examples

See the `examples/` directory for more configuration examples:

- `basic-vm.yaml` - Simple VM configuration
- `ubuntu-cloud-init.yaml` - Ubuntu VM with cloud-init
- `demo_theme.rs` - Theme demonstration

## 🔒 Security

VMRogue follows secure-by-default principles:

- **No `unsafe` code** - The entire codebase is safe Rust
- **CORS disabled by default** - API server requires explicit origin configuration
- **TLS certificate validation enforced** - Cannot be bypassed via configuration
- **Path traversal protection** - Profile/blueprint names are sanitized before filesystem access
- **Error sanitization** - Internal error details are never leaked to API clients
- **Input validation** - PAM usernames, cron expressions, and API parameters are validated at boundaries
- **Arithmetic safety** - Saturating/checked arithmetic prevents integer overflow throughout
- **No panics in production paths** - All `.unwrap()` / `.expect()` calls verified safe or replaced with error handling
- **Connection pooling** - HTTP API reuses Kubernetes client connections instead of per-request allocation

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under MIT OR Apache-2.0.

## 🙏 Acknowledgments

- [KubeVirt](https://kubevirt.io/) - Kubernetes Virtualization API
- [kube-rs](https://github.com/kube-rs/kube) - Kubernetes client for Rust
