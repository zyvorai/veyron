# Veyron — Customer Documentation

**Veyron** is the Kubernetes-native VM command center for **KubeVirt** — declarative VM builder, OS templates, blueprints, browser consoles, GitOps export, and Mission Control with 65+ dashboard pages.

| You want to… | Open |
|--------------|------|
| Install and log in | [Getting Started](getting-started.md) |
| Learn the shell | [Using the Dashboard](using-the-dashboard.md) |
| Follow a page, step by step | [Page-by-page guides](pages/README.md) |
| Look up any screen by route | [Complete page index](PAGE_INDEX.md) |
| Deploy, auth, ports | [Admin basics](admin-basics.md) |
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
| `Veyron-Getting-Started.pdf` | Access, login, dashboard basics, workflows |
| `Veyron-Page-by-Page.pdf` | Complete page manual |
| `Veyron-Admin-Basics.pdf` | Deploy, auth, ports |

## Product at a glance

```text
  Surfaces   →  CLI · TUI · Mission Control · REST API
  GitOps     →  VeyronVM CRD · Go operator · Helm
  KubeVirt   →  lifecycle · snapshots · migrate · consoles
```

## Support surfaces (quick map)

| Need | Typical path |
|------|----------------|
| Mission Control | `/dashboard` |
| VMs | `/dashboard#vms` |
| Consoles | `/dashboard#console-hub` |
| Templates / blueprints | `/dashboard#app-store`, `/dashboard#blueprint-studio` |
| Ask Zyra | `/dashboard#ask-zyra` |
| Settings | `/dashboard#settings` |

---

*ZyvorAI Labs · [zyvor.dev](https://zyvor.dev) · Veyron*
