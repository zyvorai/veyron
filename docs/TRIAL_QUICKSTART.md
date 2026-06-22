# Veyron Trial Quickstart

Veyron includes a **30-day free trial** — no key or sign-up required to install.
After 30 days the pod will stop and show an upgrade message. Contact sales@zyvor.dev
to obtain a licence key and continue running.

## Install via Helm

**Prerequisites:** Kubernetes cluster with KubeVirt installed, Helm 3.8+.

```bash
# Install — no licence key needed
helm install veyron oci://ghcr.io/hypersdk/charts/veyron \
  --version 0.3.0 \
  --namespace veyron-system \
  --create-namespace

# Confirm the pod started
kubectl -n veyron-system logs deploy/veyron-api | head -3
# → Veyron trial licence: Trial — valid until 2026-07-22
```

The dashboard is available at `https://<node-ip>:30151` (NodePort, self-signed cert).
Default API key is auto-generated; retrieve it with:

```bash
kubectl -n veyron-system get secret veyron-api-key -o jsonpath='{.data.api-key}' | base64 -d
```

## After the trial — apply a licence key

Once you receive a key from sales@zyvor.dev:

```bash
kubectl create secret generic veyron-license \
  --from-literal=license.key="<your-key>" \
  -n veyron-system

helm upgrade veyron oci://ghcr.io/hypersdk/charts/veyron \
  --version 0.3.0 \
  --reuse-values \
  --set license.existingSecret="veyron-license" \
  -n veyron-system
```

## Trial expiry message

When the trial expires the pod exits with:

```
Veyron 30-day trial has expired (build: 2026-06-22).
To continue, contact sales@zyvor.dev for a licence key.
```

## Upgrade

```bash
helm upgrade veyron oci://ghcr.io/hypersdk/charts/veyron --version <new-version> -n veyron-system
```

## Uninstall

```bash
helm uninstall veyron -n veyron-system
kubectl delete namespace veyron-system
```
