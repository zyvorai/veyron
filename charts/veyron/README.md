# Veyron Helm chart

> **Image caveat (v0.4.0).** `ghcr.io/zyvorai/veyron:0.4.0` is not published. `image.tag` defaults to `latest`, which is
> the older 2026-10-04 image (before the Kairon rewrite), whatever the chart's `appVersion` says. Build your own image
> (`scripts/deploy-remote.sh` builds it on a node) and set `--set image.repository=... --set image.tag=...`.
> Without overrides the chart installs the lab credentials `admin` / `Admin@321` and API key `Admin@321`: always set
> `auth.apiKey` and `auth.adminPassword`.

Install the Veyron API (HTTPS dashboard + `/api/v1` and `/api/v1/veyron` aliases):

```bash
helm install veyron ./charts/veyron \
  -n veyron-system --create-namespace \
  --set auth.apiKey='your-key-here'
```

## Resource names

| Kind | Name |
|------|------|
| Namespace | `veyron-system` (release namespace) |
| Deployment / Service | `veyron-api` |
| ServiceAccount | `veyron` |
| ClusterRole | `veyron` |
| API key Secret | `veyron-api-key` |

## Environment

The chart sets both `VEYRON_API_KEY` and `VEYRON_API_KEY` from the same Secret for backward compatibility.

## Operator (legacy, KubeVirt only)

The operator chart manages KubeVirt `VirtualMachine`s through the `veyron.io` CRDs. It is not used with the Kairon engine (the default) and
is only relevant with `VEYRON_VM_BACKEND=kubevirt`.

```bash
helm install veyron-operator ./charts/veyron-operator -n veyron-system
```

CRDs remain under **`veyron.io`**; only Kubernetes object names use the Veyron brand.

## Legacy

`charts/veyron` and `charts/veyron-operator` are deprecated aliases of the same templates.
