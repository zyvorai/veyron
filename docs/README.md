# Documentation Index

This directory contains focused guides for VMRogue features and operations. Start here when looking for a specific topic.

**Repository layout:** default development branch is **`main`** (Rust CLI + web API). The historical Go-era tree is preserved on branch **`main-go`** for reference only.

## Platform positioning

- [`VMROGUE_HYPERSDK_BOUNDARY.md`](VMROGUE_HYPERSDK_BOUNDARY.md): VMRogue (KubeVirt ops) vs HyperSDK (migration/portability)
- [`OIDC_SSO.md`](OIDC_SSO.md): enterprise SSO / OIDC configuration

## Getting Started

- [`../README.md`](../README.md): project overview, install, API/dashboard, deployment options
- [`PACKAGE_BINARY_REMOTE.md`](PACKAGE_BINARY_REMOTE.md): build a Linux amd64 client tarball on a remote host (`scripts/package-binary-remote.sh`)
- [`../QUICK_REFERENCE.md`](../QUICK_REFERENCE.md): high-signal command cheatsheet
- [`../DEVELOPMENT.md`](../DEVELOPMENT.md): architecture map and contributor workflows
- [`../CONTRIBUTING.md`](../CONTRIBUTING.md): pull requests, style, and CI expectations
- [`../SECURITY.md`](../SECURITY.md): vulnerability reporting
- [`../CHANGELOG.md`](../CHANGELOG.md): release notes

## Core Operations

- [`SNAPSHOTS.md`](SNAPSHOTS.md): snapshot and restore workflows
- [`DISK_MANAGEMENT.md`](DISK_MANAGEMENT.md): storage and disk operations
- [`NETWORK_MANAGEMENT.md`](NETWORK_MANAGEMENT.md): VM networking operations
- [`DEVELOPER_VM_ACCESS.md`](DEVELOPER_VM_ACCESS.md): SSH and `virtctl` access patterns
- [`OS_TEMPLATES.md`](OS_TEMPLATES.md): template catalog and guidance

## TUI and UX

- [`INTERACTIVE_TUI.md`](INTERACTIVE_TUI.md): interactive TUI guide
- [`INTERACTIVE_TUI_README.md`](INTERACTIVE_TUI_README.md): TUI overview and usage
- [`TUI_FEATURES_DEMO.md`](TUI_FEATURES_DEMO.md): walkthrough of notable TUI flows
- [`THEME.md`](THEME.md): color/theme behavior and customization notes

## Advanced and Feature Guides

- [`ADVANCED_FEATURES.md`](ADVANCED_FEATURES.md): advanced workflows and capabilities
- [`INNOVATIVE_FEATURES.md`](INNOVATIVE_FEATURES.md): differentiating feature set
- [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md): capability map and implementation status

## Windows on KubeVirt

- [`WINDOWS_KUBEVIRT_PRODUCTION.md`](WINDOWS_KUBEVIRT_PRODUCTION.md): production runbook (golden image, cloudbase-init, sysprep)
- [`WINDOWS_PACKER_GITOPS_PIPELINE.md`](WINDOWS_PACKER_GITOPS_PIPELINE.md): image pipeline with Packer, CDI, and GitOps integration

## Helm and monitoring

- [`../charts/vmrogue-monitoring/README.md`](../charts/vmrogue-monitoring/README.md): optional Prometheus/Grafana/Alertmanager umbrella chart

When you add or change user-visible behavior, update this index if you introduce a new top-level guide under `docs/`.

