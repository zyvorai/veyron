# Development Status

## Project Statistics

- **Lines of Code**: ~120,000
- **Modules**: 56 public modules + 11 handler modules (all promoted, zero experimental)
- **Templates**: 44 OS templates (with containerDisk images)
- **CLI Commands**: 169+
- **Resource Profiles**: 8 built-in
- **Deployment Blueprints**: 5 built-in
- **API Endpoints**: 49 (REST + WebSocket), all returning real data
- **API Handlers Wired to Real K8s**: 49 (zero stubs remaining)
- **Dashboard Pages**: 15 (Dashboard, VMs, Snapshots, Nodes, Pods, Events, Cost, VNC Console, Security, Monitoring, Workloads, and more)
- **TUI Views**: 31 (all rendering live data)
- **Tests**: 2,433 (all passing)
- **Compiler Warnings**: 0
- **Dependencies**: 30 core + 1 dev
- **Helm Charts**: vmrogue, vmrogue-operator

## Architecture

```
src/
├── lib.rs              (575 lines - thin routing layer)
├── main.rs             (binary entry point)
├── handlers/           (11 modules - command implementations)
│   ├── vm.rs           (core VM CRUD operations)
│   ├── profiles.rs     (profiles, blueprints, deploy, health)
│   ├── infra.rs        (snapshots, monitoring, disk, network)
│   ├── backup.rs       (backup, disaster recovery, migration)
│   ├── security.rs     (scanning, hardening, compliance, audit)
│   ├── cost.rs         (cost analysis, budgets, optimization)
│   ├── automation.rs   (rules, workflows, schedules)
│   ├── observability.rs (logs, metrics, alerts, insights)
│   ├── multitenancy.rs (tenants, users, roles, quotas, groups)
│   ├── devexp.rs       (completions, config templates, diff, init)
│   └── api.rs          (REST API, webhooks, TUI)
├── cli/                (command-line parsing with clap)
├── config/             (VM configuration types, builder, validator)
├── kube/               (Kubernetes client, CRD types, converter)
├── templates/          (44 OS templates)
├── tui/                (terminal UI with ratatui)
├── profiles/           (resource profile system)
├── blueprints/         (multi-VM deployment templates)
├── snapshots/          (KubeVirt snapshot management)
├── monitoring/         (metrics collection and analysis)
├── health/             (VM health checks and scoring)
├── backup/             (backup, recovery, scheduling)
├── migration/          (live migration, HA, evacuation)
├── security/           (scanning, hardening, compliance, audit)
├── cost/               (cost tracking, budgets, optimization)
├── automation/         (rules, workflows, schedules)
├── observability/      (logs, metrics, alerts, insights)
├── multitenancy/       (tenants, RBAC, quotas)
├── api/                (REST API, OpenAPI, webhooks)
│   ├── http_server.rs  (axum web server, 49 real K8s handlers)
│   ├── web/            (dashboard HTML, noVNC bundle)
│   └── handlers/       (49 endpoint handler modules)
├── devexp/             (completions, config templates, diff, init)
├── operator_crds/      (VMRoguePolicy and other operator CRDs)
└── [12 more modules]   (networking, finops, edge, secrets, etc.)

# All modules are included in default builds (no experimental feature gates).
```

## Completed Features

### Core Infrastructure
- [x] KubeVirt CRD definitions and Kubernetes client
- [x] VM CRUD operations (create, list, get, delete, start, stop, restart)
- [x] VM cloning, export, batch operations
- [x] Interactive creation wizard
- [x] Configuration validation (46 tests)
- [x] 43 OS templates with containerDisk images (Ubuntu, Fedora, CentOS, Debian, RHEL, Windows, etc.)
- [x] Builder pattern for VMConfig
- [x] YAML/JSON output formatting
- [x] Policy enforcement on VM creation (VMRoguePolicy CRDs)
- [x] Multi-cluster management (kubeconfig context discovery)
- [x] GitOps export (`vmrogue gitops-export` command)

