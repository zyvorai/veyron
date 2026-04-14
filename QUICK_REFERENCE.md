# VMRogue Quick Reference Card

## 🎯 Common Commands

### VM Snapshots & Backup
```bash
vmrogue snapshot-create my-vm --name backup-20260205
vmrogue snapshot-list my-vm          # List snapshots for VM
vmrogue snapshot-list                # List all snapshots
vmrogue snapshot-get backup-20260205 # Get snapshot details
vmrogue snapshot-restore backup-20260205 --target restored-vm
vmrogue snapshot-restore backup-20260205 --in-place  # Overwrite existing
vmrogue snapshot-delete old-snapshot
```

### VM Profiles
```bash
vmrogue profiles                  # List all profiles
vmrogue profiles --details        # Detailed profile info
vmrogue profile database          # View specific profile
```

### Multi-VM Blueprints
```bash
vmrogue blueprints                # List all blueprints
vmrogue blueprints --tag web      # Filter by tag
vmrogue blueprint lamp            # View blueprint details
vmrogue deploy lamp --dry-run     # Preview deployment
vmrogue deploy lamp --prefix prod # Deploy with custom prefix
vmrogue deploy lamp --start       # Deploy and start VMs
```

### Resource Recommendations
```bash
vmrogue recommend database        # Database workload
vmrogue recommend web             # Web server workload
vmrogue recommend ml              # Machine learning workload
```

### Health Checks
```bash
vmrogue health my-vm              # Check VM health
vmrogue health my-vm --detailed   # Detailed checks
```

### VM Operations
```bash
vmrogue create myvm --template ubuntu --profile prod
vmrogue list                      # List all VMs
vmrogue get myvm                  # Get VM details
vmrogue start myvm                # Start VM
vmrogue stop myvm                 # Stop VM
vmrogue delete myvm               # Delete VM
vmrogue status myvm               # Detailed status
vmrogue clone source target       # Clone VM
```

### Templates
```bash
vmrogue templates                 # List all templates
vmrogue template ubuntu-22.04     # View template details
```

### Web Dashboard & API
```bash
# Start API server with dashboard
export VMROGUE_API_KEY="your-key"
vmrogue api-serve --port 5151     # Dashboard at /dashboard

# Deploy to Kubernetes
./scripts/deploy-k8s.sh deploy    # Full K8s deployment
./scripts/deploy-k8s.sh status    # Check deployment
./scripts/deploy-k8s.sh logs      # Tail logs

# Deploy via SSH
./scripts/deploy-ssh.sh HOST USER # Deploy to remote server
```

### Advanced
```bash
vmrogue wizard                    # Interactive wizard
vmrogue batch config.yaml         # Batch operations
vmrogue export myvm               # Export config
vmrogue validate config.yaml      # Validate config
```

---

## 📦 Available Profiles

| Profile | CPU | RAM | Disk | Use Case |
|---------|-----|-----|------|----------|
| minimal | 1 | 512Mi | 5Gi | DNS, agents |
| dev | 1 | 2Gi | 10Gi | Development |
| test | 2 | 4Gi | 20Gi | CI/CD |
| web | 4 | 8Gi | 40Gi | Web servers |
| prod | 4 | 8Gi | 40Gi | Production |
| database | 6 | 16Gi | 200Gi | Databases |
| microservice | 2 | 4Gi | 20Gi | Containers |
| high-perf | 8 | 16Gi | 100Gi | ML, big data |

---

## 🏗️ Available Blueprints

| Blueprint | VMs | Description |
|-----------|-----|-------------|
| lamp | 2 | MySQL + Apache |
| k8s-cluster | 3 | K8s control + workers |
| 3tier | 3 | Web + App + DB |
| cicd | 3 | GitLab + Jenkins + Registry |
| dev-stack | 3 | DB + Cache + Workspace |

---

## 🐧 OS Templates (44 total)

### Linux
- **Ubuntu**: 18.04, 20.04, 22.04, 24.04, latest
- **Fedora**: 38, 39, 40, latest
- **CentOS**: stream9, 7, latest
- **Debian**: 11, 12, latest
- **RHEL**: 8, 9, latest
- **AlmaLinux**: 8, 9, latest
- **Rocky**: 8, 9, latest
- **OpenSUSE**: leap, tumbleweed, latest
- **Alpine**: 3.18, latest
- **Arch**: latest
- **Oracle**: 8, 9, latest

### BSD
- **FreeBSD**: 13, 14, latest

### Container
- **Flatcar**: stable
- **Talos**: latest

### Windows
- 2k19, 2k22, 10, 11, latest

---

## 🎨 Status Symbols

- ● Green = Running
- ◐ Yellow = Pending/Starting
- ○ Gray = Stopped
- ✗ Red = Failed
- ⟳ Blue = Restarting
- ⏸ Yellow = Paused

---

## 💡 Quick Examples

### Create Development VM
```bash
vmrogue create dev-vm --template ubuntu --profile dev
vmrogue start dev-vm
```

### Create Production Database
```bash
vmrogue recommend database
vmrogue create prod-db --template almalinux --profile database
vmrogue health prod-db
vmrogue start prod-db
```

### Deploy LAMP Stack
```bash
vmrogue blueprint lamp
vmrogue deploy lamp --prefix myapp --start
vmrogue list
```

### Deploy Kubernetes Cluster
```bash
vmrogue blueprint k8s-cluster
vmrogue deploy k8s-cluster --prefix prod --start
```

### Check VM Health
```bash
vmrogue health my-vm --detailed
```

---

## 🔧 Configuration

Theme config: `~/.config/vmrogue/tui.toml`

Environment variables:
```bash
export VMROGUE_NAMESPACE=default
export KUBECONFIG=~/.kube/config
```

---

## 📚 More Info

- `INNOVATIVE_FEATURES.md` - Complete feature guide
- `OS_TEMPLATES.md` - All OS templates
- `THEME_DESIGN.md` - Theme documentation
- `README.md` - Main documentation
