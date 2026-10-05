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

### Removed

- The legal templates (`docs/legal/`), the subscription document and the legal sync, bundle and
  license-acceptance scripts.
- `presentations/`, `terraform-provider-vmrogue/`, `web/`, `e2e/`, `scripts/customer-docs/`,
  `scripts/zyvor-branding/`, `QUICK_REFERENCE.md`, `DEVELOPMENT.md`, and the PDF and welcome-page
  generation in the customer bundle.
