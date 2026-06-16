# Veyron Helm chart

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

The chart sets both `VEYRON_API_KEY` and `VMROGUE_API_KEY` from the same Secret for backward compatibility.

## Operator

```bash
helm install veyron-operator ./charts/veyron-operator -n veyron-system
```

CRDs remain under **`vmrogue.io`**; only Kubernetes object names use the Veyron brand.

## Legacy

`charts/vmrogue` and `charts/vmrogue-operator` are deprecated aliases of the same templates.
