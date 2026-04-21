# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Helm chart `charts/vmrogue-monitoring`** — optional install of Prometheus, Grafana, and Alertmanager (kube-prometheus-stack dependency) plus VMRogue ServiceMonitors, PrometheusRules, and Grafana dashboard provisioning; `scripts/install-vmrogue-monitoring.sh` and `make helm-monitoring-validate`.
- **vmrogue-operator Service** — ClusterIP `metrics` / `health` ports so Prometheus `ServiceMonitor` scrapes can reach the operator manager.

#### All 49 API Handlers Now Return Real Data
- **RBAC** - Lists ClusterRoles and ClusterRoleBindings from K8s API
- **Quotas** - Lists ResourceQuotas with CPU/memory limits and usage
- **NetworkPolicies** - Lists policies with pod selectors and rule counts
- **Workloads** - Lists Deployments, StatefulSets, DaemonSets with status
- **Clones** - Creates VM clones via KubeClient
- **Ingress** - Lists Ingress resources with hosts, TLS, and rules
- **HPA** - Lists HorizontalPodAutoscalers with replica status
- **Backups** - Snapshot-based backups via VirtualMachineSnapshot CRDs
- **Security** - Analyzes VMs for RNG, TPM, resource limits
- **Compliance** - CIS benchmark compliance checking across VM fleet
- **Monitoring** - Detects Prometheus/Grafana/Alertmanager services
- **Audit** - Event-based audit trail from K8s events
- **Scheduling** - Pending/scheduled pod counts and unschedulable nodes
- **Heatmap** - Per-node CPU/memory utilization with VM density
- **Helm** - Discovers Helm releases from Secrets (owner=helm)
- **Operators** - Discovers operators from Deployment naming patterns
- **CustomResources** - Lists CRDs via apiextensions API
- **Recommendations** - Config-based recommendations from VM analysis
- **Performance** - Per-VM metrics via MetricsCollector
- **Forecasting** - Resource growth projections from VM/node counts
- **Observability** - Monitoring stack detection and event counts
- **Notifications** - Warning events as notifications
- **Autoscaler** - HPA-based autoscaler policies
- **Webhooks** - CRUD via ConfigMaps (vmrogue.io/type=webhook)
- **Alerts** - Warning events as alerts with create/resolve
- **Cilium** - DaemonSet detection and NetworkPolicy fallback
- **GitOps** - Config discovery and sync status
- **ChaosEngineering** - CRD detection for Chaos Mesh/LitmusChaos
- **SLO** - VM availability as SLO objective
- **CustomDashboards** - Dashboard storage via ConfigMaps
- **RDP** - VM discovery with Windows detection, default config

#### New CLI Commands
- `pause` / `unpause` - Pause/unpause running VMs via virtctl
- `resize` - Resize VM CPU and/or memory
- `events` - List K8s events with table/yaml/json output
- `nodes` - List cluster nodes with status, roles, resources
- `pods` - List pods with status, node, IP, restarts
- `import` - Import VM from KubeVirt YAML manifest
- `clusters-list` / `clusters-discover` - Multi-cluster management
- `gitops-diff` / `gitops-status` - GitOps manifest comparison
- `search` - Natural language VM search (e.g. "running vms in production")
- `troubleshoot` - AI-assisted VM diagnostics
- `capacity` - Cluster capacity analysis with per-node breakdown
- `placement` - Optimal node recommendation (spread/binpack/leastloaded)
- `list-json` - JSON Lines output for scripting

#### All 31 TUI Views Render Live Data
- All 31 TUI views now render live data from the cluster (was 12 real + 19 preview)
- SecurityDashboard, CostAnalytics, ComplianceChecker, VulnerabilityScanner
- AuditPanel, Timeline, DependencyGraph, ForecastDashboard
- PerformanceProfiler, NlpSearch, RbacVisualizer, CustomMetrics
- Autoscaler, SecurityPosture, AiTroubleshoot, ChangeApproval
- MacroView, SessionSharing, MigrationWizard
- Plus all 12 previously-wired views

#### Infrastructure
- **Helm charts** for VMRogue API and operator with full RBAC, TLS, NetworkPolicy
- **Kustomize overlays** for dev and prod environments
- **Prometheus ServiceMonitor** and **PrometheusRule** with 6 alerts
- **Grafana dashboard** with 10 panels (VM count, CPU, memory, network, storage, migrations)
- **Operator Prometheus metrics** (7 custom metrics)

