// Interactive overlay audit.
//
// The Dynamic Island bug was invisible to a static page sweep: the panel only
// misrendered once EXPANDED. This opens each overlay/modal/popover and measures
// it in its open state, looking for the same failure modes:
//   trapped   overlay rendered inside clipped/fixed chrome instead of on top
//   offtop    content above the viewport top (unreachable)
//   offside   content past the left/right viewport edge
//   clipped   overlay's own box clips its content
//   noescape  Escape does not dismiss it (keyboard trap)
//   error     opening it threw, or logged a console error
const { chromium } = require(process.env.PW_CORE);

const HOST = process.argv[2];
const PORT = process.argv[3] || '30151';
const VW = parseInt(process.argv[4] || '1440', 10);
const KEY = process.env.VEYRON_API_KEY || 'CHANGE_ME';
const THEME = process.env.VEYRON_TEST_THEME || 'light';
const BASE = `https://${HOST}:${PORT}`;

// [label, open expression, selector of the thing that should become visible]
const OVERLAYS = [
  ['spotlight',        'openSpotlight()',              '#spotlight-modal, .spotlight-modal, #spotlight'],
  ['launchpad',        'openLaunchpad()',              '#launchpad, .launchpad-overlay'],
  ['missionControl',   'openMissionControlOverlay()',  '#mission-control-overlay, .mission-control-overlay'],
  ['notificationCtr',  'toggleNotificationCenter()',   '#mac-notification-center'],
  ['controlCenter',    'toggleControlCenter()',        '#mac-control-center'],
  ['about',            'openAboutModal()',             '#about-modal, .about-modal'],
  ['preferences',      'openMacPreferences()',         '#mac-preferences-modal, #mac-preferences'],
  ['dockEditor',       'openDockEditor()',             '#dock-editor-modal'],
  ['quickForge',       'openQuickForge()',             '#quick-forge-modal, #quickforge-modal'],
  ['askZyra',          'openAskZyra()',                '#copilot-panel, #copilot-modal, .id-zeus'],
  ['classicNav',       'toggleClassicNav()',           '#classic-nav-backdrop, #browse-menu, .classic-nav-panel'],
  ['actionLog',        'toggleActionLog()',            '#vmr-action-log-panel'],
  ['createVm',         'openCreateModal()',            '#forge-wizard-modal, #create-modal'],
  ['island',           'toggleDynamicIsland()',        '#mac-dynamic-island-expanded'],
  ['macMenuGo',        "toggleMacMenu('go')",          '#mac-menu-go'],
  ['macMenuView',      "toggleMacMenu('view')",        '#mac-menu-view'],
  ['macMenuZeusos',    "toggleMacMenu('zeusos')",      '#mac-menu-zeusos'],
  ['crdCreate',        'openCrdCreateModal()',         '#crd-create-modal'],
  ['snapSchedule',     'openScheduleModal()',          '#snap-schedule-modal'],
  ['platformInstall',  'openPlatformInstallModal()',   '#platform-install-modal'],
  ['imageImport',      'openImageImportModal()',       '#image-import-modal'],
  ['imageUpload',      'openImageUploadModal()',       '#image-upload-modal'],
];

