#!/usr/bin/env node
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const CUSTOMER = resolve(ROOT, 'docs/customer')
const SITE = resolve(process.argv[2] ?? resolve(ROOT, '../zyvor-web'))
const PRODUCT = process.env.CUSTOMER_DOCS_PRODUCT || 'Veyron'
const SLUG = PRODUCT.toLowerCase()
const TARGET = join(SITE, `docs/${SLUG}-manual`)
const PDF_TARGET = join(SITE, `static/downloads/${SLUG}-docs`)

if (!existsSync(join(SITE, 'docusaurus.config.ts'))) {
  console.error(`ERROR: ${SITE} is not zyvor-web`)
  process.exit(1)
}

const TOP_LEVEL_POSITION = {
  'index.md': 1,
  'getting-started.md': 2,
  'using-the-dashboard.md': 3,
  'workflows.md': 4,
  'admin-basics.md': 5,
  'sso-setup.md': 6,
  'page-index.md': 8,
}

const REPO_ONLY = new RegExp(
  [
    '(\\.\\./)+(handbook|guides|architecture|admin-guide|user-guide|getting-started|developer-guide|legal|client)/',
    `${SLUG}-customer-feature-guide`,
    'DEPLOYMENT_GUIDE',
    'AIRGAP_INSTALL',
    'CLI_GUIDE',
  ].join('|'),
)

function renameTarget(p) {
  return p.replace(/(^|\/)README\.md/, '$1index.md').replace(/(^|\/)PAGE_INDEX\.md/, '$1page-index.md')
}

function transformLinks(md) {
  return md.replace(/\[([^\]]*)\]\(([^)]+)\)/g, (full, text, target) => {
    const t = target.trim()
    if (/^(https?:|mailto:|#)/.test(t)) return full
    if (REPO_ONLY.test(t)) return text
    return `[${text}](${renameTarget(t)})`
  })
}

function rewriteIndexPdfSection(md) {
  const pdfNames = [
    `${PRODUCT}-Customer-README`,
    `${PRODUCT}-Getting-Started`,
    `${PRODUCT}-Page-by-Page`,
    `${PRODUCT}-Admin-Basics`,
  ]
  let out = md.replace(
    /```bash\nnode scripts\/customer-docs\/build-customer-pdfs\.mjs\n```\n\nOutput lands in \[`pdf\/`\]\(pdf\/\):/,
    'Prefer paper or offline reading? Download the print-ready PDFs:',
  )
  for (const name of pdfNames) {
    out = out.replaceAll(`\`${name}.pdf\``, `[${name.replace(`${PRODUCT}-`, '').replace(/-/g, ' ')} (PDF)](/downloads/${SLUG}-docs/${name}.pdf)`)
  }
  return out.replace(/\nAlso available:[^\n]*\n/, '\n')
}

function frontMatter(fields) {
  const lines = ['---']
  for (const [k, v] of Object.entries(fields)) lines.push(`${k}: ${typeof v === 'string' ? JSON.stringify(v) : v}`)
  lines.push('---', '', '')
  return lines.join('\n')
}

function walk(dir) {
  const out = []
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry)
    if (statSync(full).isDirectory()) out.push(...walk(full))
    else out.push(full)
  }
  return out
}

rmSync(TARGET, { recursive: true, force: true })
mkdirSync(TARGET, { recursive: true })
mkdirSync(PDF_TARGET, { recursive: true })

let written = 0
for (const file of walk(CUSTOMER)) {
  const rel = relative(CUSTOMER, file)
  if (rel.startsWith('pdf/') || !rel.endsWith('.md')) continue
  const target = join(TARGET, renameTarget(rel))
  mkdirSync(dirname(target), { recursive: true })
  let body = transformLinks(readFileSync(file, 'utf8'))
  const targetName = renameTarget(rel)
  if (targetName === 'index.md') {
    body = rewriteIndexPdfSection(body)
    body = frontMatter({ title: `${PRODUCT} Manual`, sidebar_position: 1, slug: `/${SLUG}-manual` }) + body
  } else if (targetName === 'pages/index.md') {
    body = frontMatter({ title: 'Page-by-page guides', sidebar_position: 1 }) + body
  } else if (TOP_LEVEL_POSITION[targetName]) {
    body = frontMatter({ sidebar_position: TOP_LEVEL_POSITION[targetName] }) + body
  }
  writeFileSync(target, body)
  written++
}

writeFileSync(
  join(TARGET, 'pages/_category_.json'),
  JSON.stringify({ label: 'Page-by-page guides', position: 7, collapsed: true, key: `${SLUG}-manual-pages` }, null, 2) + '\n',
)

const pagesDir = join(TARGET, 'pages')
if (existsSync(pagesDir)) {
  let i = 2
  for (const dir of readdirSync(pagesDir).sort()) {
    const full = join(pagesDir, dir)
    if (!statSync(full).isDirectory()) continue
    const label = dir.replace(/-/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
    writeFileSync(join(full, '_category_.json'), JSON.stringify({ label, position: i++, collapsed: true, key: `${SLUG}-manual-pages-${dir}` }, null, 2) + '\n')
  }
}

let pdfs = 0
const pdfDir = join(CUSTOMER, 'pdf')
if (existsSync(pdfDir)) {
  for (const f of readdirSync(pdfDir)) {
    if (!f.endsWith('.pdf')) continue
    cpSync(join(pdfDir, f), join(PDF_TARGET, f))
    pdfs++
  }
}

console.log(`Synced ${written} markdown files -> ${TARGET}`)
console.log(`Copied ${pdfs} PDFs -> ${PDF_TARGET}`)
