.PHONY: build release check test clippy fmt lint clean install help docker deploy

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build debug binary
	cargo build

release: ## Build optimized release binary
	cargo build --release

check: ## Run cargo check
	cargo check

test: ## Run all tests
	RUST_MIN_STACK=8388608 cargo test

clippy: ## Run clippy linter
	cargo clippy --all-targets -- -D warnings

fmt: ## Format code
	cargo fmt

fmt-check: ## Check formatting without modifying
	cargo fmt --all -- --check

lint: fmt-check clippy ## Run all lints (format + clippy)

clean: ## Clean build artifacts
	cargo clean

install: ## Install to ~/.cargo/bin
	cargo install --path .

ci: fmt-check clippy test ## Run full CI pipeline locally

docker: ## Build Docker image
	./scripts/build-deploy.sh docker

deploy: ## Deploy to Kubernetes
	./scripts/build-deploy.sh deploy

pipeline: ## Full pipeline: test -> build -> docker -> push -> deploy
	./scripts/build-deploy.sh all

config-show: build ## Show current configuration
	./target/debug/vmrogue config-show

config-init: build ## Create default config file
	./target/debug/vmrogue config-init

tui: build ## Launch TUI
	./target/debug/vmrogue tui --interactive

templates: build ## List available templates
	./target/debug/vmrogue templates

profiles: build ## List resource profiles
	./target/debug/vmrogue profiles --details
