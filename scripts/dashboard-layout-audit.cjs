// Layout audit — sweeps every dashboard page for the bug class found in the
// Dynamic Island report: content colliding with fixed chrome, and containers
// that silently clip their own children instead of scrolling/wrapping.
//
//   PW_CORE=… node layout-audit.cjs <host> [port] [viewportW] [tier]
const { chromium } = require(process.env.PW_CORE);

const HOST = process.argv[2];
const PORT = process.argv[3] || '30151';
const VW = parseInt(process.argv[4] || '1920', 10);
const TIER = process.argv[5] || 'advanced';
const KEY = process.env.VEYRON_API_KEY || 'Admin@321';
const THEME = process.env.VEYRON_TEST_THEME || 'light';
const BASE = `https://${HOST}:${PORT}`;

const PAGES = [
  'dashboard', 'vms', 'snapshots', 'nodes', 'pods', 'storage', 'events', 'crds',
  'blueprint-studio', 'policies', 'insights', 'actions', 'costs', 'security',
  'monitoring', 'workloads', 'alerts', 'audit', 'soc', 'notifications', 'helm',
  'operators', 'custom-resources', 'slo', 'chaos', 'rbac', 'quotas', 'ingress',
  'hpa', 'backups', 'catalog', 'app-store', 'console-hub', 'stack-health',
  'vm-capsule', 'network-intel', 'settings', 'integrations', 'metrics',
  'mission-control', 'topology', 'dependencies', 'autoscaler', 'forecasting',
  'gitops', 'ask-zyra', 'scheduling', 'cilium', 'observability', 'performance',
  'webhooks', 'compliance', 'dr', 'heatmap', 'custom-dashboards',
  'network-policies', 'images', 'traces', 'logs', 'incidents', 'reactor',
  'fleet-constellation', 'gallery-wall', 'vcentre', 'tenants',
];

const AUDIT = () => {
  const out = { overlaps: [], clipped: [], overflow: [], offscreen: [] };
  const desc = (el) => {
    const id = el.id ? '#' + el.id : '';
    const cls = (el.className && el.className.toString ? el.className.toString() : '').trim().split(/\s+/).slice(0, 3).join('.');
    return el.tagName.toLowerCase() + id + (cls ? '.' + cls : '');
  };
  const vis = (el) => {
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden' || cs.opacity === '0') return false;
    const r = el.getBoundingClientRect();
    return r.width > 1 && r.height > 1;
  };

  // 1. Content colliding with the fixed top bar.
  const bar = document.querySelector('.apple-topnav') || document.querySelector('.mac-menubar');
  if (bar && vis(bar)) {
    const br = bar.getBoundingClientRect();
    const active = document.querySelector('.page.section-shell:not([hidden])') || document.body;
    active.querySelectorAll('*').forEach((el) => {
      if (!vis(el)) return;
      if (bar.contains(el) || el.contains(bar)) return;
      const cs = getComputedStyle(el);
      if (cs.position !== 'fixed' && cs.position !== 'sticky' && cs.position !== 'absolute') return;
      const r = el.getBoundingClientRect();
      if (r.top < br.bottom && r.bottom > br.top && r.left < br.right && r.right > br.left) {
        const z = parseInt(cs.zIndex, 10);
        if (!isNaN(z) && z >= 120) return; // legit overlay above the bar
        out.overlaps.push({ el: desc(el), top: Math.round(r.top), barBottom: Math.round(br.bottom), pos: cs.position, z: cs.zIndex });
      }
    });
  }

  // 2. Containers that clip their own content (overflow hidden + content wider).
  document.querySelectorAll('.page.section-shell:not([hidden]) *, .apple-topnav, .apple-rail, .mac-menubar, .navbar-right, .mac-menubar-center, .mac-menubar-left').forEach((el) => {
    if (!vis(el)) return;
    const cs = getComputedStyle(el);
    if (cs.overflowX !== 'hidden' && cs.overflow !== 'hidden') return;
    const over = el.scrollWidth - el.clientWidth;
    if (over > 4 && el.clientWidth > 40) {
      out.clipped.push({ el: desc(el), clippedPx: over, scrollW: el.scrollWidth, clientW: el.clientWidth });
    }
  });

  // 3. Whole-page horizontal overflow.
  const de = document.documentElement;
  if (de.scrollWidth > window.innerWidth + 2) {
    out.overflow.push({ scrollW: de.scrollWidth, innerW: window.innerWidth });
  }

  // 4. Visible elements running past the right viewport edge.
  // A horizontally-scrollable ancestor (overflow-x: auto/scroll, actually
  // scrollable) legitimately holds children whose own box extends past the
  // viewport — that content is reachable by scrolling the ancestor, not
  // unreachable/broken. Found on the workloads/network-intel tables: their
  // wrapper (.vmr-table-wrap, overflow-x:auto) sat entirely on-screen while
  // the <table> child's true (unclipped) rendered width ran past the edge —
  // getBoundingClientRect reports that full box regardless of the ancestor's
  // clipping, so without this check every scrollable table on a narrow
  // viewport reports as "offscreen" even though nothing is actually broken.
  const inScrollableAncestor = (el) => {
    for (let a = el.parentElement; a; a = a.parentElement) {
      const cs = getComputedStyle(a);
      if ((cs.overflowX === 'auto' || cs.overflowX === 'scroll') && a.scrollWidth > a.clientWidth + 2) {
        return true;
      }
    }
    return false;
  };
  const seen = new Set();
  document.querySelectorAll('.page.section-shell:not([hidden]) *').forEach((el) => {
    if (!vis(el)) return;
    const r = el.getBoundingClientRect();
    if (r.right > window.innerWidth + 2 && r.width < window.innerWidth) {
      if (inScrollableAncestor(el)) return;
      const k = desc(el);
      if (seen.has(k)) return;
      seen.add(k);
      out.offscreen.push({ el: k, right: Math.round(r.right), innerW: window.innerWidth });
    }
  });
  return out;
};

