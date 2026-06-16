.PHONY: build release check test clippy fmt lint clean install help docker deploy catalog-generate catalog-check

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

test-vm-e2e-remote: ## Run daily VM ops E2E against remote API (HOST=... PORT=30151)
	@test -n "$(HOST)" || (echo "Usage: make test-vm-e2e-remote HOST=<ip> [PORT=30151]" >&2; exit 1)
	./scripts/test-vm-daily-ops-remote.sh "$(HOST)" "$(if $(PORT),$(PORT),30151)"

helm-monitoring-validate: ## Validate vmrogue-monitoring Helm chart (template)
	helm dependency build charts/vmrogue-monitoring
	helm template vmrogue-monitoring-ci charts/vmrogue-monitoring -n monitoring >/dev/null
	@echo "helm template vmrogue-monitoring: OK"

helm-veyron-validate: ## Validate veyron + veyron-operator Helm charts (template)
	helm template veyron-ci charts/veyron -n veyron-system >/dev/null
	helm template veyron-operator-ci charts/veyron-operator -n veyron-system >/dev/null
	@echo "helm template veyron + veyron-operator: OK"

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

catalog-generate: build ## Export VMTemplate/VMProfile YAML from Rust templates
	./scripts/generate-catalog-crds.sh

catalog-check: catalog-generate ## Regenerate catalog; fail if operator/config/catalog drifts
	@if git diff --quiet operator/config/catalog; then \
	  echo "catalog-check: operator/config/catalog matches export"; \
	else \
	  echo "catalog-check: operator/config/catalog is out of date (run make catalog-generate and commit)" >&2; \
	  git diff --stat operator/config/catalog; \
	  exit 1; \
	fi

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
