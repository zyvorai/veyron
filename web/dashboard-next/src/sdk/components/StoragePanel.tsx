// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  fetchPvcs,
  fetchStorageClasses,
  type PvcRecord,
  type StorageClassRecord,
} from "../../lib/api";

type Props = {
  scopeNamespace?: string;
};

export function StoragePanel({ scopeNamespace = "all" }: Props) {
  const [classes, setClasses] = useState<StorageClassRecord[]>([]);
  const [pvcs, setPvcs] = useState<PvcRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [scRows, pvcRows] = await Promise.all([
        fetchStorageClasses(),
        fetchPvcs(scopeNamespace),
      ]);
      setClasses(scRows);
      setPvcs(pvcRows);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load storage");
      setClasses([]);
      setPvcs([]);
    } finally {
      setLoading(false);
    }
  }, [scopeNamespace]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Storage</h2>
          <p style={subtitle}>
            {loading
              ? "Loading PVCs and storage classes…"
              : `${classes.length} storage class(es) · ${pvcs.length} PVC(s) · namespace: ${scopeNamespace}`}
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

      <section style={section}>
        <h3 style={sectionTitle}>Storage classes</h3>
        {classes.length === 0 && !loading ? (
          <p style={muted}>No storage classes returned.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>Provisioner</th>
                  <th style={th}>Reclaim</th>
                  <th style={th}>Binding</th>
                  <th style={th}>Default</th>
                </tr>
              </thead>
              <tbody>
                {classes.map((sc) => (
                  <tr key={sc.name} style={tr}>
                    <td style={td}><strong>{sc.name}</strong></td>
                    <td style={tdMono}>{sc.provisioner || "—"}</td>
                    <td style={td}>{sc.reclaim_policy || "—"}</td>
                    <td style={td}>{sc.volume_binding_mode || "—"}</td>
                    <td style={td}>{sc.is_default ? "Yes" : "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>PersistentVolumeClaims</h3>
        {pvcs.length === 0 && !loading ? (
          <p style={muted}>No PVCs in this scope.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Namespace</th>
                  <th style={th}>Name</th>
                  <th style={th}>Status</th>
                  <th style={th}>Capacity</th>
                  <th style={th}>Class</th>
                  <th style={th}>Access modes</th>
                </tr>
              </thead>
              <tbody>
                {pvcs.map((pvc) => (
                  <tr key={`${pvc.namespace}/${pvc.name}`} style={tr}>
                    <td style={td}>{pvc.namespace}</td>
                    <td style={td}><strong>{pvc.name}</strong></td>
                    <td style={td}>
                      <span style={{ ...badge, ...pvcStatusStyle(pvc.status) }}>{pvc.status || "—"}</span>
                    </td>
                    <td style={td}>{pvc.capacity || "—"}</td>
                    <td style={td}>{pvc.storage_class || "—"}</td>
                    <td style={td}>{pvc.access_modes?.length ? pvc.access_modes.join(", ") : "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <p style={footerNote}>
        GitOps, snapshot scheduling, and CSI drill-downs remain in the{" "}
        <a href="/dashboard" style={link}>full dashboard</a>.
      </p>
    </div>
  );
}

function pvcStatusStyle(status: string): CSSProperties {
  const s = status.toLowerCase();
  if (s === "bound") return { backgroundColor: "#dcfce7", color: "#15803d" };
  if (s === "pending") return { backgroundColor: "#fef9c3", color: "#a16207" };
  return { backgroundColor: "#f3f4f6", color: "#374151" };
}

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1400, margin: "0 auto" };
const header: CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, marginBottom: 20, flexWrap: "wrap",
};
const title: CSSProperties = { margin: 0, fontSize: 22, color: "#222324" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const refreshBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const errorBox: CSSProperties = {
  padding: "12px 14px", marginBottom: 16, borderRadius: 8, background: "#fef2f2", border: "1px solid #fecaca", color: "#b91c1c", fontSize: 13,
};
const section: CSSProperties = { marginBottom: 28 };
const sectionTitle: CSSProperties = { margin: "0 0 12px", fontSize: 16, color: "#222324" };
const muted: CSSProperties = { color: "#6b7280", fontSize: 14 };
const tableWrap: CSSProperties = { overflowX: "auto", background: "#fff", borderRadius: 8, border: "1px solid #e5e7eb" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "12px 14px", background: "#f9fafb", borderBottom: "1px solid #e5e7eb", fontWeight: 600, color: "#374151", whiteSpace: "nowrap",
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151", verticalAlign: "top" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };
const badge: CSSProperties = { display: "inline-block", padding: "3px 8px", borderRadius: 999, fontSize: 12, fontWeight: 600 };
const footerNote: CSSProperties = { marginTop: 8, fontSize: 12, color: "#9ca3af" };
const link: CSSProperties = { color: "#f0583a", fontWeight: 600 };

export default StoragePanel;
