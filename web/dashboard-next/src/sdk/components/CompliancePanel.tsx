// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState, type CSSProperties } from "react";
import {
  fetchComplianceReports,
  fetchComplianceStatus,
  type ComplianceReportRecord,
  type ComplianceStatusRecord,
} from "../../lib/api";
import ErrorBanner from "./ErrorBanner";

type Props = { scopeNamespace?: string };

export function CompliancePanel({ scopeNamespace = "all" }: Props) {
  const [statuses, setStatuses] = useState<ComplianceStatusRecord[]>([]);
  const [reports, setReports] = useState<ComplianceReportRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [st, rep] = await Promise.all([
        fetchComplianceStatus(scopeNamespace),
        fetchComplianceReports(scopeNamespace),
      ]);
      setStatuses(st);
      setReports(rep);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load compliance data");
      setStatuses([]);
      setReports([]);
    } finally {
      setLoading(false);
    }
  }, [scopeNamespace]);

  useEffect(() => {
    void load();
  }, [load]);

  const avgScore = useMemo(() => {
    if (!statuses.length) return 0;
    return Math.round(statuses.reduce((s, x) => s + (x.score || 0), 0) / statuses.length);
  }, [statuses]);

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Compliance</h2>
          <p style={subtitle}>
            {loading
              ? "Loading frameworks…"
              : `${statuses.length} framework(s) · scope: ${scopeNamespace}`}
          </p>
        </div>
        <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      {error ? <ErrorBanner message={error} /> : null}

      <div style={statGrid}>
        <div style={statCard}>
          <div style={statLabel}>Frameworks</div>
          <div style={statValue}>{statuses.length}</div>
        </div>
        <div style={statCard}>
          <div style={statLabel}>Avg score</div>
          <div style={statValue}>{avgScore}%</div>
        </div>
        <div style={statCard}>
          <div style={statLabel}>Reports</div>
          <div style={statValue}>{reports.length}</div>
        </div>
        <div style={statCard}>
          <div style={statLabel}>Scope</div>
          <div style={statValue}>{scopeNamespace}</div>
        </div>
      </div>

      <section style={section}>
        <h3 style={sectionTitle}>Framework status</h3>
        <div style={tableWrap}>
          <table style={table}>
            <thead>
              <tr>
                <th style={th}>Framework</th>
                <th style={th}>Score</th>
                <th style={th}>Passing</th>
                <th style={th}>Failing</th>
                <th style={th}>Compliant</th>
                <th style={th}>Checked</th>
              </tr>
            </thead>
            <tbody>
              {statuses.length === 0 ? (
                <tr>
                  <td colSpan={6} style={emptyTd}>
                    {loading ? "Loading…" : "No frameworks in scope"}
                  </td>
                </tr>
              ) : (
                statuses.map((s) => (
                  <tr key={s.framework} style={tr}>
                    <td style={td}><strong>{s.framework}</strong></td>
                    <td style={td}>{s.score}%</td>
                    <td style={td}>{s.passing_controls}</td>
                    <td style={td}>{s.failing_controls}</td>
                    <td style={td}>{s.compliant ? "yes" : "no"}</td>
                    <td style={tdMono}>{formatTime(s.last_checked)}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Reports</h3>
        <div style={tableWrap}>
          <table style={table}>
            <thead>
              <tr>
                <th style={th}>ID</th>
                <th style={th}>Framework</th>
                <th style={th}>Generated</th>
                <th style={th}>Score</th>
                <th style={th}>Findings</th>
              </tr>
            </thead>
            <tbody>
              {reports.length === 0 ? (
                <tr>
                  <td colSpan={5} style={emptyTd}>
                    {loading ? "Loading…" : "No reports"}
                  </td>
                </tr>
              ) : (
                reports.map((r) => (
                  <tr key={r.id} style={tr}>
                    <td style={tdMono}>{r.id}</td>
                    <td style={td}>{r.framework}</td>
                    <td style={tdMono}>{formatTime(r.generated_at)}</td>
                    <td style={td}>{r.summary?.score ?? "—"}%</td>
                    <td style={td}>{r.findings?.length ?? 0}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}

function formatTime(iso: string): string {
  if (!iso) return "—";
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
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
const section: CSSProperties = { marginBottom: 28 };
const sectionTitle: CSSProperties = { margin: "0 0 12px", fontSize: 16, color: "#222324" };
const statGrid: CSSProperties = {
  display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))", gap: 12, marginBottom: 24,
};
const statCard: CSSProperties = {
  background: "#fff", border: "1px solid #e5e7eb", borderRadius: 8, padding: "12px 14px",
};
const statLabel: CSSProperties = { fontSize: 11, fontWeight: 600, color: "#6b7280", textTransform: "uppercase" };
const statValue: CSSProperties = { marginTop: 6, fontSize: 18, fontWeight: 700, color: "#222324" };
const tableWrap: CSSProperties = { overflowX: "auto", background: "#fff", borderRadius: 8, border: "1px solid #e5e7eb" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "12px 14px", background: "#f9fafb", borderBottom: "1px solid #e5e7eb", fontWeight: 600,
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };
const emptyTd: CSSProperties = { ...td, textAlign: "center", color: "#9ca3af" };

export default CompliancePanel;
