import { lazy, Suspense, useCallback, useEffect, useState } from 'react';
import { fetchNamespaces } from './lib/api';
import {
  clearAuthSession,
  getDisplayUser,
  isAuthenticated,
  setAuthSession,
} from './lib/auth';
import {
  scrollDashboardSection,
  VMROGUE_NAV_EVENT,
  type VmrogueNavDetail,
  type VmrogueView,
} from './lib/nav';
import { Login } from './sdk/components/Login';
import { HyperShell, type HyperShellView } from './sdk/layout/HyperShell';

const Dashboard = lazy(() =>
  import('./sdk/Dashboard').then((m) => ({ default: m.Dashboard }))
);
const VMBrowser = lazy(() => import('./sdk/components/VMBrowser'));
const NodesPanel = lazy(() => import('./sdk/components/NodesPanel'));
const StoragePanel = lazy(() => import('./sdk/components/StoragePanel'));
const PlatformPanel = lazy(() => import('./sdk/components/PlatformPanel'));

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
  const [view, setView] = useState<HyperShellView>('dashboard');
  const [displayUser, setDisplayUser] = useState('');
  const [inventoryNs, setInventoryNs] = useState('all');
  const [namespaces, setNamespaces] = useState<Array<{ name: string; vmCount: number }>>([]);

  useEffect(() => {
    setAuthed(isAuthenticated());
    setDisplayUser(getDisplayUser());
    setChecking(false);
  }, []);

  useEffect(() => {
    if (!authed) return;
    void fetchNamespaces()
      .then((rows) => setNamespaces(rows))
      .catch(() => setNamespaces([]));
  }, [authed]);

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

  const handleLogout = useCallback(() => {
    clearAuthSession();
    setAuthed(false);
    setDisplayUser('');
    setView('dashboard');
    setInventoryNs('all');
  }, []);

  const navigate = useCallback((next: VmrogueView, scrollTo?: string) => {
    setView(next);
    if (next === 'dashboard' && scrollTo) {
      window.requestAnimationFrame(() => scrollDashboardSection(scrollTo));
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
    return <Login onLogin={handleLogin} />;
  }

  return (
    <HyperShell
      activeView={view}
      onViewChange={setView}
      onLogout={handleLogout}
      displayUser={displayUser}
    >
      <div style={{ flex: 1, overflow: 'auto' }}>
        {view === 'inventory' || view === 'storage' || view === 'platform' ? (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '12px',
              padding: '12px 16px',
              background: '#fff',
              borderBottom: '1px solid #e5e7eb',
            }}
          >
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
              {namespaces.map((ns) => (
                <option key={ns.name} value={ns.name}>
                  {ns.name} ({ns.vmCount} VMs)
                </option>
              ))}
            </select>
          </div>
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
        ) : view === 'platform' ? (
          <Suspense fallback={<ViewFallback />}>
            <PlatformPanel scopeNamespace={inventoryNs} />
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
