#!/usr/bin/env node
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const PAGES = resolve(ROOT, 'docs/customer/pages')
const { routes } = JSON.parse(readFileSync(resolve(ROOT, 'scripts/customer-docs/routes.json'), 'utf8'))
const purposes = JSON.parse(readFileSync(resolve(ROOT, 'scripts/customer-docs/page-purposes.json'), 'utf8'))

function catDir(category) {
  return category
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'other'
}

function slug(path) {
  return path.replace(/^#?\/?/, '').replace(/\//g, '-') || 'dashboard'
}

function guideTemplate({ title, category, purpose }) {
  return `# ${title}

## Purpose

${purpose}

## When to use it

- Open this page when the job matches the purpose above
- Prefer **Mission Control** (Overview) first if you are unsure where to start
- Confirm API auth and namespace scope if lists look empty

## How to get there

- Left rail: **${category} → ${title}**
- Use the top-bar **Search** to jump here directly

## What you can do

1. Open this page from the left rail and wait for live API data from the Veyron server.
2. Use filters (namespace, label, status) when the page provides them.
3. Select a row to open the Inspector for detail, then jump to related pages (console, snapshots).
4. For mutating actions (create VM, migrate, delete): review impact and role gates first.

If the page stays empty, check API health, auth (\`VEYRON_API_KEY\` / OIDC), KubeVirt CRDs, and that the workload namespace is selected.

## Related pages

- [Getting Started](../../getting-started.md)
- [Mission Control](../overview/mission.md)
- [Virtual machines](../compute/vms.md)
- [Page index](../../PAGE_INDEX.md)
`
}

let written = 0
let skipped = 0
for (const r of routes) {
  const dir = catDir(r.category)
  const file = join(PAGES, dir, `${slug(r.path)}.md`)
  mkdirSync(dirname(file), { recursive: true })
  if (existsSync(file)) {
    skipped++
    continue
  }
  writeFileSync(
    file,
    guideTemplate({
      title: r.label,
      path: r.path,
      category: r.category,
      purpose: purposes[r.path] || `${r.label} page.`,
    }),
  )
  written++
}
console.log(`Wrote ${written} guides (skipped existing ${skipped})`)
