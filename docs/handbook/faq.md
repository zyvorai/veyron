# Veyron FAQ

> Part of the [Veyron Handbook](README.md) · see also
> [Product Guide](product-guide.md) ·
> [Administration & Configuration](admin-configuration.md) ·
> [Troubleshooting](troubleshooting.md)

## General

**1. What is Veyron?**
A Kubernetes-native VM command center for **KubeVirt**, written in Rust. It provides
a CLI, TUI, REST + WebSocket API, and a web dashboard to manage the full KubeVirt
VM lifecycle: templates, blueprints, consoles, migration, snapshots, GitOps, and
policy.

**2. Where does Veyron fit in the Zyvor stack?**
It is the **KubeVirt VM command center** tier — the control surface for VMs running
on Kubernetes. It complements `machina` (bare-metal hypervisor OS), `zeus-os`/`v9s`
(cloud/KubeVirt control plane), `guestkit` (offline VM assurance), and `packetwolf`
(network intelligence).

**3. What language and stack is it built on?**
Rust (edition 2024, ≥ 1.85) using `tokio`, `axum` (web), `kube-rs` (Kubernetes),
`ratatui` (TUI), and `clap` (CLI). The operator is Go (controller-runtime). The
dashboard is a single-page app served by the API.

**4. Is it open source / what's the license?**
It ships under a proprietary Zyvor license (`LicenseRef-Zyvor-Proprietary`). See
`LICENSE` and `docs/legal/`.