### Web Dashboard & API
- [x] Single-file HTML dashboard embedded in binary (Inter + JetBrains Mono fonts, dark theme)
- [x] 49 REST API endpoints with JWT Bearer auth, multi-key RBAC, rate limiting, security headers
- [x] All 49 API handlers wired to real Kubernetes (zero stubs remaining)
- [x] VNC console via bundled noVNC (284KB) + direct K8s API WebSocket (no virtctl timeout)
- [x] VM detail panels with status, resources, network interfaces, guest OS, security posture
- [x] VM creation from templates (44) with profile picker (8), cloud-init, auto-start
- [x] Snapshot create/restore/delete from UI
- [x] Nodes and Pods pages with auto-refresh
- [x] Security page with posture score and findings table
- [x] Monitoring page with Prometheus/Grafana detection and recommendations
- [x] Workloads page with Deployment/StatefulSet/DaemonSet listing
- [x] Real cluster capacity gauges (CPU, memory, nodes from K8s API)
- [x] Real Kubernetes Metrics Server integration (replaced fake random data)
- [x] WebSocket metrics streaming (`/api/v1/ws/metrics`)
- [x] Batch VM operations (`POST /api/v1/vms/batch`)
- [x] Cost dashboard with namespace breakdown and forecast
- [x] Cost budget alerts (ConfigMap-based, `POST/GET /costs/budgets`)
- [x] Search/filter on VMs page
- [x] API key login modal with remember me (localStorage)
- [x] Kubernetes deployment with RBAC, NodePort, scratch Docker image (~75MB)
- [x] Helm charts for vmrogue and vmrogue-operator
- [x] Kustomize overlays for dev and prod environments
- [x] Prometheus ServiceMonitor with 6 alert rules
- [x] Grafana dashboard with 10 panels
- [x] K8s-only remote deployment via `scripts/deploy-k8s-remote.sh`
- [x] Structured API error types (`VMRogueError` with 6 new variants + `IntoResponse`)

### Resource Profiles & Blueprints
- [x] 8 built-in profiles (minimal, dev, test, web, prod, database, microservice, high-perf)
- [x] Custom profile CRUD with filesystem persistence
- [x] 5 built-in blueprints (LAMP, K8s cluster, 3-tier, CI/CD, dev-stack)
- [x] Custom blueprint CRUD with validation
- [x] Blueprint deployment with dependency ordering
- [x] Topological sort and cycle detection

### Snapshots & Backup
- [x] KubeVirt snapshot CRD integration
- [x] Snapshot create/list/get/delete/restore
- [x] Retention policies with enforcement
- [x] Backup create/list/verify with compression
- [x] Backup scheduling (daily, hourly, weekly)
- [x] Disaster recovery plans and execution

### Monitoring & Health
- [x] Live VM monitoring with metrics
- [x] Health checks with scoring (0-100)
- [x] Resource recommendations
- [x] Performance comparison across VMs
- [x] Top resource consumers view

### Network & Migration
- [x] Network interface management
- [x] Traffic analysis and bandwidth monitoring
- [x] Network policies (Kubernetes + Cilium)
- [x] Live migration with progress tracking
- [x] HA configuration and failover
- [x] Node evacuation planning

### Security & Compliance
- [x] Vulnerability scanning (quick/standard/deep/compliance)
- [x] Security assessment with scoring
- [x] CIS and STIG hardening profiles
- [x] Compliance checking (PCI-DSS, HIPAA, SOC2, GDPR, NIST)
- [x] Audit logging and statistics

### Cost Management
- [x] Cost analysis per VM and namespace
- [x] Budget management with alerts
- [x] Cost optimization recommendations
- [x] Waste detection and reporting
- [x] Cost forecasting

### Automation & Orchestration
- [x] Automation rules with triggers (schedule, event, metric)
- [x] Multi-step workflows with templates
- [x] Scheduled task management
- [x] Workflow execution tracking
- [x] Automation execution engine (real K8s API calls)

### Observability
- [x] Log querying with filtering
- [x] Metrics collection and aggregation
- [x] Alert rule management
- [x] Insight generation and recommendations
- [x] Trend analysis

