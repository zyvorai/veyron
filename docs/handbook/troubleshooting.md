# Veyron Troubleshooting

> Part of the [Veyron Handbook](README.md) · see also
> [Product Guide](product-guide.md) ·
> [Administration & Configuration](admin-configuration.md) · [FAQ](faq.md)

Symptom-indexed fixes with real diagnostic commands. Set logging to `debug` for
detail: `RUST_LOG=debug veyron api-serve …` or `[logging] level = "debug"` /
`VEYRON_… LOG_LEVEL`. In-cluster, check pod logs and events first.

```bash
kubectl -n veyron-system get pods
kubectl -n veyron-system logs deploy/veyron-api --tail=200
kubectl -n veyron-system describe pod -l app.kubernetes.io/name=veyron
```

---

## Auth & access

### Every request returns `503 AUTH_NOT_CONFIGURED`
The API key is not set. The server logs
`VEYRON_API_KEY … is not set - API will reject all requests`.

```bash
# Local
export VEYRON_API_KEY="$(openssl rand -hex 24)"
veyron api-serve --port 8080

# Helm
helm upgrade veyron charts/veyron -n veyron-system \
  --reuse-values --set auth.apiKey="$(openssl rand -hex 24)"
kubectl -n veyron-system rollout restart deploy/veyron-api
```

### `401 UNAUTHORIZED` on API calls
The key is wrong or missing from the request. Send it correctly:

```bash
curl -k -H "X-API-Key: $VEYRON_API_KEY" https://HOST:30151/api/v1/health
# or
curl -k -H "Authorization: Bearer $VEYRON_API_KEY" https://HOST:30151/api/v1/vms
```

The dashboard keeps the key in `localStorage` — if the dashboard 401s, re-enter the
key on the login screen (clears/refreshes stored key).

### `401 Invalid or expired WebSocket ticket` on console/VNC
Console WebSockets need a fresh ticket (short-lived, single-use). Reopen the console
so the UI fetches a new `GET /api/v1/ws/ticket`; don't reuse or bookmark a
`?ticket=` URL.

### OIDC/SSO login fails
Verify all `VEYRON_OIDC_*` values (issuer, client ID, authorization/token/userinfo/
JWKS URLs, redirect URI) and the role claim. Check `/api/v1/auth/oidc/config`
returns your provider config. See [OIDC/SSO](../OIDC_SSO.md).

---

## Connectivity & ports

### Dashboard unreachable at `https://HOST:30151/dashboard`
```bash
kubectl -n veyron-system get svc veyron-api          # confirm NodePort 30151
kubectl -n veyron-system get pods                     # pod Running/Ready?
curl -k https://HOST:30151/api/v1/health              # health probe
```
- Use **https** (TLS is on by default in-container) and accept the self-signed cert
  (`curl -k`) unless you supplied `tls.existingSecret`.
- If the port differs, it's set by `service.nodePort` / `api.port`.

### `connection refused` locally on 8080
The local default binds `127.0.0.1`. To reach it from another host, run
`veyron api-serve --host 0.0.0.0 --port 8080` (and mind the firewall).

### Console/expose links point to the wrong host
Set `VEYRON_API_NODE_HOST` and `VEYRON_API_NODE_PORT` (e.g. `30151`) so generated
URLs resolve externally; `VEYRON_API_CLUSTER_IP`/`VEYRON_CLUSTER_DNS` govern
internal URLs.

### TLS handshake errors / no redirect
With TLS on, an HTTP→HTTPS redirect runs on `port − 1` (5150). If the redirect port
is taken, the log shows `Could not start HTTP redirect server on …` — free the port
or ignore if you only use HTTPS. Verify cert/key paths (`tls_cert`/`tls_key`) exist
and match.

---

## Kubernetes / KubeVirt

### VMs don't appear / `no matches for kind "VirtualMachine"`
KubeVirt isn't installed or the CRDs are missing.

```bash
kubectl get crd | grep kubevirt
kubectl -n kubevirt get kubevirt kubevirt -o jsonpath='{.status.phase}'   # Deployed
```
Install KubeVirt, then re-check.

### Disk import / DataVolume stuck
CDI is missing or unhealthy.

