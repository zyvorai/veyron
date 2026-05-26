// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import { fetchClusterNodes, type ClusterNodeRecord } from "../../lib/api";

export function NodesPanel() {
  const [nodes, setNodes] = useState<ClusterNodeRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rows = await fetchClusterNodes();
      setNodes(rows);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load nodes");
      setNodes([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const readyCount = nodes.filter((n) => n.status === "Ready").length;

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Cluster nodes</h2>
          <p style={subtitle}>
            {loading
              ? "Loading node inventory…"
              : `${readyCount} ready · ${nodes.length} total · read-only view`}
          </p>
        </div>
        <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      {error ? (
        <div style={errorBox} role="alert">
          {error}
        </div>
      ) : null}

      {loading && nodes.length === 0 ? (
        <p style={muted}>Loading nodes from the Kubernetes API…</p>
      ) : nodes.length === 0 ? (
        <p style={muted}>No nodes returned. Check API RBAC and cluster connectivity.</p>
      ) : (
        <div style={tableWrap}>
          <table style={table}>
            <thead>
              <tr>
                <th style={th}>Name</th>
                <th style={th}>Status</th>
                <th style={th}>Roles</th>
                <th style={th}>CPU</th>
                <th style={th}>Memory</th>
                <th style={th}>Kubelet</th>
                <th style={th}>OS</th>
                <th style={th}>Age</th>
              </tr>
            </thead>
            <tbody>
              {nodes.map((node) => (
                <tr key={node.name} style={tr}>
                  <td style={td}>
                    <strong>{node.name}</strong>
                  </td>
                  <td style={td}>
                    <span style={{ ...badge, ...statusStyle(node.status) }}>{node.status}</span>
                  </td>
                  <td style={td}>{node.roles.length ? node.roles.join(", ") : "—"}</td>
                  <td style={td}>
                    {node.cpu_allocatable || node.cpu_capacity
                      ? `${node.cpu_allocatable || "?"} / ${node.cpu_capacity || "?"}`
                      : "—"}
                  </td>
                  <td style={td}>
                    {node.memory_allocatable || node.memory_capacity
                      ? `${node.memory_allocatable || "?"} / ${node.memory_capacity || "?"}`
                      : "—"}
                  </td>
                  <td style={tdMono}>{node.kubelet_version || "—"}</td>
                  <td style={td}>{node.os_image || "—"}</td>
                  <td style={td}>{node.age || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <p style={footerNote}>
        Storage classes, GitOps, and operator CRDs remain in the{" "}
        <a href="/dashboard" style={link}>
          full dashboard
        </a>
        .
      </p>
    </div>
  );
}

function statusStyle(status: string): CSSProperties {
  if (status === "Ready") {
    return { backgroundColor: "#dcfce7", color: "#15803d" };
  }
  if (status === "NotReady") {
    return { backgroundColor: "#fee2e2", color: "#b91c1c" };
  }
  return { backgroundColor: "#e0e7ff", color: "#3730a3" };
}

const wrap: CSSProperties = {
  padding: "20px 24px 32px",
  maxWidth: 1400,
  margin: "0 auto",
};

const header: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "flex-start",
  gap: 16,
  marginBottom: 20,
  flexWrap: "wrap",
};

const title: CSSProperties = {
  margin: 0,
  fontSize: 22,
  color: "#222324",
};

const subtitle: CSSProperties = {
  margin: "6px 0 0",
  fontSize: 13,
  color: "#6b7280",
};

const refreshBtn: CSSProperties = {
  padding: "8px 14px",
  borderRadius: 6,
  border: "1px solid #d1d5db",
  background: "#fff",
  fontWeight: 600,
  fontSize: 13,
  cursor: "pointer",
};

const errorBox: CSSProperties = {
  padding: "12px 14px",
  marginBottom: 16,
  borderRadius: 8,
  background: "#fef2f2",
  border: "1px solid #fecaca",
  color: "#b91c1c",
  fontSize: 13,
};

const muted: CSSProperties = {
  color: "#6b7280",
  fontSize: 14,
};

const tableWrap: CSSProperties = {
  overflowX: "auto",
  background: "#fff",
  borderRadius: 8,
  border: "1px solid #e5e7eb",
};

const table: CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: 13,
};

const th: CSSProperties = {
  textAlign: "left",
  padding: "12px 14px",
  background: "#f9fafb",
  borderBottom: "1px solid #e5e7eb",
  fontWeight: 600,
  color: "#374151",
  whiteSpace: "nowrap",
};

const tr: CSSProperties = {
  borderBottom: "1px solid #f3f4f6",
};

const td: CSSProperties = {
  padding: "12px 14px",
  color: "#374151",
  verticalAlign: "top",
};

const tdMono: CSSProperties = {
  ...td,
  fontFamily: "ui-monospace, monospace",
  fontSize: 12,
};

const badge: CSSProperties = {
  display: "inline-block",
  padding: "3px 8px",
  borderRadius: 999,
  fontSize: 12,
  fontWeight: 600,
};

const footerNote: CSSProperties = {
  marginTop: 20,
  fontSize: 12,
  color: "#9ca3af",
};

const link: CSSProperties = {
  color: "#f0583a",
  fontWeight: 600,
};

export default NodesPanel;
