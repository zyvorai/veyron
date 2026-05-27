# Template Catalog (VMTemplate / VMProfile CRDs)

Cluster-scoped **VMTemplate** and **VMProfile** CRDs provide a shared catalog for the CLI, API, dashboard, and operator.

## Architecture

| Layer | Source |
|-------|--------|
| CLI `vmrogue deploy` | Rust templates + profiles (offline) |
| Operator reconcile | VMTemplate + VMProfile CRDs (cluster) or embedded fallback |
| GitOps | Explicit `override` on blueprint / VMRogueVM specs |

Resolution order in the operator (`operator/internal/catalog/resolver.go`):

1. VMTemplate defaults
2. VMProfile CPU/memory/disk
3. Blueprint per-VM cpu/memory/diskSize
4. Explicit `override` (wins)

## Bootstrap

```bash
./scripts/generate-catalog-crds.sh   # or: make catalog-generate
kubectl apply -f operator/config/crd/bases/vmrogue.io_vmtemplates.yaml
kubectl apply -f operator/config/crd/bases/vmrogue.io_vmprofiles.yaml
vmrogue catalog sync                 # create or replace catalog CRDs (idempotent)
```

CI can verify checked-in YAML matches Rust export:

```bash
make catalog-check
```

## CLI inspection

```bash
vmrogue templates --source cluster          # list cluster VMTemplate CRDs
vmrogue catalog list                        # templates + profiles from cluster
vmrogue catalog list --templates            # templates only
vmrogue catalog list --profiles             # profiles only
```

## Windows secrets

Use Secret refs instead of inline passwords:

```yaml
cloudInit:
  delivery: configdrive
  userDataSecretRef:
    name: win-sysprep
    key: userdata
windows:
  domainJoinSecretRef:
    name: ad-join
    key: join.json
```

See [examples/windows-domain-join/](../examples/windows-domain-join/).

## Drift

Operator compares resolved spec hash to KubeVirt VM annotations and sets `status.driftDetected`. API: `GET /api/v1/vms/:ns/:name/drift` (VMRogueVM CR name).
