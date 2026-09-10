# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **`veyron-rbac-keys` Secret + `deploy/k8s.yaml` `envFrom` entry** for `VEYRON_API_KEYS` — lets a deployment provide readonly/write/admin-role keys without them being wiped by `bootstrap-integrations.sh`'s full regeneration of `veyron-integrations` (same pattern as `veyron-atlas-auth`/`veyron-oidc`). Added after `scripts/customer-readiness.sh`'s P3 RBAC matrix skipped enforcement checks on a deployment that only had a single admin `VEYRON_API_KEY` configured; verified live that a readonly key gets `403` on mutating routes and a write key gets `403` on admin-only routes once the three-role secret is present.

### Changed

- **Pinned KubeVirt/CDI baseline bumped v1.8.4/v1.65.0 → v1.9.0/v1.66.0** (`scripts/cluster/versions.env`) after a full `customer-readiness.sh` go-live gate (P0-P7) plus repeated VM-lifecycle E2E runs passed cleanly against the newer versions on 2026-09-10. `K3S_VERSION` left unchanged pending a check of KubeVirt 1.9.x's own tested-Kubernetes-version window.

### Fixed

- **Real errors silently collapsed into generic/misleading results across the dashboard, API, CLI, and operator** — a recurring pattern: a specific, actionable failure (a ResourceQuota rejection, an admission-webhook denial, a Kubernetes NotFound/Forbidden/Conflict, a real network error) was discarded and replaced with a bare "Internal server error", a false-success response, or a misleading "not configured"/empty UI state, making real problems indistinguishable from healthy-empty states. Fixed across ~30 sites:
  - `src/api/http_server.rs` — `sanitize_error()` gained missing patterns (`deadline exceeded`, `dial tcp`, `connection reset`, `is forbidden`/`exceeded quota`); a new `status_for_sanitized_error()` helper replaced hardcoded `500`s with the correct status (403/404/409) across 23 VM/snapshot/migration/expose/hotplug/resize handlers.
  - `src/api/handlers/{snapshots,clones,crds}.rs` — snapshot/clone/CRD-write handlers no longer return a bare, message-less 500.
  - `src/api/handlers/dr.rs` — `dr_apply`'s `restore_latest_snapshot` no longer reports `"applied"` success when the snapshot lookup failed or found nothing; a DR failover can no longer silently produce a fresh/empty VM while claiming success.
  - `src/api/handlers/{network_policies,backups}.rs` — a failed list call no longer returns an empty array indistinguishable from "genuinely none exist".
  - `src/api/handlers/compliance.rs` — NetworkPolicy/RoleBinding list failures now set `data_incomplete`/`warnings` instead of silently deflating `cis_score`/`nist_score`.
  - `src/api/handlers/gitops.rs` — a failed VM list no longer flips `drift_detected = true` (false "out of sync" alarm from an API hiccup, not real drift).
  - `src/kube/vm_internet.rs` and `operator/internal/network/internet.go` — VM teardown no longer discards NetworkPolicy/CiliumNetworkPolicy delete errors and orphans the object.
  - `operator/internal/controller/veyronvm_controller.go` — KubeVirt Create/Update failures now reach `status.conditions` instead of only a transient Event.
  - `operator/internal/controller/veyronblueprint_controller.go` — child VeyronVM delete errors during blueprint teardown are no longer discarded before removing the finalizer.
  - `operator/internal/controller/veyronpolicy_controller.go` — an invalid `spec.selector` now sets a condition instead of reconciling away silently.
  - `src/api/handlers/packetwolf.rs` — PacketWolf's actual auth model is a JWT bearer token from `POST /api/v1/auth/login`, not a static `X-API-Key` header; a deployment configured with only an API key got a `/status` that reported "reachable" (the unauthenticated `/health` probe succeeded) while every proxied call (`/network/overview`, `/flows`) 401'd. `VEYRON_PACKETWOLF_USERNAME`/`_PASSWORD` now drive a cached, auto-refreshed bearer token; the API key remains a fallback for PacketWolf deployments that genuinely use that model.

### Added

