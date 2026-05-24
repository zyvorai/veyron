import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  fetchCostSummary,
  fetchMonitoringStatus,
  fetchRecentEvents,
  fetchSecurityPosture,
  type ClusterEventRecord,
  type CostSummaryRecord,
  type MonitoringStatusRecord,
  type SecurityPostureRecord,
} from "../../lib/api";

type Props = { scopeNamespace?: string };

type SectionErrors = {
  monitoring?: string;
  security?: string;
  costs?: string;
  events?: string;
};

function sectionErrorMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : "Failed to load";
}

export function InsightsPanel({ scopeNamespace = "all" }: Props) {
  const [monitoring, setMonitoring] = useState<MonitoringStatusRecord | null>(null);
  const [security, setSecurity] = useState<SecurityPostureRecord | null>(null);
  const [costs, setCosts] = useState<CostSummaryRecord | null>(null);
  const [events, setEvents] = useState<ClusterEventRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [sectionErrors, setSectionErrors] = useState<SectionErrors>({});

  const load = useCallback(async () => {
    setLoading(true);
    setSectionErrors({});
    const results = await Promise.allSettled([
      fetchMonitoringStatus(scopeNamespace),
      fetchSecurityPosture(scopeNamespace),
      fetchCostSummary(scopeNamespace),
      fetchRecentEvents(scopeNamespace),
    ]);
    const errors: SectionErrors = {};

    if (results[0].status === "fulfilled") {
      setMonitoring(results[0].value);
    } else {
      setMonitoring(null);
      errors.monitoring = sectionErrorMessage(results[0].reason);
    }

    if (results[1].status === "fulfilled") {
      setSecurity(results[1].value);
    } else {
      setSecurity(null);
      errors.security = sectionErrorMessage(results[1].reason);
    }

    if (results[2].status === "fulfilled") {
      setCosts(results[2].value);
    } else {
      setCosts(null);
      errors.costs = sectionErrorMessage(results[2].reason);
    }

    if (results[3].status === "fulfilled") {
      setEvents(results[3].value.slice(0, 25));
    } else {
      setEvents([]);
      errors.events = sectionErrorMessage(results[3].reason);
    }

    setSectionErrors(errors);
    setLoading(false);
  }, [scopeNamespace]);

  useEffect(() => {
    void load();
  }, [load]);

  const failedSections = Object.keys(sectionErrors).length;

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Insights</h2>
          <p style={subtitle}>
            {loading
              ? "Loading monitoring, security, costs, and events…"
              : `Namespace scope: ${scopeNamespace}${failedSections ? ` · ${failedSections} section(s) unavailable` : ""}`}
          </p>
        </div>
        <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      <section style={section}>
        <h3 style={sectionTitle}>Monitoring</h3>
        {sectionErrors.monitoring ? (
          <p style={sectionError}>{sectionErrors.monitoring}</p>
        ) : monitoring ? (
          <div style={statGrid}>
            <Stat label="Prometheus" value={monitoring.prometheus_available ? "Available" : "Not detected"} ok={monitoring.prometheus_available} />
            <Stat label="Grafana" value={monitoring.grafana_available ? "Available" : "Not detected"} ok={monitoring.grafana_available} />
            <Stat label="Alertmanager" value={monitoring.alertmanager_available ? "Available" : "Not detected"} ok={monitoring.alertmanager_available} />
            <Stat label="Active alerts" value={String(monitoring.active_alerts)} ok={monitoring.active_alerts === 0} />
            <Stat
              label="Healthy targets"
              value={`${monitoring.healthy_targets}/${monitoring.total_targets}`}
              ok={monitoring.total_targets === 0 || monitoring.healthy_targets > 0}
            />
          </div>
        ) : (
          <p style={muted}>{loading ? "…" : "No monitoring data"}</p>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Security posture</h3>
        {sectionErrors.security ? (
          <p style={sectionError}>{sectionErrors.security}</p>
        ) : security ? (
          <p style={bodyText}>
            Score <strong>{security.overall_score}</strong> · risk <strong>{security.risk_level}</strong> ·
            findings: {security.critical_findings} critical, {security.high_findings} high, {security.medium_findings} medium
          </p>
        ) : (
          <p style={muted}>{loading ? "…" : "No security posture data"}</p>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Cost estimate</h3>
        {sectionErrors.costs ? (
          <p style={sectionError}>{sectionErrors.costs}</p>
        ) : costs ? (
          <>
            <p style={bodyText}>
              <strong>{costs.total_cost.toFixed(2)} {costs.currency}</strong> / {costs.period}
              {costs.pricing_model ? ` · ${costs.pricing_model}` : ""}
            </p>
            {costs.disclaimer ? <p style={note}>{costs.disclaimer}</p> : null}
          </>
        ) : (
          <p style={muted}>{loading ? "…" : "No cost summary"}</p>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Recent events</h3>
        {sectionErrors.events ? (
          <p style={sectionError}>{sectionErrors.events}</p>
        ) : events.length === 0 && !loading ? (
          <p style={muted}>No recent events.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Time</th>
                  <th style={th}>Type</th>
                  <th style={th}>Reason</th>
                  <th style={th}>Object</th>
                  <th style={th}>Message</th>
                </tr>
              </thead>
              <tbody>
                {events.map((ev) => (
                  <tr key={ev.id || `${ev.timestamp}-${ev.reason}`} style={tr}>
                    <td style={tdMono}>{ev.timestamp ? ev.timestamp.slice(0, 19) : "—"}</td>
                    <td style={td}>{ev.event_type}</td>
                    <td style={td}>{ev.reason}</td>
                    <td style={td}>{ev.involved_object}</td>
                    <td style={td}>{ev.message}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}

function Stat({ label, value, ok }: { label: string; value: string; ok?: boolean }) {
  return (
    <div style={statCard}>
      <div style={statLabel}>{label}</div>
      <div style={{ ...statValue, color: ok ? "#15803d" : "#374151" }}>{value}</div>
    </div>
  );
}

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1400, margin: "0 auto" };
const header: CSSProperties = { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, marginBottom: 20, flexWrap: "wrap" };
const title: CSSProperties = { margin: 0, fontSize: 22, color: "#222324" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const refreshBtn: CSSProperties = { padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer" };
const section: CSSProperties = { marginBottom: 28 };
const sectionTitle: CSSProperties = { margin: "0 0 12px", fontSize: 16, color: "#222324" };
const sectionError: CSSProperties = { margin: 0, fontSize: 13, color: "#b91c1c" };
const muted: CSSProperties = { color: "#6b7280", fontSize: 14 };
const bodyText: CSSProperties = { margin: 0, fontSize: 14, color: "#374151" };
const note: CSSProperties = { margin: "8px 0 0", fontSize: 12, color: "#6b7280", lineHeight: 1.45 };
const statGrid: CSSProperties = { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 12 };
const statCard: CSSProperties = { background: "#fff", border: "1px solid #e5e7eb", borderRadius: 8, padding: "12px 14px" };
const statLabel: CSSProperties = { fontSize: 11, fontWeight: 600, color: "#6b7280", textTransform: "uppercase", letterSpacing: "0.04em" };
const statValue: CSSProperties = { marginTop: 6, fontSize: 14, fontWeight: 600 };
const tableWrap: CSSProperties = { overflowX: "auto", background: "#fff", borderRadius: 8, border: "1px solid #e5e7eb" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = { textAlign: "left", padding: "12px 14px", background: "#f9fafb", borderBottom: "1px solid #e5e7eb", fontWeight: 600, color: "#374151", whiteSpace: "nowrap" };
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151", verticalAlign: "top" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };

export default InsightsPanel;
