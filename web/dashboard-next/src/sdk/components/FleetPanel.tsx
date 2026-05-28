// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  fetchClusters,
  syncCluster,
  type ClusterSummary,
  type ClustersResponse,
} from "../../lib/api";
import CapabilityBanner from "./CapabilityBanner";
import ErrorBanner from "./ErrorBanner";

export function FleetPanel() {
  const [data, setData] = useState<ClustersResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [syncing, setSyncing] = useState<string | null>(null);

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

  const clusters = data?.clusters ?? [];

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Fleet</h2>
          <p style={subtitle}>
            {loading
              ? "Loading kubeconfig contexts…"
              : `Current context: ${data?.current_context || "—"} · ${clusters.length} context(s)`}
          </p>
        </div>
        <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      {error ? <ErrorBanner message={error} /> : null}
      {data?.vmrogue_context ? <CapabilityBanner context={data.vmrogue_context} /> : null}

      <p style={note}>
        Contexts are read from the API pod&apos;s kubeconfig. Switching the active cluster requires
        redeploying the API with a different kubeconfig or in-cluster credentials.
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
              <th style={th}>Primary</th>
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
                <tr key={c.name} style={tr}>
                  <td style={td}>
                    <strong>{c.name}</strong>
                    <div style={sub}>{c.context}</div>
                  </td>
                  <td style={td}>{c.environment}</td>
                  <td style={td}>{c.health}</td>
                  <td style={td}>{c.node_count}</td>
                  <td style={td}>{c.vm_count}</td>
                  <td style={td}>{c.is_primary ? "yes" : "—"}</td>
                  <td style={td}>
                    <button
                      type="button"
                      style={syncBtn}
                      disabled={syncing === c.name}
                      onClick={() => void runSync(c.name)}
                    >
                      {syncing === c.name ? "…" : "Sync"}
                    </button>
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
const note: CSSProperties = { fontSize: 13, color: "#6b7280", lineHeight: 1.5, marginBottom: 16 };
const refreshBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const syncBtn: CSSProperties = {
  padding: "4px 10px", borderRadius: 6, border: "1px solid #f0583a", background: "#fff", color: "#c2410c", fontWeight: 600, fontSize: 12, cursor: "pointer",
};
const tableWrap: CSSProperties = { overflowX: "auto", background: "#fff", borderRadius: 8, border: "1px solid #e5e7eb" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "12px 14px", background: "#f9fafb", borderBottom: "1px solid #e5e7eb", fontWeight: 600,
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151", verticalAlign: "middle" };
const sub: CSSProperties = { fontSize: 11, color: "#9ca3af", marginTop: 2 };
const emptyTd: CSSProperties = { ...td, textAlign: "center", color: "#9ca3af" };

export default FleetPanel;
