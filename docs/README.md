# Veyron Documentation

Veyron — Kubernetes-native KubeVirt VM command center

## Start Here

| Goal | Document |
|------|----------|
| Main README | [README.md](../README.md) |
| **Licensing & trial token** | [LICENSING.md](LICENSING.md) |
| **Handbook (product · admin · FAQ · troubleshooting)** | [handbook/README.md](handbook/README.md) |
| **CloudOS dashboard shell** | [CLOUDOS_VISION.md](CLOUDOS_VISION.md) |
| **Ask Zyra AI assistant** | [VEYRON_AI.md](VEYRON_AI.md) |
| **User journeys & acceptance criteria** | [User Stories](USER_STORIES.md) |

## Infrastructure & Cluster

| Goal | Document |
|------|----------|
| **GPU cloud strategy & phase roadmap** | [NEO_CLOUD_GPU_ROADMAP.md](NEO_CLOUD_GPU_ROADMAP.md) |
| **GPU passthrough runbook** (vfio → permittedHostDevices → GPU VM) | [GPU_PASSTHROUGH.md](GPU_PASSTHROUGH.md) |
| **Two-rack WireGuard mesh** (join site-2 workers, storage locality) | [MULTI_SITE_MESH.md](MULTI_SITE_MESH.md) |

## Dashboard & Testing

| Task | Command / doc |
|------|----------------|
| Architecture, tiers, **Light/Dark Apple themes** | [CLOUDOS_VISION.md](CLOUDOS_VISION.md) · `CLAUDE.md` § Design system |
| AI modules & LLM config | [VEYRON_AI.md](VEYRON_AI.md) |
| Ask Zyra standalone page | `https://HOST:30151/ask-zyra` (chat + quick chips) |
| PVC resize | `PATCH /api/v1/storage/pvcs/:ns/:name` · `{"new_size":"50Gi"}` |
| Post-deploy smoke test | `VEYRON_API_KEY='…' ./scripts/verify-veyron-remote.sh HOST [30151]` |
| VM daily ops E2E (create, VNC ticket, SSH/RDP, snapshots) | `VEYRON_API_KEY='…' ./scripts/test-vm-daily-ops-remote.sh HOST [30151]` |
| Dashboard Chrome console sweep | `VEYRON_API_KEY='…' ./scripts/dashboard-console-check.sh --host HOST` |
| Customer go-live gate | `VEYRON_API_KEY='…' ./scripts/customer-readiness.sh HOST [--ssh-user U] [--full]` |
| Cluster E2E | `./scripts/test-remote.sh HOST USER` |
| OpenRouter for Ask Zyra | `./scripts/configure-zyra-openrouter.sh HOST USER` |

Cache-bust the dashboard after UI deploy: `https://HOST:30151/dashboard?dash=<rev>` (see `veyron-dashboard-rev` meta tag in `dashboard.html`; current rev `20260902b`).

## User Stories

Persona-based journeys with acceptance criteria: **[USER_STORIES.md](USER_STORIES.md)**

| Persona | Focus |
|---------|-------|
| Alex (KubeVirt Admin) | Declarative VM lifecycle |
| Morgan (GitOps Engineer) | VeyronVM CR-driven deployments |
| Jordan (Developer) | Templates and blueprints for stacks |

## Ecosystem

Part of the [Zyvor platform stack](https://zyvor.dev):

| Product | Role |
|---------|------|
| **hypercluster** | Kubernetes bootstrap |
| **machina** | Bare-metal hypervisor OS |
| **zeus-os (v9s)** | Cloud / KubeVirt control plane |
| **forge** | AI infrastructure on K8s |
| **transiva / h2kvm** | VM migration |
| **guestkit** | Offline VM assurance |
| **packetwolf** | Network intelligence |
| **Axiom** | Runtime portability |
| **hermes** | Application layer for K8s |

See also: [../README.md](../README.md)
