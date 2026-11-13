# Veyron Helm chart

> **Image caveat (v0.4.1).** `ghcr.io/zyvorai/veyron:0.4.1` and `:latest` are published, but the GHCR package is private:
> create a docker-registry Secret and pass it with `--set 'imagePullSecrets[0].name=<secret>'`, or use your own registry
> (`--set image.repository=...`); see [docs/deploy.md](../../docs/deploy.md#container-image). `image.tag` defaults to
> `latest`, which moves with each release; pin it (`--set image.tag=0.4.1`) for reproducible installs.
> Without overrides the chart installs the lab credentials `admin` / `Admin@321` and API key `Admin@321`: always set
> `auth.apiKey` and `auth.adminPassword`.

Installs the Veyron API and console (HTTPS, `/console`, `/api/v1` and the `/api/v1/veyron` alias):

```bash
helm install veyron ./charts/veyron \
  -n veyron-system --create-namespace \
  --set image.repository=registry.example.com/veyron --set image.tag=0.4.1 \
  --set auth.apiKey='your-key-here' --set auth.adminPassword='your-console-password'
```

`make helm-veyron-validate` runs `helm template` on this chart and the operator chart (a local check; CI only templates
the monitoring chart).

## Resource names

| Kind | Name |
|------|------|
| Namespace | `veyron-system` (release namespace) |
| Deployment / Service | `veyron-api` |
| ServiceAccount | `veyron` |
| ClusterRole | `veyron` |
| API key Secret | `veyron-api-key` (`api-key`, `admin-password`, `jwt-secret`, `jwt-issuer`) |
| Image store PVC | `veyron-api-image-store` (kept on uninstall) |

## Values

| Value | Default | Notes |
|---|---|---|
| `image.repository`, `image.tag`, `image.pullPolicy` | `ghcr.io/zyvorai/veyron`, `latest`, `IfNotPresent` | See the caveat above. `imagePullSecrets` for a private registry |
| `replicaCount` | `1` | Console tickets, rate limits and the login limiter live in each pod's memory; use session affinity before raising it |
| `service.type`, `service.port`, `service.nodePort` | `NodePort`, `5151`, `30151` | |
| `api.host`, `api.port` | `0.0.0.0`, `5151` | Passed to `veyron api-serve` |
| `api.namespace` | `default` | Sets `VEYRON_NAMESPACE`: where the `veyron-users` Secret and the SOC and settings ConfigMaps live (not the release namespace) |
| `tls.enabled`, `tls.autoGenerate`, `tls.existingSecret` | `true`, `true`, `""` | Auto-generated certificates are self-signed (init container, new on every pod start). Use `existingSecret` (`tls.crt`, `tls.key`) for a real one |
| `auth.apiKey` | lab default `Admin@321` | Primary key, role `admin` (`VEYRON_API_KEY`). Kept from an existing Secret on upgrade |
| `auth.adminUser`, `auth.adminPassword` | `admin`, `Admin@321` | Console bootstrap account. `auth.bootstrapAdminSync` (`"1"`) resets its password from the Secret on every start |
| `auth.jwtSecret`, `auth.jwtIssuer` | random 64 characters, `veyron` | Session-token signing. The random secret is kept across upgrades |
| `auth.additionalKeys` | `""` | `VEYRON_API_KEYS`: `name:key:role,...` ([docs/api.md](../../docs/api.md#keys)) |
| `auth.secretName` | `veyron-api-key` | |
| `rateLimit.perMinute` | `60` | `--rate-limit`; `0` disables |
| `imageStore.persistence`, `size`, `storageClass`, `existingClaim` | `true`, `50Gi`, `""`, `""` | Kairon image uploads. Without persistence an `emptyDir` loses uploads on restart |
| `networkPolicy.enabled` | `true` | Ingress: the API port. Egress: DNS and the Kubernetes API (443, 6443) only. See below |
| `podDisruptionBudget` | enabled, `maxUnavailable: 1` | |
| `rbac.create`, `rbac.clusterRoleName` | `true`, `veyron` | Keep in step with `deploy/k8s.yaml` |
| `integrations.*` | empty | Prometheus, OpenCost, Trivy, Loki, Jaeger/Tempo, Alertmanager, Argo CD, Netra, Paqtra URLs and credentials; `integrations.existingSecret` is loaded as `envFrom` |
| `oidc.*`, `oidc.existingSecret` | empty | Single sign-on ([docs/sso.md](../../docs/sso.md)). Setting `oidc.existingSecret` makes the chart ignore the other `oidc.*` values, so that Secret must hold every `VEYRON_OIDC_*` key. There is no value for `VEYRON_OIDC_AUDIENCE` or `_USERINFO_URL`; put them in a Secret named in `integrations.existingSecret` |
| `ai.url`, `apiKey`, `model`, `mode`, `maxToolRounds`, `timeoutSecs`, `rateLimitPerMin` | `""`, `""`, `gpt-4o-mini`, `""`, `2`, `30`, `20` | `VEYRON_AI_*`. `maxToolRounds` and `timeoutSecs` replace the code defaults (8 and 6 rounds; 300 s for an in-cluster model) while non-zero; set them to `0` to leave the variables unset |
| `extraEnv`, `extraEnvFrom` | `[]` | Extra container environment (standard `EnvVar` entries, so `valueFrom` works) and `envFrom` sources (`secretRef` / `configMapRef`), for settings that have no value of their own |
| `resources`, `nodeSelector`, `tolerations`, `affinity`, `serviceAccount.*` | | Standard |

## What the chart does not set up

- **The kairon-node relay.** `deploy/k8s.yaml` loads an optional `veyron-kairon` Secret; this chart does not. Put
  `VEYRON_KAIRON_NODE_URL` (for example `http://{node_ip}:8090`), `KAIRON_NODE_CONSOLE_TOKEN` and, for a self-signed
  relay, `VEYRON_KAIRON_NODE_TLS_INSECURE=1` in a Secret and name it in `integrations.existingSecret`. Without them
  consoles, guest exec, logs and VM-state snapshots do not work on Kairon.
- **Other settings with no value**: `VEYRON_PUBLIC_HOST`, `VEYRON_OIDC_AUDIENCE`, `VEYRON_IMAGE_STORE_MAX_BYTES`,
  `VEYRON_WS_METRICS_MAX`, `VEYRON_CORS_ORIGINS` and the SIEM variables ([docs/soc.md](../../docs/soc.md)) travel the same
  way (`integrations.existingSecret` or `oidc.existingSecret`), or set them one by one with `extraEnv`, for example
  `--set 'extraEnv[0].name=VEYRON_PUBLIC_HOST' --set 'extraEnv[0].value=veyron.example.com'`.
- **Egress for those calls.** With the default NetworkPolicy the pod cannot reach the relay (8090), Prometheus, an
  LLM endpoint or Netra. Add a policy for them or set `networkPolicy.enabled=false`.
- **The HTTP-to-HTTPS redirect listener** (port 5150 in `deploy/k8s.yaml`) is not exposed by the Service.

## Environment

The chart sets `VEYRON_API_KEY` from the `veyron-api-key` Secret (key `api-key`). Everything else listed above is
optional environment on the same container.

## Operator (legacy, KubeVirt only)

`charts/veyron-operator` is a separate chart for the Go operator that manages KubeVirt `VirtualMachine`s through the
`veyron.io` CRDs. It is not used with the Kairon engine (the default) and is only relevant with `VEYRON_VM_BACKEND=kubevirt`.
The release workflow builds no operator image; `scripts/deploy-all-remote.sh` builds `veyron-operator:latest` on the node.

```bash
helm install veyron-operator ./charts/veyron-operator -n veyron-system
```

CRDs remain under **`veyron.io`**; only Kubernetes object names use the Veyron brand.
