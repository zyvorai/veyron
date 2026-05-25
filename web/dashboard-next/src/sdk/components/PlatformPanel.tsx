import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  fetchCustomResources,
  fetchGitOpsStatus,
  triggerGitOpsSync,
  type CustomResourceRecord,
  type GitOpsStatusRecord,
} from "../../lib/api";
import CapabilityBanner from "./CapabilityBanner";
import ErrorBanner from "./ErrorBanner";

type Props = {
  scopeNamespace?: string;
};

export function PlatformPanel({ scopeNamespace = "all" }: Props) {
  const [gitops, setGitops] = useState<GitOpsStatusRecord | null>(null);
  const [crds, setCrds] = useState<CustomResourceRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [gitopsError, setGitopsError] = useState<string | null>(null);
  const [crdsError, setCrdsError] = useState<string | null>(null);
  const [syncBusy, setSyncBusy] = useState(false);
  const [syncMessage, setSyncMessage] = useState<string | null>(null);
  const [syncError, setSyncError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setGitopsError(null);
    setCrdsError(null);
    const [statusResult, resourcesResult] = await Promise.allSettled([
      fetchGitOpsStatus(scopeNamespace),
      fetchCustomResources(),
    ]);

    if (statusResult.status === "fulfilled") {
      setGitops(statusResult.value);
    } else {
      setGitops(null);
      setGitopsError(
        statusResult.reason instanceof Error
          ? statusResult.reason.message
          : "Failed to load GitOps status",
      );
    }

    if (resourcesResult.status === "fulfilled") {
      setCrds(resourcesResult.value);
    } else {
      setCrds([]);
      setCrdsError(
        resourcesResult.reason instanceof Error
          ? resourcesResult.reason.message
          : "Failed to load CRD inventory",
      );
    }

    setLoading(false);
  }, [scopeNamespace]);

  useEffect(() => {
    void load();
  }, [load]);

  const runSync = async () => {
    setSyncBusy(true);
    setSyncError(null);
    setSyncMessage(null);
    try {
      const result = await triggerGitOpsSync({ force: false, dry_run: false });
      setSyncMessage(result.message ?? "Sync triggered");
      await load();
    } catch (e) {
      setSyncError(e instanceof Error ? e.message : "GitOps sync failed");
    } finally {
      setSyncBusy(false);
    }
  };

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Platform</h2>
          <p style={subtitle}>
            {loading
              ? "Loading GitOps and CRD inventory…"
              : `GitOps: ${gitops?.sync_status ?? "unknown"} · ${crds.length} CRD(s) · namespace scope: ${scopeNamespace}`}
          </p>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
            {loading ? "Refreshing…" : "Refresh"}
          </button>
          <button type="button" style={syncBtn} onClick={() => void runSync()} disabled={syncBusy || loading}>
            {syncBusy ? "Syncing…" : "Sync now"}
          </button>
        </div>
      </div>

      {syncError ? <ErrorBanner message={syncError} /> : null}
      {syncMessage && !syncError ? (
        <p style={successNote}>{syncMessage}</p>
      ) : null}

      <section style={section}>
        <h3 style={sectionTitle}>GitOps status</h3>
        {gitopsError ? <ErrorBanner message={gitopsError} /> : null}
        {gitops ? (
          <>
            <CapabilityBanner context={gitops.vmrogue_context} />
            <div style={statGrid}>
              <div style={statCard}>
                <div style={statLabel}>Repo</div>
                <div style={statValue}>{gitops.repo_url || "Not configured"}</div>
              </div>
              <div style={statCard}>
                <div style={statLabel}>Branch</div>
                <div style={statValue}>{gitops.branch || "—"}</div>
              </div>
              <div style={statCard}>
                <div style={statLabel}>Sync</div>
                <div style={{ ...statValue, ...syncStyle(gitops.sync_status) }}>{gitops.sync_status}</div>
              </div>
              <div style={statCard}>
                <div style={statLabel}>Drift</div>
                <div style={statValue}>{gitops.drift_detected ? "Detected" : "Clear"}</div>
              </div>
            </div>
            {(gitops.argo_applications?.length ?? 0) > 0 ? (
              <p style={note}>
                <strong>Argo CD applications:</strong> {gitops.argo_applications!.join(", ")}
              </p>
            ) : null}
            {(gitops.flux_kustomizations?.length ?? 0) > 0 ? (
              <p style={note}>
                <strong>Flux kustomizations:</strong> {gitops.flux_kustomizations!.join(", ")}
              </p>
            ) : null}
            {gitops.note ? <p style={note}>{gitops.note}</p> : null}
          </>
        ) : !gitopsError ? (
          <p style={muted}>{loading ? "Loading…" : "No GitOps status returned."}</p>
        ) : null}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Custom resource definitions</h3>
        {crdsError ? <ErrorBanner message={crdsError} /> : null}
        {crds.length === 0 && !loading && !crdsError ? (
          <p style={muted}>No CRDs returned (check API RBAC for apiextensions.k8s.io).</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Name</th>
                  <th style={th}>Group</th>
                  <th style={th}>Version</th>
                  <th style={th}>Kind</th>
                  <th style={th}>Scope</th>
                  <th style={th}>Instances</th>
                </tr>
              </thead>
              <tbody>
                {crds.map((cr) => (
                  <tr key={cr.name} style={tr}>
                    <td style={td}><strong>{cr.name}</strong></td>
                    <td style={tdMono}>{cr.group}</td>
                    <td style={td}>{cr.version}</td>
                    <td style={td}>{cr.kind}</td>
                    <td style={td}>{cr.scope}</td>
                    <td style={td}>{cr.instance_count}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <p style={footerNote}>
        VMRogue CRD editors and full GitOps workflows remain in the{" "}
        <a href="/dashboard" style={link}>full dashboard</a>.
      </p>
    </div>
  );
}

function syncStyle(status: string): CSSProperties {
  if (status === "synced") return { color: "#15803d" };
  if (status === "out_of_sync") return { color: "#b45309" };
  if (status === "not_configured") return { color: "#6b7280" };
  return { color: "#1d4ed8" };
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
const syncBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #f0583a", background: "#f0583a", color: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const section: CSSProperties = { marginBottom: 28 };
const sectionTitle: CSSProperties = { margin: "0 0 12px", fontSize: 16, color: "#222324" };
const muted: CSSProperties = { color: "#6b7280", fontSize: 14 };
const note: CSSProperties = { margin: "12px 0 0", fontSize: 12, color: "#6b7280", lineHeight: 1.45 };
const successNote: CSSProperties = { margin: "0 0 16px", fontSize: 13, color: "#15803d", fontWeight: 600 };
const statGrid: CSSProperties = {
  display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 12,
};
const statCard: CSSProperties = {
  background: "#fff", border: "1px solid #e5e7eb", borderRadius: 8, padding: "12px 14px",
};
const statLabel: CSSProperties = { fontSize: 11, fontWeight: 600, color: "#6b7280", textTransform: "uppercase", letterSpacing: "0.04em" };
const statValue: CSSProperties = { marginTop: 6, fontSize: 14, fontWeight: 600, color: "#222324", wordBreak: "break-word" };
const tableWrap: CSSProperties = { overflowX: "auto", background: "#fff", borderRadius: 8, border: "1px solid #e5e7eb" };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "12px 14px", background: "#f9fafb", borderBottom: "1px solid #e5e7eb", fontWeight: 600, color: "#374151", whiteSpace: "nowrap",
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151", verticalAlign: "top" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };
const footerNote: CSSProperties = { marginTop: 8, fontSize: 12, color: "#9ca3af" };
const link: CSSProperties = { color: "#f0583a", fontWeight: 600 };

export default PlatformPanel;
