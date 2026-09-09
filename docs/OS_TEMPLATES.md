# 🎯 Veyron OS Templates - Complete Catalog

**See also:** [Documentation index](README.md), [TEMPLATE_CATALOG.md](TEMPLATE_CATALOG.md) (cluster VMTemplate/VMProfile CRDs).

Veyron includes **44 OS templates** across **15 operating system families**.

## Dual source (CLI vs cluster)

| Source | Use when |
|--------|----------|
| **Rust embedded** (`src/templates/mod.rs`) | Offline CLI: `veyron create`, `veyron deploy`, `veyron templates` |
| **VMTemplate CRDs** (cluster) | Operator GitOps: `template:` on VeyronVM / VeyronBlueprint resolves via operator catalog |

Bootstrap cluster catalog:

```bash
./scripts/generate-catalog-crds.sh
veyron catalog sync   # or deploy-all-remote applies operator/config/catalog/*.yaml
```

Operator and CLI should agree on template **names** (`windows-2022`, `ubuntu-22.04`, …). Blueprint **`override`** still wins for explicit GitOps patches.

## 📊 Template Statistics

- **Total Templates**: 44
- **OS Families**: 15
- **Linux Distros**: 39
- **Windows Versions**: 5
- **BSD Systems**: 3

## 🐧 Available OS Templates

### Ubuntu (5 templates)
```bash
ubuntu          # Default template (Jammy-era image; see `src/templates/mod.rs`)
ubuntu-24.04    # Noble Numbat (2 CPU, 4GB RAM)
ubuntu-22.04    # Jammy Jellyfish (2 CPU, 4GB RAM)
ubuntu-20.04    # Focal Fossa (2 CPU, 4GB RAM)
ubuntu-18.04    # Bionic Beaver (2 CPU, 4GB RAM)
```
**Credentials**: `ubuntu` / `ubuntu`

**Containerdisk**: `ubuntu-18.04` and `ubuntu-20.04` keep their template keys for compatibility; the default root disk uses the v9s-tested refs `quay.io/containerdisks/ubuntu:22.04` and `quay.io/containerdisks/ubuntu:24.04` respectively (not `:18.04` / `:20.04`, which are unreliable for pulls).

### Fedora (2 templates)
```bash
fedora          # Fedora Cloud (containerdisks `fedora:latest`)
fedora-43       # Template key “43”; root disk uses tested `quay.io/containerdisks/fedora:latest`
fedora-42       # Template key “42”; same tested image as `fedora-43` (version-specific tags are not pinned)
```
**Credentials**: `veyron` / `veyron`

### CentOS Stream (3 templates)
```bash
centos              # Latest (Stream 9)
centos-stream-9     # Current stable (2 CPU, 4GB RAM)
centos-stream-8     # Template key “stream-8”; root disk uses tested `quay.io/containerdisks/centos-stream:9`
```
**Credentials**: `veyron` / `veyron`

### Debian (3 templates)
```bash
debian          # Latest (12 Bookworm)
debian-12       # Bookworm (2 CPU, 4GB RAM)
debian-11       # Template key “11”; root disk uses tested `quay.io/containerdisks/debian:12`
```
**Credentials**: `veyron` / `veyron`

### Red Hat Enterprise Linux (3 templates)
```bash
rhel            # Latest (9)
rhel-9          # RHEL 9 (2 CPU, 4GB RAM, 30GB disk)
rhel-8          # RHEL 8 (2 CPU, 4GB RAM, 30GB disk)
```
**Note**: Requires valid RHEL subscription
**Credentials**: `veyron` / `veyron`

### AlmaLinux (3 templates) 🆕
```bash
almalinux       # Latest (9) - RHEL clone
almalinux-9     # AlmaLinux 9 (2 CPU, 4GB RAM)
almalinux-8     # AlmaLinux 8 (2 CPU, 4GB RAM)
```
**Credentials**: `veyron` / `veyron`
**Why**: Free RHEL alternative, binary compatible

### Rocky Linux (3 templates) 🆕
```bash
rocky           # Latest (9) - RHEL clone
rocky-9         # Rocky Linux 9 (2 CPU, 4GB RAM)
rocky-8         # Rocky Linux 8 (2 CPU, 4GB RAM)
```
**Credentials**: `veyron` / `veyron`
**Why**: Community-driven RHEL alternative

### OpenSUSE (3 templates) 🆕
```bash
opensuse                # Latest (Leap)
opensuse-leap           # Leap - Stable (2 CPU, 4GB RAM)
opensuse-tumbleweed     # Tumbleweed - Rolling (2 CPU, 4GB RAM)
```
**Credentials**: `veyron` / `veyron`
**Why**: Enterprise-grade with rolling release option

