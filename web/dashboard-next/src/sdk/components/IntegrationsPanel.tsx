// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  fetchIntegrationsStatus,
  type IntegrationOpenLink,
  type IntegrationStatusItem,
  type IntegrationsStatusResponse,
} from "../../lib/api";
import { requestVmrogueNav, type VmrogueView } from "../../lib/nav";
import CapabilityBanner from "./CapabilityBanner";
import ErrorBanner from "./ErrorBanner";

const IN_APP_VIEWS = new Set<VmrogueView>([
  "platform",
  "catalog",
  "operations",
  "compliance",
  "fleet",
  "insights",
]);

function probeLabel(probe: string): string {
  switch (probe) {
    case "ok":
      return "Reachable";
    case "failed":
      return "Unreachable";
    case "not_configured":
      return "Not configured";
    default:
      return probe;
  }
}

function probeStyle(probe: string): CSSProperties {
  const base: CSSProperties = {
    display: "inline-block",
    padding: "2px 8px",
    borderRadius: 4,
    fontSize: 11,
    fontWeight: 600,
  };
  if (probe === "ok") return { ...base, background: "#dcfce7", color: "#166534" };
  if (probe === "failed") return { ...base, background: "#fee2e2", color: "#991b1b" };
  if (probe === "not_configured") return { ...base, background: "#f3f4f6", color: "#6b7280" };
  return { ...base, background: "#fef3c7", color: "#92400e" };
}

function openIntegration(link: IntegrationOpenLink) {
  if (link.kind === "external") {
    window.open(link.href, "_blank", "noopener,noreferrer");
    return;
  }
  if (link.kind === "in_app_view" && IN_APP_VIEWS.has(link.href as VmrogueView)) {
    requestVmrogueNav({ view: link.href as VmrogueView });
    return;
  }
  requestVmrogueNav({ view: "insights", scrollTo: link.href });
}

export function IntegrationsPanel() {
  const [data, setData] = useState<IntegrationsStatusResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setData(await fetchIntegrationsStatus());
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load integrations");
      setData(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const items = data?.integrations ?? [];

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Integrations</h2>
          <p style={subtitle}>
            {loading
              ? "Checking optional backends…"
              : `${data?.configured_count ?? 0} configured · ${items.length} integration(s)`}
          </p>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <button
            type="button"
            style={secondaryBtn}
            onClick={() => requestVmrogueNav({ view: "insights" })}
          >
            Open Insights
          </button>
          <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
            {loading ? "Refreshing…" : "Refresh"}
          </button>
        </div>
      </div>

      {error ? <ErrorBanner message={error} /> : null}
      {data?.vmrogue_context ? <CapabilityBanner context={data.vmrogue_context} /> : null}

      <p style={note}>
        Wire cluster services via the <code style={code}>vmrogue-integrations</code> Secret (auto-applied
        on deploy when Prometheus/Loki/OpenCost/Trivy/Jaeger/Grafana are detected) or Helm{" "}
        <code style={code}>integrations.*</code> values. External console links use NodePort + node IP when
        reachable from your browser; override with <code style={code}>VMROGUE_*_EXTERNAL_URL</code> env vars.
      </p>

      <div style={tableWrap}>
        <table style={table}>
          <thead>
            <tr>
              <th style={th}>Integration</th>
              <th style={th}>Env var</th>
              <th style={th}>Endpoint</th>
              <th style={th}>Status</th>
              <th style={th}>Open</th>
              <th style={th}>Feeds</th>
            </tr>
          </thead>
          <tbody>
            {items.length === 0 ? (
              <tr>
                <td colSpan={6} style={emptyTd}>
                  {loading ? "Loading…" : "No integration metadata"}
                </td>
              </tr>
            ) : (
              items.map((row: IntegrationStatusItem) => (
                <tr key={row.id} style={tr}>
                  <td style={td}>
                    <strong>{row.name}</strong>
                  </td>
                  <td style={tdMono}>{row.env_var}</td>
                  <td style={td}>{row.endpoint ?? "—"}</td>
                  <td style={td}>
                    <span style={probeStyle(row.probe)}>{probeLabel(row.probe)}</span>
                  </td>
                  <td style={td}>
                    {row.open ? (
                      <button type="button" style={openBtn} onClick={() => openIntegration(row.open!)}>
                        {row.open.label}
                      </button>
                    ) : (
                      "—"
                    )}
                  </td>
                  <td style={tdMuted}>{row.feeds}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1200, margin: "0 auto" };
const header: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "flex-start",
  gap: 16,
  marginBottom: 16,
  flexWrap: "wrap",
};
const title: CSSProperties = { margin: 0, fontSize: 22, color: "#222324" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const note: CSSProperties = { fontSize: 13, color: "#6b7280", lineHeight: 1.5, marginBottom: 16 };
const code: CSSProperties = { fontSize: 12, background: "#f3f4f6", padding: "1px 4px", borderRadius: 3 };
const refreshBtn: CSSProperties = {
  padding: "8px 14px",
  borderRadius: 6,
  border: "1px solid #d1d5db",
  background: "#fff",
  fontWeight: 600,
  fontSize: 13,
  cursor: "pointer",
};
const secondaryBtn: CSSProperties = {
  ...refreshBtn,
  borderColor: "#f0583a",
  color: "#c2410c",
};
const openBtn: CSSProperties = {
  padding: "4px 10px",
  borderRadius: 6,
  border: "1px solid #f0583a",
  background: "#fff",
  color: "#c2410c",
  fontWeight: 600,
  fontSize: 12,
  cursor: "pointer",
};
const tableWrap: CSSProperties = {
  overflowX: "auto",
  background: "#fff",
  borderRadius: 8,
  border: "1px solid #e5e7eb",
};
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left",
  padding: "12px 14px",
  background: "#f9fafb",
  borderBottom: "1px solid #e5e7eb",
  fontWeight: 600,
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151", verticalAlign: "top" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 11 };
const tdMuted: CSSProperties = { ...td, fontSize: 12, color: "#6b7280", maxWidth: 240 };
const emptyTd: CSSProperties = { ...td, textAlign: "center", color: "#9ca3af" };

export default IntegrationsPanel;
