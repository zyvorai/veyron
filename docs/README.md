<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Veyron documentation

Veyron is the command center for virtual machines on Kubernetes: a web console, an HTTPS API and
a CLI in one Rust binary, driving [Kairon](https://github.com/zyvorai/kairon) `Machine`s through the
Kubernetes API.

| Goal | Guide |
|---|---|
| Install and create a first VM | [getting-started.md](getting-started.md) |
| How the pieces fit | [architecture.md](architecture.md) |
| Deploy to a cluster, Helm, client tarball, trial builds | [deploy.md](deploy.md) |
| API keys, roles, routes | [api.md](api.md) |
| Single sign-on with Keycloak, Okta, Auth0 or Azure AD | [sso.md](sso.md) |
| GPU virtual machines | [gpu.md](gpu.md) |
| Windows guests and golden images | [windows.md](windows.md) |
| Two racks or two sites | [multi-site.md](multi-site.md) |
| Security operations (detections, SIEM export) | [soc.md](soc.md) |
| AI assistant, MCP, proposals, sandboxes, in-cluster models | [ai.md](ai.md) |
| Prometheus, Atlas, Netra, Paqtra, Copilot and more | [integrations.md](integrations.md) |

**Status.** Kairon is replacing KubeVirt and CDI as Veyron's VM engine. Until that work ships, the
released build still manages KubeVirt `VirtualMachine`s; pages note where that matters.

## Enterprise workflow center

See [enterprise workflow guide](enterprise-workflows.md) for durable power operations, tenant quota reservations, namespace authorization and readiness assessments.