**5. Do I need KubeVirt to use it?**
Yes — Veyron manages KubeVirt objects. You need a cluster with KubeVirt (and CDI
for disk imports). See [KubeVirt prerequisites](admin-configuration.md#kubevirt-prerequisites).

## Surfaces & usage

**6. What surfaces does Veyron expose?**
CLI (`veyron`, 200+ subcommands), TUI (`veyron tui`), a REST + WebSocket API
(`veyron api-serve`), and the "Mission Control" web dashboard (40+ pages) served at
`/dashboard`.

**7. How do I start the API/dashboard?**
`veyron api-serve` (the README's `veyron serve` shorthand maps to this). Locally it
binds `127.0.0.1:8080`; the container serves HTTPS on `0.0.0.0:5151`.

**8. How do I create a VM?**
`veyron create --template ubuntu-22.04 --name web-01 -n dev`, or via the dashboard,
or by applying a `VeyronVM` CR for the operator to reconcile.

**9. How many OS templates and profiles ship?**
Per the README: **44 OS templates** and **8 resource profiles**, plus multi-VM
blueprints (LAMP, Kubernetes, 3-tier, CI/CD). See [OS templates](../OS_TEMPLATES.md)
and [template catalog](../TEMPLATE_CATALOG.md).

**10. How do I list the API routes?**
`veyron api-routes`, or fetch the embedded OpenAPI at `/api/openapi.json`
(`veyron api-spec`).

**11. Is there an API version prefix?**
Yes, routes are under `/api/v1` (with a `/api/v2` surface and a `/api/v1/veyron`
product alias).

**12. How do I open a VM console?**
`veyron console <vm>` / `veyron vnc <vm>` from the CLI, or use the Console Hub page
in the dashboard. Consoles run over a direct Kubernetes WebSocket, avoiding
`virtctl` timeouts.

## Ports & networking

**13. What port does Veyron listen on?**
Default local: **8080** (HTTP, `127.0.0.1`). In-container/Helm: **5151** (HTTPS),
exposed via NodePort **30151**. See the [ports table](admin-configuration.md#ports).

**14. Why is there a service on port 5150 / NodePort 30150?**
When TLS is enabled the server starts an HTTP→HTTPS redirect on `port − 1` (5150),
exposed on NodePort 30150 in `deploy/k8s.yaml`.

**15. How do I change the port?**
`veyron api-serve --port <n>`, or `[api] port` in `config.toml`, or Helm
`api.port`/`service.*`. Precedence: flag > file > default.

**16. How do external console/expose URLs get their host and port?**
From `VEYRON_API_NODE_HOST` and `VEYRON_API_NODE_PORT` (e.g. `30151`), with
`VEYRON_API_CLUSTER_IP`/`VEYRON_CLUSTER_DNS` for internal URLs.

## Authentication & security

**17. How is the API authenticated?**
An API-key gate. Set `VEYRON_API_KEY` (admin). Clients send `X-API-Key`,
`Authorization: Bearer <key>`, or `?token=<key>`. Without the key set, the API
returns `503 AUTH_NOT_CONFIGURED`.

**18. Can I have multiple keys with different roles?**
Yes — `VEYRON_API_KEYS="name:key:role,..."` with roles `admin`, `write`, or
`readonly`.

**19. How does the dashboard authenticate?**
It stores the API key in `localStorage` and sends it via `X-API-Key` on every
request — the same gate as any other client.

**20. How do WebSocket consoles authenticate (they can't set headers)?**
They fetch a short-lived ticket from `GET /api/v1/ws/ticket` and pass it as
`?ticket=` on the WS URL.

**21. Does Veyron support SSO?**
Yes — OIDC/SSO via `VEYRON_OIDC_*` (endpoints under `/api/v1/auth/oidc/*`) and JWT
bearer via `VEYRON_JWT_*`. See [OIDC/SSO](../OIDC_SSO.md).

**22. How do I enable TLS?**
`[api] tls = true` with `tls_cert`/`tls_key`, or `--tls --tls-cert --tls-key`. In
Helm, `tls.enabled=true` with `tls.autoGenerate=true` (self-signed) or
`tls.existingSecret`.

**23. What's the default rate limit?**
60 requests/minute (`rate_limit` / `rateLimit.perMinute`). Set to `0` to disable.

**24. Is the sample key `CHANGE_ME` safe for production?**
No. It appears in `scripts/deploy-k8s.sh` for convenience only — always override
`VEYRON_API_KEY` / Helm `auth.apiKey` with a strong random value.

## Deployment & operations

**25. What's the fastest way to deploy?**
`helm upgrade --install veyron charts/veyron -n veyron-system --create-namespace
--set auth.apiKey=<key>`, then browse `https://<node>:30151/dashboard`.

**26. What container image is used?**
`ghcr.io/ssahani/veyron` (tag `latest` by default; pin a version in production).

**27. Where does config come from?**
Layered: compiled defaults → `/etc/veyron/config.toml` → `~/.config/veyron/config.toml`
→ env/CLI. Initialize with `veyron config-init`; inspect with `veyron config-show`.

**28. Do I need the operator?**
Only for CR-driven GitOps. Apply `VeyronVM`/`VeyronPolicy`/etc. and the operator
(`charts/veyron-operator`) reconciles them into KubeVirt objects. The CLI/API work
without it.

**29. What CRDs does the operator define?**
Group `veyron.io`: `VeyronVM`, `VeyronPolicy`, `VeyronBlueprint`, `VMTemplate`,
`VMProfile`, `VeyronAction`, `VeyronInsight`.

**30. How do I export existing VMs to GitOps?**
`veyron gitops-export --namespace <ns> -o manifests/` renders live VMs to CR
manifests.

**31. How do I take/schedule snapshots?**
`veyron snapshot-create` / `snapshot-restore`, or the API `/api/v1/snapshots*`.
Schedules are ConfigMap-driven (`/api/v1/snapshot-schedules`) with an optional HA
lease (`VEYRON_SCHEDULER_LEASE_*`). See [Snapshots](../SNAPSHOTS.md).

**32. How do I live-migrate a VM?**
`veyron migrate <vm>` or `POST /api/v1/vms/:ns/:name/migrate`. GuestKit's
migrate-score gauges readiness from inside the guest.

**33. Can I run Windows VMs?**
Yes — set `VEYRON_VIRTIO_WIN_CONTAINER_DISK` and enable RDP exposure. See
[Windows on KubeVirt](../WINDOWS_KUBEVIRT_PRODUCTION.md).

## Integrations & AI

**34. What observability backends integrate?**
Prometheus, Loki, Jaeger/Tempo, Alertmanager, and Grafana via `VEYRON_*_URL` vars.
The `charts/veyron-monitoring` chart ships ServiceMonitors, rules, and dashboards.

**35. What SIEM/SOC exporters are supported?**
Splunk, QRadar (LEEF), Microsoft Sentinel, Elastic, and a generic SOAR webhook. See
[SOC](../SOC.md).

**36. What is "Ask Zeus"?**
Veyron's built-in LLM assistant plus per-domain copilots, configured via
`VEYRON_AI_*`. Optional and provider-agnostic (e.g. OpenRouter via
`scripts/configure-zeus-openrouter.sh`). See [Veyron AI](../VEYRON_AI.md).

**37. Is there a Terraform provider?**
Yes — `terraform-provider-veyron/`.

**38. How do I run the post-deploy smoke test?**
`VEYRON_API_KEY=<key> scripts/verify-veyron-remote.sh HOST 30151` and
`scripts/test-vm-daily-ops-remote.sh HOST 30151`.
