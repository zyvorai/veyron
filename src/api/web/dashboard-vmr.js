/* Veyron Command Center module */
(function () {
  'use strict';

  var VMR_TABS = [
    { page: 'dashboard', label: 'Home' },
    { page: 'vms', label: 'Fleet' },
    { page: 'app-store', label: 'Templates' },
    { page: 'console-hub', label: 'ConsoleHub' },
    { page: 'snapshots', label: 'Snapshots' },
    { page: 'security', label: 'Security' },
    { page: 'stack-health', label: 'Stack Health' }
  ];

  var VMR_PAGE_TITLES = {
    dashboard: 'Veyron Mission Control',
    vms: 'Veyron Fleet Command',
    'app-store': 'Veyron Template Foundry',
    'console-hub': 'Veyron ConsoleHub',
    snapshots: 'Veyron Snapshots',
    security: 'Veyron Security Posture',
    'stack-health': 'Veyron Stack Health',
    events: 'Veyron Event Intelligence',
    monitoring: 'Veyron Stack Health'
  };

  window.vmrFleetViewMode = window.vmrFleetViewMode || 'cards';
  window.vmrFoundryCategory = window.vmrFoundryCategory || 'all';
  window.lastStackHealth = window.lastStackHealth || null;

  function esc(s) {
    if (typeof window.esc === 'function') return window.esc(s);
    var d = document.createElement('div');
    d.textContent = s || '';
    return d.innerHTML;
  }

  function jsArgs() {
    if (typeof window.jsArgs === 'function') return window.jsArgs.apply(null, arguments);
    return Array.prototype.map.call(arguments, function (a) {
      return JSON.stringify(a == null ? '' : a);
    }).join(', ');
  }

  function onStopHandler(expr) {
    if (typeof window.onStopHandler === 'function') return window.onStopHandler(expr);
    return 'onclick="event.stopPropagation();' + expr + '"';
  }

  function onHandler(expr) {
    if (typeof window.onHandler === 'function') return window.onHandler(expr);
    return 'onclick="' + expr + '"';
  }

  window.syncVmrPrimaryTabs = function syncVmrPrimaryTabs(page) {
    document.querySelectorAll('.vmr-tab').forEach(function (btn) {
      btn.classList.toggle('active', btn.dataset.page === page);
    });
  };

  window.initVmrShell = function initVmrShell() {
    var bar = document.getElementById('vmr-primary-tabs');
    if (!bar || bar.dataset.inited) return;
    bar.dataset.inited = '1';
    bar.innerHTML = VMR_TABS.map(function (t) {
      return '<button type="button" class="vmr-tab" data-page="' + esc(t.page) + '" onclick="navigate(\'' + t.page + '\')">' + esc(t.label) + '</button>';
    }).join('');
    syncVmrPrimaryTabs(typeof currentPage !== 'undefined' ? currentPage : 'dashboard');

    var search = document.getElementById('vmr-header-search');
    if (search) {
      search.addEventListener('keydown', function (e) {
        if (e.key === 'Enter') {
          if (typeof openSpotlight === 'function') openSpotlight();
        }
      });
    }
  };

  window.renderVmrMetricsStrip = function renderVmrMetricsStrip(elId, metrics) {
    var el = document.getElementById(elId);
    if (!el) return;
    el.className = 'vmr-metrics-strip';
    el.innerHTML = metrics.map(function (m) {
      var cls = m.tone ? ' vmr-metric-value ' + m.tone : ' vmr-metric-value';
      return '<div class="vmr-metric"><div class="vmr-metric-label">' + esc(m.label) +
        '</div><div class="' + cls.trim() + '">' + esc(String(m.value)) + '</div></div>';
    }).join('');
  };

  window.renderVmrPageHero = function renderVmrPageHero(elId, title, tagline, actionHtml) {
    var el = document.getElementById(elId);
    if (!el) return;
    el.className = 'vmr-page-hero';
    el.innerHTML = '<div><h1>' + esc(title) + '</h1>' +
      (tagline ? '<p class="vmr-tagline">' + esc(tagline) + '</p>' : '') + '</div>' +
      (actionHtml || '');
  };

  window.renderMissionControlVmr = function renderMissionControlVmr() {
    var heroEl = document.getElementById('vmr-mission-hero');
    if (!heroEl) return;
    var ns = typeof currentNamespace !== 'undefined' ? currentNamespace : 'all';
    var vms = typeof vmData !== 'undefined' ? vmData : [];
    var running = vms.filter(function (v) { return v.status === 'Running'; }).length;
    var stopped = vms.filter(function (v) { return v.status === 'Running' ? false : v.status !== 'Failed'; }).length;
    var failed = vms.filter(function (v) { return v.status === 'Failed' || v.status === 'Error'; }).length;
    var ov = typeof lastOverview !== 'undefined' ? lastOverview : {};
    var nodes = ov.nodes || {};
    var warnings = typeof islandAlertCount !== 'undefined' ? islandAlertCount : 0;
    if (!warnings && typeof lastEvents !== 'undefined') {
      warnings = lastEvents.filter(function (e) { return e.type === 'Warning'; }).length;
    }

    renderVmrPageHero('vmr-mission-hero', 'Veyron Mission Control',
      'Kubernetes-native VM command center · ' + ns + ' workspace',
      '<button type="button" class="btn-create glass-btn-primary" onclick="openCreateModal()">+ Forge VM</button>');

    renderVmrMetricsStrip('vmr-mission-metrics', [
      { label: 'Total VMs', value: vms.length },
      { label: 'Running', value: running, tone: 'ok' },
      { label: 'Stopped', value: stopped },
      { label: 'Warnings', value: warnings, tone: warnings ? 'warn' : '' },
      { label: 'Nodes', value: (nodes.ready_nodes != null ? nodes.ready_nodes + '/' + nodes.total_nodes : '—') }
    ]);

    var qa = document.getElementById('vmr-quick-actions');
    if (qa) {
      qa.innerHTML = [
        { label: 'Forge VM', fn: 'openCreateModal()' },
        { label: 'Open Console', fn: "navigate('console-hub')" },
        { label: 'Template Foundry', fn: "navigate('app-store')" },
        { label: 'Stack Health', fn: "navigate('stack-health')" },
        { label: 'Snapshots', fn: "navigate('snapshots')" },
        { label: 'PacketWolf', fn: "navigate('cilium')" },
        { label: 'Ask Veyron', fn: 'openAskZeus()' }
      ].map(function (a) {
        return '<button type="button" class="vmr-quick-btn" onclick="' + a.fn + '">' + esc(a.label) + '</button>';
      }).join('');
    }

    if (typeof renderCloudOsDatacenter === 'function') renderCloudOsDatacenter();
    if (typeof renderDashboardCommandCenter === 'function') renderDashboardCommandCenter();
    renderVmrEventPreview();
  };

  window.renderVmrEventPreview = function renderVmrEventPreview() {
    var el = document.getElementById('vmr-event-preview');
    if (!el) return;
    var events = typeof lastEvents !== 'undefined' ? lastEvents.slice(0, 5) : [];
    if (!events.length) {
      el.innerHTML = '<p style="color:var(--muted);font-size:.84rem">No recent cluster events.</p>';
      return;
    }
    el.innerHTML = events.map(function (ev) {
      return renderVmrIncidentCard(ev);
    }).join('') + '<button type="button" class="glass-btn-secondary glass-btn-sm" style="margin-top:8px" onclick="navigate(\'events\')">View all events</button>';
  };

  window.renderVmrIncidentCard = function renderVmrIncidentCard(ev) {
    if (!ev) return '';
    var reason = ev.reason || ev.type_ || 'Event';
    var msg = ev.message || '';
    var sev = (ev.type === 'Warning' || reason.indexOf('Failed') >= 0 || reason.indexOf('Error') >= 0) ? 'failed' :
      (ev.type === 'Warning' ? 'warning' : '');
    var impact = reason.indexOf('Failed') >= 0 || reason.indexOf('Pull') >= 0 ? 'VM or workload may not start.' : 'Monitor cluster signal.';
    var fixes = '';
    if (reason.indexOf('Failed') >= 0 && msg.indexOf('pull') >= 0) {
      fixes = '<ul style="margin:8px 0 0 18px;font-size:.82rem;color:var(--muted)">' +
        '<li>Verify image name and registry access</li><li>Add imagePullSecret if private</li>' +
        '<li>Replace with a Veyron template image</li></ul>';
    }
    return '<div class="vmr-incident-card ' + sev + '">' +
      '<div style="font-weight:700;color:var(--text-bright);margin-bottom:4px">' + esc(reason) + '</div>' +
      '<div style="font-size:.78rem;color:var(--muted);margin-bottom:6px">' + esc(ev.involved_object || ev.namespace || '') + ' · ' + esc(ev.timestamp || '') + '</div>' +
      '<div style="font-size:.84rem;line-height:1.45;margin-bottom:8px">' + esc(msg) + '</div>' +
      '<div style="font-size:.78rem;color:var(--orange)">Impact: ' + esc(impact) + '</div>' + fixes +
      '<div style="display:flex;gap:8px;margin-top:10px;flex-wrap:wrap">' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'events\')">Open events</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs(msg.slice(0, 120)) + ')">Ask Veyron</button></div></div>';
  };

  window.setVmrFleetView = function setVmrFleetView(mode) {
    window.vmrFleetViewMode = mode;
    document.querySelectorAll('.vmr-view-switch button').forEach(function (b) {
      b.classList.toggle('active', b.dataset.view === mode);
    });
    if (typeof filterVMs === 'function') filterVMs();
  };

  window.vmrFleetCard = function vmrFleetCard(vm, showActions) {
    if (typeof window.vmCard === 'function' && window.vmrFleetViewMode !== 'cards') {
      /* table/topology use alternate renderers */
    }
    var ns = vm.namespace || 'default';
    var isRunning = vm.status === 'Running';
    var isIssue = vm.status === 'Failed' || vm.status === 'Error' || vm.status === 'Pending';
    var cardCls = 'vmr-fleet-card' + (isRunning ? '' : isIssue ? ' issue' : ' stopped');
    var icon = typeof osFamilyIcon === 'function' ? osFamilyIcon(typeof guessOsFamily === 'function' ? guessOsFamily(vm) : 'linux') : '◫';
    var meta = esc(ns) + ' · ' + esc(vm.node || 'no node') + (vm.ip && vm.ip !== 'N/A' ? ' · ' + esc(vm.ip) : '');
    var stats = esc(vm.cpu || '—') + ' vCPU · ' + esc(vm.memory || '—') +
      (typeof vmHealthPill === 'function' ? vmHealthPill(vm) : '');
    var actions = '';
    if (showActions !== false) {
      if (isRunning) {
        actions = '<button type="button" class="glass-btn-primary glass-btn-sm" ' + onStopHandler('openConnectModal(' + jsArgs(ns, vm.name) + ')') + '>Console</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('selectVm(' + jsArgs(ns, vm.name) + ')') + '>Details</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('navigate(\'snapshots\')') + '>Snapshot</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openCopilotNetwork(' + jsArgs(ns, vm.name) + ')') + '>Network</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openAskZeus(' + jsArgs('VM ' + vm.name) + ')') + '>Ask Veyron</button>';
      } else {
        actions = '<button type="button" class="glass-btn-primary glass-btn-sm" ' + onStopHandler('vmAction(' + jsArgs(ns, vm.name, 'start') + ')') + '>Start</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openCopilotDoctor(' + jsArgs(ns, vm.name) + ')') + '>Diagnose</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openCreateModal()') + '>Edit Hardware</button>' +
          '<button type="button" class="glass-btn-destructive glass-btn-sm" ' + onStopHandler('vmDelete(' + jsArgs(ns, vm.name) + ')') + '>Delete</button>';
      }
    }
    return '<div class="' + cardCls + '" data-vm-ns="' + esc(ns) + '" data-vm-name="' + esc(vm.name) + '" ' + onHandler('selectVm(' + jsArgs(ns, vm.name) + ')') + '>' +
      '<div class="vmr-fleet-card-head"><div><span style="font-size:1.2rem;margin-right:8px">' + icon + '</span>' +
      '<span class="vmr-fleet-card-name">' + esc(vm.name) + '</span></div>' +
      '<span class="vm-badge ' + (isRunning ? 'running' : 'stopped') + '">' + esc(vm.status) + '</span></div>' +
      '<div class="vmr-fleet-card-meta">' + meta + '<br>' + stats + '</div>' +
      '<div class="vmr-fleet-actions">' + actions + '</div></div>';
  };

  window.renderVmrFleetTable = function renderVmrFleetTable(vms) {
    var rows = vms.map(function (vm) {
      var ns = vm.namespace || 'default';
      return '<tr class="clickable-row" onclick="selectVm(' + jsArgs(ns, vm.name) + ')">' +
        '<td>' + esc(vm.name) + '</td><td>' + esc(vm.status) + '</td><td>' + esc(ns) + '</td>' +
        '<td>' + esc(vm.node || '—') + '</td><td>' + esc(vm.ip || '—') + '</td>' +
        '<td>' + esc(vm.cpu || '—') + '</td><td>' + esc(vm.memory || '—') + '</td>' +
        '<td><button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('selectVm(' + jsArgs(ns, vm.name) + ')') + '>Open</button></td></tr>';
    }).join('');
    return '<table class="table"><thead><tr><th>Name</th><th>State</th><th>Namespace</th><th>Node</th><th>IP</th><th>CPU</th><th>Mem</th><th></th></tr></thead><tbody>' +
      rows + '</tbody></table>';
  };

  window.renderVmrFleetTopology = function renderVmrFleetTopology(vms) {
    var byNode = {};
    vms.forEach(function (vm) {
      var n = vm.node || 'Unscheduled';
      if (!byNode[n]) byNode[n] = [];
      byNode[n].push(vm);
    });
    var html = '<div class="vmr-panel"><div class="vmr-panel-title">Cluster topology</div>';
    Object.keys(byNode).sort().forEach(function (node) {
      html += '<div style="margin-bottom:12px"><strong style="color:var(--cyan)">' + esc(node) + '</strong><ul style="list-style:none;padding:8px 0 0 16px">';
      byNode[node].forEach(function (vm) {
        html += '<li style="font-size:.84rem;padding:4px 0;color:var(--muted)">├── ' + esc(vm.name) + ' · ' + esc(vm.status) + '</li>';
      });
      html += '</ul></div>';
    });
    return html + '</div>';
  };

  window.renderFleetCommandVmr = function renderFleetCommandVmr() {
    var hero = document.getElementById('vmr-fleet-hero');
    if (!hero) return;
    var vms = typeof vmData !== 'undefined' ? vmData : [];
    var running = vms.filter(function (v) { return v.status === 'Running'; }).length;
    var stopped = vms.length - running;
    renderVmrPageHero('vmr-fleet-hero', 'Veyron Fleet Command',
      vms.length + ' virtual machines · ' + running + ' running · ' + stopped + ' stopped',
      '<button type="button" class="btn-create glass-btn-primary" onclick="openCreateModal()">+ Forge VM</button>');
    renderVmrMetricsStrip('vmr-fleet-metrics', [
      { label: 'Total', value: vms.length },
      { label: 'Running', value: running, tone: 'ok' },
      { label: 'Stopped', value: stopped },
      { label: 'Issues', value: vms.filter(function (v) { return v.status === 'Failed' || v.status === 'Error'; }).length, tone: 'bad' }
    ]);
  };

  window.patchRenderVmFullList = function patchRenderVmFullList() {
    if (window._vmrPatchedRenderVmFullList) return;
    window._vmrPatchedRenderVmFullList = true;
    var orig = window.renderVmFullList;
    window.renderVmFullList = function (vms) {
      renderFleetCommandVmr();
      var el = document.getElementById('vm-full-list');
      if (!el) return orig ? orig(vms) : undefined;
      if (!vms.length) {
        el.innerHTML = typeof vmListEmptyHtml === 'function' ? vmListEmptyHtml(true) : '<p>No VMs</p>';
        return;
      }
      var mode = window.vmrFleetViewMode || 'cards';
      if (mode === 'table') {
        el.innerHTML = renderVmrFleetTable(vms);
      } else if (mode === 'topology') {
        el.innerHTML = renderVmrFleetTopology(vms);
      } else {
        el.innerHTML = '<div class="vmr-card-grid">' + vms.map(function (v) { return vmrFleetCard(v, true); }).join('') + '</div>';
      }
      if (typeof syncVmSelectionUi === 'function') syncVmSelectionUi();
    };
  };

  window.renderFoundryVmr = function renderFoundryVmr() {
    var rail = document.getElementById('vmr-foundry-rail');
    if (!rail) return;
    var cats = [
      { id: 'all', label: 'All templates' },
      { id: 'linux', label: 'Linux' },
      { id: 'windows', label: 'Windows' },
      { id: 'enterprise', label: 'Enterprise' },
      { id: 'cloud', label: 'Cloud Native' },
      { id: 'custom', label: 'Custom Image' }
    ];
    rail.innerHTML = cats.map(function (c) {
      return '<button type="button" class="' + (window.vmrFoundryCategory === c.id ? 'active' : '') +
        '" onclick="setVmrFoundryCategory(\'' + c.id + '\')">' + esc(c.label) + '</button>';
    }).join('');
    renderVmrPageHero('vmr-foundry-hero', 'Veyron Template Foundry',
      'Launch Linux, Windows, BSD, Talos, and custom KubeVirt VMs.',
      '<button type="button" class="btn-create glass-btn-primary" onclick="openCreateModal()">+ Forge VM</button>');
  };

  window.setVmrFoundryCategory = function setVmrFoundryCategory(cat) {
    window.vmrFoundryCategory = cat;
    renderFoundryVmr();
    if (typeof fetchAppStore === 'function') fetchAppStore();
  };

  window.patchFetchAppStore = function patchFetchAppStore() {
    if (window._vmrPatchedFetchAppStore) return;
    window._vmrPatchedFetchAppStore = true;
    var orig = window.fetchAppStore;
    window.fetchAppStore = async function () {
      await orig.apply(this, arguments);
      renderFoundryVmr();
      var grid = document.getElementById('app-store-grid');
      if (!grid || !window.appStoreCache) return;
      var cat = window.vmrFoundryCategory || 'all';
      var filtered = window.appStoreCache.filter(function (t) {
        if (cat === 'all') return true;
        if (cat === 'custom') return false;
        var fam = (t.os_family || '').toLowerCase();
        if (cat === 'linux') return fam !== 'windows';
        if (cat === 'windows') return fam === 'windows';
        if (cat === 'enterprise') return /rhel|oracle|almalinux|rocky|windows/.test((t.id || '').toLowerCase());
        if (cat === 'cloud') return /talos|flatcar|arch/.test((t.id || '').toLowerCase());
        return true;
      });
      var icon = { windows: '🪟', ubuntu: '🐧', debian: '🐧', fedora: '🎩', linux: '🐧' };
      var cards = filtered.map(function (t) {
        var em = icon[t.os_family] || '◫';
        return '<div class="vmr-template-card">' +
          '<div style="display:flex;justify-content:space-between;align-items:flex-start">' +
          '<div><span style="font-size:1.4rem">' + em + '</span> <strong>' + esc(t.title) + '</strong></div>' +
          '<span class="glass-badge ok">READY</span></div>' +
          '<div style="font-size:.82rem;color:var(--muted)">' + esc(t.subtitle) + '</div>' +
          '<div style="font-size:.78rem;color:var(--muted)">' + esc(t.cpu || '2') + ' vCPU · ' + esc(t.memory || '4Gi') + ' · ' + esc(t.default_disk_size || '20Gi') + '</div>' +
          '<div class="vmr-template-card-actions">' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('forgeFromTemplate(' + jsArgs(t.id) + ')') + '>Customize</button>' +
          '<button type="button" class="glass-btn-primary glass-btn-sm" ' + onStopHandler('forgeFromTemplate(' + jsArgs(t.id) + ')') + '>Forge VM</button></div></div>';
      }).join('');
      grid.className = 'vmr-card-grid';
      grid.innerHTML = cards + '<div class="vmr-template-card" style="border-style:dashed">' +
        '<strong>Import custom image</strong><p style="font-size:.82rem;color:var(--muted);margin:8px 0">QCOW2 / VMDK via CDI</p>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openImageImportModal && openImageImportModal()">Import Image</button></div>';
      var sumEl = document.getElementById('app-store-summary');
      if (sumEl) {
        var windows = window.appStoreCache.filter(function (t) { return (t.os_family || '').toLowerCase() === 'windows'; }).length;
        sumEl.innerHTML = '<div class="vmr-metrics-strip">' +
          '<div class="vmr-metric"><div class="vmr-metric-label">Templates</div><div class="vmr-metric-value">' + window.appStoreCache.length + '</div></div>' +
          '<div class="vmr-metric"><div class="vmr-metric-label">Linux</div><div class="vmr-metric-value">' + (window.appStoreCache.length - windows) + '</div></div>' +
          '<div class="vmr-metric"><div class="vmr-metric-label">Windows</div><div class="vmr-metric-value">' + windows + '</div></div></div>';
      }
    };
  };

  window.fetchStackHealth = async function fetchStackHealth() {
    try {
      if (window.lastStackHealth && typeof lastOverview !== 'undefined' && lastOverview && lastOverview.platform) {
        window.lastStackHealth = lastOverview.platform;
        renderStackHealthVmr();
        return;
      }
      var j = await apiJson('/api/v1/dashboard/overview');
      var d = typeof unwrapData === 'function' ? unwrapData(j) : j;
      window.lastStackHealth = (d && d.platform) || null;
      renderStackHealthVmr();
    } catch (e) {
      toast('Stack Health: ' + (e.message || String(e)), 'error');
    }
  };

  window.renderStackHealthVmr = function renderStackHealthVmr() {
    var el = document.getElementById('vmr-stack-body');
    if (!el) return;
    var p = window.lastStackHealth;
    if (!p) {
      el.innerHTML = '<p style="color:var(--muted)">Loading stack health…</p>';
      return;
    }
    var checks = [
      { ok: p.kubevirt_api_ok, label: 'KubeVirt API', detail: p.kubevirt_api_message || 'VirtualMachine API list' },
      { ok: p.kubevirt_control_plane_ok, label: 'KubeVirt Controller', detail: p.kubevirt_detail || '' },
      { ok: p.cdi_operator_ok, label: 'CDI Operator', detail: p.cdi_detail || '' },
      { ok: p.forge_vm_ready, label: 'Forge VM', detail: p.forge_vm_ready ? 'Dashboard forge workflow ready' : 'Resolve KubeVirt first' }
    ];
    var passed = checks.filter(function (c) { return c.ok; }).length;
    renderVmrPageHero('vmr-stack-hero', 'Veyron Stack Health',
      passed + '/4 passed — KubeVirt, CDI, VM API, and Forge workflow.',
      '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="openPlatformInstallModal()">Install Stack</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="fetchStackHealth()">Recheck</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus()">Ask Veyron</button>');
    el.innerHTML = '<div class="vmr-stack-hero"><div class="vmr-stack-ring">' + passed + '/4</div>' +
      '<div class="vmr-stack-timeline">' +
      ['CRDs', 'RBAC', 'API', 'Controller', 'CDI', 'Template', 'Forge VM'].map(function (s, i) {
        return '<span class="' + (i < passed + 2 ? 'ok' : '') + '">' + s + (i < 6 ? ' →' : '') + '</span>';
      }).join(' ') + '</div></div>' +
      checks.map(function (c) {
        return '<div class="vmr-panel" style="margin-bottom:12px;border-left:3px solid ' + (c.ok ? 'var(--green)' : 'var(--red)') + '">' +
          '<div style="display:flex;justify-content:space-between"><strong>' + esc(c.label) + '</strong>' +
          '<span style="color:' + (c.ok ? 'var(--green)' : 'var(--red)') + '">' + (c.ok ? 'PASSED' : 'FAILED') + '</span></div>' +
          '<div style="font-size:.82rem;color:var(--muted);margin-top:6px">' + esc(c.detail) + '</div></div>';
      }).join('');
  };

  window.renderConsoleHubVmr = function renderConsoleHubVmr() {
    renderVmrPageHero('vmr-console-hero', 'Veyron ConsoleHub',
      'VNC, serial, RDP, and SSH consoles for running VMs.',
      '');
    var el = document.getElementById('vmr-console-list');
    if (!el) return;
    var vms = (typeof vmData !== 'undefined' ? vmData : []).filter(function (v) { return v.status === 'Running'; });
    if (!vms.length) {
      el.innerHTML = '<div class="empty-state"><p>No running VMs — start a VM to open a console.</p></div>';
      return;
    }
    el.innerHTML = '<div class="vmr-card-grid">' + vms.map(function (vm) {
      var ns = vm.namespace || 'default';
      return '<div class="vmr-fleet-card">' +
        '<div class="vmr-fleet-card-name">' + esc(vm.name) + '</div>' +
        '<div class="vmr-fleet-card-meta">' + esc(ns) + ' · ' + esc(vm.ip || '—') + '</div>' +
        '<div class="vmr-fleet-actions">' +
        '<button type="button" class="glass-btn-primary glass-btn-sm" ' + onStopHandler('openConnectModal(' + jsArgs(ns, vm.name) + ')') + '>Console</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openConnectModal(' + jsArgs(ns, vm.name, 'vnc') + ')') + '>VNC</button></div></div>';
    }).join('') + '</div>';
  };

  window.patchNavigateVmr = function patchNavigateVmr() {
    if (window._vmrPatchedNavigate) return;
    window._vmrPatchedNavigate = true;
    var orig = window.navigate;
    window.navigate = function (page, opts) {
      orig.apply(this, arguments);
      syncVmrPrimaryTabs(page);
      document.title = (VMR_PAGE_TITLES[page] || 'Veyron') + ' · Kubernetes VM Command Center';
      if (typeof updateMacPageToolbar === 'function') {
        var meta = typeof PAGE_META !== 'undefined' ? PAGE_META[page] : null;
        if (meta) updateMacPageToolbar(page, meta);
      }
      if (page === 'console-hub') renderConsoleHubVmr();
      if (page === 'stack-health') fetchStackHealth();
      if (page === 'events' && typeof renderEventIntelligenceVmr === 'function') renderEventIntelligenceVmr();
    };
  };

  window.renderEventIntelligenceVmr = function renderEventIntelligenceVmr() {
    var events = typeof lastEvents !== 'undefined' ? lastEvents : [];
    var warnings = events.filter(function (e) { return e.type === 'Warning'; }).length;
    renderVmrPageHero('vmr-events-hero', 'Veyron Event Intelligence',
      events.length + ' signals · ' + warnings + ' warnings · namespace ' + (typeof currentNamespace !== 'undefined' ? currentNamespace : 'all'),
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="fetchEvents(true)">Refresh</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus()">Ask Veyron</button>');
    var el = document.getElementById('vmr-events-list');
    if (!el) return;
    if (!events.length) {
       el.innerHTML = '<p style="color:var(--muted);font-size:.84rem">No cluster events in scope.</p>';
      return;
    }
    el.innerHTML = events.slice(0, 40).map(function (ev) {
      return renderVmrIncidentCard(ev);
    }).join('');
  };

  window.initVmrModule = function initVmrModule() {
    initVmrShell();
    patchRenderVmFullList();
    patchFetchAppStore();
    patchNavigateVmr();
    renderMissionControlVmr();
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initVmrModule);
  } else {
    setTimeout(initVmrModule, 0);
  }
})();
