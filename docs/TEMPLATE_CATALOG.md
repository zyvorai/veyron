# Template Catalog (VMTemplate / VMProfile CRDs)

Cluster-scoped **VMTemplate** and **VMProfile** CRDs provide a shared catalog for the CLI, API, dashboard, and operator.

## Architecture

| Layer | Source |
|-------|--------|
| CLI `veyron deploy` | Rust templates + profiles (offline) |
| Operator reconcile | VMTemplate + VMProfile CRDs (cluster) or embedded fallback |
| GitOps | Explicit `override` on blueprint / VeyronVM specs |

Resolution order in the operator (`operator/internal/catalog/resolver.go`):

1. VMTemplate defaults
2. VMProfile CPU/memory/disk **+ GPUs** (`spec.gpus` — the VM's own explicit `gpus` win over the profile's)
3. Blueprint per-VM cpu/memory/diskSize
4. Explicit `override` (wins)

A GPU-granting profile makes `profile: gpu-large` attach e.g. `nvidia.com/gpu`
without the VM spec mentioning GPUs — note a passthrough GPU makes the VM
non-live-migratable (see [GPU_PASSTHROUGH.md](GPU_PASSTHROUGH.md)):

```yaml
apiVersion: veyron.io/v1alpha1
kind: VMProfile
metadata: { name: gpu-large }
spec:
  cores: 8
  memory: 32Gi
  diskSize: 100Gi
  gpus:
    - { name: gpu0, deviceName: nvidia.com/gpu }
```

## Bootstrap

```bash
./scripts/generate-catalog-crds.sh   # or: make catalog-generate
kubectl apply -f operator/config/crd/bases/veyron.io_vmtemplates.yaml
kubectl apply -f operator/config/crd/bases/veyron.io_vmprofiles.yaml
veyron catalog sync                 # create or replace catalog CRDs (idempotent)
```

CI can verify checked-in YAML matches Rust export:

```bash
make catalog-check
```

## CLI inspection

```bash
veyron templates --source cluster          # list cluster VMTemplate CRDs
veyron catalog list                        # templates + profiles from cluster
veyron catalog list --templates            # templates only
veyron catalog list --profiles             # profiles only
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

Operator compares resolved spec hash to KubeVirt VM annotations and sets `status.driftDetected`. API: `GET /api/v1/vms/:ns/:name/drift` (VeyronVM CR name).
