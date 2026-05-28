// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  activateCluster,
  fetchClusters,
  syncCluster,
  type ClusterSummary,
  type ClustersResponse,
} from "../../lib/api";
import CapabilityBanner from "./CapabilityBanner";
import ErrorBanner from "./ErrorBanner";

export const VMROGUE_KUBE_CONTEXT_EVENT = "vmrogue:kube-context-changed";

export function FleetPanel() {
  const [data, setData] = useState<ClustersResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [syncing, setSyncing] = useState<string | null>(null);
  const [activating, setActivating] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setData(await fetchClusters());
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load clusters");
      setData(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const runSync = async (name: string) => {
    setSyncing(name);
    setError(null);
    setMessage(null);
    try {
      const updated = await syncCluster(name);
      setData((prev) => {
        if (!prev) return prev;
        const clusters = prev.clusters.map((c) => (c.name === name ? updated : c));
        return { ...prev, clusters };
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Sync failed");
    } finally {
      setSyncing(null);
    }
  };

  const runActivate = async (name: string) => {
    setActivating(name);
    setError(null);
    setMessage(null);
    try {
      const res = await activateCluster(name);
      setMessage(res.message);
      await load();
      window.dispatchEvent(new CustomEvent(VMROGUE_KUBE_CONTEXT_EVENT, { detail: res }));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Activate failed");
    } finally {
      setActivating(null);
    }
  };

  const clusters = data?.clusters ?? [];
  const multiContext = clusters.length > 1;

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Fleet</h2>
          <p style={subtitle}>
            {loading
              ? "Loading kubeconfig contexts…"
              : `Active API context: ${data?.current_context || "—"} · ${clusters.length} context(s)`}
          </p>
          {data?.kubeconfig_current_context &&
          data.kubeconfig_current_context !== data.current_context ? (
            <p style={fileCtx}>
              Kubeconfig file default: <strong>{data.kubeconfig_current_context}</strong>
            </p>
          ) : null}
        </div>
        <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      {error ? <ErrorBanner message={error} /> : null}
      {message && !error ? <p style={successNote}>{message}</p> : null}
      {data?.vmrogue_context ? <CapabilityBanner context={data.vmrogue_context} /> : null}

      <p style={note}>
        {multiContext
          ? "Use Activate to point the VMRogue API at another kubeconfig context (persisted in a ConfigMap). Inventory and VM operations then target that cluster until you switch again."
          : "Only one kubeconfig context is visible. Mount a multi-context kubeconfig on the API pod to enable fleet switching."}
      </p>

      <div style={tableWrap}>
        <table style={table}>
          <thead>
            <tr>
              <th style={th}>Context</th>
              <th style={th}>Environment</th>
              <th style={th}>Health</th>
              <th style={th}>Nodes</th>
              <th style={th}>VMs</th>
              <th style={th}>Active</th>
              <th style={th} />
            </tr>
          </thead>
          <tbody>
            {clusters.length === 0 ? (
              <tr>
                <td colSpan={7} style={emptyTd}>
                  {loading ? "Loading…" : "No contexts in kubeconfig"}
                </td>
              </tr>
            ) : (
              clusters.map((c: ClusterSummary) => (
                <tr key={c.name} style={c.is_active ? activeRow : tr}>
                  <td style={td}>
                    <strong>{c.name}</strong>
                    <div style={sub}>{c.context}</div>
                  </td>
                  <td style={td}>{c.environment}</td>
                  <td style={td}>{c.health}</td>
                  <td style={td}>{c.node_count}</td>
                  <td style={td}>{c.vm_count}</td>
                  <td style={td}>{c.is_active ? "yes" : "—"}</td>
                  <td style={td}>
                    <div style={btnRow}>
                      <button
                        type="button"
                        style={activateBtn}
                        disabled={activating === c.name || c.is_active || !multiContext}
                        onClick={() => void runActivate(c.name)}
                      >
                        {activating === c.name ? "…" : c.is_active ? "Active" : "Activate"}
                      </button>
                      <button
                        type="button"
                        style={syncBtn}
                        disabled={syncing === c.name}
                        onClick={() => void runSync(c.name)}
                      >
                        {syncing === c.name ? "…" : "Sync"}
                      </button>
                    </div>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1400, margin: "0 auto" };
const header: CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, marginBottom: 16, flexWrap: "wrap",
};
const title: CSSProperties = { margin: 0, fontSize: 22, color: "#222324" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const fileCtx: CSSProperties = { margin: "4px 0 0", fontSize: 12, color: "#9ca3af" };
const note: CSSProperties = { fontSize: 13, color: "#6b7280", lineHeight: 1.5, marginBottom: 16 };
const successNote: CSSProperties = { margin: "0 0 12px", fontSize: 13, color: "#15803d", fontWeight: 600 };
const refreshBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const activateBtn: CSSProperties = {
  padding: "4px 10px", borderRadius: 6, border: "1px solid #f0583a", background: "#f0583a", color: "#fff", fontWeight: 600, fontSize: 12, cursor: "pointer",
};
const syncBtn: CSSProperties = {
  padding: "4px 10px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", color: "#374151", fontWeight: 600, fontSize: 12, cursor: "pointer",
};
const btnRow: CSSProperties = { display: "flex", gap: 6, flexWrap: "wrap" };
const tableWrap: CSSProperties = { overflowX: "auto", background: "#fff", borderRadius: 8, border: "1px solid #e5e7eb" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "12px 14px", background: "#f9fafb", borderBottom: "1px solid #e5e7eb", fontWeight: 600,
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const activeRow: CSSProperties = { ...tr, background: "#fff7ed" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151", verticalAlign: "middle" };
const sub: CSSProperties = { fontSize: 11, color: "#9ca3af", marginTop: 2 };
const emptyTd: CSSProperties = { ...td, textAlign: "center", color: "#9ca3af" };

export default FleetPanel;
