# Veyron — Feature Guide

> **Kubernetes-native VM command center.**

Veyron turns verbose KubeVirt YAML into a single, opinionated command center for virtual machines on Kubernetes. It ships four surfaces over one Rust core — a CLI, a k9s-style TUI, a CloudOS web dashboard, and a REST/WebSocket API — plus a Go operator that makes VMs GitOps-native through the VeyronVM custom resource. From a 44-image template catalog and multi-VM blueprints to browser consoles, snapshots, live migration, SOC detections, and an AI copilot, Veyron covers the full VM lifecycle from create to retire.

**44** OS templates across 15 families · **40+** dashboard pages · **4** surfaces: CLI, TUI, Web, API · **8** resource profiles

This is the customer-facing feature reference. A print-ready PDF of the same content sits alongside this file. Generated from the product's actual capabilities.

## Contents

1. [VM Provisioning & Templates](#1-vm-provisioning-templates)
2. [Consoles & Guest Access](#2-consoles-guest-access)
3. [Snapshots & Data Protection](#3-snapshots-data-protection)
4. [Migration & High Availability](#4-migration-high-availability)
5. [Storage & Disks](#5-storage-disks)
6. [Networking](#6-networking)
7. [AI Copilot — Ask Zeus](#7-ai-copilot-—-ask-zeus)
8. [Security & Compliance](#8-security-compliance)
9. [Cost & FinOps](#9-cost-finops)
10. [Observability & Monitoring](#10-observability-monitoring)
11. [GitOps & Operator](#11-gitops-operator)
12. [Platform & Surfaces](#12-platform-surfaces)

## 1. VM Provisioning & Templates

_From a blank cluster to a running fleet without hand-writing manifests._

- **Declarative VM Builder** — Create KubeVirt VirtualMachines from the CLI, wizard, or dashboard with CPU, memory, disk, storage class, and cloud-init. — _Skip error-prone YAML — describe the VM you want and Veyron generates a validated manifest._
- **44-Image OS Catalog** — Launch from 44 curated templates spanning 15 OS families — Ubuntu, Fedora, Debian, RHEL, Rocky, AlmaLinux, Windows, and BSD. — _Battle-tested base images with sensible defaults for every common guest._
- **Multi-VM Blueprints** — Deploy whole stacks — LAMP, a Kubernetes cluster, 3-tier apps, CI/CD — as one dependency-ordered blueprint. — _Stand up an entire environment with a single deploy instead of provisioning VMs one at a time._
- **Resource Profiles** — Apply prebuilt sizing profiles (dev, test, prod, high-perf, microservice) or author your own with custom CPU topology. — _Consistent, right-sized VMs across teams without re-deciding cores and memory each time._
- **Interactive Wizard & Batch** — Walk through guided VM creation, or provision many VMs at once from a batch YAML/JSON file. — _One prompt-driven flow for newcomers, one file for bulk fleet builds._
- **Clone, Export & Dry-Run** — Duplicate an existing VM, export any VM as a manifest, or preview the generated resource before it touches the cluster. — _Reproduce known-good VMs and review exactly what will be applied first._

> Templates resolve from an embedded catalog for offline CLI use and from cluster VMTemplate/VMProfile CRDs for GitOps — the operator and CLI agree on names.

## 2. Consoles & Guest Access

_Reach any VM directly from the browser or terminal — no virtctl timeouts._

- **Browser VNC Console** — Open a graphical VNC session to any VM over a direct Kubernetes WebSocket, right in the dashboard. — _Full desktop access without VPNs, jump hosts, or virtctl port-forwards that time out._
- **Serial Console & Logs** — Attach to a VM's serial console and stream logs from its virt-launcher pod with follow and tail. — _Diagnose boot problems and kernel panics where a graphical console can't reach._
- **RDP Screen Sharing** — Expose and connect to Windows guests over RDP through the dashboard's Screen Sharing surface. — _Native Windows remote-desktop access managed from the same console as your Linux VMs._
- **SSH & Key Injection** — SSH into running VMs and inject public keys via KubeVirt accessCredentials, propagated through the guest agent or configDrive. — _Provision secure key-based access to a VM without rebuilding or manual guest edits._
- **Guest Agent Signals** — Read QEMU guest-agent data — in-guest filesystem usage, network info, and health signals — from CLI or dashboard. — _See what's actually happening inside the guest, not just what Kubernetes reports outside it._

## 3. Snapshots & Data Protection

_Application-consistent point-in-time copies, backups, and disaster-recovery plans._

- **VM Snapshots** — Create, list, inspect, delete, and restore KubeVirt VirtualMachineSnapshots, including in-place restore. — _Roll a VM back to a known-good state in seconds before risky changes._
- **App-Consistent Freeze** — Quiesce guest filesystems with fsfreeze via the QEMU guest agent before snapshotting, then thaw them. — _Snapshots that databases and apps can actually be restored from — not just crash-consistent copies._
- **VM Backups** — Take full, incremental, or differential backups with gzip/zstd/lz4 compression and integrity verification. — _Portable, verifiable VM copies with scheduled full/incremental rotation._
- **Backup Schedules** — Define hourly, daily, weekly, or monthly backup schedules scoped to specific VMs or the whole fleet. — _Set-and-forget protection so no VM drifts out of your backup window._
- **Velero & DR Plans** — Surface Velero Backup/Restore status and author disaster-recovery plans with dry-run execution and failover. — _Cluster-level recovery readiness alongside per-VM backups, tested before you need it._

## 4. Migration & High Availability

_Move workloads between nodes and keep critical VMs running through maintenance._

- **Live & Offline Migration** — Migrate VMs across nodes using live, offline, or post-copy strategies, with a plan preview before executing. — _Rebalance load or evacuate hardware without downtime for migratable workloads._
- **Node Evacuation & Drain** — Cordon, uncordon, and evacuate nodes with controlled parallel migrations, timeouts, and status tracking. — _Take a node down for maintenance while VMs safely relocate themselves._
- **VM High Availability** — Configure per-VM HA priority and eviction strategy (live-migrate, shutdown, none) and view HA status. — _Guarantee that your critical VMs are the ones that survive node failures._
- **Migration Tracking & Policies** — Watch live migration progress and manage KubeVirt migration policies from the scheduling surface. — _Know exactly where a migration is and enforce fleet-wide migration guardrails._

## 5. Storage & Disks

_Grow, hotplug, and monitor VM disks without downtime._

- **Online Disk Expansion** — Expand VM disks and resize PVCs, with a plan preview and generated in-guest filesystem-grow scripts. — _Add capacity to a running VM instead of rebuilding it when it fills up._
- **Volume Hotplug** — Attach and detach PVCs on a running VM via virtctl addvolume/removevolume. — _Give a live VM more storage on demand — no restart required._
- **Disk Health & Usage** — Report per-VM disk health and usage statistics, sorted by utilization, size, or free space. — _Spot the disks about to fill before they take an application down._
- **Storage Pools & PVCs** — Browse PVCs, storage classes, and storage pools across the cluster from the Storage page. — _One inventory of every volume backing your VM fleet._

## 6. Networking

_Per-VM connectivity, policy enforcement, and network intelligence._

- **Interfaces & Bandwidth** — List VM network interfaces, monitor per-interface bandwidth in watch mode, and analyze traffic and top talkers. — _See which VM is saturating the network and on which interface._
- **Multus Multi-Network** — Attach additional Multus networks to a VM and manage NetworkAttachmentDefinitions. — _Give VMs the multiple NICs and VLANs that real workloads need._
- **Network Policies & Egress** — List and inspect NetworkPolicies, and apply a default internet-egress policy on VM create (opt-out available). — _VMs come locked down by default instead of wide open to the internet._
- **Cilium & PacketWolf** — View Cilium agent status and policies, with an optional PacketWolf network-intelligence integration. — _Deeper network visibility for clusters running Cilium or the PacketWolf brain._

## 7. AI Copilot — Ask Zeus

_Plain-language diagnosis with cluster evidence and one-click fixes._

- **Ask Zeus Assistant** — Ask VM questions in natural language (⌘J) and get root cause, cluster evidence, and recommended actions. — _Skip the stack traces — describe the problem and get a grounded answer._
- **Veyron Doctor** — Generate a per-VM health score with prioritized issues drawn from live KubeVirt/Kubernetes state. — _A quick triage read on any VM before you dig into it manually._
- **Scheduling Explainer** — Translate a Pending or Unschedulable VM into a human-readable reason and fix. — _Understand why a VM won't start without decoding scheduler events yourself._
- **Error Explainer & YAML Builder** — Turn cryptic K8s/KubeVirt errors into plain English and generate cluster-validated VirtualMachine YAML. — _Get unblocked on obscure errors and hand off valid manifests in one step._
- **Lens Advisors** — Purpose-built advisors — Network Lens, Storage Doctor, Backup Advisor, Cost Advisor, Security Sentinel — inspect a VM or the whole fleet. — _Domain-specific expertise on connectivity, bloat, coverage gaps, spend, and exposure._

> The v1 assistant is deterministic and works without any external LLM by composing real cluster data; connect an OpenAI-compatible model (VEYRON_AI_*) to enable richer conversational answers.

## 8. Security & Compliance

_Detections, hardening, policy enforcement, and an audit trail for the VM fleet._

- **SOC & Detections** — Run a security operations center over VM activity — detections, threat hunts, attack-surface mapping, and SIEM export. — _Turn VM events into actionable security signal, pushed to Elastic or your SIEM._
- **Hardening Profiles** — Scan VMs and apply hardening baselines — CIS, STIG, PCI-DSS, NIST — with verify-only mode. — _Bring VMs up to a recognized security standard on demand._
- **Compliance Frameworks** — Check VMs against PCI-DSS, HIPAA, SOC 2, ISO 27001, GDPR, NIST, and CIS, and produce reports. — _Evidence for auditors without manual per-VM checklists._
- **Policy Enforcement** — Define VeyronPolicy CRDs with CEL deny rules so violating VMs never make it into the cluster. — _Stop misconfigured or non-compliant VMs at admission instead of cleaning up after._
- **RBAC, Audit & SSO** — Inspect cluster RBAC roles and bindings, review a namespace-aware audit trail, and sign in via OIDC SSO. — _Know who did what, and gate access to identity your org already trusts._

## 9. Cost & FinOps

_Understand, budget, and trim what your VM fleet spends._

- **Cost Analysis** — Break down fleet and per-VM cost by namespace, team, or project over configurable time windows. — _See where the money goes across every VM you run._
- **Budgets & Alerts** — Create budgets scoped to global, namespace, team, or project with configurable alert thresholds. — _Get warned before spend blows past what you planned for._
- **Waste & Optimization** — Surface idle, oversized, storage, and snapshot waste, and get prioritized cost-optimization recommendations. — _Reclaim spend from the VMs and volumes nobody is using._
- **Cost Forecasting** — Project 7/30/90-day costs and compare them against a budget. — _Plan next quarter's VM spend before it arrives._

## 10. Observability & Monitoring

_Live metrics, hotspots, SLOs, and stack discovery across the fleet._

- **Live VM Monitoring** — Stream live CPU/memory/disk stats for a VM, compare multiple VMs, and rank the top resource consumers. — _Watch a VM in real time and instantly find the fleet's heaviest workloads._
- **Hotspots & Heatmap** — Aggregate workload requests and render a resource heatmap to expose CPU and memory pressure. — _Spot the nodes and VMs running hot at a glance._
- **SLOs & Alerts** — Track fleet availability SLO objectives and read namespace-aware warning-event narratives and alerts. — _Turn raw Kubernetes events into an availability story you can act on._
- **Stack Discovery** — Auto-detect the observability stack — Prometheus, Grafana, Loki, Jaeger — and deep-link into it. — _Jump from Veyron straight into the monitoring tools you already run._

## 11. GitOps & Operator

_CR-native VMs, drift detection, and infrastructure-as-code delivery._

- **VeyronVM Operator** — A Go controller-runtime operator reconciles VeyronVM and VeyronBlueprint custom resources into live KubeVirt VMs. — _Manage VMs the Kubernetes-native way — declare them and let the operator converge._
- **GitOps Export** — Export every VM in a namespace as VeyronVM CRD manifests with a single command. — _Capture a running fleet as version-controlled YAML for Argo CD or Flux._
- **Drift Detection** — Compare live VMs against their desired CRD state and surface drift as badges, filters, and operator insights. — _Know the moment a VM diverges from what Git says it should be._
- **Helm & Terraform** — Install via Helm charts and provision through a dedicated Terraform provider. — _Fit Veyron into whatever IaC pipeline your platform team already uses._
- **Catalog CRDs** — Publish embedded templates and profiles to the cluster as VMTemplate/VMProfile CRDs and keep them in sync. — _One authoritative, GitOps-managed catalog shared by CLI, operator, and dashboard._

## 12. Platform & Surfaces

_One Rust core, four ways to drive it, across many clusters and tenants._

| Surface | Best for |
|---|---|
| Web dashboard (CloudOS) | Visual operations, consoles, guided fixes |
| CLI (veyron) | Scripting, automation, CI/CD pipelines |
| TUI | Fast keyboard-driven cluster navigation |
| REST / WebSocket API | Custom tooling and integrations |

- **CloudOS Dashboard** — A macOS-style Mission Control web shell with 40+ pages, Spotlight (⌘K), a dock, Finder sidebar, and Fix-it flows. — _A polished, discoverable home for operators who'd rather click than memorize flags._
- **TUI & CLI** — A k9s-style terminal UI with themes and dialogs, plus a full-featured headless CLI for scripting. — _Fast keyboard-driven control at the terminal, and automatable commands for pipelines._
- **REST & WebSocket API** — A Rust Axum API server (49 routes) with embedded OpenAPI and WebSocket console tunnels. — _Build your own tooling on the same API the dashboard and CLI use._
- **Multi-Cluster & Tenancy** — Switch between kubeconfig contexts from a cluster bar and carve the fleet into tenant workspaces with quotas and billing tags. — _Operate many clusters and isolate teams from a single control plane._
- **Automation & Integrations** — Define automation rules (manual, schedule, event, metric triggers) and probe optional integrations from one page. — _Codify routine day-2 responses and see which backends are wired at a glance._

## Getting started

1. **Build and install** — Clone the repo and run cargo build --release to produce the veyron binary.
2. **Create your first VM** — Run veyron create --template ubuntu-22.04 --name web-01 to launch a VM from a template.
3. **Deploy a stack** — Run veyron blueprint deploy lamp --namespace dev to stand up a full multi-VM blueprint.
4. **Open Mission Control** — Run veyron serve and browse to https://localhost:8080 for the CloudOS dashboard and API.
5. **Go GitOps** — Install the operator from charts/veyron-operator and run veyron gitops-export to capture VMs as VeyronVM manifests.

> **Good to know:** Veyron is a control plane over KubeVirt and Kubernetes and depends on those APIs being present. Several dashboard pages are honest about their limits: costs are estimates unless OpenCost is wired, security posture is configuration-derived rather than scanner-backed, and forecasting, observability counts, and scheduling metrics are heuristic. Optional integrations (OpenCost, Trivy, Prometheus, PacketWolf, Elastic/SIEM, an OpenAI-compatible LLM) unlock deeper data when configured.

---
_Veyron is developed by ZyvorAI Labs. Contact **info@zyvor.dev** · Proprietary & Confidential._