#### Security & Auth
- **Multi-key RBAC** via VMROGUE_API_KEYS (admin/write/readonly roles)
- **JWT Bearer token** validation with OIDC issuer checking
- **Persistent audit trail** saved to disk
- **Local secrets encryption** with key expansion and integrity tag
- **Webhook delivery** via curl subprocess with SSRF protection
- **Notification delivery** for Log and Webhook channels

#### Operator Enhancements
- **CEL policy expressions** via google/cel-go for custom compliance rules
- **4 new action types**: CreateSnapshot, DeleteVM, Migrate, SendNotification

#### All Modules Promoted from Experimental (Zero Feature Gates Remaining)
- capacity, edge, finops, multicloud, servicemesh, hypervisor
- nlp_search, ai_troubleshoot, session_sharing
- The `experimental` Cargo feature flag has been removed; all modules are included in default builds

#### Web Dashboard
- Security page with posture score and findings table
- Monitoring page with Prometheus/Grafana detection and recommendations
- Workloads page with Deployment/StatefulSet/DaemonSet listing

#### All API Handlers Wired to Real Kubernetes (49 total, zero stubs)
- All 49 API handlers return real data from the Kubernetes API
- Previously 19 handlers were wired with 31 stubs returning HTTP 501; now all stubs have been replaced with real implementations
- **VNC Console** uses direct K8s API WebSocket (no virtctl timeout)

#### New Capabilities
- **WebSocket metrics streaming** - Live metrics via `/api/v1/ws/metrics`
- **Policy enforcement** - VMRoguePolicy CRDs enforce constraints on VM creation
- **Batch VM operations** - `POST /api/v1/vms/batch` for bulk VM lifecycle actions
- **Cost dashboard enhancements** - Namespace breakdown, cost forecast
- **GitOps export** - `vmrogue gitops-export` command to export VM manifests
- **Cost budget alerts** - ConfigMap-based budgets via `POST/GET /costs/budgets`
- **Multi-cluster management** - Kubeconfig context discovery for managing multiple clusters
- **DR cross-cluster replication** - `export_dr_manifests` for disaster recovery across clusters
- **Automation execution engine** - Executes automation rules via real K8s API calls

#### Web Dashboard
- Full-featured web dashboard embedded in binary (single HTML, no build tools)
- **VNC Console** - Browser-based VM console via bundled noVNC (284KB), using direct K8s API WebSocket (no virtctl timeout)
- **VM Detail Panel** - Click any VM to expand inline: status, resources, network interfaces (IP/MAC), guest OS info, security posture
- **VM Creation Modal** - Forge VMs from 43 templates with profile picker (8 profiles), CPU/memory/disk overrides, cloud-init editor, auto-start
- **Security Posture** - Per-VM scoring (0-100) checking Secure Boot, TPM 2.0, RNG, eviction strategy, resource limits
- **Nodes Page** - Kubernetes node table with CPU/memory capacity/allocatable, kubelet version, OS image
- **Pods Page** - Pod listing with phase, node, IP, containers, restarts, age
- **Search & Filter** - Text search and status dropdown on VMs page
- **Snapshot Management** - Create, restore, and delete snapshots from the UI
- **Real Capacity Gauges** - Dashboard resource bars show actual cluster CPU/memory utilization from node data
- **API Key Modal** - Login modal with password field, eye toggle, "Remember me" (localStorage persistence)
- **Configurable Refresh** - 5s/10s/30s/off auto-refresh interval saved to localStorage
- **Activity Feed** - VM-specific event feed filtered to VirtualMachine/VMI/Snapshot kinds

#### API Endpoints (12 new)
- `GET /api/v1/nodes` - Kubernetes nodes with capacity, kubelet version, OS
- `GET /api/v1/pods` - Pods with phase, node, IP, containers, restarts
- `GET /api/v1/profiles` - VM resource profiles (8 built-in)
- `GET /api/v1/namespaces` - Namespace listing
- `GET /api/v1/activity` - VM-specific activity feed
- `GET /api/v1/vms/:ns/:name/vnc` - WebSocket VNC proxy via virtctl
- `GET /api/v1/vms/:ns/:name/security` - VM security posture scoring
- `POST /api/v1/vms` - Create VM with template, profile, cloud-init, auto-start
- `POST /api/v1/snapshots/:ns/:vm/create` - Create snapshot
- `POST /api/v1/snapshots/:ns/:name/restore` - Restore snapshot
- `GET /api/v1/templates` - List all VM templates with specs
- `GET /api/v1/dashboard/overview` - Enhanced with real node capacity data