- **Operator `Reconcile()` test coverage** — the operator previously had zero tests exercising a controller's `Reconcile()` method itself (only pure helper functions were tested). Added a fake-client harness (`operator/internal/controller/suite_test.go`) and tests for all 6 reconcilers, including direct regression coverage for the silent-error fixes above (a KubeVirt Create failure surfaces `Ready=False` with the real error; a failing child delete keeps a `VeyronBlueprint`'s finalizer instead of orphaning the child; an invalid `VeyronPolicy` selector sets a condition; a non-`NotFound` egress-cleanup delete error propagates).
- **`lopdf`/`quick-xml` DoS CVEs fixed (RUSTSEC-2026-0187/0194/0195)** — guestkit's PDF export (`src/export/pdf.rs`) migrated `printpdf` 0.7→0.12 (full API rewrite: the old page/layer/`use_text` model replaced with `printpdf`'s `Op`-list model) and `evtx` 0.8→0.12; `quick-xml` no longer appears in the dependency tree, `lopdf` resolves to a fixed 0.44.0. Verified with a permanent PDF-output sanity test (`%PDF-`/`%%EOF` structure) plus a manual visual check.
  - `src/handlers/vm.rs` — `handle_ssh`/`handle_troubleshoot` no longer discard the real connection error; `handle_clone`/`handle_import`/`handle_wizard` now say clearly when the primary action succeeded but a follow-up `start` failed; `handle_batch --continue-on-error` no longer exits `0` when VMs actually failed (the check was unreachable dead code).
  - `src/handlers/crds.rs` — `Api::get` failures other than a real 404 are no longer treated as "doesn't exist yet, so create".
  - `src/api/web/dashboard.html` — the VM resize "Edit Hardware" panel no longer shows fabricated fallback capacity numbers indistinguishable from real measured headroom; Stack Health/Mission Control no longer show "Not detected"/all-zero tiles on API failures indistinguishable from a genuinely healthy-empty cluster; Quick Forge's template dropdown now warns when showing a hardcoded fallback list.

### Added

- **CLI `--gpu` flag** — `veyron create` and `veyron vrvm-create` accept `--gpu <count | resource | resource:count>` (e.g. `--gpu 1`, `--gpu nvidia.com/GRID_T4-2Q:2`).
- **GPU-granting VMProfiles** — `VMProfile.spec.gpus` lets a catalog profile (e.g. `gpu-large`) attach GPUs; a VM's own explicit `gpus` win over the profile's (`operator/internal/catalog/resolver.go`).
- **vGPU migration attestation** — `VEYRON_VGPU_LIVE_MIGRATION=1` (explicit operator attestation; upstream KubeVirt ≤1.8 cannot migrate mdev VMIs, so this is never auto-derived) relaxes the migration gate for vGPU-only VMs and surfaces `day2_ops.vgpu_live_migration` in platform capabilities.
- **E2E GPU coverage** — `test-vm-daily-ops-remote.sh` gains a GPU inventory check, a 422-preflight assertion on GPU-less clusters, and a conditional Tier D GPU-VM lifecycle (create → migrate must 409 → delete) that activates when the cluster advertises GPU resources.
- **Customer feature guide** — GPU VMs (Passthrough) and Migration Eligibility Guard sections (md + html + regenerated PDF; 58 features).

### Fixed

- **virt-launcher label selectors matched nothing on KubeVirt 1.8** — SSH expose Services, logs-by-VM, OpenCost pod→VM mapping, guest Prometheus discovery, and per-VM metrics selected pods by `kubevirt.io/domain`, a label KubeVirt 1.8 launchers no longer carry (verified on v1.8.4: expose Services had **zero endpoints**). All selector sites now use `vm.kubevirt.io/name` (`kube::VM_NAME_LABEL`), with the legacy label kept as a read fallback for pre-1.8 clusters.
- **VeyronVM CRD schema pruned valid fields** — `windows` (sysprep/domain-join secret refs), `allowInternet`, and `cloudInit.userDataSecretRef` were missing from the hand-maintained CRD schema (`operator/config/crd/bases/veyron.io_veyronvms.yaml`), so strict clusters silently dropped them; `cloudInit.userData` is no longer required (a secret ref suffices).
- **Daily-ops E2E false skips** — the Tier-C admin read probes (`platform/versions`, orphan list, …) were sent without the API key and reported misleading "redeploy for Day-2 ops" skips on healthy deployments.
- **TUI migration assistant drift** — the pre-migration wizard now delegates per-VM checks to the same eligibility guard the API enforces (one source of truth, including GPU blockers).
- **`join-remote-worker.sh` on non-Debian hosts** — wireguard-tools install now falls back to dnf/yum/zypper.

