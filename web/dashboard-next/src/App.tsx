// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { lazy, Suspense, useCallback, useEffect, useMemo, useState } from 'react';
import { fetchClusters, fetchNamespaces, fetchTenants, type TenantRecord } from './lib/api';
import {
  clearAuthSession,
  getDisplayUser,
  isAuthenticated,
  setAuthSession,
  setOidcSession,
} from './lib/auth';
import { completeOidcCallback, fetchOidcConfig } from './lib/oidc';
import {
  scrollDashboardSection,
  scrollViewSection,
  VMROGUE_NAV_EVENT,
  type VmrogueNavDetail,
  type VmrogueView,
} from './lib/nav';
import { Login } from './sdk/components/Login';
import ErrorBanner from './sdk/components/ErrorBanner';
import { HyperShell } from './sdk/layout/HyperShell';
import { VMROGUE_KUBE_CONTEXT_EVENT } from './sdk/components/FleetPanel';

const Dashboard = lazy(() =>
  import('./sdk/Dashboard').then((m) => ({ default: m.Dashboard }))
);
const VMBrowser = lazy(() => import('./sdk/components/VMBrowser'));
const NodesPanel = lazy(() => import('./sdk/components/NodesPanel'));
const StoragePanel = lazy(() => import('./sdk/components/StoragePanel'));
const BackupsPanel = lazy(() => import('./sdk/components/BackupsPanel'));
const CatalogPanel = lazy(() => import('./sdk/components/CatalogPanel'));
const PlatformPanel = lazy(() => import('./sdk/components/PlatformPanel'));
const WorkloadsPanel = lazy(() => import('./sdk/components/WorkloadsPanel'));
const InsightsPanel = lazy(() => import('./sdk/components/InsightsPanel'));
const CompliancePanel = lazy(() => import('./sdk/components/CompliancePanel'));
const OperationsPanel = lazy(() => import('./sdk/components/OperationsPanel'));
const FleetPanel = lazy(() => import('./sdk/components/FleetPanel'));
const IntegrationsPanel = lazy(() => import('./sdk/components/IntegrationsPanel'));

function ViewFallback() {
  return (
    <div
      style={{
        padding: 24,
        color: '#64748b',
        fontSize: 14,
        textAlign: 'center',
      }}
    >
      Loading view…
    </div>
  );
}

