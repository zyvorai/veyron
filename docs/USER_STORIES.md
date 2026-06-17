# VMRogue User Stories

**Product:** Veyron — Kubernetes-native KubeVirt VM command center

Cross-reference: [Documentation index](README.md) · [Main README](../README.md)

## Personas

| Persona | Name | Focus |
|---------|------|-------|
| KubeVirt Admin | Alex | Declarative VM lifecycle |
| GitOps Engineer | Morgan | VeyronVM CR-driven deployments |
| Developer | Jordan | Templates and blueprints for stacks |

---

### Story 1 — Deploy VM from template

**As Alex** (KubeVirt Admin), I want 44 os templates with validation, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | veyron CLI, VMConfig YAML |

---

### Story 2 — Multi-VM blueprint

**As Jordan** (Developer), I want deploy lamp or 3-tier stack in one command, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | blueprints, dependency ordering |

---

### Story 3 — GitOps export

**As Morgan** (GitOps Engineer), I want export vms as veyronvm cr manifests, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | veyron gitops-export, operator/ |

---

### Story 4 — Browser VNC console

**As Alex** (KubeVirt Admin), I want direct k8s websocket console without virtctl timeout, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | ConsoleHub, 49 API endpoints |

---

### Story 5 — Policy enforcement

**As Morgan** (GitOps Engineer), I want deny non-compliant vm creates via veyronpolicy, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | operator CEL policies |

---

## Validation

Map each story to smoke tests, CI jobs, or manual lab steps before marking production-ready.
