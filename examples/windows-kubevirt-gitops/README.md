# Windows golden image → CDI → KubeVirt (GitOps-friendly)

This example shows:

1. **CDI `DataVolume`** importing a QCOW2 over HTTP(S) (`kustomize/example/datavolume.yaml`).
2. A **`VirtualMachine`** using that disk plus **`cloudInitConfigDrive`** (`kustomize/example/vm.yaml`).

It does **not** run Packer (that runs on a CI worker or image factory). See the repo guide:

`docs/windows.md`

## Quick apply

```bash
kubectl create namespace kubevirt-vms --dry-run=client -o yaml | kubectl apply -f -
# Edit kustomize/example/datavolume.yaml — set URL, storage class, size
kubectl kustomize kustomize/example/   # dry-run: validate YAML
kubectl apply -k kustomize/example/
```

Wait until the DataVolume reaches `Succeeded`, then start the VM:

```bash
virtctl start win11-vm -n kubevirt-vms
```

## Patch `veyron generate` output

Windows templates omit `cloud_init`, so `veyron generate … --kubevirt` has **no** `cloudinitdisk` volume. Use:

```bash
veyron generate win11-vm --template windows-11 --kubevirt --memory 8Gi -o vm.yaml
pip install pyyaml
scripts/patch_kubevirt_configdrive.py vm.yaml win11-vm win11-golden-dv \
  --userdata-file ./first-boot.ps1
kubectl apply -f vm.yaml
```

Replace namespace with `-n` / `metadata.namespace` as needed.
