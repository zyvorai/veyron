// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
//
// Headless console-error sweep of a deployed Veyron dashboard.
//
// Drives real Chrome (via playwright-core, channel: 'chrome') against the live
// dashboard, navigates each SPA page (location.hash routing) and reports any
// console errors, uncaught page errors, and failed (non-data:) requests per page.
// Exits non-zero when any page has errors, so it doubles as a CI regression gate.
//
// This is a repo-committed version of the throwaway scripts that caught the CSP
// data:-image block, the `lastOverview` null `.nodes` crash, and the
// /api/v1/health/ready 404 on the Nodes page. Reuse it after dashboard edits.
//
// Run it via the wrapper (installs playwright-core + uses installed Chrome):
//   ./scripts/dashboard-console-check.sh --host 80.79.5.173
// Or directly (PW_CORE = path to a playwright-core install):
//   PW_CORE=/path/to/node_modules/playwright-core \
//     node scripts/dashboard-console-check.cjs --host H --port 30151 --key Admin@321
//
// Flags:
//   --host H          dashboard host            (default $VEYRON_HOST or 127.0.0.1)
//   --port P          HTTPS NodePort            (default $VEYRON_PORT or 30151)
//   --key K           X-API-Key                 (default $VEYRON_API_KEY or Admin@321)
//   --pages a,b,c     comma-separated page ids  (default: the built-in sweep list)
//   --page X          focus a single page (implies a screenshot of it)
//   --screenshot DIR  save full-page PNGs per page into DIR
//   --scheme http|https                          (default https)

const pwPath = process.env.PW_CORE || 'playwright-core';
const { chromium } = require(pwPath);

function arg(name, def) {
  const i = process.argv.indexOf('--' + name);
  return i !== -1 && process.argv[i + 1] ? process.argv[i + 1] : def;
}
function has(name) {
  return process.argv.includes('--' + name);
}

const HOST = arg('host', process.env.VEYRON_HOST || '127.0.0.1');
const PORT = arg('port', process.env.VEYRON_PORT || '30151');
const KEY = arg('key', process.env.VEYRON_API_KEY || 'Admin@321');
const SCHEME = arg('scheme', 'https');
const SHOT_DIR = arg('screenshot', has('screenshot') ? '.' : null);
const FOCUS = arg('page', null);

// Real dashboard page ids (SPA #hash routes). Keep in sync with dashboard.html
// `id="page-*"` blocks; unknown ids just render empty and are flagged.
const DEFAULT_PAGES = [
  'dashboard', 'vms', 'snapshots', 'integrations', 'storage', 'nodes', 'pods',
  'topology', 'monitoring', 'backups', 'costs', 'security', 'soc', 'gitops',
  'network-intel', 'network-policies', 'cilium', 'alerts', 'events', 'reactor',
  'compliance', 'chaos', 'catalog',
];
const PAGES = FOCUS ? [FOCUS] : arg('pages', '').split(',').filter(Boolean).length
  ? arg('pages', '').split(',').map(s => s.trim()).filter(Boolean)
  : DEFAULT_PAGES;

const BASE = `${SCHEME}://${HOST}:${PORT}`;

(async () => {
  const browser = await chromium.launch({
    headless: true, channel: 'chrome', args: ['--ignore-certificate-errors'],
  });
  const ctx = await browser.newContext({ ignoreHTTPSErrors: true, viewport: { width: 1600, height: 1050 } });
  await ctx.addInitScript((k) => { try { localStorage.setItem('veyron_api_key', k); } catch (e) {} }, KEY);
  const page = await ctx.newPage();

  const perPage = {};
  let current = 'boot';
  const push = (m) => { (perPage[current] = perPage[current] || []).push(m); };
  page.on('console', (m) => {
    if (m.type() !== 'error') return;
    let t = m.text();
    if (/Content Security Policy/.test(t)) t = 'CSP: ' + ((t.match(/directive: "([^"]+)"/) || [])[1] || 'violation');
    push('CONSOLE ' + t.slice(0, 180));
  });
  page.on('pageerror', (e) => push('PAGEERROR ' + e.message.slice(0, 180)));
  page.on('requestfailed', (r) => {
    if (r.url().startsWith('data:')) return;
    push('REQFAIL ' + (r.failure() ? r.failure().errorText : '') + ' ' + r.url().slice(0, 90));
  });

  await page.goto(`${BASE}/dashboard?token=${encodeURIComponent(KEY)}#dashboard`,
    { waitUntil: 'networkidle', timeout: 60000 }).catch((e) => push('GOTO ' + e.message));
  await page.waitForTimeout(3000);

  for (const p of PAGES) {
    current = p;
    await page.evaluate((pg) => { try { if (typeof navigate === 'function') navigate(pg); } catch (e) {} }, p);
    await page.waitForTimeout(2500);
    const rendered = await page.evaluate((pg) => {
      const el = document.getElementById('page-' + pg);
      return el ? (el.innerText || '').trim().length : -1;
    }, p).catch(() => -2);
    perPage[p] = perPage[p] || [];
    if (rendered <= 0) push('RENDER empty or #page-' + p + ' missing (len=' + rendered + ')');
    if (SHOT_DIR) {
      await page.screenshot({ path: `${SHOT_DIR}/dashboard-${p}.png`, fullPage: true }).catch(() => {});
    }
  }

  console.log(`=== Veyron dashboard console sweep · ${BASE} ===`);
  let clean = 0, pagesWithErrors = 0;
  for (const p of ['boot', ...PAGES]) {
    const errs = (perPage[p] || []).filter((v, i, a) => a.indexOf(v) === i);
    if (!errs.length) { clean++; continue; }
    pagesWithErrors++;
    console.log(`\n[${p}]`);
    errs.slice(0, 8).forEach((e) => console.log('  - ' + e));
    if (errs.length > 8) console.log(`  … +${errs.length - 8} more`);
  }
  console.log(`\n${clean}/${PAGES.length + 1} groups clean` + (SHOT_DIR ? ` · screenshots in ${SHOT_DIR}` : ''));
  await browser.close();
  process.exit(pagesWithErrors > 0 ? 1 : 0);
})().catch((e) => { console.error('FATAL', e); process.exit(2); });
