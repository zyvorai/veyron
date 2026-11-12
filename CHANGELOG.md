# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- **Session tokens are revocable.** Login-minted JWTs now carry a per-user token version; deleting a user or resetting
  their password rejects every token issued before (user list cached up to 5 s). A role change still applies only when
  the token expires. Tokens from older versions expire within 24 h as before.
- **OIDC audience.** `VEYRON_OIDC_AUDIENCE`, when set, must appear in the token's `aud`; unset keeps the old behaviour.
- **Login rate limit is per client.** 30 attempts per 5 minutes per client IP and 10 per IP and username, instead of one
  global budget that a single flood could exhaust for everyone. Keyed on the TCP peer address, so behind a reverse proxy
  clients share the proxy's budget.
- **Legacy `?token=` restricted** to WebSocket upgrade routes (`/vnc`, `/serial`, `/tunnel`, `/ws/metrics`); other
  routes ignore it and answer `401`.
- **API webhook delivery** passes the secret and headers to curl on stdin instead of its command line, re-validates the
  URL at delivery time and pins curl to the validated IP (`--resolve`).
- **HTTP-to-HTTPS redirect** uses `VEYRON_PUBLIC_HOST` when set and otherwise rejects a malformed `Host` header.
- **KubeVirt console proxy verifies the Kubernetes API certificate** against the cluster CA. Opt out with
  `VEYRON_KUBEVIRT_WS_INSECURE_TLS=1` (previously it never verified).
- **Image-store uploads are capped** (`VEYRON_IMAGE_STORE_MAX_BYTES`, default 64 GiB, `0` = unlimited); larger bodies
  answer `413 UPLOAD_TOO_LARGE` and the partial file is removed.

- **VeyronAction approval and VeyronPolicy changes are Admin.** `POST /crds/actions/:ns/:name/approve` and every
  non-read request to `/crds/policies*` now need `admin` (they were `write`). `POST /crds/actions` ignores `approved` and
  `autoApprove` from a caller below `admin`, so a `write` key could neither approve its own action nor delete the
  `Deny` policy that blocks it.
- **Audit log can no longer be silenced by a long URL.** The SOC audit entry for a mutating call stored the request
  path (four times) unbounded; enough long paths pushed the `veyron-soc-events` ConfigMap past 1 MiB and every later
  write was rejected without notice. Paths are cut at 256 bytes, the actor at 128, and the stored log is trimmed to
  900 KB, oldest first.
- **`/ws/metrics` streams are capped** (`VEYRON_WS_METRICS_MAX`, default 64; extra upgrades answer `503`). Each stream
  lists every VM and node every 5 s and a `readonly` key may open one.

### Fixed

- **`timeout_seconds` on guest exec was unbounded.** `u64::MAX` overflowed the relay's `+ 15 s` margin (a panic in
  debug builds, a 14 s HTTP timeout in release). It is capped at 3600 s.
- **`cpus` on `POST /vms` was not range-checked.** `0` silently became one vCPU and billions went to the backend.
  Values outside 1-256 answer `400 INVALID_CPUS`.
- **`veyron api-key-create` / `api-key-delete` / `api-key-list` no longer pretend.** There is no key store; they printed
  success and persisted nothing. They now fail and point at `VEYRON_API_KEY` / `VEYRON_API_KEYS`.

- **A read-only key could open the interactive VNC and serial consoles.** `GET /vms/:ns/:name/vnc` and `/serial` were
  classified as safe methods, so any `readonly` credential (header or `?token=`) could drive a guest's console, while the
  TCP tunnel needs `write` and screenshot/input need `admin`. Both now need `write`; the read-only
  `/console/serial` hint is unchanged. Browser sessions were unaffected because `POST /ws/ticket` already needs `write`.
- **`--rate-limit 0` rejected every request.** The flag, config and docs say `0` disables the limit, but the limiter
  answered `429` to everything. `0` now disables it.
- **`gpu.count` on `POST /vms` was unbounded** (a `write` caller could make the API allocate billions of GPU devices).
  Counts above 16 answer `400 INVALID_GPU`.

## [0.4.0] - 2026-11-11

First release published from CI. The version numbers 0.3.0 to 0.3.2 were already taken by container images pushed to
`ghcr.io/zyvorai/veyron` on 2026-10-04 (before the move to Kairon), so this release continues from 0.4.0; the `v0.3.0` and
`v0.3.1` git tags were never released. Contents are those of the 0.3.0 entry below, plus:

- **Release workflow** logs in to GHCR with the built-in `GITHUB_TOKEN` instead of a personal access token secret that was
  not configured. The `veyron` package must grant this repository Write access (package settings, *Manage Actions access*).

Known gaps in the v0.4.0 release: the container image `ghcr.io/zyvorai/veyron:0.4.0` was **not published** (the GHCR package
refused the push), so Helm's default `latest` is still the older 2026-10-04 image; and the cosign signature and SBOM were not
attached. The assets are the four `veyron-{linux,darwin}-{amd64,arm64}` binaries and `SHA256SUMS`.

## [0.3.1 (unreleased)] - 2026-11-11

