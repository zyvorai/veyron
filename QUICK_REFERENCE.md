# Veyron Quick Reference

This file is aligned to the current CLI in `src/cli/mod.rs`. Use `veyron commands` and `veyron <command> --help` for the full surface.

## Remote Deployment Helpers

```bash
# Full remote deployment (API + operator); HOST USER = SSH target (node + login)
./scripts/deploy-remote.sh HOST USER
./scripts/deploy-all-remote.sh HOST USER

# Quick mode (skip image build/import)
./scripts/deploy-all-remote.sh HOST USER --quick

# Deploy tuning (defaults: layer cache on, parallel API+operator builds)
VEYRON_DEPLOY_NO_CACHE=1 ./scripts/deploy-remote.sh HOST USER   # clean rebuild
VEYRON_DEPLOY_DIAGNOSTICS=1 ./scripts/deploy-remote.sh HOST USER  # full preflight

# API-only remote deploy
./scripts/deploy-k8s-remote.sh HOST USER

# Post-deploy smoke test (NodePort health, templates, VM list)
VEYRON_API_KEY='Admin@321' ./scripts/verify-veyron-remote.sh HOST [30151]

# VM daily ops E2E (create, lifecycle, SSH/RDP expose, snapshots; ~10–15 min)
VEYRON_API_KEY='Admin@321' ./scripts/test-vm-daily-ops-remote.sh HOST [30151]

# Client tarball (linux/amd64 binary) — see docs/PACKAGE_BINARY_REMOTE.md
./scripts/package-binary-remote.sh HOST USER --fetch
./scripts/package-binary-remote.sh HOST USER --reuse-image --fetch   # skip rebuild if image exists
```

## In-cluster operator (CRDs / GitOps)

`VeyronVM` and related **`veyron.io/v1alpha1`** objects are reconciled by the **Go operator** in `operator/` (controller-runtime): it materializes **KubeVirt `VirtualMachine`** objects from `VeyronVM` specs, manages finalizers/status, and runs sibling controllers for blueprints, policies, insights, and actions. Rust `src/operator_crds/` mirrors CRD shapes for CLI/API only. See **`DEVELOPMENT.md`** (Operator Architecture) and **`charts/veyron-operator`**. Export manifests with `veyron gitops-export`.

## Core VM Lifecycle

```bash
veyron doctor
veyron create my-vm --template ubuntu-22.04 --cpus 4 --memory 8Gi
veyron create my-vm --from-file examples/basic-vm.yaml --dry-run
veyron list
veyron ls -A
veyron get my-vm -o yaml
veyron status my-vm --watch
veyron start my-vm
veyron stop my-vm
veyron restart my-vm
veyron pause my-vm
veyron unpause my-vm
veyron resize my-vm --cpus 8 --memory 16Gi
veyron clone source-vm cloned-vm --start
veyron delete my-vm --yes
```

## Templates, Profiles, and Blueprints

```bash
veyron templates
veyron template ubuntu-22.04

veyron profiles
veyron profiles --details
veyron profile database
veyron recommend database --alternatives

veyron blueprints
veyron blueprints --tag web --details
veyron blueprint lamp
veyron deploy lamp --prefix prod --dry-run
veyron deploy lamp --prefix prod --start
```

## Snapshots and Backup

```bash
veyron snapshot-create my-vm --name backup-20260417
veyron snapshot-list
veyron snapshot-list my-vm
veyron snapshot-get backup-20260417
veyron snapshot-restore backup-20260417 --target restored-vm --start
veyron snapshot-delete backup-20260417 --yes

veyron backup-create my-vm --name nightly-001
veyron backup-list
veyron backup-get nightly-001
veyron backup-verify nightly-001 --verification-type full
veyron backup-restore nightly-001 --target restored-vm --start
veyron backup-delete nightly-001 --yes
```

## Migration, Disk, and Network

```bash
veyron migrate my-vm --plan
veyron migration-status my-vm --watch
veyron migration-list -A
veyron ha-config my-vm --enable --priority critical --eviction-strategy live-migrate
veyron evacuate-node worker-01 --plan

veyron disk-expand my-vm rootdisk 100Gi --plan
veyron disk-health my-vm --detailed
veyron disk-script --filesystem lvm --device /dev/vda
veyron disk-usage

# PVC resize via API (also available in dashboard storage UI)
# PATCH /api/v1/storage/pvcs/:ns/:name   body: {"new_size":"50Gi"}

veyron network-list my-vm
veyron network-get my-vm eth0
veyron network-bandwidth my-vm --watch
veyron network-traffic my-vm --period 1h
veyron network-policies -A
```

## Security, Cost, and Observability

```bash
veyron health my-vm --detailed
veyron security-scan my-vm --scan-type standard
veyron security-assess my-vm
veyron security-harden my-vm --profile cis --verify-only
veyron compliance-check my-vm --framework cis
veyron audit-list my-vm

veyron cost-analyze --period 30d
veyron cost-summary --group-by namespace
veyron cost-report --report-type monthly --format json
veyron budget-list
veyron budget-create team-a --amount 500 --period monthly --scope namespace:production

veyron logs-query --level error --search timeout
veyron metrics-collect my-vm
veyron alerts-list
veyron insights-generate my-vm
veyron recommendations --category cost --with-savings
veyron trends-analyze cpu_usage --window 24
```

## API, Dashboard, and TUI

```bash
export VEYRON_API_KEYS="admin:supersecret,write:devkey,readonly:viewkey"

veyron api-serve --host 0.0.0.0 --port 5151
veyron api-status
veyron api-routes
veyron api-spec --format yaml --output openapi.yaml

veyron api-key-list
veyron api-key-create dev --permissions read,write
veyron webhook-list

veyron tui
veyron tui --basic
veyron tui --no-splash --theme dark
```

Dashboard URL after `api-serve`:

```text
http://localhost:5151/dashboard
```

## GitOps, Import, and Cluster Views

```bash
veyron gitops-export --directory gitops/
veyron gitops-diff --directory gitops/
veyron gitops-status

veyron import examples/web-server.kubevirt.yaml --dry-run
veyron import examples/web-server.kubevirt.yaml --start

veyron clusters-list
veyron clusters-discover
veyron events --limit 25
veyron nodes
veyron pods
veyron capacity --detailed
veyron placement my-vm --strategy leastloaded
veyron list-json
```

## Developer Helpers

```bash
veyron commands
veyron info --detailed
veyron config-show
veyron config-init
veyron completions zsh
veyron generate my-vm --template fedora-40 --kubevirt
veyron validate examples/basic-vm.yaml
veyron wizard my-vm
veyron batch some-batch.yaml --dry-run
```

## Notes

- `veyron list` has the visible alias `veyron ls`.
- `veyron delete` has the visible alias `veyron rm`.
- `veyron generate` has the visible alias `veyron gen`.
- `veyron status` has the visible alias `veyron stat`.
- `veyron wizard` has the visible alias `veyron wiz`.
