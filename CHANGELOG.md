# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

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
