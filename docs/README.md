# Veyron Documentation

Veyron — Kubernetes-native KubeVirt VM command center

## Start Here

| Goal | Document |
|------|----------|
| Main README | [README.md](../README.md) |
| **CloudOS dashboard shell** | [CLOUDOS_VISION.md](CLOUDOS_VISION.md) |
| **Ask Zeus AI assistant** | [VEYRON_AI.md](VEYRON_AI.md) |
| **User journeys & acceptance criteria** | [User Stories](USER_STORIES.md) |

## Dashboard & Testing

| Task | Command / doc |
|------|----------------|
| Architecture, tiers, themes | [CLOUDOS_VISION.md](CLOUDOS_VISION.md) |
| AI modules & LLM config | [VEYRON_AI.md](VEYRON_AI.md) |
| Ask Zeus standalone page | `https://HOST:30151/ask-zeus` (chat + quick chips) |
| PVC resize | `PATCH /api/v1/storage/pvcs/:ns/:name` · `{"new_size":"50Gi"}` |
| Post-deploy smoke test | `VEYRON_API_KEY='…' ./scripts/verify-veyron-remote.sh HOST [30151]` |
| VM daily ops E2E | `VEYRON_API_KEY='…' ./scripts/test-vm-daily-ops-remote.sh HOST [30151]` |
| Cluster E2E | `./scripts/test-remote.sh HOST USER` |
| OpenRouter for Ask Zeus | `./scripts/configure-zeus-openrouter.sh HOST USER` |

Cache-bust the dashboard after UI deploy: `https://HOST:30151/dashboard?dash=<rev>` (see `veyron-dashboard-rev` meta tag in `dashboard.html`).

## User Stories

Persona-based journeys with acceptance criteria: **[USER_STORIES.md](USER_STORIES.md)**

| Persona | Focus |
|---------|-------|
| Alex (KubeVirt Admin) | Declarative VM lifecycle |
| Morgan (GitOps Engineer) | VeyronVM CR-driven deployments |
| Jordan (Developer) | Templates and blueprints for stacks |

## Ecosystem

Part of the [Zyvor / HyperSDK platform stack](https://zyvor.dev):

| Product | Role |
|---------|------|
| **hypercluster** | Kubernetes bootstrap |
| **machina** | Bare-metal hypervisor OS |
| **zeus-os (v9s)** | Cloud / KubeVirt control plane |
| **forge** | AI infrastructure on K8s |
| **hypersdk / hyper2kvm** | VM migration |
| **guestkit** | Offline VM assurance |
| **packetwolf** | Network intelligence |
| **Aether** | Runtime portability |
| **hermes** | Application layer for K8s |

See also: [../README.md](../README.md)
