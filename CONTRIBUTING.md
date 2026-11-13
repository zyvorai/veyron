# Contributing to veyron

Thank you for your interest in contributing to veyron! This document provides guidelines and instructions for contributing.

## 🎯 How to Contribute

### Reporting Bugs

If you find a bug, please create an issue with:
- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Your environment (OS, Rust version, etc.)
- Relevant logs or error messages

### Suggesting Features

We welcome feature suggestions! Please create an issue with:
- Clear description of the feature
- Use case and motivation
- Proposed implementation approach (optional)
- Any relevant examples

### Pull Requests

1. **Fork the repository**
   ```bash
   git clone git@github.com:zyvorai/veyron.git
   cd veyron
   ```

2. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make your changes**
   - Write clear, documented code
   - Follow the existing code style
   - Add tests for new functionality
   - Update documentation as needed — for user-facing behavior, add new guides under `docs/` and link them from [`docs/README.md`](docs/README.md)

4. **Run tests**
   ```bash
   (cd frontend && npm ci && npm run build)   # the binary embeds frontend/dist
   make ci    # runs fmt-check, clippy, script lint, and tests
   # or manually:
   RUST_MIN_STACK=8388608 cargo test
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   ```
   The build needs a checkout of `zyvorai/zyvor-guestkit` at `./guestkit` (the commit pinned as `GUESTKIT_REF` in
   `.github/workflows/ci.yml`). CI also runs the console's `npm run lint` and `npm test`, the Go operator's `go vet` and
   `go test` (`operator/`), `cargo audit`, and a Kind + KubeVirt integration job on `main`.

5. **Commit your changes**
   ```bash
   git add .
   git commit -m "Add feature: description"
   ```

6. **Push and create PR**
   ```bash
   git push origin feature/your-feature-name
   ```
   Then create a pull request on GitHub.

## 📋 Code Style

### Rust Code

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting: `cargo fmt`
- Pass `clippy` without warnings: `cargo clippy`
- Write doc comments for public APIs
- Keep functions focused and small
- Use descriptive variable names

### Example

```rust
/// Creates a new VM from the specified configuration.
///
/// # Arguments
///
/// * `config` - The VM configuration to use
///
/// # Returns
///
/// Returns `Ok(VirtualMachine)` on success, or an error if creation fails.
///
/// # Errors
///
/// This function will return an error if:
/// - The VM already exists
/// - The configuration is invalid
/// - The Kubernetes API call fails
pub async fn create_vm(&self, config: &VMConfig) -> Result<VirtualMachine> {
    // Implementation
}
```

### Error Handling in API Handlers

API handlers should return `VeyronError` which implements `IntoResponse`. This ensures all error paths produce proper HTTP status codes and consistent JSON error bodies. Do not return `(StatusCode, String)` tuples or `200 OK` with empty data for errors.

```rust
use crate::api::handlers::VeyronError;

pub async fn my_handler(
    State(state): State<AppState>,
) -> Result<Json<MyResponse>, VeyronError> {
    let data = do_something().map_err(|e| VeyronError::KubeError(e.to_string()))?;
    Ok(Json(data))
}
```

### 501 Pattern for Stub Handlers

When adding a new endpoint that is not yet implemented, use the `not_implemented("feature_name")` helper function. This returns `501 Not Implemented` with a clear JSON message, signaling to clients that the feature exists in the API surface but is not yet functional.

```rust
pub async fn my_future_handler() -> impl IntoResponse {
    not_implemented("my_future_feature")
}
```

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(cli): add status command with watch mode

Add new status command that displays detailed VM information
including resources, volumes, networks, and conditions.
Includes watch mode for real-time monitoring.

Closes #42
```

```
fix(validator): correct memory size validation regex

The regex was incorrectly rejecting valid memory sizes
ending in 'M' or 'G'. Updated to accept both formats.

Fixes #38
```

## 🧪 Testing

### Running Tests

```bash
# Run all tests (recommended)
make test

# Or manually (RUST_MIN_STACK needed for large CLI enum)
RUST_MIN_STACK=8388608 cargo test

# Run specific test
RUST_MIN_STACK=8388608 cargo test test_name

# Run with output
RUST_MIN_STACK=8388608 cargo test -- --nocapture

