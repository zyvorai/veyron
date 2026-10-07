# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Kairon is the VM engine.** Veyron is being rebuilt as the command center for
  [Kairon](https://github.com/zyvorai/kairon) `Machine`s (`kairon.zyvor.dev`), talking to the
  Kubernetes API directly. KubeVirt, CDI and the Veyron operator are on the way out.
- **New console look.** Login, buttons, colors and components follow the Zyvor (Netra) design
  system, in light and dark themes.
- **License: Zyvor Production License v1.0** (`LicenseRef-Zyvor-Production-1.0`). The source is
  public; non-production use is free and production use needs a commercial license from
  [zyvor.dev](https://zyvor.dev). Every source file carries the SPDX header; customer tarballs ship
  `LICENSE`, `NOTICE`, `README.md` and `docs/`.
- **Fresh documentation.** The old guides, decks, PDFs, screenshots and run logs are gone; `docs/`
  now holds a small set of current guides, and the README has new hero, capability and
  architecture images built from `docs/social/`.

### Added

- **Veyron AI** ([docs/ai.md](docs/ai.md)). Bring any OpenAI-compatible model, or deploy one
  in-cluster (llama.cpp on CPU, vLLM on GPU) and select it from the console. Chat with tools,
  intent-to-VM planning, natural-language search, policy drafting, capacity forecasts and
  automatic incident investigations. Every change is a **proposal** that a human approves;
  pending proposals expire after `VEYRON_AI_PROPOSAL_TTL_HOURS` (default 72).
- **MCP server and client.** `/mcp` exposes Veyron's tools to external agents; registered external
  MCP servers show up as `ext.<name>.*` tools.
- **Sandboxes.** Disposable KubeVirt VMs from a warm pool for agent code execution
  (`/api/v1/sandboxes`). They're hidden from the VM list unless `include_sandboxes=true`.
- **`scripts/console-audit.sh`**: headless layout and console-error audit of every page in the
  React console, part of `customer-readiness.sh`. Replaces the stale dashboard audit.
- **`scripts/test-ai-remote.sh`**: Veyron AI end-to-end (MCP, sandboxes, proposals, investigations,
  policy, search, forecast; `VEYRON_E2E_MODEL=1` also deploys and uses an in-cluster model).
- `adapt-existing-cluster.sh` checks the terminated-pod GC threshold and Cilium's socket-LB
  `hostns-only` setting (needed for KubeVirt guest DNS).
- VM list rows carry an `os` hint from template labels or the container disk image.
- **Kryton integration.** `/api/v1/kryton/*` proxies the Kryton machine API: status, catalog,
  machines, power, snapshots, golden-image bootstrap and jobs (`VEYRON_KRYTON_URL`,
  `VEYRON_KRYTON_TOKEN`, `VEYRON_KRYTON_PROJECT`).
- **Refreshed OS templates**, aligned with the Kryton catalog: Ubuntu 26.04, Debian 13, Fedora 44,
  CentOS Stream 10, AlmaLinux 10, Rocky 10 (Kryton golden image), openSUSE Leap 16 and Windows
  Server 2025. `ubuntu`, `debian`, `windows` and the other bare family names now point at the newest
  release; Windows templates default to 80 GiB disks with TPM.

### Fixed

- Cloud-init user-data over 2048 bytes is stored in a Secret instead of being rejected by KubeVirt.
- Cloud-init Secret references used the wrong field name (`userDataSecretRef` instead of
  `secretRef`) and were silently dropped, in both the API and the operator.
- In-cluster llama.cpp models default to a 16k context (the agent prompt overflowed 4k), and
  requests to an in-cluster model get a 300 s timeout.
- The investigator no longer opens incidents for VMs that are being deleted.
- Paqtra status checks no longer time out on a cold cache.
- The legacy `/dashboard` redirect keeps its query string, so an OIDC callback's `?code=` survives;
  the default OIDC redirect URI is now `/console`.
- Customer tarball docs (`CLUSTER_SETUP.txt`, `PREREQUISITES.txt`) said "VMRogue" and quoted old
  KubeVirt/CDI versions; install scripts printed `/dashboard` URLs.

### Removed

- `scripts/dashboard-layout-audit.sh` and `scripts/test-remote.sh` (targeted the retired dashboard).
- Unused scripts: `deploy/remote-deploy.sh`, `demo_colors.sh`, `test_theme.sh`,
  `bootstrap-remote-alma-fix.sh`, `configure-zyra-openrouter.sh`, `console-actions-smoke.sh`,
  `publish-ghcr.sh`, `publish-customer-release.sh`, `rebuild-all-customer-tarballs-remote.sh`, and
  unsourced `scripts/lib` helpers.
- `deploy/monitoring/` (duplicated the `charts/veyron-monitoring` chart).
- Unreferenced examples: `demo_theme.rs`, `library_usage.rs`, `batch-web-cluster.yaml`,
  `custom-blueprint-example.yaml`, `production-database.kubevirt.yaml`, `web-server.kubevirt.yaml`.
- Retired templates for end-of-life or placeholder images: Ubuntu 18.04/20.04, Fedora 42/43,
  CentOS Stream 8, Debian 11, AlmaLinux/Rocky 8, RHEL, Oracle Linux, Alpine, Arch, openSUSE
  Tumbleweed, FreeBSD, Flatcar, Talos and Windows 10.

- The legal templates (`docs/legal/`), the subscription document and the legal sync, bundle and
  license-acceptance scripts.
- `presentations/`, `terraform-provider-vmrogue/`, `web/`, `e2e/`, `scripts/customer-docs/`,
  `scripts/zyvor-branding/`, `QUICK_REFERENCE.md`, `DEVELOPMENT.md`, and the PDF and welcome-page
  generation in the customer bundle.
