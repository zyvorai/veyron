# Development Guide

This document is a current codebase map for contributors. It avoids roadmap-style claims and focuses on the structure that exists in the repository today.

## Repository Shape

- Main Rust crate: `src/`
- Kubernetes operator in Go: `operator/`
- Helm charts: `charts/`
- Kubernetes manifests and overlays: `deploy/`
- End-user and feature docs: `docs/`
- Integration tests: `tests/`

Current source footprint from the repository tree:

- `343` Rust/Go source files under `src/` and `operator/`
- `57` top-level public Rust modules exported from `src/lib.rs`
- `49` API handler modules declared in `src/api/handlers/mod.rs`
- `186` command dispatch arms in `src/lib.rs`

## Entry Points

### Rust binary

- `src/main.rs` parses CLI args and calls `vmrogue::run`.
- `src/lib.rs` is the central dispatch layer for the CLI and the public library surface.

### Operator

- `operator/main.go` starts the controller-runtime manager.
- Reconcilers live in `operator/internal/controller`.

## Rust Architecture

### Command surface

- `src/cli/mod.rs` defines the full `clap` command tree.
- Commands are grouped around VM lifecycle, snapshots, backup, migration, networking, security, cost, observability, multitenancy, API server, GitOps, and operator CRDs.

### Dispatch and handlers

- `src/lib.rs` loads config, initializes logging, and matches on the CLI command.
- `src/handlers/mod.rs` groups the higher-level command handlers:
  - `vm`
  - `profiles`
  - `backup`
  - `cost`
  - `automation`
  - `observability`
  - `multitenancy`
  - `api`
  - `security`
  - `infra`
  - `devexp`
  - `crds`

### Kubernetes and KubeVirt integration

- `src/kube/mod.rs` owns client creation, kubeconfig resolution, and common VM operations.
- `src/kube/converter.rs` converts `VMConfig` into KubeVirt manifests.
- `src/kube/types.rs` contains KubeVirt-compatible Rust types.

### Configuration model

- `src/config/types.rs` defines the VM configuration schema.
- `src/config/builder.rs` provides the fluent builder API.
- `src/config/validator.rs` contains config validation rules.
- `src/config/app_config.rs` handles user config loading and defaults.

### API server

- `src/api/http_server.rs` contains the Axum server wiring.
- `src/api/handlers` contains endpoint handlers for the web dashboard and REST API.
- `src/api/openapi.rs` builds the OpenAPI document.
- `src/api/websocket` contains WebSocket handlers for console, metrics, watch, and RDP flows.
- Static dashboard assets are under `src/api/web`.

### TUI

- `src/tui/app.rs` is the TUI state machine and event loop.
- `src/tui/state.rs` stores shared UI state.
- `src/tui/ui` contains screen renderers.
- `src/tui/widgets` contains reusable UI components.

### Feature modules

The crate also exposes many domain modules directly from `src/lib.rs`, including:

- `snapshots`, `backup`, `migration`, `security`, `cost`, `automation`, `observability`
- `profiles`, `blueprints`, `health`, `monitoring`
- `multitenancy`, `gitops`, `networking`, `multicloud`, `servicemesh`
- `capacity`, `edge`, `finops`, `secrets`, `recommendation`
- `operator_crds` for the Rust-side CRD types

## Operator Architecture

The Go operator is a standard **controller-runtime** application (`operator/main.go` registers all reconcilers with one manager).

- CRD type definitions: `operator/api/v1alpha1`
- Reconcilers: `operator/internal/controller`
- KubeVirt conversion logic: `operator/internal/converter/kubevirt.go`
- Optional event bus integration: `operator/internal/eventbus` (enable with **`NATS_URL`** env or `-nats-url` flag)
- Metrics: `operator/internal/metrics/metrics.go`

**Registered controllers** (each watches its own `vmrogue.io` kind):

| Reconciler | File (typical) | Role |
|------------|----------------|------|
| `VMRogueVM` | `vmroguevm_controller.go` | **Primary:** `VMRogueVM` → KubeVirt `VirtualMachine`; finalizers; config-drive Secret when requested; status from VMI |
| `VMRogueBlueprint` | `vmrogueblueprint_controller.go` | Multi-VM / blueprint CRs |
| `VMRoguePolicy` | `vmroguepolicy_controller.go` | Policy CRs |
| `VMRogueInsight` | `vmrogueinsight_controller.go` | Insight CRs |
| `VMRogueAction` | `vmrogueaction_controller.go` | Action CRs |

The main VM reconciler flow in `vmroguevm_controller.go`:

- fetch `VMRogueVM`
- add finalizer
- convert spec to a KubeVirt `VirtualMachine`
- create or update the target KubeVirt object
- read VMI runtime state
- update CR status and conditions

## Common Contributor Workflows

### Build

```bash
cargo build
```

### Test

```bash
RUST_MIN_STACK=8388608 cargo test
```

### Formatting and linting

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
```

### Useful local commands

```bash
cargo run -- doctor
cargo run -- commands
cargo run -- templates
cargo run -- profiles --details
cargo run -- blueprints --details
cargo run -- api-routes
cargo run -- config-show
```

## Documentation Pointers

- Documentation index: `docs/README.md`
- User-facing overview: `README.md`
- Command examples: `QUICK_REFERENCE.md`
- Feature-specific guides: `docs/*.md`
- Contribution workflow: `CONTRIBUTING.md`
