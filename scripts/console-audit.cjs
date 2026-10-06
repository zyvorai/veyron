// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//
// Headless audit of the React console at /console. Driven by console-audit.sh.
// Prints one JSON object: { width, pages, findings: {page: {...}}, consoleErrors: {page: [...]} }.
'use strict';

const fs = require('fs');
const path = require('path');
const { chromium } = require(process.env.PW_CORE || 'playwright-core');

const [host, port, width, only] = process.argv.slice(2);
const apiKey = process.env.VEYRON_API_KEY || 'Admin@321';
const base = `https://${host}:${port}/console`;
const settleMs = Number(process.env.VEYRON_AUDIT_SETTLE_MS || 2500);

/** Page ids in menu order, read from the console's own NAV definition. */
function pageIds() {
  const src = fs.readFileSync(path.join(__dirname, '..', 'frontend', 'src', 'resources.js'), 'utf8');
  const start = src.indexOf('export const NAV');
  const end = src.indexOf('export const PAGE_ORDER');
  const block = src.slice(start, end > start ? end : undefined);
  const ids = [...block.matchAll(/\[\s*'([a-z0-9-]+)',\s*(?:'|null)/g)].map((m) => m[1]);
  const uniq = [...new Set(ids)];
  if (!only) return uniq;
  const want = new Set(only.split(','));
  return uniq.filter((p) => want.has(p));
}

/** Runs in the page: geometry problems a console check can't see. */
function layoutProbe() {
  const out = { overflow: [], offscreen: [], overlaps: [], clipped: [], empty: [] };
  const vw = window.innerWidth;
  const label = (el) => {
    const id = el.id ? `#${el.id}` : '';
    const cls = typeof el.className === 'string' && el.className.trim()
      ? `.${el.className.trim().split(/\s+/).slice(0, 2).join('.')}` : '';
    const text = (el.innerText || el.getAttribute('aria-label') || '').trim().replace(/\s+/g, ' ').slice(0, 40);
    return `${el.tagName.toLowerCase()}${id}${cls}${text ? ` "${text}"` : ''}`;
  };
  const visible = (el) => {
    const r = el.getBoundingClientRect();
    if (r.width < 2 || r.height < 2) return false;
    const s = getComputedStyle(el);
    return s.visibility !== 'hidden' && s.display !== 'none' && Number(s.opacity) > 0.05;
  };
  const scrollsX = (el) => {
    for (let p = el.parentElement; p; p = p.parentElement) {
      const o = getComputedStyle(p).overflowX;
      if ((o === 'auto' || o === 'scroll') && p.scrollWidth > p.clientWidth) return true;
    }
    return false;
  };

  const doc = document.scrollingElement;
  if (doc.scrollWidth > doc.clientWidth + 1) out.overflow.push(`document ${doc.scrollWidth}px > ${doc.clientWidth}px`);

  const main = document.querySelector('main') || document.querySelector('.stage') || document.body;
  if ((main.innerText || '').trim().length < 20) out.empty.push('main has no content');

  const nav = document.querySelector('nav[aria-label="Global"]') || document.querySelector('nav');
  const navBottom = nav ? nav.getBoundingClientRect().bottom : 0;
  const stage = document.querySelector('.stage');
  const atTop = !stage || stage.scrollTop === 0;

  for (const el of main.querySelectorAll('button, a, input, select, textarea, h1, h2, h3, td, th, [role="button"]')) {
    if (!visible(el) || el.closest('[role="dialog"]')) continue;
    const r = el.getBoundingClientRect();
    if (r.right > vw + 1 && !scrollsX(el)) out.offscreen.push(`${label(el)} right=${Math.round(r.right)}`);
    if (atTop && nav && !nav.contains(el) && r.top < navBottom - 1 && r.bottom > 0 && getComputedStyle(el).position !== 'fixed') {
      out.overlaps.push(`${label(el)} top=${Math.round(r.top)} under nav (bottom=${Math.round(navBottom)})`);
    }
  }

  for (const el of document.querySelectorAll('body *')) {
    const s = getComputedStyle(el);
    if (!(s.overflowX === 'hidden' || s.overflowX === 'clip') || el.scrollWidth <= el.clientWidth + 2 || !visible(el)) continue;
    const box = el.getBoundingClientRect();
    const cut = [...el.querySelectorAll('button, a, input, select')].find((c) => {
      if (!visible(c)) return false;
      const r = c.getBoundingClientRect();
      return r.left < box.right - 4 ? r.right > box.right + 2 : true;
    });
    if (cut) out.clipped.push(`${label(el)} hides ${label(cut)}`);
  }

  for (const k of Object.keys(out)) {
    out[k] = [...new Set(out[k])].slice(0, 8);
    if (!out[k].length) delete out[k];
  }
  return out;
}

(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const ctx = await browser.newContext({ ignoreHTTPSErrors: true, viewport: { width: Number(width), height: 900 } });
  await ctx.addInitScript((key) => {
    localStorage.setItem('veyron_auth_token', key);
    localStorage.setItem('veyron_auth_user', JSON.stringify({ username: 'audit', display_name: 'Audit', role: 'admin' }));
  }, apiKey);
  const page = await ctx.newPage();

  let current = 'boot';
  const consoleErrors = {};
  const apiErrors = {};
  const push = (map, msg) => { (map[current] ||= []).push(msg.slice(0, 300)); };
  page.on('console', (m) => { if (m.type() === 'error') push(consoleErrors, m.text()); });
  page.on('pageerror', (e) => push(consoleErrors, `uncaught: ${e.message}`));
  page.on('response', (r) => {
    if (r.url().includes('/api/') && r.status() >= 500) push(apiErrors, `${r.status()} ${new URL(r.url()).pathname}`);
  });

  await page.goto(`${base}#/mission`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.waitForTimeout(settleMs);

  const pages = pageIds();
  const findings = {};
  for (const id of pages) {
    current = id;
    await page.evaluate((h) => { window.location.hash = h; }, `#/${id}`);
    await page.waitForTimeout(settleMs);
    if (!(await page.evaluate(() => localStorage.getItem('veyron_auth_token')))) {
      findings[id] = { auth: ['console shows the sign-in screen; check VEYRON_API_KEY'] };
      break;
    }
    const f = await page.evaluate(layoutProbe);
    if (apiErrors[id]) f.api = [...new Set(apiErrors[id])].slice(0, 8);
    if (Object.keys(f).length) findings[id] = f;
  }

  await browser.close();
  const errs = Object.fromEntries(Object.entries(consoleErrors).map(([k, v]) => [k, [...new Set(v)].slice(0, 5)]));
  process.stdout.write(JSON.stringify({ width: Number(width), pages, findings, consoleErrors: errs }));
})().catch((e) => {
  process.stderr.write(`console-audit: ${e.stack || e}\n`);
  process.exit(4);
});
