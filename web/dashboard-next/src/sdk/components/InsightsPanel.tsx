// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  createCostBudget,
  fetchCostBudgets,
  fetchCostSummary,
  fetchIncidents,
  fetchLogs,
  fetchMonitoringStatus,
  fetchRecentEvents,
  fetchSecurityFindings,
  fetchSecurityPosture,
  fetchTraces,
  fetchMetricsTimeline,
  fetchVmInventory,
  type ClusterEventRecord,
  type CostBudgetRecord,
  type CostSummaryRecord,
  type IncidentsTimelineRecord,
  type LogsDashboardRecord,
  type MonitoringStatusRecord,
  type SecurityFindingRecord,
  type SecurityPostureRecord,
  type TracesResponse,
  type MetricsTimelineResponse,
} from "../../lib/api";
import CapabilityBanner from "./CapabilityBanner";
import ErrorBanner from "./ErrorBanner";

type Props = { scopeNamespace?: string };

type SectionErrors = {
  monitoring?: string;
  security?: string;
  findings?: string;
  costs?: string;
  events?: string;
  incidents?: string;
  logs?: string;
  budgets?: string;
  traces?: string;
  timeline?: string;
};

function sectionErrorMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : "Failed to load";
}

export function InsightsPanel({ scopeNamespace = "all" }: Props) {
  const [monitoring, setMonitoring] = useState<MonitoringStatusRecord | null>(null);
  const [security, setSecurity] = useState<SecurityPostureRecord | null>(null);
  const [findings, setFindings] = useState<SecurityFindingRecord[]>([]);
  const [costs, setCosts] = useState<CostSummaryRecord | null>(null);
  const [events, setEvents] = useState<ClusterEventRecord[]>([]);
  const [incidents, setIncidents] = useState<IncidentsTimelineRecord | null>(null);
  const [logs, setLogs] = useState<LogsDashboardRecord | null>(null);
  const [budgets, setBudgets] = useState<CostBudgetRecord[]>([]);
  const [traces, setTraces] = useState<TracesResponse | null>(null);
  const [timeline, setTimeline] = useState<MetricsTimelineResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [sectionErrors, setSectionErrors] = useState<SectionErrors>({});
  const [budgetBusy, setBudgetBusy] = useState(false);
  const [budgetForm, setBudgetForm] = useState({
    name: "",
    namespace: scopeNamespace === "all" ? "default" : scopeNamespace,
    monthly_limit: 500,
    alert_threshold_percent: 80,
  });

  const load = useCallback(async () => {
    setLoading(true);
    setSectionErrors({});
    const results = await Promise.allSettled([
      fetchMonitoringStatus(scopeNamespace),
      fetchSecurityPosture(scopeNamespace),
      fetchSecurityFindings(scopeNamespace),
      fetchCostSummary(scopeNamespace),
      fetchRecentEvents(scopeNamespace),
      fetchIncidents(scopeNamespace),
      fetchLogs(scopeNamespace, 40),
      fetchCostBudgets(),
      fetchTraces(scopeNamespace),
      fetchVmInventory(scopeNamespace).then(async (vms) => {
        const running = vms.find((v) => v.status === "Running");
        if (!running) return null;
        return fetchMetricsTimeline({
          namespace: running.namespace,
          vm: running.name,
          metric: "cpu",
          hours: 6,
        });
      }),
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
      setFindings(results[2].value.slice(0, 10));
    } else {
      setFindings([]);
      errors.findings = sectionErrorMessage(results[2].reason);
    }

    if (results[3].status === "fulfilled") {
      setCosts(results[3].value);
    } else {
      setCosts(null);
      errors.costs = sectionErrorMessage(results[3].reason);
    }

    if (results[4].status === "fulfilled") {
      setEvents(results[4].value.slice(0, 25));
    } else {
      setEvents([]);
      errors.events = sectionErrorMessage(results[4].reason);
    }

    if (results[5].status === "fulfilled") {
      setIncidents(results[5].value);
    } else {
      setIncidents(null);
      errors.incidents = sectionErrorMessage(results[5].reason);
    }

    if (results[6].status === "fulfilled") {
      setLogs(results[6].value);
    } else {
      setLogs(null);
      errors.logs = sectionErrorMessage(results[6].reason);
    }

    if (results[7].status === "fulfilled") {
      setBudgets(results[7].value);
    } else {
      setBudgets([]);
      errors.budgets = sectionErrorMessage(results[7].reason);
    }

    if (results[8].status === "fulfilled") {
      setTraces(results[8].value);
    } else {
      setTraces(null);
      errors.traces = sectionErrorMessage(results[8].reason);
    }

    if (results[9].status === "fulfilled") {
      setTimeline(results[9].value);
    } else {
      setTimeline(null);
      errors.timeline = sectionErrorMessage(results[9].reason);
    }

    setSectionErrors(errors);
    setLoading(false);
  }, [scopeNamespace]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setBudgetForm((prev) => ({
      ...prev,
      namespace: scopeNamespace === "all" ? prev.namespace : scopeNamespace,
    }));
  }, [scopeNamespace]);

  const createBudget = async () => {
    if (!budgetForm.name.trim()) {
      setSectionErrors((prev) => ({ ...prev, budgets: "Budget name is required" }));
      return;
    }
    setBudgetBusy(true);
    setSectionErrors((prev) => ({ ...prev, budgets: undefined }));
    try {
      await createCostBudget({
        name: budgetForm.name.trim(),
        namespace: budgetForm.namespace.trim() || "default",
        monthly_limit: budgetForm.monthly_limit,
        alert_threshold_percent: budgetForm.alert_threshold_percent,
      });
      setBudgetForm((prev) => ({ ...prev, name: "" }));
      const rows = await fetchCostBudgets();
      setBudgets(rows);
    } catch (e) {
      setSectionErrors((prev) => ({
        ...prev,
        budgets: e instanceof Error ? e.message : "Create budget failed",
      }));
    } finally {
      setBudgetBusy(false);
    }
  };

  const failedSections = Object.keys(sectionErrors).length;

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Insights</h2>
          <p style={subtitle}>
            {loading
              ? "Loading monitoring, security, costs, incidents, logs, and events…"
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
          <>
            <p style={bodyText}>
              Score <strong>{security.overall_score}</strong> · risk <strong>{security.risk_level}</strong> ·
              findings: {security.critical_findings} critical, {security.high_findings} high, {security.medium_findings} medium
            </p>
            {sectionErrors.findings ? (
              <p style={sectionError}>{sectionErrors.findings}</p>
            ) : findings.length > 0 ? (
              <div style={{ ...tableWrap, marginTop: 12 }}>
                <table style={table}>
                  <thead>
                    <tr>
                      <th style={th}>Severity</th>
                      <th style={th}>Title</th>
                      <th style={th}>Resource</th>
                      <th style={th}>Recommendation</th>
                    </tr>
                  </thead>
                  <tbody>
                    {findings.map((f) => (
                      <tr key={f.id} style={tr}>
                        <td style={td}>{f.severity}</td>
                        <td style={td}><strong>{f.title}</strong></td>
                        <td style={tdMono}>{f.resource}</td>
                        <td style={td}>{f.recommendation}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : !loading ? (
              <p style={note}>No detailed findings for this scope.</p>
            ) : null}
          </>
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
            <CapabilityBanner context={costs.vmrogue_context} />
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
        <h3 style={sectionTitle}>Cost budgets</h3>
        {sectionErrors.budgets ? <ErrorBanner message={sectionErrors.budgets} /> : null}
        <div style={budgetFormRow}>
          <input
            placeholder="Budget name"
            value={budgetForm.name}
            onChange={(e) => setBudgetForm((p) => ({ ...p, name: e.target.value }))}
            style={input}
          />
          <input
            placeholder="Namespace"
            value={budgetForm.namespace}
            onChange={(e) => setBudgetForm((p) => ({ ...p, namespace: e.target.value }))}
            style={input}
          />
          <input
            type="number"
            min={1}
            placeholder="Monthly limit"
            value={budgetForm.monthly_limit}
            onChange={(e) => setBudgetForm((p) => ({ ...p, monthly_limit: parseFloat(e.target.value) || 0 }))}
            style={input}
          />
          <input
            type="number"
            min={1}
            max={100}
            placeholder="Alert %"
            value={budgetForm.alert_threshold_percent}
            onChange={(e) =>
              setBudgetForm((p) => ({ ...p, alert_threshold_percent: parseFloat(e.target.value) || 80 }))
            }
            style={input}
          />
          <button type="button" style={refreshBtn} disabled={budgetBusy} onClick={() => void createBudget()}>
            {budgetBusy ? "Saving…" : "Add budget"}
          </button>
        </div>
        {budgets.length === 0 && !loading ? (
          <p style={muted}>No budgets configured.</p>
        ) : (
          <div style={{ ...tableWrap, marginTop: 12 }}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>Namespace</th>
                  <th style={th}>Spend / limit</th>
                  <th style={th}>Threshold</th>
                  <th style={th}>Status</th>
                </tr>
              </thead>
              <tbody>
                {budgets.map((b) => (
                  <tr key={`${b.namespace}/${b.name}`} style={tr}>
                    <td style={td}><strong>{b.name}</strong></td>
                    <td style={td}>{b.namespace}</td>
                    <td style={td}>
                      {b.current_spend.toFixed(2)} / {b.monthly_limit.toFixed(2)}
                    </td>
                    <td style={td}>{b.alert_threshold_percent}%</td>
                    <td style={td}>{b.status}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Incidents</h3>
        {sectionErrors.incidents ? (
          <p style={sectionError}>{sectionErrors.incidents}</p>
        ) : incidents ? (
          <>
            <CapabilityBanner context={incidents.vmrogue_context} />
            <p style={bodyText}>
              {incidents.open_incidents} open · {incidents.critical} critical · {incidents.warning} warning ·{" "}
              {incidents.resolved_last_24h} resolved (24h)
            </p>
            {incidents.events.length === 0 && !loading ? (
              <p style={muted}>No incidents in this scope.</p>
            ) : (
              <div style={{ ...tableWrap, marginTop: 12 }}>
                <table style={table}>
                  <thead>
                    <tr>
                      <th style={th}>Time</th>
                      <th style={th}>Severity</th>
                      <th style={th}>Title</th>
                      <th style={th}>Namespace</th>
                      <th style={th}>Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {incidents.events.slice(0, 20).map((ev) => (
                      <tr key={ev.id} style={tr}>
                        <td style={tdMono}>{ev.timestamp ? ev.timestamp.slice(0, 19) : "—"}</td>
                        <td style={td}>{ev.severity}</td>
                        <td style={td}><strong>{ev.title}</strong></td>
                        <td style={td}>{ev.namespace}</td>
                        <td style={td}>{ev.resolved ? "resolved" : "open"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </>
        ) : (
          <p style={muted}>{loading ? "…" : "No incident data"}</p>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Recent logs</h3>
        {sectionErrors.logs ? (
          <p style={sectionError}>{sectionErrors.logs}</p>
        ) : logs ? (
          <>
            <CapabilityBanner context={logs.vmrogue_context} />
            <p style={bodyText}>
              {logs.error_count} errors · {logs.warn_count} warnings · {logs.info_count} info · {logs.total_1h} lines
            </p>
            {logs.lines.length === 0 && !loading ? (
              <p style={muted}>No log lines for this scope.</p>
            ) : (
              <div style={{ ...tableWrap, marginTop: 12 }}>
                <table style={table}>
                  <thead>
                    <tr>
                      <th style={th}>Time</th>
                      <th style={th}>Level</th>
                      <th style={th}>Source</th>
                      <th style={th}>Message</th>
                    </tr>
                  </thead>
                  <tbody>
                    {logs.lines.slice(0, 30).map((line, idx) => (
                      <tr key={`${line.ts}-${idx}`} style={tr}>
                        <td style={tdMono}>{line.ts !== "-" ? line.ts.slice(0, 19) : "—"}</td>
                        <td style={td}>{line.level}</td>
                        <td style={tdMono}>{line.source}</td>
                        <td style={td}>{line.msg}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </>
        ) : (
          <p style={muted}>{loading ? "…" : "No log data"}</p>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Distributed traces</h3>
        {sectionErrors.traces ? <p style={sectionError}>{sectionErrors.traces}</p> : null}
        {traces ? (
          <>
            <CapabilityBanner context={traces.vmrogue_context} />
            <div style={statGrid}>
              <Stat label="Traces" value={String(traces.total_traces)} />
              <Stat label="Success rate" value={`${traces.success_rate.toFixed(1)}%`} ok />
              <Stat label="Errors (1h)" value={String(traces.errors_1h)} />
            </div>
            {traces.traces.length > 0 ? (
              <div style={{ ...tableWrap, marginTop: 12 }}>
                <table style={table}>
                  <thead>
                    <tr>
                      <th style={th}>Trace</th>
                      <th style={th}>Service</th>
                      <th style={th}>Operation</th>
                      <th style={th}>Duration</th>
                      <th style={th}>Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {traces.traces.slice(0, 15).map((t) => (
                      <tr key={t.trace_id} style={tr}>
                        <td style={tdMono}>{t.trace_id}</td>
                        <td style={td}>{t.service}</td>
                        <td style={td}>{t.operation}</td>
                        <td style={td}>{t.duration_ms} ms</td>
                        <td style={td}>{t.status}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <p style={muted}>No traces in scope.</p>
            )}
          </>
        ) : (
          <p style={muted}>{loading ? "…" : "No trace data"}</p>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>VM metrics timeline (Prometheus)</h3>
        {sectionErrors.timeline ? <p style={sectionError}>{sectionErrors.timeline}</p> : null}
        {timeline ? (
          <>
            <CapabilityBanner context={timeline.vmrogue_context} />
            <p style={bodyText}>
              {timeline.vm_name ? (
                <>
                  <strong>{timeline.namespace}/{timeline.vm_name}</strong> — {timeline.metric} ({timeline.unit})
                </>
              ) : (
                "Select a running VM in a single namespace to populate timeline."
              )}
            </p>
            {timeline.points.length > 0 ? (
              <p style={note}>
                {timeline.points.length} samples · latest {timeline.points[timeline.points.length - 1]?.value.toFixed(2)}{" "}
                {timeline.unit}
              </p>
            ) : (
              <p style={muted}>Set VMROGUE_PROMETHEUS_URL and ensure kubevirt_vmi_* metrics are scraped.</p>
            )}
          </>
        ) : (
          <p style={muted}>{loading ? "…" : "No timeline data"}</p>
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
const budgetFormRow: CSSProperties = { display: "flex", flexWrap: "wrap", gap: 8, alignItems: "center", marginBottom: 12 };
const input: CSSProperties = { padding: "8px 10px", borderRadius: 6, border: "1px solid #d1d5db", fontSize: 13, minWidth: 120 };

export default InsightsPanel;
