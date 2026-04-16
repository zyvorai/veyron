# Contributing to vmrogue

Thank you for your interest in contributing to vmrogue! This document provides guidelines and instructions for contributing.

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
   git clone git@github.com:ssahani/VMRogue.git
   cd vmrogue
   ```

2. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make your changes**
   - Write clear, documented code
   - Follow the existing code style
   - Add tests for new functionality
   - Update documentation as needed

4. **Run tests**
   ```bash
   make ci    # runs fmt-check, clippy, and tests
   # or manually:
   RUST_MIN_STACK=8388608 cargo test
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   ```

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

API handlers should return `VMRogueError` which implements `IntoResponse`. This ensures all error paths produce proper HTTP status codes and consistent JSON error bodies. Do not return `(StatusCode, String)` tuples or `200 OK` with empty data for errors.

```rust
use crate::api::handlers::VMRogueError;

pub async fn my_handler(
    State(state): State<AppState>,
) -> Result<Json<MyResponse>, VMRogueError> {
    let data = do_something().map_err(|e| VMRogueError::KubeError(e.to_string()))?;
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
- `ADVANCED_FEATURES.md` - Advanced features guide
- `DEVELOPMENT.md` - Development roadmap

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

# Clone repository
git clone git@github.com:ssahani/VMRogue.git
cd vmrogue

# Build and test
cargo build
cargo test

# Install for local testing
cargo install --path .

# Run with verbose logging
RUST_LOG=debug vmrogue --verbose list
```

### Feature Flags

VMRogue uses the `experimental` Cargo feature flag to gate scaffolding-only modules (operator CRDs, experimental handlers, etc.). These modules are excluded from default builds to reduce compile time and attack surface.

When working on experimental modules:

```bash
# Build with experimental modules enabled
cargo build --features experimental

# Test with experimental modules
RUST_MIN_STACK=8388608 cargo test --features experimental

# Check clippy on experimental code
cargo clippy --features experimental --all-targets -- -D warnings
```

If you are adding a new module that is not yet production-ready, gate it behind this flag in `Cargo.toml` and `src/lib.rs`.

### Useful Commands

A Makefile is provided for common tasks:

```bash
make help       # Show all available commands
make build      # Build debug binary
make release    # Build optimized release binary
make test       # Run all tests
make clippy     # Run clippy linter
make fmt        # Format code
make lint       # Run fmt-check + clippy
make ci         # Full CI pipeline locally
make clean      # Clean build artifacts
make install    # Install to ~/.cargo/bin
make tui        # Launch interactive TUI
```

Or use cargo directly:

```bash
cargo check                          # Check for errors
cargo doc --open                     # Generate and open docs
cargo run --example library_usage    # Run example
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

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create release commit
4. Tag release: `git tag -a v0.2.0 -m "Release v0.2.0"`
5. Push: `git push origin main --tags`
6. Create GitHub release
7. Publish to crates.io: `cargo publish`

## 🤝 Community

- Be respectful and inclusive
- Help others learn and grow
- Provide constructive feedback
- Celebrate contributions

## 📜 License

By contributing to vmrogue, you agree that your contributions will be licensed under the same MIT OR Apache-2.0 license that covers the project.

## ❓ Questions?

Feel free to:
- Open an issue for questions
- Start a discussion on GitHub Discussions
- Reach out to maintainers

---

Thank you for contributing to vmrogue! 🚀
