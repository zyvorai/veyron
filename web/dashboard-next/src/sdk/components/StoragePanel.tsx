// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import {
  deleteSnapshotSchedule,
  deleteVmSnapshot,
  fetchAllSnapshots,
  fetchPvcs,
  fetchSnapshotSchedules,
  fetchStorageClasses,
  restoreVmSnapshot,
  type PvcRecord,
  type SnapshotScheduleRecord,
  type StorageClassRecord,
  type VmSnapshot,
} from "../../lib/api";
import ErrorBanner from "./ErrorBanner";

type Props = {
  scopeNamespace?: string;
};

export function StoragePanel({ scopeNamespace = "all" }: Props) {
  const [classes, setClasses] = useState<StorageClassRecord[]>([]);
  const [pvcs, setPvcs] = useState<PvcRecord[]>([]);
  const [snapshots, setSnapshots] = useState<VmSnapshot[]>([]);
  const [schedules, setSchedules] = useState<SnapshotScheduleRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [snapError, setSnapError] = useState<string | null>(null);
  const [schedError, setSchedError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    setSnapError(null);
    setSchedError(null);
    const results = await Promise.allSettled([
      fetchStorageClasses(),
      fetchPvcs(scopeNamespace),
      fetchAllSnapshots(scopeNamespace),
      fetchSnapshotSchedules(scopeNamespace),
    ]);
    if (results[0].status === "fulfilled") setClasses(results[0].value);
    else {
      setClasses([]);
      setError(results[0].reason instanceof Error ? results[0].reason.message : "Failed to load storage classes");
    }
    if (results[1].status === "fulfilled") setPvcs(results[1].value);
    else {
      setPvcs([]);
      if (!error) setError(results[1].reason instanceof Error ? results[1].reason.message : "Failed to load PVCs");
    }
    if (results[2].status === "fulfilled") setSnapshots(results[2].value);
    else {
      setSnapshots([]);
      setSnapError(results[2].reason instanceof Error ? results[2].reason.message : "Failed to load snapshots");
    }
    if (results[3].status === "fulfilled") setSchedules(results[3].value);
    else {
      setSchedules([]);
      setSchedError(results[3].reason instanceof Error ? results[3].reason.message : "Failed to load schedules");
    }
    setLoading(false);
  }, [scopeNamespace]);

  useEffect(() => {
    void load();
  }, [load]);

  const removeSnapshot = async (snap: VmSnapshot) => {
    if (!confirm(`Delete snapshot "${snap.name}" in ${snap.namespace}?`)) return;
    const key = `del-${snap.namespace}/${snap.name}`;
    setBusy(key);
    setSnapError(null);
    try {
      await deleteVmSnapshot(snap.namespace, snap.name);
      await load();
    } catch (e) {
      setSnapError(e instanceof Error ? e.message : "Delete failed");
    } finally {
      setBusy(null);
    }
  };

  const restoreSnapshot = async (snap: VmSnapshot) => {
    if (!confirm(`Restore "${snap.name}" for VM ${snap.vm_name}? Stop the VM first.`)) return;
    const key = `restore-${snap.namespace}/${snap.name}`;
    setBusy(key);
    setSnapError(null);
    try {
      await restoreVmSnapshot(snap.namespace, snap.name);
    } catch (e) {
      setSnapError(e instanceof Error ? e.message : "Restore failed");
    } finally {
      setBusy(null);
    }
  };

  const removeSchedule = async (row: SnapshotScheduleRecord) => {
    if (!confirm(`Delete schedule "${row.name}"?`)) return;
    setBusy(`sched-${row.name}`);
    setSchedError(null);
    try {
      await deleteSnapshotSchedule(row.namespace, row.name);
      await load();
    } catch (e) {
      setSchedError(e instanceof Error ? e.message : "Delete schedule failed");
    } finally {
      setBusy(null);
    }
  };

  return (
    <div style={wrap}>
      <div style={header}>
        <div>
          <h2 style={title}>Storage</h2>
          <p style={subtitle}>
            {loading
              ? "Loading storage, snapshots, and schedules…"
              : `${classes.length} class(es) · ${pvcs.length} PVC(s) · ${snapshots.length} snapshot(s) · ${schedules.length} schedule(s) · ${scopeNamespace}`}
          </p>
        </div>
        <button type="button" style={refreshBtn} onClick={() => void load()} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      {error ? <ErrorBanner message={error} /> : null}

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

      <section id="storage-snapshots" style={section}>
        <h3 style={sectionTitle}>VM snapshots (KubeVirt)</h3>
        <p style={hint}>
          Create snapshots per VM under <strong>Clusters &amp; VMs</strong>. Restore and delete from here.
        </p>
        {snapError ? <ErrorBanner message={snapError} /> : null}
        {snapshots.length === 0 && !loading ? (
          <p style={muted}>No VirtualMachineSnapshots in this scope.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Namespace</th>
                  <th style={th}>Snapshot</th>
                  <th style={th}>VM</th>
                  <th style={th}>Status</th>
                  <th style={th}>Age</th>
                  <th style={th} />
                </tr>
              </thead>
              <tbody>
                {snapshots.map((snap) => {
                  const key = `${snap.namespace}/${snap.name}`;
                  return (
                    <tr key={key} style={tr}>
                      <td style={td}>{snap.namespace}</td>
                      <td style={td}><strong>{snap.name}</strong></td>
                      <td style={td}>{snap.vm_name}</td>
                      <td style={td}>
                        <span style={{ ...badge, ...snapStatusStyle(snap.status) }}>{snap.status}</span>
                      </td>
                      <td style={td}>{snap.age || "—"}</td>
                      <td style={td}>
                        <div style={btnRow}>
                          <button
                            type="button"
                            style={actionBtn}
                            disabled={busy !== null}
                            onClick={() => void restoreSnapshot(snap)}
                          >
                            Restore
                          </button>
                          <button
                            type="button"
                            style={dangerBtn}
                            disabled={busy !== null}
                            onClick={() => void removeSnapshot(snap)}
                          >
                            Delete
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <section id="storage-schedules" style={section}>
        <h3 style={sectionTitle}>Snapshot schedules</h3>
        <p style={hint}>
          Cron schedules are stored as ConfigMaps and run on the API lease leader (UTC). Add schedules on a VM detail row.
        </p>
        {schedError ? <ErrorBanner message={schedError} /> : null}
        {schedules.length === 0 && !loading ? (
          <p style={muted}>No snapshot schedules in this scope.</p>
        ) : (
          <div style={tableWrap}>
            <table style={table}>
              <thead>
                <tr>
                  <th style={th}>Namespace</th>
                  <th style={th}>Schedule</th>
                  <th style={th}>VM</th>
                  <th style={th}>Cron (UTC)</th>
                  <th style={th}>Retention</th>
                  <th style={th}>Last run</th>
                  <th style={th} />
                </tr>
              </thead>
              <tbody>
                {schedules.map((row) => (
                  <tr key={`${row.namespace}/${row.name}`} style={tr}>
                    <td style={td}>{row.namespace}</td>
                    <td style={tdMono}>{row.name}</td>
                    <td style={td}>{row.vm_name}</td>
                    <td style={tdMono}>{row.cron}</td>
                    <td style={td}>
                      {row.max_snapshots > 0 ? `keep ${row.max_snapshots}` : "unlimited"}
                      {!row.enabled ? " · disabled" : ""}
                    </td>
                    <td style={td}>{row.last_run ? row.last_run.slice(0, 19) : "—"}</td>
                    <td style={td}>
                      <button
                        type="button"
                        style={dangerBtn}
                        disabled={busy !== null}
                        onClick={() => void removeSchedule(row)}
                      >
                        Delete
                      </button>
                    </td>
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

function pvcStatusStyle(status: string): CSSProperties {
  const s = status.toLowerCase();
  if (s === "bound") return { backgroundColor: "#dcfce7", color: "#15803d" };
  if (s === "pending") return { backgroundColor: "#fef9c3", color: "#a16207" };
  return { backgroundColor: "#f3f4f6", color: "#374151" };
}

function snapStatusStyle(status: string): CSSProperties {
  const s = status.toLowerCase();
  if (s.includes("succeed")) return { backgroundColor: "#dcfce7", color: "#166534" };
  if (s.includes("fail")) return { backgroundColor: "#fee2e2", color: "#991b1b" };
  if (s.includes("progress") || s.includes("pending")) {
    return { backgroundColor: "#fef3c7", color: "#92400e" };
  }
  return { backgroundColor: "#f3f4f6", color: "#374151" };
}

const wrap: CSSProperties = { padding: "20px 24px 32px", maxWidth: 1400, margin: "0 auto" };
const header: CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, marginBottom: 20, flexWrap: "wrap",
};
const title: CSSProperties = { margin: 0, fontSize: 22, color: "#222324" };
const subtitle: CSSProperties = { margin: "6px 0 0", fontSize: 13, color: "#6b7280" };
const hint: CSSProperties = { margin: "0 0 12px", fontSize: 13, color: "#6b7280", lineHeight: 1.45 };
const refreshBtn: CSSProperties = {
  padding: "8px 14px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontWeight: 600, fontSize: 13, cursor: "pointer",
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
const btnRow: CSSProperties = { display: "flex", gap: 6, flexWrap: "wrap" };
const actionBtn: CSSProperties = {
  padding: "4px 10px", borderRadius: 6, border: "1px solid #d1d5db", background: "#fff", fontSize: 12, fontWeight: 600, cursor: "pointer",
};
const dangerBtn: CSSProperties = {
  ...actionBtn, borderColor: "#fecaca", color: "#b91c1c",
};

export default StoragePanel;