#### Kubernetes Deployment
- `deploy/k8s.yaml` - Full manifest with RBAC (VMs, nodes, pods, VNC subresources, snapshots), NodePort 30151
- `scripts/deploy-k8s.sh` - One-command K8s deploy (build/push/deploy/status/logs/delete)
- `Dockerfile` - Scratch-based image (~75MB) with static musl binary + virtctl

#### Remote Deployment
- `scripts/deploy-k8s-remote.sh` - Deploy to remote Kubernetes cluster via SSH (replaces systemd-based deploy)
- `Dockerfile.deploy` - Quick image builds with virtctl included

### Fixed
- **VM creation preserved containerDisk** - Templates with OS container images (quay.io/containerdisks/*) were being converted to blank disks; now preserves disk source type, cloud-init, RNG, clock, features, firmware, and labels
- **Rate limiter** - Dashboard API endpoints exempt from rate limiting (was causing constant 429 errors with 5s auto-refresh)
- **VM detail panel persistence** - Auto-refresh no longer wipes expanded detail panels
- **API key auth** - Token query param support for WebSocket connections; 401 re-prompts for key
- **VNC proxy** - Reuse TCP connection from port check (virtctl accepts only one connection)

### Changed

#### Infrastructure
- **All 49 API handlers return real data** - Zero stub handlers remaining; all previously-501 endpoints now wired to real Kubernetes
- **Structured API error types** - `VMRogueError` expanded with 6 new variants plus `IntoResponse` implementation for consistent error responses
- **CI integration tests** - Added Kind + KubeVirt integration test suite
- **All experimental modules promoted** - The `experimental` Cargo feature flag has been removed; all modules included in default builds
- **Consolidated K8s quantity parsers** - Unified quantity parsing utilities in `utils` module
- **CRDPolicyRule `value` field** - Policy rules now carry a `value` field for threshold-based enforcement
- **K8s-only deployment** - Deployment is Kubernetes-native via `scripts/deploy-k8s-remote.sh`; systemd-based SSH deployment removed

#### Statistics
- **Tests**: 2,433 (all passing)
- **API handlers**: 49 (all returning real data)
- **TUI views**: 31 (all rendering live data)

### Security

- **Auth bypass removed** - Removed referer-based authentication bypass
- **Path traversal prevention** - Profile and blueprint storage now sanitize names to block directory traversal attacks
- **CORS restricted by default** - API server CORS defaults to disabled instead of wildcard `*` origins
- **RDP cert validation enforced** - `ignore_cert` field is now ignored; TLS certificate validation is always enforced
- **Error message sanitization** - HTTP API responses no longer leak internal Kubernetes error details to clients
- **Request ID uniqueness** - API request IDs now include random suffix to prevent collisions under concurrency
- **PAM username length** - Username validation tightened from 256 to 32 characters (PAM LOGIN_NAME_MAX)
- **Secret zeroization** - Replaced `unsafe { as_mut_vec() }` with `zeroize` crate for memory-safe secret clearing on rotate, revoke, and drop
- **Secret non-cloneable** - Removed `Clone` from `Secret` to prevent accidental copies that bypass zeroization
- **SSRF DNS rebinding prevention** - Webhook delivery now resolves DNS upfront, validates all returned IPs, and pins curl via `--resolve` to prevent TOCTOU attacks
- **CGNAT range blocked** - Webhook SSRF filter now blocks `100.64.0.0/10` (RFC 6598 shared address space)
- **Async DNS resolution** - Webhook URL validation uses `tokio::net::lookup_host` instead of blocking `std::net::ToSocketAddrs`
- **No `/tmp` fallback** - Data persistence no longer falls back to world-readable `/tmp`; returns error if `XDG_DATA_HOME`/`HOME` is unset
- **Atomic writes with fsync** - All JSON persistence uses write-to-temp + `sync_all` + rename for crash-safe, race-free file updates
- **Restrictive file permissions** - Persisted data files created with mode `0600` on Unix
- **Unique temp file names** - Atomic writes use randomized `.tmp.<hex>` suffix to prevent races between concurrent writers

### Fixed

#### Crash Prevention
- **Terminal cleanup on panic** - TUI now restores terminal state even if `app.run()` panics or errors
- **Remove server panics** - Replaced `.unwrap()` with safe fallbacks in HTTP server JSON serialization and K8s client initialization
- **Lock poisoning recovery** - TUI blueprint/profile views use `unwrap_or_else` instead of `.expect()` on RwLock
- **Storage init fallback** - Profile/blueprint storage falls back to read-only mode instead of panicking on init failure
- **Rollback lifetime safety** - `execute_rollback()` returns owned `RollbackExecution` instead of borrowed reference

#### Error Handling
- **K8s connection failures surfaced** - Replaced `.unwrap_or_default()` with proper `?` error propagation on `list_vms()` calls in handlers and HTTP server
- **Evacuation error reporting** - `list_all_vms()` in backup handler now logs warnings on failure instead of silently returning empty
- **Audit log overflow warning** - Audit log now emits `log::warn!` when dropping oldest events at capacity
- **Buffer trim logging** - Anomaly detector, autoscaler, leak detector, and log aggregator now log when trimming history buffers

#### Arithmetic Safety
- **Integer overflow prevention** - Cost handler casts use `(val as u64).min(u32::MAX as u64) as u32` for memory/storage values
- **Pagination precision** - Page count calculation uses `u64` arithmetic before clamping to `u32` to avoid truncation
- **Year overflow** - Cost report `monthly_report()` uses `year.saturating_add(1)` instead of `year + 1`
- **Division by zero** - Cost forecast `project_weighted()` now guards `recent_days > 0.0` before dividing

#### Drain Panic Prevention
- Fixed 10 `Vec::drain()` operations across `audit_trail`, `search_history`, `tui/state`, `autoscaler`, `anomaly`, `log_aggregation`, `custom_metrics`, `leak_detector`, and `notifications` that could panic on boundary conditions

#### Logic Bugs
- **Anti-affinity rule fix** - Empty `vm_selector` now correctly means "no match" instead of unconditionally matching all nodes with VMs
- **Cron range validation** - Invalid ranges like `"5-1"` now return `false` instead of silently misbehaving
- **String slice bounds check** - Log pattern extraction guards against out-of-bounds string slicing on trailing quote characters

#### TUI Fixes
- **Tab state preserved** - VM details view retains selected tab when switching views (was always reset to 0)
- **Widget rendering bounds** - Input widget help text uses `saturating_add/sub` to prevent rendering outside allocated area
- **Placement bounds check** - Placement engine uses `.get(i)` instead of direct `[i]` indexing for node alternatives
- **Filter index validation** - `cycle_status_filter()` resets selection index safely against filtered list bounds
- **IP lookup error logging** - TUI state refresh logs debug message on `get_vm_ip()` failure instead of silently dropping
- **Unused import removed** - Removed unused `Span` import from bar chart widget (eliminated compiler warning)

#### Connection Pooling
- **HTTP server client reuse** - All 8 API handlers refactored to use shared `WebState.get_client()` instead of creating new `KubeClient::new()` per request
- **Kubeconfig caching** - Kubeconfig parsed once via `tokio::sync::OnceCell` and reused across all client creation; `KubeClient::new()` now delegates to cached `get_client()`
- **Centralized client factory** - New `kube::get_client()` replaces scattered `Client::try_default()` calls, ensuring consistent kubeconfig resolution

#### Code Quality
- **Centralized data directory** - New `utils::data_dir()` replaces 6 duplicated `dirs::data_dir()` + `/tmp` fallback patterns
- **Centralized atomic write** - New `utils::atomic_write()` replaces 6 duplicated write-to-temp-then-rename implementations
- **VM CLI overrides deduplicated** - Extracted `CliOverrides` struct and `apply_cli_overrides()` to eliminate duplicated override logic between `handle_create` and `handle_generate`

#### RDP Session
- **Serialization error handling** - RDP session creation logs error and returns error JSON instead of silently returning empty object

## [0.2.0] - 2026-02-28

### Added

#### Application Configuration
- Layered config file support: `/etc/vmrogue/config.toml` (system) + `~/.config/vmrogue/config.toml` (user)
- `config-show` command - display active configuration with source indicators
- `config-init` command - generate default config file
- Configurable: namespace, kubeconfig, API port/host/TLS/auth, logging level, output format, TUI preferences
- CLI args always take priority over config file values

#### Storage Module
- PVC management types: `PvcSpec`, `PvcStatus`, `StorageClassInfo`
- Storage size utilities: `parse_size_to_bytes()`, `format_bytes()` for K8s size strings (Ki/Mi/Gi/Ti)
- Builder pattern for PVC specs with namespace, storage class, access modes, volume mode

#### TUI Improvements
- Implemented actual VM creation in interactive mode (was stub)
- Implemented actual snapshot creation in interactive mode (was stub)
- Extract disk info from KubeVirt volumes instead of hardcoded values
- Extract node info from VM status conditions

#### CLI Tests
- 25 new CLI argument parsing tests
- Tests for core commands, resource overrides, flags, error cases

#### Integration Tests
- 15 new integration tests (6 → 21 total)
- All 44 templates validated and converted to KubeVirt
- All profiles and blueprints validated
- Example file parsing tests
- End-to-end workflow tests

### Changed

#### Architecture Refactor
- Extracted all command handlers from `lib.rs` into `src/handlers/` module (11 submodules)
- `lib.rs` reduced from 5,335 to 575 lines (-89%)
- Handler modules: vm, profiles, infra, backup, security, cost, automation, observability, multitenancy, devexp, api

#### Code Quality
- Eliminated all 79 compiler warnings
- Eliminated all 64 clippy warnings
- Replaced ~30 unsafe `.unwrap()` calls with proper error handling
- Fixed RwLock guards held across await points
- Renamed `from_str()` methods to `parse()` to avoid `FromStr` trait confusion
- Applied `cargo fmt` across entire codebase (189 files)

#### Naming Conventions
- Fixed non-camel-case enum variants: `PCI_DSS` → `PciDss`, `AI_ML` → `AiMl`, `AWS_KMS` → `AwsKms`, etc.
- Fixed deprecated `Frame::size()` → `Frame::area()`

#### CI/CD
- Added `RUST_MIN_STACK` to prevent test stack overflow
- Added job dependencies: fmt → clippy → test → build
- Consolidated cache paths

#### Documentation
- Updated `DEVELOPMENT.md` to reflect current state
- Removed 11 stale progress/completion reports
- Moved feature docs into `docs/` directory
- Clean project root: README, DEVELOPMENT, CONTRIBUTING, CHANGELOG, SECURITY, QUICK_REFERENCE

### Fixed
- Fixed 2 failing tests (`test_create_custom` in profiles and blueprints) - stale test data cleanup
- Fixed 5 snapshot test compilation errors (async/await, type mismatches)
- Fixed test imports broken by unused import cleanup
- Fixed 17 unused `mut` warnings in test code

#### Release Build
- Added LTO, single codegen unit, strip symbols to release profile
- Binary size reduced from 17MB to 11MB

### Statistics
- Commands: 169
- Templates: 44 OS templates
- Resource Profiles: 8 built-in
- Deployment Blueprints: 5 built-in
- Tests: 2,388 (all passing: 2,326 unit + 54 integration + 8 doc)
- Compiler warnings: 0
- Clippy warnings: 0
- Lines of code: ~107,000

## [0.1.0] - 2024-02-05

### Added

#### Core VM Management
- `create` command - Create VMs from templates or configuration files
- `list` command - List VMs with table, YAML, or JSON output
- `get` command - Get detailed VM information
- `delete` command - Delete VMs with confirmation prompt
- `start`, `stop`, `restart` commands - VM lifecycle management
- `status` command - Detailed VM status with watch mode
- `clone` command - Clone existing VMs
- `resources` command - Cluster-wide resource usage summary
- `export` command - Export VM configurations
- `wizard` command - Interactive VM creation
- `batch` command - Batch VM creation from files

#### Templates & Configuration
- 6 built-in VM templates (Ubuntu, CentOS, Fedora, Debian, RHEL, Windows)
- `generate` command - Generate VM manifests
- `validate` command - Validate configuration files
- Cloud-init support, Builder pattern

#### Kubernetes Integration
- Full KubeVirt VirtualMachine CRD support
- VMConfig to KubeVirt manifest converter
- CRUD operations via Kubernetes API

#### Developer Features
- 31 unit and integration tests
- Library API for programmatic usage
- CI/CD with GitHub Actions

[Unreleased]: https://github.com/ssahani/VMRogue/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/ssahani/VMRogue/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ssahani/VMRogue/releases/tag/v0.1.0
