/* Veyron Command Center module */
(function () {
  'use strict';

  var VEYRON_NAV = [
    { page: 'dashboard', label: 'Mission Control', icon: '◫' },
    { page: 'vms', label: 'Fleet Command', icon: '▣' },
    { page: 'app-store', label: 'Template Foundry', icon: '⬡' },
    { page: 'console-hub', label: 'ConsoleHub', icon: '▶' },
    { page: 'vm-capsule', label: 'VM Capsule', icon: '⬡' },
    { page: 'snapshots', label: 'Snapshots & Backups', icon: '⧉' },
    { page: 'network-intel', label: 'Network Intelligence', icon: '⟷' },
    { page: 'security', label: 'Security Posture', icon: '⛨' },
    { page: 'stack-health', label: 'Stack Health', icon: '♥' },
    { page: 'events', label: 'Event Intelligence', icon: '⚡' },
    { page: 'costs', label: 'Cost Explorer', icon: '$' },
    { page: 'workloads', label: 'Workloads', icon: '◧' },
    { page: 'nodes', label: 'Cluster Nodes', icon: '⬢' },
    { page: 'settings', label: 'Settings', icon: '⚙' }
  ];

  var VMR_PAGE_TITLES = {
    dashboard: 'Veyron Mission Control',
    vms: 'Veyron Fleet Command',
    'app-store': 'Veyron Template Foundry',
    'console-hub': 'Veyron ConsoleHub',
    'vm-capsule': 'Veyron VM Capsule',
    snapshots: 'Veyron Snapshots & Backups',
    'network-intel': 'Veyron Network Intelligence',
    security: 'Veyron Security Posture',
    'stack-health': 'Veyron Stack Health',
    events: 'Veyron Event Intelligence',
    cilium: 'Veyron Network Intelligence',
    costs: 'Veyron Cost Explorer',
    workloads: 'Veyron Workloads',
    nodes: 'Veyron Cluster Nodes',
    settings: 'Veyron Settings',
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

  window.syncVeyronSidebar = function syncVeyronSidebar(page) {
    document.querySelectorAll('.veyron-nav-item').forEach(function (btn) {
      btn.classList.toggle('active', btn.dataset.page === page);
    });
  };

  window.syncVeyronTopbar = function syncVeyronTopbar() {
    var pill = document.getElementById('veyron-alerts-pill');
    if (pill) {
      var n = typeof islandAlertCount !== 'undefined' ? islandAlertCount : 0;
      if (!n && typeof lastEvents !== 'undefined' && lastEvents.length) {
        n = lastEvents.filter(function (e) { return e.type === 'Warning'; }).length;
      }
      pill.textContent = n ? n + ' Alert' + (n === 1 ? '' : 's') : 'No alerts';
      pill.classList.toggle('empty', !n);
    }
    var ctxNs = document.getElementById('veyron-context-ns');
    if (ctxNs) {
      ctxNs.textContent = typeof currentNamespace !== 'undefined' ? currentNamespace : 'all';
    }
    var ctxScope = document.getElementById('veyron-context-scope');
    if (ctxScope) {
      ctxScope.textContent = (typeof currentNamespace !== 'undefined' && currentNamespace === 'all')
        ? 'All Namespaces' : 'Single namespace';
    }
  };

  window.syncVmrPrimaryTabs = syncVeyronSidebar;

  window.initVeyronShell = function initVeyronShell() {
    var nav = document.getElementById('veyron-sidebar-nav');
    if (nav && !nav.dataset.inited) {
      nav.dataset.inited = '1';
      nav.innerHTML = VEYRON_NAV.map(function (item) {
        return '<button type="button" class="veyron-nav-item" data-page="' + esc(item.page) +
          '" onclick="navigate(\'' + item.page + '\')">' +
          '<span class="veyron-nav-icon" aria-hidden="true">' + item.icon + '</span>' +
          esc(item.label) + '</button>';
      }).join('') +
        '<div class="veyron-advanced-link">' +
        '<button type="button" class="veyron-nav-item" onclick="toggleClassicNav()">' +
        '<span class="veyron-nav-icon">⋯</span>Advanced</button></div>';
      syncVeyronSidebar(typeof currentPage !== 'undefined' ? currentPage : 'dashboard');
    }

    var vNs = document.getElementById('veyron-ns-select');
    var mainNs = document.getElementById('ns-selector');
    if (vNs && mainNs && !vNs.dataset.synced) {
      vNs.dataset.synced = '1';
      Array.prototype.forEach.call(mainNs.options, function (opt) {
        vNs.appendChild(opt.cloneNode(true));
      });
      vNs.value = mainNs.value;
      vNs.addEventListener('change', function () {
        if (typeof setNamespace === 'function') setNamespace(vNs.value);
        mainNs.value = vNs.value;
      });
      mainNs.addEventListener('change', function () { vNs.value = mainNs.value; });
    }

    var copilotIn = document.getElementById('veyron-copilot-input');
    if (copilotIn) {
      copilotIn.addEventListener('click', function () {
        if (typeof openAskZeus === 'function') openAskZeus();
      });
      copilotIn.addEventListener('keydown', function (e) {
        if (e.key === 'Enter' && typeof openAskZeus === 'function') openAskZeus();
      });
    }

    syncVeyronTopbar();
  };

  window.initVmrShell = initVeyronShell;

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
      { label: 'Nodes Ready', value: (nodes.ready_nodes != null ? nodes.ready_nodes + '/' + nodes.total_nodes : '—') },
      { label: 'Est. Monthly Cost', value: (typeof lastCostSummary !== 'undefined' && lastCostSummary) ? lastCostSummary : '—' },
      { label: 'Active Alerts', value: warnings || 0, tone: warnings ? 'bad' : '' }
    ]);

    var qa = document.getElementById('vmr-quick-actions');
    if (qa) {
      qa.innerHTML = [
        { label: 'Forge VM', fn: 'openCreateModal()' },
        { label: 'Open Console', fn: "navigate('console-hub')" },
        { label: 'Template Foundry', fn: "navigate('app-store')" },
        { label: 'View Snapshots', fn: "navigate('snapshots')" },
        { label: 'Run Health Scan', fn: "navigate('stack-health')" },
        { label: 'Network Intelligence', fn: "navigate('network-intel')" },
        { label: 'Cost Explorer', fn: "navigate('costs')" },
        { label: 'Ask Veyron', fn: 'openAskZeus()' }
      ].map(function (a) {
        return '<button type="button" class="vmr-quick-btn" onclick="' + a.fn + '">' + esc(a.label) + '</button>';
      }).join('');
    }

    if (typeof renderCloudOsDatacenter === 'function') renderCloudOsDatacenter();
    if (typeof renderDashboardCommandCenter === 'function') renderDashboardCommandCenter();
    renderVeyronFleetHealth();
    renderVeyronCopilotPanel();
    renderVeyronActiveAlerts();
    renderVeyronRecentActivity();
    renderVeyronMissionBottom();
    if (typeof renderPinnedVmsFromClient === 'function') renderPinnedVmsFromClient();
    if (typeof syncVeyronTopbar === 'function') syncVeyronTopbar();
  };

  window.renderVeyronFleetHealth = function renderVeyronFleetHealth() {
    var el = document.getElementById('vmr-fleet-health');
    if (!el) return;
    var ov = typeof lastOverview !== 'undefined' ? lastOverview : {};
    var c = ov.cluster || {};
    var n = ov.nodes || {};
    var home = typeof lastExperienceHome !== 'undefined' ? lastExperienceHome : null;
    var score = home && home.health_score != null ? home.health_score : (
      c.total_vms > 0 ? Math.round((c.running_vms / c.total_vms) * 100) : 96
    );
    var warnings = typeof islandAlertCount !== 'undefined' ? islandAlertCount : 0;
    if (!warnings && typeof lastEvents !== 'undefined') {
      warnings = lastEvents.filter(function (e) { return e.type === 'Warning'; }).length;
    }
    var cpuPct = n.total_cpu_allocatable > 0 ? Math.round((c.total_vcpus_allocated / n.total_cpu_allocatable) * 100) : 0;
    var memPct = n.total_memory_allocatable_gb > 0 ? Math.round((c.total_memory_allocated_gb / n.total_memory_allocatable_gb) * 100) : 0;
    function row(label, ok, detail) {
      return '<div class="vmr-health-row"><span>' + esc(label) + '</span><span class="' + (ok ? 'ok' : 'warn') + '">' + esc(detail) + '</span></div>';
    }
    el.innerHTML = '<div class="vmr-health-layout">' +
      '<div class="vmr-health-ring-lg' + (score >= 85 ? '' : score >= 65 ? ' warn' : ' bad') + '">' +
      '<div class="vmr-health-pct">' + esc(String(score)) + '%</div><div class="vmr-health-lbl">Healthy</div></div>' +
      '<div class="vmr-health-checklist">' +
      row('Compute', cpuPct < 90, cpuPct < 90 ? 'Healthy' : 'Pressure') +
      row('Memory', memPct < 90, memPct < 90 ? 'Healthy' : 'Pressure') +
      row('Storage', true, 'Healthy') +
      row('Network', true, 'Healthy') +
      row('Security', warnings < 5, warnings ? warnings + ' warnings' : 'Healthy') +
      row('Recovery', true, 'Ready') +
      '</div></div>';
  };

  window.renderVeyronCopilotPanel = function renderVeyronCopilotPanel() {
    var el = document.getElementById('vmr-copilot-recommendations');
    if (!el) return;
    var items = window._copilotBriefingCache || [];
    var home = typeof lastExperienceHome !== 'undefined' ? lastExperienceHome : null;
    if (!items.length && home && home.copilot_briefing) items = home.copilot_briefing;
    if (!items.length) {
      items = [
        { title: 'Fleet is healthy overall', detail: 'No critical recommendations from Copilot.', severity: 'info' },
        { title: 'Review security warnings', detail: 'Open Security Posture for hardening signals.', severity: 'warning', action: 'navigate:security' },
        { title: 'Verify backup coverage', detail: 'Ensure snapshots or Velero policies protect VMs.', severity: 'info', action: 'navigate:snapshots' }
      ];
    }
    var html = '<p style="font-size:.84rem;color:var(--muted);margin:0 0 10px">Fleet is healthy overall.</p><ol class="vmr-copilot-list">';
    items.slice(0, 4).forEach(function (item, i) {
      html += '<li><button type="button" class="vmr-copilot-rec" onclick="runCopilotBriefing(' + i + ')">' +
        esc(item.title) + '<span>' + esc(item.detail || '') + '</span></button></li>';
    });
    html += '</ol><div class="vmr-copilot-actions">' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'events\')">View All Recommendations</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus()">Ask Veyron</button>' +
      '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="openAskZeus(\'Suggest auto-fixes for fleet issues\')">Auto Fix</button></div>';
    el.innerHTML = html;
    window._copilotBriefingCache = items;
  };

  window.renderVeyronActiveAlerts = function renderVeyronActiveAlerts() {
    var el = document.getElementById('vmr-active-alerts');
    if (!el) return;
    var alerts = typeof lastVeyronAlerts !== 'undefined' ? lastVeyronAlerts : [];
    if (!alerts.length) {
      var events = typeof lastEvents !== 'undefined' ? lastEvents : [];
      alerts = events.filter(function (e) {
        return e.type === 'Warning' || (e.reason && /Failed|Error|BackOff|Pull/i.test(e.reason));
      }).map(function (ev) {
        return {
          name: ev.reason || ev.type || 'Event',
          severity: /Failed|Error|BackOff/i.test(ev.reason || '') ? 'critical' : 'warning',
          status: 'firing',
          source: ev.involved_object || ev.message || '',
          message: ev.message || '',
          namespace: ev.namespace || ''
        };
      });
    }
    var firing = alerts.filter(function (a) {
      var st = (a.status || '').toLowerCase();
      return !st || st === 'firing' || st === 'active';
    }).slice(0, 6);
    if (!firing.length) {
      el.innerHTML = '<p style="font-size:.82rem;color:var(--muted)">No active alerts in scope.</p>';
      return;
    }
    el.innerHTML = firing.map(function (a) {
      var reason = a.name || a.alertname || 'Alert';
      var sev = (a.severity || 'warning').toLowerCase();
      var sevCls = sev === 'critical' ? 'critical' : 'warning';
      var obj = a.source || a.message || '';
      return '<div class="vmr-alert-stack-item ' + sevCls + '">' +
        '<div class="vmr-alert-title">' + esc(reason) + '</div>' +
        '<div class="vmr-alert-meta">' + esc(obj) + (a.namespace ? ' · ' + esc(a.namespace) : '') + '</div>' +
        '<div class="vmr-alert-foot"><span class="vmr-alert-sev">' + esc(sevCls) + '</span>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs(String(a.message || reason).slice(0, 100)) + ')">Fix</button></div></div>';
    }).join('');
  };

  window.renderVeyronRecentActivity = function renderVeyronRecentActivity() {
    var el = document.getElementById('vmr-recent-activity');
    if (!el) return;
    var events = typeof lastEvents !== 'undefined' ? lastEvents.slice(0, 8) : [];
    if (!events.length) {
      el.innerHTML = '<p style="color:var(--muted);font-size:.84rem">No recent activity.</p>';
      return;
    }
    el.innerHTML = events.map(function (ev) {
      var reason = ev.reason || 'Event';
      var msg = ev.message || '';
      var cls = ev.type === 'Warning' || /Failed|Error/i.test(reason) ? 'failed' :
        ev.type === 'Normal' ? 'success' : 'info';
      var label = msg.length > 72 ? msg.slice(0, 72) + '…' : msg || reason;
      return '<div class="vmr-activity-row ' + cls + '"><span class="vmr-activity-dot"></span>' +
        '<span class="vmr-activity-text">' + esc(label) + '</span>' +
        '<span class="vmr-activity-badge">' + esc(cls === 'success' ? 'Success' : cls === 'failed' ? 'Failed' : cls === 'failed' ? 'Failed' : ev.type || 'Info') + '</span></div>';
    }).join('');
  };

  window.renderVeyronMissionBottom = function renderVeyronMissionBottom() {
    var el = document.getElementById('vmr-mission-bottom');
    if (!el) return;
    var ov = typeof lastOverview !== 'undefined' ? lastOverview : {};
    var n = ov.nodes || {};
    var c = ov.cluster || {};
    var vms = typeof vmData !== 'undefined' ? vmData : [];
    var cost = typeof lastCostSummary !== 'undefined' ? lastCostSummary : '—';
    var forecast = typeof lastCostForecast !== 'undefined' ? lastCostForecast : '—';
    var cpuPct = n.total_cpu_allocatable > 0 ? Math.round((c.total_vcpus_allocated / n.total_cpu_allocatable) * 100) : 0;
    var memPct = n.total_memory_allocatable_gb > 0 ? Math.round((c.total_memory_allocated_gb / n.total_memory_allocatable_gb) * 100) : 0;
    el.innerHTML =
      '<div class="vmr-bottom-card"><div class="vmr-panel-title">Cost Overview</div>' +
      '<div class="vmr-bottom-stat">' + esc(cost) + '</div><div class="vmr-bottom-sub">Est. monthly · Forecast ' + esc(forecast) + '</div>' +
      '<div class="vmr-bottom-bars"><div class="vmr-bottom-bar"><span>CPU alloc</span><span>' + cpuPct + '%</span></div>' +
      '<div class="vmr-bottom-bar-track"><div class="vmr-bottom-bar-fill" style="width:' + cpuPct + '%"></div></div></div>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" style="margin-top:10px" onclick="navigate(\'costs\')">Open Cost Explorer</button></div>' +
      '<div class="vmr-bottom-card"><div class="vmr-panel-title">Cluster Nodes</div>' +
      '<div class="vmr-bottom-stat">' + esc(n.ready_nodes != null ? n.ready_nodes + '/' + n.total_nodes : '—') + '</div>' +
      '<div class="vmr-bottom-sub">Ready nodes · ' + esc(n.total_nodes != null ? n.total_nodes + ' total' : '—') + '</div>' +
      '<div class="vmr-bottom-bars"><div class="vmr-bottom-bar"><span>Memory alloc</span><span>' + memPct + '%</span></div>' +
      '<div class="vmr-bottom-bar-track"><div class="vmr-bottom-bar-fill mem" style="width:' + memPct + '%"></div></div></div>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" style="margin-top:10px" onclick="navigate(\'nodes\')">View nodes</button></div>' +
      '<div class="vmr-bottom-card"><div class="vmr-panel-title">Workload Overview</div>' +
      '<div class="vmr-bottom-stat">' + vms.length + ' VMs</div>' +
      '<div class="vmr-bottom-sub">' + vms.filter(function (v) { return v.status === 'Running'; }).length + ' running · ' +
      vms.filter(function (v) { return v.status === 'Failed' || v.status === 'Error'; }).length + ' issues</div>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" style="margin-top:10px" onclick="navigate(\'workloads\')">View workloads</button></div>';
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
    var stats = 'CPU: ' + esc(vm.cpu || '—') + ' · Memory: ' + esc(vm.memory || '—');
    var extras = '<div class="vmr-fleet-card-stats">' +
      '<span>Network: ' + (isRunning ? 'Live' : '—') + '</span>' +
      '<span>Backup: Missing</span>' +
      '<span>Guest: ' + (isRunning ? 'Detected' : 'unknown') + '</span></div>';
    if (!isRunning && isIssue) {
      extras = '<div class="vmr-fleet-card-stats issue-text">Last issue: ImagePullBackOff</div>';
    }
    var actions = '';
    if (showActions !== false) {
      if (isRunning) {
        actions = '<button type="button" class="glass-btn-primary glass-btn-sm" ' + onStopHandler('openConnectModal(' + jsArgs(ns, vm.name) + ')') + '>Console</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('navigateToVmCapsule(' + jsArgs(ns, vm.name) + ')') + '>Capsule</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('navigate(\'snapshots\')') + '>Snapshot</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openCopilotNetwork(' + jsArgs(ns, vm.name) + ')') + '>Network</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openAskZeus(' + jsArgs('VM ' + vm.name) + ')') + '>More</button>';
      } else {
        actions = '<button type="button" class="glass-btn-primary glass-btn-sm" ' + onStopHandler('vmAction(' + jsArgs(ns, vm.name, 'start') + ')') + '>Start</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openCopilotDoctor(' + jsArgs(ns, vm.name) + ')') + '>Diagnose</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openCreateModal()') + '>Edit HW</button>' +
          '<button type="button" class="glass-btn-destructive glass-btn-sm" ' + onStopHandler('vmDelete(' + jsArgs(ns, vm.name) + ')') + '>Delete</button>';
      }
    }
    var spark = isRunning
      ? '<div class="vmr-fleet-spark" aria-hidden="true"><span style="height:60%"></span><span style="height:45%"></span><span style="height:70%"></span><span style="height:55%"></span><span style="height:65%"></span></div>'
      : '';
    return '<div class="' + cardCls + '" data-vm-ns="' + esc(ns) + '" data-vm-name="' + esc(vm.name) + '" ' + onHandler('selectVm(' + jsArgs(ns, vm.name) + ')') + '>' +
      '<div class="vmr-fleet-card-head"><div><span style="font-size:1.2rem;margin-right:8px">' + icon + '</span>' +
      '<span class="vmr-fleet-card-name">' + esc(vm.name) + '</span></div>' +
      '<span class="vm-badge ' + (isRunning ? 'running' : 'stopped') + '">' + esc(vm.status) + '</span></div>' +
      '<div class="vmr-fleet-card-meta">' + meta + '<br>' + stats + extras + spark + '</div>' +
      '<div class="vmr-fleet-actions">' + actions + '</div></div>';
  };

  window.renderVmrFleetTable = function renderVmrFleetTable(vms) {
    var rows = vms.map(function (vm) {
      var ns = vm.namespace || 'default';
      var isRunning = vm.status === 'Running';
      var osIcon = typeof osFamilyIcon === 'function' ? osFamilyIcon(typeof guessOsFamily === 'function' ? guessOsFamily(vm) : 'linux') : '◫';
      return '<tr class="clickable-row" onclick="selectVm(' + jsArgs(ns, vm.name) + ')">' +
        '<td>' + esc(vm.name) + '</td>' +
        '<td><span class="vm-badge ' + (isRunning ? 'running' : 'stopped') + '">' + esc(vm.status) + '</span></td>' +
        '<td>' + esc(ns) + '</td>' +
        '<td>' + esc(vm.node || '—') + '</td>' +
        '<td>' + esc(vm.ip || '—') + '</td>' +
        '<td>' + osIcon + '</td>' +
        '<td>' + esc(vm.cpu || '—') + '</td>' +
        '<td>' + esc(vm.memory || '—') + '</td>' +
        '<td>—</td>' +
        '<td style="color:var(--orange)">Missing</td>' +
        '<td style="color:' + (isRunning ? 'var(--green)' : 'var(--muted)') + '">' + (isRunning ? 'Detected' : '—') + '</td>' +
        '<td>' + esc(vm.age || '—') + '</td>' +
        '<td><button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('selectVm(' + jsArgs(ns, vm.name) + ')') + '>Open</button></td></tr>';
    }).join('');
    return '<div style="overflow-x:auto"><table class="table" style="min-width:900px"><thead><tr>' +
      '<th>Name</th><th>State</th><th>Namespace</th><th>Node</th><th>IP</th><th>OS</th>' +
      '<th>CPU</th><th>Memory</th><th>Disks</th><th>Backup</th><th>Guest Agent</th><th>Age</th><th></th>' +
      '</tr></thead><tbody>' + rows + '</tbody></table></div>';
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
    var issues = vms.filter(function (v) { return v.status === 'Failed' || v.status === 'Error'; }).length;
    var cost = typeof lastCostSummary !== 'undefined' ? lastCostSummary : '—';
    renderVmrMetricsStrip('vmr-fleet-metrics', [
      { label: 'Total VMs', value: vms.length },
      { label: 'Running', value: running, tone: 'ok' },
      { label: 'Stopped', value: stopped },
      { label: 'Issues', value: issues, tone: issues ? 'bad' : '' },
      { label: 'Drift', value: '—' },
      { label: 'Est. Cost', value: cost }
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
      { id: 'all', label: 'All Templates' },
      { id: 'linux', label: 'Linux' },
      { id: 'windows', label: 'Windows' },
      { id: 'enterprise', label: 'Enterprise' },
      { id: 'cloud', label: 'Cloud Native' },
      { id: 'bsd', label: 'BSD' },
      { id: 'talos', label: 'Talos OS' },
      { id: 'custom', label: 'Custom Images' }
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
        if (cat === 'cloud') return /flatcar|arch|k3s|rancher/.test((t.id || '').toLowerCase());
        if (cat === 'bsd') return /bsd|freebsd|openbsd/.test((t.id || '').toLowerCase());
        if (cat === 'talos') return /talos/.test((t.id || '').toLowerCase());
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
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('openFoundryPreview(' + jsArgs(t.id) + ')') + '>Preview</button>' +
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
      {
        ok: p.kubevirt_api_ok, label: 'KubeVirt API',
        tagline: 'API is healthy · VirtualMachine CRD reachable',
        detail: p.kubevirt_api_message || 'VirtualMachine API list',
        problem: 'KubeVirt API not reachable',
        impact: 'Cannot create, list, or manage VMs.',
        rootCause: 'KubeVirt operator not running or CRDs not installed.',
        fix: 'Install KubeVirt: kubectl apply -f https://github.com/kubevirt/kubevirt/releases/latest/download/kubevirt-operator.yaml',
        cmd: 'kubectl get kubevirt -n kubevirt'
      },
      {
        ok: p.kubevirt_control_plane_ok, label: 'KubeVirt Controller',
        tagline: 'Controller manager and API server running',
        detail: p.kubevirt_detail || 'virt-controller, virt-api, virt-handler pods',
        problem: 'KubeVirt control plane pods not ready',
        impact: 'VM scheduling, migration, and lifecycle operations unavailable.',
        rootCause: 'virt-controller or virt-handler pods are not Running.',
        fix: 'Check pod status in kubevirt namespace',
        cmd: 'kubectl get pods -n kubevirt'
      },
      {
        ok: p.cdi_operator_ok, label: 'CDI Operator',
        tagline: 'UploadProxy ready · DataVolume CRD installed',
        detail: p.cdi_detail || 'Containerized Data Importer',
        problem: 'CDI Operator not detected',
        impact: 'Cannot import QCOW2/VMDK images via DataVolume. Template import will fail.',
        rootCause: 'CDI not installed or cdi.kubevirt.io CRD missing.',
        fix: 'Install CDI: kubectl apply -f https://github.com/kubevirt/containerized-data-importer/releases/latest/download/cdi-operator.yaml',
        cmd: 'kubectl get cdi -n cdi'
      },
      {
        ok: p.forge_vm_ready, label: 'Forge VM',
        tagline: 'Templates synced · Forge workflow ready',
        detail: p.forge_vm_ready ? 'Dashboard forge workflow ready' : 'Resolve KubeVirt first',
        problem: 'Forge VM workflow unavailable',
        impact: 'Cannot create VMs from the dashboard wizard.',
        rootCause: 'KubeVirt API check failed — resolve it first.',
        fix: 'Resolve KubeVirt API check, then recheck stack health.',
        cmd: 'kubectl get virtualmachines -A'
      }
    ];
    var passed = checks.filter(function (c) { return c.ok; }).length;
    var allOk = passed === checks.length;
    renderVmrPageHero('vmr-stack-hero', 'Veyron Stack Health',
      passed + ' / ' + checks.length + ' — ' + (allOk ? 'All Systems Operational' : (checks.length - passed) + ' issue(s) detected'),
      '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="openPlatformInstallModal()">Install Stack</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="fetchStackHealth()">Recheck</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Diagnose KubeVirt stack issues\')">Ask Veyron</button>');
    var ringCls = allOk ? '' : passed >= 2 ? ' warn' : ' bad';
    el.innerHTML =
      '<div class="vmr-stack-hero">' +
        '<div class="vmr-stack-ring' + ringCls + '" style="color:' + (allOk ? 'var(--green)' : passed >= 2 ? 'var(--orange)' : 'var(--red)') + ';border-color:' + (allOk ? 'var(--green)' : passed >= 2 ? 'var(--orange)' : 'var(--red)') + '">' +
          '<div style="font-size:1.4rem;font-weight:800">' + passed + '/' + checks.length + '</div>' +
          '<div style="font-size:.68rem;color:var(--muted)">' + (allOk ? 'All clear' : 'Issues') + '</div>' +
        '</div>' +
        '<div class="vmr-stack-timeline">' +
          ['CRDs', 'RBAC', 'API', 'Controller', 'CDI', 'Template', 'Forge VM'].map(function (s, i) {
            var ok = i < passed + 2;
            return '<span style="color:' + (ok ? 'var(--green)' : 'var(--muted)') + '">' + s + (i < 6 ? ' <span style="opacity:.4">→</span>' : '') + '</span>';
          }).join(' ') +
        '</div>' +
      '</div>' +
      '<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:14px;margin-top:16px">' +
      checks.map(function (c) {
        if (c.ok) {
          return '<div class="vmr-panel" style="border-left:3px solid var(--green)">' +
            '<div style="display:flex;justify-content:space-between;align-items:flex-start">' +
              '<strong style="color:var(--text-bright,#f7f9fb)">' + esc(c.label) + '</strong>' +
              '<span style="color:var(--green);font-size:.78rem;font-weight:700">PASSED</span>' +
            '</div>' +
            '<div style="font-size:.8rem;color:var(--muted);margin-top:6px">' + esc(c.tagline) + '</div>' +
            '</div>';
        }
        return '<div class="vmr-panel" style="border-left:3px solid var(--red)">' +
          '<div style="display:flex;justify-content:space-between;align-items:flex-start;margin-bottom:8px">' +
            '<strong style="color:var(--text-bright,#f7f9fb)">' + esc(c.label) + '</strong>' +
            '<span style="color:var(--red);font-size:.78rem;font-weight:700">FAILED</span>' +
          '</div>' +
          '<div style="font-size:.8rem;margin-bottom:6px"><span style="color:var(--orange);font-weight:600">Problem: </span><span style="color:var(--muted)">' + esc(c.problem) + '</span></div>' +
          '<div style="font-size:.8rem;margin-bottom:6px"><span style="color:var(--orange);font-weight:600">Impact: </span><span style="color:var(--muted)">' + esc(c.impact) + '</span></div>' +
          '<div style="font-size:.8rem;margin-bottom:10px"><span style="color:var(--muted);font-weight:600">Root cause: </span><span style="color:var(--muted)">' + esc(c.rootCause) + '</span></div>' +
          '<div style="display:flex;flex-wrap:wrap;gap:8px">' +
            '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="openAskZeus(' + jsArgs('Fix: ' + c.problem) + ')">Fix with Veyron</button>' +
            '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigator.clipboard&&navigator.clipboard.writeText(' + jsArgs(c.cmd) + ')">Copy command</button>' +
            '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs('Show logs for ' + c.label) + ')">Open logs</button>' +
          '</div>' +
          '</div>';
      }).join('') +
      '</div>';
  };

  window.consoleHubRfb = null;
  window.consoleHubVm = null;

  window.disconnectConsoleHubVnc = function disconnectConsoleHubVnc() {
    if (window.consoleHubRfb) {
      try { window.consoleHubRfb.disconnect(); } catch (e) { /* ignore */ }
      window.consoleHubRfb = null;
    }
    var container = document.getElementById('vmr-console-vnc-container');
    if (container) container.querySelectorAll('canvas, div').forEach(function (el) { el.remove(); });
    var ph = document.getElementById('vmr-console-vnc-placeholder');
    if (ph) ph.style.display = 'none';
  };

  window.setConsoleHubVncStatus = function setConsoleHubVncStatus(text, tone) {
    var el = document.getElementById('vmr-console-vnc-status');
    if (!el) return;
    el.textContent = text;
    el.style.color = tone === 'ok' ? 'var(--green)' : tone === 'bad' ? 'var(--red)' : 'var(--muted)';
  };

  window.onConsoleHubVncPresetChange = function onConsoleHubVncPresetChange(value) {
    try { localStorage.setItem('vmrogue_vnc_preset', value); } catch (e) { /* ignore */ }
    if (window.consoleHubVm && window.consoleHubRfb) {
      connectConsoleHubInline(window.consoleHubVm.ns, window.consoleHubVm.name);
    }
  };

  window.openConsoleHubFullscreen = function openConsoleHubFullscreen() {
    if (!window.consoleHubVm) return;
    var h = window.consoleHubVm;
    disconnectConsoleHubVnc();
    if (typeof openVncConsole === 'function') openVncConsole(h.ns, h.name);
  };

  window.connectConsoleHubInline = async function connectConsoleHubInline(ns, name) {
    disconnectConsoleHubVnc();
    var embed = document.getElementById('vmr-console-embed');
    var container = document.getElementById('vmr-console-vnc-container');
    var placeholder = document.getElementById('vmr-console-vnc-placeholder');
    var alt = document.getElementById('vmr-console-alt-panel');
    var toolbar = document.getElementById('vmr-console-vnc-toolbar');
    if (!embed || !container || !placeholder) return;
    embed.style.display = '';
    if (alt) alt.style.display = 'none';
    if (toolbar) toolbar.style.display = '';
    container.style.display = '';
    placeholder.style.display = 'block';
    placeholder.textContent = 'Loading noVNC…';
    setConsoleHubVncStatus('Connecting…', '');

    var presetSel = document.getElementById('vmr-console-vnc-preset');
    if (presetSel) {
      var pv = localStorage.getItem('vmrogue_vnc_preset') || 'balanced';
      presetSel.value = (pv === 'lan' || pv === 'low' || pv === 'balanced') ? pv : 'balanced';
    }

    if (!window.RFB) {
      try {
        var module = await import('/assets/novnc.min.js');
        window.RFB = module.default;
        if (!window.RFB) throw new Error('RFB not found');
      } catch (e) {
        placeholder.textContent = 'Failed to load VNC client: ' + (e.message || String(e));
        setConsoleHubVncStatus('Load failed', 'bad');
        return;
      }
    }

    var url;
    try {
      url = await wsConsoleUrl('/api/v1/vms/' + encodeURIComponent(ns) + '/' + encodeURIComponent(name) + '/vnc');
    } catch (e) {
      placeholder.textContent = 'VNC ticket error: ' + (e.message || String(e));
      setConsoleHubVncStatus('Ticket error', 'bad');
      return;
    }

    placeholder.style.display = 'none';
    var presets = typeof VNC_ENCODE_PRESETS !== 'undefined' ? VNC_ENCODE_PRESETS : {
      lan: { qualityLevel: 9, compressionLevel: 2, clipViewport: true },
      balanced: { qualityLevel: 6, compressionLevel: 5, clipViewport: true },
      low: { qualityLevel: 3, compressionLevel: 8, clipViewport: true }
    };
    var pid = localStorage.getItem('vmrogue_vnc_preset') || 'balanced';
    var enc = presets[pid] || presets.balanced;

    try {
      window.consoleHubRfb = new window.RFB(container, url, {
        wsProtocols: ['binary'],
        qualityLevel: enc.qualityLevel,
        compressionLevel: enc.compressionLevel,
        clipViewport: enc.clipViewport
      });
      window.consoleHubRfb.scaleViewport = true;
      window.consoleHubRfb.resizeSession = false;
      window.consoleHubRfb.addEventListener('connect', function () {
        setConsoleHubVncStatus('Connected', 'ok');
      });
      window.consoleHubRfb.addEventListener('disconnect', function (e) {
        var reason = (e.detail && e.detail.reason) || 'Disconnected';
        setConsoleHubVncStatus(reason, 'bad');
      });
      window.consoleHubRfb.addEventListener('credentialsrequired', function () {
        window.consoleHubRfb.sendCredentials({ password: '' });
      });
    } catch (e) {
      placeholder.style.display = 'block';
      placeholder.textContent = 'VNC error: ' + (e.message || String(e));
      setConsoleHubVncStatus('Error', 'bad');
    }
  };

  window.renderConsoleHubVmr = function renderConsoleHubVmr() {
    renderVmrPageHero('vmr-console-hero', 'Veyron ConsoleHub',
      'Secure browser console access for virtual machines.',
      '');
    var sel = document.getElementById('vmr-console-vm-select');
    var vms = typeof vmData !== 'undefined' ? vmData : [];
    if (sel) {
      sel.innerHTML = '<option value="">— Select VM —</option>' + vms.map(function (vm) {
        var ns = vm.namespace || 'default';
        return '<option value="' + esc(ns + '/' + vm.name) + '">' + esc(vm.name) + ' (' + esc(vm.status) + ')</option>';
      }).join('');
    }
    window.vmrConsoleProto = window.vmrConsoleProto || 'vnc';
    onConsoleHubVmSelect();
  };

  window.setConsoleHubProto = function setConsoleHubProto(p) {
    window.vmrConsoleProto = p;
    document.querySelectorAll('.vmr-console-protocol-tabs button').forEach(function (b) {
      b.classList.toggle('active', b.dataset.proto === p);
    });
    onConsoleHubVmSelect();
  };

  window.onConsoleHubVmSelect = function onConsoleHubVmSelect() {
    var sel = document.getElementById('vmr-console-vm-select');
    var status = document.getElementById('vmr-console-status');
    var actions = document.getElementById('vmr-console-actions');
    var empty = document.getElementById('vmr-console-empty');
    var embed = document.getElementById('vmr-console-embed');
    if (!sel) return;
    disconnectConsoleHubVnc();
    var val = sel.value;
    if (!val) {
      window.consoleHubVm = null;
      if (status) status.textContent = 'Select a running VM';
      if (actions) actions.innerHTML = '';
      if (empty) empty.style.display = '';
      if (embed) embed.style.display = 'none';
      return;
    }
    var parts = val.split('/');
    var ns = parts[0];
    var name = parts.slice(1).join('/');
    var vm = (typeof vmData !== 'undefined' ? vmData : []).find(function (v) {
      return v.name === name && (v.namespace || 'default') === ns;
    });
    window.consoleHubVm = { ns: ns, name: name, vm: vm };
    var running = vm && vm.status === 'Running';
    var proto = window.vmrConsoleProto || 'vnc';
    if (status) {
      status.innerHTML = running
        ? '<span style="color:var(--green)">Connected-ready</span> · Protocol: ' + esc(proto.toUpperCase()) + ' · ' + esc(ns + '/' + name)
        : '<span style="color:var(--orange)">Console offline</span> · VM stopped';
    }
    if (actions) {
      if (running) {
        actions.innerHTML =
          '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="connectConsoleHubForProto()">Connect inline</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openConnectModal(' + jsArgs(ns, name, proto) + ')">Open window</button>' +
          (typeof isWindowsVm === 'function' && vm && isWindowsVm(vm)
            ? '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openRdpSheet(' + jsArgs(ns, name) + ')">RDP</button>'
            : '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="showSshPopover(' + jsArgs(ns, name) + ')">SSH</button>');
      } else {
        actions.innerHTML =
          '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="vmAction(' + jsArgs(ns, name, 'start') + ')">Start VM</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'events\')">View Events</button>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openCopilotDoctor(' + jsArgs(ns, name) + ')">Diagnose</button>';
      }
    }
    if (empty) empty.style.display = running ? 'none' : '';
    if (!running) {
      if (embed) embed.style.display = 'none';
      return;
    }
    connectConsoleHubForProto();
  };

  window.connectConsoleHubForProto = function connectConsoleHubForProto() {
    if (!window.consoleHubVm) return;
    var ns = window.consoleHubVm.ns;
    var name = window.consoleHubVm.name;
    var vm = window.consoleHubVm.vm;
    var proto = window.vmrConsoleProto || 'vnc';
    var embed = document.getElementById('vmr-console-embed');
    var alt = document.getElementById('vmr-console-alt-panel');
    var toolbar = document.getElementById('vmr-console-vnc-toolbar');
    disconnectConsoleHubVnc();
    if (proto === 'vnc') {
      if (embed) embed.style.display = '';
      if (alt) alt.style.display = 'none';
      connectConsoleHubInline(ns, name);
      return;
    }
    if (embed) embed.style.display = '';
    if (toolbar) toolbar.style.display = 'none';
    var container = document.getElementById('vmr-console-vnc-container');
    if (container) container.style.display = 'none';
    if (alt) {
      alt.style.display = '';
      var hint = proto === 'rdp' && typeof isWindowsVm === 'function' && vm && isWindowsVm(vm)
        ? 'Remote Desktop opens in a dedicated sheet with NodePort details.'
        : proto === 'ssh'
          ? 'SSH connection details open in a popover — use your terminal client.'
          : 'Serial console opens in a dedicated window for interactive shell access.';
      alt.innerHTML = '<div class="vmr-console-alt-inner"><h3>' + esc(proto.toUpperCase()) + ' console</h3><p>' + esc(hint) + '</p>' +
        '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="openConnectModal(' + jsArgs(ns, name, proto) + ')">Open ' + esc(proto.toUpperCase()) + '</button></div>';
    }
  };

  window.forgeWizardStep = 1;
  window.openForgeWizard = function openForgeWizard() {
    window.forgeWizardStep = 1;
    updateForgeWizardUi();
    var modal = document.getElementById('forge-wizard-modal');
    if (modal) modal.classList.add('open');
    if (typeof fetchTemplates === 'function') fetchTemplates();
    if (typeof fetchProfiles === 'function') fetchProfiles();
    syncForgeWizardFromLegacy();
    var tpl = document.getElementById('forge-wiz-template');
    var legacyTpl = document.getElementById('vm-template');
    if (tpl && legacyTpl && legacyTpl.options.length > 1 && tpl.options.length <= 1) {
      tpl.innerHTML = legacyTpl.innerHTML;
    }
    renderForgeWizardProfiles();
  };
  window.closeForgeWizard = function closeForgeWizard() {
    var modal = document.getElementById('forge-wizard-modal');
    if (modal) modal.classList.remove('open');
  };
  function syncForgeWizardFromLegacy() {
    var map = [['vm-name', 'forge-wiz-name'], ['vm-namespace', 'forge-wiz-namespace'], ['vm-cpus', 'forge-wiz-cpus'],
      ['vm-memory', 'forge-wiz-memory'], ['vm-disk', 'forge-wiz-disk'], ['vm-cloudinit', 'forge-wiz-cloudinit']];
    map.forEach(function (pair) {
      var a = document.getElementById(pair[0]);
      var b = document.getElementById(pair[1]);
      if (a && b && b.value === '' && a.value) b.value = a.value;
    });
  }
  function syncForgeWizardToLegacy() {
    var map = [['forge-wiz-name', 'vm-name'], ['forge-wiz-namespace', 'vm-namespace'], ['forge-wiz-cpus', 'vm-cpus'],
      ['forge-wiz-memory', 'vm-memory'], ['forge-wiz-disk', 'vm-disk'], ['forge-wiz-cloudinit', 'vm-cloudinit'],
      ['forge-wiz-template', 'vm-template']];
    map.forEach(function (pair) {
      var a = document.getElementById(pair[0]);
      var b = document.getElementById(pair[1]);
      if (a && b) b.value = a.value;
    });
    var autostart = document.getElementById('forge-wiz-autostart');
    var leg = document.getElementById('vm-autostart');
    if (autostart && leg) leg.checked = autostart.checked;
    var inet = document.getElementById('forge-wiz-internet');
    var legInet = document.getElementById('vm-allow-internet');
    if (inet && legInet) legInet.checked = inet.checked;
    var exEn = document.getElementById('forge-wiz-expose');
    var legEx = document.getElementById('vm-expose-enabled');
    if (exEn && legEx) {
      legEx.checked = exEn.checked;
      var opts = document.getElementById('vm-expose-options');
      if (opts) opts.style.display = exEn.checked ? 'block' : 'none';
    }
    var exPort = document.getElementById('forge-wiz-expose-port');
    var legPort = document.getElementById('vm-expose-port');
    var legTarget = document.getElementById('vm-expose-target');
    if (exPort && legPort) {
      legPort.value = exPort.value;
      if (legTarget) legTarget.value = exPort.value;
    }
  }
  function updateForgeWizardUi() {
    var step = window.forgeWizardStep || 1;
    document.querySelectorAll('.forge-step-pane').forEach(function (p) {
      p.classList.toggle('active', Number(p.dataset.step) === step);
    });
    document.querySelectorAll('#forge-wizard-steps button').forEach(function (b) {
      b.classList.toggle('active', Number(b.dataset.step) === step);
    });
    var next = document.getElementById('forge-wiz-next');
    var submit = document.getElementById('forge-wiz-submit');
    var submitStart = document.getElementById('forge-wiz-submit-start');
    if (next) next.style.display = step < 6 ? '' : 'none';
    if (submit) submit.style.display = step === 6 ? '' : 'none';
    if (submitStart) submitStart.style.display = step === 6 ? '' : 'none';
    if (step === 6) renderForgeWizardReview();
  }
  window.forgeWizardNext = function forgeWizardNext() {
    if (window.forgeWizardStep < 6) window.forgeWizardStep++;
    updateForgeWizardUi();
  };
  window.forgeWizardPrev = function forgeWizardPrev() {
    if (window.forgeWizardStep > 1) window.forgeWizardStep--;
    updateForgeWizardUi();
  };
  window.renderForgeWizardProfiles = function renderForgeWizardProfiles() {
    var el = document.getElementById('forge-wiz-profiles');
    if (!el) return;
    var cache = typeof profileCache !== 'undefined' ? profileCache : [];
    var sel = typeof selectedProfile !== 'undefined' ? selectedProfile : '';
    if (!cache.length) {
      el.innerHTML = '<p style="font-size:.84rem;color:var(--muted)">Loading profiles…</p>';
      return;
    }
    el.innerHTML = cache.map(function (p) {
      return '<div class="profile-card' + (sel === p.name ? ' selected' : '') + '" onclick="selectProfile(' + jsArgs(p.name) + ');syncForgeWizardHardware();renderForgeWizardProfiles()">' +
        '<div class="pc-name">' + esc(p.name) + '</div>' +
        '<div class="pc-spec">' + p.cpu_cores + 'C / ' + esc(p.memory) + '</div></div>';
    }).join('');
  };
  window.syncForgeWizardHardware = function syncForgeWizardHardware() {
    var map = [['vm-cpus', 'forge-wiz-cpus'], ['vm-memory', 'forge-wiz-memory'], ['vm-disk', 'forge-wiz-disk']];
    map.forEach(function (pair) {
      var a = document.getElementById(pair[0]);
      var b = document.getElementById(pair[1]);
      if (a && b && a.value) b.value = a.value;
    });
  };
  window.renderForgeWizardReview = function renderForgeWizardReview() {
    var el = document.getElementById('forge-wiz-review');
    if (!el) return;
    var name = (document.getElementById('forge-wiz-name') || {}).value || '—';
    var ns = (document.getElementById('forge-wiz-namespace') || {}).value || 'default';
    var tpl = (document.getElementById('forge-wiz-template') || {}).value || '—';
    var cpus = (document.getElementById('forge-wiz-cpus') || {}).value || '2';
    var mem = (document.getElementById('forge-wiz-memory') || {}).value || '4Gi';
    var disk = (document.getElementById('forge-wiz-disk') || {}).value || '20Gi';
    var prof = typeof selectedProfile !== 'undefined' && selectedProfile ? selectedProfile : '—';
    var inet = (document.getElementById('forge-wiz-internet') || {}).checked;
    var expose = (document.getElementById('forge-wiz-expose') || {}).checked;
    var yaml = 'apiVersion: kubevirt.io/v1\nkind: VirtualMachine\nmetadata:\n  name: ' + name + '\n  namespace: ' + ns +
      '\nspec:\n  template:\n    spec:\n      domain:\n        cpu:\n          cores: ' + cpus + '\n        resources:\n          requests:\n            memory: ' + mem;
    el.innerHTML = '<div class="vmr-forge-review-grid">' +
      '<div><strong>Name:</strong> ' + esc(name) + '</div>' +
      '<div><strong>Namespace:</strong> ' + esc(ns) + '</div>' +
      '<div><strong>Template:</strong> ' + esc(tpl) + '</div>' +
      '<div><strong>Profile:</strong> ' + esc(prof) + '</div>' +
      '<div><strong>CPU/Mem/Disk:</strong> ' + esc(cpus) + ' / ' + esc(mem) + ' / ' + esc(disk) + '</div>' +
      '<div><strong>Internet:</strong> ' + (inet ? 'Allowed' : 'Denied') + '</div>' +
      (expose ? '<div><strong>Expose:</strong> port ' + esc((document.getElementById('forge-wiz-expose-port') || {}).value || '22') + '</div>' : '') +
      '</div><pre class="vmr-forge-yaml">' + esc(yaml) + '</pre>';
  };
  window.submitForgeWizard = function submitForgeWizard(startAfter) {
    syncForgeWizardToLegacy();
    if (startAfter) {
      var autostart = document.getElementById('vm-autostart');
      if (autostart) autostart.checked = true;
    }
    closeForgeWizard();
    if (typeof forgeVM === 'function') forgeVM();
  };
  window.patchOpenCreateModal = function patchOpenCreateModal() {
    if (window._veyronPatchedCreate) return;
    window._veyronPatchedCreate = true;
    window._openCreateModalLegacy = window.openCreateModal;
    window.openCreateModal = function () {
      if (document.getElementById('forge-wizard-modal')) openForgeWizard();
      else if (window._openCreateModalLegacy) window._openCreateModalLegacy();
    };
  };
  window.patchFetchExperienceHome = function patchFetchExperienceHome() {
    if (window._veyronPatchedExpHome) return;
    window._veyronPatchedExpHome = true;
    var orig = window.fetchExperienceHome;
    if (!orig) return;
    window.fetchExperienceHome = async function () {
      await orig.apply(this, arguments);
      if (typeof renderMissionControlVmr === 'function') renderMissionControlVmr();
    };
  };

  window.patchNavigateVmr = function patchNavigateVmr() {
    if (window._vmrPatchedNavigate) return;
    window._vmrPatchedNavigate = true;
    var orig = window.navigate;
    window.navigate = function (page, opts) {
      if (typeof currentPage !== 'undefined' && currentPage === 'console-hub' && page !== 'console-hub') {
        disconnectConsoleHubVnc();
      }
      orig.apply(this, arguments);
      syncVeyronSidebar(page);
      syncVeyronTopbar();
      document.title = (VMR_PAGE_TITLES[page] || 'Veyron') + ' · Kubernetes VM Command Center';
      if (typeof updateMacPageToolbar === 'function') {
        var meta = typeof PAGE_META !== 'undefined' ? PAGE_META[page] : null;
        if (meta) updateMacPageToolbar(page, meta);
      }
      if (page === 'console-hub') renderConsoleHubVmr();
      if (page === 'stack-health') fetchStackHealth();
      if (page === 'events' && typeof renderEventIntelligenceVmr === 'function') renderEventIntelligenceVmr();
      if (page === 'network-intel') renderNetworkIntelVmr();
      if (page === 'vm-capsule') renderVmCapsuleVmr();
      if (page === 'settings') renderSettingsVmr();
      if (page === 'cilium') renderNetworkIntelVmr();
      if (page === 'workloads') renderWorkloadsVmr();
      if (page === 'nodes') renderNodesVmr();
    };
  };

  window.renderNodesVmr = function renderNodesVmr() {
    var placementEl = document.getElementById('vmr-nodes-placement');
    if (!placementEl) return;
    var vms = typeof vmData !== 'undefined' ? vmData : [];
    var ov = typeof lastOverview !== 'undefined' ? lastOverview : {};
    var n = ov.nodes || {};
    var ready = n.ready_nodes != null ? n.ready_nodes : 0;
    var total = n.total_nodes != null ? n.total_nodes : 0;
    var unscheduled = vms.filter(function (v) { return !v.node; }).length;
    renderVmrPageHero('vmr-nodes-hero', 'Veyron Cluster Nodes',
      ready + '/' + total + ' ready · ' + vms.length + ' VMs placed across cluster',
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="runNodeAdvisor&&runNodeAdvisor()">Node Advisor</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Show KubeVirt status on all nodes\')">KubeVirt Status</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'workloads\')">Workloads</button>');
    renderVmrMetricsStrip('vmr-nodes-metrics', [
      { label: 'Nodes Ready', value: ready + '/' + total, tone: ready === total ? 'ok' : 'warn' },
      { label: 'VMs Placed', value: vms.filter(function (v) { return !!v.node; }).length },
      { label: 'Unscheduled VMs', value: unscheduled, tone: unscheduled ? 'warn' : '' },
      { label: 'CPU Alloc %', value: n.total_cpu_allocatable > 0 ? Math.round(((ov.cluster || {}).total_vcpus_allocated / n.total_cpu_allocatable) * 100) + '%' : '—' },
      { label: 'Memory Alloc %', value: n.total_memory_allocatable_gb > 0 ? Math.round(((ov.cluster || {}).total_memory_allocated_gb / n.total_memory_allocatable_gb) * 100) + '%' : '—' },
      { label: 'KubeVirt', value: 'Running', tone: 'ok' }
    ]);
    /* build per-node VM placement table */
    var byNode = {};
    vms.forEach(function (v) {
      var nd = v.node || 'Unscheduled';
      if (!byNode[nd]) byNode[nd] = [];
      byNode[nd].push(v);
    });
    var nodeNames = Object.keys(byNode).sort();
    if (!nodeNames.length) { placementEl.innerHTML = ''; return; }
    var rows = nodeNames.map(function (nodeName) {
      var nodeVms = byNode[nodeName];
      var running = nodeVms.filter(function (v) { return v.status === 'Running'; }).length;
      var isUnscheduled = nodeName === 'Unscheduled';
      return '<tr>' +
        '<td><strong>' + esc(nodeName) + '</strong></td>' +
        '<td><span class="vm-badge ' + (isUnscheduled ? 'stopped' : 'running') + '">' + (isUnscheduled ? 'N/A' : 'Ready') + '</span></td>' +
        '<td>' + nodeVms.length + ' VMs · ' + running + ' running</td>' +
        '<td><span style="color:var(--green);font-size:.82rem">◉ virt-handler Ready</span></td>' +
        '<td>' +
          (isUnscheduled ? '' :
            '<button type="button" class="glass-btn-secondary glass-btn-sm" style="margin-right:4px" onclick="openAskZeus(' + jsArgs('Drain node ' + nodeName + ' safely') + ')">Drain</button>' +
            '<button type="button" class="glass-btn-secondary glass-btn-sm" style="margin-right:4px" onclick="openAskZeus(' + jsArgs('kubectl cordon ' + nodeName) + ')">Cordon</button>') +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'vms\');setTimeout(function(){if(typeof filterByNode===\'function\')filterByNode(' + jsArgs(nodeName) + ')},200)">View VMs</button>' +
        '</td></tr>';
    }).join('');
    placementEl.innerHTML = '<div class="vmr-panel" style="margin-bottom:16px">' +
      '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">' +
        '<div class="vmr-panel-title">VM Placement by Node</div>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Balance VM placement across nodes\')">Rebalance</button>' +
      '</div>' +
      '<div style="overflow-x:auto"><table class="table"><thead><tr>' +
        '<th>Node</th><th>Status</th><th>VM Load</th><th>KubeVirt</th><th>Actions</th>' +
      '</tr></thead><tbody>' + rows + '</tbody></table></div></div>';
  };

  window._vmrWorkloadsTab = window._vmrWorkloadsTab || 'all';

  window.setVmrWorkloadsTab = function setVmrWorkloadsTab(tab) {
    window._vmrWorkloadsTab = tab;
    document.querySelectorAll('.vmr-wl-tab').forEach(function (b) {
      b.classList.toggle('active', b.dataset.tab === tab);
    });
    renderWorkloadsTable();
  };

  window.renderWorkloadsVmr = function renderWorkloadsVmr() {
    var bodyEl = document.getElementById('workloads-body');
    if (!bodyEl) return;
    var vms = typeof vmData !== 'undefined' ? vmData : [];
    var pods = typeof podsCache !== 'undefined' ? podsCache : [];
    renderVmrPageHero('vmr-workloads-hero', 'Veyron Workloads',
      'VMs, VMIs, virt-launcher, CDI, KubeVirt control-plane, and Veyron pods.',
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="fetchWorkloads&&fetchWorkloads()">Refresh</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'nodes\')">Cluster Nodes</button>');
    var running = vms.filter(function (v) { return v.status === 'Running'; }).length;
    var wlPods = pods.filter(function (p) {
      return /virt-launcher|cdi-|virt-controller|virt-api|virt-handler|virt-operator|vmrogue/.test((p.name || ''));
    });
    renderVmrMetricsStrip('vmr-workloads-metrics', [
      { label: 'VirtualMachines', value: vms.length },
      { label: 'Running VMIs', value: running, tone: 'ok' },
      { label: 'virt-launcher pods', value: wlPods.filter(function (p) { return /virt-launcher/.test(p.name || ''); }).length },
      { label: 'CDI pods', value: wlPods.filter(function (p) { return /cdi-/.test(p.name || ''); }).length },
      { label: 'KubeVirt ctrl', value: wlPods.filter(function (p) { return /virt-controller|virt-api|virt-handler|virt-operator/.test(p.name || ''); }).length },
      { label: 'Veyron pods', value: wlPods.filter(function (p) { return /vmrogue/.test(p.name || ''); }).length }
    ]);
    /* tab bar */
    var tableContainer = document.getElementById('workloads-table');
    if (tableContainer) {
      var tabs = [
        { id: 'all', label: 'All' },
        { id: 'vms', label: 'VMs' },
        { id: 'vmis', label: 'VMIs' },
        { id: 'virt-launcher', label: 'virt-launcher' },
        { id: 'cdi', label: 'CDI' },
        { id: 'kubevirt-ctrl', label: 'KubeVirt ctrl' },
        { id: 'veyron', label: 'Veyron' }
      ];
      var tabBar = document.getElementById('vmr-wl-tabbar');
      if (!tabBar) {
        tabBar = document.createElement('div');
        tabBar.id = 'vmr-wl-tabbar';
        tabBar.className = 'vmr-capsule-tabs';
        tabBar.style.marginBottom = '12px';
        tableContainer.parentElement && tableContainer.parentElement.insertBefore(tabBar, tableContainer);
      }
      tabBar.innerHTML = tabs.map(function (t) {
        return '<button type="button" class="vmr-capsule-tab-btn vmr-wl-tab' + (window._vmrWorkloadsTab === t.id ? ' active' : '') +
          '" data-tab="' + esc(t.id) + '" onclick="setVmrWorkloadsTab(\'' + t.id + '\')">' + esc(t.label) + '</button>';
      }).join('');
    }
    window._vmrWorkloadsVms = vms;
    window._vmrWorkloadsPods = pods;
    renderWorkloadsTable();
  };

  window.renderWorkloadsTable = function renderWorkloadsTable() {
    var bodyEl = document.getElementById('workloads-body');
    if (!bodyEl) return;
    var tab = window._vmrWorkloadsTab || 'all';
    var vms = window._vmrWorkloadsVms || [];
    var pods = window._vmrWorkloadsPods || [];
    var rows = [];

    function vmRow(vm) {
      var ns = vm.namespace || 'default';
      var isRunning = vm.status === 'Running';
      return '<tr>' +
        '<td>' + esc(vm.name) + '</td>' +
        '<td><span class="glass-badge" style="background:rgba(63,124,255,.1);color:var(--blue)">VirtualMachine</span></td>' +
        '<td>' + esc(ns) + '</td>' +
        '<td><span class="vm-badge ' + (isRunning ? 'running' : 'stopped') + '">' + esc(vm.status) + '</span></td>' +
        '<td>' + esc(vm.node || '—') + '</td>' +
        '<td>—</td>' +
        '<td>' + esc(vm.age || '—') + '</td>' +
        '<td style="color:var(--muted);font-size:.78rem">VirtualMachine</td>' +
        '<td><button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('selectVm(' + jsArgs(ns, vm.name) + ')') + '>Inspect</button></td></tr>';
    }

    function podRow(pod) {
      var isOk = (pod.status || '').toLowerCase() === 'running';
      var kind = /virt-launcher/.test(pod.name || '') ? 'virt-launcher'
        : /cdi-/.test(pod.name || '') ? 'CDI Pod'
        : /virt-controller|virt-api|virt-handler|virt-operator/.test(pod.name || '') ? 'KubeVirt ctrl'
        : /vmrogue/.test(pod.name || '') ? 'Veyron'
        : 'Pod';
      return '<tr>' +
        '<td>' + esc(pod.name || '—') + '</td>' +
        '<td><span class="glass-badge" style="background:rgba(39,215,255,.08);color:var(--cyan)">' + esc(kind) + '</span></td>' +
        '<td>' + esc(pod.namespace || '—') + '</td>' +
        '<td><span class="vm-badge ' + (isOk ? 'running' : 'stopped') + '">' + esc(pod.status || '—') + '</span></td>' +
        '<td>' + esc(pod.node || '—') + '</td>' +
        '<td>' + esc(pod.restarts != null ? String(pod.restarts) : '—') + '</td>' +
        '<td>' + esc(pod.age || '—') + '</td>' +
        '<td style="color:var(--muted);font-size:.78rem">' + esc(pod.owner || '—') + '</td>' +
        '<td><button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs('Inspect pod ' + (pod.name || '')) + ')">Logs</button></td></tr>';
    }

    if (tab === 'vms' || tab === 'all') vms.forEach(function (v) { rows.push(vmRow(v)); });
    if (tab === 'vmis' || tab === 'all') {
      vms.filter(function (v) { return v.status === 'Running'; }).forEach(function (v) {
        var ns = v.namespace || 'default';
        rows.push('<tr>' +
          '<td>' + esc(v.name) + '</td>' +
          '<td><span class="glass-badge" style="background:rgba(50,224,176,.08);color:var(--green)">VMI</span></td>' +
          '<td>' + esc(ns) + '</td>' +
          '<td><span class="vm-badge running">Running</span></td>' +
          '<td>' + esc(v.node || '—') + '</td><td>—</td><td>—</td>' +
          '<td style="color:var(--muted);font-size:.78rem">VirtualMachine</td>' +
          '<td><button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('selectVm(' + jsArgs(ns, v.name) + ')') + '>Inspect</button></td></tr>');
      });
    }
    (pods || []).forEach(function (pod) {
      var n = pod.name || '';
      var isLauncher = /virt-launcher/.test(n);
      var isCdi = /cdi-/.test(n);
      var isKvCtrl = /virt-controller|virt-api|virt-handler|virt-operator/.test(n);
      var isVeyron = /vmrogue/.test(n);
      if (tab === 'all' || tab === 'virt-launcher' && isLauncher ||
          tab === 'cdi' && isCdi || tab === 'kubevirt-ctrl' && isKvCtrl ||
          tab === 'veyron' && isVeyron) {
        if (isLauncher || isCdi || isKvCtrl || isVeyron) rows.push(podRow(pod));
      }
    });

    if (!rows.length) {
      bodyEl.innerHTML = '<tr><td colspan="9" style="color:var(--muted);text-align:center;padding:24px">No workloads in this category.</td></tr>';
    } else {
      bodyEl.innerHTML = rows.join('');
    }
    var thead = bodyEl.closest('table') && bodyEl.closest('table').querySelector('thead tr');
    if (thead) thead.innerHTML = '<th>Name</th><th>Kind</th><th>Namespace</th><th>Status</th><th>Node</th><th>Restarts</th><th>Age</th><th>Owner</th><th></th>';
  };

  window._vmrEventFilter = window._vmrEventFilter || 'all';
  window._vmrEventMode = window._vmrEventMode || 'cards';

  window.renderEventIntelligenceVmr = function renderEventIntelligenceVmr() {
    var events = typeof lastEvents !== 'undefined' ? lastEvents : [];
    var warnings = events.filter(function (e) { return e.type === 'Warning'; }).length;
    var failed = events.filter(function (e) { return /Failed|Error|BackOff/i.test(e.reason || ''); }).length;
    renderVmrPageHero('vmr-events-hero', 'Veyron Event Intelligence',
      events.length + ' signals · ' + warnings + ' warnings · ' + failed + ' failures',
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="setVmrEventMode(window._vmrEventMode===\'cards\'?\'timeline\':\'cards\')" id="vmr-event-mode-btn">' +
      (window._vmrEventMode === 'timeline' ? 'Card View' : 'Timeline') + '</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="fetchEvents(true)">Refresh</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus()">Ask Veyron</button>');
    var filterBar = document.getElementById('vmr-events-filter-bar');
    if (filterBar) {
      var filters = ['all', 'Info', 'Warning', 'Failed', 'VMs', 'Images', 'KubeVirt', 'Storage', 'Network', 'Security'];
      filterBar.innerHTML = filters.map(function (f) {
        return '<button type="button" class="vmr-filter-chip' + (window._vmrEventFilter === f ? ' active' : '') +
          '" onclick="setVmrEventFilter(\'' + f + '\')">' + esc(f === 'all' ? 'All' : f) + '</button>';
      }).join('');
    }
    var el = document.getElementById('vmr-events-list');
    if (!el) return;
    if (!events.length) {
      el.innerHTML = '<p style="color:var(--muted);font-size:.84rem">No cluster events in scope.</p>';
      return;
    }
    var filter = window._vmrEventFilter || 'all';
    var filtered = events.filter(function (ev) {
      if (filter === 'all') return true;
      var r = (ev.reason || '').toLowerCase();
      var m = (ev.message || '').toLowerCase();
      var obj = (ev.involved_object || '').toLowerCase();
      if (filter === 'Info') return ev.type === 'Normal';
      if (filter === 'Warning') return ev.type === 'Warning';
      if (filter === 'Failed') return /failed|error|backoff/i.test(r);
      if (filter === 'Images') return /pull|image/i.test(r + m);
      if (filter === 'KubeVirt') return /kubevirt|virt|vminstance/i.test(obj + m);
      if (filter === 'Storage') return /pvc|volume|storage|disk/i.test(obj + m);
      if (filter === 'Network') return /network|svc|service|ingress|cilium/i.test(obj + m);
      if (filter === 'Security') return /security|rbac|forbidden|unauthorized/i.test(r + m);
      if (filter === 'VMs') return /virtualmachine/i.test(obj);
      return true;
    }).slice(0, 40);
    if (window._vmrEventMode === 'timeline') {
      el.innerHTML = '<div style="font-family:var(--font-mono,monospace);font-size:.82rem">' +
        filtered.map(function (ev) {
          var t = ev.timestamp ? ev.timestamp.slice(11, 19) : '—';
          var r = ev.reason || ev.type || 'Event';
          var cls = /Failed|Error|BackOff/i.test(r) ? 'var(--red)' :
            ev.type === 'Warning' ? 'var(--orange)' :
            ev.type === 'Normal' ? 'var(--green)' : 'var(--cyan)';
          var label = /Failed|Error|BackOff/i.test(r) ? 'Failed' :
            ev.type === 'Warning' ? 'Warning' : 'Success';
          return '<div style="display:flex;gap:14px;padding:6px 0;border-bottom:1px solid var(--line)">' +
            '<span style="color:var(--muted);width:60px;flex-shrink:0">' + esc(t) + '</span>' +
            '<span style="color:' + cls + ';width:60px;flex-shrink:0;font-weight:600">' + label + '</span>' +
            '<span style="color:var(--text-muted,#a6b0bc)">' + esc((ev.message || r).slice(0, 100)) + '</span></div>';
        }).join('') + '</div>';
    } else {
      el.innerHTML = filtered.map(function (ev) {
        return renderVmrIncidentCardFull(ev);
      }).join('');
    }
  };

  window.setVmrEventFilter = function setVmrEventFilter(f) {
    window._vmrEventFilter = f;
    renderEventIntelligenceVmr();
  };

  window.setVmrEventMode = function setVmrEventMode(m) {
    window._vmrEventMode = m;
    renderEventIntelligenceVmr();
  };

  window.renderVmrIncidentCardFull = function renderVmrIncidentCardFull(ev) {
    if (!ev) return '';
    var reason = ev.reason || ev.type_ || 'Event';
    var msg = ev.message || '';
    var obj = ev.involved_object || ev.namespace || '';
    var ts = ev.timestamp || '';
    var sev = /failed|error|backoff/i.test(reason) ? 'failed' : ev.type === 'Warning' ? 'warning' : '';
    var sevLabel = sev === 'failed' ? 'Failed' : sev === 'warning' ? 'Warning' : 'Info';
    var sevColor = sev === 'failed' ? 'var(--red)' : sev === 'warning' ? 'var(--orange)' : 'var(--cyan)';

    var impact = sev === 'failed'
      ? (/pull|image/i.test(reason + msg) ? 'VM cannot start — image unavailable.' : 'Workload may be degraded or offline.')
      : sev === 'warning' ? 'Monitor — may escalate to failure.' : 'Informational, no immediate impact.';

    var rootCause = '';
    if (/insufficient_scope|authorization|unauthorized/i.test(msg)) rootCause = 'Registry credentials missing or token scope insufficient.';
    else if (/pull|image/i.test(reason)) rootCause = 'Image name incorrect or registry unreachable.';
    else if (/oom|memory/i.test(reason + msg)) rootCause = 'Container or VM exceeded memory limit.';
    else if (/backoff/i.test(reason)) rootCause = 'Container restarting repeatedly after crash.';

    var fixes = [];
    if (/pull|image/i.test(reason + msg)) {
      fixes = ['Verify the image name and tag are correct.', 'Add an imagePullSecret if the registry is private.', 'Replace with a validated Veyron template image.', 'Re-run the Forge VM workflow.'];
    } else if (/oom|memory/i.test(reason + msg)) {
      fixes = ['Increase memory limit for the VM or pod.', 'Check for memory leaks in the guest workload.', 'Enable swap or balloon driver.'];
    } else if (/backoff/i.test(reason)) {
      fixes = ['Check pod logs for crash details.', 'Verify the container image entrypoint.', 'Review resource limits — pod may be OOM-killed.'];
    }

    return '<div class="vmr-incident-card ' + sev + '" style="margin-bottom:12px">' +
      '<div style="display:flex;justify-content:space-between;align-items:flex-start;margin-bottom:8px">' +
        '<div><strong style="font-size:.9rem;color:var(--text-bright,#f7f9fb)">' + esc(reason) + '</strong>' +
        (obj ? '<div style="font-size:.76rem;color:var(--muted);margin-top:2px">' + esc(obj) + (ev.namespace ? ' · ' + esc(ev.namespace) : '') + '</div>' : '') +
        '</div>' +
        '<div style="display:flex;gap:8px;align-items:center;flex-shrink:0">' +
          '<span style="font-size:.7rem;font-weight:700;color:' + sevColor + ';text-transform:uppercase">' + sevLabel + '</span>' +
          (ts ? '<span style="font-size:.72rem;color:var(--muted)">' + esc(ts.slice(11, 19) || ts.slice(0, 16)) + '</span>' : '') +
        '</div>' +
      '</div>' +
      (msg ? '<div style="font-size:.82rem;color:var(--muted);margin-bottom:8px;line-height:1.4">' + esc(msg.slice(0, 180)) + '</div>' : '') +
      '<div style="font-size:.78rem;margin-bottom:6px"><span style="color:' + sevColor + ';font-weight:600">Impact: </span><span style="color:var(--muted)">' + esc(impact) + '</span></div>' +
      (rootCause ? '<div style="font-size:.78rem;margin-bottom:8px"><span style="color:var(--muted,#a6b0bc);font-weight:600">Root cause: </span><span style="color:var(--muted)">' + esc(rootCause) + '</span></div>' : '') +
      (fixes.length ? '<ol style="margin:0 0 10px 18px;font-size:.78rem;color:var(--muted)">' + fixes.map(function (f) { return '<li>' + esc(f) + '</li>'; }).join('') + '</ol>' : '') +
      '<div style="display:flex;flex-wrap:wrap;gap:8px">' +
        '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="openAskZeus(' + jsArgs('Fix: ' + reason + ' — ' + msg.slice(0, 80)) + ')">Fix with Veyron</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigator.clipboard&&navigator.clipboard.writeText(' + jsArgs('kubectl get events -n ' + (ev.namespace || 'default') + ' --sort-by=.lastTimestamp') + ')">Copy kubectl</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'events\')">Open Events</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs(msg.slice(0, 120)) + ')">Ask AI</button>' +
      '</div></div>';
  };

  window.fetchVeyronAlerts = async function fetchVeyronAlerts() {
    try {
      var j = await apiJson('/api/v1/alerts' + (typeof nsParam === 'function' ? nsParam() : ''));
      window.lastVeyronAlerts = typeof asArray === 'function' ? asArray(j) : (j.data || j || []);
      var firing = lastVeyronAlerts.filter(function (a) {
        return (a.status || '').toLowerCase() === 'firing';
      }).length;
      if (typeof window.islandAlertCount !== 'undefined') window.islandAlertCount = firing;
      if (typeof currentPage !== 'undefined' && currentPage === 'dashboard' && typeof renderMissionControlVmr === 'function') {
        renderMissionControlVmr();
      }
      if (typeof syncVeyronTopbar === 'function') syncVeyronTopbar();
    } catch (e) { /* alerts optional */ }
  };

  window.patchRenderPinnedVms = function patchRenderPinnedVms() {
    if (window._veyronPinnedPatch) return;
    window._veyronPinnedPatch = true;
    var orig = window.renderPinnedVms;
    window.renderPinnedVms = function (vms) {
      var sec = document.getElementById('dc-pinned-section');
      var el = document.getElementById('dc-pinned-vms');
      if (!sec || !el) return orig ? orig(vms) : undefined;
      var list = (vms || []).slice(0, 4);
      if (!list.length) { sec.style.display = 'none'; return; }
      sec.style.display = '';
      el.innerHTML = '<div class="vmr-card-grid">' + list.map(function (v) {
        var vm = typeof v === 'object' ? v : { name: v, status: 'Running', namespace: 'default' };
        return typeof vmrFleetCard === 'function' ? vmrFleetCard(vm, true) : (typeof pinnedVmCard === 'function' ? pinnedVmCard(vm) : '');
      }).join('') + '</div>';
    };
  };

  window.patchFetchTemplatesForForge = function patchFetchTemplatesForForge() {
    if (window._veyronTplPatch) return;
    window._veyronTplPatch = true;
    var orig = window.fetchTemplates;
    if (!orig) return;
    window.fetchTemplates = async function () {
      await orig.apply(this, arguments);
      var tpl = document.getElementById('forge-wiz-template');
      var legacyTpl = document.getElementById('vm-template');
      if (tpl && legacyTpl && legacyTpl.options.length) tpl.innerHTML = legacyTpl.innerHTML;
    };
  };

  window.fetchVeyronMissionCosts = async function fetchVeyronMissionCosts() {
    try {
      var j = await apiJson('/api/v1/costs/summary');
      var s = typeof unwrapData === 'function' ? unwrapData(j) : j;
      if (s && s.total_cost != null) {
        window.lastCostSummary = '$' + Number(s.total_cost).toFixed(2);
      }
      try {
        var fc = await apiJson('/api/v1/costs/forecast');
        var f = typeof unwrapData === 'function' ? unwrapData(fc) : fc;
        if (f && f.forecast_cost != null) window.lastCostForecast = '$' + Number(f.forecast_cost).toFixed(2);
        else if (f && f.total_forecast != null) window.lastCostForecast = '$' + Number(f.total_forecast).toFixed(2);
      } catch (e2) { /* optional */ }
      if (typeof currentPage !== 'undefined' && currentPage === 'dashboard' && typeof renderMissionControlVmr === 'function') {
        renderMissionControlVmr();
      }
    } catch (e) { /* costs optional */ }
  };

  window.patchRefreshVeyron = function patchRefreshVeyron() {
    if (window._veyronPatchedRefresh) return;
    window._veyronPatchedRefresh = true;
    var orig = window.refresh;
    if (!orig) return;
    window.refresh = function () {
      orig.apply(this, arguments);
      fetchVeyronMissionCosts();
      fetchVeyronAlerts();
      if (typeof syncVeyronTopbar === 'function') syncVeyronTopbar();
    };
  };

  window.navigateToVmCapsule = function navigateToVmCapsule(ns, name) {
    window._vmCapsuleTarget = { ns: ns, name: name };
    if (typeof navigate === 'function') navigate('vm-capsule');
  };

  window.renderVmCapsuleVmr = function renderVmCapsuleVmr() {
    var el = document.getElementById('vmr-capsule-body');
    if (!el) return;
    var target = window._vmCapsuleTarget;
    var vms = typeof vmData !== 'undefined' ? vmData : [];
    var vm = target ? vms.find(function (v) { return v.name === target.name && (v.namespace || 'default') === target.ns; }) : null;
    if (!vm && vms.length) vm = vms[0];
    if (!vm) {
      el.innerHTML = '<div class="vmr-panel"><p style="color:var(--muted)">Select a VM from Fleet Command to open its capsule.</p>' +
        '<button type="button" class="glass-btn-primary glass-btn-sm" style="margin-top:12px" onclick="navigate(\'vms\')">Open Fleet Command</button></div>';
      return;
    }
    var ns = vm.namespace || 'default';
    var isRunning = vm.status === 'Running';
    var warnings = typeof islandAlertCount !== 'undefined' ? islandAlertCount : 0;
    var score = Math.max(0, 100 - (warnings * 10) - (isRunning ? 0 : 5));
    var heroActions = isRunning
      ? '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="openConnectModal(' + jsArgs(ns, vm.name) + ')">Console</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openConnectModal(' + jsArgs(ns, vm.name, 'ssh') + ')">SSH / RDP</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="vmAction(' + jsArgs(ns, vm.name, 'restart') + ')">Restart</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'snapshots\')">Snapshot</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs('VM ' + vm.name) + ')">Ask Veyron</button>'
      : '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="vmAction(' + jsArgs(ns, vm.name, 'start') + ')">Start</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openCopilotDoctor(' + jsArgs(ns, vm.name) + ')">Diagnose</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openCreateModal()">Edit Hardware</button>' +
        '<button type="button" class="glass-btn-destructive glass-btn-sm" onclick="vmDelete(' + jsArgs(ns, vm.name) + ')">Delete</button>';
    el.innerHTML =
      '<div class="vmr-capsule-hero">' +
        '<div class="vmr-capsule-hero-info">' +
          '<div class="vmr-capsule-name">' + esc(vm.name) + '</div>' +
          '<span class="vm-badge ' + (isRunning ? 'running' : 'stopped') + '">' + esc(vm.status) + '</span>' +
          '<div class="vmr-capsule-meta">' + esc(ns) + ' · ' + esc(vm.node || 'no node') + ' · ' + esc(vm.ip || 'no IP') + '</div>' +
        '</div>' +
        '<div class="vmr-capsule-hero-actions">' + heroActions + '</div>' +
      '</div>' +
      '<div class="vmr-capsule-tabs" id="vmr-capsule-tabs">' +
        ['Overview', 'Performance', 'Network', 'Storage', 'Hardware', 'Security', 'Events', 'YAML'].map(function (t, i) {
          return '<button type="button" class="vmr-capsule-tab-btn' + (i === 0 ? ' active' : '') + '" data-tab="' + i + '" onclick="setVmrCapsuleTab(' + i + ')">' + t + '</button>';
        }).join('') +
      '</div>' +
      '<div id="vmr-capsule-tab-content">' +
        '<div class="vmr-capsule-pane active" data-tab="0">' +
          '<div class="vmr-command-grid">' +
            '<div class="vmr-panel">' +
              '<div class="vmr-panel-title">Identity</div>' +
              '<div class="vmr-health-row"><span>Name</span><span>' + esc(vm.name) + '</span></div>' +
              '<div class="vmr-health-row"><span>Namespace</span><span>' + esc(ns) + '</span></div>' +
              '<div class="vmr-health-row"><span>Phase</span><span>' + esc(vm.status) + '</span></div>' +
              '<div class="vmr-health-row"><span>Node</span><span>' + esc(vm.node || '—') + '</span></div>' +
              '<div class="vmr-health-row"><span>IP</span><span>' + esc(vm.ip || '—') + '</span></div>' +
              '<div class="vmr-health-row"><span>Age</span><span>' + esc(vm.age || '—') + '</span></div>' +
            '</div>' +
            '<div class="vmr-panel">' +
              '<div class="vmr-panel-title">Performance</div>' +
              (isRunning
                ? '<div class="vmr-health-row"><span>CPU</span><span>' + esc(vm.cpu || '—') + '</span></div>' +
                  '<div class="vmr-health-row"><span>Memory</span><span>' + esc(vm.memory || '—') + '</span></div>' +
                  '<div class="vmr-health-row"><span>Guest agent</span><span style="color:var(--green)">Detected</span></div>'
                : '<p style="font-size:.84rem;color:var(--muted);margin-bottom:12px">No live metrics. Start the VM and install Guest Tools.</p>' +
                  '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="vmAction(' + jsArgs(ns, vm.name, 'start') + ')">Start VM</button>') +
            '</div>' +
          '</div>' +
          '<div class="vmr-command-grid">' +
            '<div class="vmr-panel">' +
              '<div class="vmr-panel-title">Hardware</div>' +
              '<div class="vmr-health-row"><span>CPU</span><span>' + esc(vm.cpu || '—') + '</span></div>' +
              '<div class="vmr-health-row"><span>Memory</span><span>' + esc(vm.memory || '—') + '</span></div>' +
              '<div class="vmr-health-row"><span>Boot mode</span><span>UEFI</span></div>' +
              '<div class="vmr-health-row"><span>TPM</span><span style="color:var(--orange)">Disabled</span></div>' +
              '<div class="vmr-health-row"><span>Secure Boot</span><span style="color:var(--orange)">Disabled</span></div>' +
              '<div style="display:flex;flex-wrap:wrap;gap:8px;margin-top:12px">' +
                '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openCreateModal()">Edit CPU/Memory</button>' +
                '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Enable TPM for VM ' + vm.name + '\')">Enable TPM</button>' +
                '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Enable Secure Boot for VM ' + vm.name + '\')">Enable Secure Boot</button>' +
              '</div>' +
            '</div>' +
            '<div class="vmr-panel">' +
              '<div class="vmr-panel-title">Security Posture</div>' +
              '<div style="font-size:1.4rem;font-weight:800;color:var(--cyan);margin:8px 0">' + score + ' <span style="font-size:.88rem;font-weight:400;color:var(--muted)">/ 100</span></div>' +
              '<div class="vmr-health-row"><span>Security</span><span class="warn">' + score + '%</span></div>' +
              '<div class="vmr-health-row"><span>Backup</span><span class="warn">20%</span></div>' +
              '<div class="vmr-health-row"><span>Compliance</span><span class="ok">70%</span></div>' +
              '<div class="vmr-health-row"><span>Hardening</span><span class="warn">50%</span></div>' +
              '<ul style="list-style:none;padding:8px 0 0;font-size:.78rem;color:var(--muted)">' +
                (warnings ? '<li style="color:var(--orange)">⚠ ' + warnings + ' active warnings</li>' : '') +
                '<li style="color:var(--orange)">⚠ TPM disabled</li>' +
                '<li style="color:var(--orange)">⚠ Secure Boot disabled</li>' +
                '<li style="color:var(--orange)">⚠ No backup policy</li>' +
              '</ul>' +
              '<div style="display:flex;flex-wrap:wrap;gap:8px;margin-top:12px">' +
                '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="openAskZeus(\'Auto-fix security issues for VM ' + vm.name + '\')">Fix Automatically</button>' +
                '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Explain security risks for VM ' + vm.name + '\')">Explain Risk</button>' +
              '</div>' +
            '</div>' +
          '</div>' +
        '</div>' +
        ['Performance', 'Network', 'Storage', 'Hardware', 'Security', 'Events', 'YAML'].map(function (t, i) {
          return '<div class="vmr-capsule-pane" data-tab="' + (i + 1) + '"><div class="vmr-panel"><p style="color:var(--muted);font-size:.84rem">' + t + ' data loads from the cluster API. Open <strong>Fleet Command</strong> to browse VMs, then return here.</p></div></div>';
        }).join('') +
      '</div>';
    var heroEl = document.getElementById('vmr-capsule-hero-slot');
    if (heroEl) heroEl.innerHTML = '';
  };

  window.setVmrCapsuleTab = function setVmrCapsuleTab(idx) {
    document.querySelectorAll('.vmr-capsule-tab-btn').forEach(function (b) {
      b.classList.toggle('active', Number(b.dataset.tab) === idx);
    });
    document.querySelectorAll('.vmr-capsule-pane').forEach(function (p) {
      p.classList.toggle('active', Number(p.dataset.tab) === idx);
    });
  };

  window.renderNetworkIntelVmr = function renderNetworkIntelVmr() {
    var el = document.getElementById('vmr-network-intel-body');
    if (!el) return;
    var vms = typeof vmData !== 'undefined' ? vmData : [];
    renderVmrPageHero('vmr-network-intel-hero', 'Veyron Network Intelligence',
      'VM traffic, flows, policies, and live network behavior.',
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Analyze network flows\')">Ask Veyron</button>');
    renderVmrMetricsStrip('vmr-network-intel-metrics', [
      { label: 'Live Flows', value: '—' },
      { label: 'Allowed', value: '—', tone: 'ok' },
      { label: 'Denied', value: '—', tone: 'bad' },
      { label: 'Unknown', value: '—' },
      { label: 'Policy Gaps', value: '—', tone: 'warn' },
      { label: 'External Dests', value: '—' }
    ]);
    var banner = '<div class="vmr-network-banner" id="vmr-network-packetwolf-banner">' +
      '<span style="color:var(--muted);font-size:.84rem">PacketWolf not configured. Set <code>VMROGUE_PACKETWOLF_URL</code> to enable live flow intelligence.</span>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'settings\')">Configure</button></div>';
    var vmRows = vms.slice(0, 6).map(function (vm) {
      var ns = vm.namespace || 'default';
      return '<tr><td>' + esc(vm.name) + '</td><td>' + esc(ns) + '</td>' +
        '<td style="color:var(--cyan)">—</td><td>—</td><td>—</td><td>—</td>' +
        '<td><span style="color:var(--green)">Allowed</span></td><td>—</td><td>—</td>' +
        '<td><button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs('Explain network flows for ' + vm.name) + ')">Explain</button></td></tr>';
    }).join('');
    el.innerHTML = banner +
      '<div class="vmr-panel" style="margin-top:16px">' +
        '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">' +
          '<div class="vmr-panel-title">Flow Table</div>' +
          '<div style="display:flex;gap:8px">' +
            '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Generate NetworkPolicy for all VMs\')">Generate Network Policy</button>' +
            '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Lock VM egress traffic\')">Lock VM Traffic</button>' +
          '</div>' +
        '</div>' +
        '<div style="overflow-x:auto"><table class="table" style="min-width:800px"><thead><tr>' +
          '<th>Source VM</th><th>Namespace</th><th>Destination</th><th>Protocol</th><th>Port</th><th>Bytes</th>' +
          '<th>Verdict</th><th>Process</th><th>Policy</th><th></th>' +
        '</tr></thead><tbody>' + (vmRows || '<tr><td colspan="10" style="color:var(--muted);text-align:center">No flow data. Configure PacketWolf for live intelligence.</td></tr>') + '</tbody></table></div>' +
      '</div>';
    fetchNetworkIntelBanner();
  };

  window.fetchNetworkIntelBanner = async function fetchNetworkIntelBanner() {
    try {
      var j = await apiJson('/api/v1/packetwolf/status');
      var d = typeof unwrapData === 'function' ? unwrapData(j) : j;
      var banner = document.getElementById('vmr-network-packetwolf-banner');
      if (!banner) return;
      if (d && d.healthy) {
        banner.innerHTML = '<span style="color:var(--green);font-size:.84rem">◉ PacketWolf connected · Brain healthy</span>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(\'Show live flows from PacketWolf\')">Watch Live Flows</button>';
        banner.style.borderColor = 'rgba(50,224,176,.3)';
      }
    } catch (e) { /* PacketWolf optional */ }
  };

  window.renderSettingsVmr = function renderSettingsVmr() {
    var el = document.getElementById('vmr-settings-body');
    if (!el) return;
    renderVmrPageHero('vmr-settings-hero', 'Veyron Settings',
      'Platform identity, access control, integrations, and appearance.',
      '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="toast(\'Settings saved\',\'success\')">Save Settings</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'integrations\')">Advanced</button>');

    function field(f) {
      var row = '<div style="padding:8px 0;border-bottom:1px solid var(--line)">';
      if (f.type === 'checkbox') {
        return '<label style="display:flex;gap:10px;align-items:center;padding:8px 0;border-bottom:1px solid var(--line);font-size:.84rem;cursor:pointer">' +
          '<input type="checkbox" id="' + esc(f.id) + '"' + (f.checked ? ' checked' : '') + '> ' + esc(f.label) + '</label>';
      }
      if (f.type === 'select') {
        return row + '<label class="form-label" style="font-size:.74rem;color:var(--muted)">' + esc(f.label) + '</label>' +
          '<select class="form-select" id="' + esc(f.id) + '">' +
          (f.options || []).map(function (o) { return '<option>' + esc(o) + '</option>'; }).join('') +
          '</select></div>';
      }
      if (f.type === 'badge') {
        return row + '<label class="form-label" style="font-size:.74rem;color:var(--muted)">' + esc(f.label) + '</label>' +
          '<div style="font-size:.84rem;font-family:monospace;color:' + (f.ok ? 'var(--green)' : 'var(--orange)') + '">' + esc(f.value || '—') + '</div></div>';
      }
      if (f.type === 'action') {
        return '<div style="padding:8px 0;border-bottom:1px solid var(--line)">' +
          '<label class="form-label" style="font-size:.74rem;color:var(--muted)">' + esc(f.label) + '</label>' +
          '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="' + (f.onclick || '') + '">' + esc(f.btn || 'Run') + '</button></div>';
      }
      return row + '<label class="form-label" style="font-size:.74rem;color:var(--muted)">' + esc(f.label) + '</label>' +
        '<input type="' + (f.password ? 'password' : 'text') + '" class="form-input" id="' + esc(f.id) + '" placeholder="' + esc(f.placeholder || '') + '"></div>';
    }

    function section(title, fields, extra) {
      return '<div class="vmr-panel">' +
        '<div class="vmr-panel-title">' + esc(title) + '</div>' +
        fields.map(field).join('') +
        (extra || '') +
        '</div>';
    }

    var ns = typeof currentNamespace !== 'undefined' ? currentNamespace : 'default';
    var sections = [
      /* 1 — General */
      section('General', [
        { label: 'Default namespace', type: 'text', placeholder: ns, id: 'set-default-ns' },
        { label: 'Default storage class', type: 'text', placeholder: 'standard', id: 'set-storage-class' },
        { label: 'Auto-refresh interval (s)', type: 'text', placeholder: '30', id: 'set-refresh-interval' },
        { label: 'API request timeout (s)', type: 'text', placeholder: '30', id: 'set-api-timeout' }
      ]),
      /* 2 — Cluster Context */
      section('Cluster Context', [
        { label: 'KUBECONFIG path', type: 'text', placeholder: '~/.kube/config', id: 'set-kubeconfig' },
        { label: 'KubeVirt namespace', type: 'text', placeholder: 'kubevirt', id: 'set-kubevirt-ns' },
        { label: 'CDI namespace', type: 'text', placeholder: 'cdi', id: 'set-cdi-ns' },
        { label: 'Cluster label (display name)', type: 'text', placeholder: 'production', id: 'set-cluster-label' }
      ]),
      /* 3 — Namespaces */
      section('Namespaces', [
        { label: 'Namespace auto-discovery', type: 'checkbox', id: 'set-ns-autodiscover', checked: true },
        { label: 'Allow cross-namespace queries', type: 'checkbox', id: 'set-ns-cross', checked: true },
        { label: 'Allowed namespaces (comma-separated)', type: 'text', placeholder: 'default, kube-system', id: 'set-ns-allow' }
      ]),
      /* 4 — API Keys */
      section('API Keys', [
        { label: 'API key (masked)', type: 'text', password: true, placeholder: '••••••••', id: 'set-api-key' },
        { label: 'Multi-key RBAC (admin:k1,write:k2,readonly:k3)', type: 'text', placeholder: 'VMROGUE_API_KEYS', id: 'set-api-keys-rbac' },
        { type: 'action', label: 'Rotate API key', btn: 'Rotate key', onclick: 'openAskZeus(\'Rotate the VMRogue API key\')' }
      ]),
      /* 5 — RBAC */
      section('RBAC', [
        { label: 'Read-only mode (disable writes)', type: 'checkbox', id: 'set-readonly' },
        { label: 'Allow VM delete', type: 'checkbox', id: 'set-allow-delete', checked: true },
        { label: 'Allow VM migrate', type: 'checkbox', id: 'set-allow-migrate', checked: true },
        { label: 'Minimum dashboard tier', type: 'select', options: ['Normal', 'Power', 'Advanced'], id: 'set-min-tier' }
      ]),
      /* 6 — Console Settings */
      section('Console Settings', [
        { label: 'Enable VNC console', type: 'checkbox', id: 'set-vnc', checked: true },
        { label: 'Enable RDP expose', type: 'checkbox', id: 'set-rdp', checked: true },
        { label: 'Enable SSH expose', type: 'checkbox', id: 'set-ssh', checked: true },
        { label: 'Default link quality', type: 'select', options: ['LAN (best quality)', 'Balanced', 'Low bandwidth'], id: 'set-vnc-quality' },
        { label: 'Auto-reconnect VNC', type: 'checkbox', id: 'set-vnc-reconnect', checked: true }
      ]),
      /* 7 — Template Registry */
      section('Template Registry', [
        { label: 'Template source', type: 'select', options: ['Built-in (embedded)', 'Cluster CRDs (VMTemplate)', 'Custom registry URL'], id: 'set-tpl-source' },
        { label: 'Custom registry URL', type: 'text', placeholder: 'https://templates.example.com/catalog.json', id: 'set-tpl-registry-url' },
        { type: 'action', label: 'Sync template catalog now', btn: 'Sync now', onclick: 'typeof fetchAppStore==="function"&&fetchAppStore();toast("Templates synced","success")' }
      ]),
      /* 8 — Backup Settings */
      section('Backup Settings', [
        { label: 'Default snapshot retention (days)', type: 'text', placeholder: '7', id: 'set-snap-retention' },
        { label: 'Default snapshot name prefix', type: 'text', placeholder: 'auto-', id: 'set-snap-prefix' },
        { label: 'Velero integration URL', type: 'text', placeholder: 'http://velero.velero:8085', id: 'set-velero-url' },
        { label: 'Enable scheduled snapshots', type: 'checkbox', id: 'set-snap-schedule', checked: true }
      ]),
      /* 9 — PacketWolf Integration */
      section('PacketWolf Integration', [
        { label: 'PacketWolf URL', type: 'text', placeholder: 'VMROGUE_PACKETWOLF_URL', id: 'set-packetwolf' },
        { label: 'API key (if required)', type: 'text', password: true, placeholder: '••••••••', id: 'set-packetwolf-key' },
        { label: 'Trust cluster networks', type: 'checkbox', id: 'set-pw-trust', checked: true },
        { type: 'action', label: 'Test PacketWolf connection', btn: 'Test connection', onclick: 'fetchNetworkIntelBanner&&fetchNetworkIntelBanner().then(function(){toast("PacketWolf test complete","success")})' }
      ]),
      /* 10 — Prometheus Integration */
      section('Prometheus Integration', [
        { label: 'Prometheus URL', type: 'text', placeholder: 'VMROGUE_PROMETHEUS_URL', id: 'set-prometheus' },
        { label: 'Query timeout (s)', type: 'text', placeholder: '10', id: 'set-prom-timeout' },
        { label: 'Default time range', type: 'select', options: ['Last 1 hour', 'Last 6 hours', 'Last 24 hours', 'Last 7 days'], id: 'set-prom-range' },
        { type: 'action', label: 'Test Prometheus connection', btn: 'Test connection', onclick: 'toast("Prometheus: not configured","info")' }
      ]),
      /* 11 — Appearance */
      section('Appearance', [
        { label: 'Theme', type: 'select', options: ['Veyron Dark (default)', 'Carbon Black', 'Midnight Blue', 'Steel Grey'], id: 'set-theme' },
        { label: 'Sidebar accent color', type: 'select', options: ['Electric Blue (#3f7cff)', 'Cyan (#27d7ff)', 'Violet (#9b7cff)', 'Green (#32e0b0)'], id: 'set-accent' },
        { label: 'Font size', type: 'select', options: ['Small (13px)', 'Default (14px)', 'Large (16px)'], id: 'set-font-size' },
        { label: 'Sidebar density', type: 'select', options: ['Comfortable', 'Compact', 'Ultra-compact'], id: 'set-sidebar-density' }
      ]),
      /* 12 — Audit */
      section('Audit', [
        { label: 'Show audit log in dashboard', type: 'checkbox', id: 'set-audit-show', checked: false },
        { label: 'Log API key usage', type: 'checkbox', id: 'set-audit-api', checked: true },
        { label: 'Log VM lifecycle events', type: 'checkbox', id: 'set-audit-vm', checked: true },
        { type: 'action', label: 'Export audit log (JSON)', btn: 'Export', onclick: 'toast("Audit export: check /api/v1/alerts","info")' }
      ])
    ];

    el.innerHTML = '<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(340px,1fr));gap:16px">' +
      sections.join('') +
      '</div>' +
      '<div style="display:flex;gap:8px;margin-top:20px;padding-top:16px;border-top:1px solid var(--line)">' +
      '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="toast(\'Settings saved\',\'success\')">Save Settings</button>' +
      '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigate(\'integrations\')">Advanced Integrations</button>' +
      '</div>';
  };

  window.openFoundryPreview = function openFoundryPreview(templateId) {
    var drawer = document.getElementById('vmr-foundry-drawer');
    var body = document.getElementById('vmr-fdrawer-body');
    var titleEl = document.getElementById('vmr-fdrawer-title');
    if (!drawer || !body) return;
    var tpl = null;
    if (typeof appStoreCache !== 'undefined') {
      tpl = appStoreCache.find(function (t) { return t.id === templateId || t.title === templateId; });
    }
    if (!tpl) {
      tpl = { id: templateId, title: templateId, subtitle: '', os_family: 'linux' };
    }
    var osIcons = { windows: '🪟', ubuntu: '🐧', debian: '🐧', fedora: '🎩', centos: '🎩', rhel: '🎩', linux: '🐧', bsd: '🐡', talos: '⚙', custom: '◫' };
    var isWin = (tpl.os_family || '').toLowerCase() === 'windows';
    var icon = osIcons[(tpl.os_family || '').toLowerCase()] || '◫';
    var cpu = tpl.cpu || tpl.default_cpu || '2 vCPU';
    var mem = tpl.memory || tpl.default_memory || '4 GiB';
    var disk = tpl.disk || tpl.default_disk_size || '20 Gi';
    var boot = isWin ? 'UEFI + Secure Boot' : 'UEFI';
    var tags = [];
    if (tpl.cloud_init) tags.push({ label: 'cloud-init' });
    if (isWin) tags.push({ label: 'RDP', cls: 'win' }, { label: 'VirtIO', cls: 'win' });
    else tags.push({ label: 'SSH' }, { label: 'VirtIO' });
    tags.push({ label: 'Guest Tools' });
    if (tpl.os_family === 'talos') tags.push({ label: 'K8s node' });
    var tagHtml = tags.map(function (t) {
      return '<span class="vmr-fdrawer-tag' + (t.cls ? ' ' + t.cls : '') + '">' + esc(t.label) + '</span>';
    }).join('');
    var yamlPreview = 'apiVersion: kubevirt.io/v1\nkind: VirtualMachine\nmetadata:\n  name: my-' + (tpl.id || 'vm') + '\nspec:\n  template:\n    spec:\n      domain:\n        cpu:\n          cores: 2\n        memory:\n          guest: ' + esc(mem) + '\n        devices:\n          disks:\n            - name: rootdisk\n              disk:\n                bus: virtio';
    if (titleEl) titleEl.textContent = 'TEMPLATE PREVIEW';
    body.innerHTML =
      '<div class="vmr-fdrawer-hero">' +
        '<div class="vmr-fdrawer-icon">' + icon + '</div>' +
        '<div>' +
          '<div class="vmr-fdrawer-name">' + esc(tpl.title || tpl.id) + '</div>' +
          '<div class="vmr-fdrawer-sub">' + esc(tpl.subtitle || (tpl.os_family || 'KubeVirt template')) + '</div>' +
        '</div>' +
      '</div>' +
      '<div class="vmr-fdrawer-tags">' + tagHtml + '</div>' +
      '<div class="vmr-panel" style="margin-bottom:12px">' +
        '<div class="vmr-panel-title">Default Hardware</div>' +
        '<div class="vmr-health-row"><span>CPU</span><span>' + esc(String(cpu)) + '</span></div>' +
        '<div class="vmr-health-row"><span>Memory</span><span>' + esc(String(mem)) + '</span></div>' +
        '<div class="vmr-health-row"><span>Root disk</span><span>' + esc(String(disk)) + '</span></div>' +
        '<div class="vmr-health-row"><span>Boot mode</span><span>' + esc(boot) + '</span></div>' +
        '<div class="vmr-health-row"><span>NIC</span><span>VirtIO (masquerade)</span></div>' +
        '<div class="vmr-health-row"><span>RNG</span><span style="color:var(--green)">Enabled</span></div>' +
      '</div>' +
      '<div class="vmr-panel" style="margin-bottom:12px">' +
        '<div class="vmr-panel-title">Security Readiness</div>' +
        '<div class="vmr-health-row"><span>UEFI</span><span style="color:var(--green)">✓</span></div>' +
        '<div class="vmr-health-row"><span>Secure Boot</span><span style="color:' + (isWin ? 'var(--green)' : 'var(--orange)') + '">' + (isWin ? '✓' : 'Optional') + '</span></div>' +
        '<div class="vmr-health-row"><span>TPM 2.0</span><span style="color:' + (isWin ? 'var(--green)' : 'var(--orange)') + '">' + (isWin ? '✓' : 'Optional') + '</span></div>' +
        '<div class="vmr-health-row"><span>Guest Tools</span><span style="color:var(--cyan)">Via cloud-init</span></div>' +
        '<div class="vmr-health-row"><span>cloud-init</span><span style="color:' + (isWin ? 'var(--orange)' : 'var(--green)') + '">' + (isWin ? 'config-drive' : 'NoCloud') + '</span></div>' +
      '</div>' +
      '<div class="vmr-panel" style="margin-bottom:12px">' +
        '<div class="vmr-panel-title" style="margin-bottom:8px">YAML Preview</div>' +
        '<pre style="font-size:.72rem;color:var(--muted);white-space:pre-wrap;word-break:break-all;line-height:1.5;max-height:160px;overflow:auto">' + esc(yamlPreview) + '</pre>' +
      '</div>' +
      '<div class="vmr-fdrawer-actions">' +
        '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="forgeFromTemplate(' + jsArgs(tpl.id || tpl.title) + ')">Forge VM</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openCreateModal()">Customize</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigator.clipboard&&navigator.clipboard.writeText(' + jsArgs(yamlPreview) + ')">Export YAML</button>' +
        '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs('Tell me about the ' + (tpl.title || tpl.id) + ' template') + ')">Ask Veyron</button>' +
      '</div>';
    drawer.style.display = 'flex';
    drawer.removeAttribute('aria-hidden');
  };

  window.closeFoundryPreview = function closeFoundryPreview() {
    var drawer = document.getElementById('vmr-foundry-drawer');
    if (drawer) { drawer.style.display = 'none'; drawer.setAttribute('aria-hidden', 'true'); }
  };

  window.patchSelectVm = function patchSelectVm() {
    if (window._vmrPatchedSelectVm) return;
    window._vmrPatchedSelectVm = true;

    var origSelect = window.selectVm;
    var origClear = window.clearVmSelection;

    function injectInspectorBar(ns, name) {
      var focusPane = document.getElementById('vm-focus-pane');
      if (!focusPane) return;
      var existing = document.getElementById('vmr-inspector-bar');
      if (existing) existing.remove();
      var isRunning = false;
      if (typeof vmData !== 'undefined') {
        var vm = (vmData || []).find(function (v) { return v.namespace === ns && v.name === name; });
        if (vm) isRunning = vm.status === 'Running';
      }
      var bar = document.createElement('div');
      bar.id = 'vmr-inspector-bar';
      bar.className = 'vmr-inspector-bar';
      bar.innerHTML =
        '<div class="vmr-inspector-bar-name">' + esc(name) + '</div>' +
        '<div class="vmr-inspector-bar-meta">' + esc(ns) + ' · ' +
          (isRunning
            ? '<span style="color:var(--green)">Running</span>'
            : '<span style="color:var(--muted)">Stopped</span>') +
        '</div>' +
        '<div class="vmr-inspector-bar-actions">' +
          (isRunning
            ? '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="openConnectModal(' + jsArgs(ns, name) + ')">Open Console</button>' +
              '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigateToVmCapsule(' + jsArgs(ns, name) + ')">Full Capsule</button>' +
              '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs('Explain VM ' + name) + ')">Ask Veyron</button>' +
              '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openCreateModal()">Edit Hardware</button>'
            : '<button type="button" class="glass-btn-primary glass-btn-sm" onclick="vmAction(' + jsArgs(ns, name, 'start') + ')">Start VM</button>' +
              '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="navigateToVmCapsule(' + jsArgs(ns, name) + ')">Full Capsule</button>' +
              '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openCopilotDoctor(' + jsArgs(ns, name) + ')">Diagnose</button>' +
              '<button type="button" class="glass-btn-secondary glass-btn-sm" onclick="openAskZeus(' + jsArgs('Show YAML for VM ' + name) + ')">Show YAML</button>') +
        '</div>';
      var header = document.getElementById('mac-inspector-header');
      var ref = header && header.nextSibling ? header.nextSibling : document.getElementById('vm-focus-empty');
      focusPane.insertBefore(bar, ref || focusPane.firstChild);
      var titleEl = document.getElementById('vm-inspector-title');
      if (titleEl) titleEl.textContent = 'VM INSPECTOR';
    }

    window.selectVm = function (ns, name, opts) {
      origSelect && origSelect.apply(this, arguments);
      if (name) requestAnimationFrame(function () { injectInspectorBar(ns, name); });
    };

    window.clearVmSelection = function () {
      origClear && origClear.apply(this, arguments);
      var bar = document.getElementById('vmr-inspector-bar');
      if (bar) bar.remove();
      var titleEl = document.getElementById('vm-inspector-title');
      if (titleEl) titleEl.textContent = 'Inspector';
    };
  };

  window.initVeyronModule = function initVeyronModule() {
    initVeyronShell();
    patchRenderVmFullList();
    patchFetchAppStore();
    patchNavigateVmr();
    patchRefreshVeyron();
    patchOpenCreateModal();
    patchFetchExperienceHome();
    patchRenderPinnedVms();
    patchFetchTemplatesForForge();
    patchSelectVm();
    renderMissionControlVmr();
    fetchVeyronMissionCosts();
    fetchVeyronAlerts();
  };

  window.initVmrModule = initVeyronModule;

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initVeyronModule);
  } else {
    setTimeout(initVeyronModule, 0);
  }
})();