# Run integration tests only
RUST_MIN_STACK=8388608 cargo test --test integration_tests
```

### Writing Tests

- Add unit tests in the same file as the code
- Add integration tests in `tests/`
- Test edge cases and error conditions
- Use descriptive test names

```rust
#[test]
fn test_validate_vm_config_with_invalid_name() {
    let config = VMConfigBuilder::new("Invalid-Name")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .build();

    assert!(validate_vm_config(&config).is_err());
}
```

### API Integration Tests

API handler tests use the tower/axum test pattern: build the router with `app()`, then send requests via `tower::ServiceExt::oneshot`. This avoids spinning up a real HTTP server.

```rust
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn test_handler_returns_501() {
    let app = app(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/some-stub")
                .header("X-API-Key", "test-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
}
```

### CI: Kind + KubeVirt Integration Tests

Integration tests that require a real Kubernetes cluster run automatically on pushes to `main`. The CI job provisions a Kind cluster with KubeVirt installed, then runs the full test suite. These tests are not triggered on pull request branches to avoid long feedback loops during development.

## 📚 Documentation

### Code Documentation

- Add doc comments to all public items
- Include examples in doc comments
- Document errors and panics
- Keep examples up to date

### User Documentation

Update relevant documentation files:
- `README.md` - Overview and quick start
- `docs/` - Guides (getting started, architecture, API, deploy, SSO, GPU, Windows, SOC, integrations)
- `CHANGELOG.md` - User-visible changes

## 🔍 Code Review Process

1. **Automated Checks**
   - CI must pass (tests, format, clippy)
   - No conflicts with main branch

2. **Manual Review**
   - Code quality and style
   - Test coverage
   - Documentation completeness
   - Breaking changes assessment

3. **Approval**
   - At least one maintainer approval required
   - Address all review comments

4. **Merge**
   - Squash and merge (typically)
   - Maintain clean commit history

## 🌟 Good First Issues

Looking for where to start? Check out issues labeled:
- `good-first-issue` - Great for newcomers
- `help-wanted` - Community help needed
- `documentation` - Documentation improvements

## 💡 Development Tips

### Setting Up Development Environment

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone repository (plus guestkit and a console build, see "Run tests" above)
git clone git@github.com:zyvorai/veyron.git
cd veyron

# Build and test
cargo build
cargo test

# Install for local testing
cargo install --path .

# Run with verbose logging
RUST_LOG=debug veyron --verbose list
```

### Feature Flags

`Cargo.toml` defines these features (there is no `experimental` feature):

- `web` (default): the HTTPS API, console and WebSocket proxies. Everything under `src/api/` is gated on it.
- `kairon` (default): the Kairon `Machine` backend (`src/kairon/`, `src/api/vm_backend.rs`).
- `trial`: the time-limited evaluation build for the client tarball (`src/trial.rs`, `src/bin/trial-tool.rs`). Never enabled
  for in-cluster images.

```bash
cargo build --no-default-features              # CLI only, no web server
cargo clippy --features kairon --all-targets -- -D warnings   # what CI also runs
```

Code that is not ready for production should answer `501` with a reason (see above) rather than sit behind a flag.

### Useful Commands

A Makefile is provided for common tasks:

```bash
make help       # Show all available commands
make build      # Build debug binary
make release    # Build optimized release binary
make test       # Run all tests
make clippy     # Run clippy linter
make fmt        # Format code
make lint       # Run fmt-check + clippy + script lint
make ci         # fmt-check, clippy, script lint and tests, as CI does
make helm-veyron-validate   # helm template for charts/veyron and charts/veyron-operator
make clean      # Clean build artifacts
make install    # Install to ~/.cargo/bin
```

Or use cargo directly:

```bash
cargo check                          # Check for errors
cargo doc --open                     # Generate and open docs
```

### Debugging

```rust
// Use env_logger for logging
use log::{debug, info, warn, error};

debug!("Debug message: {:?}", value);
info!("Info message");
warn!("Warning message");
error!("Error message");
```

Run with logging:
```bash
RUST_LOG=debug cargo run -- command
```

## 📦 Release Process

(For maintainers)

1. Bump `version` in `Cargo.toml` and `Cargo.lock`, `charts/veyron/Chart.yaml` (`version` and `appVersion`), and move the
   `CHANGELOG.md` entries from `[Unreleased]` under the new version. Merge that through a pull request.
2. Tag the merge commit and push the tag: `git tag -a v0.4.2 -m "Veyron 0.4.2" && git push origin v0.4.2`. Pushing a `v*`
   tag starts `.github/workflows/release.yml`.
3. The workflow builds the console, runs lint and tests, then runs two jobs in parallel after the tests:
   `build` produces the four binaries (`veyron-{linux,darwin}-{amd64,arm64}`), and `docker` builds and pushes
   `ghcr.io/zyvorai/veyron:<version>` and `:latest` with an SBOM and provenance, then signs the digest with keyless cosign.
   The `release` job needs only `build` (since PR #59): it writes `SHA256SUMS`, an SPDX SBOM and a cosign bundle for
   `SHA256SUMS`, and creates the GitHub Release with the binaries and those files. The macOS binaries are not signed or notarized.
4. **If the image push fails** (`permission_denied: write_package`), the GitHub Release is still created, because `release`
   no longer waits for `docker`. The `veyron` GHCR package must grant `zyvorai/veyron` the Write role (package settings,
   **Manage Actions access**); then re-run the failed `docker` job ([docs/deploy.md](docs/deploy.md#container-image)).
   Re-running the whole workflow also re-runs `release`, which re-uploads the release assets (this is how v0.4.1 ended up
   with the cosign bundle and SBOM after a hand-published first release). The GHCR package is private, so pulling the image
   needs `docker login ghcr.io`.
   If the `release` job itself fails, the manual fallback is (not re-run when this section was written):

   ```bash
   gh run download <run-id> -R zyvorai/veyron -p 'veyron-*' -D dl        # one folder per artifact
   mkdir rel && find dl -type f -name 'veyron-*' -exec mv {} rel/ \;      # flatten
   (cd rel && sha256sum veyron-* > SHA256SUMS)
   gh release create v0.4.2 rel/* -R zyvorai/veyron --title v0.4.2 --notes-file notes.md
   ```

   State in the notes what is missing and check the checksums against the release page's asset digests
   (`gh release view v0.4.2 --json assets`). Hand-written release notes are not updated by a re-run, so check them afterwards.
5. Crates.io is not published (the crate depends on a local `guestkit` path), so there is no `cargo publish` step.

## 🤝 Community

- Be respectful and inclusive
- Help others learn and grow
- Provide constructive feedback
- Celebrate contributions

## 📜 License

Contributions are accepted under the [Zyvor Production License v1.0](LICENSE). By submitting a change you confirm you have the right to contribute it, and you agree Zyvor AI Labs may use, modify, distribute and commercially license it (section 17 of the license).

New source files start with the SPDX header used across the tree:

```rust
// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
```

Use `#` for shell, Python and YAML, and `<!-- … -->` for Markdown and HTML.

## ❓ Questions?

Feel free to:
- Open an issue for questions
- Start a discussion on GitHub Discussions
- Reach out to maintainers

---

Thank you for contributing to veyron! 🚀