- **GPU VMs (passthrough)** — first-class KubeVirt `domain.devices.gpus` support end-to-end: `gpus`/`gpu` on `POST /api/v1/vms` (with `vgpu_profile` sugar and a 422 preflight when no node advertises the resource; `"force": true` overrides), `VMConfig.gpus` + `add_gpu()` builder, operator `VeyronVM.spec.gpus`/`hostDevices`, and automatic KVM-hidden for NVIDIA passthrough. `GET /api/v1/gpus` reports the per-node GPU inventory (passthrough/MIG/vGPU classification); `GET /api/v1/platform/capabilities` gains `day2_ops.gpu_passthrough`, `day2_ops.vgpu`, `nodes.gpu_capable`, and a `gpus` section (resource totals + KubeVirt `permittedHostDevices` counts).
- **Migration eligibility gate** — `POST /vms/:ns/:name/migrate`, `POST /migrations`, and bulk `migrate` now consult a per-VM eligibility check (passthrough GPU/host devices, VMI phase, KubeVirt's `LiveMigratable` condition) and return **409 with a structured blocker list** (code/message/cold-move suggestion) instead of firing a doomed migration. Pass `"force": true` to bypass.
- **Typed MigrationPolicy validation** — `POST/PUT /api/v1/kubevirt/migration-policies` payloads are validated against a typed `migrations.kubevirt.io/v1alpha1` spec (must select something) before the passthrough apply.

### Changed

- **Bulk migrate behavior** — `POST /api/v1/vms/bulk` with `action: "migrate"` now fails individual non-migratable VMs with the blocker text in `results[].error` (previously they "succeeded" into a migration that could never complete).

### Fixed

- **GPU/host-device passthrough never bound** — `hostDevices` serialized the device-plugin resource as `resourceName`, but KubeVirt's schema field is `deviceName`; the field was pruned server-side and the device silently never attached (`src/kube/types.rs`).

- **Distro SVG icons** — per-distro inline SVG icons (`distroSvg()`) for app store, foundry, and launchpad pages; Launchpad fixed to render SVGs correctly (overflow:hidden + sizing CSS).
- **Ask Zeus page** — standalone `/ask-zeus` route with chat thread UI and quick-chip prompts (`src/api/web/dashboard.html`).
- **PVC resize API** — `PATCH /api/v1/storage/pvcs/:ns/:name` with `new_size` body; **Resize** button wired in the storage dashboard UI (`src/api/handlers/storage.rs`).
- **Dashboard favicon** — inline SVG data URI (Veyron orange-to-red gradient with V mark).
- **GitOps Sync button** — `POST /api/v1/gitops/sync` with toast feedback in the GitOps dashboard page.
- **Helm PodDisruptionBudget** — `charts/veyron/templates/pdb.yaml` with `maxUnavailable: 1`; enabled by default in `charts/veyron/values.yaml`.
- **Auto-install build prerequisites** — `deploy-all-remote.sh` installs `build-essential`, `pkg-config`, `libssl-dev`, Node.js ≥18 (via NodeSource 22.x), and Rust/cargo on first deploy; idempotent on re-runs.
- **Dashboard live-data wiring** — ~50 previously-static dashboard pages/actions wired to real API endpoints in `src/api/web/dashboard.html`: cost budgets (CRUD), disks (list/expand), logs query, network interfaces + bandwidth, live migrations (list/cancel), **Tenants** (new page + CRUD), SOC playbooks, RDP sessions/defaults/VMs, KubeVirt migration policies, CRD action approval, storage pools, network-attachment-definitions, notifications test, Windows setup plan, VMIs, platform readiness, cluster locations, saved SOC hunts, and a create-alert modal. `loadPageData()` now triggers each page's full fetcher set.
- **`DELETE /api/v1/costs/budgets/:name`** — delete a cost budget (removes the `veyron-budget-<name>` ConfigMap); wired to a delete button on the budgets table (`src/api/handlers/costs.rs`).
- **`GET /api/v1/snapshots/export`** — registered directly in `http_server.rs` (cluster-wide snapshot export, `namespace=all` aware); wired to the snapshots Export button.

### Changed

- **Operator build toolchain → Go 1.26** — `operator/go.mod` (`go 1.26.0`) and `operator/Dockerfile` (`golang:1.26`) raised from Go 1.23 to match the development toolchain and unblock the operator image build (`GOTOOLCHAIN=local`).
- **Helm charts version** — `charts/veyron` and `charts/veyron-operator` bumped from 0.2.0 to 0.3.0.
- **Dashboard app store** — "Get" label on Forge VM cards replaced with "Forge VM".
- **About modal** — Veyron app icon + Zyvor branding header.

### Fixed

- **Path-param routes under axum 0.7 / matchit 0.7.3** — 18 route definitions used axum-0.8 brace syntax (`/alerts/{id}/resolve`), which matchit 0.7.3 treats as a **literal** segment, so they 404'd on real IDs. Converted to colon syntax (`:id`/`:name`) across `alerts`, `backups`, `clusters`, `crds`, `metrics`, `migrations`, `rdp`, `snapshots`, `soc`, `tenants`, `vmis`, and others — restoring resolve/restore/cancel/approve/delete actions wired in the dashboard.
- **`/api/v1/snapshots/export` unreachable** — the `snapshots` handler router is intentionally excluded from `handlers::all_routes()`, so the export route was never merged; registered it directly in `http_server.rs` alongside the other snapshot routes.
- **`prepare-guestkit-docker.sh` git mode** — tracked as `100644`, so `deploy-all-remote.sh`'s `[ -x … ]` guard skipped GuestKit build-context prep on fresh checkouts (shipping an incomplete `guestkit/` and failing the API image build); marked executable (`100755`).
- **Green CI** — `cargo fmt` drift across 10 files; `tui::colors::gradient::test_gradient_text` expectation (a `VMRogue`→`Veyron` rename leftover: 7 chars → 6); operator `go vet` (resolved by the Go 1.26 toolchain bump).
- **Dockerfiles for podman** — `FROM rust:…`, `FROM alpine:…`, `FROM golang:…` rewritten to fully-qualified `docker.io/library/…` names; podman clusters without unqualified-search registries configured in `/etc/containers/registries.conf` no longer fail image builds.
- **Deploy node version check** — hardened `_cur` assignment in `deploy-all-remote.sh` prereqs block against empty `sed` output to prevent `[ "" -lt 18 ]` arithmetic error under `set -euo pipefail`.
- **Deploy rollout fallback** — `deploy-all-remote.sh` patches pod template annotation (not the Deployment itself) to force a rollout when the same image tag is reused and `imagePullPolicy: Never` would otherwise leave stale pods.
- **Deploy script YAML quoting** — fixed integer quoting and image fallback for selector mismatch in deploy manifests.
- **Launchpad icon rendering** — SVG icons in the launchpad now display correctly (was clipped/invisible).

- **PacketWolf integration** — `VEYRON_PACKETWOLF_URL` / optional `VEYRON_PACKETWOLF_EXTERNAL_URL` and `VEYRON_PACKETWOLF_API_KEY`; auto-wired by `scripts/lib/bootstrap-integrations.sh` when `packetwolf-api` exists in `cilium-system` or `packetwolf`; `GET /api/v1/packetwolf/status`; Integrations page probe; Cilium dashboard banner (`src/api/handlers/packetwolf.rs`).
- **API auth hardening** — route-level RBAC via `src/api/auth_context.rs`; short-lived single-use WebSocket console tickets (`POST /api/v1/ws/ticket`, `src/api/ws_ticket.rs`) for VNC/serial upgrades.
- **`scripts/test-vm-daily-ops-remote.sh`** — curl-based HTTPS E2E for daily VM API workflows (create, lifecycle, SSH/RDP expose, snapshots); optional `VEYRON_E2E_RESTORE=1`.
- **SOC (Security Operations)** — `src/soc/` with normalized `SecurityEvent` stream, ConfigMap-backed store, built-in detections (RDP/SSH expose, drift, privileged VM, scheduling burst, namespace without NetworkPolicy), SIEM push (Elastic ECS, Splunk HEC, Sentinel DCE, QRadar LEEF), Elastic/Splunk threat hunts, attack-surface scan, SOAR webhooks; API `/api/v1/soc/*` and dashboard **Security → SOC** (`docs/SOC.md`).
- **Ask Zeus OpenRouter** — `src/copilot/llm.rs` reads `OPENROUTER_API_KEY` / `OPENROUTER_MODEL` from env; `scripts/configure-zeus-openrouter.sh` wires remote `veyron-integrations` from `~/.zshrc`.
- **Windows RDP NodePort expose** — `GET/PUT/DELETE /api/v1/vms/{ns}/{name}/rdp-expose` with per-VM NodePort (`30100`–`30199`, valid range `30000`–`32767`), Service selector `kubevirt.io/vm`, and Windows spec/userdata bootstrap (`src/kube/windows_rdp.rs`, `src/kube/vm_rdp.rs`).
- **Cilium virt-launcher clusterwide egress** — `deploy/k8s/bootstrap/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml` applied by deploy scripts when CCNP CRD exists (guest internet on default-deny clusters).
- **Per-VM internet egress API** — `GET/PUT/DELETE /api/v1/vms/{ns}/{name}/network/internet` (`src/kube/vm_internet.rs`).

### Changed

- **Remote deploy speed** — `deploy-all-remote.sh` uses layer cache by default (removed `--no-cache`); parallel API + operator image builds; lighter rsync excludes; diagnostics opt-in via `VEYRON_DEPLOY_DIAGNOSTICS=1`; `VEYRON_DEPLOY_NO_CACHE=1` for clean rebuilds; Dockerfile BuildKit cargo cache mounts with `/out` binary staging.
- **Pause/unpause API** — uses KubeVirt `virtualmachineinstances/pause` and `unpause` subresources instead of in-pod `virtctl` (`src/kube/kubevirt_subresources.rs`).
- **RDP NodePort validation** — suggested ports and conflict checks scan all cluster Service NodePorts, not only Veyron RDP services.

### Documentation

- PacketWolf, deploy-speed env vars, and auth/ws-ticket notes across `CLAUDE.md`, `README.md`, `QUICK_REFERENCE.md`, `DEVELOPMENT.md`, `docs/OPTIONAL_INTEGRATIONS.md`, `docs/NETWORK_MANAGEMENT.md`, `docs/FEATURE_MATRIX.md`, and `docs/README.md`.
- Added `docs/SOC.md`; updated `docs/FEATURE_MATRIX.md`, `docs/OPTIONAL_INTEGRATIONS.md`, `docs/README.md`, `CLAUDE.md`, and `deploy/k8s/optional-integrations.env.example.yaml` for SOC/SIEM env vars.
- Refreshed `docs/README.md` as the canonical index (branch note: `main` vs `main-go`, monitoring chart, CONTRIBUTING/SECURITY links).
- Cross-linked guides (TUI, snapshots, disk, network, Windows, feature matrix) and normalized deploy examples to `HOST USER` placeholders in README, QUICK_REFERENCE, and `CLAUDE.md`.
- CONTRIBUTING clone path uses `Veyron` directory name.
- CHANGELOG updated for the dashboard live-data wiring, matchit path-param route fixes, `/api/v1/snapshots/export` and `DELETE /api/v1/costs/budgets/:name`, the Go 1.26 operator toolchain, and the `prepare-guestkit-docker.sh` exec-bit fix. Operator build now requires **Go 1.26** (`operator/go.mod`, `operator/Dockerfile`).

### Added

- **Helm chart `charts/veyron-monitoring`** — optional install of Prometheus, Grafana, and Alertmanager (kube-prometheus-stack dependency) plus Veyron ServiceMonitors, PrometheusRules, and Grafana dashboard provisioning; `scripts/install-veyron-monitoring.sh` and `make helm-monitoring-validate`.
- **veyron-operator Service** — ClusterIP `metrics` / `health` ports so Prometheus `ServiceMonitor` scrapes can reach the operator manager.

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
- **Webhooks** - CRUD via ConfigMaps (veyron.io/type=webhook)
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
- **Helm charts** for Veyron API and operator with full RBAC, TLS, NetworkPolicy
- **Kustomize overlays** for dev and prod environments
- **Prometheus ServiceMonitor** and **PrometheusRule** with 6 alerts
- **Grafana dashboard** with 10 panels (VM count, CPU, memory, network, storage, migrations)
- **Operator Prometheus metrics** (7 custom metrics)

#### Security & Auth
- **Multi-key RBAC** via VEYRON_API_KEYS (admin/write/readonly roles)
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
- **Policy enforcement** - VeyronPolicy CRDs enforce constraints on VM creation
- **Batch VM operations** - `POST /api/v1/vms/batch` for bulk VM lifecycle actions
- **Cost dashboard enhancements** - Namespace breakdown, cost forecast
- **GitOps export** - `veyron gitops-export` command to export VM manifests
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
- **Structured API error types** - `VeyronError` expanded with 6 new variants plus `IntoResponse` implementation for consistent error responses
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
- Layered config file support: `/etc/veyron/config.toml` (system) + `~/.config/veyron/config.toml` (user)
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

[Unreleased]: https://github.com/zyvorai/veyron/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/zyvorai/veyron/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/zyvorai/veyron/releases/tag/v0.1.0