### Multi-Tenancy & RBAC
- [x] Tenant management with namespaces
- [x] User and group management
- [x] Role-based access control
- [x] Resource quotas with presets

### Developer Experience
- [x] Shell completions (bash, zsh, fish, powershell, elvish)
- [x] Configuration template save/load/list
- [x] YAML config diff tool
- [x] Project initialization scaffolding
- [x] Environment info and diagnostics

### API & Interface
- [x] REST API server with OpenAPI spec
- [x] API key management
- [x] Webhook management with event filtering
- [x] Interactive TUI with ratatui
- [x] TUI VM creation and snapshot creation

### Additional Modules
- [x] FinOps (allocation, budgets, optimization, waste, reports)
- [x] Advanced networking (IPAM, BGP, DNS, QoS, topology)
- [x] Service mesh integration
- [x] Disaster recovery (failover, HA, replication, cross-cluster DR via `export_dr_manifests`)
- [x] Edge computing (nodes, sync, telemetry)
- [x] Secrets management (encryption, rotation, zeroization via `zeroize` crate)
- [x] Multi-cloud (providers, federation, connectivity, portability)
- [x] Capacity planning
- [x] AI/ML (GPU management, inference)
- [x] GitOps integration

### Security & Robustness
- [x] JWT Bearer token auth with HMAC-SHA256 signature verification
- [x] Multi-key RBAC (admin/write/readonly) via `VMROGUE_API_KEYS`
- [x] Persistent audit trail saved to disk
- [x] Local secrets encryption with key expansion and integrity tag
- [x] Centralized data directory (`utils::data_dir()`) — no `/tmp` fallback
- [x] Atomic file writes with fsync and unique temp names (`utils::atomic_write()`)
- [x] Kubeconfig caching via `tokio::sync::OnceCell` (`kube::get_client()`)
- [x] SSRF prevention with DNS rebinding protection and CGNAT blocking
- [x] Secret memory zeroization on drop/rotate/revoke
- [x] Restrictive file permissions (0600) on persisted data
- [x] Auth bypass removed (referer-based authentication bypass eliminated)
- [x] CI integration tests with Kind + KubeVirt
- [x] Consolidated K8s quantity parsers in `utils`
- [x] All modules promoted (zero experimental feature gates)

## Quick Test Commands

```bash
# Build
cargo build

# Run all tests
cargo test

# List templates
cargo run -- templates

# Show template details
cargo run -- template ubuntu

# Generate a VM manifest
cargo run -- generate my-vm --template fedora --cpus 4 --memory 8Gi

# Validate a configuration
cargo run -- validate examples/basic-vm.yaml

# Create from template (dry-run)
cargo run -- create test-vm --template ubuntu --dry-run

# List profiles
cargo run -- profiles

# List blueprints
cargo run -- blueprints

# Launch TUI
cargo run -- tui --interactive
```

## Design Decisions

1. **Handler-based architecture**: All command logic in `src/handlers/` modules, `lib.rs` is a thin router
2. **Separation of concerns**: Config types separate from Kubernetes CRDs
3. **Builder pattern**: Ergonomic programmatic VM creation
4. **Template system**: Uses `once_cell` for efficient template loading
5. **Validation first**: All configs validated before operations
6. **Async-first**: Built on Tokio for Kubernetes operations
7. **Proper error handling**: No unsafe `.unwrap()` in production code, `anyhow` for error propagation
8. **Centralized persistence**: All stores use `utils::data_dir()` and `utils::atomic_write()` for consistent, crash-safe file I/O
9. **Client caching**: Kubeconfig parsed once and cached; `kube::get_client()` provides cheap client creation
10. **Defense in depth**: SSRF prevention with DNS resolution + IP validation + connection pinning; secret zeroization on all code paths
11. **All handlers return real data**: Every API handler is wired to real Kubernetes; no stubs or fake data
12. **Helm-first deployment**: Helm charts with full RBAC, TLS, and monitoring integration for production deployments
