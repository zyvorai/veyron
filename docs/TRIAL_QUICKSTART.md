# Veyron Trial Quickstart

Request a 30-day trial key at https://zyvor.dev/contact?intent=trial&product=veyron
or email sales@zyvor.dev. You will receive a key string like:

```
eyJwIjoidmV5cm9uIiwiaXNzIjoiMjAyNi0wNi0yMiIsImV4cCI6IjIwMjYtMDctMjIiLCJ3aG8iOiJBY21lIENvcnAifQ.SomeSig
```

## Install via Helm

**Prerequisites:** Kubernetes cluster with KubeVirt installed.

```bash
# Add the Zyvor Helm repository
helm repo add zyvor https://ssahani.github.io/veyron
helm repo update

# Install Veyron with your trial key
helm install veyron zyvor/veyron \
  --namespace veyron-system \
  --create-namespace \
  --set license.key="<your-trial-key>"

# Confirm the pod started and the licence was accepted
kubectl -n veyron-system logs deploy/veyron-api | head -3
# → Veyron trial licence: Acme Corp — valid until 2026-07-22
```

The dashboard is available at `https://<node-ip>:30151` (NodePort, self-signed cert).
Default API key is auto-generated; retrieve it with:

```bash
kubectl -n veyron-system get secret veyron-api-key -o jsonpath='{.data.api-key}' | base64 -d
```

## Using an existing Secret (GitOps / ArgoCD)

Create the secret before `helm install`:

```bash
kubectl create secret generic veyron-license \
  --from-literal=license.key="<your-trial-key>" \
  -n veyron-system
```

Then install without embedding the key in Helm values:

```bash
helm install veyron zyvor/veyron \
  --namespace veyron-system \
  --create-namespace \
  --set license.existingSecret="veyron-license"
```

## Trial expiry

When the trial expires the pod will not start. `kubectl logs` on the failed pod shows:

```
Veyron trial licence expired on 2026-07-22.
Contact sales@zyvor.dev to purchase a commercial licence.
```

Contact sales@zyvor.dev to obtain a commercial licence key and update the Secret.

## Upgrade

```bash
helm repo update
helm upgrade veyron zyvor/veyron -n veyron-system
```

## Uninstall

```bash
helm uninstall veyron -n veyron-system
kubectl delete namespace veyron-system
```