export default function App() {
  const [checking, setChecking] = useState(true);
  const [authed, setAuthed] = useState(false);
  const [view, setView] = useState<VmrogueView>('dashboard');
  const [displayUser, setDisplayUser] = useState('');
  const [inventoryNs, setInventoryNs] = useState('all');
  const [namespaces, setNamespaces] = useState<Array<{ name: string; vmCount: number }>>([]);
  const [namespaceError, setNamespaceError] = useState<string | null>(null);
  const [tenants, setTenants] = useState<TenantRecord[]>([]);
  const [activeTenantId, setActiveTenantId] = useState<string>('');
  const [kubeContext, setKubeContext] = useState('');
  const [clusterCount, setClusterCount] = useState(0);

  useEffect(() => {
    setAuthed(isAuthenticated());
    setDisplayUser(getDisplayUser());
    setChecking(false);
  }, []);

  useEffect(() => {
    if (checking || authed) return;
    void (async () => {
      const config = await fetchOidcConfig();
      if (!config) return;
      const result = await completeOidcCallback(config);
      if (!result) return;
      const res = await fetch('/api/v1/health', {
        headers: { Authorization: `Bearer ${result.accessToken}` },
      });
      if (!res.ok) return;
      setOidcSession(result.accessToken, result.username);
      setAuthed(true);
      setDisplayUser(result.username);
    })();
  }, [checking, authed]);

  const reloadScope = useCallback(() => {
    setNamespaceError(null);
    void Promise.all([
      fetchNamespaces(),
      fetchTenants().catch(() => []),
      fetchClusters().catch(() => null),
    ])
      .then(([rows, tenantRows, clusters]) => {
        setNamespaces(rows);
        setTenants(tenantRows);
        setNamespaceError(null);
        if (clusters) {
          setKubeContext(clusters.current_context);
          setClusterCount(clusters.clusters.length);
        }
      })
      .catch((err) => {
        setNamespaces([]);
        setTenants([]);
        setNamespaceError(err instanceof Error ? err.message : 'Failed to load namespaces');
      });
  }, []);

  useEffect(() => {
    if (!authed) return;
    reloadScope();
  }, [authed, reloadScope]);

  useEffect(() => {
    if (!authed) return;
    const onKubeContext = () => reloadScope();
    window.addEventListener(VMROGUE_KUBE_CONTEXT_EVENT, onKubeContext);
    return () => window.removeEventListener(VMROGUE_KUBE_CONTEXT_EVENT, onKubeContext);
  }, [authed, reloadScope]);

  const activeTenant = useMemo(
    () => tenants.find((t) => t.id === activeTenantId) ?? null,
    [tenants, activeTenantId],
  );

  const scopedNamespaces = useMemo(() => {
    if (!activeTenant?.namespaces?.length) return namespaces;
    const allowed = new Set(activeTenant.namespaces);
    return namespaces.filter((ns) => allowed.has(ns.name));
  }, [namespaces, activeTenant]);

  const handleLogin = useCallback(async (username: string, password: string) => {
    const res = await fetch('/api/v1/health', {
      headers: { 'X-API-Key': password },
    });
    if (!res.ok) {
      throw new Error('Invalid API key or API unavailable');
    }
    setAuthSession(password, username.trim());
    setAuthed(true);
    setDisplayUser(username.trim());
  }, []);

  const handleOidcLogin = useCallback(async () => {
    const config = await fetchOidcConfig();
    if (!config) {
      throw new Error('OIDC is not configured on this cluster');
    }
    const { startOidcLogin } = await import('./lib/oidc');
    await startOidcLogin(config);
  }, []);

  const handleLogout = useCallback(() => {
    clearAuthSession();
    setAuthed(false);
    setDisplayUser('');
    setView('dashboard');
    setInventoryNs('all');
    setActiveTenantId('');
  }, []);

  const navigate = useCallback((next: VmrogueView, scrollTo?: string) => {
    setView(next);
    if (scrollTo) {
      window.requestAnimationFrame(() => {
        window.requestAnimationFrame(() => {
          if (next === 'dashboard') {
            scrollDashboardSection(scrollTo);
          } else {
            scrollViewSection(scrollTo);
          }
        });
      });
    }
  }, []);

  useEffect(() => {
    if (!authed) return;
    const onNav = (event: Event) => {
      const detail = (event as CustomEvent<VmrogueNavDetail>).detail;
      if (!detail?.view) return;
      navigate(detail.view, detail.scrollTo);
    };
    window.addEventListener(VMROGUE_NAV_EVENT, onNav);
    return () => window.removeEventListener(VMROGUE_NAV_EVENT, onNav);
  }, [authed, navigate]);

  if (checking) {
    return (
      <div
        style={{
          minHeight: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: '#020617',
          color: '#94a3b8',
          fontFamily: 'Inter, sans-serif',
        }}
      >
        Loading VMRogue…
      </div>
    );
  }

  if (!authed) {
    return <Login onLogin={handleLogin} onOidcLogin={handleOidcLogin} />;
  }

  return (
    <HyperShell
      activeView={view}
      onViewChange={setView}
      onLogout={handleLogout}
      displayUser={displayUser}
    >
      <div style={{ flex: 1, overflow: 'auto' }}>
        {view === 'inventory' ||
        view === 'storage' ||
        view === 'backups' ||
        view === 'platform' ||
        view === 'workloads' ||
        view === 'insights' ||
        view === 'compliance' ||
        view === 'operations' ? (
          <>
            {namespaceError ? (
              <div style={{ padding: '12px 16px 0', background: '#fff' }}>
                <ErrorBanner message={namespaceError} />
              </div>
            ) : null}
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '12px',
                padding: '12px 16px',
                background: '#fff',
                borderBottom: '1px solid #e5e7eb',
                flexWrap: 'wrap',
              }}
            >
              {tenants.length > 0 ? (
                <>
                  <label htmlFor="tenant-scope" style={{ fontSize: '13px', fontWeight: 600, color: '#374151' }}>
                    Tenant
                  </label>
                  <select
                    id="tenant-scope"
                    value={activeTenantId}
                    onChange={(e) => {
                      setActiveTenantId(e.target.value);
                      setInventoryNs('all');
                    }}
                    style={{
                      padding: '6px 10px',
                      borderRadius: '6px',
                      border: '1px solid #d1d5db',
                      fontSize: '13px',
                      minWidth: '160px',
                    }}
                  >
                    <option value="">All tenants</option>
                    {tenants.map((t) => (
                      <option key={t.id} value={t.id}>
                        {t.display_name || t.id}
                      </option>
                    ))}
                  </select>
                </>
              ) : null}
              {kubeContext ? (
                <span style={{ fontSize: '12px', color: '#6b7280' }}>
                  Kube context: <strong style={{ color: '#374151' }}>{kubeContext}</strong>
                  {clusterCount > 1 ? ` · ${clusterCount} contexts in kubeconfig` : ''}
                </span>
              ) : null}
              <label htmlFor="inventory-ns" style={{ fontSize: '13px', fontWeight: 600, color: '#374151' }}>
                Namespace
              </label>
              <select
                id="inventory-ns"
                value={inventoryNs}
                onChange={(e) => setInventoryNs(e.target.value)}
                style={{
                  padding: '6px 10px',
                  borderRadius: '6px',
                  border: '1px solid #d1d5db',
                  fontSize: '13px',
                  minWidth: '180px',
                }}
              >
                <option value="all">All namespaces</option>
                {scopedNamespaces.map((ns) => (
                  <option key={ns.name} value={ns.name}>
                    {ns.name} ({ns.vmCount} VMs)
                  </option>
                ))}
              </select>
            </div>
          </>
        ) : null}
        {view === 'dashboard' ? (
          <Suspense fallback={<ViewFallback />}>
            <Dashboard />
          </Suspense>
        ) : view === 'nodes' ? (
          <Suspense fallback={<ViewFallback />}>
            <NodesPanel />
          </Suspense>
        ) : view === 'storage' ? (
          <Suspense fallback={<ViewFallback />}>
            <StoragePanel scopeNamespace={inventoryNs} />
          </Suspense>
        ) : view === 'backups' ? (
          <Suspense fallback={<ViewFallback />}>
            <BackupsPanel scopeNamespace={inventoryNs} />
          </Suspense>
        ) : view === 'catalog' ? (
          <Suspense fallback={<ViewFallback />}>
            <CatalogPanel />
          </Suspense>
        ) : view === 'platform' ? (
          <Suspense fallback={<ViewFallback />}>
            <PlatformPanel scopeNamespace={inventoryNs} />
          </Suspense>
        ) : view === 'workloads' ? (
          <Suspense fallback={<ViewFallback />}>
            <WorkloadsPanel scopeNamespace={inventoryNs} />
          </Suspense>
        ) : view === 'insights' ? (
          <Suspense fallback={<ViewFallback />}>
            <InsightsPanel scopeNamespace={inventoryNs} />
          </Suspense>
        ) : view === 'compliance' ? (
          <Suspense fallback={<ViewFallback />}>
            <CompliancePanel scopeNamespace={inventoryNs} />
          </Suspense>
        ) : view === 'operations' ? (
          <Suspense fallback={<ViewFallback />}>
            <OperationsPanel scopeNamespace={inventoryNs} />
          </Suspense>
        ) : view === 'fleet' ? (
          <Suspense fallback={<ViewFallback />}>
            <FleetPanel />
          </Suspense>
        ) : view === 'integrations' ? (
          <Suspense fallback={<ViewFallback />}>
            <IntegrationsPanel />
          </Suspense>
        ) : (
          <Suspense fallback={<ViewFallback />}>
            <VMBrowser
              provider="kubevirt"
              inventoryNamespace={inventoryNs}
              autoDiscoverOnMount
            />
          </Suspense>
        )}
      </div>
    </HyperShell>
  );
}