(async () => {
  const browser = await chromium.launch({ headless: true, channel: 'chrome', args: ['--ignore-certificate-errors'] });
  const ctx = await browser.newContext({ ignoreHTTPSErrors: true, viewport: { width: VW, height: 1080 } });
  await ctx.addInitScript(([k, theme]) => {
    try {
      localStorage.setItem('veyron_api_key', k);
      localStorage.setItem('veyron_shell', 'desktop');
      localStorage.setItem('veyron_zen_mode', '0');
      localStorage.setItem('veyron_desktop_tier', 'advanced');
      localStorage.setItem('veyron_desktop_tier_auto', '0');
      localStorage.setItem('veyron_theme', theme);
    } catch (e) {}
  }, [KEY, THEME]);
  const page = await ctx.newPage();
  let cur = 'boot';
  const consoleErrs = {};
  page.on('console', (m) => { if (m.type() === 'error') (consoleErrs[cur] = consoleErrs[cur] || []).push(m.text().slice(0, 140)); });
  page.on('pageerror', (e) => (consoleErrs[cur] = consoleErrs[cur] || []).push('pageerror: ' + e.message.slice(0, 140)));

  await page.goto(`${BASE}/dashboard?token=${encodeURIComponent(KEY)}#dashboard`, { waitUntil: 'domcontentloaded', timeout: 60000 });
  await page.waitForTimeout(5000);

  const findings = {};
  for (const [label, openExpr, sel] of OVERLAYS) {
    cur = label;
    try {
      const opened = await page.evaluate((expr) => {
        try { eval(expr); return true; } catch (e) { return 'threw: ' + e.message; }
      }, openExpr);
      if (opened !== true) { findings[label] = [{ kind: 'error', detail: opened }]; continue; }
      await page.waitForTimeout(label === 'createVm' ? 4000 : 900);

      const r = await page.evaluate((sel) => {
        const vis = (el) => {
          if (!el) return false;
          const cs = getComputedStyle(el);
          if (cs.display === 'none' || cs.visibility === 'hidden' || cs.opacity === '0') return false;
          const b = el.getBoundingClientRect();
          return b.width > 1 && b.height > 1;
        };
        const el = sel.split(',').map(s => document.querySelector(s.trim())).find(vis);
        if (!el) return { notVisible: true };
        const b = el.getBoundingClientRect();
        const out = { rect: { top: Math.round(b.top), bottom: Math.round(b.bottom), left: Math.round(b.left), right: Math.round(b.right) }, iw: window.innerWidth, ih: window.innerHeight };
        // Is any ancestor clipping this overlay (overflow hidden + fixed height)?
        // A plain overflow:hidden ancestor can only clip a position:fixed descendant if
        // it also establishes a containing block for it (transform/filter/perspective/
        // contain/will-change/backdrop-filter) — per spec, overflow:hidden alone does not.
        // Without this check, any overflow:hidden layout container (common in this shell)
        // falsely flags every fixed, full-viewport overlay as "trapped".
        const elIsFixed = getComputedStyle(el).position === 'fixed';
        const establishesContainingBlock = (cs) =>
          (cs.transform && cs.transform !== 'none') ||
          (cs.filter && cs.filter !== 'none') ||
          (cs.perspective && cs.perspective !== 'none') ||
          (cs.backdropFilter && cs.backdropFilter !== 'none') ||
          (cs.willChange && /transform|perspective|filter/.test(cs.willChange)) ||
          (cs.contain && /paint|layout|strict|content/.test(cs.contain));
        let n = el.parentElement, trapped = null;
        while (n && n !== document.body) {
          const cs = getComputedStyle(n);
          const clips = cs.overflow === 'hidden' || cs.overflowY === 'hidden';
          if (clips && (!elIsFixed || establishesContainingBlock(cs))) {
            const nb = n.getBoundingClientRect();
            if (b.bottom > nb.bottom + 1 || b.top < nb.top - 1) {
              trapped = (n.tagName.toLowerCase() + (n.id ? '#' + n.id : '') + '.' + (n.className || '').toString().split(/\s+/)[0]);
              break;
            }
          }
          n = n.parentElement;
        }
        out.trapped = trapped;
        out.selfClipped = el.scrollWidth > el.clientWidth + 4 && getComputedStyle(el).overflowX === 'hidden';
        return out;
      }, sel);

      const f = [];
      if (r.notVisible) {
        f.push({ kind: 'error', detail: 'opened but nothing visible matched selector' });
      } else {
        if (r.trapped) f.push({ kind: 'trapped', detail: `clipped by ancestor ${r.trapped}`, rect: r.rect });
        if (r.rect.top < 0) f.push({ kind: 'offtop', detail: `top ${r.rect.top}`, rect: r.rect });
        if (r.rect.left < 0) f.push({ kind: 'offside', detail: `left ${r.rect.left}`, rect: r.rect });
        if (r.rect.right > r.iw + 2) f.push({ kind: 'offside', detail: `right ${r.rect.right} > vw ${r.iw}`, rect: r.rect });
        if (r.selfClipped) f.push({ kind: 'clipped', detail: 'overlay clips its own content' });
      }

      // Escape should dismiss
      await page.keyboard.press('Escape');
      await page.waitForTimeout(450);
      const stillOpen = await page.evaluate((sel) => {
        const vis = (el) => {
          if (!el) return false;
          const cs = getComputedStyle(el);
          if (cs.display === 'none' || cs.visibility === 'hidden' || cs.opacity === '0') return false;
          const b = el.getBoundingClientRect();
          return b.width > 1 && b.height > 1;
        };
        return sel.split(',').some(s => vis(document.querySelector(s.trim())));
      }, sel);
      if (stillOpen && !r.notVisible) f.push({ kind: 'noescape', detail: 'still visible after Escape' });
      // force-close so the next overlay starts clean
      await page.evaluate(() => { try { document.body.click(); } catch (e) {} });
      await page.waitForTimeout(250);

      if ((consoleErrs[label] || []).length) f.push({ kind: 'error', detail: consoleErrs[label][0] });
      if (f.length) findings[label] = f;
    } catch (e) {
      findings[label] = [{ kind: 'error', detail: String(e).slice(0, 160) }];
    }
  }

  console.log(JSON.stringify({ viewport: VW, theme: THEME, findings }, null, 2));
  await browser.close();
})().catch((e) => { console.error('ERROR', e); process.exit(2); });
