.PHONY: build release check test clippy fmt lint clean install help docker deploy dashboard-next

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build debug binary
	cargo build

dashboard-next: ## Build and embed React operator UI (dashboard-next)
	./scripts/build-dashboard-next.sh

release: dashboard-next ## Build optimized release binary
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

helm-monitoring-validate: ## Validate vmrogue-monitoring Helm chart (template)
	helm dependency build charts/vmrogue-monitoring
	helm template vmrogue-monitoring-ci charts/vmrogue-monitoring -n monitoring >/dev/null
	@echo "helm template vmrogue-monitoring: OK"

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

# ============================================================================
# Operator targets (Go)
# ============================================================================

operator-build: ## Build Go operator binary
	cd operator && go build -o bin/manager main.go

operator-test: ## Run operator tests
	cd operator && go test ./... -v

operator-fmt: ## Format operator code
	cd operator && go fmt ./...

operator-vet: ## Run go vet on operator
	cd operator && go vet ./...

operator-docker: ## Build operator Docker image
	docker build -t ghcr.io/ssahani/vmrogue-operator:latest -f operator/Dockerfile operator/

operator-install: ## Install CRDs into cluster
	kubectl apply -f operator/config/crd/bases/

operator-uninstall: ## Remove CRDs from cluster
	kubectl delete -f operator/config/crd/bases/

operator-deploy: operator-install ## Deploy operator + CRDs to cluster
	kubectl apply -f operator/config/rbac/
	kubectl apply -f operator/config/manager/

operator-undeploy: ## Remove operator from cluster
	kubectl delete -f operator/config/manager/ || true
	kubectl delete -f operator/config/rbac/ || true
	kubectl delete -f operator/config/crd/bases/ || true

operator-samples: ## Deploy sample CRs
	kubectl apply -f operator/config/samples/

nats-deploy: ## Deploy NATS event bus
	kubectl apply -f operator/config/nats/

ci-all: ci operator-vet operator-test ## Full CI for Rust + Go
