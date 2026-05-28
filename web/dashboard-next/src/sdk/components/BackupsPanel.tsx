// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  fetchAllSnapshots,
  fetchSnapshotSchedules,
  fetchVeleroStatus,
  fetchVmrogueBackups,
  type SnapshotScheduleRecord,
  type VeleroBackupRecord,
  type VeleroRestoreRecord,
  type VeleroStatusResponse,
  type VmBackupRecord,
  type VmSnapshot,
} from "../../lib/api";
import { requestVmrogueNav } from "../../lib/nav";
import CapabilityBanner from "./CapabilityBanner";
import ErrorBanner from "./ErrorBanner";

type Props = { scopeNamespace?: string };

export function BackupsPanel({ scopeNamespace = "all" }: Props) {
  const [velero, setVelero] = useState<VeleroStatusResponse | null>(null);
  const [vmBackups, setVmBackups] = useState<VmBackupRecord[]>([]);
  const [snapshots, setSnapshots] = useState<VmSnapshot[]>([]);
  const [schedules, setSchedules] = useState<SnapshotScheduleRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [veleroError, setVeleroError] = useState<string | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setVeleroError(null);
    setBackupError(null);
    const results = await Promise.allSettled([
      fetchVeleroStatus(),
      fetchVmrogueBackups(scopeNamespace),
      fetchAllSnapshots(scopeNamespace),
      fetchSnapshotSchedules(scopeNamespace),
    ]);
    if (results[0].status === "fulfilled") setVelero(results[0].value);
    else {
      setVelero(null);
      setVeleroError(results[0].reason instanceof Error ? results[0].reason.message : "Velero status failed");
    }
    if (results[1].status === "fulfilled") setVmBackups(results[1].value);
    else {
      setVmBackups([]);
      setBackupError(results[1].reason instanceof Error ? results[1].reason.message : "Backup list failed");
    }
    if (results[2].status === "fulfilled") setSnapshots(results[2].value);
    else setSnapshots([]);
    if (results[3].status === "fulfilled") setSchedules(results[3].value);
    else setSchedules([]);
    setLoading(false);
  }, [scopeNamespace]);

  useEffect(() => {
    void load();
  }, [load]);

  const goStorage = (section: string) => {
    requestVmrogueNav({ view: "storage", scrollTo: section });
  };

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Backups</h2>
          <p style={subtitle}>
            {loading
              ? "Loading Velero and KubeVirt backup data…"
              : `Velero ${velero?.velero_available ? "on" : "off"} · ${velero?.backups.length ?? 0} cluster backup(s) · ${snapshots.length} snapshot(s) · ${schedules.length} schedule(s)`}
          </p>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <button type="button" style={secondaryBtn} onClick={() => goStorage("storage-snapshots")}>
            KubeVirt snapshots
          </button>
          <button type="button" style={secondaryBtn} onClick={() => goStorage("storage-schedules")}>
            Cron schedules
          </button>
          <button type="button" style={secondaryBtn} onClick={() => requestVmrogueNav({ view: "operations" })}>
            DR / failover
          </button>
          <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
            {loading ? "Refreshing…" : "Refresh"}
          </button>
        </div>
      </div>

      {veleroError ? <ErrorBanner message={veleroError} /> : null}
      {backupError ? <ErrorBanner message={backupError} /> : null}

      <section style={section}>
        <h3 style={sectionTitle}>Velero cluster backups</h3>
        {velero ? (
          <>
            <CapabilityBanner context={velero.vmrogue_context} />
            {!velero.velero_available ? (
              <p style={muted}>
                Velero CRDs not found. Use KubeVirt snapshots and schedules below, or install Velero for
                namespace/cluster backups.
              </p>
            ) : null}
            <VeleroTable title="Backups" rows={velero.backups} kind="backup" />
            <VeleroTable title="Restores" rows={velero.restores} kind="restore" />
          </>
        ) : (
          <p style={muted}>{loading ? "…" : "No Velero status"}</p>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>VM backup API (snapshots)</h3>
        <p style={hint}>
          <code style={code}>GET /api/v1/backups</code> maps VirtualMachineSnapshots in scope. Create
          snapshots from <strong>Clusters &amp; VMs</strong> on each VM row.
        </p>
        {vmBackups.length === 0 && !loading ? (
          <p style={muted}>No snapshots returned for this scope.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Namespace</th>
                  <th style={th}>Backup</th>
                  <th style={th}>VM</th>
                  <th style={th}>Status</th>
                  <th style={th}>Created</th>
                </tr>
              </thead>
              <tbody>
                {vmBackups.slice(0, 50).map((b) => (
                  <tr key={`${b.namespace}/${b.name}`} style={tr}>
                    <td style={td}>{b.namespace}</td>
                    <td style={td}><strong>{b.name}</strong></td>
                    <td style={td}>{b.vm_name}</td>
                    <td style={td}>{b.status}</td>
                    <td style={tdMono}>{b.created_at ? b.created_at.slice(0, 19) : "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <section style={section}>
        <h3 style={sectionTitle}>Snapshot schedules (summary)</h3>
        {schedules.length === 0 && !loading ? (
          <p style={muted}>No cron schedules in scope.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Namespace</th>
                  <th style={th}>VM</th>
                  <th style={th}>Cron (UTC)</th>
                  <th style={th}>Retention</th>
                </tr>
              </thead>
              <tbody>
                {schedules.slice(0, 20).map((row) => (
                  <tr key={`${row.namespace}/${row.name}`} style={tr}>
                    <td style={td}>{row.namespace}</td>
                    <td style={td}>{row.vm_name}</td>
                    <td style={tdMono}>{row.cron}</td>
                    <td style={td}>{row.max_snapshots > 0 ? `keep ${row.max_snapshots}` : "unlimited"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {schedules.length > 20 ? (
          <p style={hint}>
            Showing 20 of {schedules.length}.{" "}
            <button type="button" style={linkBtn} onClick={() => goStorage("storage-schedules")}>
              View all schedules
            </button>
          </p>
        ) : null}
      </section>
    </div>
  );
}

function VeleroTable({
  title,
  rows,
  kind,
}: {
  title: string;
  rows: VeleroBackupRecord[] | VeleroRestoreRecord[];
  kind: "backup" | "restore";
}) {
  if (rows.length === 0) {
    return <p style={muted}>No Velero {title.toLowerCase()}.</p>;
  }
  return (
    <>
      <h4 style={subTitle}>{title}</h4>
      <div style={tableWrap}>
        <table style={table}>
          <thead>
            <tr>
              <th style={th}>Name</th>
              <th style={th}>Namespace</th>
              <th style={th}>Phase</th>
              <th style={th}>Details</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={`${kind}-${row.namespace}-${row.name}`} style={tr}>
                <td style={td}><strong>{row.name}</strong></td>
                <td style={td}>{row.namespace}</td>
                <td style={td}>{row.phase}</td>
                <td style={tdMono}>
                  {kind === "backup"
                    ? (row as VeleroBackupRecord).storage_location ||
                      `${(row as VeleroBackupRecord).items_backed_up} items`
                    : `from ${(row as VeleroRestoreRecord).backup_name || "—"}`}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1200, margin: "0 auto" };
const header: CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, marginBottom: 16, flexWrap: "wrap",
};
const title: CSSProperties = { margin: 0, fontSize: 22, color: "#222324" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const hint: CSSProperties = { margin: "0 0 12px", fontSize: 13, color: "#6b7280", lineHeight: 1.45 };
const muted: CSSProperties = { color: "#6b7280", fontSize: 14, margin: "0 0 12px" };
const code: CSSProperties = { fontSize: 12, background: "#f3f4f6", padding: "1px 4px", borderRadius: 3 };
const section: CSSProperties = { marginBottom: 28 };
const sectionTitle: CSSProperties = { margin: "0 0 12px", fontSize: 16, color: "#222324" };
const subTitle: CSSProperties = { margin: "16px 0 8px", fontSize: 14, color: "#374151" };
const refreshBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
};
const secondaryBtn: CSSProperties = {
  ...refreshBtn, borderColor: "#f0583a", color: "#c2410c",
};
const linkBtn: CSSProperties = {
  border: "none", background: "none", color: "#c2410c", fontWeight: 600, cursor: "pointer", padding: 0, fontSize: 13,
};
const tableWrap: CSSProperties = { overflowX: "auto", background: "#fff", borderRadius: 8, border: "1px solid #e5e7eb", marginBottom: 12 };
const table: CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 13 };
const th: CSSProperties = {
  textAlign: "left", padding: "12px 14px", background: "#f9fafb", borderBottom: "1px solid #e5e7eb", fontWeight: 600,
};
const tr: CSSProperties = { borderBottom: "1px solid #f3f4f6" };
const td: CSSProperties = { padding: "12px 14px", color: "#374151", verticalAlign: "top" };
const tdMono: CSSProperties = { ...td, fontFamily: "ui-monospace, monospace", fontSize: 12 };

export default BackupsPanel;
