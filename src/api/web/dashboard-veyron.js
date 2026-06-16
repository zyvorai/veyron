/* Veyron Command Center module */
(function () {
  'use strict';

  var VEYRON_NAV = [
    { page: 'dashboard', label: 'Mission Control', icon: '◫' },
    { page: 'vms', label: 'Fleet Command', icon: '▣' },
    { page: 'app-store', label: 'Template Foundry', icon: '⬡' },
    { page: 'console-hub', label: 'ConsoleHub', icon: '▶' },
    { page: 'snapshots', label: 'Snapshots & Backups', icon: '⧉' },
    { page: 'cilium', label: 'Network Intelligence', icon: '⟷' },
    { page: 'security', label: 'Security Posture', icon: '⛨' },
    { page: 'stack-health', label: 'Stack Health', icon: '♥' },
    { page: 'events', label: 'Event Intelligence', icon: '⚡' },
    { page: 'costs', label: 'Cost Explorer', icon: '$' },
    { page: 'workloads', label: 'Workloads', icon: '◧' },
    { page: 'nodes', label: 'Cluster Nodes', icon: '⬢' }
  ];

  var VMR_PAGE_TITLES = {
    dashboard: 'Veyron Mission Control',
    vms: 'Veyron Fleet Command',
    'app-store': 'Veyron Template Foundry',
    'console-hub': 'Veyron ConsoleHub',
    snapshots: 'Veyron Snapshots & Backups',
    security: 'Veyron Security Posture',
    'stack-health': 'Veyron Stack Health',
    events: 'Veyron Event Intelligence',
    cilium: 'Veyron Network Intelligence',
    costs: 'Veyron Cost Explorer',
    workloads: 'Veyron Workloads',
    nodes: 'Veyron Cluster Nodes',
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
        '<button type="button" class="veyron-nav-item" onclick="openMacPreferences()">' +
        '<span class="veyron-nav-icon">⚙</span>Settings</button>' +
        '<button type="button" class="veyron-nav-item" onclick="toggleClassicNav()">' +
        '<span class="veyron-nav-icon">⋯</span>Advanced pages</button></div>';
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
        { label: 'Network Intelligence', fn: "navigate('cilium')" },
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
          '<button type="button" class="glass-btn-secondary glass-btn-sm" ' + onStopHandler('selectVm(' + jsArgs(ns, vm.name) + ')') + '>Details</button>' +
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