First published release (the `v0.3.0` tag was cut before the release workflow's GHCR login was fixed and never produced a release). Contents are those of the 0.3.0 entry below plus:

- **Release workflow** logs in to GHCR with the built-in `GITHUB_TOKEN` instead of a personal access token secret that was not configured.

## [0.3.0 (unpublished)] - 2026-11-11

### Fixed

- **Supply chain.** CI and the release workflow pin the guestkit dependency to one tested commit (the
  release workflow was unpinned and built against whatever guestkit's default branch held), which also
  removes the `evtx` 0.12 downgrade that carried RUSTSEC-2026-0431. Dependencies are updated to their
  latest semver-compatible versions; `cargo audit` reports no vulnerabilities.

### Changed

- **Kairon is the VM engine.** Veyron is being rebuilt as the command center for
  [Kairon](https://github.com/zyvorai/kairon) `Machine`s (`kairon.zyvor.dev`), talking to the
  Kubernetes API directly. KubeVirt is a legacy, explicit opt-in that is still in the tree and not removed:
  `VEYRON_VM_BACKEND=kubevirt` (API), the CDI image routes (`/images/upload`, `/images/publish`), the Go
  `veyron-operator` chart and the CLI's VM commands (`veyron create`, `list`, `start`, `snapshot-*`, `tui`), which
  talk to KubeVirt `VirtualMachine`s directly and do not go through the Kairon backend in this release.
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

- **Eject install media (Kairon).** `POST /api/v1/vms/:ns/:name/cdroms/:cdrom/eject` (Write) drops
  the drive from the Machine's `spec.cdroms` (guarded by `resourceVersion`); the Kairon agent ejects
  it from the running guest, after which the VM can live-migrate. `GET /vms` reports remaining
  drives as `cdroms`, and the console gains an **Eject media** VM action. Needs Kairon with
  cdrom eject (Kairon PR #37) and FluxVM with `POST /v1/vms/{id}/cdroms/{name}/eject`.
- **Docs: network at kernel speed.** New [docs/ebpf.md](docs/ebpf.md) and README card `docs/assets/readme-ebpf.jpg`
  (rendered from `docs/social/ebpf.html`) on the FluxVM eBPF VM edge that Veyron VMs on Kairon
  run behind: what it does, evidence-linked numbers, and which controls live in Kairon today.
- **Capture a VM as a golden image (Kairon).** `POST /api/v1/vms/:ns/:name/capture` backs up the
  VM's root disk, stores it in Veyron's image store and publishes it as a `MachineImage`; track it
  with `GET /api/v1/image-captures` or the console's "Capture image" VM action.
- **Kairon only.** Veyron runs VMs on Kairon by default and never falls back to KubeVirt
  (`VEYRON_VM_BACKEND=kubevirt` is an explicit legacy opt-in). KubeVirt/CDI/operator-only
  routes answer `501 KUBEVIRT_ONLY`, readiness reports the Kairon engine, and the deploy
  scripts no longer install CDI unless `VEYRON_INSTALL_CDI=1`.
- **Import migrated VMs.** `POST /api/v1/imports` boots a disk converted by
  [h2kvm](https://github.com/zyvorai/h2kvm) (qcow2, raw, OVA, VMDK, VHD/X) as a Kairon Machine
  from an HTTP(S) URL pinned by SHA-256; h2kvm's new `--deploy-kairon` calls it, and the
  console's Images page now opens an import form.
- **Images without CDI.** On Kairon, uploads stream into Veyron's image store
  (`PUT /api/v1/image-store/:name`), `POST /api/v1/machine-images` publishes them (or any URL +
  SHA-256) as a versioned Kairon `MachineImage`, and `POST /vms` takes `image` (a catalog disk),
  `iso` + `driver_iso` (install onto a blank disk; Windows templates add `virtio-win`) and
  `root_volume` (a PVC Kairon seeds from the image). The console's Images page lists the catalog
  and uploads with progress, and the new-VM sheet gains **Boot from**: template, catalog image or
  ISO install. The API deployment now mounts a `veyron-image-store` PVC.
- **Full Kairon backend.** The whole VM API runs on Kairon Machines: pause, clone, run strategy, CPU/memory hotplug, live/cold migration with cancel,
  snapshots and backups, guest exec/patch/disk reclaim, VM logs (`GET /vms/:ns/:name/logs`),
  VNC and serial consoles through the kairon-node relay, and SSH/RDP expose. Stopping a Kairon
  VM now powers it off (`Halted`) instead of tearing it down, so image-booted disks survive.
  Relay settings come from an optional `veyron-kairon` Secret. The `kairon` cargo feature is on
  by default.
- **Gryvia integration.** Veyron auto-detects [Gryvia](https://github.com/zyvorai/gryvia) in
  `gryvia-system`, reads its API key from `gryvia-api-key`, and shows its health, container GPU
  capacity, GPU nodes, tenants and GPU-hour usage on a new **Gryvia** console page
  (`GET /api/v1/gryvia/{status,nodes,tenants,quotas,usage}`). GPU VMs honor the NVIDIA GPU
  Operator's `nvidia.com/gpu.workload.config` label: `container` nodes are reported as not
  VM-usable, and a VM whose GPU only exists there gets `422 GPU_RESERVED_FOR_CONTAINERS`.

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

- **`POST /api/v1/snapshots` answered `405`.** The static `GET /snapshots` route shadowed the nested
  router's `POST`; the route is now registered on it directly. `POST /snapshots/:ns/:vm/create`
  also honours a custom `name` (it only read `snapshot_name`).
- **Snapshots of a VM that doesn't exist** are rejected with `404` instead of leaving a snapshot
  stuck `InProgress` (CLI, both API routes and the scheduler).
- **New VMs no longer flash `Stopped`.** A VM that is meant to run reports `Starting` until the
  engine's status catches up.

- Linux template cloud-init now quotes the generated password. About one VM in 22 got a password
  starting with `!`, `@` or `%`, which made the YAML invalid: the guest skipped its user, password
  and GuestKit setup, and merging custom user-data silently dropped the template's part.
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
