import { useCallback, useEffect, useMemo, useState, type CSSProperties } from "react";
import { fetchPods, type PodRecord } from "../../lib/api";

type Props = { scopeNamespace?: string };

const PHASES = ["all", "Running", "Pending", "Failed", "Succeeded", "Unknown"] as const;

export function WorkloadsPanel({ scopeNamespace = "all" }: Props) {
  const [pods, setPods] = useState<PodRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [phaseFilter, setPhaseFilter] = useState<(typeof PHASES)[number]>("all");

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setPods(await fetchPods(scopeNamespace));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load pods");
      setPods([]);
    } finally {
      setLoading(false);
    }
  }, [scopeNamespace]);

  useEffect(() => {
    void load();
  }, [load]);

  const filteredPods = useMemo(() => {
    const q = search.trim().toLowerCase();
    return pods.filter((pod) => {
      if (phaseFilter !== "all" && pod.phase !== phaseFilter) return false;
      if (!q) return true;
      const haystack = [
        pod.namespace,
        pod.name,
        pod.node_name,
        pod.ip,
        pod.phase,
        ...(pod.containers ?? []),
      ]
        .join(" ")
        .toLowerCase();
      return haystack.includes(q);
    });
  }, [pods, search, phaseFilter]);

  const running = pods.filter((p) => p.phase === "Running").length;

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Workloads</h2>
          <p style={subtitle}>
            {loading
              ? "Loading pods…"
              : `${running} running · ${filteredPods.length} shown · ${pods.length} total · namespace: ${scopeNamespace}`}
          </p>
        </div>
        <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      <div style={toolbar}>
        <input
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search name, namespace, node, container…"
          style={searchInput}
          aria-label="Search pods"
        />
        <label style={filterLabel}>
          Phase
          <select
            value={phaseFilter}
            onChange={(e) => setPhaseFilter(e.target.value as (typeof PHASES)[number])}
            style={select}
          >
            {PHASES.map((phase) => (
              <option key={phase} value={phase}>
                {phase === "all" ? "All phases" : phase}
              </option>
            ))}
          </select>
        </label>
      </div>

      {error ? (
        <div style={errorBox} role="alert">
          {error}
        </div>
      ) : null}
      {filteredPods.length === 0 && !loading ? (
        <p style={muted}>{pods.length === 0 ? "No pods in this scope." : "No pods match the current filters."}</p>
      ) : (
        <div style={tableWrap}>
          <table style={table}>
            <thead>
              <tr>
                <th style={th}>Namespace</th>
                <th style={th}>Name</th>
                <th style={th}>Phase</th>
                <th style={th}>Containers</th>
                <th style={th}>Node</th>
                <th style={th}>IP</th>
                <th style={th}>Restarts</th>
                <th style={th}>Age</th>
              </tr>
            </thead>
            <tbody>
              {filteredPods.map((pod) => (
                <tr key={`${pod.namespace}/${pod.name}`} style={tr}>
                  <td style={td}>{pod.namespace}</td>
                  <td style={td}>
                    <strong>{pod.name}</strong>
                  </td>
                  <td style={td}>
                    <span style={{ ...badge, ...phaseStyle(pod.phase) }}>{pod.phase}</span>
                  </td>
                  <td style={td}>{pod.containers?.length ? pod.containers.join(", ") : "—"}</td>
                  <td style={td}>{pod.node_name}</td>
                  <td style={tdMono}>{pod.ip || "—"}</td>
                  <td style={td}>{pod.restarts}</td>
                  <td style={td}>{pod.age || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function phaseStyle(phase: string): CSSProperties {
  if (phase === "Running") return { backgroundColor: "#dcfce7", color: "#15803d" };
  if (phase === "Pending") return { backgroundColor: "#fef9c3", color: "#a16207" };
  if (phase === "Failed") return { backgroundColor: "#fee2e2", color: "#b91c1c" };
  return { backgroundColor: "#f3f4f6", color: "#374151" };
}

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1400, margin: "0 auto" };
const header: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "flex-start",
  gap: 16,
  marginBottom: 16,
  flexWrap: "wrap",
};
const toolbar: CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: 12,
  marginBottom: 16,
  alignItems: "center",
};
const searchInput: CSSProperties = {
  flex: "1 1 220px",
  minWidth: 200,
  padding: "8px 10px",
  borderRadius: 6,
  border: "1px solid #d1d5db",
  fontSize: 13,
};
const filterLabel: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  fontSize: 13,
  fontWeight: 600,
  color: "#374151",
};
const select: CSSProperties = {
  padding: "6px 10px",
  borderRadius: 6,
  border: "1px solid #d1d5db",
  fontSize: 13,
};
const title: CSSProperties = { margin: 0, fontSize: 22, color: "#222324" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
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
const muted: CSSProperties = { color: "#6b7280", fontSize: 14 };
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
  color: "#374151",
  whiteSpace: "nowrap",
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151", verticalAlign: "top" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };
const badge: CSSProperties = {
  display: "inline-block",
  padding: "3px 8px",
  borderRadius: 999,
  fontSize: 12,
  fontWeight: 600,
};

export default WorkloadsPanel;
