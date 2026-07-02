# Veyron Handbook

The consolidated operator + user handbook for **Veyron** — the Kubernetes-native
VM command center for **KubeVirt**, and Veyron's role in the [Zyvor platform
stack](https://zyvor.dev).

Veyron turns verbose KubeVirt YAML into a declarative VM lifecycle: a Rust CLI/TUI,
a REST + WebSocket API server, a "Mission Control" web dashboard (40+ pages), a Go
operator with `veyron.io` CRDs, Helm charts, and a Terraform provider. It manages
the full VM lifecycle (create, start/stop, migrate, snapshot, console), OS templates
and multi-VM blueprints, GitOps export, policy enforcement, and deep observability
and security integrations.

---

## 60-second orientation

```text
        CLI ─┐        TUI ─┐        Web Dashboard ─┐
             │             │        (Mission Control)│
             └──────► veyron REST + WebSocket API ◄──┘
                              │  (axum, /api/v1)
                              ▼
                     kube-rs  ──►  KubeVirt / CDI
                              ▲         (VirtualMachine, VMI,
   VeyronVM CRD ─► Go operator┘          snapshots, migrations)
   (veyron.io group)
```

- **Core** — Rust binary `veyron` (`src/`): CLI, TUI, library, and the API server.
- **Operator** — Go controller-runtime (`operator/`): reconciles `veyron.io` CRDs
  (`VeyronVM`, `VeyronPolicy`, `VeyronBlueprint`, `VMTemplate`, `VMProfile`,
  `VeyronAction`, `VeyronInsight`) into KubeVirt objects.
- **Web** — dashboard shell served by the API (`src/api/web/`).
- **Deploy** — Helm charts (`charts/`), raw manifest (`deploy/k8s.yaml`), and
  `scripts/deploy-*.sh`.

## Ports at a glance

| Context | Port | Protocol | Notes |
|---------|------|----------|-------|
| `veyron api-serve` default | **8080** | HTTP | Binds `127.0.0.1`, TLS off, auth `none` (local dev default from `AppConfig`). |
| Container / Helm default | **5151** | HTTPS | `CMD ["api-serve","--port","5151","--host","0.0.0.0"]`; TLS on. |
| HTTP→HTTPS redirect | **5150** | HTTP | Auto-started at `port − 1` when TLS is enabled. |
| Service NodePort (HTTPS) | **30151** | HTTPS | Chart `service.nodePort`; maps to container 5151. |
| Service NodePort (HTTP redirect) | **30150** | HTTP | From `deploy/k8s.yaml` (port 80 → 5150). |

The API port is set via `veyron api-serve --port`, `config.toml` `[api] port`, or
the Helm/manifest env. See [admin-configuration.md](admin-configuration.md#ports).

## Fastest deploy path

```bash
# Helm (recommended) — needs a cluster with KubeVirt + CDI installed
helm upgrade --install veyron charts/veyron \
  --namespace veyron-system --create-namespace \
  --set auth.apiKey="$(openssl rand -hex 24)"

# Then reach the dashboard on any node
open https://<node-ip>:30151/dashboard
```

Local build & run:

```bash
cargo build --release
VEYRON_API_KEY=dev-key ./target/release/veyron api-serve --port 8080
# → http://127.0.0.1:8080/dashboard
```

See [admin-configuration.md](admin-configuration.md#deployment-models) for the
`scripts/deploy-k8s.sh` and remote-deploy paths.

## The four guides

| Guide | Read it for |
|-------|-------------|
| [Product Guide](product-guide.md) | Concepts, the CLI/TUI/API/dashboard surfaces, and a feature deep-dive (VM lifecycle, templates, blueprints, consoles, migration, snapshots, GitOps, policy, AI). |
| [Administration & Configuration](admin-configuration.md) | Deployment models, ports, the full env-var / `config.toml` reference, auth & TLS, KubeVirt prerequisites, building from source, production checklist. |
| [FAQ](faq.md) | 30+ real questions on setup, auth, ports, KubeVirt, GitOps, and integrations. |
| [Troubleshooting](troubleshooting.md) | Symptom-indexed fixes with real diagnostic commands. |

## Related deep-dive docs

The handbook links to, rather than duplicates, the existing topic guides:
[OIDC/SSO](../OIDC_SSO.md) ·
[Ask Zeus / AI](../VEYRON_AI.md) ·
[Snapshots](../SNAPSHOTS.md) ·
[Network management](../NETWORK_MANAGEMENT.md) ·
[Disk management](../DISK_MANAGEMENT.md) ·
[Template catalog](../TEMPLATE_CATALOG.md) ·
[OS templates](../OS_TEMPLATES.md) ·
[Developer VM access](../DEVELOPER_VM_ACCESS.md) ·
[Interactive TUI](../INTERACTIVE_TUI.md) ·
[Windows on KubeVirt](../WINDOWS_KUBEVIRT_PRODUCTION.md) ·
[SOC / SIEM export](../SOC.md) ·
[Optional integrations](../OPTIONAL_INTEGRATIONS.md) ·
[Feature matrix](../FEATURE_MATRIX.md) ·
[User stories](../USER_STORIES.md).