(async () => {
  const browser = await chromium.launch({ headless: true, channel: 'chrome', args: ['--ignore-certificate-errors'] });
  const ctx = await browser.newContext({ ignoreHTTPSErrors: true, viewport: { width: VW, height: 1080 } });
  await ctx.addInitScript(([k, tier, theme]) => {
    try {
      localStorage.setItem('veyron_api_key', k);
      localStorage.setItem('veyron_shell', 'desktop');
      localStorage.setItem('veyron_zen_mode', '0');
      localStorage.setItem('veyron_desktop_tier', tier);
      localStorage.setItem('veyron_desktop_tier_auto', '0');
      localStorage.setItem('veyron_theme', theme);
    } catch (e) {}
  }, [KEY, TIER, THEME]);
  const page = await ctx.newPage();
  const errs = {};
  let cur = 'boot';
  page.on('console', (m) => { if (m.type() === 'error') (errs[cur] = errs[cur] || []).push(m.text().slice(0, 160)); });
  page.on('pageerror', (e) => (errs[cur] = errs[cur] || []).push('pageerror: ' + e.message.slice(0, 160)));

  await page.goto(`${BASE}/dashboard?token=${encodeURIComponent(KEY)}#dashboard`, { waitUntil: 'domcontentloaded', timeout: 60000 });
  await page.waitForTimeout(5000);

  const report = {};
  for (const p of PAGES) {
    cur = p;
    try {
      await page.evaluate((id) => { if (typeof navigate === 'function') navigate(id); else location.hash = '#' + id; }, p);
      await page.waitForTimeout(2200);
      const r = await page.evaluate(AUDIT);
      const n = r.overlaps.length + r.clipped.length + r.overflow.length + r.offscreen.length;
      if (n) report[p] = r;
    } catch (e) {
      report[p] = { error: String(e).slice(0, 200) };
    }
  }
  console.log(JSON.stringify({ viewport: VW, tier: TIER, theme: THEME, findings: report, consoleErrors: errs }, null, 2));
  await browser.close();
})().catch((e) => { console.error('ERROR', e); process.exit(2); });