### Alpine Linux (2 templates) 🆕
```bash
alpine          # Latest (3.19) - Ultra lightweight!
alpine-3.19     # Alpine 3.19 (1 CPU, 512MB RAM!)
```
**Credentials**: `alpine` / `alpine`
**Why**: Smallest Linux distro, perfect for containers/microservices
**Specs**: Only 512MB RAM, 1 CPU, 10GB disk!

### Oracle Linux (3 templates) 🆕
```bash
oracle          # Latest (9)
oracle-9        # Oracle Linux 9 (2 CPU, 4GB RAM)
oracle-8        # Oracle Linux 8 (2 CPU, 4GB RAM)
```
**Credentials**: `veyron` / `veyron`
**Why**: Oracle's enterprise Linux, RHEL compatible

### Arch Linux (1 template) 🆕
```bash
arch            # Rolling release (2 CPU, 2GB RAM)
```
**Credentials**: `veyron` / `veyron`
**Why**: Bleeding edge, highly customizable

### Windows (5 templates)
```bash
windows         # Latest (Server 2022)
windows-2022    # Windows Server 2022 (4 CPU, 8GB RAM, 60GB disk)
windows-2019    # Windows Server 2019 (4 CPU, 8GB RAM, 60GB disk)
windows-11      # Windows 11 (8 vCPU = 4 cores × 2 sockets, 4GB RAM, 16GB disk — lab defaults; increase for production)
windows-10      # Windows 10 (4 vCPU, 8GB RAM, 60GB disk)
```
**Note**: Requires Windows license and installation media. The built-in templates attach the `virtio-win` driver disk, but they do not bundle a Windows installer ISO or preinstalled OS image.

### FreeBSD (3 templates) 🆕
```bash
freebsd         # Latest (14)
freebsd-14      # FreeBSD 14 (2 CPU, 2GB RAM)
freebsd-13      # FreeBSD 13 (2 CPU, 2GB RAM)
```
**Why**: BSD Unix, ZFS, jails, high performance

### Flatcar Linux (1 template) 🆕
```bash
flatcar         # Stable - Container-optimized (2 CPU, 2GB RAM)
```
**Credentials**: `veyron` / `veyron`
**Why**: Container-optimized, auto-updating, minimal

### Talos Linux (1 template) 🆕
```bash
talos           # Latest - Kubernetes-native (2 CPU, 4GB RAM)
```
**Why**: Purpose-built for Kubernetes, API-driven, no SSH

## 🚀 Quick Usage Examples

### Create VMs with Different OS:

```bash
# Ubuntu 24.04 (latest)
veyron create my-ubuntu --template ubuntu-24.04 --cpus 4 --memory 8Gi

# Fedora 41 (latest)
veyron create my-fedora --template fedora-43 --cpus 2 --memory 4Gi

# AlmaLinux (RHEL alternative)
veyron create my-alma --template almalinux --cpus 4 --memory 16Gi

# Rocky Linux (another RHEL alternative)
veyron create my-rocky --template rocky --cpus 4 --memory 16Gi

# Alpine (ultra lightweight!)
veyron create tiny-vm --template alpine --cpus 1 --memory 512Mi

# Arch Linux (bleeding edge)
veyron create arch-vm --template arch --cpus 2 --memory 2Gi

# OpenSUSE Tumbleweed (rolling)
veyron create suse-vm --template opensuse-tumbleweed

# FreeBSD (Unix)
veyron create bsd-vm --template freebsd-14

# Flatcar (container-optimized)
veyron create flatcar-vm --template flatcar

# Talos (Kubernetes-native)
veyron create k8s-node --template talos --cpus 4 --memory 8Gi

# Windows Server 2022
veyron create win-server --template windows-2022 --cpus 8 --memory 16Gi

# Windows 11
veyron create win11-vm --template windows-11 --cpus 4 --memory 8Gi
```

### View Template Details:

```bash
# Show template configuration
veyron template almalinux

# Show as JSON
veyron template rocky --output json

# Show as YAML
veyron template alpine --output yaml
```

### List All Templates:

```bash
# Simple list
veyron templates

# Grouped by family (coming soon)
# veyron templates --by-family
```

## 📋 Template Specifications

