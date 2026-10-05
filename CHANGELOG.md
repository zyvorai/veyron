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

- **Kryton integration.** `/api/v1/kryton/*` proxies the Kryton machine API: status, catalog,
  machines, power, snapshots, golden-image bootstrap and jobs (`VEYRON_KRYTON_URL`,
  `VEYRON_KRYTON_TOKEN`, `VEYRON_KRYTON_PROJECT`).
- **Refreshed OS templates**, aligned with the Kryton catalog: Ubuntu 26.04, Debian 13, Fedora 44,
  CentOS Stream 10, AlmaLinux 10, Rocky 10 (Kryton golden image), openSUSE Leap 16 and Windows
  Server 2025. `ubuntu`, `debian`, `windows` and the other bare family names now point at the newest
  release; Windows templates default to 80 GiB disks with TPM.

### Removed

- Retired templates for end-of-life or placeholder images: Ubuntu 18.04/20.04, Fedora 42/43,
  CentOS Stream 8, Debian 11, AlmaLinux/Rocky 8, RHEL, Oracle Linux, Alpine, Arch, openSUSE
  Tumbleweed, FreeBSD, Flatcar, Talos and Windows 10.

- The legal templates (`docs/legal/`), the subscription document and the legal sync, bundle and
  license-acceptance scripts.
- `presentations/`, `terraform-provider-vmrogue/`, `web/`, `e2e/`, `scripts/customer-docs/`,
  `scripts/zyvor-branding/`, `QUICK_REFERENCE.md`, `DEVELOPMENT.md`, and the PDF and welcome-page
  generation in the customer bundle.
