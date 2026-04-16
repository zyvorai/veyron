# VMRogue Operator Makefile

IMG ?= ghcr.io/ssahani/vmrogue-operator:latest

.PHONY: all
all: build

##@ Development

.PHONY: fmt
fmt: ## Run go fmt.
	go fmt ./...

.PHONY: vet
vet: ## Run go vet.
	go vet ./...

.PHONY: test
test: fmt vet ## Run tests.
	go test ./... -v -coverprofile cover.out

.PHONY: lint
lint: ## Run golangci-lint.
	golangci-lint run ./...

##@ Build

.PHONY: build
build: fmt vet ## Build manager binary.
	go build -o bin/manager main.go

.PHONY: run
run: fmt vet ## Run against the configured cluster.
	go run main.go

.PHONY: docker-build
docker-build: ## Build Docker image.
	docker build -t ${IMG} .

.PHONY: docker-push
docker-push: ## Push Docker image.
	docker push ${IMG}

##@ Deployment

.PHONY: install
install: ## Install CRDs into the cluster.
	kubectl apply -f config/crd/bases/

.PHONY: uninstall
uninstall: ## Uninstall CRDs from the cluster.
	kubectl delete -f config/crd/bases/

.PHONY: deploy
deploy: install ## Deploy operator to the cluster.
	kubectl apply -f config/rbac/
	kubectl apply -f config/manager/

.PHONY: undeploy
undeploy: ## Undeploy operator from the cluster.
	kubectl delete -f config/manager/ || true
	kubectl delete -f config/rbac/ || true
	kubectl delete -f config/crd/bases/ || true

.PHONY: deploy-nats
deploy-nats: ## Deploy NATS event bus.
	kubectl apply -f config/nats/

.PHONY: deploy-samples
deploy-samples: ## Deploy sample CRs.
	kubectl apply -f config/samples/

##@ Helpers

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