```bash
kubectl get crd | grep cdi
kubectl -n cdi get pods
```
Install/repair CDI (helper: `scripts/ensure-cdi-remote.sh`).

### `403 Forbidden` from the Kubernetes API in logs (RBAC)
The ServiceAccount lacks permissions. Confirm the ClusterRole/binding:

```bash
kubectl get clusterrole veyron -o yaml
kubectl -n veyron-system get sa veyron
kubectl auth can-i list virtualmachines.kubevirt.io \
  --as=system:serviceaccount:veyron-system:veyron
```
Reinstall with `rbac.create=true`.

### Local CLI: `KUBECONFIG` not found / wrong cluster
Veyron uses `KUBECONFIG` (or `config.toml` `kubeconfig`, which overrides it).

```bash
veyron config-show                # shows effective config incl. kubeconfig
KUBECONFIG=~/.kube/prod veyron -n default vms list
```

### Live migration fails or never completes
Requires migratable storage/network. Check the migration object and node capacity:

```bash
veyron migration-status <vm> -n <ns>
kubectl -n <ns> get virtualmachineinstancemigration
kubectl -n <ns> describe vmi <vm>
```

---

## Snapshots & storage

### Scheduled snapshots not running
The scheduler uses a lease for HA. If disabled or namespaced wrong, schedules stall.
Check `VEYRON_SCHEDULER_LEASE_DISABLED` / `VEYRON_SCHEDULER_LEASE_NAMESPACE` and the
schedule ConfigMap:

```bash
curl -k -H "X-API-Key: $VEYRON_API_KEY" https://HOST:30151/api/v1/snapshot-schedules
kubectl -n <ns> get virtualmachinesnapshot
```
See [Snapshots](../SNAPSHOTS.md).

### PVC resize has no effect
Resize with the API and confirm the StorageClass allows volume expansion:

```bash
curl -k -X PATCH -H "X-API-Key: $VEYRON_API_KEY" \
  -H 'content-type: application/json' -d '{"new_size":"50Gi"}' \
  https://HOST:30151/api/v1/storage/pvcs/<ns>/<pvc>
kubectl get storageclass <sc> -o jsonpath='{.allowVolumeExpansion}'   # true
```
See [Disk management](../DISK_MANAGEMENT.md).

---

## AI / integrations

### "Ask Zyra" returns errors or is disabled
Verify AI config; if unset, the assistant is off.

```bash
env | grep '^VEYRON_AI_'
# needs at least VEYRON_AI_MODE, VEYRON_AI_URL, VEYRON_AI_API_KEY, VEYRON_AI_MODEL
```
Configure OpenRouter with `scripts/configure-zyra-openrouter.sh HOST USER`. See
[Veyron AI](../VEYRON_AI.md).

### Observability / cost panels empty
The relevant backend URL isn't set. Provide `VEYRON_PROMETHEUS_URL`,
`VEYRON_LOKI_URL`, `VEYRON_OPENCOST_URL`, etc.; each is independent and optional.
See [Optional integrations](../OPTIONAL_INTEGRATIONS.md).

### SIEM export not delivering events
Check the exporter's required vars (e.g. Splunk `VEYRON_SPLUNK_HEC_URL` +
`_HEC_TOKEN`; Sentinel `VEYRON_SENTINEL_*`) and look for export-loop errors in the
pod logs. See [SOC](../SOC.md).

---

## Build & deploy

### `cargo build` fails on the guestkit dependency
The submodule isn't initialized:

```bash
git submodule update --init --recursive
cargo build --release
```

### Rust toolchain too old
Veyron needs Rust ≥ 1.85 (edition 2024): `rustup update stable`.

### Dashboard shows stale UI after redeploy
Cache-bust with a rev query param:
`https://HOST:30151/dashboard?dash=<rev>` (the `veyron-dashboard-rev` meta tag in
`dashboard.html`).

### General health / self-diagnosis
```bash
veyron doctor                 # environment & cluster diagnostics
veyron info --diagnostics     # build + connectivity info
veyron api-status             # API server status
VEYRON_API_KEY=… scripts/verify-veyron-remote.sh HOST 30151   # remote smoke test
```
