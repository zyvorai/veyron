# Veyron — Customer Documentation

**Veyron** is the Kubernetes-native VM command center for **KubeVirt** — a Rust/Axum API, a CLI,
and a React console for managing VM lifecycle, storage, networking, and day-2 operations.

| You want to… | Open |
|--------------|------|
| Install and log in | [Getting Started](getting-started.md) |
| Learn the console shell | [Using the Console](using-the-console.md) |
| Follow a page, step by step | [Page-by-page guides](pages/README.md) |
| Look up any page by id | [Complete page index](PAGE_INDEX.md) |
| Deploy, auth, ports | [Admin basics](admin-basics.md) |
| Set up SSO | [Setting Up SSO](sso-setup.md) |
| Multi-page jobs | [Common workflows](workflows.md) |
| Capability map | [Feature Guide](../veyron-customer-feature-guide.md) |

## Printable PDFs

```bash
node scripts/customer-docs/build-customer-pdfs.mjs
```

Output lands in [`pdf/`](pdf/):

| PDF | Contents |
|-----|----------|
| `Veyron-Customer-README.pdf` | This overview |
| `Veyron-Getting-Started.pdf` | Access, login, console basics, workflows |
| `Veyron-Page-by-Page.pdf` | Complete page manual |
| `Veyron-Admin-Basics.pdf` | Deploy, auth, ports |

## Product at a glance

```text
  Surfaces   →  CLI · TUI · React console (/console) · REST API
  GitOps     →  VeyronVM CRD · Go operator · Helm
  KubeVirt   →  lifecycle · snapshots · migrate · consoles
```

## Support surfaces (quick map)

| Need | Open in the console |
|------|----------------------|
| Fleet overview | **Mission Control** (Overview) |
| VMs | **Virtual machines** (Compute) |
| Consoles | **ConsoleHub** (Overview) |
| Templates | **Template Foundry** (Compute) |
| Storage & Ceph | **Storage**, **Atlas** (Storage & network) |
| Settings | **Settings** (System) |

The console lives at `/console` and is a single left-rail navigation shell (no hash routing) —
see [Using the Console](using-the-console.md).

---

*ZyvorAI Labs · [zyvor.dev](https://zyvor.dev) · Veyron*
