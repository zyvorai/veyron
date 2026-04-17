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
- `57` top-level public Rust modules exported from [`src/lib.rs`](/Users/ssahani/tt/VMRogue/src/lib.rs:1)
- `49` API handler modules declared in [`src/api/handlers/mod.rs`](/Users/ssahani/tt/VMRogue/src/api/handlers/mod.rs:1)
- `186` command dispatch arms in [`src/lib.rs`](/Users/ssahani/tt/VMRogue/src/lib.rs:1)

## Entry Points

### Rust binary

- [`src/main.rs`](/Users/ssahani/tt/VMRogue/src/main.rs:1) parses CLI args and calls `vmrogue::run`.
- [`src/lib.rs`](/Users/ssahani/tt/VMRogue/src/lib.rs:1) is the central dispatch layer for the CLI and the public library surface.

### Operator

- [`operator/main.go`](/Users/ssahani/tt/VMRogue/operator/main.go:1) starts the controller-runtime manager.
- Reconcilers live in [`operator/internal/controller`](/Users/ssahani/tt/VMRogue/operator/internal/controller).

## Rust Architecture

### Command surface

- [`src/cli/mod.rs`](/Users/ssahani/tt/VMRogue/src/cli/mod.rs:1) defines the full `clap` command tree.
- Commands are grouped around VM lifecycle, snapshots, backup, migration, networking, security, cost, observability, multitenancy, API server, GitOps, and operator CRDs.

### Dispatch and handlers

- [`src/lib.rs`](/Users/ssahani/tt/VMRogue/src/lib.rs:97) loads config, initializes logging, and matches on the CLI command.
- [`src/handlers/mod.rs`](/Users/ssahani/tt/VMRogue/src/handlers/mod.rs:1) groups the higher-level command handlers:
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

- [`src/kube/mod.rs`](/Users/ssahani/tt/VMRogue/src/kube/mod.rs:1) owns client creation, kubeconfig resolution, and common VM operations.
- [`src/kube/converter.rs`](/Users/ssahani/tt/VMRogue/src/kube/converter.rs:1) converts `VMConfig` into KubeVirt manifests.
- [`src/kube/types.rs`](/Users/ssahani/tt/VMRogue/src/kube/types.rs:1) contains KubeVirt-compatible Rust types.

### Configuration model

- [`src/config/types.rs`](/Users/ssahani/tt/VMRogue/src/config/types.rs:1) defines the VM configuration schema.
- [`src/config/builder.rs`](/Users/ssahani/tt/VMRogue/src/config/builder.rs:1) provides the fluent builder API.
- [`src/config/validator.rs`](/Users/ssahani/tt/VMRogue/src/config/validator.rs:1) contains config validation rules.
- [`src/config/app_config.rs`](/Users/ssahani/tt/VMRogue/src/config/app_config.rs:1) handles user config loading and defaults.

### API server

- [`src/api/http_server.rs`](/Users/ssahani/tt/VMRogue/src/api/http_server.rs:1) contains the Axum server wiring.
- [`src/api/handlers`](/Users/ssahani/tt/VMRogue/src/api/handlers) contains endpoint handlers for the web dashboard and REST API.
- [`src/api/openapi.rs`](/Users/ssahani/tt/VMRogue/src/api/openapi.rs:1) builds the OpenAPI document.
- [`src/api/websocket`](/Users/ssahani/tt/VMRogue/src/api/websocket) contains WebSocket handlers for console, metrics, watch, and RDP flows.
- Static dashboard assets are under [`src/api/web`](/Users/ssahani/tt/VMRogue/src/api/web).

### TUI

- [`src/tui/app.rs`](/Users/ssahani/tt/VMRogue/src/tui/app.rs:1) is the TUI state machine and event loop.
- [`src/tui/state.rs`](/Users/ssahani/tt/VMRogue/src/tui/state.rs:1) stores shared UI state.
- [`src/tui/ui`](/Users/ssahani/tt/VMRogue/src/tui/ui) contains screen renderers.
- [`src/tui/widgets`](/Users/ssahani/tt/VMRogue/src/tui/widgets) contains reusable UI components.

### Feature modules

The crate also exposes many domain modules directly from [`src/lib.rs`](/Users/ssahani/tt/VMRogue/src/lib.rs:1), including:

- `snapshots`, `backup`, `migration`, `security`, `cost`, `automation`, `observability`
- `profiles`, `blueprints`, `health`, `monitoring`
- `multitenancy`, `gitops`, `networking`, `multicloud`, `servicemesh`
- `capacity`, `edge`, `finops`, `secrets`, `recommendation`
- `operator_crds` for the Rust-side CRD types

## Operator Architecture

The Go operator is a standard controller-runtime application.

- CRD type definitions: [`operator/api/v1alpha1`](/Users/ssahani/tt/VMRogue/operator/api/v1alpha1)
- Reconcilers: [`operator/internal/controller`](/Users/ssahani/tt/VMRogue/operator/internal/controller)
- KubeVirt conversion logic: [`operator/internal/converter/kubevirt.go`](/Users/ssahani/tt/VMRogue/operator/internal/converter/kubevirt.go:1)
- Optional event bus integration: [`operator/internal/eventbus`](/Users/ssahani/tt/VMRogue/operator/internal/eventbus)
- Metrics: [`operator/internal/metrics/metrics.go`](/Users/ssahani/tt/VMRogue/operator/internal/metrics/metrics.go:1)

The main reconciler flow for VM CRs is in [`operator/internal/controller/vmroguevm_controller.go`](/Users/ssahani/tt/VMRogue/operator/internal/controller/vmroguevm_controller.go:1):

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

- User-facing overview: [README.md](/Users/ssahani/tt/VMRogue/README.md:1)
- Command examples: [QUICK_REFERENCE.md](/Users/ssahani/tt/VMRogue/QUICK_REFERENCE.md:1)
- Feature-specific docs: [`docs/`](/Users/ssahani/tt/VMRogue/docs)
- Contribution workflow: [CONTRIBUTING.md](/Users/ssahani/tt/VMRogue/CONTRIBUTING.md:1)
