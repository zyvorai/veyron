# VMRogue Quick Reference

This file is aligned to the current CLI in [`src/cli/mod.rs`](/Users/ssahani/tt/VMRogue/src/cli/mod.rs:1). Use `vmrogue commands` and `vmrogue <command> --help` for the full surface.

## Core VM Lifecycle

```bash
vmrogue doctor
vmrogue create my-vm --template ubuntu-22.04 --cpus 4 --memory 8Gi
vmrogue create my-vm --from-file examples/basic-vm.yaml --dry-run
vmrogue list
vmrogue ls -A
vmrogue get my-vm -o yaml
vmrogue status my-vm --watch
vmrogue start my-vm
vmrogue stop my-vm
vmrogue restart my-vm
vmrogue pause my-vm
vmrogue unpause my-vm
vmrogue resize my-vm --cpus 8 --memory 16Gi
vmrogue clone source-vm cloned-vm --start
vmrogue delete my-vm --yes
```

## Templates, Profiles, and Blueprints

```bash
vmrogue templates
vmrogue template ubuntu-22.04

vmrogue profiles
vmrogue profiles --details
vmrogue profile database
vmrogue recommend database --alternatives

vmrogue blueprints
vmrogue blueprints --tag web --details
vmrogue blueprint lamp
vmrogue deploy lamp --prefix prod --dry-run
vmrogue deploy lamp --prefix prod --start
```

## Snapshots and Backup

```bash
vmrogue snapshot-create my-vm --name backup-20260417
vmrogue snapshot-list
vmrogue snapshot-list my-vm
vmrogue snapshot-get backup-20260417
vmrogue snapshot-restore backup-20260417 --target restored-vm --start
vmrogue snapshot-delete backup-20260417 --yes

vmrogue backup-create my-vm --name nightly-001
vmrogue backup-list
vmrogue backup-get nightly-001
vmrogue backup-verify nightly-001 --verification-type full
vmrogue backup-restore nightly-001 --target restored-vm --start
vmrogue backup-delete nightly-001 --yes
```

## Migration, Disk, and Network

```bash
vmrogue migrate my-vm --plan
vmrogue migration-status my-vm --watch
vmrogue migration-list -A
vmrogue ha-config my-vm --enable --priority critical --eviction-strategy live-migrate
vmrogue evacuate-node worker-01 --plan

vmrogue disk-expand my-vm rootdisk 100Gi --plan
vmrogue disk-health my-vm --detailed
vmrogue disk-script --filesystem lvm --device /dev/vda
vmrogue disk-usage

vmrogue network-list my-vm
vmrogue network-get my-vm eth0
vmrogue network-bandwidth my-vm --watch
vmrogue network-traffic my-vm --period 1h
vmrogue network-policies -A
```

## Security, Cost, and Observability

```bash
vmrogue health my-vm --detailed
vmrogue security-scan my-vm --scan-type standard
vmrogue security-assess my-vm
vmrogue security-harden my-vm --profile cis --verify-only
vmrogue compliance-check my-vm --framework cis
vmrogue audit-list my-vm

vmrogue cost-analyze --period 30d
vmrogue cost-summary --group-by namespace
vmrogue cost-report --report-type monthly --format json
vmrogue budget-list
vmrogue budget-create team-a --amount 500 --period monthly --scope namespace:production

vmrogue logs-query --level error --search timeout
vmrogue metrics-collect my-vm
vmrogue alerts-list
vmrogue insights-generate my-vm
vmrogue recommendations --category cost --with-savings
vmrogue trends-analyze cpu_usage --window 24
```

## API, Dashboard, and TUI

```bash
export VMROGUE_API_KEYS="admin:supersecret,write:devkey,readonly:viewkey"

vmrogue api-serve --host 0.0.0.0 --port 5151
vmrogue api-status
vmrogue api-routes
vmrogue api-spec --format yaml --output openapi.yaml

vmrogue api-key-list
vmrogue api-key-create dev --permissions read,write
vmrogue webhook-list

vmrogue tui
vmrogue tui --basic
vmrogue tui --no-splash --theme dark
```

Dashboard URL after `api-serve`:

```text
http://localhost:5151/dashboard
```

## GitOps, Import, and Cluster Views

```bash
vmrogue gitops-export --directory gitops/
vmrogue gitops-diff --directory gitops/
vmrogue gitops-status

vmrogue import examples/web-server.kubevirt.yaml --dry-run
vmrogue import examples/web-server.kubevirt.yaml --start

vmrogue clusters-list
vmrogue clusters-discover
vmrogue events --limit 25
vmrogue nodes
vmrogue pods
vmrogue capacity --detailed
vmrogue placement my-vm --strategy leastloaded
vmrogue list-json
```

## Developer Helpers

```bash
vmrogue commands
vmrogue info --detailed
vmrogue config-show
vmrogue config-init
vmrogue completions zsh
vmrogue generate my-vm --template fedora-40 --kubevirt
vmrogue validate examples/basic-vm.yaml
vmrogue wizard my-vm
vmrogue batch some-batch.yaml --dry-run
```

## Notes

- `vmrogue list` has the visible alias `vmrogue ls`.
- `vmrogue delete` has the visible alias `vmrogue rm`.
- `vmrogue generate` has the visible alias `vmrogue gen`.
- `vmrogue status` has the visible alias `vmrogue stat`.
- `vmrogue wizard` has the visible alias `vmrogue wiz`.