| Template | CPU | Memory | Disk | User/Pass | Use Case |
|----------|-----|--------|------|-----------|----------|
| ubuntu-24.04 | 2 | 4Gi | 20Gi | ubuntu/ubuntu | General purpose, latest |
| alpine | 1 | 512Mi | 10Gi | alpine/alpine | Microservices, minimal |
| almalinux | 2 | 4Gi | 20Gi | veyron/veyron | RHEL alternative, free |
| rocky | 2 | 4Gi | 20Gi | veyron/veyron | RHEL alternative, enterprise |
| fedora-43 | 2 | 4Gi | 20Gi | veyron/veyron | Latest features |
| arch | 2 | 2Gi | 20Gi | veyron/veyron | Bleeding edge |
| opensuse-leap | 2 | 4Gi | 20Gi | veyron/veyron | Enterprise stability |
| freebsd-14 | 2 | 2Gi | 20Gi | - | BSD Unix, ZFS |
| flatcar | 2 | 2Gi | 20Gi | veyron/veyron | Containers |
| talos | 2 | 4Gi | 20Gi | - | Kubernetes nodes |
| windows-2022 | 4 | 8Gi | 60Gi | - | Windows Server; TPM in template |
| windows-2019 | 4 | 8Gi | 60Gi | - | Windows Server |
| windows-10 | 4 | 8Gi | 60Gi | - | Windows desktop |
| windows-11 | 8 (4×2×1) | 4Gi | 16Gi | - | Win11; Secure Boot + TPM; **raise RAM/disk** for real installs |

## 🎯 Use Case Guide

### For Web Servers:
```bash
ubuntu-24.04    # Modern, well-supported
rocky           # Enterprise, RHEL-compatible
debian-12       # Stable, lightweight
```

### For Containers:
```bash
alpine          # Minimal footprint
flatcar         # Container-optimized
talos           # Kubernetes-native
```

### For Kubernetes Nodes:
```bash
ubuntu-22.04    # Most common
almalinux       # RHEL compatibility
talos           # Purpose-built for K8s
```

### For Enterprise:
```bash
rhel-9          # Official RHEL
rocky           # Free RHEL alternative
almalinux       # Free RHEL alternative
opensuse-leap   # Enterprise-grade
oracle          # Oracle ecosystem
```

### For Development:
```bash
fedora-43       # Latest packages
arch            # Bleeding edge
ubuntu-24.04    # Modern tools
```

### For Learning/Testing:
```bash
alpine          # Fast, minimal
debian-12       # Stable, well-documented
ubuntu-22.04    # Large community
```

### For BSD Users:
```bash
freebsd-14      # Latest FreeBSD
freebsd-13      # Stable FreeBSD
```

### For Windows:
```bash
windows-2022    # Latest server
windows-11      # Modern desktop
windows-2019    # Stable server
```

## 🔧 Container Disk Images

Most templates use **containerdisks** from `quay.io/containerdisks/` (public tags only):

- `ubuntu:22.04`, `ubuntu:24.04` (tested)
- `fedora:latest` (tested) for Fedora Cloud–style templates
- `centos-stream:9` (tested); stream-8 template may map to `:9`
- `debian:12` (tested); debian-11 template may map to `:12`
- `almalinux:9` (tested). The `:8` tag is gone — AlmaLinux 8 templates use `:9`
- `opensuse-leap:15.6`, `opensuse-tumbleweed:1.0.0`
- **Alpine** and **Rocky** images on quay are auth-gated (401). At convert time
  `resolve_container_disk_image` remaps them: alpine → `quay.io/kubevirt/cirros-container-disk-demo:latest`,
  rocky → `almalinux:9`

Some require blank disks (bring your own ISO):
- RHEL (requires subscription)
- Oracle Linux
- Windows (requires license)
- FreeBSD
- OpenSUSE
- Arch Linux

## ✨ Default Credentials Summary

| OS Family | Username | Password |
|-----------|----------|----------|
| Ubuntu | ubuntu | ubuntu |
| Fedora/CentOS/Debian | veyron | veyron |
| AlmaLinux/Rocky | veyron | veyron |
| Alpine | alpine | alpine |
| Arch/OpenSUSE | veyron | veyron |
| Oracle/Flatcar | veyron | veyron |
| RHEL | veyron | veyron |
| Windows | - | (Configure manually) |
| FreeBSD | - | (Configure manually) |
| Talos | - | (API-driven, no SSH) |

## 🧪 Testing

All templates are tested:

```bash
# Run template tests
cargo test --lib templates

# Test specific template
veyron template almalinux --output json | jq
veyron template alpine --output yaml
```

## 📊 Statistics

```
Total Templates:    44
Linux:              39
Windows:            5
BSD:                3
Container-opt:      2 (Alpine, Flatcar)
Kubernetes-native:  1 (Talos)
Rolling Release:    2 (Arch, OpenSUSE Tumbleweed)
LTS/Stable:         30
```

## 🎨 Themed Output

All template commands support the Veyron theme:

```bash
# Colored template list
veyron templates

# Themed template details
veyron template ubuntu
```

## 🚀 Future Enhancements

Coming soon:
- [ ] More OS versions (Gentoo, NixOS, etc.)
- [ ] Custom template creation
- [ ] Template variants (minimal, desktop, server)
- [ ] Template tags and search
- [ ] Template versioning
- [ ] Community template repository

---

**Now you have 44 OS templates at your fingertips!** 🎉

Choose the right OS for your workload and deploy in seconds!
